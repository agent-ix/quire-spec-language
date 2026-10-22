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

Verify the checker's verdict for each NFR-016 finding kind over fixture
inputs, and that an unsafe block in a first-party crate fails the real gate.
Scope: NFR-016-AC-2 to NFR-016-AC-7.

Each fixture is a scan report in `cargo geiger --output-format Json` form, a
closure in `cargo metadata` form and an `unsafe-inventory.toml`. The base
fixture is a closure of `quire-eval`, `quire-checked`, `quire-exact`,
`quire-walk` and `quire-canonical` (all declaring `unsafe_code = "forbid"`
and reporting no unsafe code), the third-party crate `big` reached through
`quire-exact` and the third-party crate `fmt` reached through
`quire-canonical` (each reporting one unsafe expression), the third-party
crate `plain` reached through `quire-exact` (reporting none), the proc-macro
crate `derive` and the crate `tokens` reached only through `derive`, with
well-formed entries for `big` and `fmt`.

## Test Procedure

1. Run the checker on the base fixture.
2. Remove `quire-walk`'s forbid declaration and run it.
3. Give `quire-checked` one unsafe expression and run it; then also add an
   entry for `quire-checked` and run it again.
4. Remove the entry for `big` and run it; then restore it with
   `reached_through = ["quire-exact"]` and run it again.
5. Add an entry for a crate `absent` that is not in the closure and run it;
   separately add an entry for `plain` and run it; separately set `big`'s
   `reached_through` to `["quire-canonical"]` and run it.
6. Separately: add a `version` key to `big`'s entry; set `fmt`'s
   `acceptance` to `""`; duplicate `big`'s entry; swap the order of the two
   entries. Run the checker on each.
7. Give `derive` and `tokens` one unsafe expression each and run it; then
   add an entry for `derive` and run it.
8. Mark `fmt` as a package the scan reports without metrics and run it.
9. In a scratch copy of `agent-ix/quire-eval`, add `unsafe {}` to a
   function body in `quire-eval`'s source and run `make audit-unsafe`.

## Expected Results

1. No finding; the checker exits zero.
2. `first-party-not-forbidden quire-walk`; non-zero exit.
3. `first-party-unsafe quire-checked`, both times; non-zero exit.
4. `unlisted-unsafe big` naming `quire-exact`, non-zero exit; then no
   finding and exit zero.
5. `stale-entry absent`; `stale-entry plain`; `reached-through-mismatch big`;
   each with a non-zero exit.
6. `malformed-entry` for each of the four inventories, each with a non-zero
   exit.
7. No finding and exit zero; then `stale-entry derive` and a non-zero exit.
8. `unscanned fmt`; non-zero exit.
9. The target exits non-zero.
