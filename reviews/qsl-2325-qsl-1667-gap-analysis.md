---
id: SR-20261104231204
title: "gap-analysis of QSL-1667 spec diff (quire-spec-language PR 2325)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR 2325 head b3d90793c51500e2f5edd5c302ff654b0e4db409; new and changed ACs FR-196-AC-14, FR-202-AC-8, FR-204-AC-6..11 against tagged tests on the head"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
---

# Gap analysis of QSL-1667 spec diff (quire-spec-language PR 2325)

## Summary

Ticket: QSL-1667. Spec-only PR; no production code changed. This is a manual AC-to-test check scoped
to the diff. Quoin's matrix was not run (spec-only tier; no cargo). Measured on the head:
`FreeDelays`/`free-delays` appear in no `.rs` file, and `#[trace("TC-642", ...)]` tags exist only for
FR-204-AC-5 (qsl-analyze/tests/it/digital_clocks.rs:419,543) and the FR-204 FR-level tag (:757).
FR-196-AC-14, FR-202-AC-8 and FR-204-AC-6..11 have no tagged test. That is the expected state, since
the PR's own Dependencies place the implementation under QSL-1667 in the QSL-1498 lane. Once the spec
merges, the current mixed-state `WorkloadResolve` code path contradicts FR-204 until that
implementation lands. Each new AC has a TC step with concrete expected values, so a test can be
written directly from it.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Clean for a spec-only PR. The implementation gap is owned by QSL-1667 and is not a defect of this diff.

## Dispositions (pass 1)

| FND | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | no action | Placeholder; no finding. The new FR-204-AC-12 from the spec-review fix has its TC-642 step 8. Its tagged test belongs to the QSL-1667 code change, like AC-6 to AC-11. |
