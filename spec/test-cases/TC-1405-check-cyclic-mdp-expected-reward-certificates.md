---
id: TC-1405
title: "Check finite cyclic MDP first-hit expected-reward certificates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
---
# TC-1405: Check finite cyclic MDP first-hit expected-reward certificates

## Description

Specify FR-201-AC-43 to AC-46's finite every-scheduler reward proof
obligations. These procedures are not execution evidence. Reconstruct
all actual actions/draw rewards and first-B stopped coverage independently;
do not trust producer policy, properness labels or supplied costs.

## Test Procedure

1. Give non-B s two actual actions with the same self/B probabilities
   (1/2,1/2) but named R=1 on both draws of a and R=2 on both draws of b.
   Offer minimum 2 under Every AtLeast 2 and maximum 4 under Every AtMost
   4, B value/rank zero and s rank 1. Test bounds 5/2 and 7/2 respectively.
   Change only entry-B R, then swap equal-probability draw costs and change
   their mean. Verify actual costs before coalescing, preserving operation,
   receiver, arguments and typed draw identity. Test checked declared and
   compound units, exact equivalent-bound conversion and wrong UnitId.
2. Give s WAIT self with R=0 and GO to B with R=1. Offer minimum 1/rank 1,
   then minimum 0 and 1/2 with otherwise valid rows. Offer maximum 1 with
   otherwise valid rows, and inspect the native maximum infinity outcome.
   Separately use a proper zero-cost retry with positive B probability at
   threshold 0; mutate its positive rank to zero. Use non-B terminal and
   zero-cost closed-class controls for both extrema. Explicitly verify
   empty actions cannot vacuously satisfy maximum progress.
3. Give initial s a safe R=1 exit to B and another action to a non-B
   self-trap t with R=0. Inspect native minimum values (1, infinity, 0)
   for (s,t,B); require unsupported mixed-infinite coverage with no finite
   document. Offer forged complete finite rows with t value zero, then
   omit t's rows. Keep native unsupported distinct from checker rejection.
4. Use initial B with an original positive-cost outgoing operation; then
   use A-to-B with entry cost and an admitted positive-cost cycle reachable
   only after B. Offer only the first-hit rows and stopped target set.
   Mutate after-B rows and targets separately. Round-trip valid documents
   through a fresh checker process without analyzer/provider linkage.
   Apply every AC-45 objective/identity, binding, rational, JSON, full-key,
   ordering, value and rank mutation, asserting precise rule and locus.
   Retain old acyclic and DTMC canonical controls and producer preferences.
5. Observe original/stopped admission, actual current/candidate/post/raw/
   full-key, all-action/draw, B/reward/unit, exact sums and existential-tight
   or universal progress phases. Cancel using the actual caller token for
   Requested and Deadline with matched successful controls, not fixed poll
   offsets. Exhaust actual byte/state/edge/work/storage/representation and
   arithmetic requirements. Check earlier model/negative refusal precedence
   and deferred undefined/unsupported evaluation without zero substitution.
   Exercise unsupported shapes and zero direct over instances.

Executable assertions SHALL carry actual criterion attributes such as
`#[trace("TC-1405", "FR-201-AC-44")]`. This procedure writes no matrix and
does not qualify external transition/interpreter allocations or all AC-30.

## Expected Results

- Step 1 accepts minimum 2 and maximum 4 at inclusive equality; interior
  misses reject ThresholdMissed at the initial full key. Distinct actions
  remain distinct despite identical successor distributions. Changed draw
  costs require changed Bellman values; wrong units refuse objective mismatch.
- Step 2 accepts genuine minimum 1 and proper zero retry with positive
  rank. False minimum fixed points and false finite maximum reject
  RankBroken; a nontight GO cannot justify minimum progress. Native maximum,
  terminal and improper closed classes retain infinity without finite
  documents. Zero value does not waive rank or properness checks.
- Step 3 emits no native finite document despite finite initial minimum.
  Forged trap rows reject RankBroken; missing rows reject Malformed at
  values/ranking. No infinity becomes zero or an internal fault.
- Step 4 charges entry B once, charges nothing at initial B or after B,
  and preserves exact stopped coverage and strict canonical transport.
  Extra rows reject coverage and extra targets refuse objective mismatch.
  Existing bytes/verdicts and sealed acceptance are unchanged.
- Step 5 retains actual typed refusal, unsupported, limit or cancellation
  causes with no accepted token/truth. Zero instances retain
  UnsupportedBinding/unsupported without invented binding or evidence.
  No new bound direction, infinity document, refutation or FR-413 transport
  is implied.
