---
id: TC-650
title: "The protocol subject builds, refuses bad bindings and keys states canonically"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: verifies
---
# TC-650: The protocol subject builds, refuses bad bindings and keys states canonically

## Description

Verify the protocol subject's construction and refusals, the thread paths and ordinals, binder removal and the canonical state key.

Scope: FR-205-AC-1 to FR-205-AC-4.

## Test Procedure

Use ADR-027 §7's `Fill` unit (universe `{c}`, `c.v = 0`).

1. Build the subject; build variants differing in `over` and in the
   `terminal` member and compare obligation identities; build with an absent
   protocol name, an unbound `w`, and `k` bound outside the universe.
2. Read the initial key; take `fork(Both)` and read the key; serialize it
   twice; read `observations` in every reachable state; repeat with the
   variant whose post-join node reads `a`.
3. Run FR-205-AC-3's `repeat`/`join any … outstanding continue` protocol and
   read the paths created by each fork under both completion orders.
4. Build `Fill` with two initial snapshots, and an `on each` protocol.

Tag the tests `#[trace("TC-650", "FR-205-AC-n")]`.

## Expected Results

- Step 1: the subject builds; identities differ; the three refusals of
  FR-205-AC-1, each naming its protocol, role or parameter.
- Step 2: the keys of FR-205-AC-2, sorted by JCS bytes and byte-equal on
  re-serialization; `observations` empty throughout `Fill`; in the variant
  the binder is present exactly from `attempt(A)` to the reading step.
- Step 3: ordinal 1 while instance 0's `b` runs, ordinal 0 otherwise; the
  two histories meet in one state.
- Step 4: two initial states at `fork Both`; no instance in the `on each`
  initial state.
