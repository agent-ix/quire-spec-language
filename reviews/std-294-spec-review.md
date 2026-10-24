---
id: SR-16000
title: "Spec review of quire-spec-language PR #1258: STD-294, the choice partition proof charges s3.work_units"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e618309a2b98c8388c4fb155bb1569774ea19495; PR #1258 (git diff origin/main...e618309a2): spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/test-cases/TC-1172-choice-partition-proof-charges-the-s3-work-budget.md; read for context: FR-255, FR-258-AC-4, FR-096 (stage-limit locus table), FR-062, ADR-030, TC-133, agent-ix/quire-specification main 5ee0071fc FR-053"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1172
    type: reviews
---
# SR-16000: Spec review of quire-spec-language PR #1258: STD-294

## Summary

Ticket: STD-294. Sub-analyses folded in: integrity, EARS, failure-domain. The head e618309a2 is behind main 423ac6aa0 but merges cleanly. `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` on the merge result exits 0 with no diagnostic on FR-218 or TC-1172. FR-218-AC-30 and TC-1172 appear on no other open PR head and not on main. #1258 and #1261 both edit FR-218 and merge with each other cleanly. No status wording and no compat layer.

The rule itself is sound. The partition proof enumerates every valuation of the choice's `n` distinct atoms, because `failed` reports both `missing-arm` and `duplicate-arm`, so `2^n` charges is the right count. The ticket's "independent-atom proof" is the same proof: FR-218's repeat-progress rule is structural and evaluates no valuations. The 24-atom vector is right: the default `s3.work_units` is 16777216 = 2^24 (FR-255), so the valuations alone use the whole budget and any earlier charge makes it stop, and 33554432 = 2^25 leaves 2^24 units for everything else. The deviation from the ticket's `Incomplete(LimitReached)` is correct. S3's `CheckingLimits` ceilings are stage limits reported as `stage_limit_exceeded` (FR-096, FR-258-AC-4, FR-062), and `Incomplete` is an evaluation-stage settlement, so it is not an S3 outcome.

## Verdict

**NOT MERGEABLE as written.** One major finding: the AC's `w - 1` vector does not fix which charge is denied (FND-001). Two medium findings: the new limit producer has no locus row in FR-096, and the AC's 8-unit claim has no stated way to observe it. Two low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | [major] AC-30's budget vector does not fix which charge is denied. `w` is "the unit's measured work with that choice proved", which is the whole S3 run. At `w - 1` the denied charge is the run's last unit, and it is a valuation only if the partition proof is the last thing S3 charges. FR-258-AC-4 shows it is not: S3 charges the declaration's precheck work up front, and per-node lowering charges come later ("At `w` the declaration's own charge is admitted, and the stop is a later lowering charge"). So the stop at `w - 1` is most likely a lowering charge at a lowering node, not `work-budget-exceeded` at the choice's span. Fix: state when the proof charges relative to the precheck and lowering charges, and give the vector relative to the meter count `c` when the proof starts. At `c + 8` the proof completes; at `c + 7` the eighth valuation is denied at the choice, naming bound `c + 7` and count `c + 8`. Alternatively, rely on the 24-atom row for the stop and drop the `w - 1` row. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:454; spec/test-cases/TC-1172-choice-partition-proof-charges-the-s3-work-budget.md:22-23,30-31 |
| FND-002 | medium | [minor] The partition proof is a new `s3.work_units` producer whose locus is the choice's span, but FR-096's producer-to-locus table has only three S3 rows: the declaration precheck at the declaration span, `Typer`/lowering node count at a node, and lowering's per-node work at a node. FR-096 says "Each producer's locus is the position at which its charge failed", so the table needs a fourth row. It must also say whether the stop is a family-check `LimitExceeded` re-reported by `PackageDeclarations::check`, or a direct `CheckRefusal` as lowering's charge is. FR-096-AC-16 pins the lowering case, and the partition proof needs the same. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:353-364; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:198-203 |
| FND-003 | medium | [minor] "checks, charging 8 valuation units" names no observable. The meter reports a total, so a test cannot read the proof's own charge unless the AC says how to compute it. Name the comparison, for example the measured work of this unit minus that of the same unit whose guards are `a` and `not a` (2 valuations) is 6, with guard-term node charges equalised. Without it the 8-unit claim passes whatever the proof charges. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:454; spec/test-cases/TC-1172-choice-partition-proof-charges-the-s3-work-budget.md:29 |
| FND-004 | low | [minor] "A gap among three atoms" (AC-30) and "the three-atom choice with guards `a and b` and `not a`" (TC-1172 step 4) disagree with FR-218's atom rule. The atoms are the atoms of the guards, so those guards have two atoms, `a` and `b`, and 4 valuations. `c` is only listed in `visible`. Say two atoms, or add `c` to a guard. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:454; spec/test-cases/TC-1172-choice-partition-proof-charges-the-s3-work-budget.md:24 |
| FND-005 | low | [minor] "Collecting and comparing atoms SHALL charge no more than the node charges of the guard terms" sets an upper limit that no AC checks. It also leaves the pairwise syntactic comparison of atoms (up to quadratic in the guard size) as work that is charged to nothing. Either say the comparison is covered by the guard terms' node charges because it is linear in them, or charge it. Separately, TC-133 still says "Measure zero, exact and one-short proof budgets" for the old path. The PR body says TC-1172 takes over those rows, but TC-133 is left as it was. That is acceptable if QSL-840 deletes TC-133, but the planner should confirm it. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:356-357; spec/test-cases/TC-133-admit-bounded-choreography-control.md:22 |

