//! Windows handles pin the inspected target and its ancestors until staging.
use crate::{ensure_no_reparse_path, is_reparse_point, path_key};
use serde::{Deserialize, Serialize};
use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
};
use windows::{
    Win32::{Foundation::HANDLE, Storage::FileSystem::*, System::Com::*, UI::Shell::*},
    core::PCWSTR,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub volume: u32,
    pub index: u64,
    pub created: u64,
}

fn information(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // The File owns this handle and remains alive throughout the call.
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }
        .map_err(io::Error::other)?;
    if crate::attributes_need_skipping(info.dwFileAttributes) {
        return Err(io::Error::other(
            "Links and placeholders are not supported for cleanup.",
        ));
    }
    Ok(info)
}
fn file_identity(file: &File) -> io::Result<Identity> {
    let info = information(file)?;
    Ok(Identity {
        volume: info.dwVolumeSerialNumber,
        index: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        created: ((info.ftCreationTime.dwHighDateTime as u64) << 32)
            | info.ftCreationTime.dwLowDateTime as u64,
    })
}
fn open(path: &Path, access: u32, share: u32) -> io::Result<File> {
    OpenOptions::new()
        .access_mode(access)
        .share_mode(share)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
}
pub fn identity(path: &Path) -> io::Result<Identity> {
    file_identity(&open(
        path,
        FILE_READ_ATTRIBUTES.0,
        FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0,
    )?)
}
pub fn ensure_idle_file(path: &Path) -> io::Result<()> {
    let file = open(
        path,
        FILE_READ_DATA.0 | FILE_READ_ATTRIBUTES.0,
        FILE_SHARE_READ.0,
    )?;
    information(&file)?;
    Ok(())
}
pub fn lock_evidence(project: &Path, names: &[String]) -> io::Result<Vec<File>> {
    let mut files = Vec::new();
    for name in names {
        if name.contains(['/', '\\']) || name == ".." {
            return Err(io::Error::other("Invalid evidence path."));
        }
        let path = project.join(name);
        if path.is_file() {
            let file = open(
                &path,
                FILE_READ_DATA.0 | FILE_READ_ATTRIBUTES.0,
                FILE_SHARE_READ.0,
            )?;
            information(&file)?;
            files.push(file);
        }
    }
    Ok(files)
}
pub fn supported_volume(path: &Path) -> io::Result<()> {
    let key = path_key(path);
    let bytes = key.as_bytes();
    if bytes.len() < 3 || bytes[1] != b':' || bytes[2] != b'\\' || !bytes[0].is_ascii_alphabetic() {
        return Err(io::Error::other(
            "Cleanup requires a local fixed NTFS drive. Network/device paths are rejected.",
        ));
    }
    let root: Vec<u16> = format!("{}\\", &key[..2])
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut name = [0u16; 32];
    // Buffers are NUL-terminated or sized slices as required by these APIs.
    unsafe {
        if GetDriveTypeW(PCWSTR(root.as_ptr())) != 3 {
            return Err(io::Error::other(
                "Cleanup is unavailable on removable or network drives.",
            ));
        }
        GetVolumeInformationW(
            PCWSTR(root.as_ptr()),
            None,
            None,
            None,
            None,
            Some(&mut name),
        )
        .map_err(io::Error::other)?;
    }
    let len = name.iter().position(|v| *v == 0).unwrap_or(name.len());
    if String::from_utf16_lossy(&name[..len]) != "NTFS" {
        return Err(io::Error::other("Cleanup currently supports NTFS only."));
    }
    Ok(())
}

pub struct CleanupGuard {
    _parents: Vec<File>,
    target: Option<File>,
    path: PathBuf,
}
impl CleanupGuard {
    pub fn acquire(path: &Path, expected: &Identity) -> io::Result<Self> {
        supported_volume(path)?;
        ensure_no_reparse_path(path)?;
        let mut parents = Vec::new();
        let ancestors: Vec<_> = path
            .parent()
            .ok_or_else(|| io::Error::other("No parent directory."))?
            .ancestors()
            .collect();
        for parent in ancestors.into_iter().rev() {
            let file = open(
                parent,
                FILE_READ_ATTRIBUTES.0,
                FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0,
            )?;
            information(&file)?;
            parents.push(file);
        }
        // Deny other writers and renames of the target while it is revalidated.
        let target = open(path, DELETE.0 | FILE_READ_ATTRIBUTES.0, FILE_SHARE_READ.0)?;
        if file_identity(&target)? != *expected {
            return Err(io::Error::other(
                "The target was replaced. Scan and plan again.",
            ));
        }
        Ok(Self {
            _parents: parents,
            target: Some(target),
            path: path.into(),
        })
    }
    pub fn staging_path(&self) -> io::Result<PathBuf> {
        let guid = unsafe { CoCreateGuid() }.map_err(io::Error::other)?;
        Ok(self
            .path
            .with_file_name(format!(".benchlight-recycle-{guid:?}")))
    }
    pub fn stage(&mut self, destination: &Path) -> io::Result<()> {
        if destination.parent() != self.path.parent() || destination.exists() {
            return Err(io::Error::other("Invalid staging destination."));
        }
        let filename: Vec<u16> = destination.as_os_str().encode_wide().collect();
        let offset = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
        let length = offset + filename.len() * 2;
        // Vec<u64> provides sufficient alignment for the variable-length Windows structure.
        let mut buffer = vec![0u64; length.div_ceil(8)];
        let pointer = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        unsafe {
            (*pointer).Anonymous.ReplaceIfExists = false;
            (*pointer).RootDirectory = HANDLE::default();
            (*pointer).FileNameLength =
                (filename.len() * 2).try_into().map_err(io::Error::other)?;
            std::ptr::copy_nonoverlapping(
                filename.as_ptr(),
                buffer.as_mut_ptr().cast::<u8>().add(offset).cast::<u16>(),
                filename.len(),
            );
            let file = self
                .target
                .as_ref()
                .ok_or_else(|| io::Error::other("Target already staged."))?;
            SetFileInformationByHandle(
                HANDLE(file.as_raw_handle()),
                FileRenameInfo,
                pointer.cast(),
                length.try_into().map_err(io::Error::other)?,
            )
            .map_err(io::Error::other)?;
        }
        self.path = destination.into();
        Ok(())
    }
    pub fn recycle(mut self, expected: &Identity) -> io::Result<()> {
        if file_identity(
            self.target
                .as_ref()
                .ok_or_else(|| io::Error::other("No staged target."))?,
        )? != *expected
        {
            return Err(io::Error::other("Staged target changed."));
        }
        self.target.take();
        recycle(&self.path, expected)
    }
}

