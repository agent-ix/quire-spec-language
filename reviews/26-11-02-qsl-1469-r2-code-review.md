---
id: SR-2026110216371
title: "Code review — QSL-1469 round 2: root only the nodes FR-496 rows name"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/lowering/model.rs; qsl-package/src/emit/tests/owners.rs; qsl-package/src/emit/tests/variant_types.rs; qsl-package/src/emit/tests/admission_corpus.rs; FR-094; FR-496; FR-360-AC-8"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: reviews
---

## Summary

Ticket: QSL-1469. PR agent-ix/quire-spec-language#2125, head 4de61267d, one commit over origin/main. Code review with the Rust review lane folded in. QSpec: fresh origin/main b666b75d.

The PR replaces whole-view retention (`record_keys()`) in `Lowering::operation_effect_nodes` with `AdmittedModel::operation_effect_declarations()`: each projected operation's declaring owner, the owner of each written field (or the relationship itself), and each `creates`/`deletes` subject. Alias-less models take `generated_location()`. The variant_types test returns to one `variant_type` node, a new owners test covers the alias-less rows, and the admission corpus gives the process `Fulfilment` an operation so its `Model(Process)` row still emits.

## Verdict

**CHANGES REQUESTED** (FND-001).

What holds, measured:

- The retained set matches exactly what `checked_model_operation_effects` (check/mod.rs) puts in a row: `declaring`, `modifies` (field owner or relationship), receiver `modifies` (field owner), `creates`, `deletes`. No row reference can dangle. Probes (scratch tests, reverted) of an operation that modifies `Gauge/d`, creates `Gauge` or deletes `Gauge` with no unit clause naming `Gauge` read back `Verified` at the PR head.
- qsl-package lib, full: 521 passed, 0 failed, 2 ignored. That covers timed, hybrid, protocol, refinement, state-transition, child-composite and abstraction fixtures that go through `operation_effect_nodes`.
- Whole-view retention existed only since #2083 (aaaef0ccc). The only corpus row that depended on it is `Model(Process)` from #2111. The `StateMachine` row still emits from the refinement fixture's own operations. Giving `Fulfilment` an operation roots its process node through the declaring-owner path, so the edit still exercises a real QSL emission route. Moving `Process` back to `NOT_EMITTED_BY_QSL` would be the false classification. The edit is legitimate and not a weakening.
- No `#[allow]`, ignored test, compatibility path or pin in the diff.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high     | FR-094-AC-2's test `check::lowering::model::tests::reference_types_key_over_their_model_nodes_and_record_the_correspondence` (TC-417) fails at the PR head. The graph holds the model nodes Invoice, Order and Sub; the AC requires exactly two (Order, Invoice). `acme/orders` declares operations `Order/size`, `Order/count` and `Sub/size`, so the new retention roots Order and Sub as FR-496 declaring nodes. FR-094 (lines 136-140) roots only declarations a checked node or an ADR-013 O-08 frame subject names, and an FR-496 row is neither, so the PR's "FR-094" justification does not cover this retention and the spec contradicts the code. It also fails on origin/main e7b7a57e3, where billedTo is extra too, so main is already red. The PR narrows the failure but does not fix it. This needs a decision: amend FR-094 to make FR-496 row targets roots and restate AC-2/TC-417, or keep the AC and choose another way to satisfy IR's join. | qsl-semantics/src/check/lowering/model.rs:2064-2086; qsl-semantics/src/check/lowering/model/tests.rs:483-491; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:136-140,412 |

## FND-001 detail

Measured with `cargo test -p qsl-semantics --lib` at 4de61267d: 749 passed, 1 failed (the test above). Left `[Invoice, Order, Sub]`, right `[Invoice, Order]`. On origin/main e7b7a57e3 the same test fails with left `[Invoice, Order, Sub, billedTo]`.

The round-1 review ran only the qsl-semantics `dispatch|operation_effect` filter, so it did not see this test.
