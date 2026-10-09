# Dependency decisions

Cargo.lock and package-lock.json record exact resolved versions. The desktop
uses Tauri 2 and bundled UI assets.

| Dependency | Purpose | License |
| --- | --- | --- |
| Tauri 2 / tauri-build / JS API and CLI | Windows shell and IPC | MIT OR Apache-2.0 |
| React / React DOM | UI | MIT |
| Vite / React plugin | Local asset build | MIT |
| TypeScript 6.0.3 | Type checking | Apache-2.0 |
| rusqlite / bundled SQLite | Local metadata | MIT / public domain SQLite |
| serde / serde_json | IPC and CLI serialization | MIT OR Apache-2.0 |
| thiserror | Domain errors | MIT OR Apache-2.0 |
| clap | CLI arguments | MIT OR Apache-2.0 |
| tempfile | Isolated tests | MIT OR Apache-2.0 |
| winreg | Read-only Windows registry metadata | MIT |
| windows / windows-core | NTFS identities, handles, COM recycling | MIT OR Apache-2.0 |
| sha2 0.11 | Metadata and manifest fingerprints | MIT OR Apache-2.0 |
| node-semver 2.2 | npm-compatible runtime ranges | Apache-2.0 |
| pep440_rs 0.7 | Python runtime specifiers | Apache-2.0 OR BSD-2-Clause |
| toml 1.1 | Bounded runtime declarations | MIT OR Apache-2.0 |
| ctrlc 3.5 | Cooperative CLI scan cancellation | MIT OR Apache-2.0 |
| Prettier 3.9 | Development source formatting | MIT |
| ESLint / typescript-eslint / React hooks plugin | Development linting | MIT |
| Vitest / jsdom / Testing Library | Development tests | MIT |

These licenses are compatible with GPL-3.0-or-later. Distribution must preserve
third-party notices. [The generated inventory](third-party/README.md) includes
resolved Cargo packages and production JavaScript packages, with original
available license/notice files. `scripts/notices.ps1` regenerates it. The installer
ships the inventory, notices and Benchlight's GPL license.
TypeScript is pinned to the version supported by the locked lint toolchain.

Sources: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/),
[Tauri commands](https://v2.tauri.app/develop/calling-rust/),
[React versions](https://react.dev/versions), [Vite guide](https://vite.dev/guide/),
[rusqlite](https://docs.rs/rusqlite/0.40.2/rusqlite/),
[TypeScript](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-6-0.html).
Windows bindings and hash APIs were checked against
[windows-rs](https://github.com/microsoft/windows-rs),
[winreg](https://docs.rs/winreg/0.56.0/winreg/) and
[RustCrypto sha2](https://docs.rs/sha2/0.11.0/sha2/).
