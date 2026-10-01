---
id: TC-545
title: "The refinement mapping builds the abstract state from a concrete state and its history"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: verifies
---
# TC-545: The refinement mapping builds the abstract state from a concrete state and its history

## Description

Verify `map_state`: the key-preserving population image, reference lifting,
field rows through the clause evaluator, history reads, hidden slots, and
`MappingFailure::Undefined` for a row that evaluates undefined.

Scope: FR-140-AC-1 to FR-140-AC-4.

## Test Procedure

Fixtures: `CasRefinesCounter` (ADR-020 §8); `RingIsQueue` (FR-139);
`RegisterHistory` (FR-138).

1. Map `(1, 0, f, 0, t)`, `(0, 0, t, 0, f)` and `(0, 0, f, 0, f)` and
   compare keys with states built in `Spec`.
2. Map the ring state of FR-140-AC-2.
3. Map ring states with `size >= 1` under FR-140-AC-3's `only` condition.
4. Map `value = 1`, `last = 0`, with and without the `prev` row.

Tag the tests `#[trace("TC-545", "FR-140-AC-n")]`.

## Expected Results

- Step 1: `c.value = 1` with a key equal to `Spec`'s; the last two keys are
  equal.
- Step 2: one `Queue` `r` with `items = [1, 0]` and no slot objects.
- Step 3: `MappingFailure::Undefined` naming the `items` row, `r` and the
  undefined cause.
- Step 4: `value = 1`, `prev = 0`; then `hidden = [(r, prev)]`.
