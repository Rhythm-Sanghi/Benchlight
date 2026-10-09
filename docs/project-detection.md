# Project discovery

Projects are recognized from manifest names (.NET solution/project extensions
included) and .git directories or worktree files. Evidence is retained verbatim.
Filename comparisons are case insensitive on the Windows target. No source file
contents are parsed. Language labels describe manifests, not a source-code census.

Each selected root is traversed on one worker. Directory iterators are bounded
to 128 levels. Reparse points are skipped, as are .git, node_modules, Python
environments and __pycache__ subtrees. Errors continue the scan and the first 500
details are retained. Counts include all reported errors, even after that limit.
Filesystem activity is the newest immediate child timestamp; it does not prove
a project is actively used and is not recursive source activity.

Results are committed incrementally to SQLite, queried in pages of at most 200,
and progress is persisted at most every 200 ms. Desktop polling has one request
in flight. Cancellation is checked during directory enumeration and traversal.
An OS file lock prevents competing CLI/desktop scans; a crashed process releases
the lock and its unfinished run is marked interrupted on the next scan.

Cancellation retains partial results. Opening the app does not scan. A selected
network or removable root can disappear or block in Windows filesystem calls;
cancellation cannot preempt a blocked OS read. Last-scan state is always shown.
