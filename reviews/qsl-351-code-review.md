---
id: SR-935
title: "Code review of QSL-351 (reopened): TerminalValue::Inconclusive, Declined code, typed RequestIndex"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@65e310bc3990a417ba1117e5e610c34304cd7a65; qsl-foundation/src/lib.rs, qsl-foundation/src/request_index.rs, qsl-replay/src/lib.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/result.rs, qsl-route/src/request.rs"
review_set: subset
---

## Summary

Ticket: QSL-351 (reopened, early review, no PR yet; branch
`task/351-inconclusive`, two commits on `3dc4f522c`). Code review with the
rust-review lane over `git diff origin/main...HEAD`. This file replaces the
clean SR-935 review of PR #551 (the ToolPin part, reviewed at `cfef8e79`);
that review stays in git history and in its Linear comment.

The rulings relayed with the dispatch are taken as decided and not
re-litigated here: `TerminalValue::Inconclusive(ReplayInconclusiveCause)`,
vacuity staying `Proved { success_checks: 0 }`, the reporting
`InconclusiveCause`, FR-121's timing key, `Declined { cause, code }`, a typed
`RequestIndex` in `qsl_foundation`, and `ManifestDigest` kept for route
registry conflict detection.

Checked:

- `TerminalValue` and `TerminalRecord` lose `Copy` and `Hash`. Needed:
  `ReplayParity` carries a `DisagreementCause`, whose `Witness` arm boxes
  `SeparatingWitnessRecord`s (a kernel `Value`, strings, a `Vec` path), so
  neither derive can hold. Nothing in the workspace hashed or copied either
  type; `category`, `vacuous_proof_cause` and the accessors move to `&self`
  and `value()` returns `&TerminalValue`. Coherent.
- `RequestIndex` moves to `qsl_foundation` and gains a public `new`. Both
  writers in `qsl-route/src/request.rs` and the reader in
  `proof_result.rs` use the one type, so the request and the terminal record
  join on it. The only remaining in-workspace user outside those is a
  `result.rs` test. `qsl_route::request::RequestIndex` is gone as a path
  (the `use` is private); no workspace code named it.
- `from_replay_refusal`: `Fault` and `Admission(AdmissionFailure::Fault)`
  give `Failed`; every other `ReplayRefusal` gives
  `Inconclusive(ReplayRefused(refusal.code()))`. Matches ADR-013 C-09 and
  the IR confirmation on the ticket.
- `inconclusive_cause` is exhaustive with no `_` arm; `category` still has
  none. The envelope's cause is `Some` exactly for the three inconclusive
  shapes (tc_177 asserts it for all eleven cases).
- Consumer reach: CG depends on `qsl-replay` alone (CG `Cargo.toml`:
  "qsl-replay is the only QSL crate this repository depends on (QSL
  arch-lint T12-A)"). See FND-001.
- Size bound: `measured_encoded_bytes` now adds
  `size_of::<RequestIndex>() + size_of::<TerminalValue>()` per item plus
  `ReplayInconclusiveCause::measured_bytes`, which sums the `given` and
  `derived` witness records of a `DisagreementCause::Witness`. See FND-002
  and FND-003.
- No new `unwrap`, `unsafe`, integer narrowing or ambient read.
- Focused run: `cargo test -p qsl-replay --lib proof_result` under
  `locked-build.sh` (result in the Verdict).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | CG, the C-09 producer of terminal records, depends on `qsl-replay` alone (arch-lint T12-A), but `qsl-replay` re-exports neither `RequestIndex` nor `Code`. CG cannot call `TerminalRecord::new(RequestIndex::new(i), ..)` at all, and cannot name `Code` for `Declined { cause, code }` literals or its own signatures. Add `pub use qsl_foundation::RequestIndex;` and `pub use qsl_foundation::diagnostic::Code;` to the `qsl-replay` root, with an outside-the-crate test that builds a `TerminalRecord` through root paths only | qsl-replay/src/lib.rs:61-64, qsl-replay/src/proof_result.rs:280 |
| FND-002 | medium | `ReplayInconclusiveCause::measured_bytes` counts the two witness records but not `DisagreementCause::Witness.failure`: `WitnessFailure::Separation { reason }` carries free-length data in two arms: `UndefinedEvaluation { expression: Location, cause: String }` (a `Vec<usize>` path and a string) and `Refused(SeparationRefusal { code: String, cause: String, fields: BTreeMap<String, String> })`. None of it is in `size_of::<TerminalValue>()`, so an `Inconclusive(ReplayParity)` record can carry unmeasured content past `MAX_ENCODED_BYTES`, against the function's own B3 claim ("never a caller-declared number a source could understate") | qsl-replay/src/proof_result.rs:126-136, qsl-replay/src/result.rs:147-188 |
| FND-003 | medium | No test reaches the new witness-byte measurement. tc_178's oversized source now inflates `backend_identity` only, and tc_177 uses the `Verdicts` parity arm, which measures 0. Replacing `ReplayInconclusiveCause::measured_bytes` with `0` passes every test. Add an oversized case whose bytes sit in a `DisagreementCause::Witness` record (and, after FND-002, in an `UndefinedEvaluation` cause and a `SeparationRefusal`) | qsl-replay/src/proof_result.rs:630-642 |

## Verdict

Changes requested. The type changes themselves are needed and coherent:
dropping `Copy`/`Hash` follows from carrying a `DisagreementCause`, the
`&self`/`&TerminalValue` API is the right consequence, and one
`RequestIndex` shared by the writer and the record is what the ticket asked
for. What is missing is reach and the bound: CG cannot construct a
`TerminalRecord` through the only QSL crate it may depend on (FND-001), and
the replay-parity size measurement is incomplete and untested (FND-002,
FND-003).

Focused run at the reviewed sha: `cargo test -p qsl-replay --lib proof_result`
through `locked-build.sh`: 6 passed, 0 failed (tc_177, tc_178, tc_179,
tc_769, the FR-121-AC-16 test, `to_source_refuses_an_empty_envelope_set`).

## Dispositions

Disposition pass 1, reviewed at `43a2844c6d8c37d0b6c4eea2d04c5ea4d297234a`
(range `65e310bc..43a2844c`). Focused run through `locked-build.sh`:
`cargo test -p qsl-replay --lib --test terminal_record_facade` filtered to
`proof_result`, `disagreement_cause` and `terminal_records`: 10 passed, 0 failed.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 43a2844c: `pub use qsl_foundation::{Code, RequestIndex};` at the `qsl-replay` root; `qsl-replay/tests/terminal_record_facade.rs` builds and reads terminal records through root paths only |
| FND-002 | fixed | 43a2844c: `InconclusiveCause::measured_bytes` delegates to `DisagreementCause::measured_bytes`, which adds `WitnessFailure::measured_bytes` (origin name, location depth, undefined cause, refusal code, cause and fields) |
| FND-003 | fixed | 43a2844c: `tc_178_refuses_an_oversized_replay_parity_cause` (bytes only in a `SeparationRefusal` cause) and `a_disagreement_cause_measures_its_records_and_failure` (each arm against independently summed totals) |
