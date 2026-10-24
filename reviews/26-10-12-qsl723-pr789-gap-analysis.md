---
id: SR-7857
title: "QSL-723 PR 789 gap analysis: TC-235 M15"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0d84e0cea7b32600caa6e2f2d95b18ea13d2dd0d; QSpec TC-235 M15, qsl-eval/tests/it/tc235_business_meaning_runtime.rs, qsl-semantics/src/model/intake.rs"
review_set: subset
---

Redacted: QSpec content removed (public repo); the unredacted record is in the private tracker.

## Summary

Plan completion: not assessed

Ticket: QSL-723. A manual requirement-to-test-to-source review of the M15 runs this PR enables. No computed matrix and no build were run. The coder's frame analysis is correct: QSL FR-106 check 11 (FR-106-AC-5) refuses a change to a field outside `modifies` with `frame_violation`/`unauthorized-change` before any clause is evaluated (see also QSpec FR-013-AC-1/AC-5). The fixture works around the QSpec TC-235 M15 row locally; details in the private record.

## Verdict

PASS WITH FINDINGS. The M15 runs this PR enables trace to real production paths. The open gaps are tracking and evidence gaps, not wrong results.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The M15 fixture departs from the authoritative QSpec TC-235 M15 row in its `modifies` frame, and no QSpec ticket asks to correct the row. The row also conflicts with QSpec FR-340-AC-3. The divergence is recorded only in a test comment. | qsl-eval/tests/it/tc235_business_meaning_runtime.rs:123 |
| FND-002 | medium | The `wrong_snapshot`/`forbidden-pre-read` check for a model `Operation.pre` has no implementation: intake discards model operation `pre` clauses (`ctx.array_field("pre")` keeps nothing). The test is ignored with no ticket id, so the gap is not tracked. | qsl-semantics/src/model/intake.rs:2397 |
| FND-003 | medium | The M15 test does not assert every outcome the QSpec TC-235 M15 row states for runs 1 and 2. QSL-723 AC-1 asks for the full stated outcome. | qsl-eval/tests/it/tc235_business_meaning_runtime.rs:803 |
| FND-004 | medium | No test emits a checked package containing the new `model`/`state_machine` or `scalar_type` `string`/`uuid`/`timestamp` nodes and passes it through the checked-package-v2 reader, as FR-094-AC-8 does for units. Such a test would expose the conflicts in SR-7856 FND-001 and FND-002. | qsl-semantics/src/check/lowering/model/tests.rs:1360 |

## Coverage

Examined: QSpec TC-235 M15, QSpec FR-013-AC-1/AC-5, QSL FR-106 check 11 and FR-106-AC-5, QSL-723 AC-1/AC-2, the M15 test body and fixture, and intake `read_operation_member`.

Context only: QSpec FR-208-AC-10/AC-11 and FR-340-AC-2/AC-3. Linear has no ticket for the run 6 gap or the TC-235 row defect; a search for `Operation.pre` and `M15` returned only QSL-723.
