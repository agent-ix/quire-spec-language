---
id: TC-202
title: "value::ieee and value::division evaluation is unchanged by negotiate_* removal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-078
    type: verifies
---
# TC-202: value::ieee and value::division evaluation is unchanged by negotiate_* removal

## Description

Verify that removing the `negotiate_*` functions from `value::ieee` and
`value::division` leaves every evaluation function's input/output behavior
unchanged. Scope: FR-078-AC-3.

Catches an over-aggressive removal that deletes or breaks evaluation logic
that happened to sit near the removed negotiation functions (for example,
by deleting a shared helper both the evaluator and the negotiation
predicate called, without noticing the evaluator still needs it).

Verified by the pre-existing evaluation tests in `tests/ieee_profiles.rs` and
`tests/integer_division.rs` that exercise `evaluate_ieee`, `compare_ieee`,
`convert_ieee_width`, `exact_to_ieee`, `ieee_to_exact`, `divide` and `modulo`
(QSL-131 added the `TC-202`/`FR-078-AC-3` tags to their existing `#[trace]`
attributes, alongside each test's original FR-148/FR-147 tags): these tests
passed unchanged before and after the `negotiate_*` removal, which is this
criterion's before/after comparison. The one admission-focused test in
`tests/ieee_profiles.rs`
(`semantic_admission_refuses_missing_repeated_mismatched_or_reserved_bindings`)
is excluded, since it exercises admission, not evaluation.

## Test Procedure

1. Record the evaluated results of a fixture set of IEEE arithmetic value
   expressions (covering normal values, negative zero, NaN and infinities)
   and integer-division value expressions (covering exact division,
   truncation and division-by-zero handling) evaluated through
   `value::ieee` and `value::division` on `main`, before the removal.
2. Apply the `negotiate_*` removal change.
3. Re-run the identical fixture set through `value::ieee` and
   `value::division` after the removal.
4. Compare the before and after results.

## Expected Results

Every fixture's evaluated result after the removal is identical to its
result before the removal, for both modules and across all covered cases.