fn recycle(path: &Path, expected: &Identity) -> io::Result<()> {
    ensure_no_reparse_path(path)?;
    if is_reparse_point(&std::fs::symlink_metadata(path)?) || identity(path)? != *expected {
        return Err(io::Error::other("Target changed before recycling."));
    }
    let path = path.to_string_lossy();
    let path = path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned();
    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    // A dedicated worker owns this STA; all COM objects drop before uninitialization.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }.map_err(io::Error::other)?;
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    let _apartment = Apartment;
    let result = (|| -> windows::core::Result<()> {
        unsafe {
            let operation: IFileOperation =
                CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)?;
            operation.SetOperationFlags(FILEOPERATION_FLAGS(
                FOFX_RECYCLEONDELETE.0
                    | FOFX_EARLYFAILURE.0
                    | FOF_NOERRORUI.0
                    | FOF_SILENT.0
                    | FOF_NOCONFIRMATION.0
                    | FOFX_ADDUNDORECORD.0,
            ))?;
            let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)?;
            let recycled = Arc::new(AtomicBool::new(false));
            let sink: IFileOperationProgressSink = RecycleOnly {
                recycled: recycled.clone(),
            }
            .into();
            operation.DeleteItem(&item, &sink)?;
            operation.PerformOperations()?;
            if operation.GetAnyOperationsAborted()?.as_bool() {
                return Err(windows::core::Error::from_hresult(
                    windows::Win32::Foundation::E_ABORT,
                ));
            }
            if !recycled.load(Ordering::SeqCst) {
                return Err(windows::core::Error::from_hresult(
                    windows::Win32::Foundation::E_FAIL,
                ));
            }
            Ok(())
        }
    })();
    result.map_err(io::Error::other)?;
    if Path::new(&path).exists() {
        return Err(io::Error::other(
            "Windows did not recycle the item; the staged directory remains.",
        ));
    }
    Ok(())
}

use windows::core::{HRESULT, Ref, implement};
#[implement(IFileOperationProgressSink)]
struct RecycleOnly {
    recycled: Arc<AtomicBool>,
}
#[allow(non_snake_case, unused_variables)]
impl IFileOperationProgressSink_Impl for RecycleOnly_Impl {
    fn StartOperations(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn FinishOperations(&self, hrresult: HRESULT) -> windows::core::Result<()> {
        hrresult.ok()
    }
    fn PreDeleteItem(&self, dwflags: u32, psiitem: Ref<IShellItem>) -> windows::core::Result<()> {
        // The Shell may offer a permanent-delete fallback. Never authorize it.
        if dwflags & TSF_DELETE_RECYCLE_IF_POSSIBLE.0 as u32 == 0 {
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_ABORT,
            ));
        }
        Ok(())
    }
    fn PostDeleteItem(
        &self,
        dwflags: u32,
        psiitem: Ref<IShellItem>,
        hrdelete: HRESULT,
        psinewlycreated: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        hrdelete.ok()?;
        if psinewlycreated.is_none() {
            return Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_FAIL,
            ));
        }
        self.recycled.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn PreRenameItem(&self, a: u32, b: Ref<IShellItem>, c: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostRenameItem(
        &self,
        a: u32,
        b: Ref<IShellItem>,
        c: &PCWSTR,
        d: HRESULT,
        e: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn PreMoveItem(
        &self,
        a: u32,
        b: Ref<IShellItem>,
        c: Ref<IShellItem>,
        d: &PCWSTR,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostMoveItem(
        &self,
        a: u32,
        b: Ref<IShellItem>,
        c: Ref<IShellItem>,
        d: &PCWSTR,
        e: HRESULT,
        f: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn PreCopyItem(
        &self,
        a: u32,
        b: Ref<IShellItem>,
        c: Ref<IShellItem>,
        d: &PCWSTR,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostCopyItem(
        &self,
        a: u32,
        b: Ref<IShellItem>,
        c: Ref<IShellItem>,
        d: &PCWSTR,
        e: HRESULT,
        f: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn PreNewItem(&self, a: u32, b: Ref<IShellItem>, c: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn PostNewItem(
        &self,
        a: u32,
        b: Ref<IShellItem>,
        c: &PCWSTR,
        d: &PCWSTR,
        e: u32,
        f: HRESULT,
        g: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn UpdateProgress(&self, a: u32, b: u32) -> windows::core::Result<()> {
        Ok(())
    }
    fn ResetTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn PauseTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
    fn ResumeTimer(&self) -> windows::core::Result<()> {
        Ok(())
    }
}
