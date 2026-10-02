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

Scope: FR-206-AC-1 to FR-206-AC-6.

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
   its `repeat` with no maximum and no invariant or variant.
6. Run FR-206-AC-6's `join any … outstanding continue` protocol and read
   `finish`'s enabledness before and after branch `b` completes.

Tag the tests `#[trace("TC-651", "FR-206-AC-n")]`.

## Expected Results

- Step 1: states s0 to s6 and the enabled steps of ADR-027 §7's table;
  seven states, six transitions.
- Step 2: three `attempt` steps equal to FR-120's successors; the
  omitted-precondition attempt enabled; the unsatisfiable attempt waits.
- Step 3: the enabledness and dispositions FR-206-AC-3 states.
- Step 4: `send`, `receive`, `duplicate` and `lose` enabled exactly as
  FR-206-AC-4 states.
- Step 5: no step for structural moves; two body entries then `exhausted`;
  the false `check` waits; `timeout(a)` enabled while waiting; the
  unbounded `repeat` re-enters its body after each attempt and never
  enters `exhausted`.
- Step 6: `finish` not enabled while `b` is `running`; enabled once `b`
  completes, and its step marks the instance finished.
