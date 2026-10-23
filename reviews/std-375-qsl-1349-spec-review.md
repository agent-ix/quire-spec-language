---
id: SR-16328
title: "spec-review of FR-454 schedulability claim node and FR-246 grammar, unit and defect order (quire-spec-language PR 1349)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@949d19ee; spec/functional/FR-454-emit-a-checked-schedulability-claim-as-a-v2-claim-node.md; spec/test-cases/TC-1202-a-schedulability-claim-emits-a-v2-claim-node.md; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md; spec/functional/FR-247-analyse-a-task-set-in-closed-form.md; spec/functional/FR-248-settle-a-closed-form-verdict-after-recomputing-its-evidence.md; spec/decisions/ADR-026-dense-time.md; spec/test-cases/TC-701-task-sets-and-schedulability-claims-check.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-454
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-248
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1202
    type: references
---

# Spec review of FR-454 schedulability claim node and FR-246 grammar, unit and defect order (quire-spec-language PR 1349)

## Summary

Ticket: STD-375. One spec-review pass over `git diff origin/main...949d19ee`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-repo names, mergeability and gates.

Gate: `QSPEC_DIR=<worktree of QSpec PR 404 head> make check-spec-validation`: exit 0, 19 warnings (none in the touched files).

Examined, clean:

- Names checked against QSpec PR 404 head fcbf602c: FR-482's `claim`/`schedulability`, the five body bindings, the ten task bindings, `fixed-priority`/`edf`/`amc` against source `fixed_priority`, `wcet_hi`, the reader refusals and loci used by FR-454-AC-4, and FR-419. The source grammar (`schedulable N on S under P;`, `response N on S.t <= d under fixed_priority;`) matches PR 404's shared grammar.
- Vectors by hand: `Ctl` fixpoints 1, 3 and 12 (third iterates 5, 9, 12, 12), EDF utilization 1/4 + 2/6 + 5/12 = 1; with the third WCET 6 the third fixpoint passes 12 (6, 10, 13) so both refute; in `us` the fixpoints are 1000, 3000, 12000 and `12 ms` is 12000; the four-refusal order in FR-246-AC-3 matches QSpec FR-419-AC-10.
- FR-337 and FR-410 already state that each lookup accepts only its own node form, which FR-454 relies on. No deferral or narrowing language. Ids: FR-454, FR-454-AC-1 to AC-4, TC-1202 and FR-246-AC-3 appear on no other open PR head or on main.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-454 requires FR-248's checker to read the claim "through the request's triple", to settle the item `Failed` without calling `check_closed_form` when the triple does not resolve, and to refuse a mismatch "naming both node ids". FR-248 is not amended for this: `check_closed_form(request, item: &ObligationIdentity, outcome, cancel)` has no triple parameter, FR-098's `ReplayRequest` is not shown to carry one, and FR-248 says the mismatch names both identities. The behaviour is stated only in FR-454, where the checker it constrains is defined elsewhere. Fix: amend FR-248 (signature or the request, and the `Failed` settlement), as FR-410 does through FR-169. | spec/functional/FR-454-emit-a-checked-schedulability-claim-as-a-v2-claim-node.md:97-105 | wrong-requirement |
| FND-002 | low | The second several-defect vector does not spell out the order it claims. With tasks `a`, `a` and a third with period 0 ms, source order puts the duplicate-name refusal at the second `a` before the period refusal at the third task, but AC-3 and TC-701 step 3 list "the period refusal and the duplicate-declaration in source order". A reader taking the list as the order expects the period first, and the AC does not say which span the duplicate refusal sits at. Fix: write the expected order explicitly (duplicate-declaration, then invalid-task-set) and name which task has which span. | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:AC-3 | wrong-requirement |
| FND-003 | low | The task set's unit is "the unit of the first time quantity in source order", and `TaskSet.unit` is not optional, but a task set with no task (refused by FR-246) has no time quantity, and "a task set refused for its own defects is still checked against each claim", including a `response` claim whose bound `d` must be converted into that unit. No unit exists to convert into. Fix: say the bound keeps its own unit when the set has none. | spec/functional/FR-246-check-task-sets-and-schedulability-claims.md:55-65 | wrong-requirement |

## Verdict

Mergeable after FND-001 (medium) are fixed or dispositioned by the lead.

Conflicts with main: none (clean merge-tree). Clean with open #1360, #1365 and #1374 (ADR-026). Depends on QSpec PR 404 for FR-482, FR-419 and the grammar; merge 404 first or together.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Fix 9ec98d90. FR-248 now takes `claim: &ClaimTriple`, resolves it through FR-454's lookup, returns `clause-root-unresolved` and checks no evidence, settles `Failed` (new map row), and refuses `stale_dependency`/`content-mismatch` naming both identities. New FR-248-AC-5 and TC-703 step 5 cover the own triple, three bad triples and `CtlEdf`'s triple. FR-454 and AC-3 are reworded to match (identities, not node ids). |
| FND-002 | fixed | AC-3 now fixes the fixture: tasks `a` (valid), `a` (valid), `z` (period 0 ms) report `duplicate-declaration` at the second `a`, then `invalid-task-set` at `z`. Hand-checked: the duplicate span precedes `z` in source order. |
| FND-003 | fixed | `TaskSet.unit` is `Option<UnitId>`; an empty task set has no unit and a `response` claim over it keeps its bound's own unit; AC-3 adds `taskset Empty { }` with `response R on Empty.c <= 12 ms ...`: `invalid-task-set` naming `Empty`, `missing-name` at `c`, bound kept as `12 ms`. |

Round 1 verdict at 9ec98d90: Mergeable, with or after QSpec #404 (006819eb). All three findings fixed, no new defect; FR-248-AC-5 is unique across open heads. Gate: exit 0, 19 warnings with QSPEC_DIR at the #404 head. Merge-tree with main is clean.
