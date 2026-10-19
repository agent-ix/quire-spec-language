---
id: SR-12188
title: "Gap analysis of quire-spec-language PR #1027: FR-129 criterion and trace coverage at S3"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@355e95b6dcf5beee59a54379a49ce5c939070c4c; PR #1027: FR-129 (AC-1, AC-2, Behavior), TC-530, trace tags in qsl-semantics/tests/it/strong_fairness.rs; implementation read: qsl-semantics/src/check/temporal_clause.rs, qsl-semantics/src/check/lowering/temporal.rs; context: ADR-019 sections 1, 5 (SV-5) and 6, FR-123 and its TC-518 tests"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1027

## Summary

Ticket: QSL-532 (partial: E10a). Planless. Plan completion: not assessed.
Read-only; the criterion-to-test check was done by hand over the `#[trace]`
tags, since the computed Test Matrix (`quoin matrix`) does not finish in
reasonable time on this repository.

- FR-129-AC-1 is tagged by
  `an_unmarked_kind_is_weak_and_the_kind_enters_the_identity` and FR-129-AC-2
  by `weak_and_strong_stay_apart_and_strong_needs_an_infinite_trace`, both
  `#[trace("TC-530", ...)]`. No other test in the workspace names FR-129 or
  TC-530, and both ACs had no tagged test before this PR.
- Implementation owning the behaviour is already on main:
  `checked_constraint` maps `None | Weak` to `FairnessKind::Weak` and
  `Strong` to `FairnessKind::Strong`, with `None | Whole` to `Whole`;
  `checked_fairness` refuses every constraint under a bounded profile at its
  span and deduplicates by full `FairnessConstraint` equality in source
  order of first occurrence; the lowering puts `kind` into each `fairness`
  node, so into the clause key (ADR-019 SV-5). No production code in this PR,
  so no unowned code.
- No stub or coverage inflation: both tests assert concrete values and fail
  on their AC's behaviour (see SR-12187).
- Declared remainder, not a finding: FR-130 (TC-531), FR-131 (TC-532) and
  UnfairLasso settlement are blocked on E9 (QSL-530) and ticketed as
  QSL-896; FR-130-AC-4/5 also need QSL-893.

## Verdict

PASS. FR-129-AC-1 and FR-129-AC-2 are each covered by a test that can fail
on the criterion's behaviour. One low gap is recorded against a Behavior
bullet that no AC states.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-129 Behavior says two constraints with equal resolved kind, operation and granularity check as one "whether or not the kind was written", but no test writes an unmarked and a `weak` constraint together (AC-2 covers only `strong` twice; TC-518 covers an unmarked granularity, not an unmarked kind). It follows today from AC-1's set equality plus `set.contains`, so this is a missing direct test, not a defect in the code. | spec/functional/FR-129-check-strong-fairness-constraints.md:69-70; qsl-semantics/tests/it/strong_fairness.rs:285-288 |
