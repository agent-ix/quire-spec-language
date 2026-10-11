---
id: SR-7816
title: QSL-710 checked request implementation code and Rust review
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language@1c61829dfc531958dc95e8c688bd940179ce59d3
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-160
    type: reviews
---

## Summary

Ticket QSL-710, PR 741. Reviewed the changed request, claims and test files, the claims occurrence-keying implementation, existing hand-built occurrence fixtures, FR-062 AC13, FR-075 AC15/16 and TC-160 step 12. Rust review is included. The producer/accessor path preserves canonical occurrence identity and fails closed when a narrowed claim has zero or multiple direct narrow matches.

## Initial finding

Round 1 found that TC-160 step 12 lacked zero-match, two-match and nested inner-versus-outer controls. Round 2 found the first nested test supplied its own predicate, bypassing the production operation/operand matcher. Both findings are resolved by commit `bacf88df38a94cce0fc015bea041652e51889985`: the nested fixture adds an enclosing narrow occurrence to the checked graph, calls production `key_claims`, and asserts the independently identified direct occurrence. Mutating production matching to accept the outer narrow makes the test fail closed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-160 nested direct-versus-outer narrow selection needed a production-path control; fixed by the checked-graph `key_claims` fixture at `bacf88df38a94cce0fc015bea041652e51889985`. | TC-160; FR-062-AC-13 |

## Final disposition

**PASS for the QSL payload code and Rust review.** No new findings. The item-level `None` assertion and FR-075-AC-15 trace were added in `c3f770f234e8388b973f8661290d952b6119e5b5`.

Runtime qualification remains a separate gate. The driver’s use of the occurrence context and independent lowering roots remains explicitly deferred to the separate driver implementation slice; this review does not claim full QSL-710 completion.
