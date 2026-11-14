# Patched nih_log

This is nih_log 0.3.1, the logger NIH-plug depends on, taken from
<https://github.com/robbert-vdh/nih-log> at `0f3819a` ("Bump to version 0.3.1",
the latest release). The license is in `LICENSE` (ISC).

**The one change:** the `atty` crate is gone. `src/target.rs` asks
`std::io::stderr().is_terminal()` (stable since Rust 1.70) instead of
`atty::is(atty::Stream::Stderr)`, and `Cargo.toml` drops the dependency. `atty`
is unmaintained and has an open advisory (RUSTSEC-2021-0145 / GHSA-g98v-hv3f-hcfr:
a potentially unaligned read on Windows). No upstream release removes it.

Both workspaces use this copy through `[patch.crates-io]` (the root
`Cargo.toml` and `web/Cargo.toml`). Drop the copy and the patch lines once a
nih_log release without `atty` exists, or NIH-plug stops depending on it.
