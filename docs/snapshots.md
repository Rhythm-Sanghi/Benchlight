# Working states

Mark as working records the user's assertion that a scanned project works. It
does not run tests or certify a usable environment. Capture is explicit; there is
no watcher. Every mark creates an immutable SQLite record. Compare reads the most
recent mark and captures current metadata without replacing that baseline.

The shared Rust core captures root-level known lockfile SHA-256 hashes (maximum
32 MiB per file), current process PATH tool paths/versions, Git branch/commit and
counts of tracked worktree changes, and presence of NODE_ENV, VIRTUAL_ENV,
JAVA_HOME, RUSTUP_TOOLCHAIN and DATABASE_URL. Values, .env files, file contents and
Git filenames are not persisted. Presence belongs to Benchlight's process: a
different terminal or virtual environment can have a different environment.
Present-to-present value changes are deliberately not detected.

Tool collection uses Tools' fixed bounded commands from the metadata directory,
not project scripts. It does not resolve a project's version-manager selection,
virtual-environment packages or nested lockfiles. Git uses fixed argument arrays,
disabled filesystem monitors, optional locks and untracked-file enumeration;
commands time out after five seconds and output is limited to 16 KiB. Failed
commands produce unknown fields. No raw command output is saved as a log.

package.json engines.node/npm use npm-compatible ranges; pyproject.toml
project.requires-python uses PEP 440 for stable Python releases; Cargo.toml
package.rust-version is a minimum. Inherited workspace declarations and Python
pre-release policy remain explicitly unknown. Malformed manifests do not produce
a compatibility claim. A mismatch is evidence, not proof of the cause of a
failure. Unsupported runtime declarations are not interpreted.
Manifest reads are limited to 1 MiB and individual declared ranges to 512 bytes;
oversized declarations remain Unknown rather than being sent to the UI.

Files are bounded, regular, without reparse ancestors, and opened denying writers
and deletion while read. Snapshots are observations taken over time, not atomic
machine images. Unreadable fields compare as Unknown, never Unchanged. Local
working-state history currently has no retention policy.

CLI `mark-good <project> --health-executable C:\path\cargo.exe --health-arg test`
explicitly runs that native executable in the project before marking. A failure,
two-minute timeout or 16 KiB output limit prevents a new baseline. Arguments and
output are not stored; only success is recorded. No shell wrappers are accepted.
Comparison does not repeat the command and reports its current result as Unknown.
The desktop Mark action records your assertion without executing a health command.

CLI: `mark-good <project>`, `working-state <project>` and `changes <project>`.
All use the same core as the Changes screen. CLI JSON can be redirected manually;
it contains local paths and should be reviewed before sharing.
