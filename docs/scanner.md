# Scanner

Scans traverse explicitly selected roots on a single background worker. Metadata
and lengths are read; source contents are not indexed. Results commit incrementally
to SQLite. The UI receives 100-row pages and aggregated categories. Directory
iterators are capped at 128 levels and error details at 500 entries.

Logical bytes count file lengths. These are not allocated bytes or guaranteed
recoverable space: hard links, compression and sparse files affect actual disk
usage. Overlapping selected roots are collapsed before traversal. Nested project
sizes include nested files; do not sum project sizes as a disk-usage total.

Reparse points and user exclusions are skipped, including during artifact
measurement. Candidates with unreadable/skipped entries or cancellation have an
incomplete measurement. Permission failures do not abort other directories.
Cancellation is cooperative at filesystem entry boundaries and cannot interrupt
a blocked Windows/network filesystem call.

New UNC and mapped-network scan roots are rejected before traversal. Existing
roots from older databases can still fail or block in Windows I/O; remove them
and choose a local folder. Removable local drives can be read, but ordinary
cleanup requires a fixed NTFS drive. Nonresident offline/recall attributes are
skipped even without a reparse bit, to avoid cloud-file hydration. Attribute
rules are tested; a live OneDrive or removable-drive session was not available
for this release's manual review. Names are compared without case on ordinary
Windows volumes; unusual case-sensitive directory configurations can lose
discoveries and are not supported for cleanup claims.

Directory modification times are checked when enumeration finishes. Membership
changes/disappearances report a partial measurement. This is not an atomic
filesystem snapshot; content can change without a directory timestamp changing.

Known global cache locations are opt-in in Settings. Custom tool configurations
are not guessed. Poetry caches and Maven repositories need review because they
may hold environments or locally published artifacts. Generic build/dist folders
also need review. Cargo target requires CACHEDIR.TAG, Maven target maven-status,
and .NET obj project.assets.json in addition to surrounding manifest evidence.

Run scripts/benchmark.ps1 for a temporary 50-project / 20,050-file fixture. It
reports scan duration, 100 paginated database queries, cancellation latency and
sampled process working set (including fixture creation). Measurements are
machine-specific; no general speed claim is made.

Cache references: [pip](https://pip.pypa.io/en/stable/topics/caching/),
[uv](https://docs.astral.sh/uv/concepts/cache/),
[npm](https://docs.npmjs.com/configuring-npm/folders.html/),
[Cargo](https://doc.rust-lang.org/cargo/guide/cargo-home.html),
[Gradle](https://docs.gradle.org/current/userguide/directory_layout.html),
[Poetry](https://python-poetry.org/docs/configuration/).


Existing local exclusions are resolved to canonical paths before comparison,
including Windows 8.3 aliases. Missing exclusions remain configured for future
scans. Network paths and reparse paths are not canonicalized for this purpose.
