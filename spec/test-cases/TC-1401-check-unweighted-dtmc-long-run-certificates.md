---
id: TC-1401
title: "Check unweighted DTMC long-run documents against independent bottom components"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-199
    type: verifies
---
# TC-1401: Check unweighted DTMC long-run documents against independent bottom components

## Description

Specify the positive, adverse and resource-control checks for the closed
unweighted workload DTMC profile in FR-201-AC-27 to FR-201-AC-30.
These procedures are acceptance obligations, not a claim that the new
profile or its bounded key owner is already implemented.

## Test Procedure

1. Compile and admit ADR-028 §15.3's availability claim under Steady. Use
   fixture model identities whose complete key ordering puts up before
   down. Produce and independently check the AC-27 component document,
   including both inclusive threshold sides. Offer it at the two
   beyond-side thresholds. Alter each bias/gain/value separately and try
   a reachability-profile document for the same source.
2. Compile and admit AC-28's three-state graph under a fully resolving
   workload, and its periodic two-state graph. Check exact component
   boundaries, terminal stutter, centered equations and transient values.
   For each valid document separately merge, split, remove or duplicate a
   component; insert s in a component; alter only V(s); alter both a
   recurrent value and all harmonic V rows while leaving the predicate
   unchanged. Keep a control with correct equations and coverage.
3. Write canonical bytes and check them in a fresh process through the
   qualified source/document entry without linking qsl-analyze or the
   provider. Apply every malformed, identity, binding and objective
   mutation of AC-29 separately. Shift all biases in one component by the
   same nonzero constant to preserve its equations but violate canonical
   normalization. Retain byte-for-byte controls for each older profile.
4. Exercise actual caller source/document/key, state/edge, graph
   work/storage and exact scalar/rational ceilings just below the measured
   requirement of each pass. Run pre-cancellation and cancellation during
   each operation listed in AC-30 using observed phase entry rather than
   a fixed poll count. Include undefined reachable P and workload inputs,
   each excluded shape/kind/profile (including weighted claims), and zero
   admitted instances. Independently retain FR-199's existing weighted
   zero-weight component control through its own solver entry.
   Preserve explicit typed outcomes and
   control consumption rather than merely testing that acceptance failed.

Tag executable assertions with `#[trace("TC-1401", "FR-201-AC-n")]`
using their actual criterion. Retain the existing FR-199-AC-1/TC-634
original-gain assertion; no test should rewrite that result to zero.

## Expected Results

- Step 1 accepts exactly the AC-27 conditions at equality and rejects the
  beyond-side thresholds with `ThresholdMissed`. Component equations use
  centered gain zero while the transported gain remains 1800/1801.
- Step 2 accepts values 1/3,1,0 with singleton BSCCs and accepts the
  periodic chain at 1/2. Independent BSCC comparison rejects false
  partitions. Checking only harmonic values cannot admit forged recurrent
  boundaries. Terminal P has value 1 and terminal not-P value 0 through
  infinite stutter; P truth does not stop the original live transitions.
- Step 3 round-trips valid documents exactly and rejects malformed
  encodings/coverage/normalization. Wrong identity refuses before
  enumeration; wrong objective prevents acceptance. Each failure retains
  FR-201's rule and full-key/member locus. Equivalent independently
  checked source changes may accept, and older profile controls are
  unchanged.
- Step 4 preserves typed limit/cancellation/undefined/refusal or explicit
  unsupported outcomes without truth values or an accepted document.
  A weighted zero-weight case remains `Unsupported(ZeroWeightComponent)`.
  Zero instances retain vacuous settlement without invented evidence.
