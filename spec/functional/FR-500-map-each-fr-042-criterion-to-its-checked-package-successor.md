---
id: FR-500
title: "Map each FR-042 criterion to its checked-package successor"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
---
# FR-500: Map each FR-042 criterion to its checked-package successor

## Description

[FR-042](FR-042-publish-compiled-protocol-artifacts.md) AC-1 to AC-15 are marked
retired as historical, and the legacy emitter, reader and handoff tests
(`native_*_emission`, `domain_protocol_emission`, `native_numeric_domain_emission`,
`handoff_writer`, `compiled_protocol_v2`) are tagged to them. No successor test
carries an FR-042 tag; the `qsl-package` and `qsl-eval` protocol tests are tagged
FR-383, FR-390, TC-142 and TC-1061 to TC-1063. This requirement maps each
criterion to its successor criterion and test, says where a criterion has no
tagged test, and stages each legacy file. A gap is a ticket, never a deletion.
Locations are the `#[trace(...)]` attribute lines on current main and are
re-verified by TC-1294.

Successor criteria with **no tagged test** on main: FR-383-AC-11, AC-15, AC-16,
AC-18, AC-21, AC-22, AC-23; FR-390-AC-13, AC-15, AC-16, AC-18 to AC-22, AC-24
to AC-27; all of FR-384. A row that depends on one of these is PARTIAL (QSL-1510).

## FR-042 criterion to successor

Paths are under `qsl-package/src/emit/tests/` (QP) and
`qsl-eval/tests/it/protocol_slots` (EV).

