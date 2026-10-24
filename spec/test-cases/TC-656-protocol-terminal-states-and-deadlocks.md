---
id: TC-656
title: "Protocol terminal declarations check and protocol deadlocks are reported with blocked threads"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: verifies
---
# TC-656: Protocol terminal declarations check and protocol deadlocks are reported with blocked threads

## Description

Verify the protocol `terminal` member, the terminal, intended and deadlocked classification, the `on each` deadlock clause, boundary states, and the deadlock-freedom item with its blocked threads.

Scope: FR-211-AC-1 to FR-211-AC-5.

## Test Procedure

1. Check `Fill` with each `terminal` form, with two members, and with a
   predicate reading binder `a`.
2. Classify s2 and s6 under each form; run the deadlock-freedom item and
   read its counterexample's blocked threads.
3. Run the item over the repaired `Fill`; write a request with two items
   over one subject.
4. Run FR-211-AC-4's `on each` protocol with bound 2.
5. Run FR-211-AC-5's channel protocols, each channel declared with the
   stated delivery policy, and read each deadlock counterexample's blocked
   threads. Run FR-211-AC-5's `Pay` compensation fixture and its
   empty-population replicated-role fixture and read their blocked threads.
   Call the cause function directly on the states FR-211-AC-5 gives with a
   `spawn` enabled and at a `seq_cst` `fence` under `tso` with a non-empty
   buffer; neither is a deadlock state.

Tag the tests `#[trace("TC-656", "FR-211-AC-n")]`.

## Expected Results

- Step 1: `None`, `When`, `Any` with pairwise different identities; the
  `ambiguous_declaration`/`ambiguous-name` refusal of the second member
  and the non-state-read refusal.
- Step 2: s2 deadlocked under `None`, intended under `When(k.v = 1)`;
  `refuted` with the blocked threads of FR-211-AC-2; no item under `Any`.
- Step 3: `proved`, basis `Exhaustive`, ten states; one item.
- Step 4: the one-instance waiting state is a deadlock with cause
  `Precondition`; the instance-limited and initial states are not.
- Step 5: `FullQueue`, `HeadBlocked`, `Constraint` (FIFO and `unordered`),
  `Displaced` (displacement, `lost` send and `lose` step) and `EmptyQueue`
  (after a loss then a `receive`, and with nothing sent), each naming the
  channel in its qualified name, one cause per thread. The compensation
  thread `(0, Refund, 0)` is `CommitClosed{Refund}` and the root `(0)` is
  `FinishWaiting` naming it; `NoRoleInstance{r}` for each of the five node
  kinds with the population empty or retired, and for a `receive` instead of
  `EmptyQueue`. The cause function called on the `spawn`-enabled state gives
  no `NoRoleInstance`, and on the `fence` state gives `Gate` and no other
  cause.
