---
id: SR-2026111305495002
title: "base review of QSL-1901 postcondition range facts (FR-577)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-577-discharge-a-postcondition-s-range-obligations-from-its-operation-s-preconditions.md; spec/functional/FR-104-check-state-clauses.md; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; spec/test-cases/TC-1416-a-postcondition-s-range-is-proved-from-its-operation-s-precondition.md; PR quire-spec-language#2696"
review_set: subset
---

## Summary

Ticket: QSL-1901. Base checklist over `git diff origin/main...HEAD`: the new
FR-577 (six ACs), the FR-104 amendments (fact sources, plus the definedness-fact
bullet), the FR-121 amendment (the `region` field and AC-37), and the new
TC-1416. The IDs are well formed and unique. FR-577 sits above the highest
existing FR id, and TC-1416 is new. The relationship targets resolve. TC-1416
verifies FR-577 and FR-121 and covers every FR-577 AC and FR-121-AC-37. Clause
order and reorder identities agree with FR-121 ("in declaration order") and
FR-104-AC-6. The FR-577 dependency paragraph describes QSpec FR-146
accurately: FR-146 has no relational fact, "No other reasoning is admitted",
and a producer "admits nothing that these rules do not prove". The FR-146
amendment is recorded upstream and is not counted as a finding. No hash,
digest, pin or tracking-record antipattern was found.

The ACs prove the main path and the refusals the author chose to show. Most of
the boundary SHALL statements, and all of the join and false-outcome rules,
have no AC.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Many FR-577 SHALL statements have no AC or test. Uncovered: the join rule (a bound kept only when it is on every alternative, with the larger `c`); the false-outcome rule (for example `not (amount > self.balance)` or the `else` of `if`); keeping the smaller `c` for a repeated pair; non-transfer of a `let`/quantifier/comprehension binder fact; an unreachable precondition; non-transfer across a subtype or supertype context or into an invariant or another precondition; ignoring a kept `requires`; difference facts in function bodies; and not widening FR-151 own_postcondition guard facts. Each could regress silently, and several decide soundness (join, binder). | FR-577:74-92, FR-577:104-136, FR-577-AC-1..AC-6 | correct-requirement-no-evidence |
| FND-002 | low | The rendered form of an interval is unspecified. FR-577 says the message "SHALL name" the required and proved intervals, and AC-2 and TC-1416 assert the substrings `[0, 100]` and `[-100, 100]`. No requirement fixes that spelling, or the spelling of an unbounded end (an `Integer` operand). | FR-577:142-147, FR-577-AC-2, TC-1416:59-62 | wrong-requirement |
| FND-003 | low | The Description describes the current state rather than the target design. "Today those facts are only the clause's own guards..." describes the code before this change. The spec should state the rule as designed. | FR-577:28-34 | wrong-requirement |
| FND-004 | low | FR-577-AC-1 bundles four checks: admission, the `Int[0, 100]` typing, `call_site` clause order, and reorder identity. AC-4 and AC-5 each bundle two or three independent fixtures, so one failing row hides which behaviour broke. | FR-577-AC-1, FR-577-AC-4, FR-577-AC-5 | wrong-requirement |

## New findings (disposition pass 1)

Checked at 7b0adbe07.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-577-AC-29's fixture has two refusals. With `CanAfford`'s body replaced by `self.limit <= self.balance`, `Debited` loses its bound and also refuses at `pre(self.balance) - amount`. `Debited` is declared before the added `Room`, so the first refusal (the one `call_site` and the region report) is most likely `Debited`'s, not the asserted `self.balance - self.limit`. The AC therefore does not isolate "no transfer into an invariant". Keep `amount <= self.balance` in `CanAfford`, for example `amount <= self.balance and self.limit <= self.balance`. | FR-577-AC-29 (FR-577:231), TC-1416:89-90 |
| FND-006 | low | TC-1416's procedure runs every compile through `call_site` with the `Bank::Account::withdraw` selection "unless the step names another entry". Steps 3 and 12 compile a unit that declares only a function. Step 12 expects "compiles", but a `withdraw` selection on a unit with no model or operation refuses `UnknownOperation`. Steps 3 and 12 should name a function selection (`g`, `f`) or a plain compile. | TC-1416:31-33, TC-1416:57, TC-1416:91 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 7b0adbe07 adds AC-6..AC-31: the join (AC-11, AC-12), negation (AC-8), smaller `c` (AC-9, AC-10), binder (AC-20, AC-21), unreachable (AC-22, AC-23), context (AC-24, AC-25), family (AC-26, AC-27), non-postcondition targets (AC-28..AC-30) and a function body (AC-31). The `own_postcondition_clauses` SHALL became the Description scope sentence at FR-577:64-65. That is acceptable: it now states scope, not a behaviour needing a test. AC-29 has a fixture defect; see FND-005. |
| FND-002 | fixed | FR-577:183-185 now fixes the spelling `[lo, hi]`, with base-10 ends, `, ` as separator, and `-inf`/`+inf`. AC-5 covers the unbounded case. |
| FND-003 | fixed | The "Today ..." sentence is gone. The Description now states the target rule and the bank example as a design statement. |
| FND-004 | fixed | AC-1..AC-4 are now single checks (typing, reorder identity, refusal intervals, `call_site` message), and AC-4/AC-5's old fixtures are split into AC-14..AC-18. |
| FND-005 | fixed | c5b1581c6: AC-29's `CanAfford` is `amount <= self.balance and self.limit <= self.balance`, so `Debited` still checks and the only refusal is `Room`'s at `self.balance - self.limit`. |
| FND-006 | fixed | c5b1581c6: TC-1416:29-34 and steps 3 and 12 compile the function-only units through `call_site` with the function selections `g` and `f`. |
