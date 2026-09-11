---
id: SR-372
title: "Base specification review of domain-event Boolean choices"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/TC-121, type: references }
---

## Summary

Applied QUOIN spec-review's base checklist and six coverage rules to the
domain-event amendment at PR readiness, as explicitly requested. No installed
AssuranceProfile with required review_selection was found. The selected set is
base plus failure-domain; other analyses and optional semantic gap review were
not requested. The active quoin SpecReview skeleton and schema were read before
authoring. The owner-specified reviews directory is used for these artifacts.

FR-042 retains explicit original source/model/producer inputs, constructor-private
admission and typed artifact/refusal outputs, dependencies and US-004 lineage.
Its change states exact resolved event-role equality, original field/binder/anchor
identity, causal availability, independent qualified registration prerequisites,
and no inferred activation/effect. TC-121 procedure 5 and the wire contract agree.

## Verdict

**CONDITIONAL** for inherited metadata and full-criterion breadth only. No new
contradiction, ambiguous eligibility rule or unowned changed requirement was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-121 retains an inherited prose procedure without explicit Type/Priority metadata required by the full base checklist. The domain amendment does not introduce or repair that corpus convention. | spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md |
| FND-002 | low | New tests cover the selected domain-event slice, not every option/boundary/error of the broad ten-criterion artifact contract. Existing corpus gaps and external B acceptance remain recorded; tag backing alone does not establish all permutations. | FR-042-AC-1 through FR-042-AC-10; SR-371 |

## Coverage

IDs and relationships: FR-042/TC-121 and AC numbering remain unchanged and linked;
all ten FR criteria name TC-121. Scoped spec grammar validation is 435/435 clean,
zero grammar findings, with duplicate loader-definition diagnostics. External
ix references retain their declared ownership; local grammar validation is not
verification of external repository completion.

Six-rule assessment: requirement coverage maps the amendment to AC-5 and TC-121;
ordinary/qualified and same/foreign ownership combinations have concrete cases;
References and Entries exact/one-short plus fresh retry are exercised; hidden or
composite visibility and nonpartition paths are explicit refusals; registration,
activation and effect authority remain distinct, without claiming runtime state
transition execution; equal-typed joined observations and original anchors are
tested as edge cases. General permutations remain limited as FND-002 states.

Eight new tests reached the actual public pipeline in the parent's 41 passing
focused tests. Both strict Clippy phases subsequently finished after the initial
unused-helper correction; parent full suites subsequently passed (597 minimal,
614 all-feature, plus five doctests each). The ninth new test, commit-record
ineligibility, passed its focused run and substantive Opus recheck. This is
the author's evidence-backed specification review, not an independent Claude
code review; SR-374 records that separate review.
