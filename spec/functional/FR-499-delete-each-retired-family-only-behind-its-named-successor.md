---
id: FR-499
title: "Delete each retired native and composed family only behind its named successor"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-033
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-500
    type: references
---
# FR-499: Delete each retired native and composed family only behind its named successor

## Description

[FR-493](FR-493-retire-the-native-package-runtime-front-end-and-protocol-artifact-surfaces.md)
retires the native package, runtime-input and front-end paths and states the
rule for the rest: a retired path is deleted in the change that makes its last
caller unreachable, never before its replacement works, and a retired
acceptance criterion is removed or re-owned. This requirement applies that rule
to the five families that remain after FR-493: the composed `Protocol` family,
the protocol-artifact remainder, `src/temporal`, `src/native_model` and the
composed-lane remainder. For each it states what is retired, which retained
requirement and test covers each retired behavior, and the order of deletion.
The behavior of a retired path is never dropped: a behavior with no successor
below is a **GAP**, and a GAP blocks the deletion until a requirement and a test
own it.

A successor counts as green only when its test runs and passes on the head of
the deleting change and is tagged with the successor requirement's acceptance
criterion. A legacy test is deleted with its path only after the row that names
it shows a green successor.

## Order of deletion

1. Gate closes: QSL-852 (FR-390-AC-1, AC-2, AC-4, AC-5, AC-9, AC-10), QSL-1414
   (FR-048-AC-8 authenticated timed selection), QSL-1384 and QSL-1446
   (authenticated temporal on the spine, FR-050).
2. **S1** QSL-840: the B8 sink (`protocol_artifact::native`, handoff writer,
   examples, committed snapshots), staged per file by [FR-500](FR-500-map-each-fr-042-criterion-to-its-checked-package-successor.md).
3. **S2** QSL-841: the composed `Protocol` family. It follows S1 because B8
   matches `DeclarationKind::Protocol`.
4. **B6** QSL-838 and **T3b** QSL-858 land as one change, because each imports
   the other: `src/temporal` imports `protocol_artifact::wire` and
   `ProtocolNumber`, and `protocol_artifact::v2::intake` imports
   `crate::temporal` (`profile.rs` constants, B9). They follow S1. The change
   may instead be split if its first part moves the `intake` constants off
   `crate::temporal`; the split changes no row below.
5. **NM** QSL-499: `src/native_model`, in the change that removes its last
   importer (ADR-016 §9). Its importers are the `0-draft` front end (QSL-837,
   FR-493-AC-3), B8 and B9 (steps 2 and 4), `src/temporal` (step 4) and the
   composed lane (step 6). If the composed `models.rs` and `solver.rs` still
   import it, NM and step 6 land as one change.
6. **S3** QSL-830 and QSL-868: the composed-lane remainder. One change satisfies
   both tickets: QSL-868 owns FR-362-AC-6 and QSL-830 owns TC-1027 steps 2 and 3.
7. QSL-316 and the epic QSL-67 close when their last child lands.

## Behavior

- QSL SHALL NOT delete a path named in a table below until every row of that
  path shows a green successor and no row is a GAP.
- When a deleting change removes a path, it SHALL remove each requirement and
  test case that only describes that path and re-own, in the same change, each
  acceptance criterion that a deleted test alone backed, by tagging the
  successor test named in the row.
- When a deleting change removes the last composed or native family, it SHALL
  delete the interim wording that keeps those families normative (FR-362 items 8
  and 9, ADR-033, FR-493), and the by-absence tests of TC-1027 and TC-1276 SHALL
  pass.

## QSL-841 (S2): the composed `Protocol` family

Retired: `DeclarationKind::Protocol`; `parser/composed/protocol.rs`; the protocol
types of `syntax/composed.rs`; `linking/composed/scopes/protocol.rs` and
`protocol/flow.rs`; the protocol arms of `scopes.rs`, `scopes/values.rs`,
`definitions.rs`, `definition_source.rs` (`RegisteredDefinition::Protocol`),
`dependencies.rs`, `models.rs`, `requests.rs` (`Family::Protocol`),
`checking/composed.rs`, `solver/roots.rs`, `proofs/correspondence.rs` and
`proofs/engine/roots.rs`; the composed protocol tests and
`tests/fixtures/composed-choreography.native`.

