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
Scope: FR-519-AC-1 to FR-519-AC-7. **R** is the package of FR-517.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `causal_relation` over **R** in process and read back; compare with FR-218's compiled edge relation | equal pairs, `repeat` back edges present, strictly ascending, no duplicate; the two derivations equal |
| 2 | 2 | `git grep` for a second derivation; the FR-383-AC-16 test's call | none; it calls the function |
| 3 | 3 | `protocol_view` over **R** and the in-process rebuild (FR-383-AC-19) | the same clause, roles, channels, queues, controls, compensations, requirement and activation rows |
| 4 | 4 | a missing id; a `temporal_clause` id; a `protocol_reference` carrier | `UnknownProtocol`; `UnknownProtocol`; `NotAProtocolClause`; no partial result |
| 5 | 5 | read each successor record kind from **R** through the views; remove its member and read again; compile-fail use of each Ended record | equal to the in-process value; `ViewRefusal` naming the member; does not compile |
| 6 | 6 | `arch-lint direction` over the four checkouts; `git grep` for byte reads in the view modules | no cycle; no byte read |
| 7 | 7 | `linear issue view QSL-1549`, relations | project and milestone set; blocks QSL-1501; blocked by QSL-1544 to QSL-1548; three obligations listed |

Tag the test `#[trace("FR-519-AC-1", ..., "FR-519-AC-7", "TC-1314")]`; row 1 also carries FR-383-AC-16.

## Expected Results

Each row's Expected column.