## New findings (disposition pass 1)

Round 1 checked head 795467742, which contains main 6fb751a31 and merges cleanly with main 879c3bf73. `make check-spec-validation` on the merge exits 0. FR-096-AC-22 appears on no other open PR head. Hand checks:
- The guards `a and b and c` (3 distinct atoms, 8 valuations) and `a and a and a` (1 distinct atom, 2 valuations) have the same nodes and the same preimage length, so the measured spends differ by exactly 6.
- The 24-atom stop must fall inside the proof. The proof runs after the precheck and before lowering, and its 2^24 valuation charges alone reach the default budget.
- The guards `a and b and c` and `not a` have three atoms and leave three valuations uncovered with no overlap, which gives `[missing-arm]`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | [minor] FR-096's prose still lists only "`Typer`'s package-wide node count, and lowering's own work charge (the other S3 rows below)" as the `CheckingLimits` stops reported directly as a `CheckRefusal`. The new partition-proof row is a third such stop (FR-218: "a `CheckRefusal`/`stage_limit_exceeded`"), so add it to that list. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:173-177 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The `w - 1` row is dropped. The proof's place is stated: after the declaration precheck (FR-062-AC-12) and before lowering. The 24-atom row carries the stop at the choice's span. |
| FND-002 | fixed | FR-096 has a new producer/locus row for the partition proof, and FR-096-AC-22 makes it a direct `CheckRefusal` with `refusal_region` set to the choice's span. |
| FND-003 | fixed | The observable is now the spend difference of exactly 6 against `a and a and a`. Checked by hand. |
| FND-004 | fixed | The gap case now uses `a and b and c` and `not a`, which has three real atoms. |
| FND-005 | fixed | Atoms are compared once per guard term node, and the 6-unit difference tests that. TC-133's old proof-budget sentence is left for QSL-840 to delete (planner item). |

## Dispositions (round 2)

Round 2 checked head 211e9af6e, which contains main 4c3562eaa and merges cleanly with main and with #1261. `make check-spec-validation` on the merge exits 0. No new findings.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-006 | fixed | FR-096's direct-`CheckRefusal` list now names "a `choice`'s partition-proof charge (FR-218)". |
