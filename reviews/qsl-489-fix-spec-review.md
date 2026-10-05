---
id: SR-1309
title: "Spec review of quire-spec-language PR #636: FR-285-AC-5, FR-277-AC-3 and their TC steps"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6014d82773ad72641e9934b6d62c2963f7b870a7; git diff origin/main...HEAD -- spec/ (PR #636): FR-285-AC-5, FR-277-AC-3, TC-769 step 5, TC-758 step 6, spec/tests.md rows; checked against ADR-029 CB-4, ADR-014 §1 B-1..B-6, FR-096, FR-010"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: reviews
---
# Spec review of quire-spec-language PR #636

## Summary

Ticket: QSL-489 (LC1, reopened). PR: quire-spec-language#636. Includes the
EARS check of the two new criteria.

- FR-285-AC-5 agrees with ADR-029 CB-4 ("`StageFailure::Limit` ... incomplete
  (22)"), FR-285-AC-3 and FR-010. It closes the gap that let `run`'s
  `CompileRefusal::Limit` reach exit 20 while AC-3 tested only the bare
  `StageFailure` mapping.
- FR-277-AC-3 states the counter's definition in place ("the consumed value
  with the denied amount"). That agrees with FR-277 Outputs and with the
  kernel record. It is testable and has a real oracle.
- The TC-769 step 5 and TC-758 step 6 procedures and expected results match
  their ACs.
- No other spec statement in the tree still says a stage limit exits 20 or
  is a refusal category. ADR-014 B-3's "A stage failure, not an outcome
  category" predates ADR-029's exit table and does not contradict it.

## Verdict

Approve, with one low ambiguity.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-285-AC-5's "and one limit of a later stage" does not say which limit, so two implementers could test different stages. TC-769 step 5 repeats the vagueness ("one limit of a later stage set below the counter its input reaches"), and the test picked `s3.nodes` at 1. Name it: "and `s3.nodes` set to 1". | spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:106; spec/test-cases/TC-769-the-exit-function-maps-every-category-and-multi-item-outcome-with.md:23 |
