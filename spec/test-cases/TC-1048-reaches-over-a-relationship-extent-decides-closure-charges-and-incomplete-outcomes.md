---
id: TC-1048
title: "reaches over a relationship extent decides one-or-more-edge closure, its charges and its incomplete and dangling outcomes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-365
    type: verifies
---
# TC-1048: reaches over a relationship extent decides one-or-more-edge closure, its charges and its incomplete and dangling outcomes

## Description

Verify the evaluation of `reaches(a, b, name)` over a relationship end: the
Boolean for each graph shape, the exact `graph` charge schedule and budgets,
and where each incomplete, dangling and undecidable outcome fires. Scope:
FR-365-AC-7 to FR-365-AC-12.

Catches a traversal that is zero-length, that uses the first-found edge only,
that loops on a cycle, that answers `true` from an incomplete extent, that
treats a dangling neighbour as an absent edge, that compares a neighbour count
with a bound, or that charges `model.navigate`.

## Test Procedure

Use TC-1046's fixture **R**, relationship `Order/next` (forward role `next`)
and TC-1033's way of building extents; every extent is complete unless a row
says otherwise and every `ScalarLimitsV1` counter is unlimited unless a row
sets one. Objects `o1`, `o2`, `o3`, `o4` and `o5` are `Order` objects of one
universe held by the observation; `o4` and `o5` have equal field values and
different identities; `r1` is a `Rush`. A reachability is written
`reaches(x, y)` with the role `next` unless stated. Charges are listed in order
with their `value_occurrences` `k` for `graph.expand`; the work and result units
are the sums (each charge is one work unit, `graph.result-retain` is also one
result unit). Rows 20 and 21 build their extents through the model-layer entry,
which makes no closure check. Tag the tests `#[trace("TC-1048", "FR-365-AC-n")]`.

Base extent **chain**: `o1`-`o2`, `o2`-`o3` (a link is source-target).

