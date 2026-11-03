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
  - target: ix://agent-ix/quire-spec-language/FR-525
    type: references
---
# FR-499: Delete each retired native and composed family only behind its named successor

## Description

[FR-493](FR-493-retire-the-native-package-runtime-front-end-and-protocol-artifact-surfaces.md)
states the deletion rule: a retired path is deleted in the change that makes its
last caller unreachable, never before its replacement works, and a retired
acceptance criterion is removed or re-owned. This requirement applies it to the
families that remain: the composed `Protocol` family, the protocol-artifact
sink and readers, `src/temporal`, `src/native_model` and the composed-lane
remainder. `native-compile/1` is not a deletion family and stays (FR-027).

Every behavior of a path to be deleted has exactly one disposition:

- **COVERED**: a retained requirement and a tagged test exist (the test is named
  by file and tag site; it counts as green only when it passes on the head of the
  deleting change).
- **PARTIAL**: a successor exists but misses a stated part; the missing part is
  named and owned by a ticket.
- **PORT**: no successor exists; the behavior moves to the spine under the named
  ticket, which blocks the deletion ticket.
- **KEEP**: the code or surface stays until the named consumer repoints.
- **RETIRE**: the behavior ends with its retired carrier. Allowed only with a planner or owner ruling cited in the row; never for a behavior with a live consumer.

A row SHALL NOT read "retired" without a named COVERED successor test. A
behavior with no successor is never removed; the planner ruling on each such row
is PORT or KEEP. A bare id without a test is never evidence. The inventory
behind these tables is the code-derived deletion table (326 rows) and the
planner's rulings on its ten decisions; this requirement records the outcome,
and the tickets named below own each PORT.

## Order of deletion

1. Gates. QSL-852 (FR-390 partial; see FR-500). QSL-1501 (quire-protocol reads a
   checked package; blocks QSL-840, QSL-838, QSL-858). QSL-1414 and
   QSL-1384/QSL-1446 (timed `/2` selection and authenticated temporal; the
   producer `emit_checked_with_temporal_selections` exists, the
   compensation-window clock of FR-050-AC-13/14 is untested). QSL-1510 (missing
   successor tags). The PORT tickets listed per family.
2. QSL-838 and QSL-858 land together, in one change. `src/temporal` imports
   `protocol_artifact::{v2, v3, wire, AdmittedPackage, ProtocolNumber}`, and
   `protocol_artifact::v2::intake` imports `crate::temporal` (the three
   profile-identity constants). The change may be split only if its first part
   moves those constants off `crate::temporal`.
3. QSL-840 (B8 sink and handoff writer) lands with or before that change and
   owns `native/temporal_v2.rs` and `temporal_v3.rs`, the handoff writer, the
   examples, committed snapshots and the `handoff-writer` feature. QSL-838 owns
   `decode, domain, encoding, intake, models, occurrence, recovery,
   validate, value_graph, wire, work, v2/*, v3/*`. QSL-858 owns `src/temporal*`. `number.rs` is owned by none of them: FR-511 moves it to the root module `number` until the QSL-1506 port is green.
   No file has two owners.
4. QSL-841 lands after QSL-840: six `native/*` files still match
   `DeclarationKind::Protocol`.
5. QSL-868 and QSL-830 land together with or after QSL-837 (the `0-draft` front
   end, which the composed lane imports) and after `formal_source` is kept and
   `qsl_inventory` is moved (QSL-1508, QSL-1509).
6. QSL-499 lands last, in the change that removes the last importer of
   `native_model` (ADR-016 §9).
7. QSL-316 and the epic QSL-67 close when their last child lands.

## Behavior

- QSL SHALL NOT delete a path until every row of its table is COVERED, or is
  PARTIAL/PORT with its owning ticket closed and the row updated to COVERED, or
  is KEEP with the file kept.
- When a deleting change removes a path, it SHALL remove each requirement and
  test case that only describes that path, re-own each acceptance criterion a
  deleted test alone backed by tagging the successor test named in the row, and
  edit the names listed in FR-500 §Names that break.
- When the last composed or native family is deleted, the interim wording that
  keeps those families normative (FR-362 items 8 and 9, ADR-033, FR-493) SHALL
  be deleted in the same change.

## QSL-858: `src/temporal`

Public surface kept (ruling 2) until IR repoints (IR-1005) and quire-protocol
repoints (QSL-1501): `temporal::{mapping_support_v2, Classification,
AuthenticatedSelection, Trace, Report, evaluate_v2}` and
`Closure, Completeness, Execution, Evidence, Position, OrderKey`. The
quire-protocol conformance kernel (`src/conformance/kernel.rs:1198-1271`) uses
them at its pinned revision.

