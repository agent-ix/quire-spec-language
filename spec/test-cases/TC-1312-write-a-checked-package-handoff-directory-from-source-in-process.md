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
Scope: FR-517-AC-1 to FR-517-AC-16.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `write(R, new dir)` with `corpus = None`, then with `Standard` | the paths of the Outputs table other than `inventory.json`, no `mutations/` first; `checked-package.json` equals `emit_checked_v2::emit` of the same request |
| 2 | 2 | read `checked-package.json` with `read_checked_package` and evidence from `models/` and `sources/`; `Selection::check` | admitted; `Ok(())`; sidecar values, compensation rows included, equal the in-process unit |
| 3 | 3 | count and list the nodes and `temporal_bindings` of each package of **R** | event position: one `protocol_clause`; two compensation nodes with different keys paired with the one `Main::Applied` effect; two clauses; four rows in ascending `NodeRef` order; fixed sample and timestamped event: one clause and one row each; kinds `EventPosition`, `FixedSample`, `TimestampedEvent` |
| 4 | 4 | read the `Wide` values of **R** | the two extrema present; bounds as declared |
| 5 | 5 | two writes into two new directories | byte-identical trees |
| 6 | 6 | write onto an existing directory; inject a failure after the temporary tree | `Error::Exists`, directory unchanged; no directory or temporary directory left |
| 7 | 7 | source with a type error; a package the reader refuses; injected `ReadBackDiffers` | the compiler refusal; the reader's refusal; `ReadBackDiffers`; nothing written in all three |
| 8 | 8 | build a crate depending on `checked-handoff` only; compare `Cargo.lock` | builds; no new lock entry |
| 9 | 9 | change the `sequence_authority` of one event-position `temporal_bindings` row to another non-empty text; `git grep` for a second reader | admitted by `read_checked_package`; `Selection::check` refuses `clock.sequence_authority`; no second reader |
| 10 | 10 | list the legacy writer's items on the head of any change that touches them | all present; a change deleting one is not merged while FR-521 row 2 is not green |
| 11 | 11 | with `qsl_inventory` over the spine unit available, write **R** and read `inventory.json` in each directory | equal to `qsl_replay::qsl_inventory` over the unit with the selected formal identity; gated on QSL-1512 |
| 12 | 12 | with the spine admitting a bounded rational value type, read the `Exact` values of **R**; gated on QSL-1522 | the two extreme rationals present |
| 13 | 13 | with the model admitting a unit on a bounded scalar, read `Amount` and `Total` of **R**; gated on QSL-1558 | both carry unit `U` |
| 14 | 14 | with a `sum` query binder admitted inside `recover`, read the `recover` conditions of `Full` and `Partial`; gated on QSL-1555 | `Full`: receipt sum equals the captured target; `Partial`: receipt sum positive and below it |
| 15 | 15 | run `emit_checked_v2` with `--clock-selections` holding each **R** package's clock configurations; then with an empty object, a repeated name, an unknown `kind`, an undefined member and a missing member; then a clause name no clause or template has | the bytes `emit` returns for the equal `Request`; each malformed file a refusal naming it and the unknown name an `UnknownClause` refusal, no bytes written |
| 16 | 16 | write a source over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`, declaring `U`, whose body is in turn `amount >= 1 U` in a `pre` of `apply`; an invariant reading only `self.ready`; `sum<M::Total>` over the receipts; `sum<Cash>` with `type Cash = Quantity<U>;` | written and admitted; `ReaderRefused` with `missing_declaration`/`missing-name` at `/types/7/fields/0/typeRef`; `UnresolvedTypeName` for `M::Total`; `IllTyped(TypeMismatch)` at the state clause |

Tag the test `#[trace("FR-517-AC-1", ..., "FR-517-AC-16", "TC-1312")]`; row 3 and 4 also carry FR-042-AC-10.

## Expected Results

Each row's Expected column.
