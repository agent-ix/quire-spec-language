---
id: TC-670
title: "Memory bounds limit stores, default to 4 and are stated in every result"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: verifies
---
# TC-670: Memory bounds limit stores, default to 4 and are stated in every result

## Description

Verify the memory bounds' default, their statement in the terminal record, bound-limited states as boundary states, and the verdicts and precedence under a bound.

Scope: FR-225-AC-1 to FR-225-AC-4.

## Test Procedure

1. Read the terminal records of ADR-025 §10's three requests; compare
   identities across `max_store_buffer` values.
2. Run FR-225-AC-2's three-store protocol, whose three locations are
   loaded by the other branch, with bound 2 and bound 3.
3. Run `SB` under `tso` with `max_store_buffer` 1 and under `ra` with
   `max_messages` 1.
4. Inspect the bound-limited state of step 2; run the combined state
   constraint and memory bound case.

Tag the tests `#[trace("TC-670", "FR-225-AC-n")]`.

## Expected Results

- Step 1: 4, not reached, for `tso` and `ra`; no memory bound for `sc`;
  equal identities.
- Step 2: V-6 `MemoryBoundReached` naming `max_store_buffer` 2; V-1, not
  reached.
- Step 3: `refuted`; V-6 naming `max_messages` 1.
- Step 4: no deadlock report and no stutter successor; `ConstraintReached`.
