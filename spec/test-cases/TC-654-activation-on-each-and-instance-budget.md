---
id: TC-654
title: "Activation on each starts instances under max_live_instances and states the bound used"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: verifies
---
# TC-654: Activation on each starts instances under max_live_instances and states the bound used

## Description

Verify per-trigger activation, instance ordinals and removal, the instance bound's default, its V-6 cause and precedence, and its statement in every terminal record.

Scope: FR-209-AC-1 to FR-209-AC-4.

## Test Procedure

1. Check FR-209-AC-1's `on each` protocol and claim with
   `max_live_instances` 1, 2 and unset; read each terminal record.
2. Check the guard-`false` variant, and, with the guard back at `true`,
   the refuted claim with bound 1.
3. With bound 2, read instance ordinals across activation, finishing and
   reactivation; run the deadlock-freedom item.
4. Run FR-209-AC-4's combined-bound cases and compare obligation
   identities across bounds.

Tag the tests `#[trace("TC-654", "FR-209-AC-n")]`.

## Expected Results

- Step 1: V-6 `InstanceBoundReached` naming `max_live_instances` 1 and 2; the unset run's
  record states 3, reached.
- Step 2: V-1 with 3, not reached; V-4 with `activate(Tick)`,
  `attempt(set)`.
- Step 3: ordinals 0 and 1, then 0 reused; no instance-limited state
  reported as a deadlock.
- Step 4: `MemoryBoundReached` first, then `InstanceBoundReached` over
  `BoundReached`; equal identities.
