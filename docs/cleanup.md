# Cleanup

Cleanup is explicit and limited to fully measured Rebuildable/Cache directories.
Protected and Review items cannot enter an ordinary plan. A filename alone never
grants deletion authority. Local modifications inside generated directories can
still be valuable; classification is not a promise that their contents are disposable.

Plans persist in SQLite with creation time, path, directory type, logical size,
classification, reason, project relationship, filesystem identity and a tree
metadata fingerprint. Filesystem identity includes NTFS volume, file index and
creation time. The fingerprint covers relative names, identities, types, sizes
and modification times, not source contents. Project manifest/lockfile contents
are hashed with SHA-256 and never stored as plaintext in the plan.

Review a plan, revalidate it, then type `RECYCLE <plan number>` to authorize apply.
CLI examples (replace the path with a directory returned by your storage scan):

```powershell
benchlight cleanup plan 'C:\source\example\node_modules'
benchlight cleanup show 1
benchlight cleanup validate 1
benchlight cleanup apply 1 --confirm 'RECYCLE 1'
benchlight cleanup log 1 --json
```

Apply validates again; confirmation cannot bypass changed evidence. A process lock
prevents concurrent Benchlight cleanups. Targets must be direct project artifacts
or exact standard cache locations on a local fixed NTFS drive. Roots, selected
project directories, Windows/application directories, the profile root, `.git`,
exclusions, metadata, traversals, device/network paths, removable drives and any
reparse point in a target or its ancestors are rejected. Busy regular files are
rejected during validation. Unsupported or unreadable entries abort validation.

Windows handles deny ancestor renames and target writes/replacement during the
last validation. Manifest handles deny concurrent edits. The target is renamed
through its own handle to a unique sibling staging name. The intended staging
path is committed before mutation. Staged metadata is compared again, then
IFileOperation requests recycling with early failure. Its progress sink rejects
permanent-delete operations and success requires the Shell to report a Recycle
Bin item. Benchlight has no permanent deletion implementation.

A failed or interrupted operation may leave a staged directory. The plan and
operation log record both paths; inspect these before restoring. Benchlight never
automatically retries an attempted plan or overwrites a newly created directory.
Windows can restore recycled items; it restores the staging name. Rename it to
the original name recorded in the log after checking that name is available.
Recycle Bin storage depends on Windows and consumes disk until you empty it.
Logical bytes are not a guarantee of space recovered.

This is conservative coordination with ordinary filesystem changes, not a
security boundary against a hostile process running as the same user or as an
administrator. Windows Shell recycling uses paths and has no public handle-only
transaction API. Keep builds/package managers stopped during cleanup. Metadata
fingerprints cannot identify content changes that deliberately preserve every
recorded attribute. Do not run cleanup against actively changing or adversarial
trees. No files outside the validated target are intentionally traversed by the
cleanup operation; Shell namespace junction traversal is never enabled.

Adapter references: [IFileOperation flags](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags),
[PreDeleteItem](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperationprogresssink-predeleteitem),
[PostDeleteItem](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperationprogresssink-postdeleteitem),
[FILE_RENAME_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info).
