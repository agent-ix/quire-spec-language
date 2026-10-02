---
id: TC-653
title: "Replicated role instances spawn under max, act, retire by lifetime and key by object"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: verifies
---
# TC-653: Replicated role instances spawn under max, act, retire by lifetime and key by object

## Description

Verify spawning under the authored `max`, acting by a live instance, each lifetime's retirement, and the object-reference key.

Scope: FR-208-AC-1 to FR-208-AC-3.

## Test Procedure

1. Run FR-208-AC-1's protocol (`max 1`, `until(p)`, universe `{w1, w2}`)
   and list enabled `spawn` and `retire` steps and the `roles` member.
2. With `max 2`, spawn in both orders and compare keys; list `event(Work)`
   steps with two live instances.
3. Run the `workflow` and `scope` lifetimes.

Tag the tests `#[trace("TC-653", "FR-208-AC-n")]`.

## Expected Results

- Step 1: the spawn and retire enabledness of FR-208-AC-1; `w1` stays
  `retired` and is never spawned again.
- Step 2: equal keys; two `event(Work)` steps naming `w1` and `w2`.
- Step 3: no role instance in the state after `finish`, which removes the
  protocol instance; under `scope`, retirement
  folded into the step that ends the region's reachability, and no
  `retire` step.
