---
id: TC-900
title: "The SMT-LIB transition-relation encoding is canonical and refuses unencodable constructs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
---
# TC-900: The SMT-LIB transition-relation encoding is canonical and refuses unencodable constructs

## Description

Verify that `encode_smt_query` prints the unrolling and induction queries
in canonical order and bytes, and refuses a construct it cannot encode.

Scope: FR-315-AC-1 to FR-315-AC-5.

## Test Procedure

1. Encode `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)` over
   `Counter`, twice.
2. Encode `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for
   `always holds(c.value <= 3)`.
3. Encode an item over a field of an unencodable sort.
4. Encode `Unrolling{depth: 2}` for `always holds(value(s.held) <= 3)` over
   FR-315-AC-4's `Slot` subject.
5. Encode `InductionStep{depth: 2}` for `always holds(c.value <= 3)` and for
   a TP-3 item.

Tag the tests `#[trace("TC-900", "FR-315-AC-n")]`.

## Expected Results

- Step 1: the declarations, assertions and `(check-sat)` FR-315-AC-1 lists;
  byte-equal scripts.
- Step 2: the base asserts the initial predicate on copy 0; the step
  asserts none, the property at 0 and its violation at 1.
- Step 3: `SmtEncodingRefusal` naming the field, no script.
- Step 4: `s0.s.held.present` to `s2.s.held.present` declared; the negated
  property includes `(not s<i>.s.held.present)` at each position 0 to 2,
  and the `take` step's guard contributes the same disjunct; every term is
  linear.
- Step 5: pairwise distinctness of copies 0 to 2; for the TP-3 item, `m0`
  to `m2` with the monitor steps.

## Status

🚧 Planned.
