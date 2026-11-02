---
id: TC-1312
title: "Write a checked-package handoff directory from source, in process"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: verifies
---
# TC-1312: Write a checked-package handoff directory from source, in process

## Description

Verify the checked-package handoff writer on the ported IT-001 recipe **R**.
Scope: FR-517-AC-1 to FR-517-AC-11.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `write(R, new dir)` with `corpus = None`, then with `Standard` | the paths of the Outputs table other than `inventory.json`, no `mutations/` first; `checked-package.json` equals `emit_checked_v2::emit` of the same request |
| 2 | 2 | read `checked-package.json` with `read_checked_package` and evidence from `models/` and `sources/`; `Selection::check` | admitted; `Ok(())`; sidecar values, compensation rows included, equal the in-process unit |
| 3 | 3 | count and list the nodes and `temporal_bindings` of **R** | one `protocol_clause`; two compensation nodes with different keys; three clauses; one row per clause and per compensation template in ascending `NodeRef` order (four in the event-position package, one each in the other two) with kinds `EventPosition`, `FixedSample`, `TimestampedEvent` |
| 4 | 4 | read the `Wide` and `Exact` values of **R** | the two extrema and the two extreme rationals present; bounds as declared |
| 5 | 5 | two writes into two new directories | byte-identical trees |
| 6 | 6 | write onto an existing directory; inject a failure after the temporary tree | `Error::Exists`, directory unchanged; no directory or temporary directory left |
| 7 | 7 | source with a type error; a package the reader refuses; injected `ReadBackDiffers` | the compiler refusal; the reader's refusal; `ReadBackDiffers`; nothing written in all three |
| 8 | 8 | build a crate depending on `checked-handoff` only; compare `Cargo.lock` | builds; no new lock entry |
| 9 | 9 | change the `sequence_authority` of the event-position row to another non-empty text; `git grep` for a second reader | admitted by `read_checked_package`; `Selection::check` refuses `clock.sequence_authority`; no second reader |
| 10 | 10 | list the legacy writer's items on the head of any change that touches them | all present; a change deleting one is not merged while FR-521 row 2 is not green |
| 11 | 11 | with `qsl_inventory` over the spine unit available, write **R** and read `inventory.json` in each directory | equal to `qsl_replay::qsl_inventory` over the unit with the selected formal identity; gated on QSL-1512 |

Tag the test `#[trace("FR-517-AC-1", ..., "FR-517-AC-11", "TC-1312")]`; row 3 and 4 also carry FR-042-AC-10.

## Expected Results

Each row's Expected column.