| Retired behavior | Successor requirement | Status |
| --- | --- | --- |
| Parse of `protocol` (FR-035-AC-1 protocol declaration) | FR-308 (S2 forms), `qsl-forms` protocol clause | green |
| Scoped anchors, binder no-shadowing, attempt-to-frame binding | FR-112, FR-113, FR-114, FR-305 | green |
| Every control, channel, relationship, requirement, capture, activation-each, role and related-by check (FR-048-AC-2 to AC-5, AC-7) | FR-218 (all S3 control constructs), FR-381 (captures, activation, compensation, recovery), FR-211 (terminal states) | green (QSL-558, QSL-855) |
| Choice-partition and independent-atom proof budgets (old TC-133 rows) | FR-218-AC-30, TC-1172 | green |
| Protocol transition system and footprints | FR-215, FR-216 | green |
| Emission of the choreography (FR-048-AC-1, AC-9) | FR-383 | green (QSL-851) |
| Protocol-node expressions, captures, compensation and recovery evaluation (FR-048-AC-6, AC-7; TC-134, TC-142) | FR-390 | open: QSL-852 |
| Authenticated timed `/2` selection (FR-048-AC-8) | no checked-route producer yet | GAP: QSL-1414 |
| Composed protocol parts of FR-035, FR-036, FR-040 | retire with the family; their lane stays normative for other families (FR-362 item 9) | amend in the change |

## QSL-838 (B6) and QSL-858 (T3b): the protocol-artifact remainder and `src/temporal`

Retired by QSL-838: `protocol_artifact::{read, v2::read, v3::read, wire,
validate, intake, encoding, value_graph, work (ACCOUNTING_VERSION)}` and their
tests, including `tests/it/protocol_artifact.rs`. Retired by QSL-858:
`src/temporal.rs` and `src/temporal/{formula, trace, result, activation,
mapping, profile, progress, budget, protocol}.rs`, `native/temporal_v2.rs` and
`temporal_v3.rs`, and their FR-043 to FR-045 tests.

