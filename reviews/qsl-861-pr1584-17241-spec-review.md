---
id: SR-17241
title: "Spec review of QSL-861 task-set front end against FR-246, FR-247 and QSpec FR-419 (quire-spec-language PR 1584)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3fd0af8e230442634534179a47b2d20865a8d1b5; PR #1584 (branch qsl-861-taskset-front-end): spec/functional/FR-246-check-task-sets-and-schedulability-claims.md, spec/functional/FR-247-analyse-a-task-set-in-closed-form.md; QSpec FR-419, FR-482, FR-290 and proposals/quire-v1/shared-grammar.md (by path)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-247
    type: references
---

## Summary

Ticket: QSL-861. The PR changes no spec file; this pass compares the implementation with the spec text for deviations. The grammar, the refusal codes and tags (`invalid_timed_model`/`invalid-task-set`, `conflicting_declaration`/`duplicate-declaration`, `missing_declaration`/`missing-name`, `ill_typed`/`type-mismatch`), the defect order (task-set defects by span, then per claim in source order, a refused set still checked against each claim) and the exact FR-142 conversion agree with FR-246 and FR-419. Deviations are listed below.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Code deviates from FR-419 in qsl-replay: `TaskSetRefusal` still reports `invalid_model_binding`/`malformed-declaration` for the task-set defects FR-246, FR-419-AC-5 and AC-10 give `invalid_timed_model`/`invalid-task-set`. Fix in this PR (SR-17239 FND-001). | qsl-replay/src/certificate/closed_form/task_set.rs | implementation-bug-despite-evidence |
| FND-002 | low | FR-246 says the unit is that of the first time quantity "in source order"; the code takes the first task in ascending task-name order, which FR-482 requires of the node and which makes the unit independent of task order. The code is right and the FR-246 text is stale (QSL-1093); AC-3's examples agree under both orders. | FR-246 Behavior "The task set's unit" | wrong-requirement |
| FND-003 | low | FR-246 is silent on a claim naming no task set and on two task sets of one name; the code refuses with `missing-name` and `duplicate-declaration`. Add both to FR-246 and AC-2 with QSL-1093, and settle the duplicate claim-name question (SR-17239 FND-004) with QSL-1109. | FR-246 Behavior | missing-requirement |
| FND-004 | low | FR-246 says a `response` claim over a set with no unit "keeps its bound in the bound's own unit"; the code converts to the root unit (SR-17239 FND-002). | FR-246 Behavior | implementation-bug-despite-evidence |

## Verdict

**CONDITIONAL** on FND-001 and FND-004; FND-002 and FND-003 are spec edits for QSL-1093.
