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

Scope: FR-224-AC-1 to FR-224-AC-7.

## Test Procedure

Use ADR-025 §10's message-passing shape.

1. Request a claim with `data` non-atomic and a `release`/`acquire` flag
   under `ra`; read the items and the race-freedom verdict.
2. Repeat with a `relaxed` flag under `ra` and under `tso`.
3. Request claims over `SB` under `tso`, over the step 1 shape under `sc`,
   and two items over one subject under `tso` and `ra`.
4. Check the authored claim of FR-224-AC-4.
5. Under `ra`, with a `release` and then a `relaxed` flag store, read the
   state key's memory member after the writer's `data := 1`, after its flag
   store, and, with the `release` store, after an `acquire` flag load that
   reads the new message; repeat with the flag passed by a `send` and
   `receive`, reading the queue entry; read it for `SB`.
6. Repeat step 5 under `tso`, reading the state also after `flush(writer)`,
   and with the flag passed by a `send` and `receive`; read each verdict.
7. Under `ra` and under `tso`, run thread `a` loading `x` non-atomically then
   storing `x := 1` with `release`, and thread `b` running `fetch_add(1)` on
   `x` with the precondition `x.v = 1`, so it reads only `a`'s store, with
   `ordering acquire`, then `relaxed`.

Tag the tests `#[trace("TC-669", "FR-224-AC-n")]`.

## Expected Results

- Step 1: one `RaceFreedom` item, `proved`.
- Step 2: `refuted` with `Race{location: data, earlier, later}` at the
  positions FR-224-AC-2 names, under both models.
- Step 3: no item, no item, then one item per request with distinct
  identities.
- Step 4: `refuted`, kind `Formula`, as with `data` atomic.
- Step 5: the race-summary entry and view `race` members of FR-224-AC-5,
  equal to QSpec FR-438-AC-6's; in the channel variant the queue entry's
  `view` has `data` at `write` true and the item is `proved`; `SB` has
  neither.
- Step 6: the `race-summaries`, element, `race-memory` and queue-entry race
  maps of FR-224-AC-6, equal to QSpec FR-438-AC-7's; `proved` for the
  `release`/`acquire` and channel variants, `refuted` on `data` for `relaxed`.
- Step 7: `proved` with the `acquire` RMW under both models; `refuted`, the
  RMW against `a`'s load being the only race, with
  `Race{location: x}`, `earlier` `a`'s load and `later` the RMW, with the
  `relaxed` RMW.
