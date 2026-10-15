---
id: TC-1033
title: "Relationship extents admit under four conditions and read in reference-key order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: verifies
---
# TC-1033: Relationship extents admit under four conditions and read in reference-key order

## Description

Verify FR-364's extent admission, that no multiplicity bound is checked at
admission, and the read contract navigation relies on, at the model layer
(extents supplied as full references, with no snapshot document).

Scope: FR-364-AC-1, FR-364-AC-2, FR-364-AC-3, FR-364-AC-4.

## Test Procedure

Fixture **orders** (domain package `ix://test/orders`; every identity below is
under it):

- object types `Sys`, `Pump`, `Tank`, all concrete, each its own supertype
  component; `Pump` has the optional field `backup: Reference<Pump>`;
- population `Q`, extent `closed`, member types `[Sys, Pump, Tank]`, holding
  `s1`, `s2` of `Sys`; `p1`, `p2`, `p3` of `Pump`; `t1` of `Tank`;
- navigation relationship `relationship/Sys-composition-Pump`: source end
  role `parts` (`Sys`, `{1,1}`), target end role `owner` (`Pump`, `{0,3}`),
  direction `source-to-target`; `s1.parts` is the forward traversal, whose
  destination is the target end `{0,3}`, and `p1.owner` the inverse, whose
  destination is the source end `{1,1}`;
- navigation relationship `relationship/Pump-housing-Tank`: source end role
  `home` (`Pump`, `{1,1}`), target end role `pump` (`Tank`, `{1,1}`),
  direction `source-to-target`; `p1.home` is the forward traversal and
  `t1.pump` the inverse;
- a Connection `pipe` between two ports, so that `pipe` is a declaration of
  the package that is not a navigation relationship.

Multiplicity `{l,u}` means `ordered: false, unique: true`. Variants of the
fixture: **bag** (both ends of `Sys-composition-Pump` `unique: false`),
**bag-target** (only its target end `unique: false`).

Objects are referenced by their full reference key. Build an observation's
extents through the model-layer entry that FR-106 also uses. Tag the tests
`#[trace("TC-1033", "FR-364-AC-n")]`.

| # | Step | Expected |
| --- | --- | --- |
| 1 | extents built in this order: composition `s1`-`p2`, `s1`-`p1`, complete; housing `p2`-`t1`, `p1`-`t1`, complete | admitted; `complete` true for both; `targets(composition, s1)` is `[p1, p2]`; `sources(composition, p1)` is `[s1]`; `targets(housing, p1)` is `[t1]`; `sources(housing, t1)` is `[p1, p2]`; the same extents built in the reverse order give the same reads |
| 2 | an extent for `pipe`; then one for `relationship/Nope-x-Pump` | `invalid_runtime_input`/`wrong-role-mapping`, retaining the relationship, twice; nothing admitted |
| 3 | two extents for composition | `invalid_runtime_input`/`conflicting-identity` retaining the relationship |
| 4 | composition instance `t1`-`p1`; then `s1`-`t1` | `invalid_runtime_input`/`wrong-value-kind` naming composition, end `source` and `Tank` against `Sys`; then end `target` and `Tank` against `Pump` |
| 5 | composition `s1`-`p1` twice | `invalid_runtime_input`/`conflicting-identity` at the second instance |
| 6 | step 5 over variant bag; then over variant bag-target | step 5's refusal over bag-target; over bag, admitted with two instances: `targets(composition, s1)` is `[p1, p1]` and `sources(composition, p1)` is `[s1, s1]` |
| 7 | add `p4` of `Pump`; composition `s1`-`p1`, `s1`-`p2`, `s1`-`p3`, `s1`-`p4`; housing with no instance for `p3`; then an observation with no extent at all | each admitted; no bound is compared, and the four targets read `[p1, p2, p3, p4]` |
| 8 | composition listed complete with no instance; composition listed `complete: false` holding `s1`-`p1`; housing not listed | `complete` true with empty `targets(composition, s1)`; `complete` false with `targets(composition, s1)` `[p1]`; `complete` false for housing |
| 9 | add a `Tank` object `p1` (a different universe from the `Pump` `p1`); read `sources(housing, Tank p1)` and `targets(housing, Pump p1)` | `[]` and `[t1]`; the two references are not equal |
| 10 | read every read of step 1 twice; clone the observation before and compare it after | equal results; the observation equals its clone; no charge was made by any read |

## Expected Results

Each refusal is the one named for its condition alone, with no extent admitted
for the refused input and no other extent changed. An extent is admitted
without a count comparison; reads are in ascending reference-key order,
independent of build order, equal under whole-key equality and free of side
effects.
