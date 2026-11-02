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

Verify FR-527-AC-1 to FR-527-AC-15. The unit **M** holds one clause per row of FR-045's table, plus the connective and no-operator clauses below, packaged and read back with the reader. Targets and premises are FR-045's and are read, not copied.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | one clause per FR-045 table row | each row's disposition and TL target |
| 2 | 2 | finite-window clause; past and future mixed; each of `once`, `historically`, `since`, `triggered` alone under both false-extension profiles | `Unsupported`; `Unsupported`; `Supported`, `mltl.origin-complete-history/v1` |
| 3 | 3 | finite-window clause with mixed directions; infinite-trace clause | `FiniteWindow` and `MixedDirection`; `InfiniteTrace` |
| 4 | 4 | `not once[0,1] holds(p)`; `holds(p) implies eventually[0,2] holds(q)`; `once[0,1] holds(p) implies eventually[0,2] holds(q)` | past-only; future-only; mixed-direction |
| 5 | 5 | `holds(p)` under each false-extension profile, `Complete` then `Open` | closed-trace target; online-prefix target |
| 6 | 6 | fixed-sample future clause; event-position future clause | `total_sample_valuation` true under both closures; false |
| 7 | 7 | an unsupported result | `CheckedNodeId`, name, profile identity and activation equal the view's; package unchanged |
| 8 | 8 | a supported result | exact target and eight premises; no correspondence claim |
| 9 | 9 | equal formula bytes under two profiles | different classifications |
| 10 | 10 | a call passing a decision-scope closure (compile-fail); `surrounding` changed | does not compile; only the future rows' target changes |
| 11 | 11 | `selection` of every result | package identity, clause, definition identity, revision, artifact identity equal to the lock row; no clock parameter map |
| 12 | 12 | constructing `AuthenticatedSelection` outside the classifier (compile-fail) | does not compile |
| 13 | 13 | **R**'s three clauses, in-process vs read-back; classify twice | equal classifications |
| 14 | 14 | a view with no matching `profile_selections` row; a node that is not a temporal clause | `NoProfileSelection`; `temporal_subject` refuses `UnresolvedClause` |
| 15 | 15 | inspect `mapping_support`'s reads | only the view and `surrounding` |

Tag the test `#[trace("FR-527-AC-1", ..., "FR-527-AC-14", "TC-1322")]`; AC-15 is Inspection and carries no test tag.

## Expected Results

Each row's Expected column.
