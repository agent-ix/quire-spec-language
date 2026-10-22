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

Scope: FR-124-AC-1 to FR-124-AC-3 and FR-124-AC-5 to FR-124-AC-10.

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
4. Check `terminal Done using v on M when { self.value = 3 };` in a
   one-object-type model with universe `{c1, c2}`, and classify its terminal
   states; repeat with universe `{}`.
5. Check the quantified predicate of FR-124-AC-6 in the model with `Counter`
   (universe `{c1}`, `inc`) and `Gate` (universe `{g1}`, `release`), and
   classify its terminal states.
6. Check `terminal Done using v on M when { self.value = 3 };` in the
   two-type model and in a model whose populations cover no object type, and
   `terminal Done using v on M when { true };` in the latter.
7. In the one-type model, classify the terminal states under
   `terminal Done using v on M when { exists(c in M::counters : c.value = 3) };`
   with universe `{c1}` and with universe `{}`.
8. Check `terminal Done using v on M when { self.value = 3 };` in a model
   whose one object type `Counter` is covered by populations `counters` and
   `archive`, the same predicate written as a quantifier over `counters`, and
   the same predicate in a model that adds `Gate` in `gates`.
9. In the one-type model with no operations, classify the terminal state under
   `terminal Done using v on M when { 2 / (2 - self.value) <= 1 };` for
   `c1.value`, `c2.value` of 2 and 1, 2 and 3, and 0 and 3.

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
- Step 4: `When`; with universe `{c1, c2}` a terminal state is intended
  exactly when both counters hold 3; with universe `{}` the state is
  terminal and intended.
- Step 5: `When`; the terminal state with `c1.value = 3` and `g1.open` is
  intended.
- Step 6: `missing_declaration`/`missing-name` at `self` for both `self`
  predicates; `When` for `{ true }`.
- Step 7: `P` is evaluated once at each terminal state; with universe `{c1}`
  and `c1.value = 3` the state is intended; with universe `{}` it is false and
  the state is deadlocked.
- Step 8: `ambiguous_declaration`/`ambiguous-name` at `self` naming `archive`
  and `counters`; `When` for the quantifier; `missing_declaration`/
  `missing-name` at `self` in the model with `Gate`.
- Step 9: deadlocked; neither intended nor deadlocked, item `refuted` with
  `UndefinedEvaluation`; intended.
