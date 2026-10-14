---
id: SR-7816
title: QSL-710 checked request implementation final code and Rust review
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language@93ea9efce64e925f22e5a9609da75cc179fa40c3
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

Final static code and Rust review of the QSL request-payload slice in PR 741. Each requirement occurrence retains the canonical occurrence key of its direct narrow, and the request item exposes it through `conversion_context() -> Option<&OccurrenceKey>`. RR-16 preserves distinct contexts for occurrences sharing one scalar application node.

## Verdict

**SR-7816: PASS at `93ea9efce64e925f22e5a9609da75cc179fa40c3`.** FND-001 is fixed. No open code-review findings or new rebase findings remain. This is a static review; runtime qualification is separate and no builds or tests were run by this reviewer.

SR-7817 FND-001 remains **deferred to the separate driver implementation slice**: production driver forwarding of occurrence context and explicit lowering roots is outside this payload PR. SR-7817 FND-002 is fixed. This review does not assert full QSL-710 completion.

## Findings

The table records the original finding; its current status is fixed as stated in Dispositions.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-160 step 12 requires zero/two matching narrow refusal and nested inner-versus-outer identity controls. The only new test is the ordinary RR-16 positive plus Own case. Existing hand-built key_claims fixtures all use SiteBound::Own; changing the new exact-one selection back to first-match would survive these controls. Add narrowed-site fixtures with independently fixed expected keys, asserting typed UnkeyableRequirements for zero/two candidates and the inner key for nesting. | qsl-semantics/src/check/claims/tests.rs; qsl-semantics/src/check/claims.rs; TC-160 |

## Dispositions

| FND | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | fixed | Zero/two-match tests call production `key_claims` and assert `KeyFault::UnkeyableRequirements`. The final nested fixture inserts an outer narrow referring to the direct narrow, co-locates its occurrence with the scalar site, calls production `key_claims`, and asserts the independently identified direct occurrence. Production owns the operation/Reference predicate; no test-written predicate replaces it. |

The first fix round added zero/two controls but its nested selector test bypassed production operand matching. The second round replaced that test with the graph/occurrence fixture described above, resolving the finding. The final rebase preserves that implementation and all controls.

## Coverage

Reviewed `qsl-route/src/request.rs`, `qsl-semantics/src/check/claims.rs`, `qsl-semantics/src/check/claims/tests.rs`, `qsl-semantics/src/check/lowering.rs`, and `tests/it/request_builder.rs`; assessed FR-062 AC13, FR-075 AC15 and TC-160 step 12. FR-075 AC16 remains the separately owned driver obligation.

The narrowed selector requires exactly one direct match, verifies the operation identity and sole Reference operand, and excludes an outer narrow that references the inner node. Own-result records and items retain `None`. The request RR-16 control carries the FR-075 AC15 trace and verifies occurrence-specific contexts without introducing extra items.

For the rebase onto main `362a7b3398a79e86a1fe0af22960afc93f120363`, all seven source/test commits are patch-equivalent and all five implementation/test files are byte-identical to the previously reviewed source. Nearby incoming checker changes concern complete-model `reaches` endpoint typing and a rational-division fixture; they do not change scalar claim collection or context propagation. The remaining PR additions are the archived review documents.
