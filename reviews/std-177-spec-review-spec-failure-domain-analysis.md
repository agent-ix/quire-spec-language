---
id: SR-13662
title: "failure-domain review of STD-177 FR-150 fold precedence (PR #1143)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@5a71fd301e44e5072238efd8a3308780b5279b55; spec/functional/FR-150-check-the-symmetric-population-annotation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: reviews
---
# SR-13662: failure-domain review of STD-177 (PR #1143)

## Summary

Ticket: STD-177. This review looks for gaps in the precedence bullet across
the dimensions in scope. These are annotation state, catalog membership,
whether the source carries a reference into the annotated population,
collection kind (Set, Bag, Sequence), fold versus reduce, nesting, and units
that read more than one population. Each case was probed over the code at
the PR head (see SR-13659). The code decides per fold, from the source type
(`symmetric_population_in(source)`). It does not decide per unit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The bullet assigns causes per unit: "In a unit that annotates the population" and "in an unannotated unit". Two cases do not fit. First, the annotation is a mark on the domain package's population record (FR-150 Inputs), not on a unit. Second, a unit may read an annotated and an unannotated population. There, a non-catalog fold over references into the unannotated one is `operator-ineligible`, both in the code and by the bullet's first sentence, yet the unit "annotates" a population. State the split per fold: "When the source contains a reference into an annotated population, the checker SHALL report ...; otherwise FR-145's `operator-ineligible` stands." | spec/functional/FR-150-check-the-symmetric-population-annotation.md:76-81 |

## Verdict

Mergeable on failure-domain grounds: one low finding about wording. No
failure mode is missing. Code and FR-150 agree in each case below.

- A Bag behaves as a Set.
- `reduce` behaves as `fold`.
- In a nested fold, the inner non-catalog fold over references is refused
  at the inner node whether the outer fold is catalog or not.
- A non-reference source in an annotated unit keeps FR-145's outcome.
- A Sequence source is outside form 1 and checks.
- FR-145's typing-first order means a mistyped step is `type-mismatch`
  before either cause.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | c5e49e738: the precedence is now decided per fold from its source type ("When the source of such a fold or reduce contains a reference into an annotated population ... otherwise ... `operator-ineligible`"), and the reason is stated: a unit may read an annotated and an unannotated population. This matches `symmetric_population_in(source)`. |
