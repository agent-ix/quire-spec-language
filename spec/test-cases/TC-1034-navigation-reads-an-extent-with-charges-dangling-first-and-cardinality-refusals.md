---
id: TC-1034
title: "Navigation reads an extent: its charges, dangling before the count bound, and cardinality refusals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: verifies
---
# TC-1034: Navigation reads an extent: its charges, dangling before the count bound, and cardinality refusals

## Description

Verify FR-364's runtime navigation over an observation's relationship extents:
the results, the exact `model.navigate` schedule, that a dangling target
refuses before the count bound, and where each cardinality and incompleteness
outcome fires.

Scope: FR-364-AC-3, FR-364-AC-5, FR-364-AC-6, FR-364-AC-7, FR-364-AC-8, FR-364-AC-9.

## Test Procedure

Use TC-1033's fixture **orders** and its variants. A navigation is written
`navigate(role, r)`: the role names the end nearest the receiver, as FR-363
and QSpec FR-322 resolve it (the source end's role is the forward traversal
from a `Sys`, `Pump` or other source-type receiver; the target end's role is
the inverse traversal from a target-type receiver), the receiver reference is
`r`, and the multiplicity bound is the destination end's. The site is already
resolved: direction is the checker's, so a row that traverses an inverse role
sets the relationship `undirected` or `target-to-source` or
`bidirectional`. Every `ScalarLimitsV1` counter is unlimited unless a
row sets one. Charges are the `model.navigate` schedule of
`quire.value.accounting/v1`, listed in order with the `value_occurrences` of
the charge when it is not the default; the work and result units are the sums
of the charges listed. Extents are complete unless a row says otherwise.
Tag the tests `#[trace("TC-1034", "FR-364-AC-n")]`.

Base extents: composition `s1`-`p2`, `s1`-`p1`; housing `p1`-`t1`, `p2`-`t1`.
Rows 4 and 10 build their extents through the model-layer entry, which makes
no closure check, so that a dangling end (row 4) and an end of a population
not held complete (row 10) exist to be navigated; a snapshot refuses rows 4
and 10 at check 8, an end naming an unlisted key of a complete population. Every other row's
extents would also admit from a snapshot.

| # | Input | Expected |
| --- | --- | --- |
| 1 | `navigate(parts, s1)` | `Set<Reference<Pump>>[0,3]` equal to `[p1, p2]`, after `model.navigate` (`value_occurrences` 1), two `collection.visit`, `collection.bound` (`value_occurrences` 2) and `collection.result-retain` (`value_occurrences` 3, `result_units += 3`): five work units and three result units |
| 2 | row 1 with `work_units: 5`; then `4` | the result of row 1, identical charges; then `incomplete { limit_kind: work_units, limit: 4, consumed: 4, next_charge: 1, charge_point: collection.result-retain }` and no value |
| 3 | add `p3`, `p4` of `Pump`; composition `s1`-`p1`, `s1`-`p2`, `s1`-`p3`, `s1`-`p4`; `navigate(parts, s1)` | `refused { code: cardinality_out_of_bound, cause: above-maximum }` with bound `[0,3]` and count 4, after `model.navigate`, four `collection.visit` and `collection.bound`: exactly six work units, no result unit, no `collection.result-retain`, no value |
| 4 | row 3 with `p4` absent from `Q` (model-layer extent) | `refused { code: dangling_reference, cause: absent-target-in-complete-population }` for `p4` after `model.navigate` and three `collection.visit` (`p1`, `p2`, `p3`): exactly four work units; decided before `collection.bound`, so no cardinality refusal |
| 5 | `navigate(home, p1)`; then `navigate(home, p2)` | `Reference<Tank>` equal to `t1` each time, after one `model.navigate`: one work unit and no result unit |
| 6 | variant **housing-undirected**; `navigate(pump, t1)` (a traversal from the target end, reading `sources`); then with the housing extent `p1`-`t1` alone | `refused { code: cardinality_out_of_bound, cause: above-maximum }` with collection type `Set<Reference<Pump>>`, bound `[1,1]` and count 2 (`p1` and `p2`) after one `model.navigate` and no other charge; then `Reference<Pump>` equal to `p1` after one `model.navigate` |
| 7 | `navigate(home, p3)` with no housing instance for `p3`; then variant **min** (the composition target end `{2,3}`) with composition `s1`-`p1` alone and `navigate(parts, s1)` | `refused { code: cardinality_out_of_bound, cause: below-minimum }` with collection type `Set<Reference<Tank>>`, bound `[1,1]` and count 0 after one `model.navigate`, no other charge; then `below-minimum` with bound `[2,3]` and count 1 after `model.navigate`, one `collection.visit` and `collection.bound`: three work units, no `collection.result-retain` |
| 8 | variant **optional** (the housing target end `{0,1}`); `navigate(home, p3)`; then `navigate(home, p1)` | `Option<Reference<Tank>>` `none` after one `model.navigate`; then the present option naming `t1` after one `model.navigate` |
| 9 | composition listed `complete: false` holding `s1`-`p1`, `navigate(parts, s1)`; composition not listed; composition listed complete with no instance and `navigate(parts, s2)`; housing complete with no instance and `navigate(home, p3)` under variant **optional** | `incomplete { code: incomplete_population, cause: incomplete-scope }` retaining composition and `s1`, after one `model.navigate` and no `collection.visit`, twice; then the empty `Set<Reference<Pump>>[0,3]` after one `model.navigate` and no `collection.visit`; then `none` |
| 10 | (model-layer extent) composition `s1`-`p9`, where the observation does not hold `p9` and a declared population covering `Pump` is not held complete (TC-1041's variant **spare**); `navigate(parts, s1)` | `incomplete_population`/`incomplete-scope`, not `dangling_reference`, after one `model.navigate`; no cardinality refusal |
| 11 | variant **bag**; composition `s1`-`p1` twice; `navigate(parts, s1)` | `Bag<Reference<Pump>>[0,3]` equal to `[p1, p1]`, with two `collection.visit` and count 2 |
| 12 | row 1 twice, then compare the observation with a clone taken before | equal results, equal charges, an unchanged observation |
| 13 | variant **bag-single** (housing both ends `unique: false`, the target end `{1,1}`); housing `p1`-`t1` twice; `navigate(home, p1)`; then with the target end `{0,1}` and the same extent | `refused { code: cardinality_out_of_bound, cause: above-maximum }` with collection type `Bag<Reference<Tank>>`, bound `[1,1]` and count 2 after one `model.navigate` and no other charge; then the same refusal with bound `[0,1]`, count 2 and collection type `Bag<Reference<Tank>>` |

## Expected Results

Every navigation takes its targets from the extent in ascending reference-key
order and no other source. A refusal makes no later charge. A dangling target
refuses before any cardinality refusal; an unlisted or incomplete extent, or a
target absent from a population not held complete, is incomplete rather than
empty or dangling; a count outside the bound refuses with its direction only
after every target resolved.
