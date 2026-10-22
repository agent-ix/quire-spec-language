---
id: TC-519
title: "Terminal declarations check, and the request carries one deadlock-freedom item per subject"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: verifies
---
# TC-519: Terminal declarations check, and the request carries one deadlock-freedom item per subject

## Description

Verify the `terminal` declaration's forms and refusals, the classification of
terminal states as intended or deadlocked, and the request writer's
deadlock-freedom item with its per-model opt-out.

Scope: FR-124-AC-1 to FR-124-AC-3 and FR-124-AC-5 to FR-124-AC-7.

## Test Procedure

Use the `Counter` unit of FR-124-AC-1 with universe `{c}` and initial value
0, in three variants: no `terminal` declaration, `terminal when` a predicate true
exactly when every counter is at 3, and `terminal any`.

1. Check each variant and compare package identities. Check a unit with two
   `terminal` declarations, and one whose `terminal when` predicate reads
   `pre(self.value)`.
2. Write a request with two temporal items over the same subject, for each
   variant; read its items.
3. Classify each reachable state under each variant; repeat with `fair weak
   inc` on one item.
4. Check `terminal when { self.value = 3 }` in a one-object-type model with
   two counters, and classify its terminal states.
5. Check the quantified predicate of FR-124-AC-6 in a model with `Counter`
   and `Gate`, and classify its terminal states.
6. Check `terminal when { self.value = 3 }` in the two-type model and in a
   model with no object type, and `terminal when { true }` in the latter.

Tag the tests `#[trace("TC-519", "FR-124-AC-n")]`.

## Expected Results

- Step 1: `None`, `When`, `Any`, pairwise different package identities;
  `ambiguous_declaration`/`ambiguous-name` naming both members;
  FR-104's refusal for the `pre` read.
- Step 2: one `DeadlockFreedom` item (form `ReachableInvariant`,
  (`temporal-satisfaction`, `Unbounded`), identity distinct from both
  authored items) for `None` and for `When`; none for `Any`.
- Step 3: value 3 is terminal in every variant, deadlocked under `None`,
  intended under `When` and `Any`; values 0 to 2 are not terminal; fairness
  changes nothing.
- Step 4: `When`; a terminal state is intended exactly when both counters
  hold 3.
- Step 5: `When`; intended exactly when every counter holds 3 and every gate
  is open.
- Step 6: `missing_declaration`/`missing-name` at `self` for both
  `self` predicates; `When` for `{ true }`.
