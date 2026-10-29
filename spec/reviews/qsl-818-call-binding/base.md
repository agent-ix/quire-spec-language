---
id: SR-11762
title: "Base review of FR-367 and TC-1053 (QSL-818, PR #915)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@35042dc696ecfa1a71a62ecbb58f10529b8fb0f4; spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/test-cases/TC-1053-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/spec.md"
review_set: subset
---
# Base review of FR-367 and TC-1053

## Summary

Ticket: QSL-818. Base checklist over FR-367, TC-1053 and the spec.md index entries: id formats, AC-to-TC coverage, relationship links, and refusal-code reuse against FR-100's code table and FR-357's causes. One defect: the `Admission` row's code list.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `Admission` row maps `Admission` and `DanglingReference` to `invalid_runtime_input`, but FR-100's code table gives the model-aware `Admission` causes `foreign_reference` (`foreign-model-selection`/`foreign-type`/`foreign-universe`) and `DanglingReference` `dangling_reference` (`absent-target-in-complete-population`). The row contradicts "that refusal's own" code and FR-100. | FR-367 Outputs (BindRefusal table), FR-100 |

## Verdict

FIX. ID formats valid, FR-367-AC-1..AC-9 each have a TC-1053 procedure step and expected result, spec.md has the FR-367 contains edge and index row, relationships resolve to existing FR-092/FR-098/FR-100/FR-357/US-010.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 530ecc8c4 |
