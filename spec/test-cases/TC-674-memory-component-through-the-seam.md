---
id: TC-674
title: "The memory component, observations, atoms and footprints flow through the seam"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: verifies
---
# TC-674: The memory component, observations, atoms and footprints flow through the seam

## Description

Verify the memory member of the state key, the thread observation, atoms over memory values, the `memory` anchor, terminal states under `tso`, and memory footprints and visibility.

Scope: FR-229-AC-1 to FR-229-AC-5.

## Test Procedure

Use ADR-025 §10's `SB` protocol.

1. Read the memory member after `Sx` under `tso`, `ra` and `sc`, and in
   joined states.
2. Evaluate `holds(x.v = 1)` before and after `flush(left)`; read its
   anchor; evaluate `holds(c1.r = 0)`; try the precondition-guarded loads
   of FR-229-AC-2.
3. Apply `compare_exchange(1, 2)` on `x` at 0 under `ra`.
4. Compute visibility and independence for the steps of FR-229-AC-4.
5. Collect `SB`'s terminal states under `tso` and run its deadlock-freedom
   item.

Tag the tests `#[trace("TC-674", "FR-229-AC-n")]`.

## Expected Results

- Step 1: the members of FR-229-AC-1.
- Step 2: false then true; a `memory` anchor; the register value; enabled
  for `left`, not for `right`.
- Step 3: no new message; only the acting thread's view moves.
- Step 4: the visibility and dependence of FR-229-AC-4.
- Step 5: every buffer empty; `proved`.
