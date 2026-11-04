---
id: SR-2225002
title: "Gap analysis of PR #2225: compensation body binders lowered by checked slot names"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@fc9952562; git diff origin/main...fc9952562 (PR #2225): FR-383-AC-8, FR-517-AC-14; ticket QSL-1555"
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1555. Planless, diff-scoped gap analysis. Plan completion: not assessed.
Criteria touched by the diff: FR-383-AC-8 (new test `a_sum_binder_in_a_recovery_predicate_is_a_parameter_named_by_its_source`,
tagged FR-383-AC-8 / TC-1061) and FR-517-AC-14 (test `the_recover_conditions_compare_the_receipt_sum_with_the_target`,
still `#[ignore]`, reason updated).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-383-AC-8 | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md:537 | examined |
| FR-517-AC-14 | tests/it/checked_handoff.rs:1703-1707 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-517-AC-14 stays unverified: its only test is ignored, now gated on a Contract IR reader gap (`BinderScope` refusal of a binder bound inside a protocol slot). The PR body says this "needs a ticket in quire-contract-ir" but the ignore reason names no ticket, so the gate has no owner. Fix: file the reader ticket and cite its id in the `#[ignore]` reason and the example comment. | tests/it/checked_handoff.rs:1706; examples/checked-handoff/unit-event-position.native:26-30 |

## Verdict

Conditional. FR-383-AC-8's new behaviour is backed by a real, tagged test that fails before the fix
(`EmptyBindingName`) per the PR and passes after. No stubs or coverage inflation in the diff. The open gap is
ownership of the FR-517-AC-14 reader blocker.
