# Cleanup revalidation and recycling

Ordinary cleanup acts only on complete, strongly classified candidates. A saved
plan contains the scan identity, deterministic metadata fingerprint and project
evidence hash. Confirmation is tied to a plan ID. Every item is revalidated;
changed or unsupported paths fail closed.

Windows handles hold ancestor locations stable while the target is renamed by
handle into a unique sibling staging name. The intended recovery path is logged
before mutation. Shell recycling must report a Recycle Bin item; there is no
permanent-delete fallback or automatic crash retry. Recovery retains the
original path and the staging path because Explorer restores the staging name.

This is a narrow Windows implementation. It is deliberately not a generic
transaction abstraction: NTFS, the Shell, active writers and crashes cannot
provide an atomic rollback for a multi-directory plan. The UI and logs report
partial results, and the safety documentation explains these limits.
