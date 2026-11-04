---
id: SR-20261104013116
title: "Gap analysis — QSL-1567 replay-side TraceBinding dimensions (PR #2234)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d30159139; PR #2234 diff; FR-522 Authentication and AC-3..AC-8"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: reviews
---

## Summary

Ticket: QSL-1567. Plan completion: not assessed. The scope is the PR diff only. The ticket asks for the declaration's `temporal_bindings` row to be exposed replay-side, per occurrence, and for every FR-522 dimension to be compared, reporting `mismatched_dimension`, `asserted` and `expected`. The diff does all three. The rows are keyed per (clause NodeKey, claim ordinal). FR-522-AC-3, AC-4 (all three members), AC-5, AC-6 (both the event-position and timestamped halves) and AC-8 each have a replay-side assertion with exact asserted and expected values. There are no stubs and no inflated coverage, and the agreeing bindings are shown to run. The new code adds no behaviour that FR-522 does not state. Quoin 0.29.1. The trace was checked by reading the `#[trace]` tags.

## Verdict

**MERGEABLE.** The ticket's acceptance is met. The one trace nit (an AC-3 tag on the timestamped test) is recorded in the code-review SR, not repeated here. The replay-side "no row means `Clock` with no `expected`" branch has no replay test, because the only caller cannot reach it: the emitter refuses a missing selection first. The shared `clock_mismatch` carries the rule, and the qsl-eval evidence path exercises it. That is acceptable and not a gap.

## Scope

- FR-522 Authentication bullets: examined.
- FR-522-AC-3, AC-4, AC-5, AC-6, AC-8: examined. Each has a tagged, asserting test.
- FR-522-AC-7: context only. It is unchanged by this PR.
- QSL-1567 ticket text (data): examined.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
