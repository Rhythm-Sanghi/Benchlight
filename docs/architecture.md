# Architecture

The desktop uses React for presentation and a thin Tauri command boundary. CLI
and desktop both call benchlight-core. The core owns workflows and calls the
database, storage and Windows crates. There is no backend server.

SQLite stores metadata, not file contents. Connections enable foreign keys,
WAL and a five-second busy timeout. Migrations run transactionally and a newer
schema is rejected rather than modified by an older application.

Scanning runs on one worker with cooperative cancellation and a process-wide
filesystem lock. Cleanup, tool refreshes and snapshot capture run on Tauri's
blocking workers with their own SQLite connection. The frontend receives
summaries and bounded pages, never a full filesystem tree. Project and artifact
sorting occurs in SQLite before pagination; column widths and selection stay in
the presentation layer.

Working-state history is explicit. Comparisons preserve the last user-marked
baseline. Runtime declarations and difference classification belong to Rust;
React renders the observations without diagnosing causation. No watcher,
network client or background cleanup workflow is started.
