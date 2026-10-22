---
id: TC-651
title: "The protocol system takes each step kind with structural moves folded"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: verifies
---
# TC-651: The protocol system takes each step kind with structural moves folded

## Description

Verify the enabledness and effects of the attempt, event, channel, fork, join, timeout and finish steps, the folding of structural moves and the waiting rule.

Scope: FR-206-AC-1 to FR-206-AC-10.

## Test Procedure

1. Explore ADR-027 §7's `Fill` with `ProtocolSystem<Sc>` and list each
   state's enabled steps.
2. Expand FR-206-AC-2's `bump()` attempt and compare with `ModelSystem`'s
   successors for the same application; run the `contracts`-omission and
   unsatisfiable-binder variants.
3. Run the three-branch protocol under each join policy and `outstanding`
   rule of FR-206-AC-3.
4. Run the channel variants of FR-206-AC-4.
5. Run the `choice`/`repeat`/`check`/`await` protocol of FR-206-AC-5, and
   its `repeat` with no maximum and no invariant or variant, each with an
   `exhausted` control `attempt Z`, and the variant with a following
   `attempt W`.
6. Run FR-206-AC-6's `join any … outstanding continue` protocol and read
   `finish`'s enabledness before and after branch `b` completes.
7. Send at an empty and a full capacity-2 `fifo` channel under each
   overflow policy and read the binder and the send record of FR-206-AC-7.
8. Run FR-206-AC-8's fence protocols under `sc` and `tso`.
9. Run FR-206-AC-9's nested-cancel protocol to the `join(Outer)` step and
   explore every later state.
10. Run FR-206-AC-10's protocol with the `effect` after the `all` join and
    read `event(E)`'s enabledness and the root's environment after `join`.

Tag the tests `#[trace("TC-651", "FR-206-AC-n")]`.

## Expected Results

- Step 1: states s0 to s6 and the enabled steps of ADR-027 §7's table;
  seven states, six transitions.
- Step 2: three `attempt` steps equal to FR-120's successors; the
  omitted-precondition attempt enabled; the unsatisfiable attempt waits.
- Step 3: the enabledness and dispositions FR-206-AC-3 states.
- Step 4: `send`, `receive`, `duplicate` and `lose` enabled exactly as
  FR-206-AC-4 states.
- Step 5: no step for structural moves; the thread takes `X` twice and then
  rests at `Z`, or at `W` in the variant; the
  false `check` waits; `timeout(a)` enabled while waiting; the unbounded
  `repeat` rests at `X` after each attempt and no reachable state rests
  at `Z`.
- Step 6: `finish` not enabled while `b` is `running`; enabled once `b`
  completes, and its step marks the instance finished.
- Step 7: the binder holds the sent payload; the send records of
  FR-206-AC-7, recorded as the step's result and absent from the state key.
- Step 8: the fence enabledness of FR-206-AC-8.
- Step 9: `b`, `x` and `y` `cancelled`, `Inner` removed, no later step by
  any of them.
- Step 10: `event(E)` is enabled after `join`, with `A`'s binder in the root's
  environment.
