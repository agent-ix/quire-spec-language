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

Scope: FR-222-AC-1 to FR-222-AC-5.

## Test Procedure

1. Check `SB` under `ra`; count states; read the counterexample's depths,
   messages and views.
2. Run each litmus test of ADR-025 §3.3 under `ra` and compare its
   outcome with QSpec FR-437's expected outcome.
3. Run the two-thread `fetch_add`; read message counts after `SB`'s join and
   after each step. In `SB` with both threads loading `x` after `Sx`,
   list the messages at the state after both threads' current views have
   passed `x0`, while a message view still points at it.
4. Run MP with fences and MP over a channel.
5. Run the looping store and load of `x` under `ra` with `max_messages` 1.

Tag the tests `#[trace("TC-667", "FR-222-AC-n")]`.

## Expected Results

- Step 1: 14 + 4 = 18 states; `refuted` with §10's `ra` counterexample.
- Step 2: each outcome equals the `ra` column's expected outcome.
- Step 3: final value 2 in every joined state, no message read by two
  RMWs; one message per location after the join; unreachable messages
  removed, including `x0` when only a message view or `S` points at it.
- Step 4: `proved` twice.
- Step 5: `proved`, bound not reached, one live message of `x` in every state.
