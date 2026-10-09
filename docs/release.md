# Windows releases

Benchlight uses a per-user NSIS installer for Windows 11 x86-64. It bundles the
desktop app, CLI, UI assets, license and dependency notices. WebView2 must already
be installed. Releases are currently unsigned.

## Build and verify

1. Install the prerequisites in the [README](../README.md#build-and-check) and
   run `npm ci` in `apps/desktop`.
2. From the repository root, run `scripts/check.ps1`.
3. In `apps/desktop`, run `npm run tauri build`.
4. From the root, run `scripts/test-installer.ps1 -Installer
   target/release/bundle/nsis/Benchlight_0.1.0_x64-setup.exe`.
5. Run `scripts/source-archive.ps1` against the finalized source. Distribute the
   matching archive and dependency notices with the binaries.

The installer test uses workspace-local directories and refuses to replace an
existing user installation. It checks fixture scans, same-version reinstall,
executable hashes, license notices, uninstall and database preservation. The
hash check allows Tauri's documented UNK-to-NSS bundle marker patch. Review an
existing test directory before retrying.

## Manual checks

Before a release, review navigation, both themes, narrow windows, keyboard focus,
zoom, an explicit scan and a saved snapshot comparison. Test cleanup only on
throwaway fixture directories, including changed evidence, locks and junctions.
Check that the bundled app opens without the development server or network access.

For 0.1.0, installer lifecycle, fixture recycling and offline operation were
reviewed on October 7, 2026. The rebuilt interface was reviewed on October 8,
including an 800 px window and 200% zoom. Different-version upgrades and the
remaining platform checks are listed in [compatibility notes](../STATUS.md).

## Publish

Write release notes in `docs/releases/<version>.md`, run the checks and push a
`v<version>` tag matching the package version. The Windows release workflow checks
and builds that commit, creates the source archive and SHA-256 checksums, uploads
five assets to a draft and publishes an early-release prerelease.

`scripts/publish-release.ps1` does not overwrite existing releases. If an upload
fails, inspect the draft before retrying. Keep published tags and their matching
source archives unchanged; use a new version for subsequent binary releases.
