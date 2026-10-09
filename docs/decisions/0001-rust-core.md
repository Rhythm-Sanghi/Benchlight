# 0001: Shared Rust core

Accepted. Desktop and CLI call the same Rust workflows. Analysis in a UI would
make safety rules diverge and prevent reliable CLI automation. Crates correspond
to real domain/platform/storage boundaries rather than individual functions.
