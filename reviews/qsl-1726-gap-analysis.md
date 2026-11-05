---
id: SR-20261105101011
title: "Gap analysis — QSL-1726 trace-subject temporal route"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2391; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md, qsl-replay/src/spine/clause/trace_subject.rs; FR-330-AC-17"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quire-spec-language/FR-330", type: references}
---
# Gap analysis — QSL-1726 trace-subject temporal route

## Summary

Ticket: QSL-1726. Planless gap analysis scoped to the changed trace-subject route, its FR-330-AC-17 requirement, and its tagged tests. The route's aggregate success/insufficient-data examples and malformed-input refusals have tagged tests. The refusal case for a model-subject clause without aggregate terms is not exercised by the tagged test: the fixture has no selected model, and `Plain` is a non-model `Counter` record trace subject. Plan completion: not assessed.

## Verdict

**CONDITIONAL** — the AC-17 test tag covers the main outcome examples, but it does not exercise the model-subject/no-aggregate refusal it claims to verify; the same test instead asserts an unrequired refusal for a record trace subject.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The test `trace_subject_refuses_malformed_and_unsupported_requests_before_evaluation` is tagged to FR-330-AC-17 and includes `Plain`, but `checked_types` uses `PackageInput::default()` with no model selection and `Plain` is declared over the record `Counter`. Thus this assertion does not verify the AC's model-subject/no-aggregate refusal. It asserts a refusal for a valid non-model record trace subject instead. Add the specified model-subject case to the tagged test and assert the ordinary record-subject behavior required by the route contract. | qsl-replay/src/spine/clause/trace_subject.rs:650-667, 700-738, 826-846; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:79-94, 288 | correct-requirement-no-evidence |

## Coverage

- `quoin matrix --repo . --json` produced no output during roughly four minutes of CPU-bound execution and was stopped at the lead's direction. Matrix counts and whole-repository reverse-gap/stub results are unavailable; no whole-repository pass is claimed.
- Manual changed-scope mapping: FR-330-AC-17 has two tagged tests in `trace_subject.rs`. The aggregate insufficiency/violation test covers the specified results. The malformed-input test covers empty samples, non-record value, subject mismatch, no-aggregate record clause, and unsupported profile. The no-aggregate record case does not prove the AC's model-subject/no-aggregate refusal (FND-001).
- The changed production route is owned by FR-330's trace-subject route behavior and AC-17. The identified extra refusal of a non-model record clause is also recorded in the separate code-review artifact.
- Plan completion: not assessed.
- Semantic intent review: not requested and skipped.