| # | Input | Expected |
| --- | --- | --- |
| 1 | **chain**; `reaches(o1, o3)` | `true`: `graph.expand` o1 (`k`=1), `graph.edge`, `graph.expand` o2 (`k`=2), `graph.edge`, `graph.result-retain`: five work units, one result unit |
| 2 | **chain**; `reaches(o3, o1)`; `reaches(o1, o1)` | `false` after `graph.expand` o3, `graph.result-retain` (two work units); `false` after expand o1, edge, expand o2, edge, expand o3, `graph.result-retain` (six work units; o3 has no outgoing link) |
| 3 | isolated `o4`; `reaches(o4, o4)`; `reaches(o4, o5)` | `false`, `false`, each after `graph.expand` and `graph.result-retain` (two work units): not zero-length, and equal-valued objects are not the same target |
| 4 | self-loop `o4`-`o4`; `reaches(o4, o4)` | `true`: expand, edge, result-retain (three work units) |
| 5 | cycle `o1`-`o2`, `o2`-`o1`; `reaches(o1, o1)` | `true`: expand o1, edge, expand o2 (`k`=2), edge to o1 matches, result-retain: five work units |
| 6 | cycle `o1`-`o2`, `o2`-`o1` and isolated `o3`; `reaches(o1, o3)` | `false`: expand o1, edge, expand o2, edge to o1 (discovered, no expand), result-retain: five work units |
| 7 | a complete extent with no link; `reaches(o1, o2)` | `false` after expand and result-retain: two work units |
| 8 | **chain** with `r1` (a `Rush`) in place of `o1`: `r1`-`o2`; `reaches(r1, o2)` | `true`: a subtype start navigates the base-typed relationship |
| 9 | variant **bag** (both ends `unique: false`): `o1`-`o2` twice and `o2`-`o3`; `reaches(o1, o3)`; then the extent `o1`-`o2` twice alone and `reaches(o1, o5)` | `true`: expand o1, edge, expand o2 (`k`=2), edge (the repeat; o2 is discovered once), edge to o3 matches, result-retain: six work units; then `false`: expand o1, edge, expand o2, edge, result-retain: five work units |
| 10 | **bidirectional** `Order/link` extent `o1`-`o2`, `o2`-`o3`; `reaches(o3, o1)` with role `incoming`; `reaches(o3, o1)` with role `outgoing` | `true` (reads `sources`: o3's source o2, o2's source o1); `false` |
| 11 | row 1 with `work_units: 5`; then `4`; then `3` | `true` with the row 1 charges; then `incomplete { limit_kind: work_units, limit: 4, consumed: 4, next_charge: 1, charge_point: graph.result-retain }`; then `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: graph.edge }`; no Boolean from either |
| 12 | row 1 with `value_occurrences: 2`; then `1` | `true`; then `incomplete { limit_kind: value_occurrences, limit: 1, consumed: 1, next_charge: 2, charge_point: graph.expand }` at o2; no Boolean |
| 13 | row 1 with `result_units: 1`; then `0` | `true`; then `incomplete { limit_kind: result_units, limit: 0, consumed: 0, next_charge: 1, charge_point: graph.result-retain }` |
| 14 | row 1 with `work_units: 0` | `incomplete { limit_kind: work_units, limit: 0, consumed: 0, next_charge: 1, charge_point: graph.expand }` before the start is discovered |
| 15 | row 1 twice, then row 11's `4` run, then row 1 again; compare the observation with a clone taken before | equal results and charges on every complete run; the observation equals its clone; the incomplete run does not change the later result |
| 16 | **chain** with the extent listed `complete: false`; `reaches(o1, o3)`; then the extent not listed; then `reaches(o1, o2)` over an incomplete extent holding only `o1`-`o2` | `incomplete_population`/`incomplete-scope` retaining `Order/next` and `o1` after `graph.expand` o1 (one work unit) and no further charge, three times: a known link does not answer `true` |
| 17 | extent `o1`-`o2`, `o1`-`o3`, `o1`-`o4` with `Order/next`'s target end `{0,1}`; `reaches(o1, o5)`; then the same graph with the target end `{1,1}` | `false` both times: expand o1 (`k`=1), edge o2, expand o2, edge o3, expand o3, edge o4, expand o4 (`k`=4), result-retain: eight work units, and no `cardinality_out_of_bound` for the three links above `{0,1}` nor `below-minimum` for `o2`, `o3` and `o4`, which hold none |
| 18 | precondition `reaches(self, target, next)` and postcondition `reaches(self, target, next)` of `reroute` with `self`=`o1`, `target`=`o2`, the pre extent `o1`-`o2` and the post extent empty | `true` for the pre clause; `false` for the post clause: each reads its own observation's extent |
| 19 | `Order/next` holds no link; `Order/link` holds `o1`-`o3`; the field `trail` of `o1` names `o3`; `reaches(o1, o3)` with role `next` | `false` after expand and result-retain (two work units): only the named relationship's extent is read, never another relationship or a field |
| 20 | (model-layer extent) `o1`-`o2`, `o2`-`d9`, where the plain `Order` `d9` is not held and the manifest is closed with every row covering `Order` complete; `reaches(o1, o3)`; then `reaches(o1, d9)` | `dangling_reference`/`absent-target-in-complete-population` retaining `d9` and `Order/next`, after expand o1, edge, expand o2 (three work units) and before any `graph.edge` for `d9`, both times: `d9` as the target `b` is refused, not matched |
| 21 | (model-layer extent, variant **spare**: an added population `fast` with member type `Rush`, not held complete, so a missing `Rush` is undecidable) `o1`-`o2`, `o2`-`s9` with `s9` a missing `Rush`; then `o2` linked to a missing `Rush` `s1` and a missing `Order` `d2`; reference-key order is (universe, type, id), the type identities decide, and in this fixture `d2` (an `Order`) sorts before `s1` (a `Rush`) | `incomplete_population`/`incomplete-scope` retaining `s9` and `Order/next` after the three work units of row 20; then `dangling_reference` retaining `d2`: the first neighbour in order that is not present decides, whichever kind it is |
| 22 | `reaches(d9, o1)` with `d9` a missing `Order` and the manifest closed and complete; under variant **spare**, `reaches(s9, o1)` with `s9` a missing `Rush` | `dangling_reference`/`absent-target-in-complete-population` retaining `d9` and `Order/next`; then `incomplete_population`/`incomplete-scope` retaining `s9`, each with no charge at all |
| 23 | every functional graph on one, two and three objects, each object holding no link or one link of `Order/next` to any object of the graph, then the same graphs as `Order/link` read through `incoming` | for every ordered pair, the Boolean equals the one-or-more-edge closure computed by an independently authored Boolean adjacency-matrix closure (for `incoming`, the closure of the reversed relation); renaming object identities changes no result; distinct equal-valued objects stay distinct |

## Expected Results

Reachability is the least relation closed under one link and composition of
links of the named relationship's extent in the clause's observation, never
zero-length. The charge schedule is the `graph` rows in discovery order, a
repeated link charges one `graph.edge` each and is discovered once, and the
exact required counter completes while one less is incomplete at the stated
charge point. An unlisted or incomplete extent is incomplete, never `false`; a
dangling neighbour refuses and an undecidable one is incomplete, both before
the `graph.edge` for that neighbour; no neighbour count is compared with a
bound; a read charges nothing and changes nothing.
