# Filesystem fixtures

These manifests are detection inputs, not runnable sample applications. Tests
copy or create them in temporary directories. Junctions are created at test time
because Git cannot store an NTFS junction. Cleanup tests must never use these
checked-in files as deletion targets.
