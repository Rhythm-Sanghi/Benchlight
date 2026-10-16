# Tools and installation evidence

Tools are explicitly refreshed. Benchlight checks the process PATH for Git,
Node/npm/pnpm/Yarn/Bun, Python/pip/uv, Java/Gradle/Maven, Go, Rust/Cargo, .NET,
Docker, WSL, CMake, Ninja and GitHub CLI. It records the first matching PATH
location and shadowed matches. Shell aliases/functions, repository-local version
managers and every possible installer are not covered. Paths are inspected in
bounded lists; the application does not crawl drives for executables.

Native executables receive only their standard version arguments from a neutral
data directory. Windows processes start suspended, enter a job that kills its
descendants on close, then resume. Commands time out after five seconds, capture
at most 16 KiB and have no interactive stdin. This is process coordination, not a
sandbox for malicious executables. Installed tools on PATH are executable code.
Git prompts, .NET telemetry/first-run behavior and rustup automatic toolchain
installation are disabled for these child commands.

Batch/script wrappers and Windows application aliases are not executed. Matching
adjacent package.json metadata can supply a package version (e.g. npm); the UI
labels that evidence rather than calling it a runtime command result. Missing or
unparseable versions are shown as unknown with the reason. Output text is not
stored; only a bounded dotted version token is retained.

Local read-only uninstall metadata is inspected under HKCU/HKLM in both registry
views. Name matches alone are weak associations. InstallLocation containment is
stronger evidence; a recorded WinGetPackageIdentifier supports a likely winget
source. WinGet portable package storage and Scoop/Chocolatey storage/shim paths
provide additional, inferential evidence. Benchlight never runs winget list/show,
refreshes repositories or assumes that a package in an online catalog installed
a local executable. Ordinary MSI installs often have an unknown installation
channel. Evidence, confidence and limitations stay visible.

Project links are established from manifest markers. They show apparent ecosystem
use, not proof a process launched that exact executable. Lists are paginated.
Tools and errors persist locally; refreshing replaces the observation set.
`benchlight tools --refresh --json` observes; `tools` reads saved observations;
`inspect node` or `inspect C:\path\tool.exe` explains metadata. Arbitrary paths
are never executed by inspect. `tool-projects node` lists related projects.

Sources: [Windows uninstall registry](https://learn.microsoft.com/en-us/powershell/scripting/samples/working-with-software-installations),
[Chocolatey package storage](https://docs.chocolatey.org/en-us/why/),
[Scoop source](https://github.com/ScoopInstaller/Scoop),
[Windows job objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[rustup environment controls](https://rust-lang.github.io/rustup/environment-variables.html).


## Interpreting installation origins

Installation channel is an inference from local evidence, not a guarantee of
package-manager ownership. PATH precedence, metadata, probe limits and unknown
results are described above. Benchlight does not contact online catalogs to
establish these origins.
