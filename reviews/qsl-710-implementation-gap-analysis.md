---
id: SR-7817
title: QSL-710 implementation acceptance gap review
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language@1c61829dfc531958dc95e8c688bd940179ce59d3
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-160
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-449
    type: reviews
---

## Summary

Ticket QSL-710, PR 741. Bounded acceptance audit of FR-062 AC13, FR-075 AC15/16, TC-160 step 12 and TC-449 steps 6/7, including the actual downstream driver. This is not a whole-repository assurance claim.

## Verdict

FAIL for full QSL-710 completion. The QSL payload slice is substantially implemented but the driver obligation remains unimplemented. PR 741 may be treated as a partial slice only with this obligation explicitly retained; it cannot close QSL-710. No runtime qualification claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The production driver does not carry occurrence conversion context or forward the add and narrow roots independently per item. Its RequestedItem has only node, requirements and backend; generate resolves only that node and passes it to RoutedGenerationItem. FR-075 AC16 and ticket deliverables 2–4 therefore remain open. IR already exposes CheckedPackageV2::lower(requested: &[CheckedNodeId], profile), so the PR body's claim of no Rust contextual-root lowering API is not a reason to wait for a new entry point. Preserve this as an explicit downstream implementation gap and correct the body. | quire-driver/src/drive.rs:44,196; quire-contract-model/src/checked_package/v2/lower.rs:373; FR-075-AC-16 |
| FND-002 | medium | The request RR-16 test is tagged FR-075 AC8 but not new AC15, and it checks only Some contexts. No request-item Own-result None control exists, so a builder regression fabricating a context for Own records is untested. Add the absent item-level None assertion and AC15 trace. | tests/it/request_builder.rs:261; FR-075-AC-15 |

## Coverage

Plan completion: not assessed

Read all changed source/tests and owning requirements. Checked driver at 11ffe68865fcb8f8d6f2e2d4b2c2f5810ffa07b3. Checked IR lower API from its origin/main. Producer RR-16 expected narrow occurrence keys are obtained independently from expression occurrence metadata. Existing request test compares propagated record keys and exactly two request items. TC-160 step 12 coverage is SR-7816 FND-001. Computed matrix capture pending; source trace search confirms no FR-075 AC15 or AC16 tags in implementation tests at this head.

## Dispositions

Round 1, reviewed 27822045d198147cf8bffe7ff23dd5251e001701. Static only; no builds.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | Lead narrowed PR741 to QSL payload scope; production driver forwarding is a separate draco-owned slice. Actual IR root-slice API and outstanding driver implementation are now accurately described in the PR body. QSL-710 remains incomplete until the driver slice lands; no driver or full-ticket PASS claimed. |
| FND-002 | fixed | c3f770f234e8388b973f8661290d952b6119e5b5 |

Current bounded disposition: QSL request propagation finding fixed; driver implementation explicitly deferred outside this PR. Overall merge review remains conditional on SR-7816.

### Round 2 update

Reviewed the final payload head `bacf88df38a94cce0fc015bea041652e51889985`. SR-7817 FND-002 is fixed: the request-builder test asserts the item-level `None` case and carries the FR-075-AC-15 trace. SR-7817 FND-001 remains deferred to the separate driver implementation slice: the driver does not yet forward occurrence contexts or lower the independent roots. The independent code/Rust review SR-7816 is now PASS on the payload slice at `bacf88df38a94cce0fc015bea041652e51889985`; runtime qualification remains pending. QSL-710 as a whole is not complete until the driver slice lands.
