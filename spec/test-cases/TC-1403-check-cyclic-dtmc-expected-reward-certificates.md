---
id: TC-1403
title: "Check finite cyclic DTMC first-hit expected-reward certificates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
---
# TC-1403: Check finite cyclic DTMC first-hit expected-reward certificates

## Description

Specify FR-201-AC-35 to AC-38's finite cyclic workload proof controls.
These are implementation obligations, not execution evidence. The checker
independently derives the first-B stopped graph, actual draw costs and
rank progress; producer values and qualitative classifications are untrusted.

## Test Procedure

1. Admit a finite untimed DTMC with initial non-B state s, self and
   B-entering draws each of probability 1/2, and checked named R=1 on
   both. Offer exact V(s)=2, V(B)=0 and ranks (s,B)=(1,0). Check both
   equality bound sides, AtMost 3/2 and AtLeast 5/2. Separately change only
   the entry-B reward to 3 and verify V(s)=4 from
   `V(s) = (1+V(s))/2 + (3+0)/2`. Swap the two equal-probability costs,
   then change their mean. Use actual declared costs 2 ms on self and
   6 ms on entry, yielding 8 ms and an equivalent 0.008 s bound. Also use
   a checked compound unit and a foreign same-dimension UnitId control.
2. Set both costs to zero while retaining the positive-probability exit.
   Offer zero values and ranks (1,0), including bounds at zero, then set
   s's rank to zero. Separately offer otherwise Bellman-valid all-zero
   values for a non-B closed self-loop and a two-state non-B closed cycle;
   no natural ranks can satisfy every required progress edge. Check the
   producer's actual FR-198 infinity outcome is not serialized as a
   finite value or accepted proof.
3. Admit an initial B with an original positive-cost operation and verify
   the stopped graph has only initial B, with V/rank zero. In another
   model let A reach B, then let an original B edge lead to a positive-cost
   non-B cycle D reachable only after B. Keep the original subject valid
   and finite, but offer first-hit value/rank rows only for A and B and
   targets only from the stopped graph. Mutate the document to add D's
   value/rank/target independently. This profile must not accidentally
   retain the old original-DAG or all-original-terminal-B gate.
4. Encode valid documents and check them in a fresh process without
   qsl-analyze/provider linkage. Apply each AC-37 identity, unit, member,
   ordering, value and rank mutation separately; assert the precise
   existing rule/locus, not only nonacceptance. Include missing top-level
   values/ranking, an extra after-B key, a binding-free model key,
   noncanonical rational data and early identity conflict. Retain old
   acyclic reward and unweighted/weighted long-run canonical controls.
5. Observe actual original/stopped graph, internal raw/current/candidate/
   post/full-key, draw/B/reward/unit, rank and exact-row phases. Cancel
   through the actual caller token with Requested and Deadline causes,
   using matched uncancelled controls rather than fixed poll offsets.
   Exercise cumulative work/storage, representation and arithmetic limits
   and byte/state/edge bounds at observed requirements. Preserve earlier
   FR-187 model refusal precedence and deferred typed nonacceptance;
   never replace an invalid reward by zero. Include unsupported shapes,
   fairness/every-scheduler, multi-post/infinity/refutation forms and
   zero direct over instances.

Executable assertions SHALL carry actual criterion trace attributes,
for example `#[trace("TC-1403", "FR-201-AC-36")]`. This procedure does not
author a matrix or claim whole-runtime allocation qualification.

## Expected Results

- Step 1 accepts 2 and 4 with actual reward units and inclusive bounds;
  the interior beyond-bound controls reject ThresholdMissed at the initial
  full key. The false 2 after entry cost changes rejects InequalityFails;
  no unit-step or post-B charging substitute verifies.
- Step 2 accepts proper zero reward, but wrong/no progress rejects
  RankBroken at the first failing full key even when V=0. A zero-cost
  closed non-target class keeps FR-198 infinity and produces no finite
  proof. No probability-value-positive exemption is used for progress.
- Step 3 counts entry-B cost exactly once, no cost after B, and excludes
  after-B-only nodes from the certificate. A post-B cycle alone does not
  invalidate the new profile; extra rows reject Malformed at values or
  ranking, and an extra target refuses objective mismatch.
- Step 4 preserves strict canonical bytes, checked objective identity,
  complete stopped coverage and precise inequality/rank/member outcomes.
  Earlier profiles are unchanged; the acyclic producer preference remains.
- Step 5 preserves the actual typed source refusal, unsupported, limit or
  cancellation cause without an accepted token or truth value. Zero direct
  instances preserve UnsupportedBinding/unsupported with no invented
  binding, terminal verdict, certificate or counterexample. No finite
  document becomes an infinity document or FR-202/FR-413 refutation.
