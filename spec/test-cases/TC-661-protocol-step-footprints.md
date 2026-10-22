---
id: TC-661
title: "Protocol steps carry static footprints, enabling footprints and visibility"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: verifies
---
# TC-661: Protocol steps carry static footprints, enabling footprints and visibility

## Description

Verify per-step read and write footprints, enabling footprints, independence and visibility over the protocol locations.

Scope: FR-216-AC-1 to FR-216-AC-7.

## Test Procedure

1. Read the footprints of `attempt(A)` and `attempt(B)` in `Fill` and in
   the two-object variant; compute independence and visibility for a claim
   over `d.v`.
2. Read `fork(Both)`'s and `join(Both)`'s footprints; compute visibility
   for `always holds(k.v <= 3)`; read the branch-end, `finish` and
   second-`fork(P)` footprints and the dependence of the fork with the
   branch-end step.
3. Read `join`'s write footprint under `cancel` and `continue`, and with
   a forked inner `parallel` under `cancel`; read the
   `receive` enabling footprint and the `Chatter` independence pairs.
4. Read §7.1's registration footprints, and the `cend` and second-activation
   footprints of FR-216-AC-4's two-registration `Refund` variant and the
   `commit Committed` variant's dependence of the commit step and `cattempt`;
   for each step of §7.1 and each model location outside its `R`, run the
   pre-state and the one-location-perturbed pre-state, compare enabledness,
   pair the successors by transition identity and by their values outside the
   location, and diff the post-states against `W`.
5. Read the enabling footprints of FR-216-AC-5's `bump` attempt and of
   `duplicate(ch, Tx)`, the reference-type `event`'s membership read, the
   `event` and `finish` constraint reads of a model field and
   the channel, template and role visibility cases.
6. Read the footprint of a `retire` step and the enabling footprint of an
   `activate` step with FR-216-AC-6's predicates, and test the dependence
   with a writer of the field they read, and a `send` from the replicated
   role against a `spawn` of it.
7. Read the `cattempt(Refund, r)` and `Committed` footprints of FR-216-AC-7's
   `attempts 2` variant, test the dependence of the commit step with the
   thread's `cend` and `cattempt`, and read the footprint of a `cattempt`
   whose attempt type holds a `Reference<Peer>`.

Tag the tests `#[trace("TC-661", "FR-216-AC-n")]`.

## Expected Results

- Step 1: dependent in `Fill`; independent in the variant, `attempt(A)`
  invisible and `attempt(B)` visible.
- Step 2: the footprints of FR-216-AC-2, including the earlier-ordinal
  reads of the second fork; control steps invisible.
- Step 3: the footprints and pairs of FR-216-AC-3, the inner threads and
  instances in the cancel case.
- Step 4: the `reg` footprints, the dependent `cend`/activation pair and the
  dependent commit step/`cattempt` pair of FR-216-AC-4; for every perturbation
  outside `R(s)`, equal enabledness, successor pairs that differ at most at the
  perturbed location, and every location outside `W(s)` unchanged from the
  pre-state in both runs.
- Step 5: exactly the enabling locations FR-216-AC-5 lists, as
  `Footprint` values with `Location::Protocol` for protocol locations.
- Step 6: the read locations FR-216-AC-6 lists; the writer of a read field is
  dependent with the `retire` step; a `send` from the replicated role reads
  `role(w)` and is dependent with a `spawn` of that role.
- Step 7: the locations FR-216-AC-7 lists; the commit step dependent with
  the thread's `cend` and `cattempt`; the `Peer` membership in the
  reference-typed `cattempt`'s read footprint.
