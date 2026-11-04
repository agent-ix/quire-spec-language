---
id: TC-1314
title: "Publish the protocol read surface the quire-protocol link layer imports"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: verifies
---
# TC-1314: Publish the protocol read surface the quire-protocol link layer imports

## Description

Verify the causal relation, the protocol view and the record-to-node map.
Scope: FR-519-AC-1 to FR-519-AC-13, AC-7 retired. **R** is the package of FR-517, except for row 12.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `causal_relation` over **R** in process and read back; compare with FR-218's compiled edge relation | equal pairs, `repeat` back edges present, strictly ascending, no duplicate; the two derivations equal |
| 2 | 2 | a test that scans `qsl-*/src` and `src` for functions that derive a causal relation from `CheckedPackageV2` nodes | exactly one, `causal_relation`; the checker's `compile` and the `wire` expansion `edges` start from other inputs |
| 3 | 3 | `protocol_view` over **R** and the in-process rebuild (FR-383-AC-19) | the same clause, roles, channels, queues, controls, compensations, requirement and activation rows |
| 4 | 4 | a missing id; a `temporal_clause` id; a `protocol_reference` carrier | `UnknownProtocol`; `UnknownProtocol`; `NotAProtocolClause`; no partial result |
| 5 | 5 | read each record kind the map marks read by a view function from **R** | equal to the value the checked clause holds in memory |
| 6 | 6 | `arch-lint direction` over the four checkouts; `git grep` for byte reads in the view modules | no cycle; no byte read |
| 7 | 7 | retired: the ticket's relations and obligations are tracker data (FR-519-AC-7) | none |
| 8 | 8 | for each view function over a node, lock or source map, a copy with the member removed | `ViewRefusal` naming the member; no partial value |
| 9 | 9 | compile-fail use of `ExportRef`, `ArtifactKind`, `Revision`, `Language`, `Definition` | none resolves |
| 10 | 10 | compile a test that passes a `CheckedPackageV2` and a `CheckedNodeId` to `protocol_view`, `causal_relation` and `binder`; read the returned ids | compiles; the ids are `CheckedNodeId` |
| 11 | 11 | read `R`'s `semantic_type` nodes and literal and reference terms through `CheckedPackageV2::term` | equal to the in-memory values; numbers in the FR-322 encoding |
| 12 | 12 | select the population declaration node from the package of TC-1054's model by node id (**R** of FR-517 declares no population) | equal to the in-memory node |
| 13 | 13 | compile-fail `use qsl_package::protocol_view::Window` | does not resolve |

Tag the test `#[trace("FR-519-AC-1", ..., "FR-519-AC-13", "TC-1314")]`; row 1 also carries FR-383-AC-16.

## Expected Results

Each row's Expected column.
