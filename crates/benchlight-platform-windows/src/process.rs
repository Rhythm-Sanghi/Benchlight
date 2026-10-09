use std::os::windows::{
    io::{AsRawHandle, FromRawHandle, OwnedHandle},
    process::CommandExt,
};
use std::{
    io::{self, Read},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use windows::Win32::System::{
    Diagnostics::ToolHelp::*,
    Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
};
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::{JobObjects::*, Pipes::PeekNamedPipe},
    },
    core::PCWSTR,
};

fn resume_process(id: u32) -> io::Result<()> {
    let raw =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }.map_err(io::Error::other)?;
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw.0) };
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    unsafe { Thread32First(HANDLE(snapshot.as_raw_handle()), &mut entry) }
        .map_err(io::Error::other)?;
    loop {
        if entry.th32OwnerProcessID == id {
            let raw = unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID) }
                .map_err(io::Error::other)?;
            let thread = unsafe { OwnedHandle::from_raw_handle(raw.0) };
            if unsafe { ResumeThread(HANDLE(thread.as_raw_handle())) } == u32::MAX {
                return Err(io::Error::last_os_error());
            }
            return Ok(());
        }
        if unsafe { Thread32Next(HANDLE(snapshot.as_raw_handle()), &mut entry) }.is_err() {
            break;
        }
    }
    Err(io::Error::other(
        "Could not resume the bounded command process.",
    ))
}

fn drain(pipe: &mut (impl Read + AsRawHandle), output: &mut Vec<u8>) -> io::Result<()> {
    let mut available = 0;
    // No other thread reads this anonymous pipe; available bytes cannot be consumed elsewhere.
    if unsafe {
        PeekNamedPipe(
            HANDLE(pipe.as_raw_handle()),
            None,
            0,
            None,
            Some(&mut available),
            None,
        )
    }
    .is_err()
    {
        return Ok(());
    }
    if available > 0 {
        let mut bytes = [0; 4096];
        let count = pipe.read(&mut bytes[..(available as usize).min(4096)])?;
        if output.len() + count > 16384 {
            return Err(io::Error::other("Command output exceeded 16 KiB."));
        }
        output.extend_from_slice(&bytes[..count]);
    }
    Ok(())
}
pub fn run_command(
    executable: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> io::Result<String> {
    run_bounded(executable, args, cwd, timeout, true)
}

pub fn run_stdout_command(
    executable: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> io::Result<String> {
    run_bounded(executable, args, cwd, timeout, false)
}

fn run_bounded(
    executable: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
    include_stderr: bool,
) -> io::Result<String> {
    let handle = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(io::Error::other)?;
    let job = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(
            HANDLE(job.as_raw_handle()),
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&limits) as u32,
        )
    }
    .map_err(io::Error::other)?;
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000 | 0x00000004)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "NUL")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env("GOTOOLCHAIN", "local")
        .spawn()?;
    if let Err(error) = unsafe {
        AssignProcessToJobObject(HANDLE(job.as_raw_handle()), HANDLE(child.as_raw_handle()))
    } {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::other(error));
    }
    if let Err(error) = resume_process(child.id()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("Missing stdout pipe."))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("Missing stderr pipe."))?;
    let start = Instant::now();
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    let result = (|| {
        loop {
            drain(&mut stdout, &mut output)?;
            drain(&mut stderr, &mut diagnostics)?;
            if output.len() + diagnostics.len() > 16 * 1024 {
                return Err(io::Error::other("Command output exceeded 16 KiB."));
            }
            if let Some(status) = child.try_wait()? {
                // Kill remaining descendants before draining their final bounded output.
                unsafe { TerminateJobObject(HANDLE(job.as_raw_handle()), 0) }
                    .map_err(io::Error::other)?;
                for _ in 0..8 {
                    drain(&mut stdout, &mut output)?;
                    drain(&mut stderr, &mut diagnostics)?;
                }
                if !status.success() {
                    return Err(io::Error::other(format!("Command exited with {status}.")));
                }
                if output.len() + diagnostics.len() > 16 * 1024 {
                    return Err(io::Error::other("Command output exceeded 16 KiB."));
                }
                if include_stderr {
                    output.extend(diagnostics);
                }
                return Ok(String::from_utf8_lossy(&output)
                    .trim()
                    .chars()
                    .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
                    .collect());
            }
            if start.elapsed() >= timeout {
                return Err(io::Error::other("Command timed out."));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    })();
    drop(job);
    let _ = child.kill();
    let _ = child.wait();
    result
}
