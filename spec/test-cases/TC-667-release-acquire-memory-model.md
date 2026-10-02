---
id: TC-667
title: "The ra memory model explores messages, views, fences and garbage collection"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: verifies
---
# TC-667: The ra memory model explores messages, views, fences and garbage collection

## Description

Verify `Ra`'s loads, stores, RMWs, fences, fork, join and channel rules, garbage collection, and the litmus outcomes of its column against QSpec FR-437's expected outcomes.

Scope: FR-222-AC-1 to FR-222-AC-4.

## Test Procedure

1. Check `SB` under `ra`; count states; read the counterexample's depths,
   messages and views.
2. Run each litmus test of ADR-025 §3.3 under `ra` and compare its
   outcome with QSpec FR-437's expected outcome.
3. Run the two-thread `fetch_add`; read message counts after `SB`'s join and
   after each step.
4. Run MP with fences and MP over a channel.

Tag the tests `#[trace("TC-667", "FR-222-AC-n")]`.

## Expected Results

- Step 1: 14 + 4 = 18 states; `refuted` with §10's `ra` counterexample.
- Step 2: each outcome equals the `ra` column's expected outcome.
- Step 3: final value 2 in every joined state, no message read by two
  RMWs; one message per location after the join; unreachable messages
  removed.
- Step 4: `proved` twice.
