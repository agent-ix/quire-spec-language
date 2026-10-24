---
id: SR-13492
title: 'Spec review of PR #1088: S3 timed-facet gate (FR-230), spec-ears-analysis'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@9cdbed126; git diff refs/remotes/origin/main...9cdbed126 (PR #1088, branch std-183-spec): spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md, spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md, spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; ticket STD-183'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-230
  type: reviews
---

## Summary

Ticket: STD-183. PR: quire-spec-language#1088, head 9cdbed126. Method: spec-review/spec-ears-analysis over the new and edited requirement statements. `make check-spec-validation` (quire 0.36.2, engine 0.50.2) reports no EARS warning on the four touched files; the findings below are what the linter does not catch. The FR-361 item 8 and FR-393 edits add a cross-reference to an existing statement and change no EARS shape.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-230 Timed facet gate bullet 1 (refusal) | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230 Timed facet gate bullet 2 (no other refusal) | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230 Timed facet gate bullet 3 (selecting unit) | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-7 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-361 item 8 | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md | examined |
| FR-393 S2 facet bullet | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Bullet 1 uses the event keyword "When" for a standing condition of the unit (the header does not select the facet). That is EARS's unwanted-behaviour shape ("If ..., then S3 SHALL refuse ...") or a state shape ("While ..."), not an event. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:217 |
| FND-002 | low | Bullet 3, "A unit that selects `quire.temporal.complete/v1` is checked by the rules above", sits in the normative list with no SHALL and no subject that acts. Make it a SHALL statement for S3 or move it into the section's prose. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:227-228 |
| FND-003 | low | FR-230-AC-7 asserts two outcomes, the four refusals without the facet and admission with it. The admission half repeats FR-230-AC-1 and fails for AC-1's reasons, so a failure of AC-7 does not show which half failed. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:246 |

## Verdict

Findings, all low. Bullet 2 is a well-formed ubiquitous constraint. FR-361 item 8 and FR-393 read correctly with the new cross-references.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | "Once the facet is selected, S3 SHALL check a `Clock` parameter ... by FR-230's clock rules as they apply to a clock outside a model object type, which this requirement does not yet state" is a SHALL with nothing to verify. Rewrite it as a non-normative note naming the open rule, or as the gate's own deferral. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:255-258 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
