---
id: SR-60036
title: "QSL-1102 slice 1 code and Rust review of the checked-package V2 writer contract (quire-spec-language PR 1609)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ea1b1d324581bc0de6c3676bdcf2c15028fad802; git diff origin/main (6 files, +62/-10): Cargo.lock, qsl-package/src/emit.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/admission_corpus.rs, qsl-package/src/emit/tests/refinement_rows.rs, src/population_fixtures.rs; context: quire-contract-ir@ff5c66dd checked_package/v2/refinement.rs, v2/mod.rs, v2/intake.rs; quire-contract-ir@f3a8c1c9 v2/protocol_body.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
---

## Summary

Ticket: QSL-1102 (PR quire-spec-language#1609, slice 1). The PR adopts IR's
`CheckedPackageSourceCorrespondenceRef` writer argument at the two
`encode_checked_package_v2` call sites in `qsl-package/src/emit.rs` with
`refinement_step_rows: None`, passes `base.refinement_step_rows()` through in
`src/population_fixtures.rs`, classifies `Relation(Refinement)` as not emitted
by QSL in the admission corpus, and adds
`a_package_without_a_refinement_writes_no_step_row_member`. Cargo.lock moves
only `quire-contract-model` (quire-contract-ir) 6f6b5621 to ff5c66dd (IR #564).

Verified:

- `git diff origin/main -- Cargo.lock Cargo.toml` is exactly that one rev move;
  Cargo.toml is untouched.
- IR at ff5c66dd: `CheckedPackageSourceCorrespondenceRef.refinement_step_rows`
  is "absent when no steps exist". The reader (refinement.rs step-row
  validation) accepts `None` when the refinement-node step count is 0, and
  refuses both `Some(rows)` with count 0 and `None` with count above 0 as
  `invalid_package`/`invalid-value` at `/refinement_step_rows`. intake.rs
  refuses a root member present while the typed field is absent. The writer
  encodes the member `optional`, so `None` writes no member. `None` is
  therefore correct for every package QSL emits today: S3 refuses every
  `refinement` declaration (`CheckCause::RefinementNotEmitted`, qsl-semantics
  check/refinement.rs:3028), so no refinement node is written.
- Mutation: both call sites changed to `Some(&[])`. The new test fails at
  refinement_rows.rs:30 (an empty array is written). Reverted, tree clean.
  The test builds a real package through `emit_checked`, so it discriminates
  the writer argument, not a constant.
- Builds: `cargo build --workspace --locked` and `--all-features --locked`
  are clean. `cargo clippy --workspace --all-targets --locked -- -D warnings`
  and the `--all-features` variant are clean. `cargo test -p qsl-package
  --locked` passes 307 of 307 (plus the 3 and 12 in the other targets).
- No `#[allow]`, no shim, no skipped or ignored test, no string-edge site in
  the diff.
- Aggregate gate `make ci` at ea1b1d324 (origin/main b052a1b7f is an
  ancestor; #1608 not merged at run time): the only red is `string-edge` with
  qsl-semantics/src/check/terminal.rs:235, 236 (`TerminalNode::of`) and 255
  (`CheckedGraph::terminal_nodes`). The PR does not touch terminal.rs, so
  these are inherited from main (terra #1595; fix is #1608). Every lane the
  gate stops before (route-lint, checked-input, refinement-versioning,
  cargo-deny-bans, ci-docs, the three arch-lint lanes, build-no-std) was run by
  hand and exits 0. The `error:` lines in the log come from xtask tests that
  compile deliberately broken fixtures.

IR-874 probe (QSL's finding confirmed by source, not by re-running the 15
failing tests): IR f3a8c1c9 (#569) contains `v2/protocol_body.rs`, "the fixed
outer body grammar of the twenty finite global protocol operators". It
refuses a protocol node whose body is not a protocol application with the
operator's fixed argument layout (`ProtocolBodyDefect::OperatorIneligible`,
`MalformedLayout`, and the `LawMissing`/`LawMismatch`/`LawUnselected` and
`MemberMismatch` checks on the profile operator and law). ff5c66dd precedes
IR #566 (4ce1df3, the first IR-874 body slice) in main, so it is the last rev
before body validation; IR #565 (e31c506) sits between and is spec-only.

QSL's producer of the old shape is
`Lowering::protocol_node` in `qsl-semantics/src/check/lowering/protocol.rs`
(about lines 101-140). For every `protocol` declaration it emits a
`protocol_clause` node with `BodyTerm::application(Operator::ProtocolControl,
operation, boolean, Vec::new())`: an application with an empty argument
list, whose `Operation` carries the fixed `quire.op.protocol.control` id,
a single `OperationLaw` passed by the caller and a
`Member::ProfileOperator { operator }` of the caller's operator name. The
reference carrier `protocol_reference` (same file) is a `reference` leaf and
is the shape IR #566 also constrains.

## Verdict

PASS. The slice does what it says, is minimal, and the new test fails under
the one mutation that matters. Two low findings; neither blocks the merge.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | The new test carries no `#[trace("TC","FR-AC")]` attribute, unlike its siblings in `emit/tests/terminal.rs` and `state_graph_claim.rs`. It asserts a writer-contract property with no criterion binding, so the Test Matrix cannot credit it. Not enforced by the gate | qsl-package/src/emit/tests/refinement_rows.rs:11 |
| FND-002 | low | The test proves only the absence side. The pass-through in `src/population_fixtures.rs` (`base.refinement_step_rows()`) has no test where the base package carries rows, and it cannot, as QSL emits none; it is covered only by the existing retarget tests passing `None`. Acceptable now; the slice that emits refinement nodes (QSL-1102 AC1-3) must add a Some case with count above 0 and the reader round-trip | src/population_fixtures.rs:482 |
| FND-003 | low | Follow-up, not this PR: IR-874 body-shape adoption. `Lowering::protocol_node` (qsl-semantics/src/check/lowering/protocol.rs) emits an empty-argument `protocol_control` application. IR #566-#569 refuse it (`ill_typed`/`operator-ineligible` at `/semantic_graph/nodes/N/body`; 15 qsl-package tests fail at f3a8c1c9 per the coder). The lock must stay at ff5c66dd until that producer emits each operator's fixed argument layout | qsl-semantics/src/check/lowering/protocol.rs |

## Coverage

All six files of the diff were read in full. IR sources read at ff5c66dd:
refinement.rs step-row validation and occurrence validation, mod.rs
`CheckedPackageSourceCorrespondenceRef` and the encoder, intake.rs. At
f3a8c1c9: protocol_body.rs head and flat_wire.rs protocol vectors.
Not run: the 15-test failure at f3a8c1c9 was not reproduced (the build of a
second rev costs more than the source confirmation).
