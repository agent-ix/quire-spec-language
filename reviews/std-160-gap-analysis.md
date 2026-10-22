---
id: SR-13782
title: 'Gap analysis of PR #1183: type of self in terminal when P (STD-160), new ACs against test-case rows'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@788935b87; git diff origin/main...788935b87 (PR #1183, branch std-160-spec): FR-124-AC-5 to FR-124-AC-7 against TC-519 and the computed test matrix; qsl-semantics/tests/it/terminal_declarations.rs; ticket STD-160'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-124
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-519
  type: reviews
---

## Summary

Ticket: STD-160. PR: quire-spec-language#1183, head 788935b87. Method: gap-analysis, limited to the new and changed ACs. Plan completion: not assessed. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

AC-to-TC rows: TC-519's scope now names FR-124-AC-5 to AC-7, and steps 4, 5 and 6 with their expected results map one to one to AC-5, AC-6 and AC-7. `quoin matrix --json` at head reports AC-5, AC-6 and AC-7 as `untagged`. This is expected for a spec-only PR, and the PR body lists the tests as remaining work. FR-124-AC-1 is also `untagged`, but that predates this PR and is out of scope.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-124-AC-5 | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:149 | examined |
| FR-124-AC-6 | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:150 | examined |
| FR-124-AC-7 | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:151 | examined |
| FR-124 behaviour bullet on `self` typing | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:90-101 | examined |
| TC-519 steps 4-6 and expected results | spec/test-cases/TC-519-terminal-declarations-and-the-deadlock-freedom-item.md:32-37,52-57 | examined |
| `a_when_predicate_needs_exactly_one_state_object_type` | qsl-semantics/tests/it/terminal_declarations.rs:190-219 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The bullet's clause "a state with no object of `T` satisfies `P`" has no AC and no TC-519 step. AC-5 fixes the universe at `{c1, c2}`, so no tested state is empty of `T`. Add an AC row, for example the AC-5 model with universe `{}` or a reachable state with no counter, classified `Intended` under `self.value = 3`, and a TC-519 step for it. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:95-96 |
| FND-002 | medium | The existing test `a_when_predicate_needs_exactly_one_state_object_type` (untagged) asserts the behaviour AC-7 replaces. It refuses `terminal when` for a model with no population as `missing-name` at the `on` alias and for several types as `ambiguous-name` at the `on` alias, whatever `P` reads. AC-7 requires the refusal at `self`, `missing-name` in both cases, and `{ true }` to check. The implementation ticket has to replace this test, not add next to it, or the suite will hold two contradicting expectations. | qsl-semantics/tests/it/terminal_declarations.rs:190-219 |

## Verdict

Every new AC has a TC-519 row, and the steps match the ACs one to one. One behaviour clause (the empty-population vacuity) has no AC, and the code side will have to retire one existing test that asserts the old refusal. Tests for AC-5 to AC-7 are pending by design and are not a finding here.

## New findings (disposition pass 1)

Disposition pass 1 at head ce7fddd31. New ACs FR-124-AC-8 and FR-391-AC-9 map to TC-519 step 7 (with step 4's `{}` repeat) and TC-1076 row 11; TC-519 and TC-1076 scopes name them. Tests for FR-124-AC-5 to AC-8 and FR-391-AC-9 remain pending by design (spec-only PR).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The per-object combination rule ("not satisfied when it is false for any object, and otherwise, when it is `Undefined` for some object, as `Undefined`") has no AC or TC-519 step. No row has one object false and another `Undefined`, so an implementation that returns `Undefined` first would pass every AC. Add a row, for example two counters where one `value` read is undefined and the other is false. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:107-109 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: FR-124-AC-8 "In the AC-5 model with universe `{}`, the single state is terminal and intended under `terminal Done using v on M when { self.value = 3 };`"; TC-519 step 4 repeats with universe `{}`, expected "with universe `{}` the state is terminal and intended". |
| FND-002 | deferred | Round 1: implementation work for the STD-160 code lane, outside this spec-only PR. The PR body's remaining work records it: "Replace (do not add beside) `a_when_predicate_needs_exactly_one_state_object_type` in `qsl-semantics/tests/it/terminal_declarations.rs`, which asserts the old refusals at the `on` alias". No separate follow-up ticket id is named. |
