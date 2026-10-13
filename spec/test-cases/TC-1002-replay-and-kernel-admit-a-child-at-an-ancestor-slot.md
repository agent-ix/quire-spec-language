---
id: TC-1002
title: "Replay decodes a child at an ancestor slot and no kernel check refuses a conforming value"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: verifies
---
# TC-1002: Replay decodes a child at an ancestor slot and no kernel check refuses a conforming value

## Description

Verify that replay admits, round-trips and refuses a record at an ancestor-typed slot by conformance, and that the checked-invariant causes now mean non-conformance. Scope: FR-359-AC-9, FR-359-AC-10.

Catches a decoder that reads the child's fields against the parent's field list, one that admits by exact type, and one that reports a checked-invariant fault for a conforming value.

## Test Procedure

1. Admit a snapshot whose `Order.total: Money` holds `{"record":{"type":".../TaxedMoney","fields":{...}}}`, with the same record as the payload of `Order.discount: Money?` and as an element of a `Sequence<Money>` field and of a parameter. Encode the observation and decode it again.
2. Re-run step 1 with `type` naming an unrelated record, an ancestor of the slot's type where one exists, and an object type.
3. Re-run step 1 with `tax_minor` removed from the `TaxedMoney` payload, then admit a `Money` payload without `tax_minor`.
4. Run `a == b` and an optional-field projection over the step 1 values in the evaluator.
5. Hand-build a checked package whose `Option<Money>` payload is a `Fee`, and one whose field read names a field absent from the value's declaration, and run each.

## Expected Results

- Step 1 admits every slot, round-trips to the same typed observation and digest, and the re-encoded `type` still names `TaxedMoney`.
- Step 2 refuses `invalid_runtime_input`/`wrong-value-kind` at the first path reached in field order.
- Step 3 refuses `missing-member` for the `TaxedMoney` payload and admits the `Money` payload.
- Step 4 reports neither `EqualityOperandSourceNotAdmitted` nor `field-option-payload-rejected`.
- Step 5 reports `field-option-payload-rejected` for the `Fee` payload and `field-slot-unresolved` for the absent field.
- Assertions compare typed causes and member paths, never message text.