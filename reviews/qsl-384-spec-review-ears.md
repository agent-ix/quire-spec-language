---
id: SR-1131
title: "QSL-384/QSL-385 EARS review: FR-325 to FR-336"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@253ddd2f2075039ffb18d46d23b78a73478a6542; diff origin/spec/366-temporal-properties...HEAD; spec/functional/FR-325..FR-336"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-333
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: reviews
---

## Summary

Ticket: QSL-384 (with QSL-385). PR agent-ix/quire-spec-language#585.

Examined: the Description and Behavior statements of FR-325 to FR-336.
Nearly all of them are ubiquitous ("X SHALL ...") or event-driven ("When
..., X SHALL ...") EARS statements with a named actor. The findings are
the few statements that are not positive EARS requirements, or that name
an alternative which does not exist.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-333 says "No default, profile ceiling or limit stands in for an absent bound" (not a SHALL statement) and "The resolver SHALL read no profile ceiling (ADR-014 B-6), default or limit". Both name alternatives that do not exist. The positive statements already say it: an absent bound resolves to `None` and `Population(None)`. Delete both sentences. | spec/functional/FR-333-read-an-optional-collection-bound-and-population-maximum.md:31,64-65 |
| FND-002 | low | FR-336 says a subject becomes finite "never through a proof bound, a default or a limit". The proof-bound half is the ADR-018 §3 ruling and is stated again positively at line 77. "A default or a limit" names alternatives that do not exist. Say "only through the subject's universes and the model's declared domains" and stop there. | spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:33-35 |
| FND-003 | low | FR-325 says "S2 SHALL NOT compare `a` with `b`" and "SHALL read no profile selection". These are negative requirements, and no test can check the first one as written. FR-325-AC-2 and AC-4 already test the positive behaviour (an inverted interval parses with no diagnostic; forms are byte-equal under every profile). State that positive behaviour instead. | spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md:60-61,67 |

## Verdict

EARS conformance is good across all twelve FRs. The three low findings are
wording only. Fixing them makes no change to behaviour.
