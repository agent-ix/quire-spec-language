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
Scope: FR-517-AC-1 to FR-517-AC-23.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `write(R, new dir)` with `corpus = None`, then with `Standard` | the paths of the Outputs table, `inventory.json` included, no `mutations/` first; `checked-package.json` equals `emit_checked_v2::emit` of the same request |
| 2 | 2 | read `checked-package.json` with `read_checked_package` and evidence from `models/` and `sources/`; `Selection::check` | admitted; `Ok(())`; sidecar values, compensation rows included, equal the in-process unit |
| 3 | 3 | count and list the nodes and `temporal_bindings` of each package of **R** | event position: one `protocol_clause`; two compensation nodes with different keys paired with the one `Main::Applied` effect; two clauses; four rows in ascending `NodeRef` order; fixed sample and timestamped event: one clause and one row each; kinds `EventPosition`, `FixedSample`, `TimestampedEvent` |
| 4 | 4 | read the `Wide` values of **R** | the two extrema present; bounds as declared |
| 5 | 5 | two writes into two new directories | byte-identical trees |
| 6 | 6 | write onto an existing directory; inject a failure after the temporary tree | `Error::Exists`, directory unchanged; no directory or temporary directory left |
| 7 | 7 | source with a type error; a package the reader refuses; injected `ReadBackDiffers` | the compiler refusal; the reader's refusal; `ReadBackDiffers`; nothing written in all three |
| 8 | 8 | build a crate depending on `checked-handoff` only; compare `Cargo.lock` | builds; no new lock entry |
| 9 | 9 | change the `sequence_authority` of one event-position `temporal_bindings` row to another non-empty text; `git grep` for a second reader | admitted by `read_checked_package`; `Selection::check` refuses `clock.sequence_authority`; no second reader |
| 10 | 10 | list the legacy writer's items on the head of any change that touches them | all present; a change deleting one is not merged while FR-521 row 2 is not green |
| 11 | 11 | write **R** and read `inventory.json` in each directory; write again | bytes equal the encoding of `qsl_replay::qsl_inventory` over the unit run through the replay spine, with the formal identity of FR-517 "The inventory file" (the sanitized file name of the unit, revision 1; `unit-event-position.native` for the event-position package); rows non-empty, `observed_count` equal to their number; the two writes equal |
| 12 | 12 | write each of **R**'s three packages; independently admit the bytes and read the Rational literal nodes and their semantic types; read the model document included in each handoff | `Ratio` and `Exact` are unit-less Rational declarations with numerator/denominator bounds -1..1/1..2 and -9223372036854775808..9223372036854775807/1..9223372036854775807 respectively; both bounded types are reached; the two extreme rationals are literal values typed by the emitted bounded `Exact` type, with no bound widened |
| 13 | 13 | write each **R** package and read the model, the `U` node, the `targetFull` and `targetPartial` capture binders and the `sum` nodes of `Healthy` and of the `recover` conditions | `Amount` and `Total` are aliases of `ix://quire/unit/U` with their bounds; the capture binders are typed by the node of `U`; each sum is `quire.op.collection.sum.quantity` |
| 14 | 14 | with the reader admitting a slot-local binder, read the `recover` conditions of `Full` and `Partial` | `Full`: receipt sum equals the captured target; `Partial`: receipt sum positive and below it |
| 15 | 15 | run `emit_checked_v2` with `--clock-selections` holding each **R** package's clock configurations; then with an empty object, a repeated name, an unknown `kind`, an undefined member and a missing member; then a clause name no clause or template has | the bytes `emit` returns for the equal `Request`; each malformed file a refusal naming it and the unknown name an `UnknownClause` refusal, no bytes written |
| 16 | 16 | current state, over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U` declaring `U`: a source whose `pre` is `amount >= 1 U`; then `sum<Cash>` with `type Cash = Quantity<U>;` | both written and admitted; the sum's node is `quire.op.collection.sum.quantity` with mode `rounding` `exact` and `type_argument` the node of `U` |
| 17 | 17 | `Inventory::read` over a written file; over the refusal vectors of FR-517 "The inventory file" (`observed_count` 3 for a two-row file, format `/2`, row kind `criterion`, `completeness` `incomplete`, `start` 10.0, a second `owner`, empty input) | rows equal the in-process outcome's; `Malformed`, `Format` naming `/2`, `Malformed` for each of the others; no value |
| 18 | 18 | delivered model-unit emission: over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`, write a source that reads no unit-typed member | the package holds the `dimension` and `unit` nodes; the reader admits it |
| 19 | 19 | `sum<M::Total>` over the receipts and `capture targetFull: M::Total`; read the selected model's `Amount` and `Total` member bounds and test their endpoints and one value outside each | both names resolve without `UnresolvedTypeName`; the capture checks and the sum lowers to `quire.op.collection.sum.quantity`; alias-bound identities and values are retained, endpoints satisfy and outside values refuse, and Amount's bounds do not replace Total's |
| 20 | 20 | `sum<N>` for `N` an integer, rational, decimal, float and quantity type, evaluated over two summands, over an empty source and over summands whose total leaves `N`'s domain; a summand of another family; `Healthy`'s sum over `Quantity<U>` of **R** once **R**'s model carries the unit (FR-517-AC-13), evaluated over receipts of 2 U and 3 U | each family's total is the exact sum of the summands in `N`'s family (`5 m` for `2 m` and `3 m`), the empty sum is the family's zero, a total outside `N`'s domain is `Undefined(SumOutOfDomain)`, a summand of another family is `IllTyped(TypeMismatch)` at the summand; the node lowers to the family's `quire.op.collection.sum` identity; **R**'s clause checks and evaluates to 5 U |
| 21 | 21 | write TC-1070 fixture E, then the `unit.native` of the inventory vector, and read each `equality.json`; write each twice | fixture E: bytes equal the FR-531 encoding of the in-process rows, three lists non-empty; `unit.native`: three empty lists; two writes byte-identical |
| 22 | 22 | a unit whose equality rows name a node the package does not hold; the written file | `Error::Unnameable`, nothing written; the file holds no source text, digest or unlisted member |
| 23 | 23 | compile a unit with a selected source record and nested opaque member types; write and read `identities.json`, then compare its source-record rows to the same final emission | the required `/2` source-record rows retain original opaque keys, final node/type references, names and order and join through native IR; the file rewrites byte-identically |

Tag the test `#[trace("FR-517-AC-1", ..., "FR-517-AC-23", "TC-1312")]`; row 3 and 4 also carry FR-042-AC-10.

## Expected Results

Each row's Expected column.
