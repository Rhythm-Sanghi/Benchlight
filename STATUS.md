# Compatibility notes

Benchlight 0.1.0 targets Windows 11 x86-64. The release is unsigned and requires
WebView2. There is no automatic updater.

Tested locally: native navigation, light/dark themes, an 800 px window, 200% zoom,
keyboard focus, repository scanning, snapshot comparison and fixture cleanup.
Installer checks covered installation, same-version reinstall, uninstall and
preservation of the local database. Windows CI runs the automated checks and
builds the installer; see the [release guide](docs/release.md).

The following still need broader testing:

- Upgrading from a different version.
- Live OneDrive hydration and physical removable drives.
- Spoken screen-reader output and global Windows text scaling.

Unusual case-sensitive directories are unsupported. Scans reject new network and
mapped-network roots and skip links and nonresident cloud placeholders.
Cancellation waits for an in-flight Windows read.

Logical file sizes do not guarantee recovered space, and nested project totals
can overlap. Partial measurements cannot authorize cleanup. Read the
[cleanup and recovery guide](docs/cleanup.md) before recycling directories.

Tool discovery reflects the application's PATH, so aliases and runtime-manager
choices may differ from your shell. Some installation origins, workspace-inherited
Cargo constraints and Python prerelease requirements can remain Unknown.
Snapshots record environment-variable presence, not values, and history has no
retention policy. See the [README](README.md#scope-and-limits) for feature limits.
