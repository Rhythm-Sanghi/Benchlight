# Security

Report vulnerabilities privately through
[GitHub security advisories](https://github.com/Rhythm-Sanghi/Benchlight/security/advisories/new).
Private vulnerability reporting is enabled for this repository. Please do not
open a public issue with exploitable details, credentials or private paths.

Include the affected version, Windows version, steps to reproduce, expected
behavior and the impact you observed. A minimal example using temporary folders
is especially helpful. Remove personal or project data from any attached logs.

Benchlight 0.1.x is the current supported early-release line. There is no
promised response time; please allow time for investigation before disclosure.

Path traversal, junction handling, cleanup revalidation and secret leakage are
security issues. Cleanup requires explicit confirmation and is limited to local
fixed NTFS drives. Its guards are not a security boundary against hostile
software running as the same user. Read [cleanup and recovery](docs/cleanup.md)
for concurrency limits and recovery steps.
