---
id: TC-669
title: "Non-atomic locations explore and the race-freedom item reports races once"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: verifies
---
# TC-669: Non-atomic locations explore and the race-freedom item reports races once

## Description

Verify the race summary, the derived race-freedom item, its race counterexample, and that races leave authored claims unchanged.

Scope: FR-224-AC-1 to FR-224-AC-4.

## Test Procedure

Use ADR-025 §10's message-passing shape.

1. Request a claim with `data` non-atomic and a `release`/`acquire` flag
   under `ra`; read the items and the race-freedom verdict.
2. Repeat with a `relaxed` flag under `ra` and under `tso`.
3. Request claims over `SB` under `tso`, over the step 1 shape under `sc`,
   and two items over one subject under `tso` and `ra`.
4. Check the authored claim of FR-224-AC-4.

Tag the tests `#[trace("TC-669", "FR-224-AC-n")]`.

## Expected Results

- Step 1: one `RaceFreedom` item, `proved`.
- Step 2: `refuted` with `Race{location: data, earlier, later}` at the
  positions FR-224-AC-2 names, under both models.
- Step 3: no item, no item, then one item per request with distinct
  identities.
- Step 4: `refuted`, kind `Formula`, as with `data` atomic.
