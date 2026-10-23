---
id: SR-16333
title: "spec-review of FR-168-AC-13 over the constraint clause and constraint-evidence replay (quire-spec-language PR 1353)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9d7974210; spec/functional/FR-168; FR-170; spec/test-cases/TC-612"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: references
---

# Spec-review of FR-168-AC-13 over the constraint clause and constraint-evidence replay (quire-spec-language PR 1353)

## Summary

Ticket: STD-388. One spec-review pass over `git diff origin/main...9d7974210`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-spec names, and mergeability against main.

Examined, clean:

- FR-168-AC-13 was derived by hand: `Half` is `2 div (2 - self.versionNumber) >= 1`. At `va = 0` it is `2 div 2 = 1`, true; at 1 it is `2 div 1 = 2`, true; at 2 the divisor is 0, so the constraint is undefined and `ReachesTwo` returns `Undefined` at `va = 2` with stem `(0) -upd(a)-> (1) -upd(a)-> (2)`. It is the same law and comparison as FR-168-AC-11's guard, and the ill-typed integer `/` of the old fixture is gone.
- FR-168-AC-12's `mod` rewrite is already on main (STD-381); that hunk is a no-op after the merge.
- The FR-170 rule (read the constraint from `where.locus`, never from the request) matches #1352's FR-128 rule and QSpec FR-364-AC-8, whose locus list names the state constraint. No deferral or narrowing language was found.

Merge status: Conflicts with origin/main d0a756b2b only in FR-168 (adjacent lines: main carries AC-12 as the PR writes it; keep the PR's AC-13). No conflict with #1352 in the files touched. Depends on #1352 (FR-102 kind, FR-105 node, FR-158 checks) and QSpec #406 (clause `constraint`): merge after both.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new constraint-evidence replay has no failing path. FR-170 says "resolved in the recompiled package" but, unlike #1352's FR-128 rule (`stale_dependency`/`content-mismatch` when the package has no `constraint` clause at the locus), it states no outcome for a locus that names no constraint clause. AC-7's adverse vectors (another transition, last step removed) are written for the guard evidence only; none alters the constraint evidence (different cause, locus naming an invariant, last step removed). A replay that always returns `reproduced-with-evaluated-witness` would pass AC-7. Fix: state the refusal and add the adverse vectors to AC-7 and TC-612 step 7. | spec/functional/FR-170-replay-state-graph-evidence.md:199 | missing-requirement |

## Verdict

**Not mergeable yet**: one trivial conflict with main (FR-168-AC-13, which sits beside the AC-12 line main already carries), a hard dependency on #1352 and QSpec #406 (merge after both), and one medium finding. Gate `QSPEC_DIR=<QSpec #406 head> make check-spec-validation`: exit 0, 19 warnings (baseline), none on touched files. No new ids.

## Dispositions

| FND | Round | Outcome | Reason |
| --- | --- | --- | --- |
| FND-001 | 1 | fixed | Fixed in head 52d6cb9c; the fix agrees with the finding and the vectors were derived by hand. |

Round 1 verdict: Finding fixed, no new defect; mergeable after #1352 and QSpec #406 (merge-tree clean; gate exit 0).
