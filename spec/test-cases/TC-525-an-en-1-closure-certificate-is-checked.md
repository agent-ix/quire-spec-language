---
id: TC-525
title: "An EN-1 closure certificate is accepted or rejected by the core checker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
---
# TC-525: An EN-1 closure certificate is accepted or rejected by the core checker

## Description

Verify that `check_closure` accepts a safety proof's closure and rejects a
missing initial state, a missing successor and a bad state.

Scope: FR-338-AC-1 to FR-338-AC-7 and FR-338-AC-10. Canonical translation-state cases below
are prospective qualification procedures; this artifact claims no execution.

## Test Procedure

1. Check the certificate of FR-126-AC-2's TP-1 proof over ADR-018 §6's
   subject.
2. Check it with the product state for model state `(1, 0)` removed; with
   the initial product state removed.
3. Check a certificate holding every reachable `Counter` state for the
   deadlock-freedom item, with no `terminal` declaration; then the same states
   for `always holds(c.value <= 3)`.
4. Check a certificate holding values 0 to 2 for `always holds(true)` over
   the `Counter` subject with `pre CanInc { 6 mod (2 - self.value) >= 0 }`.
5. Derive each FR-338 canonical-vector context from its actual checked
   formula/profile/activation/binding; compare the full UTF-8 JCS key
   octets and the stated exact state-body octets. Construct equivalent
   subsets and window debts in reverse insertion/materialization order.
6. Supply every listed malformed/context negative. Keep two distinct
   operand occurrences of one shared interval subformula, changing one
   occurrence's debt only; change past memory and Büchi membership only.
7. Read `eventually[0,1] p` over false/true and false/false letters and at
   a terminal position under false-extension. Read it on each with a
   prior activation still open when a new activation is created. Compare
   verdicts with the one trace evaluator; under infinite-trace compare
   the terminal stutter letter instead of bounded false-extension. At
   origin compare `historically[0,1] holds(p)` for p true with
   `historically[0,1] true`, distinguishing missing atomic history from constants.
   Also read open `p until[1,*] q` and finite `p until[1,1] q` on
   p-false origin/q-true distance one and check their different prefix
   semantics and release dual. Insert a false fork before the counted
   p-true step; at distance one insert an uncounted p-true letter after
   a false letter. Compare all horizons/ages before and after a memory
   step, and compare mixed-truth protocol past bucket transforms.
   Offer a full closure for origin `eventually[1,1] true` on a real
   uncounted protocol loop under adversarial scheduling/no fairness,
   with and without a counted exit; compare with a graph where each
   cycle contains a counted step. Compare origin-discharged F[0,1]true
   and the dual G[1,1]false on the uncounted loop. Check exact auxiliary
   acceptance and replay the nonempty zero-counted lasso; repeat with
   an actual fairness constraint whose enabled/taken test excludes it.
8. Admit u64::MAX interval bounds and b+1 saturation; reach the actual
   automaton-state/identity-byte budget while encoding a reachable state.
   Exhaust the auxiliary graph max_states budget and the actual evaluation
   meter separately; cancel through FR-276 first with requested, then with
   deadline. Pass each typed stop through FR-127 and through the FR-339
   closure call; compare exact limit/value/setting or cancellation source.
9. Over the `Counter` subject with no `terminal` declaration, build a
   checked package of its unit carrying each guarded clause's
   `temporal_clause` node in QSpec FR-370's shape (`kind` `each`, `trigger`
   the `over` parameter's node, `guard` the clause's guard term); use it as
   the engine's clause source and as the request's recompiled package, and
   read each `when` condition by the test's own letters. Check the
   certificates of FR-338-AC-10: `always holds(c.value <= 1)` infinite-trace
   `on each` when `c.value = 7`; `eventually[0,1] holds(c.value = 3)`
   false-extension `on each` when `c.value >= 2`, then with its value-2 body
   replaced by the obligation-free body, then with that misread state added
   beside the true one, then with every key's activation replaced by
   `"each"`; then, with the condition false at values 0 and 1 and undefined
   at value 2, the certificate of exactly the value-0 and value-1
   obligation-free `"each-when"` states.

Tag the tests `#[trace("TC-525", "FR-338-AC-n")]`.

## Expected Results

- Step 1: accepted.
- Step 2: `SuccessorMissing` at `(1, 0)`; `InitialMissing` at the initial
  state.
- Step 3: `BadState` at value 3; accepted.
- Step 4: `BadState` at `t1`.
- Step 5: exact authored octets; reversed materialization yields equal
  keys. Different counters, paths, past memory and acceptance membership
  yield different full keys. No local graph index is an identity.
- Step 6: malformed, unsorted, duplicate, wrong-context and unjustified
  decided-state inputs refuse before certificate membership is used.
- Step 7: accept then reject respectively; terminal semantics match the
  evaluator and a new activation does not erase an older open debt.
  The two historical origin cases are false and true respectively.
  Open until rejects at origin while finite until accepts; release
  preserves its dual. Fork/memory letters do not reduce distance, and
  a final-region zero-distance true letter can satisfy the pending F.
  The unresolved F[1,1]true starvation closure rejects WitnessFails,
  even with a counted exit; counted-progress cycles discharge it.
  F[0,1]true and the empty-window G dual do not falsely reject. Only
  the actual resolved fairness test can exclude a starvation lasso.
- Step 8: exact mathematical strings, including b+1 above u64::MAX;
  an explicit resource stop gives no truncated/default key.
  Also exhaust the auxiliary graph/state/automaton/evaluation budget
  before acceptance and cancel with Requested and Deadline at a charge.
  The typed Stopped result preserves the exact configured limit, charged
  value/setting or cancellation source; FR-127 settles Incomplete with
  those fields, never Accepted or CertificateRejected.
- Step 9: accepted with every body `["safety",[[[],[],[],[]]]]` and
  activation `"each-when"`, settling `proved`; accepted with the value-2
  deadline body; `SuccessorMissing` at the value-2 deadline state; accepted
  with the extra misread state; `ShapeMismatch` at the least refused state;
  `BadState` at the value-1 product state.
