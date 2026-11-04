---
id: SR-20261104091644
title: 'Gap analysis of PR #2286: PlanId covers atom, guard and helper bodies, gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@e92373842; git diff origin/main...e92373842 (PR #2286): FR-251-AC-37, FR-251-AC-38, FR-251-AC-39, QSpec FR-487-AC-13 against qsl-analyze/tests/it/monitor_plan.rs and qsl-analyze/tests/it/monitor_root.rs; ticket QSL-1630'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1630. Plan completion: not assessed. Planless gap analysis scoped to the ACs this PR traces. The tests were read against the AC text, and the focused `monitor_plan::` and `monitor_root::` integration tests were run on the PR head.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-251-AC-37 | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:499 | examined |
| FR-251-AC-38 | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:500 | examined |
| FR-251-AC-39 | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:501 | examined |
| a_swapped_atom_or_guard_body_changes_the_plan_id (TC-1081) | qsl-analyze/tests/it/monitor_plan.rs:725-796 | examined |
| the_root_key_covers_the_bodies_of_called_helpers (TC-1081) | qsl-analyze/tests/it/monitor_root.rs:477-530 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-251-AC-38's test covers only helpers reached by plain local calls (`twice` -> `bump`, `isGate`). No test swaps the body of a dispatched candidate or an imported callee, and the code does not cover them (code-review SR-20261104091643 FND-001/FND-002), so the AC's "declared helper function that an entry calls" is verified only for the `NodeKind::Call` subset. Fix: add a TC-1081 case with an atom calling a dispatched operation whose candidate body differs, alongside the code fix. | qsl-analyze/tests/it/monitor_root.rs:477-530 |

## Verdict

CONDITIONAL. AC-37 and AC-39 are fully backed. The plan is rebuilt through the public `TimedMonitorPlan::new` with the subject unchanged, and swapped atom and guard bodies each change `PlanId` while equal bodies keep it equal, with fixture-difference preconditions asserted. AC-38 is backed for direct and transitive local helpers of atoms and the guard, with atom and guard bodies asserted equal, but not for dispatch candidates or imported callees.
