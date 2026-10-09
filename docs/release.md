# Windows release verification

Release 0.1.0 targets Windows 11 x86-64, uses the per-user NSIS installer and
bundles the CLI, UI assets, application icon, GPL license and dependency notices.
WebView2 must already be present. There is no updater or runtime asset download.

## Reproduce

1. Install the build prerequisites described in the README; run `npm ci` in
   `apps/desktop`.
2. From the repository root run `scripts/check.ps1`.
3. In `apps/desktop` run `npm run tauri build`.
4. From the root run `scripts/test-installer.ps1 -Installer
   target/release/bundle/nsis/Benchlight_0.1.0_x64-setup.exe`.
5. Run `scripts/source-archive.ps1` after finalizing source and documentation.
   Distribute that archive with the installer and CLI. Preserve license notices.

The installer test refuses to replace an existing user installation. It uses
workspace-local installation and metadata directories, scans the checked-in
fixtures with the installed CLI, reinstalls the same version, checks persisted
roots/scan identity, uninstalls and verifies metadata survives. It also checks
that installed executables match release SHA-256 hashes (allowing only Tauri's
documented UNK-to-NSS bundle marker patch for the desktop executable), license
notices are present, and metadata co-located with the binaries survives uninstall
and remains readable. Review an existing test directory before retrying.
Same-version reinstall is not proof of a different-version upgrade; that needs
another release. SQLite migration tests
cover preservation of earlier metadata and rejection of newer schemas.

## Manual review on 2026-10-07

- Bundled executable opened with Vite stopped and WebView outbound requests
  blocked through test-process-only proxy/DNS arguments. No Windows network or
  privacy setting was changed. First launch displayed folder selection and no
  automatic scan.
- Explicit repository root selection and scan completed with 14 projects and no
  errors. The metadata exclusion left the Rust target directory partial and
  ineligible for cleanup. Fixture projects were discovered as normal directories.
- Native Projects sorting, row inspection, Shift+F10 menu, Escape focus return,
  keyboard resizing and pointer column resizing were reviewed.
- Space evidence/classification, Tools active/shadowed paths and Unknown origins,
  Changes baseline/comparison and light/dark appearance were reviewed. Actual
  screenshots live in `docs/screenshots`.
- Fixture-only native cleanup tests confirmed Shell recycling and rejected locked
  files, replaced directories, changed evidence and junction substitutions.
  No repository build output or dependency directory was recycled in UI review.

These checks ran locally. Windows CI now runs on GitHub Actions; code signing
is not configured.
Live OneDrive hydration, physical removable drives, unusual case-sensitive NTFS
directories and different-version installer upgrades are unverified. See scanner
and cleanup documentation for conservative handling and limits.

## Workshop Grid revision on 2026-10-08

The existing frontend has been redesigned with a fixed compact shell, semantic
light/dark tokens, ruled tables and resizable inspectors. Native review covers
both themes, an 800 px window, scan/comparison flows and keyboard focus return.
The NSIS bundle is rebuilt with these frontend assets. This revision preserves the
backend and packaging behavior; the installer lifecycle review above remains the
2026-10-07 check. See [Workshop Grid](workshop-grid.md) for the detailed UI record.


The UI follow-up saves widths, improves drawer focus and text contrast, and
supports WebView zoom shortcuts. Native 200% review confirms Settings reflow and
Project inspector keyboard reachability. NSIS, matching source and checksums are
refreshed. Spoken screen-reader output and global Windows text scaling remain
separate manual compatibility checks.
