---
id: SR-1788002
title: "PR #1788 (QSL-1219) gap analysis: FR-314-AC-3/4/5 (TC-893, TC-1165), FR-315-AC-6/53/55 (TC-900)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-replay/tests/it/refinement_smt.rs and alethe.rs unit tests at 0e8981915"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: reviews
---

# PR #1788 gap analysis

## Summary

Ticket: QSL-1219. Plan completion: not assessed.

**Covered:**
- FR-314-AC-4's verified certificate and its tamperings.
- FR-314-AC-5's byte budget, determinism and cancellation for the refinement part.
- FR-315-AC-6's exact scripts and refusals.
- FR-315-AC-53's layout and its six mutations.
- The alethe unit tests: numbering, malformed, not-refutation, conjunct indices, the named and searched resolution pivots, `define-fun` expansion, and the anchor context with `bind`.

## Verdict

Not mergeable together with SR-1788001. The negative tests do not reach the two soundness holes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | **No negative test covers an ill-sorted literal.** No test feeds an Int or numeral literal to a propositional rule, so the zero-assumption refutation of SR-1788001 FND-001 passes unseen. Pin it, plus `equiv_neg1`/`equiv_neg2` over `(= 0 1)`, as `ProofStepInvalid`. | FR-314-AC-4, alethe.rs tests |
| FND-002 | high | **No test covers integer strengthening over a non-integral coefficient.** Pin `(cl (not (> (* (/ 1 2) x) 0)) (not (< x 2)))` with `:args (2 1)` as `ProofStepInvalid`. | FR-314 `la_generic` |
| FND-003 | low | **No test covers a refused `Initial` query rejecting at `Base`.** Nothing pins the new QSL-1231 behaviour, a refused `Initial` query rejecting `QueryMismatch` at `Base`. | FR-314 Behavior |
