---
id: SR-11627
title: "Code review of quire-spec-language PR #884: replay over an admitted object environment"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@592639475368d8f4fb1405b27c7f2422e29fdb4f; qsl-replay/src/object_environment.rs, qsl-replay/src/execute.rs, qsl-replay/src/execute/argument.rs, qsl-replay/src/execute/object_environment.rs, qsl-replay/src/request.rs, qsl-replay/src/outcome.rs, qsl-replay/src/execute/tests/object_environment.rs, qsl-replay/src/execute/tests/composite.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/extents.rs"
review_set: subset
---
# Code review of quire-spec-language PR #884

## Summary

Ticket: QSL-805. One pass of code-review with the rust-review lane over this
PR's own commits (`git diff origin/relationship-extents-admission...HEAD`,
#880 still open), excluding the merged spec commit. No builds were run (disk);
clippy, string-edge, arch-lint and the no-all-features run are unrun.

Oracles: FR-098 "The object environment" steps 1 to 5 and AC-14 to AC-19,
FR-071 `object_environment` and AC-12, FR-070-AC-16/17, TC-1042 rows 1 to 33,
FR-106 checks 1, 3, 4, 6 to 8 and the `pre`/`post` member form, FR-364
`resolve`, ADR-013 C-09 (a fault stays a tool failure).

Examined: the request member reader and wiring; the step 1 to 5 order in
`replay_with_cancel`; reference conversion (shape, universe, conformance,
identity, witness type kept); `admit_object_environment`,
`admit_argument_populations`, `populations_holding`, `ResolveBundle::over`;
the `admit_current_snapshot` refactor; the new refusal arms in `code()`,
`cause()`, the outcome stage map and `ProofResult::from_replay_refusal`; all
33 TC-1042 tests against the table; the tc_906 change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | An internal fault during environment admission is reported as a replay refusal, not a tool failure. `admit` wraps every `AdmissionFailure` from `admit_object_environment` and `ResolveBundle::over`, including `AdmissionFailure::Fault` (for example `bundle-extents-missing`, `expected-snapshot-document`), as `ReplayRefusal::ObjectEnvironment(Admission(Fault))`. `ProofResult::from_replay_refusal` and `replay_stage` only treat `ReplayRefusal::Fault` and `ReplayRefusal::Admission(Fault)` as faults. So this fault settles `Inconclusive(ReplayRefused(runtime_invariant))` at stage S8 instead of `Failed` with no stage. Fix: map `AdmissionFailure::Fault` to `ReplayRefusal::Admission(Fault)` or `ReplayRefusal::Fault` in `admit`, and keep `ObjectEnvironment` for `Refused`/`Incomplete` and resolve verdicts. | qsl-replay/src/execute/object_environment.rs:207-209, qsl-replay/src/execute/object_environment.rs:224-226, qsl-replay/src/proof_result.rs:416, qsl-replay/src/outcome.rs:1001 |

## Verdict

FAIL: 1 blocking finding (FND-001).

Clean:
- Order matches FR-098: join, then step 1 (`UnusedObjectEnvironment`
  before conversion and before the document is read; rows 20 and 21), then
  conversion (shape, then universe as `ForeignReference` with the declared
  type's universe, then conformance with a proper subtype admitted, then
  non-empty UTF-8 identity), then FR-106 admission, then `resolve` per
  reference in parameter and walk order, then the call over the admitted
  environment. No environment keeps unresolved references.
- The kernel reference keeps the witness's `object_type`, so `ref(h, TH)`
  is held (row 11) and `ref(a, TH)` is dangling (row 30).
- Check 7's required set is the populations listing an object whose whole
  reference key equals an argument; rows 28 and 31 separate them correctly.
- `admit_current_snapshot` delegating with no arguments is equivalent to the
  old `admit_populations(.., None, ..)` call: the required set is empty in
  both.
- The FR-071 member form matches FR-106's `pre`/`post` reader
  (`read_document_ref`): four string labels and `sha256-jcs:<64 hex>`.
  FR-071 states this by reference to FR-106, so it is not a coder choice.
  Admission selects by digest alone; `read_document` never compares labels.
- The tc_906 change replaces the arbitrary universe `[1; 32]` with the
  declared type's universe. That is required by the new universe check. It
  keeps every assertion, so it is not weakened.
- Every TC-1042 test asserts what its row states. Mutants: universe after
  conformance (row 17), subtype refused (row 11), declared type kept (rows
  11 and 30), step 1 missing or after conversion (rows 20 and 21), unheld
  reference not refused before the call (rows 29 to 33), wrong digest (rows
  5 and 22), reversed walk order (row 32). Not caught: comparing the
  `identity` labels with the document's identity, because every test uses
  matching labels.
- Canonical-encoder lint: no new shipped file names a hasher
  (`Sha256`, `sha2`, `of_preimage(`, `ByteDigest::of(`); the tests take
  digests through `PackageDocument::jcs_digest`. Expected to pass, not run.
- string-edge: the new code compares member names only inside `matches!`
  and `LABELS.contains`, which the syntax scan does not see. Expected to
  pass, not run.

Recorded (not blocking):
- medium: `read_object_environment` names the first unknown member
  alphabetically (serde_json `Map` without `preserve_order`), not "first in
  document order" as FR-071 states. Only visible with two unknown members;
  no row tests it.
- medium: row 14 asserts only the `NoValue` disagreement, not
  `unknown_required_feature`/`unsupported-feature`/`dereference`, because
  `ReplayResult` does not carry them. Same limit as tc_906; tag is honest
  for the part asserted.
- medium: no test supplies `identity` labels that differ from the
  document's, so FR-071's "does not compare" is unguarded. Suggest a TC-1042
  row.
- medium: `ObjectEnvironmentFailure` and `Unresolved` are `pub` in a private
  module and not re-exported from `lib.rs`, so a consumer cannot match a
  `ReplayRefusal::ObjectEnvironment` to read the position or reference.
- low: `read_object_environment` is a member-name dispatch reader like
  `request.rs:690` but has no `#[qsl_attrs::string_edge]` marker.
- low: `ReferenceModel::universe` re-normalizes every selected model per
  reference argument.
- low: gates unrun: clippy all-targets all-features, string-edge, arch-lint,
  `spine::` and `model::observation` suites, no-all-features.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bf2c1dc33 |

Round 1 (reviewed 949d1de0045deb7b944a9d8f5b16c8a41d7ac347): `environment_refusal` maps
`AdmissionFailure::Fault` to `ReplayRefusal::Admission(Fault)` for both the
admission and `ResolveBundle::over`. `an_admission_fault_stays_a_fault` asserts
`TerminalValue::Failed`; `replay_stage` already maps `Admission(Fault)` to no
stage. The dda05ae20 change (supplied type looked up in the model's object
types) keeps rows 11, 17, 18 and 30. Mediums: re-exports added, a
labels-not-compared test added, the alphabetical note is in the PR body. No new
findings.
