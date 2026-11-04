---
id: SR-20261104091643
title: 'Code review of PR #2286: PlanId covers atom, guard and helper bodies, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@e92373842; git diff origin/main...e92373842 (PR #2286, branch peter/qsl-1630-planid-bodies): qsl-replay/src/monitor_plan.rs, qsl-analyze/src/monitor_plan.rs, qsl-analyze/tests/it/monitor_plan.rs, qsl-analyze/tests/it/monitor_root.rs; ticket QSL-1630'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1630. PR: quire-spec-language#2286, head e92373842. Method: code-review with the rust-review lane. `PlanId` now holds every atom's checked body, the guard body and a `BTreeMap<usize, CheckedBody>` of helper bodies collected by a new `call_closure` over `NodeKind::Call`. `MonitorProgram::with_helpers`/`helpers()` and `MonitorPlanRefusal::MissingHelperBody` are new. `Hash` is dropped from `PlanId` and `Eq` is hand-implemented.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| PlanId fields and Eq impl | qsl-replay/src/monitor_plan.rs:323-354 | examined |
| TimedMonitorPlan::id | qsl-replay/src/monitor_plan.rs:446-475 | examined |
| MonitorProgram::with_helpers / helpers | qsl-replay/src/monitor_plan.rs:222-234 | examined |
| MonitorPlanRefusal::MissingHelperBody | qsl-replay/src/monitor_plan.rs:557-562 | examined |
| produce helper wiring | qsl-analyze/src/monitor_plan.rs:181-195 | examined |
| call_closure | qsl-analyze/src/monitor_plan.rs:217-245 | examined |
| admit | qsl-analyze/src/monitor_plan.rs:265-285 | context_only |
| emit_plan_root re-derive check | qsl-analyze/src/monitor_root.rs:167-200 | context_only |
| FR-251 Build inputs and identity prose | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:175-186 | context_only |
| FR-251-AC-37, AC-38, AC-39 | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:499-501 | examined |
| quire-checked NodeKind::Dispatch / ImportedCall | quire-checked node.rs (git dep) | context_only |
| DispatchCandidate | qsl-semantics/src/check/ir.rs:776-800 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `call_closure` follows `NodeKind::Call` only. A `NodeKind::Dispatch` in an atom, guard or helper body names a dispatch table whose candidates are local package functions (`DispatchCandidate.body`, `precondition`, `precondition_clauses`), yet their bodies never enter `helpers`, so two plans that differ only in a dispatched candidate's body get an equal `PlanId`. FR-251's identity prose and AC-38 require the body of every declared helper an atom or the guard calls. Fix: on `NodeKind::Dispatch { table, .. }` push every candidate body and precondition function index from the package's dispatch table into the same closure, and add a TC-1081 case. | qsl-analyze/src/monitor_plan.rs:228-241 |
| FND-002 | medium | `NodeKind::ImportedCall` is skipped silently. `MonitorSubject` carries only the package's `LibraryName` and the clause key, and `callee` is a declaration key, so a dependency function body swap leaves `PlanId` unchanged. `emit_plan_root` imports no library, but `produce` and `admit` are public over any `CheckedPackage`. Fix: carry the admitted dependency callee's checked body in the closure (keyed by callee key), or, at minimum, refuse an imported call with a typed `MonitorPlanRefusal` so no plan is admitted with an uncovered body. | qsl-analyze/src/monitor_plan.rs:228-231 |
| FND-003 | low | The hand-written `impl Eq for PlanId` rests on "`CheckedBody` equality is total", but `CheckedLiteral::eq` delegates a `Value::Union` pair to `quire_exact::compare_keys`, which returns `None` for a `Float` or population payload, so a union literal with a float payload is not equal to itself. Reachability in a monitor body is unconfirmed. Fix: make the comment state the actual premise, or fix reflexivity upstream in quire-checked's `CheckedLiteral::eq` for union payloads. | qsl-replay/src/monitor_plan.rs:352-354 |

## Verdict

CONDITIONAL. AC-37, AC-38 (for ordinary calls) and AC-39 hold. The atom and guard bodies, and the bodies of plain local helpers, are now part of `PlanId`. A swap of any of these changes the id, equal bodies give an equal id, and admission's full-plan comparison now also covers the helpers. The `BTreeMap` keyed by function index gives deterministic identity, and no digest is minted. No exhaustive match on `MonitorPlanRefusal` exists in the workspace (grep: the enum is only constructed, converted with `#[from]` and tested with `matches!`), so the new variant breaks nothing. Dropping `Hash` is safe, since nothing hashes `PlanId`. The stated limit (dispatch and imported calls) is a real hole in the FR-251 identity rule, not just an acceptable scope note: FND-001 is reachable inside one package.