| Retired behavior | Successor requirement | Status |
| --- | --- | --- |
| Strict reading, selected-metadata reconstruction, mutation refusal of protocol bytes (FR-042-AC-7) | FR-383-AC-19, AC-20, AC-21 over the I2 reader (FR-264); FR-493-AC-4 | green; map in FR-500 |
| Bounded temporal truth, operators, intervals (FR-043-AC-1 to AC-6) | FR-325, FR-326, FR-327, FR-328 | green |
| Lasso and fair-lasso evaluation, including `Lasso::Model` over a model subject | FR-329 (Behavior for `Lasso::Model`, AC for the model half), FR-125, FR-120 | the `Lasso::Model` half SHALL be tested green on the spine before deletion (QSL-858 criterion) |
| Temporal work limits, effective-limit identity, 100,000-deep stack bound (`budget.rs`) | TC-124 (named temporal-limit successor), FR-327 charge rules, FR-255 `temporal.*` rows | green |
| Profile and clock selection, asserted-profile mismatch (FR-043-AC-2, AC-3) | FR-326, FR-110 | green |
| `on origin` and `on each` activation (FR-044-AC-8) | FR-327 | green |
| Trigger-keyed instances, duplicate delivery, immutable captures, guard dispositions (FR-044-AC-1 to AC-7, AC-9) | FR-052-AC-9 to AC-12 and FR-053-AC-7 to AC-9 (re-owned in place), FR-381, FR-449; successor tests TC-144, TC-149 | GAP: TC-144 and TC-149 are planned and not yet green |
| Closure, completeness and watermark dispositions (FR-043-AC-7, AC-10 to AC-12; FR-045 progress) | FR-327 and FR-328 outcomes (`Completed`, `Incomplete`, prefix) | GAP: watermark regression and conflicting completeness revision (FR-043-AC-12) have no spine criterion |
| Timestamped-event finite-window profile (FR-043-AC-2, AC-11) | none: the spine evaluator refuses it with `ProfileNotEvaluated` | GAP: spec ruling awaited (QSpec #272, FND-001) |
| Native-to-TL mapping classification (FR-045, `mapping.rs`) | none in QSL; consumed by Contract IR FR-350 | GAP: FR-045-AC-1 to AC-5 are re-homed unchanged onto the checked `temporal` clause node (FR-337); AC-6 retires with `/2` |
| Authenticated temporal bindings (FR-050) | FR-050 retained; QSL-1384 and QSL-1446 | GAP: not yet on the spine |

## QSL-499 (NM): `src/native_model`

Retired: `native_model.rs`, `native_model/{admission, artifact}.rs`
(`NativeModel`, `NativeModelProfile`, `ModelLimits`, `ScalarKind`, `ObjectRole`,
`OperationRole`, `ScalarSite`, `Unit`).

| Retired behavior | Successor requirement | Status |
| --- | --- | --- |
| Model objects, roles and operations (FR-041 description) | FR-056 (domain-package models), FR-084, FR-086, FR-120, FR-300, FR-303 | green |
| Scalar-site bound admission (FR-041-AC-2) | FR-056-AC-6 and FR-056-AC-9 bounded scalar sites at intake | re-own FR-041-AC-2 to FR-056 |
| Profile-bearing artifact identity (FR-041-AC-6) | FR-110 (profile by identity) | re-own to FR-110 |
| Limits at zero, exact, one-short (FR-041-AC-7) | FR-056-AC-6 (`normalize.record` bound) | re-own to FR-056 |
| Composed `bind_models` (FR-041-AC-5) | FR-300, FR-303 | retires with the composed lane |
| ConfigVersion example fixtures | spine corpus (FR-105, FR-108, TC-469) | green |

## QSL-830 and QSL-868: the composed-lane remainder

Retired: `src/linking/composed`, `src/checking/composed`, `src/parser/composed*`,
`src/syntax/composed*`, `formal_source`, and every caller of `parse_native` and
`parse_native_source`, after steps 2 to 5.

| Retired behavior | Successor requirement | Status |
| --- | --- | --- |
| Parse of every family (FR-035) | FR-362 item 1 one grammar; per-family S2 forms (FR-102, FR-308, FR-313, FR-325) | green per family |
| Link and request report (FR-036, `requests`) | FR-111, FR-110; `requests` backend disposition leaves QSL (ADR-011 SEAM-2, #185) | green |
| Check of values, state, sum, temporal, protocol, relation (FR-040) | S3 family checkers (FR-104, FR-316 to FR-318, FR-326, FR-218, FR-172) | green per family |
| Evaluation inputs (FR-049) | FR-106, FR-107 | green |
| A profile line with a version or digest is admitted by composed `parse_native` | FR-362-AC-2 by absence: no parser admits it | closes on deletion |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-499-AC-1 | For every path a deleting change removes, each row of this requirement's table for that path names a successor whose tagged test passes on the change's head; a row marked GAP blocks the deletion. | Test (TC-1293) |
| FR-499-AC-2 | After QSL-841 lands, a downstream crate cannot name `DeclarationKind::Protocol`, `Family::Protocol` or `RegisteredDefinition::Protocol`, and a `protocol` unit through the spine still refuses unsupported content with `unsupported_construct`. | Test (TC-1293) |
| FR-499-AC-3 | After QSL-838 and QSL-858 land, no import of `crate::temporal`, `crate::state`, `protocol_artifact::wire` or `ProtocolNumber` remains, and the `Lasso::Model` half of FR-329 passes on the spine. | Test (TC-1293) |
| FR-499-AC-4 | After QSL-499 lands, the path `src/native_model` is absent and the ConfigVersion spine corpus (TC-469) passes. | Test (TC-1293) |
| FR-499-AC-5 | After QSL-830 and QSL-868 land, TC-1027 steps 2 and 3 pass, no parser admits a profile line with a version or digest, and the interim wording of FR-362 items 8 and 9 is gone. | Test (TC-1027, TC-1293) |
| FR-499-AC-6 | `quire matrix` after each deleting change lists no acceptance criterion of a retained requirement left without a tagged test because its test went with a retired path. | Test (TC-1276 step 5) |

## Dependencies

- [FR-493](FR-493-retire-the-native-package-runtime-front-end-and-protocol-artifact-surfaces.md)
  states the deletion rule; [FR-500](FR-500-map-each-fr-042-criterion-to-its-checked-package-successor.md)
  maps FR-042 criteria to successors.
- ADR-011 §7.3 states the lanes M-6c to M-6e and the QSL-840 coverage table;
  ADR-016 §9 owns `native_model`'s last-importer rule.
