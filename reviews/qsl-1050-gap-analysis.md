---
id: SR-17014
title: "Gap analysis of quire-spec-language PR #1450: EN-5 exact-build stops name exact_prob.* settings"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@41c394c47c44b430fc13a96ab7de364e25582770; PR #1450 (QSL-1050) diff origin/main...HEAD; criteria FR-255-AC-3, FR-255-AC-12, FR-203-AC-6, FR-196 Behavior (TC-631)"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1450

## Summary

Ticket: QSL-1050 (engine half). This is a planless gap analysis. The
matrix came from `quire matrix --scope . --format json` at 41c394c47. The
PR adds no trace tags and removes none, so no criterion changes status.
These criteria are still `untagged`: FR-255-AC-8, AC-12, AC-13 and AC-14;
FR-203-AC-1 to AC-7; and FR-196-AC-6 to AC-9, AC-11 and AC-12. All of them
predate this PR. FR-255-AC-3 is `tagged` by earlier tests.

The changed behaviour is that an EN-5 product-build stop names the
`exact_prob.*` setting. It belongs to FR-255 "Engine limit setting names"
and to the FR-255-AC-12 clause "Every `LimitReached` an engine makes at a
member of its limits types names that member's pair". It also belongs to
FR-203's stop row ("`Stopped(Limit{limit, value})` ... with its
`exact_prob.` setting name") and to FR-203-AC-6. The PR's tests assert the
stop and its setting name, but not a settled `LimitReached` record, because
no EN-5 settlement map exists. They also do not assert FR-255-AC-12's other
clauses: the name patterns, the union of the table, the settings operation
and the other engines. Leaving them untagged is therefore correct, and the
PR body says so accurately.

Reverse gap: each new variant is owned by an FR-255 engine-table row and by
FR-203. The `build.rs` stop sites are owned by FR-196 Behavior. No
production code without a spec owner was added.

Stubs and inflation: no stub was found. The new unit test and the updated
integration test assert concrete values. Reviewer mutants kill both (see
SR-17013).

## Verdict

**CONDITIONAL**: no high finding. One medium finding about the
limit-name half of the FR-255-AC-12 pair.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255-AC-12 maps each limit member to a pair (limit name, setting name) that must equal its engine-table row, which is (`max_states`, `exact_prob.max_states`) for the EN-5 product bounds. The PR makes the setting half match. The limit half is `max-states` (and `max-transitions`, `max-automaton-states`), which fails the AC's `^[a-z][a-z0-9_]*$` pattern, and the new unit test pins that spelling. When TC-1161 step 2 is implemented, or when an EN-5 settlement map lands, the pair check fails for these three variants. The same is already true of the EN-1 members. The code side is SR-17013 FND-001. | qsl-replay/src/model_check/limits.rs:128-134, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:193-195 |

## Coverage

- Criteria examined: FR-255-AC-12 (untagged, unchanged), FR-255-AC-3
  (tagged, unchanged), FR-203-AC-6 (untagged, unchanged). The FR-196 Behavior
  budget stop is TC-631, and `a_budget_or_a_cancellation_stops_the_build`
  carries a TC-only tag.
- Untagged criteria in the touched FRs predate this PR. The PR neither
  closes nor opens any of them.
- Semantic review (intent, test and code): skipped. This was a reviewer
  subagent run with no user opt-in.
- Plan completion: not assessed

## Dispositions

Round 1, checked at e3ab7aea0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The EN-5 (limit, setting) pairs now equal their FR-255 engine-table rows: (`max_states`, `exact_prob.max_states`), (`max_transitions`, `exact_prob.max_transitions`) and (`max_automaton_states`, `exact_prob.max_automaton_states`). `exact_prob_product_bounds_pair_a_snake_case_limit_with_an_exact_prob_setting` asserts them, and a kebab-revert mutant fails it. FR-255-AC-12 stays untagged, correctly, because its other clauses are unasserted. The EN-1 pairs are QSL-1059. |
