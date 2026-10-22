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

Scope: FR-205-AC-1 to FR-205-AC-8.

## Test Procedure

Use ADR-027 §7's `Fill` unit (universe `{c}`, `c.v = 0`).

1. Build the subject; build variants differing in `over` and in the
   `terminal` member and compare obligation identities; build with an absent
   protocol name, a request binding supplied for `w`, the variant with
   `role v on M::Cell` and `v` unbound, and `k` bound outside the universe.
2. Read the initial key; take `fork(Both)` and read the key; serialize it
   twice; read `observations` in every reachable state; repeat with the
   variant whose post-join node reads `a`, the `sequence { attempt A;
   attempt A2 }` variant and the nested all-joined `parallel R` variant.
3. Run FR-205-AC-3's `repeat`/`join any … outstanding continue` protocol and
   read the paths created by each fork under both completion orders, and
   its `maximum 3` variant in which instance 0 ends while instance 1 runs,
   and its variant whose branch `b` holds an inner `parallel` with a
   running `y`.
4. Build `Fill` with two initial snapshots, and an `on each` protocol.
5. Activate FR-205-AC-5's two `Refund` registrations with and without
   `cend` between them.
6. Build subjects over two protocols that each declare `channel Pay`, and
   read a key with a queued entry and a `duplicate` identity from each.
7. Run FR-205-AC-7's `repeat` of `P` and `Q` with both values of `a.ready`;
   read the ordinals the second forks of `P` and `Q` take while the first
   `u` rests at `U1`, each `u`'s environment and `Ck` value, and the key's
   `observations`.
8. Run FR-205-AC-8's protocol: activate the two registrations by the two
   triggers in both orders, take each compensation thread's `cattempt` and
   `cend`, then step `Committed` and finish the instance;
   read the keys. Run its `attempts 2` variant with the retry relation
   `not e.ok`: take thread `(0, Refund, 0)`'s first `cattempt` with each
   record, then its retry `cattempt` and `cend`, and, on a separate path,
   step `Committed` while the thread rests at its retry point; read the
   `observations` member and the enabled steps of that thread after each
   step. Repeat with the retry relation `l.ok`.

Tag the tests `#[trace("TC-650", "FR-205-AC-n")]`.

## Expected Results

- Step 1: the subject builds with no binding for `w`; identities differ;
  the four refusals of FR-205-AC-1, each naming its protocol, role or
  parameter.
- Step 2: the keys of FR-205-AC-2, sorted by JCS bytes and byte-equal on
  re-serialization; `observations` empty throughout `Fill`; in the variant
  the binder is present exactly from `attempt(A)` to the reading step, in
  the multi-step variant through the join, and in the nested variant `b` is
  held by `x`, then `left`, then the root.
- Step 3: ordinal 1 while instance 0's `b` runs, ordinal 0 otherwise; the
  two histories meet in one state; with `maximum 3`, the third fork takes
  ordinal 0 once instance 0 is removed while instance 1 runs; ordinal 1
  while `y` runs and 0 after it completes.
- Step 4: two initial states at `fork Both`; no instance in the `on each`
  initial state.
- Step 5: ordinal 1 with `(0, Refund, 0)` live, ordinal 0 after `cend`.
- Step 6: each queue key and identity names its own protocol's qualified
  channel name.
- Step 7: `P` forks at ordinal 0 again and `Q` at ordinal 1; each `u` reads its
  own iteration's `a` (true, then false) and no state refuses; `observations`
  holds no `a` under a completed `u`.
- Step 8: distinct keys for the two activation orders, each thread's entry
  naming its registration; the commit entry from the commit step until the
  instance finishes. In the `not e.ok` variant, the thread's attempt record
  with the first attempt's `ok` value after its first `cattempt`, a retry
  `cattempt` enabled after `ok` false and only `cend` after `ok` true,
  distinct keys for the two records, and no attempt record after the second
  `cattempt`, after `cend` or after the step of `Committed`. In the `l.ok`
  variant, no attempt record in any state, and one state after the two first
  `cattempt` steps.
