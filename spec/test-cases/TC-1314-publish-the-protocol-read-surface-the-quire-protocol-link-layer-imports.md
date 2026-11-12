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
Scope: FR-519-AC-1 to FR-519-AC-23, AC-7 retired. **R** is the package of FR-517, except for rows 12 and 18.

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
| 14 | 14 | `attempts_maximum` over **R** for `Full` and `Partial`; over fixtures the test authors with `attempts 1` and `attempts 18446744073709551615` (**R** has neither; its authored value is 3); compare FR-520-AC-4 row 5 of TC-1315, which reads the same bound | 3 and 3, equal to the in-memory counts; 1; 18446744073709551615 |
| 15 | 15 | `attempts_maximum` over a copy of the `Full` node with `maximum` removed from its `attempts` group | `ViewRefusal::MissingMember` naming `attempts.maximum`; no value |
| 16 | 16 | `attempts_maximum` with an id absent from the package, a role id and a control id | `ViewRefusal::NotACompensation` naming the id, each time; no value |
| 17 | 17 | `attempts_maximum` over a copy of the `Full` node with `maximum` set to zero, and to a non-integer term | `ViewRefusal::MalformedMember` naming `attempts.maximum`, each time; no value |
| 18 | 18 | `await_anchor` over a protocol the test authors (**R** has no `await`) with one `await` after an `event` control and one after a `compensation` template; then over copies of the first node with `after` removed, with `after.kind` set to `signal`, and with `after.target` removed | `Event` with the event control's id; `Compensation` with the template's id; each equal to the in-memory `after` Group; `MissingMember` naming `after`; `MalformedMember` naming `after`, twice; no value for any refusal |
| 19 | 19 | `channel_endpoints` over **R**'s channel `Decisions`; then over copies with `from`, `to` and `carries` each removed, and with `from` set to a text term | the sender role, receiver role and message type node ids, equal to the in-memory channel; `MissingMember` naming each removed member; `MalformedMember` naming `from`; no value for any refusal |
| 20 | 20 | `choice_visibility` over **R**'s `choice` `Outcome`; then over copies with `owner` and `visible` each removed, with `owner` set to a non-reference, with `visible` set to a non-set, and over **R**'s `repeat` `Poll` | the choice's id, owner node and visible `expression` node ids, equal to the in-memory node; `MissingMember` naming each removed member; `MalformedMember` naming `owner`, then `visible`; `MalformedMember` naming `operator`; no value for any refusal |
| 21 | 21 | `channel_ordering` and `fifo_key` over **R**'s FIFO channel `Charges` and over an unordered channel the test authors; then over copies with `ordering` removed and with `ordering.kind` set to an unknown word | `Fifo` with the binder and key node ids the `ordering` Group references; `Unordered`; `fifo_key` over the unordered channel `MalformedMember` naming `ordering`; `MissingMember` naming `ordering`; `MalformedMember` naming `ordering`; no value for any refusal |
| 22 | 22 | `queue_overflow` over queues the test authors with overflow `reject`, `block` and `loss` with `head`, `tail` and `new`; then over copies with `overflow` or `loss` removed, with an unknown word, with an unknown selector, with a selector on `reject`, with a selector on `block`, and with no selector on `loss` | the equal `Overflow` value for each; `queue_channel` the channel node id the queue references, `MissingMember` naming `channel` on a copy without it and `MalformedMember` naming `channel` on a copy whose `channel` is no reference; `MissingMember` naming each removed member; `MalformedMember` naming `overflow` or `loss`; no value for any refusal |
| 23 | 23 | scan the public items of `qsl_package::protocol_view` for a function that returns a value path | none |

Tag the test `#[trace("FR-519-AC-1", ..., "FR-519-AC-23", "TC-1314")]`; row 1 also carries FR-383-AC-16.

## Expected Results

Each row's Expected column.
