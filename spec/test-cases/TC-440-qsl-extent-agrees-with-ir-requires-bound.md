---
id: TC-440
title: "QSL's extent agrees with IR's requires-bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: verifies
---
# TC-440: QSL's extent agrees with IR's requires-bound

## Description

Verify that QSL's extent classification and IR's `requires-bound` agree
over the v2 wire QSL emits. Scope: FR-097-AC-6.

## Test Procedure

1. Check and emit a package of records: `Bag` (unbounded sequence),
   `Counter` (`Integer`), `Box` (all bounded), `Outer` (names `Bag`),
   `Holder` (names `Box`), `Ints` (bounded sequence of `Integer`), `Flags`
   (bounded sequence of `Boolean`), `RangedTree` (recursive) and `Mixed`
   (`Integer` beside `Int[0,9]`).
2. Read it with IR's v2 reader and lower each record
   with `require_bounds`.
3. Classify each record with `classify_extent`.
4. Check and emit `function f using v(x: Int[0, 9], n: Integer): Integer
   pure { (x + 1) + n }`. Read its requirement records (FR-062-AC-13) and,
   with IR's v2 reader, apply IR's `requires-bound`
   predicate to each record's application node. Then check and emit
   FR-062's RR-7 (`let t = x + 1 in t * 2`) and `function k using v(x:
   Int[0, 9], n: Integer): Integer pure { if n = 0 then x + 1 else 0 }`,
   and read their records.

## Expected Results

- IR returns `RequiresBound` exactly when QSL's extent is `Unbounded`, and
  IR's first unbounded node is the form QSL names; for a recursive type it
  is a member of the recursion group of the root QSL names `Recursive`.
- Step 4: IR answers `requires-bound` for the outer `+` node, whose record
  is `Unbounded` at `n`, and not for the inner `+` node, whose record is
  `Bounded`. For RR-7's `*` record (rooted through `t`'s bound value) and
  `k`'s `+` record (rooted at `n` through the guard), IR's predicate is not
  asserted; the records' extents are asserted: `Bounded` for the `*`, and
  `Unbounded` with one `Integer` domain at `n` for `k`'s `+`.

## Status

Partly passed. `Bag` to `Mixed` pass
(`qsl-package/src/emit/extent_agreement.rs`,
`tc_440_qsl_extent_agrees_with_ir_requires_bound`); for `RangedTree` the test
asserts IR's first unbounded node is in the recursion group of the root QSL
names `Recursive`. The quantity fixture `Measure` is emitted with its
`metre` unit and `Length` dimension nodes and IR reads and lowers it
(`tc_440_a_quantity_record_is_emitted_and_reaches_ir`). Its agreement,
`tc_440_quantity_extent_agrees_with_ir_requires_bound`, is ignored: QSL
classifies `Measure` `Unbounded`, and IR's `requires-bound` treats a unit
type as needing no bound, so IR lowers it (IR-450). Step 4 passes: the inner `+` and RR-7's `*` and
`k`'s `+` (`tc_440_operation_application_records_agree_with_ir_per_node`),
and IR's bound requirement for the outer `+` of `(x + 1) + n`
(`tc_440_an_unbounded_application_record_requires_a_bound_in_ir`).
