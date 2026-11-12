---
id: SR-2026111216592600000002
title: "gap-analysis of QSL-1891 and QSL-1885 (quire-spec-language PR 2691)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-218-AC-40 (TC-663), FR-206-AC-3 and FR-206-AC-11 (TC-651), FR-216-AC-8 (TC-661), FR-206 predicate-join statement; PR 2691 diff, head df4efab42"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-663
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-651
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-661
    type: references
---

# Gap analysis of QSL-1891 and QSL-1885 (quire-spec-language PR 2691)

## Summary

Tickets: QSL-1891, QSL-1885. Planless. Plan completion: not assessed. The criteria were read
from the branch, and FR-218-AC-40 and TC-663 match main.

- FR-218-AC-40: `a_non_all_join_with_no_outstanding_rule_refuses_at_the_join` covers `any [a]`,
  `quorum(1) [a, b]` and `predicate(f) [a]`. It checks the refusal code, the cause
  (`JoinList`/`NoOutstandingRule`, parallel `P`) and the locus at the join. It checks each join
  with `cancel` and with `continue`, and `all [a, b]` with no rule.
  `a_weak_join_with_no_outstanding_rule_reports_both_refusals` asserts both refusals, sorted.
  Both tests fail without the new check. Tagged `TC-663`/`FR-218-AC-40`, honestly.
- FR-206-AC-3, predicate part, and FR-206-AC-11:
  `a_predicate_join_is_enabled_where_its_function_holds` checks enabledness at each listed
  disposition vector and cancellation to the root, with both `[c, a]` and `[a, c]`. It fails
  without the code, because Program refused `JoinPredicate`. A join-list-order binding would
  fail the `[c, a]` vectors. The added `FR-206-AC-3` tag on
  `a_join_policy_decides_when_its_join_is_enabled` is honest: that test covers the
  `any`/`quorum` parts.
- FR-216-AC-8: `a_predicate_join_reads_only_its_instance_and_owner` asserts exact read and
  enabling sets {`inst(Both)`, root `ctl`} and dependence with `attempt` of `a`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test pins the ascending-label argument order that FR-206 and FR-218 "Predicate joins" state. Every predicate-join fixture declares its branches in alphabetical order (a, b, c), so declaration order and ascending byte order agree, and the wrong order the code uses (SR-2026111216592600000001 FND-001) passes. Fix: add a TC-651 vector whose branches are declared out of byte order, for example `branch c ... branch a ...` with `f(x, y) = x and not y` over `[a, c]`, and assert that `f` receives `a` first. | qsl-analyze/tests/it/protocol_system.rs:848-906 |
| FND-002 | medium | FR-206's predicate-join clauses for an undefined result (`UndefinedEvaluation` violation at the join, never `false`) and for a resource-limited evaluation (stop with the limit's refusal) have no test and no implementation. The PR body defers this as "remaining work" under QSL-1885, which is the ticket this PR closes. Fix in this PR: add tests for both outcomes alongside the fix for SR-2026111216592600000001 FND-002. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:100-108; qsl-eval/src/simulation/protocol/steps.rs:1146-1157 |
| FND-003 | low | FR-216-AC-8 says the predicate join's write footprint under `outstanding cancel` is "the one FR-216-AC-3 gives `join any`". The test only asserts that `writes` contains each branch's `ctl`, so an extra write would pass. Fix: compare `join.writes` with the footprint of the same protocol joined `any [c, a] outstanding cancel`, or with the exact expected set. | qsl-analyze/tests/it/protocol_footprints.rs:433-436 |

## Verdict

FR-218-AC-40 is fully implemented and backed by tests that fail without the code. FR-206-AC-3,
FR-206-AC-11 and FR-216-AC-8 have tests that fail without the code. However, FR-206's
ordering clause is untested and violated (FND-001). The undefined and resource-limit clauses
are untested and unimplemented (FND-002). Fix both medium gaps in this PR before QSL-1885
closes.
