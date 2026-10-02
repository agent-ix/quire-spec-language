---
id: SR-1215
title: "Code review of quire-spec-language PR #599: arch-lint qualified-core (FR-284)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b4388c0301217c36a4e3d00665afd5b32d7f817b; PR #599 diff against origin/main: tools/arch-lint/qualified_core.rs, tools/arch-lint/api_surface.rs, tools/arch-lint/main.rs, tools/arch-lint/Cargo.toml, Makefile, FR-280, FR-284, TC-768"
review_set: subset
---
# Code review of quire-spec-language PR #599

## Summary

Ticket: QSL-594. The PR adds `arch-lint qualified-core`, a direction check
plus an ambient-input scan over the 12 QSL core crates, and the Makefile
target `arch-lint-qualified-core`, which is outside `ci:` by ruling.

What the coordinator asked to check:
- **Direction walk: sound for what it states.**
  - It is a BFS from each core workspace member over normal edges only
    (`dep_kinds` with `kind: null`), so it is transitive.
  - It does not enter proc macros. That is correct, since they link
    nothing; qsl-attrs, a proc-macro workspace member, is therefore not
    flagged.
  - It walks through allowed crates and stops at the first offender,
    naming the chain.
  - A missing core crate is a usage error, not a pass.
  - Resolution uses `--all-features --locked`.
  - Running it on this head reports exactly the two known violations and
    nothing else: clap via `agent-ix-extraction-frontend`, and
    `model/intake.rs:223`.
  - One way a real violation can slip past is the host-only platform
    filter (FND-003).
- **Ambient scan.** It catches each category FR-284 lists in the spellings
  it names, and the synthetic test checks each category's line. On the real
  core it produces no false positives, so test code, comments, strings,
  `env!`, immutable statics and atomics are correctly excluded. Coverage
  gaps are FND-001; latent false positives are FND-002.
- **Shared `api_surface.rs` changes do not weaken the existing checks.**
  - `cfg_test_lines` now also excludes `#[cfg(test)]`-gated item macros.
    That only removes non-shipped lines from what T12 scans, and every T12
    rule is `shipped_only`.
  - `has_cfg_test` is unchanged. A `cfg(any(test, kani))` item is still
    scanned, which is the conservative choice.
  - `CallPattern::macro_invocation` is additive, and no existing rule uses
    it.
  - All 99 arch-lint tests pass.
- **Synthetic graphs as the TC-767 oracle: adequate.**
  - The walk logic is pure over `DepGraph`. `parse_graph` is tested
    separately on a `cargo metadata`-shaped document (members, normal
    edges, proc-macro targets).
  - The live run gives a real-workspace cross-check. Its two findings are
    exactly the ones the Status names, and its `via` chains are correct.
  - While main fails, a live-tree test could only pin those findings, and
    the ruling rejects an allow-list.
- Rust lane (rust-review): no `unwrap` on input paths (`previous[&cursor]`
  is always populated by the walk). The errors are typed, and clippy
  `-D warnings` is clean.

## Verdict

Changes requested: one medium finding and two low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two ambient reads FR-284 names slip past the scan. (1) Filesystem and search-path reads through `Path`/`PathBuf` methods (`.exists()`, `.is_file()`, `.is_dir()`, `.canonicalize()`, `.read_dir()`, `.metadata()`, `.read_link()`) match no pattern, because only `std::fs`, `fs::…(` and `File::…` are listed. (2) A global registry held in a `static` of `OnceLock`, `OnceCell` or `LazyLock` over a map or vector, set or filled at run time, is not a `MUTABLE_STATIC_TYPES` type unless it also names `Mutex`/`RwLock`/`Cell`. Add method patterns for (1). For (2), either flag `static … OnceLock`/`OnceCell` (a compile-time `LazyLock` table can stay allowed), or state both as limitations in the module doc beside the existing ones. | tools/arch-lint/qualified_core.rs:521-538; tools/arch-lint/qualified_core.rs:568-570; tools/arch-lint/qualified_core.rs:33-40 |
| FND-002 | low | `Path("dirs")` and `Path("tempfile")` compile to a single identifier token, so any identifier `dirs` or `tempfile` matches, such as a local `let dirs = …`, a field, or a function. Macro patterns also match a comparison like `print != x`, because `!=` lexes as `!` then `=`. Today's core has no such false positive, but the first `dirs` variable will fail the gate. Spell them `dirs::` and `tempfile::` (a path prefix), and require the macro's `!` not to be followed by `=`. | tools/arch-lint/qualified_core.rs:535-536; tools/arch-lint/api_surface.rs:591-596 |
| FND-003 | low | `--filter-platform <host>` drops every dependency gated on a non-host target. A core crate adding `[target.'cfg(windows)'.dependencies] clap = …` passes on a Linux run, and the module doc names only `cfg(loom)` as what the filter drops. Also, `clap_derive` (a proc macro) is listed in `FRONTEND` but can never be reported, since proc macros are skipped first. Either walk the union over the targets QSL ships for (host plus the no_std `thumbv7em-none-eabi`), or state the host-only limitation; and drop the unreachable `clap_derive` entry. | tools/arch-lint/qualified_core.rs:342-360; tools/arch-lint/qualified_core.rs:117-125 |
