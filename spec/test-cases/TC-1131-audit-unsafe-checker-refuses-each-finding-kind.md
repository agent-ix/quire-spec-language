---
id: TC-1131
title: "The audit-unsafe checker refuses each finding kind on fixtures"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-016
    type: verifies
---
# TC-1131: The audit-unsafe checker refuses each finding kind on fixtures

## Description

Verify the checker's verdict for each NFR-016 finding kind and declaration
form over fixture inputs, and that an unsafe block in a first-party crate's
shipped source fails the real gate through detection. Scope: NFR-016-AC-2 to
NFR-016-AC-7.

Each fixture holds the checker's four inputs:

- a closure in `cargo tree --prefix depth` form, which keeps each
  package's dependency edges;
- a scan report in `cargo geiger --output-format Json` form;
- for each first-party crate, a source tree: its `Cargo.toml`, the root
  manifest of the workspace it belongs to, and its source files;
- an `unsafe-inventory.toml`.

The base fixture is a closure of `quire-eval`, `quire-checked`,
`quire-exact`, `quire-walk` and `quire-canonical`, each with a source tree
whose manifest declares `[lints.rust] unsafe_code = "forbid"` and whose
shipped source holds no unsafe item. It also has the third-party crate `big`
reached through `quire-exact` and the third-party crate `fmt` reached through
`quire-canonical` (each with one scanned unsafe expression), the third-party
crate `plain` reached through `quire-exact` (none), and well-formed entries
for `big` and `fmt`. The scan also lists the proc-macro crate `derive` and
the crate `tokens` reached only through it, neither of which is in the
closure.

## Test Procedure

1. Run the checker on the base fixture.
2. Change `quire-walk`'s manifest, one variant per run:
   (a) `unsafe_code = { level = "forbid", priority = 1 }`;
   (b) `[lints] workspace = true` under a workspace root, laid out as a git
   checkout, whose `[workspace.lints.rust]` sets `unsafe_code = "forbid"`;
   (c) `[lints] workspace = true` under a root with no `unsafe_code` lint,
   and `#![forbid(unsafe_code)]` in `src/lib.rs`;
   (d) `unsafe_code = "deny"` only;
   (e) no `unsafe_code` declaration and no crate-root attribute.
3. Add `mod raw;` to `quire-checked`'s `src/lib.rs` with `src/raw.rs` holding
   `fn f() { unsafe {} }`, and run the checker; separately declare it as
   `#[cfg(any())] mod raw;` and run it; separately declare it as
   `#[path = "gen/raw.rs"] mod raw;` with the file at `src/gen/raw.rs` and
   run it; separately add `include!("gen/table.rs");` to `src/lib.rs` with
   `src/gen/table.rs` holding the function and run it; separately declare
   `#[path = "gen/missing.rs"] mod missing;` with no such file and run it;
   separately put the same function in `quire-checked`'s `tests/raw.rs` only
   and run it.
4. Remove the entry for `big` and run the checker; then restore it and run
   it again.
5. Add an entry for a crate `absent` that is not in the closure and run it;
   separately add an entry for `plain` and run it.
6. Separately: add a `version` key to `big`'s entry; add a `reached_through`
   key to it; set `fmt`'s `acceptance` to `""`; duplicate `big`'s entry; swap
   the order of the two entries; add a well-formed entry named
   `quire-checked`. Run the checker on each.
7. Give `derive` and `tokens` one unsafe expression each and run it; then
   add an entry for `derive` and run it.
8. List `fmt` under `packages_without_metrics` and run the checker;
   separately remove `plain` from the scan's `packages` and run it;
   separately list a `serde` `out/private.rs` file under
   `used_but_not_scanned_files` and run it.
9. In a scratch copy of `agent-ix/quire-eval`, add `#[cfg(any())] mod
   probe;` to `quire-eval`'s library root, add `probe.rs` holding
   `fn f() { unsafe {} }`, and run `make audit-unsafe`.

## Expected Results

1. No finding; the checker exits zero.
2. (a), (b) and (c): no finding and exit zero. (d) and (e):
   `first-party-not-forbidden quire-walk` and a non-zero exit.
3. `first-party-unsafe quire-checked` naming `src/raw.rs` for the first two
   runs, `src/gen/raw.rs` for the third and `src/gen/table.rs` for the
   fourth, each with a non-zero exit; `unscanned quire-checked` naming
   `src/gen/missing.rs` and a non-zero exit for the fifth; no finding and
   exit zero for the sixth.
4. `unlisted-unsafe big` naming `quire-exact` and a non-zero exit; then no
   finding and exit zero.
5. `stale-entry absent`; `stale-entry plain`; each with a non-zero exit.
6. `malformed-entry` for each of the six inventories, each with a non-zero
   exit.
7. No finding and exit zero; then `stale-entry derive` and a non-zero exit.
8. `unscanned fmt` and a non-zero exit; `unscanned plain` and a non-zero
   exit; no finding and exit zero.
9. The output holds `first-party-unsafe quire-eval` naming `probe.rs`, and
   the target exits non-zero.
