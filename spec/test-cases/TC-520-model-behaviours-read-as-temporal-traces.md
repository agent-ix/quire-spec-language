---
id: TC-520
title: "A model subject's behaviours read as temporal traces, with terminal stutter and interval wrap"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: verifies
---
# TC-520: A model subject's behaviours read as temporal traces, with terminal stutter and interval wrap

## Description

Verify positions and anchors of a model behaviour, the closed reading under
a bounded profile and the terminal stutter under infinite-trace, interval
and past operators over a lasso, the `over` binding, and the empty-initial subject.

Scope: FR-125-AC-1 to FR-125-AC-5 and FR-125-AC-7 to FR-125-AC-10.

## Test Procedure

1. Build the `Counter` subject (FR-124-AC-1, no `terminal` declaration); read its
   behaviour's positions and anchors, under event-position false-extension
   and under infinite-trace.
2. Evaluate FR-125-AC-2's three formulas over that behaviour.
3. Evaluate FR-125-AC-3's two formulas over ADR-018 §6's lasso, reading the
   meter's charges.
4. Instantiate a clause with `over (c: Config::ConfigVersion)` over ADR-018
   §6's subject; build a subject with an empty `initial` list.
5. Evaluate FR-125-AC-5's formula over ADR-018 §6's lasso, reading the
   conjunction at positions 0, 3 and 9.
6. Check `ReachesTwo` and `ReachesThree` once, then instantiate the same
   checked claims over ADR-018 §6's subject (universe `{a, b}`) and over a
   subject whose universe is `{a}`; instantiate `possible Anything using v
   { true }` with no `over`; instantiate over a subtype-conforming
   population, a population with no universe and an empty universe
   (FR-125-AC-7).
7. Instantiate `over (x: Lamp)` over a population with member types `Lamp`
   and `Switch`, then `over (x: Thing)` with both subtypes of `Thing`, then a
   claim over a type no member type relates to (FR-125-AC-8).
8. Read the bound parameter at positions where the key's object is a `Lamp`
   (the canonical member type), a `Switch` (conforming to `Thing`, not to
   `Lamp`), and where the key has no object (FR-125-AC-9).
9. Instantiate a claim covering populations `P1` (universe `[d, c]`) and `P2`
   (universe `[b, a]`) and list `ModelLetters::bindings` (FR-125-AC-10).

Tag the tests `#[trace("TC-520", "FR-125-AC-n")]`.

## Expected Results

- Step 1: positions `0, 1, 2, 3` with anchors `initialization`, `inc`,
  `inc`, `inc`; under infinite-trace, then the stutter step repeating
  position 3, with a transition identity distinct from `inc`'s.
- Step 2: `false` under false-extension and `true` under infinite-trace for
  `always[0,5]`; `true`; `false`.
- Step 3: `true` and `false`, one work unit per (node, position) visit.
- Step 4: two instances, `a` and `b`; `NoInitialState` before exploration.
- Step 5: past reach 1, two loop copies (6 positions); the conjunction is
  `false` at 0, `true` at 3 and `true` at 9 by congruence; the formula is
  `true`.
- Step 6: two instances (`a`, `b`) for each of `ReachesTwo` and
  `ReachesThree`; one instance with no bound object for the claim with no
  `over`; the same checked claims give `a` alone over `{a}`, with no re-check; a
  subtype population contributes its objects; the universe-less population
  is `RequiresBound`; the empty universe gives zero instances and `NoInstance`, settled
  `inconclusive`.
- Step 7: one instance per key for `Lamp`, one (not two) per key for `Thing`,
  none for the unrelated type.
- Step 8: the `Lamp` object is read through the binding; for `x: Lamp` the
  `Switch` object and the missing object read `absent-key` undefined; for
  `x: Thing` the `Switch` object is read through the binding, its state
  reference carrying type `Switch` while the binding carries the canonical
  member type.
- Step 9: `P1/d`, `P1/c`, `P2/b`, `P2/a`.
