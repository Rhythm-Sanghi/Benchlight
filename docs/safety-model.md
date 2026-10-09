# Safety model

Root selection is explicit. Opening Benchlight does not start a scan. Roots must
be absolute regular directories, and cannot themselves be reparse points.

Reparse points are excluded from traversal, including junctions and OneDrive
placeholders. An incomplete scan must report partial failures. No scanner may
delete or modify inspected files.

Cleanup uses persisted plans, explicit review/confirmation, hostile path tests,
filesystem revalidation, Windows Recycle Bin support and operation logs. Ambiguous
directories remain protected. See [cleanup.md](cleanup.md) for the exact path
rules, staging/recovery procedure, supported drives and concurrency limits.
