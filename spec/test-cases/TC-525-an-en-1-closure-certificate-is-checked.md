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

Scope: FR-338-AC-1 to FR-338-AC-7. Canonical translation-state cases below
are prospective qualification procedures; this artifact claims no execution.

## Test Procedure

1. Check the certificate of FR-126-AC-2's TP-1 proof over ADR-018 §6's
   subject.
2. Check it with the product state for model state `(1, 0)` removed; with
   the initial product state removed.
3. Check a certificate holding every reachable `Counter` state for the
   deadlock-freedom item, with no `terminal` member; then the same states
   for `always holds(c.value <= 3)`.
4. Check a certificate holding `t1` for `always holds(true)` over
   `test/tallies`.
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

## Status

🚧 Planned.
