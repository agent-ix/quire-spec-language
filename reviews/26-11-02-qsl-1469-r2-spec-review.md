---
id: SR-2026110216373
title: "Spec review — QSL-1469 round 2: FR-094 roots the nodes FR-496 rows name"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md; spec/functional/FR-496-emit-model-operation-effects-into-checked-packages.md; spec/test-cases/TC-417-model-reference-and-population-nodes-match-key-vectors.md"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: reviews
---

## Summary

Ticket: QSL-1469. PR agent-ix/quire-spec-language#2125. Base checklist over the spec diff only. No analysis lenses were selected; the change is a reconciliation of FR-094's reachability rule with FR-496 and QSpec FR-322-AC-303.

The diff adds one paragraph to FR-094 that makes an FR-496 `operation_effects` row the second root of the reachability rule: the declaring owner of each operation with a row, the declaring node of each written field or the written relationship, and each `creates` and `deletes` subject. It restates FR-094-AC-2 to three correspondence entries (Order, Invoice, Sub), updates TC-417's step 1 expected result to match, and reconciles FR-496's "does not change semantic graph nodes" with the new root.

## Verdict

**PASS** after FND-001, which is fixed in this PR.

Checked:

- IDs: no new IDs; FR-094-AC-2 still verifies by TC-417, and TC-417's step 1 now expects M1, M3 and M5, which is the restated AC.
- Consistency with QSpec (fresh origin/main 6ae6a209): FR-322's "Operation-effect carrier" reader refuses a row whose `declaring` node is absent (`missing_declaration`/`missing-name`) and joins each effect entry as FR-340 does, so the nodes the paragraph roots are required by the reader. No contradiction with FR-322.
- Consistency inside QSL: FR-496-AC-5 (carrier present or omitted gives the same graph) holds, because the roots come from admitted records, not from the carrier. FR-094-AC-9's relationship root is unchanged; the paragraph's fallback `generated_location` applies only to a model admitted without an alias, which AC-9's alias-site root does not cover.
- Fixture derivation of AC-2: acme/orders declares `Order/size`, `Order/count`, `Order/scaled` (owner Order), `Sub/size`, `Sub/scaled` (owner Sub), all with empty frames. Order is rooted by `r`'s type and its rows, Invoice only by `s`'s type, Sub only by its rows; no frame subject adds a node, and the unaliased fixture model roots no relationship. The set is exactly {Order, Invoice, Sub}.
- Testability: the AC names a closed set; the TC-417 test asserts it, and new TC-1286 tests cover the frame-subject roots.
- Hash/digest/pin: none in the diff.
- `make check-spec-validation`: exit 0, no diagnostics on the touched files.

## Findings

| ID | Severity | Summary | Refs |
| -- | -------- | ------- | ---- |
| FND-001 | low | In FR-094's reachability section the paragraph ending "(ADR-013 O-06):" introduces the member bullet list, but the selected-relationship exception paragraph sat between them, detaching the list from its lead-in; adding the FR-496 root next to it would have worsened that. Fixed in this PR: the relationship exception now follows the list, and the FR-496 root follows it as "a further root of that reachability rule". | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:136-177 |
