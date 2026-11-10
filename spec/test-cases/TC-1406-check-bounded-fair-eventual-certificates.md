---
id: TC-1406
title: "Check bounded fair eventual certificates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-200
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
---
# TC-1406: Check bounded fair eventual certificates

## Description

Specify FR-201-AC-47 to AC-50's closed bounded-fair exact proof checks.
These procedures are not execution evidence. Keep checked source identity,
fairness reconstruction and qualified monitor semantics independent of the
producer; equal fair/unfair values do not imply equal full product keys.

## Test Procedure

1. Use TC-635's bounded Deliver/DeliverFair fixture with minimum 24/25.
   Prove AtLeast 19/20 with weak whole operation fairness, then an otherwise
   fair one-step model whose two actions reach success with probabilities
   3/5 and 4/5 (and decided failure otherwise), proving maximum AtMost
   9/10 with exact value 4/5. Exercise weak/strong and
   whole/each separately, multiple receivers and inherited operation
   declarations under aliases. Exercise horizon zero with initially true
   and false predicates and interior source thresholds; verify source
   thresholds 0 and 1 remain refused, while document state values may be
   0 and 1. Retain an unfair canonical-byte control.
2. Reorder authored fairness and repeat a spelling of one resolved
   constraint; compare canonical objective sets. Retain both weak and
   strong constraints on the same operation. Mutate one record's strength,
   granularity, operation name and declaring model/package/path/ordinal;
   use a field or undeclared operation, inherited consumer identity instead
   of declaring identity, a same-name operation from another actual owner
   (including the checked override case), foreign target, missing/extra record and empty
   set. Duplicate/reorder document records, remove/add record members and
   substitute a non-operation shared target. Change actual each receiver
   state/binding rather than inventing a receiver member in the objective.
3. Round-trip valid canonical bytes through a checker without analyzer
   linkage. Alter source identity/binding/goal/horizon, monitor context or
   one complete-key component, then targets/optimization, value and rank.
   Substitute acyclic height for actual remaining horizon; omit/add/order/
   duplicate rows. Exercise terminal false extension, inclusive h and
   malformed UTF-8/JSON, unknown/duplicate members, noncanonical rationals
   and base64url. Apply earlier source unresolved-operation refusals.
4. Observe successful fairness intake, declaring-node/text traversal,
   canonical metadata encoding, current/candidate/post raw/full-key and
   graph/equality/ranking phases. Request cancellation using actual caller
   tokens for Requested and Deadline with matched successful controls;
   exhaust actual byte/state/edge/work/storage/representation/arithmetic
   requirements, including deep structural declaration and long text
   inputs. Do not simulate cancellation by fixed poll counts. Exercise
   unsupported temporal/workload/reward/timed/multi-initial/multi-binding/
   post-state/refutation forms, Bounds/PrecisionBudget and zero direct
   instances. Preserve undefined evaluation without substituting zero.

Executable assertions SHALL carry criterion attributes such as
`#[trace("TC-1406", "FR-201-AC-48")]`; this procedure writes no matrix.

## Expected Results

- Step 1 accepts only actual inclusive bound proofs, preserves the 24/25
  minimum and exact fairness result, and rejects threshold misses under
  ThresholdMissed at the initial full key. Empty fairness uses unchanged
  old bytes; no fairness is silently discarded.
- Step 2 yields the same ordered exact set for authored reorder/dedup,
  keeps distinct weak/strong records, rejects malformed/duplicate/unsorted
  records as Malformed at objective and refuses validly formed mismatched
  sets as objective mismatch before acceptance. Wrong receiver/binding
  cannot alias the admitted subject. Earlier source refusals stay earlier.
- Step 3 independently rebuilds targets and full monitor keys, rejects
  false equality/rank/coverage and malformed transport with existing
  precise rules/loci, and refuses conflicting identity before enumeration.
  No policy, fair-MEC claim or new key component is fabricated.
- Step 4 preserves each actual typed cause without accepted truth. Zero
  instances retain UnsupportedBinding/unsupported with no invented
  evidence. No all-expansion AC-30 or external transition/private-reader/
  ordinary interpreter allocation guarantee is implied.