| FR-042-AC | Successor and tagged test | Disposition |
| --- | --- | --- |
| AC-1 selections, inventory, identity reference | FR-383-AC-1 (`QP/owner_model.rs:119`), AC-2 (`QP/protocol_requirements.rs:161`), AC-14 (`QP/protocol_controls.rs:496`, `owner_model.rs:213`) | PARTIAL: registered media/schema/encoding selections and complete original inventory have no protocol-level criterion beyond FR-264; QSL-1506 |
| AC-2 canonical bytes, number vectors | FR-383-AC-12 (`QP/protocol_controls.rs:966`); `qsl-eval/tests/it/footprints.rs:1277` is tagged TC-589 only and names FR-383-AC-12 in prose, so it is not a successor; `QP/protocol_graph.rs:439` is TC-1062 only and carries no FR-383-AC-12 or AC-15; AC-15 has no tagged test | PARTIAL: signed-64 extrema and normalized rationals in value fields and the FR-038 codec are PORT QSL-1506 |
| AC-3 one-axis selection sensitivity | FR-383-AC-14 (`QP/protocol_controls.rs:496`) | PARTIAL: producer revision, baseline, wire-selection and dependency axes have no v2 analogue; the 25 legacy per-axis tests have no successor; PORT QSL-1506 |
| AC-4 typed values, owners, query forms, wrong-type refusal | FR-383-AC-17 (`QP/protocol_requirements.rs:625,680`: missing-name refusal, not wrong-type); FR-390-AC-6 (`EV/acceptance.rs:900,2094`), AC-17 (`QP/owner_model.rs:165`), AC-23 (`EV/acceptance.rs:380`); FR-383-AC-11 untagged; FR-390-AC-18 untagged | PARTIAL |
| AC-5 control topology, choice partition, bounded progress | FR-383-AC-6 (no tag at the cited site), AC-7 (`QP/protocol_controls.rs:375,692`); FR-218-AC-34 (`QP/protocol_controls.rs:1091`); TC-1172 | PARTIAL: structural only; byte and read-back equivalence for received, own-attempt and domain-event choices has no successor; QSL-1510 |
| AC-6 binding subjects, 1/2/1 counts, repeat attempts, compensation | FR-383-AC-8 (`QP/protocol_compensation.rs:145`), AC-9 (`QP/protocol_controls.rs:453,875`), AC-16 untagged; FR-384 untagged and unimplemented; FR-390-AC-1 (`EV/acceptance.rs:1067,1133,1682`), AC-2 (`EV/protocol_slots.rs:530`) | PARTIAL: occurrence schema and delivery counts are PORT QSL-1506; FR-390 under QSL-852 |
| AC-7 reader verifies canonical bytes, mutation refusal | FR-383-AC-19 and AC-20 (`QP/protocol_compensation.rs:91`, `QP/protocol_controls.rs:521,543,573`, `qsl-package/src/checked_v2/tests/protocol_corpus.rs:8`); AC-21 untagged | PARTIAL for the new wire; legacy `/1` reader KEEP until QSL-1501 |
| AC-8 partial, unsupported, incomplete keep independent results | FR-042-AC-8 is retired as historical; per-protocol independence is kept by FR-383-AC-18, AC-28, AC-29 and TC-1297 (spec PR #2131); the unsupported owner case is FR-383-AC-27 (`QP/owner_model.rs:342`) | PARTIAL until #2131 merges and its tests are tagged |
| AC-9 work-limit vectors per dimension | FR-383-AC-24 (`QP/protocol_recursion.rs:237`), AC-25 (`QP/large_cycle.rs:7`, `protocol_recursion.rs:395`); FR-390-AC-9 (`EV/protocol_slots.rs:711`, `EV/acceptance.rs:1307,1448`), AC-10 (`EV/acceptance.rs:1495`); AC-21 untagged | PARTIAL: emit-side source, dependency, table, reference and canonical-output dimensions have no per-dimension zero/exact/one-short test; QSL-1510 |
| AC-10 handoff consumed by quire-protocol (IT-001) | none: quire-protocol runs the writer in process and never reads a checked package | KEEP until QSL-1501 |
| AC-11 domain-package identity round trip | FR-056-AC-9; FR-383-AC-26 (`QP/owner_model.rs:213,233,381`); AC-23 untagged | PARTIAL |
| AC-12 explicit null domain package | FR-383-AC-23 untagged; `owner_model.rs:119` is tagged AC-1 and AC-26 | PARTIAL: QSL-1510 |
| AC-13 population and closure pairs, operation export, `Unsupported::Export` | FR-380; `qsl-package/src/protocol_activation.rs:14` is source with no tests; `QP/protocol_graph.rs:178` tests channel premises | PORT: pair read-back has no test (QSL-1510) |
| AC-14 domain parameter and `result` binders | FR-383-AC-10 (`QP/protocol_controls.rs:457`), FR-390-AC-1, AC-23 | PARTIAL |
| AC-15 `write_v1`, `write_v2` handoff directory | none | KEEP until QSL-1501 |

## Deletion staging per legacy file

| Legacy file or group | Deleted when | Owner |
| --- | --- | --- |
| `native_compensation_emission.rs`, `native_refund_emission.rs`, `native_payment_retry_emission.rs`, `native_split_shipment_emission.rs` | FR-390-AC-1, AC-2, AC-9, AC-10 green and QSL-1414 closed; the timed `/2` test stays until then | QSL-840 |
| `native_choice_*`, `native_query_*`, `native_population_*`, `native_mixed_*`, `native_composite_*`, `native_domain_event_*`, `native_owned_attempt_progress_*` | each has a named byte and read-back equivalent | QSL-840 after QSL-1510 |
| `native_numeric_domain_emission.rs`, `domain_protocol_emission.rs` | AC-2, AC-13 rows COVERED | QSL-840 after QSL-1506, QSL-1510 |
| `native/temporal_v2.rs`, `temporal_v3.rs`, `compiled_protocol_v2.rs` temporal half, `checked_temporal_limits_v2.rs` (mounted at `compiled_protocol_v2.rs:1235-1236`) | FR-050 on the spine and a producer replaces `with_v2_inputs` | QSL-840 with QSL-858 |
| `handoff/writer.rs`, `handoff.rs`, both examples, `examples/protocol-handoff`, snapshots, `tests/it/handoff_writer.rs`, feature `handoff-writer` | QSL-1501 closed; replaced by a checked-package writer | QSL-840 |
| `native/*` emitters (`mod, controls, channels, families*, populations, values, types, layout, context, metadata, runtime*`) | all groups above | QSL-840 |
| `decode, domain, encoding, intake, models, number, occurrence, recovery, validate, value_graph, wire, work, v2/*, v3/*` and `tests/it/protocol_artifact.rs` | FR-499 QSL-838 table | QSL-838 with QSL-858 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-500-AC-1 | Each of FR-042-AC-1 to AC-15 has a row with a successor test at a stated location or a named PARTIAL, PORT or KEEP disposition. | Inspection (TC-1294) |
| FR-500-AC-2 | Every successor location stated names a `#[trace(...)]` attribute that carries the stated criterion on the head of the change. | Test (TC-1294) |
| FR-500-AC-3 | No legacy file in the staging table is deleted while its owner's gate is open. | Test (TC-1294) |

## Dependencies

- [FR-499](FR-499-delete-each-retired-family-only-behind-its-named-successor.md),
  [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md),
  [FR-390](FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md).
