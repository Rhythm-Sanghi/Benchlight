# Compatibility notes

Published Benchlight 0.1.0 targets Windows 11 x86-64. The release is unsigned
and requires WebView2. A local 0.1.1 candidate is under qualification; it has
not been published. There is no automatic updater.

For published 0.1.0, local review covered native navigation, light/dark themes,
an 800 px window, 200% zoom, keyboard focus, repository scanning, snapshot
comparison and fixture cleanup. For the local 0.1.1 candidate, a disposable
upgrade from the published 0.1.0 installer preserved roots, exclusions, theme,
snapshots and cleanup-plan history. Fresh installation, same-version reinstall,
uninstall and local-data preservation passed. Installed 0.1.1 desktop navigation,
fixture scanning, tool refresh, comparison, cleanup review, settings and restart
were checked.
Windows CI runs the automated checks and builds the installer; see the
[release guide](docs/release.md).

The following still need broader testing:

- Other upgrade paths and interruptions during installation.
- Candidate 0.1.1 at 800 px, 200% zoom, Windows high contrast and spoken output.
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
Snapshots record environment-variable presence, not values. The local 0.1.1
candidate can remove older history after exact confirmation but does not prune
it automatically. See the [README](README.md#scope-and-limits) for feature limits.
