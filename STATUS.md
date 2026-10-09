# Project status

Benchlight 0.1.0 is an early release for Windows 11 x86-64. The desktop app, CLI,
installer, source and dependency notices are included. It works locally and does
not scan until you ask. Source and releases live at
[Rhythm-Sanghi/Benchlight](https://github.com/Rhythm-Sanghi/Benchlight).

## What works

- Explicit project folders, cache scope and exclusions; cancellable scans with
  project discovery, logical sizes, error reporting and evidence.
- Storage classifications and persisted cleanup plans with exact confirmation,
  revalidation, conservative Shell recycling and operation logs.
- PATH tools, active/shadowed executables, bounded version probes, installation
  evidence and apparent project relationships.
- Saved working snapshots and comparisons of lockfiles, Git metadata, runtimes
  and environment-variable presence. Unknown observations remain Unknown.
- Sortable tables, saved column/inspector widths, keyboard navigation and context
  menus, light/dark/system themes, zoom shortcuts and manual log export.

## Verification

Local checks cover Rust formatting, Clippy with warnings denied, 38 Rust tests,
frontend formatting/lint/typechecking, 20 frontend tests, 80 contrast pairs and
production builds. Three internal subprocess fixtures are invoked by parent tests.
The Windows workflow repeats these checks and builds the installer.

Native review covered the main pages, both themes, an 800 px window, 200% zoom,
keyboard focus, an explicit repository scan and snapshot comparison. Installer
checks covered installation, same-version reinstallation, uninstall and database
preservation. Cleanup tests used temporary fixtures and checked changed evidence,
locks, replacement identities, junctions and rejected paths. See
[release verification](docs/release.md) for dates and boundaries.

## Known limitations

- The Windows release is unsigned and has no automatic updater.
- Different-version upgrades, live OneDrive hydration, physical removable drives,
  spoken screen-reader output and global Windows text scaling need further review.
- Logical sizes do not guarantee recovered space. Nested project totals can
  overlap; partial measurements cannot authorize cleanup.
- Cancellation waits for in-flight Windows I/O. Unusual case-sensitive directories
  are unsupported; new network and mapped-network roots are rejected.
- Shell recycling is conservative coordination, not an atomic transaction or a
  security boundary against hostile software running as the same user. Recovery
  uses staging names and recorded original paths; read [cleanup](docs/cleanup.md).
- Docker engine accounting and arbitrary custom caches are not implemented.
  Virtual disks and emulators remain conservatively classified.
- Tools reflect the process PATH. Aliases, runtime-manager choices and installation
  origins can remain Unknown. Version probes are bounded, not a sandbox.
- Workspace-inherited Cargo constraints and Python prerelease policy can remain
  Unknown. Snapshots do not read `.env` or store environment values, so they cannot
  detect changes to a value that remains present. History has no retention policy.
