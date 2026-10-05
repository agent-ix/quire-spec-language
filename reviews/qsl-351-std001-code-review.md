---
id: SR-1301
title: "Code review of PR #634 (QSL-351: DeclineCode::Std001, IR bump to 3be6ff70)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6725aa7a610aa33a18f3c2d76443018dd10259ff; Cargo.lock, qsl-replay/Cargo.toml, qsl-replay/src/lib.rs, qsl-replay/src/proof_result.rs, qsl-replay/tests/terminal_record_facade.rs"
review_set: subset
---

## Summary

Ticket: QSL-351. PR agent-ix/quire-spec-language#634, one commit
(`6725aa7a6`) on main `02530e7a5`. Code review with the rust-review lane
over `git diff origin/main...HEAD`. Rulings taken as decided: the `Std001`
arm names the STD-001 registry, records no issuer and refuses no
unregistered code; the IR bump to IR #292 (`3be6ff70`) with QVC at
`ec4563ff`.

Checked:

- **The new `qsl-replay` → `quire-contract-model` edge.** It adds no
  crate and no repository edge to the graph: `qsl-package` (layer 4)
  already depends on `quire-contract-model` under ADR-011 §6.1's layer-4 row,
  and `qsl-replay` (layer 6) already depends on `qsl-package`. The PR
  amends the layer-6 row to name it. `arch-lint-qualified-core` (in
  `make ci`) lists `quire-contract-model` as an allowed core dependency
  and passes in the coordinator's log; the CG-side T12-A rule ("qsl-replay
  is the only QSL crate CG depends on") is unaffected because CG still names
  one QSL crate. Allowed, and the right route: the alternative (a QSL-side
  copy or newtype of the code type) would be a second definition of IR's
  registry type.
- **Re-exports.** `pub use quire_contract_model::{std001_code, Std001Code}`
  at the `qsl-replay` root. `std001_code!` expands through
  `$crate::Std001Code`, which resolves to `quire_contract_model` whether or
  not the caller depends on it, so CG can build both arms through
  `qsl_replay` alone. `terminal_record_facade.rs` imports both from the
  root and builds a `Std001` arm with each.
- **`Std001Code` as a field of a `Copy` enum.** At `3be6ff70` it is a
  fixed inline buffer (`[u8; MAX_CODE_BYTES]`), `Copy`, no allocation and
  no leak on the run-time `new` path, so `DeclineCode` keeps `Copy` and the
  envelope's `size_of::<TerminalValue>()` measurement stays a true bound
  for it.
- **Lock.** The diff changes two lines: `qsl-replay`'s dependency list and
  `quire-contract-model`'s source rev. Every first-party crate
  (`quire-contract-model`, `quire-canonical`, `quire-canonical-derive`,
  `quire-verification-contracts`, `quire-rs`, `quire-walk`,
  `ix-trace-rs`) has one entry. The third-party duplicates (`jsonschema`,
  `syn`, `hashbrown`, ...) are on main unchanged.
- No new `unwrap` outside tests, no `unsafe`, no integer conversion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | tc_177's category table adds a `Declined` row with code `Std001Code::KANI_VACUOUS_PROOF`. ADR-013 O-16 makes a vacuous proof `inconclusive` (`kani_vacuous_proof`), never `declined`, so the fixture pairs the one code that must not settle `declined` with `Declined`. Use a STD-001 refusal code | qsl-replay/src/proof_result.rs:518-525 |

## Verdict

Approve with one low finding. The dependency is allowed by ADR-011's
layer rules and the qualified-core lint, and reaches `qsl-replay` the
right way (IR's type, re-exported; no copy). The re-exports give CG both
constructors through the one crate it may name, and the lock holds one copy
of each first-party crate.

Focused run at the reviewed sha through `locked-build.sh`: `cargo test --locked -p qsl-replay --lib --test terminal_record_facade` filtered to `tc_177`, `std001` and `terminal_records`: 3 passed, 0 failed.

## Dispositions

Disposition pass 1, reviewed at `00ad945c1` (range `6725aa7a..00ad945c`), no
build (the pre-merge `make ci` is the lead's).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 00ad945c: tc_177's STD-001 `Declined` row uses `Std001Code::KANI_BOUND_INVALID` (registered `kani_bound_invalid`, a refusal code) in place of `KANI_VACUOUS_PROOF` |
