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
and past operators over a lasso, the `over` binding, its admitted and
excluded keys, instance order and guard, and the order of the instancing
outcomes.

Scope: FR-125-AC-1 to FR-125-AC-5 and FR-125-AC-7 to FR-125-AC-16.

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
7. Build a population `P` with member types `Lamp` and `Switch`, both
   subtypes of `Thing`, whose universe keys `l` and `s` are admitted by
   initial records of a `Lamp` and a `Switch`, plus a key `u` no initial
   record admits; instantiate `over (x: Lamp)` and `over (x: Thing)`, read
   `ModelSystem::excluded_keys`; then instantiate `over (x: Lamp)` over a
   population with no member type comparable with `Lamp` (FR-125-AC-8).
8. Read the bound parameter of `over (x: Thing)` at positions where the key's
   object is a `Lamp`, a `Switch`, and absent; read that of `over (x: Lamp)`
   at the same positions; read the instance in the obligation identity and the
   FR-338 key context, and the refuted binding of `over (x: Thing)` at `s`
   (FR-125-AC-9).
9. Instantiate a claim over populations `P1` (universe `[d, c]`) and `P2`
   (universe `[b, a]`), every key admitted, and list
   `ModelSystem::instances`; repeat with `P2`'s universe `[c, a]`; repeat with
   `P1`'s universe `[d, u]` and `P2`'s `[u, a]`, no initial record admitting
   `u`, and read the excluded keys (FR-125-AC-10).
10. At a position where the key's object is a `Switch`, evaluate `x = r` and
    `contains(allInstances<Thing>(P), x)` for `over (x: Thing)`, then the same
    at a position holding a `Lamp`; with `c` listed by `P1` and `P2` of
    `Thing`, evaluate `contains(allInstances<Thing>(P2), x)` for the instance
    `c` (FR-125-AC-11).
11. Build `dogs` (member `Dog`, universe `[d]`) and `animals` (member
    `Animal` declaring `age: Int[0, 9]`, universe `[k, t]`), with an initial
    state holding a `Dog` under `k` and a `Cat` under `t` in `animals` and a
    `Dog` under `d` in `dogs`; instantiate `over (x: Dog)`; check the unguarded
    `always holds(x.age >= 0)`, the claim guarded by the disjunction `G` of
    FR-125-AC-12 and a property guarded by `animals` alone; then instantiate
    over a subject whose only population is `animals` holding only `Cat`
    objects (FR-125-AC-12).
12. Instantiate over a subject with a universe-less overlapping population
    and an empty `initial` list; over an empty `initial` list with bounded
    universes; over records under key `c` typed `Animal` in `animals` and
    `Dog` in `dogs`, in one initial state and then split over two; over the
    same records with agreeing types; over an initial state with no admitted
    key (FR-125-AC-13).
13. Build FR-125-AC-14's `P1` with `make`, check `always holds(x.value = 0)`
    and read the instances and excluded keys (FR-125-AC-14).
14. Instantiate a state-graph claim `over (x: Thing)` over step 9's
    populations; read a hyper clause's parameter `v: V::Thing` over `P` at
    positions holding a `Lamp` and a `Switch`, and a parameter typed `V::Lamp`
    at the `Switch` position (FR-125-AC-15).
15. Build FR-125-AC-16's `all` with `all = [c, e]` and check the unguarded
    `always holds(x.value <= 0)`; repeat with `all = [e, c]` and compare the
    request identities (FR-125-AC-16).

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
- Step 7: instances `l` and `s` for both claims, one per key; excluded keys
  `[u]`; none for the unrelated population.
- Step 8: `over (x: Thing)` reads a reference typed `Lamp`, a reference typed
  `Switch`, and `absent-key` undefined for the absent object; `over (x: Lamp)`
  reads the `Lamp` reference and `absent-key` undefined for the `Switch` and
  absent objects. The obligation identity and key context hold the key alone;
  the refuted binding at `s` is (`UniverseId`, `Switch`, `s`), with no
  population.
- Step 9: `d`, `c`, `b`, `a`; then `d`, `c`, `a`; then instances `d`, `a` and excluded keys `[u]`.
- Step 10: `true` for every formula.
- Step 11: instances `k`, `t`, `d`; `k` reads as the `Dog` reference; the
  unguarded claim refutes with `undefined-evaluation` naming `t`; the claim
  guarded by `G` evaluates everywhere and is `proved`; the `animals`-only
  guard is false at `d`; the `Cat`-only subject has instances, not
  `NoInstance`.
- Step 12: `RequiresBound` naming the population; `NoInitialState`;
  `invalid_runtime_input`/`conflicting-identity` naming `c` and both types,
  in both layouts; admitted; `NoInstance`. No state is explored in any run.
- Step 13: instances `c` and `d`, excluded keys `[z]`, verdict `proved`.
- Step 14: the temporal claim's instances, order and excluded keys; `v` reads
  typed `Lamp` and typed `Switch`; the `V::Lamp` parameter reads `absent-key`
  undefined.
- Step 15: `refuted` in both orders; `[c, e]` reports `c` with no undefined cause, `[e, c]` reports `e` with `undefined-evaluation`; the request identities differ.
