---
id: TC-1402
title: "Check weighted DTMC long-run certificates against independently evaluated weights"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-199
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
---
# TC-1402: Check weighted DTMC long-run certificates against independently evaluated weights

## Description

Specify acceptance and adverse controls for FR-201-AC-31 to AC-34's
weighted finite workload profile. These are implementation obligations,
not execution evidence. The checker independently rebuilds weights and
bottom components; it does not import a solver's stationary vector.

## Test Procedure

1. Admit FR-199-AC-3's original Steady availability graph, with up before
   down in canonical complete-key order. Define the checked named duration
   as 1 on every outgoing tick and 3 on every outgoing repair. Check the
   AC-31 exact weighted document at both equality bounds and its three
   strictly interior beyond/control thresholds. Independently verify
   `pi_up=1800/1801`, `pi_down=1/1801`, denominator `1803/1801`, ratio
   `600/601` and both centered state equations. Change value, gain and bias
   separately, including the unweighted certificate and zero gain.
2. Admit AC-32's periodic, draw-weighted two-state graph. Evaluate BOTH
   typed draws at a, so w(a)=2 and w(b)=1. Check its component equations
   and canonical bias. Swap the equally probable weights, then change one
   from 3 to 5, without trusting stored producer costs. Add transient
   zero-weight s going to a with probability 1/4 and an all-P recurrent
   state c with probability 3/4, where c's explicit probability-one
   self-step carries R 1; check harmonic V(s).
   Separately add an initial-reachable all-zero closed component, and a
   reachable terminal stutter. Retain controls where only transient
   positions have zero weight.
3. Serialize valid documents and independently check them in a fresh
   process without qsl-analyze/provider linkage. Apply each AC-33 mutation
   independently, including missing top-level values and a missing
   component bias/gain/value; assert typed member errors, not merely
   nonacceptance. Shift component biases by a nonzero constant to preserve
   equations but break normalization. Multiply all weights by a positive
   scalar: preserve rho but require correspondingly scaled bias. Use
   actual checked quantity weights in a declared and a compound unit;
   normalize equivalent expressions through their existing exact unit
   graph, and reject a replacement objective descriptor even if dimensions
   or display names match. Retain byte-equal earlier-profile controls.
4. At observed operation phases, exercise actual caller byte, graph,
   arithmetic, scalar and cumulative work/storage limits just below each
   measured requirement, with uncancelled/adequate controls. Cancel before
   admission and during key construction, draw/weight/unit evaluation,
   SCC/membership and weighted row sums; use actual phase events rather
   than a fixed poll offset and preserve Requested/Deadline causes.
   Include undefined/negative weights at both recurrent and transient
   states, absent rewards, unsupported kinds/shapes, and direct zero over
   instances. Distinguish FR-187's earlier finite-domain negative-reward
   admission refusal from defensive post-admission weight checks; do not
   relabel the earlier refusal as profile unsupported. Exercise integer
   representation and exact weight arithmetic
   limits separately; weights may exceed one but their arithmetic is not
   exempt from scalar/rational ceilings.

Executable assertions SHALL carry actual criterion trace attributes, for
example `#[trace("TC-1402", "FR-201-AC-31")]`. This procedure neither
authors a matrix nor asserts whole-runtime allocation qualification.

## Expected Results

- Step 1 accepts rho/gain `600/601` and normalized down bias `-2000/601`.
  Both equality bounds and AtMost `999/1000` accept; AtLeast `999/1000`
  and AtMost `599/600` reject `ThresholdMissed` at the initial key.
  All thresholds lie strictly in (0,1). Altered equations, gain or values
  reject at the specified complete-key locus.
- Step 2 accepts `2/3` with biases `0,-2/3`, preserves that acceptance
  after the equal-probability swap, and rejects the changed mean weight.
  It accepts transient V(s)=`11/12` and rejects `8/9`, the ratio formed by
  pooling component numerators and denominators. A zero-weight BSCC or terminal
  preserves `Unsupported(ZeroWeightComponent)` and produces no document;
  zero transient weights alone remain admissible.
- Step 3 preserves exact canonical valid bytes and independently checks
  actual weights/units and complete BSCC membership. Identity conflict
  refuses before enumeration; objective conflicts refuse mismatch;
  malformed members retain `Malformed` at `values`/`components`; false
  equations/coverage retain the first failing complete-key locus. Source
  changes alone do not invalidate otherwise matching, independently valid
  evidence. Existing profiles are unchanged.
- Step 4 retains typed stop/refusal/unsupported outcomes and actual caller
  cause/consumption without accepted truth values. No invalid weight is
  silently zeroed. Zero direct instances retain `UnsupportedBinding` and
  an unsupported EN-5 disposition without a terminal verdict, certificate,
  counterexample or invented binding. No weighted proof becomes an
  unweighted document or native/shared-wire refutation.