| Behavior | Successor requirement and test | Disposition |
| --- | --- | --- |
| Operators, intervals, bounded truth (FR-043-AC-1, AC-4, AC-6) | FR-325 to FR-328; `qsl-eval/tests/it/temporal_trace.rs` | COVERED (FR-327-AC-4 and FR-329-AC-5 visit-meter tests at `temporal_trace.rs:659,938`) |
| Lasso and fair lasso incl. `Lasso::Model` (FR-329) | FR-329 | PARTIAL: green `Lasso::Model` test required before deletion (QSL-858 criterion); QSL-1516 |
| `on origin` / `on each` activation (FR-044-AC-8) | FR-327 | COVERED |
| Profile and clock selected by identity (FR-043-AC-2) | FR-110, FR-326 | PARTIAL: header selection only; wrong-clock assertion is PORT QSL-1502 (FR-522) |
| Trace clock/profile mismatch refusal (FR-043-AC-3), result identity (AC-13), authenticated clock report, whole-declaration vs per-instance stop | none (`TraceInput` carries documents only); successors FR-522, FR-523 | PORT QSL-1502 |
| Open input applies neither false extension nor empty truth (AC-8), history cutoff (AC-9), four progress axes (AC-10), watermark settlement (AC-11), regressing watermark and conflicting completeness (AC-12) | FR-502 (watermark and completeness), FR-504 (event-position and fixed-sample progress), FR-501; map in FR-525 | PORT QSL-1503 (AC-12 also FR-502) |
| `timestamped-event.finite-window/v1` (FR-043-AC-2, AC-11) | FR-501 (spec PR #2131); today `qsl-eval/src/temporal/engine.rs:184` returns `ProfileNotEvaluated` | PORT (FR-501; implementation under the QSL-858 train) |
| Trigger-keyed instances, duplicate delivery, conflicting payload, capture faults, four-way trigger-scope disposition, `ProtocolLedger`, opaque trigger identity (FR-044-AC-1 to AC-7, AC-9; FR-052-AC-9 to AC-12; FR-053-AC-7 to AC-9; TC-122, TC-123, TC-125, TC-144, TC-149, TC-1277 to TC-1279) | FR-524 (instances, captures, dispositions), FR-526 (lineage); open rows OPEN-5 to OPEN-7 in FR-525; the existing `checked_temporal_successors.rs`, `checked_temporal_limits_v2.rs` and the TC-144/149 test exercise `src/temporal` itself and are not successors | PORT QSL-1503 |
| FR-045 mapping classifier incl. AC-6 authenticated selection, operator reachability | FR-527 (reads the FR-337 node through FR-515's view) | KEEP public surface; PORT QSL-1504 (AC-1 to AC-6) |
| Temporal work limits (`budget.rs`, NFR-008) | no spine test is tagged TC-124 or NFR-008-AC; the only TC-124 tests call `src/temporal` | PARTIAL: PORT QSL-1502 (FR-523); TC-124 is not a green successor |
| `native/temporal_v2.rs`, `temporal_v3.rs` producers (FR-050) | emit side `emit_checked_with_temporal_selections` (`qsl-package/src/emit.rs:3413`), `tc134_*` (`fixture_w.rs:967,1071,1114`) | PARTIAL: owned by QSL-840; FR-050-AC-13/14 untested (QSL-1414, STD-644, IR-1262) |

## QSL-838 and QSL-840: `protocol_artifact`

The per-criterion map of FR-042 is FR-500. Rows for the rest of the module:

| Behavior | Successor | Disposition |
| --- | --- | --- |
| `read`, `v2::read`, `v3::read` strict reader and refusal of mutated bytes (FR-042-AC-7) | FR-383-AC-19, AC-20 over the I2 reader (FR-264) | COVERED for the checked package only; the 25 legacy `/1` per-axis mutation tests have no analogue. KEEP until FR-521 row 7 is green (FR-515, FR-517, FR-518) |
| v2 refusal vocabulary `v2.header.*`, `v2.binding.*` (FR-050-AC-2, AC-3); `v2::encode_candidate` byte determinism (FR-050-AC-5) | none | PORT QSL-1506 (quire-protocol `tc_011` matches the code strings) |
| `ProtocolNumber`, `NUMERIC_PROFILE="quire.protocol.numeric/1"` codec (FR-038, TC-117) | none (FR-497 and FR-322 cover carried quantities, not this codec); FR-511 states the disposition | PORT QSL-1506; KEEP `number.rs` as the root module `number` (FR-511) until that port is green |
| Occurrence-key schema, `NodeRole`, nested repeat ordinals (FR-048-AC-1, FR-384, `occurrence.rs`, TC-132, TC-1064) | FR-384 specified; no code or test carries FR-384 | PORT QSL-1506 |
| `ArtifactKind`/`BindingKind` wire spellings (FR-049-AC-2, AC-5; TC-136) | none | PORT QSL-1506 |
| `/3` authored control-temporal map (FR-054-AC-1 to AC-4) | FR-449, TC-1194 | PARTIAL: one legacy test (`compiled_protocol_v2.rs:1313`) tags AC-1 to AC-4; AC-2 to AC-4 PORT QSL-1506 |
| Format identity constants `quire.compiled-protocol/1`, `/2`, media, schema strings | `quire.checked-package/v2` (Contract IR) | KEEP until FR-521 row 15 is green |
| Handoff writer `write_v1`, `write_v2`, `PUBLISHED_*` files, `expected.json`, `expected-v2.json`, `mutations/manifest.json`, the two examples, `examples/protocol-handoff`, feature `handoff-writer`, committed snapshots, writer errors | FR-517, FR-518 (TC-1312, TC-1313): the checked-package writer, selection sidecar and mutation manifest | KEEP until FR-521 row 2 is green; the writer is already its successor, so the legacy one is then deleted, not replaced |
| `intake.rs` independent exact inventories (FR-042-AC-3, FR-050) | I2 evidence map | PARTIAL; one-axis rows in FR-500 |
| Existing red test `tests/it/protocol_artifact.rs:777` (`Limits` default mismatch) | none | fix in QSL-838 before any deletion |

## QSL-841: the composed `Protocol` family

`DeclarationKind::Protocol` exists only at `src/syntax/composed.rs:106,141`; the
enums in `qsl-forms/src/syntax.rs` and `qsl-semantics/src/model/admitted.rs` have
no such variant and are retained. The composed AST holds 19 protocol types.

| Behavior | Successor requirement and test | Disposition |
| --- | --- | --- |
| Parse of `protocol` (FR-035-AC-1) | FR-308; `qsl-forms` protocol clause | COVERED (tag site to be listed by QSL-1510) |
| Scoped anchors, frame binding | FR-112, FR-113, FR-114, FR-305 | COVERED |
| Control, channel, relationship, requirement, capture, activation, role checks | FR-218 (`coverage.rs:15-62` accepts all kinds), FR-381, FR-211; `protocol_controls.rs:1091` (FR-218-AC-34) | COVERED/PARTIAL; FR-218-AC-10 and FR-381-AC-13 appear only in doc comments (QSL-1510) |
| Choice-partition proof budget | FR-218-AC-30, TC-1172 | COVERED (QSL-855) |
| Emission of the choreography | FR-383 | PARTIAL: FR-383-AC-11, 15, 16, 18, 21, 22, 23 have no tagged test (QSL-1510) |
| Protocol-node expression evaluation | FR-390; tests in FR-500 | PARTIAL (QSL-852) |
| Channel `delivery [lo,hi]` numeric interval (B8 emitted it) | none | PORT QSL-1507 |
| Capability-to-family applicability, `InapplicableCapability`, twelve-kind table (FR-057-AC-11, `requests.rs`) | none | PORT QSL-1507 |
| Use-site profile-kind admission (`WrongProfileKind`; `protocol P using <value profile>` accepted today) | none | PORT QSL-1507 |
| `ObservationBinding` closure requires `Protocol` | none | PORT QSL-1507 |
| Composed proof discharge for a protocol (`correspondence.rs:226`, `proofs/engine/roots.rs`) | none, untested | PORT QSL-1507 (state the observable, then delete) |
| FR-048-AC-4, AC-9 observed guards, independent compile | FR-218, FR-383 | PARTIAL QSL-1507 |
| FR-048-AC-8 authenticated timed `/2` | `emit_checked_with_temporal_selections` | PARTIAL (QSL-1414) |

## QSL-499: `src/native_model`

Importers span `linking`, `checking`, `model_source`, `protocol_artifact`,
`temporal`, `linking/composed/models.rs` and `checking/composed/*`; one external
consumer holds `NativeModel` (quire-protocol `AdmittedModel::schema_model`).

| Behavior | Successor | Disposition |
| --- | --- | --- |
| Objects, roles, operations (FR-041 description) | FR-056, FR-084, FR-086, FR-120, FR-300, FR-303 | PARTIAL |
| Bounded exact-rational scalar sites, consistent bounds across sites (FR-041-AC-2) | none: FR-056 states `Rational` has no bounds on the spine; FR-056-AC-6 is the `normalize.record` charge | PORT QSL-1505 |
| `/1` versus `/2` profile-bearing identity (FR-041-AC-6) and the `native-state-model/1,2` strings | none; FR-110 is header selection | RETIRE (planner decision 5: the identity strings die with the retired carrier); no successor claimed |
| Roles, entries, nodes, depth, artifact-byte limits and their refusals (FR-041-AC-7, FR-015) | analogue for one counter only | PORT QSL-1505 unless the spine has an equivalent limit for the same resource |
| FR-015-AC-1 to AC-4, AC-6 (TC-040 to TC-045), `link_native` conflicts | none | PORT/disposition QSL-1505 |
| Function exclusion, integer overflow semantics gate, sequence ceiling 10_000 | none | PORT QSL-1505 (ADR-014 B-6 evidence cites the ceiling) |

## QSL-868 and QSL-830: the composed-lane remainder

| Behavior | Successor | Disposition |
| --- | --- | --- |
| Per-family parse, link, check, evaluate inputs (FR-035, FR-036, FR-040, FR-049) | S2/S3/S6a family requirements | PARTIAL per family; FR-040 and FR-036 rows carry the retained FR-046/056/057 criteria whose only tests go with the lane (QSL-1509) |
| Multi-unit namespace (`admit_namespace`, `SourceInventory`, `SyntaxNamespace`; FR-036-AC-1, AC-2, AC-7; FR-480) | none: FR-480 unimplemented | PORT QSL-1508 |
| `checking::source_inventory::qsl_inventory` (IR FR-052 producer) | FR-362 item 10 says it lives in `qsl_replay`; in code it does not | PORT QSL-1508 |
| `StaticSubject` static-meaning identity (FR-036-AC-5) | none | PORT QSL-1509 |
| `formal_source` coordinate bridge (FR-014, TC-035 to TC-039); used by the retained compile/run command | none | KEEP; re-tag (QSL-1509) |
| CLI `quire-spec parse` with its flags, output document, exits 20, 21, 22, 30 | `qsl_cst::parse` | PORT QSL-837 (rewire; keep flags, output and exits) |
| FR-362-AC-2 (no parser admits a profile line with a version or digest) | already true in code (`ModelImport` has alias, package and span); no executable composed-parser test | add by-absence test (QSL-1510) |
| FR-362-AC-6, TC-1027 step 2 | no automated check | QSL-1510 |

## Names that break (edited in the deleting change)

`xtask/src/seam_probe.rs:189-192`; `tests/it/formatter_identity.rs:1129`,
`composed_linking.rs:44`, `composed_syntax.rs:643,847`,
`tests/fixtures/composed-choreography.native`; `README.md:166,176`;
`tests/it/clause_kind_canonical.rs:101-126` (constant 6 becomes 5);
`Makefile:276-290`, `Cargo.toml:46,139-154`, `xtask/src/ci_clean_build_tests.rs`,
`ci_feature_lane_tests.rs`, `typestate_scan.rs`, `string_edge.rs:378`;
`docs/compiled-protocol-v1.md`, `compiled-protocol-v2.md`,
`native-temporal-evaluation.md`, `native-model-checking.md`,
`formal-source-binding.md`, `native-error-codes.md`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-499-AC-1 | For every path a deleting change removes, each row of that path's table is COVERED with a passing tagged test, or its PORT ticket is closed, or the file is KEPT; no row reads retired without a named COVERED test or a cited planner or owner ruling. | Test (TC-1293) |
| FR-499-AC-2 | After QSL-841, a downstream crate cannot name `DeclarationKind::Protocol` (`src/syntax/composed.rs`), `Family::Protocol` or `RegisteredDefinition::Protocol`, and a `protocol` unit through the spine behaves as FR-383 and FR-218 state. | Test (TC-1293) |
| FR-499-AC-3 | After QSL-838 and QSL-858 land together, no import of `crate::temporal`, `protocol_artifact::wire` or `ProtocolNumber` (unless `number.rs` is KEPT) remains, and a green `Lasso::Model` test exists on the spine. | Test (TC-1293) |
| FR-499-AC-4 | After QSL-499, `src/native_model` is absent and TC-469 passes. | Test (TC-1293) |
| FR-499-AC-5 | After QSL-868 and QSL-830, TC-1027 steps 2 and 3 pass, the CLI `parse` runs on `qsl_cst::parse` with its flags and exits unchanged, and the interim FR-362 items 8 and 9 wording is gone. | Test (TC-1027, TC-1293) |
| FR-499-AC-6 | `quire matrix` after each deleting change lists no criterion of a retained requirement untagged because its test went with a retired path. | Test (TC-1276 step 5) |
| FR-499-AC-7 | The handoff writer, its examples, snapshots and feature remain until FR-521 row 2 is green. | Inspection |
| FR-499-AC-8 | No file of `src/temporal` is deleted while a PORT row of FR-525 names a successor criterion that no passing spine test carries. | Inspection |

## Dependencies

- [FR-493](FR-493-retire-the-native-package-runtime-front-end-and-protocol-artifact-surfaces.md)
  states the deletion rule; [FR-500](FR-500-map-each-fr-042-criterion-to-its-checked-package-successor.md)
  maps FR-042 criteria to successors.
- ADR-011 §7.3 and ADR-016 §9; tickets QSL-1501 to QSL-1511.
