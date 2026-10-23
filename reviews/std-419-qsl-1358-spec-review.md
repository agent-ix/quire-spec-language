---
id: SR-16334
title: "spec-review of FR-126-AC-6 refused-contract row over a test TransitionSystem (quire-spec-language PR 1358)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a890f537c; spec/functional/FR-126; spec/test-cases/TC-521"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: references
---

# Spec-review of FR-126-AC-6 refused-contract row over a test TransitionSystem (quire-spec-language PR 1358)

## Summary

Ticket: STD-419. One spec-review pass over `git diff origin/main...a890f537c`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-spec names, and mergeability against main.

Examined, clean:

- The fixture matches FR-120 ("one or more are `Undefined` or `Refused`" gives `ContractUndetermined`) and FR-107 (no linked clause evaluates to kernel `Refused`, so the system must supply it), with `wrong_snapshot`/`wrong-anchor` the catalog pair of FR-090.
- TC-521 step 6 and its expected result agree with the AC; the PR also removes the "(not built: Remaining work: QSL-898)" note that main still carries in the expected results. No deferral language remains in the PR's added lines.

Merge status: Conflicts with origin/main d0a756b2b in TC-521 step 6 only (content). No other open PR touches FR-126-AC-6 or TC-521 step 6.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The test `TransitionSystem` is described only by its expansion of value 1. The initial state, the expansion of value 0 and the claim's reach of value 1 (`inc` from 0, per FR-126-AC-3) are left implicit, so an implementer cannot tell whether value 0 expands normally. One clause naming the other expansions as the Counter subject's own would fix it. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:455 | missing-requirement |

## Verdict

**Mergeable after it merges main** (one content conflict in TC-521 step 6, where main dropped "not built" from the fixture line and the PR rewrites the same lines; take the PR's text). One low finding. Gate `QSPEC_DIR=<QSpec main> make check-spec-validation`: exit 0, 19 warnings (baseline), none on touched files. No new ids. STD-419's FR-128 half (no `AuthoredUndefined` refusal, positive and negative `CanInc` ACs, TC-539) is on main through #1342 (FR-128-AC-6 and AC-7, TC-539 steps 6 and 7), and `AuthoredUndefined` appears nowhere in main's FR-128, TC-539 or TC-521, so the PR's narrower scope leaves nothing missing.

## Dispositions

| FND | Round | Outcome | Reason |
| --- | --- | --- | --- |
| FND-001 | 1 | fixed | Fixed in head f16169a6; the fix agrees with the finding and the vectors were derived by hand. |

Round 1 verdict: Finding fixed, no new defect; mergeable (merge-tree clean; gate exit 0).
