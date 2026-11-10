---
id: TC-1404
title: "Check cyclic MDP eventual probability certificates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
---
# TC-1404: Check cyclic MDP eventual probability certificates

## Description

Specify FR-201-AC-39 to AC-42's finite untimed every-scheduler proof
controls. These are implementation obligations, not execution evidence.
The independent checker reconstructs all actions and distinguishes
minimum universal progress from maximum tight-action progress. Offered
policies, action ordinals and qualitative labels are not authorities.

## Test Procedure

1. Admit a checked finite model with initial s, target t and false terminal
   f. At s, action a has probabilities (s,t,f)=(1/2,1/5,3/10), and b has
   (1/2,2/5,1/10). Offer minimum values (s,t,f)=(2/5,1,0), maximum
   (4/5,1,0) and ranks (1,0,0). Check the admitted equality thresholds,
   minimum AtLeast 1/2 and maximum AtMost 3/4. Independently change a
   semantic action probability. Without changing the checked action set,
   offer 4/5 as the minimum after ignoring a, 2/5 as the maximum after
   ignoring b, and 3/5 after averaging both distributions. No action list
   is supplied by the document; the checker reconstructs all of them.
2. Give s the actions wait (probability-one self-loop) and go (to t).
   Verify its actual minimum is zero, and offer bogus v(s)=1/2 under
   AtLeast 1/10 with rank 1. It satisfies min Bellman equality but wait
   prevents universal progress. Then use wait and attempt=(4/5 t,1/5 f).
   Offer bogus maximum v(s)=9/10 with rank 1 under AtMost 19/20: wait is
   tight, but only the nontight attempt progresses. Separately offer the
   correct maximum 4/5. Mutate ranks so only zero-probability edges descend,
   and ensure a positive-probability edge to value-zero f does count.
3. Exercise a live minimum-zero state with an actual zero-support action
   alongside an action to t, and a maximum-zero closed class. Use false
   terminals without actions and real multi-state cyclic mixtures.
   In particular let S choose A=(1/2 fail,1/2 T) or
   B=(1/4 fail,3/4 T), with T=(2/3 target,1/3 S). Offer minimum
   (vS,vT)=(2/5,4/5) and maximum (2/3,8/9), with both live ranks 1 and
   target/fail ranks 0. At S only value-zero fail has lower rank; positive
   T has equal rank. Check the admitted initial equality thresholds 2/5
   and 2/3 rather than altering these valid ranks to satisfy the old
   positive-successor-value guard.
   For stopped coverage, let t originally reach a non-target cycle after
   first P. Admit that original subject but retain rows/targets only from
   the initial stopped graph. Add an after-target value/rank/target row
   independently. Include initial P, and all-zero eventual values with
   an admitted interior maximum threshold.
4. Encode valid documents and check them in a fresh process without
   qsl-analyze/provider linkage. Independently mutate the AC-41 objective,
   identity, binding, rational/JSON/key, member, ordering, value,
   zero-support and ranking controls. Assert the precise rule/locus, not
   just nonacceptance. Keep old acyclic and cyclic DTMC bytes/verdicts and
   their default producer preference. Malicious maximum 9/10 remains
   invalid exact evidence even though its chosen 19/20 upper property is
   also true of the correct 4/5 value.
5. Observe actual original/stopped traversal, raw/current/candidate/post/
   full keys, action/draw reconstruction, exact extrema, zero-support and
   rank phases. Cancel the actual token with Requested and Deadline,
   using matched uncancelled controls rather than fixed poll offsets.
   Exercise cumulative work/storage, byte/state/edge, representation and
   exact arithmetic stops. Include undefined state predicates, source
   refusals, unsupported shapes/kinds/fairness/workload/reward/timing/
   pending choices and zero direct over instances. Confirm source
   probability thresholds 0 and 1 stay refused and no minimum AtMost or
   maximum AtLeast source form is introduced.

Executable assertions SHALL carry actual criterion attributes, for
example `#[trace("TC-1404", "FR-201-AC-40")]`. This procedure does not
author a matrix or claim whole-runtime allocation qualification.

## Expected Results

- Step 1 accepts exact minimum 2/5 and maximum 4/5 at equality, and rejects
  the interior misses at the initial full key as ThresholdMissed. Every
  independently rederived action participates; a good policy or averaged
  distribution cannot replace the extremum.
- Step 2 rejects bogus minimum 1/2 and maximum 9/10 as RankBroken at s,
  and accepts the correct maximum 4/5. Ties to wait do not establish
  attainment. Zero-valued successors are valid boundary progress;
  zero-probability edges are not.
- Step 3 preserves exact minimum-zero closed-action and maximum-zero
  all-action support, terminal value zero and first-target stopping.
  The S/T vectors accept with both live ranks 1, proving that progress
  solely to a zero-valued boundary is sufficient.
  Extra stopped-graph rows reject Malformed at values/ranking; an extra
  objective target refuses objective mismatch. A post-target cycle alone
  does not invalidate the new profile.
- Step 4 preserves strict canonical bytes, full binding-bearing keys and
  the specified InequalityFails/RankBroken/member diagnostics. Existing
  profiles remain unchanged. No invalid exact vector is accepted solely
  because its threshold property happens to hold.
- Step 5 preserves the actual typed refusal, unsupported, limit or
  cancellation cause with no accepted truth. No evaluation failure becomes
  zero or a dropped action. Zero direct instances retain
  UnsupportedBinding/unsupported with no fabricated binding, terminal
  verdict, certificate or counterexample. No proof becomes an FR-202 or
  FR-413 refutation.
