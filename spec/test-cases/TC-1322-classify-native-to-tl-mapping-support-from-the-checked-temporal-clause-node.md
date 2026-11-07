---
id: TC-1322
title: "Classify native-to-TL mapping support from the checked temporal clause node and retain the authenticated selection"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-527
    type: verifies
---
# TC-1322: Classify native-to-TL mapping support from the checked temporal clause node and retain the authenticated selection

## Description

Verify FR-527-AC-1 to FR-527-AC-26. The unit **M** holds one clause per row of FR-045's table, plus the formulas below, packaged and read back with the reader. Targets and premises are FR-045's and are read, not copied.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | one clause per FR-045 table row | each row's disposition and TL target |
| 2 | 2 | a finite-window clause; a clause with a past and a future operator | `Unsupported` naming `FiniteWindow`; `Unsupported` naming `MixedDirection` |
| 3 | 3 | each of `once`, `historically`, `since`, `triggered` alone under both false-extension profiles | `Supported`, `mltl.origin-complete-history/v1` |
| 4 | 4 | a finite-window clause with mixed directions; an infinite-trace clause | `FiniteWindow` and `MixedDirection`; `InfiniteTrace` |
| 5 | 5 | `once[0,1] holds(p)`; `eventually[0,2] holds(q)`; a formula holding both; each with `not`, `and`, `or` and `implies` added | past-only; future-only; mixed-direction; classification unchanged by the connectives |
| 6 | 6 | `holds(p)` under each false-extension profile, `Complete` then `Open` | closed-trace target; online-prefix target |
| 7 | 7 | fixed-sample future clause; event-position future clause | `total_sample_valuation` true under both closures; false |
| 8 | 8 | an unsupported result | `CheckedNodeId`, profile row and activation equal the view's; package unchanged |
| 9 | 9 | a supported result | exact target and eight premises; no correspondence claim |
| 10 | 10 | equal formula bytes under two profiles | different classifications |
| 11 | 11 | a call passing a decision-scope closure (compile-fail) | does not compile |
| 12 | 12 | `surrounding` changed | only the future rows' target changes |
| 13 | 13 | `selection` of every result | package identity, clause, definition `authority` and `identity` (the `/vN` suffix the revision) equal to the lock row; no clock parameter map |
| 14 | 14 | constructing `AuthenticatedSelection` outside the classifier (compile-fail) | does not compile |
| 15 | 15 | each temporal clause of **R**'s three packages: the reader over the bytes the emitter returns without a file vs the reader over the written handoff tree's bytes | equal support, retained clause and selected definition facts; different admission bindings, each belonging to its own admitted package |
| 16 | 16 | a view whose profile row names an identity outside the five admitted profiles | `NoProfileSelection` |
| 17 | 17 | a node that is not a temporal clause | `temporal_subject` refuses `UnresolvedClause` |
| 18 | 18 | a unit selecting a temporal profile outside the five admitted identities | refused at check with `UnsupportedSelection`; no clause node |
| 19 | 19 | classify one clause twice | equal results |
| 20 | 20 | inspect `mapping_support`'s reads | only the view and `surrounding` |
| 21 | 21 | a package whose lock lacks the `profile_selections` row of a temporal clause's `temporal_profile` law, read with the reader | refused; no view |
| 22 | 22 | a clause selected under `quire.temporal.timed/v1` | `Unsupported` naming `TimedProfile`, no target |
| 23 | 23 | a timed clause with a past and a future operator | `Unsupported` naming `TimedProfile` and `MixedDirection` |
| 24 | 24 | the one-occurrence clauses `Gamma` (supported) and `Delta` (timed, unsupported), and FR-337-AC-16's `Alpha` and `Beta` sharing one node; the retained name | `Some("Gamma")`, `Some("Delta")` and `None`, each equal to the view's `name()` |
| 26 | 26 | supported and unsupported clauses; two independent strict admissions of identical original producer bytes; unchanged clones and a move of the original; retained selection after its source borrow and package are dropped | equal content does not authorize a foreign admission; owner binding accepts original and unchanged clones across moves, rejects the independent admission, stays retained by the owned classification, and participates in classification equality |

Tag the test `#[trace("FR-527-AC-1", ..., "FR-527-AC-19", "TC-1322")]`, and AC-21 to AC-24 likewise; AC-20 is Inspection and carries no test tag.

## Expected Results

Each row's Expected column.
