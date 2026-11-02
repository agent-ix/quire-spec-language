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

[FR-042](FR-042-publish-compiled-protocol-artifacts.md) criteria AC-1 to AC-15
are retired as historical, and the legacy emitter, reader and handoff tests
(`native_*_emission`, `domain_protocol_emission`, `native_numeric_domain_emission`,
`handoff_writer`, `compiled_protocol_v2`) are tagged to them. No successor test
is tagged to any FR-042 criterion: the `qsl-package` and `qsl-eval` protocol
tests are tagged FR-383, FR-390, TC-142 and TC-1061 to TC-1063. This requirement
publishes the mapping from each FR-042 criterion to its successor criterion and
test, names every GAP, and stages the deletion of each legacy file by the ticket
whose closure unblocks it. A GAP is a spec or test ticket, never a deletion.

## Behavior

QSL SHALL delete a legacy protocol-artifact test file only when each criterion
it backs, in the table below, has a successor row with no GAP, and the file's
staging gate has closed. Successor locations are `file:line` of the tag on
current main.

### FR-042 criterion to successor

| FR-042-AC | Successor criteria | Successor tests | Status |
| --- | --- | --- | --- |
| AC-1 selections, inventory, artifact identity | FR-383-AC-1, AC-2, AC-14, AC-15 | `qsl-package/src/emit/tests/owner_model.rs:114`, `protocol_requirements.rs:64`, `protocol_controls.rs:494` | partial; GAP-1: registered media/schema/encoding selections have no protocol-level criterion beyond the checked-package envelope (FR-264) |
| AC-2 canonical bytes, order, numbers | FR-383-AC-12, AC-15; FR-497, FR-322 numeric lowering | `protocol_graph.rs:439`, `protocol_controls.rs:966` | partial; GAP-2: signed-64 extrema and normalized rationals in protocol value fields have no protocol-tagged test |
| AC-3 one-axis selection sensitivity | FR-383-AC-14, AC-15 | `owner_model.rs:213`, `protocol_controls.rs:494` | partial; GAP-3: producer revision, accepted baseline and wire-selection axes have no v2 analogue; owner to confirm v2 content identity covers them |
| AC-4 typed values, owners, query forms, wrong-type refusal | FR-383-AC-11, AC-17; FR-390-AC-6, AC-17, AC-18, AC-23 | `protocol_requirements.rs:625`, `protocol_requirements.rs:680`, `qsl-eval/tests/it/protocol_slots/acceptance.rs:900`, `owner_model.rs:165` | covered; FR-383-AC-11 has no tag |
| AC-5 control topology, choice partition, bounded progress | FR-383-AC-6, AC-7; FR-218 (AC-30 proof budget); TC-1172 | `protocol_controls.rs:1088`, `protocol_controls.rs:375`, `protocol_controls.rs:692` | covered structurally; GAP-5: byte and read-back equivalence for received, own-attempt, domain-event choices has no per-test replacement |
| AC-6 binding subjects, 1/2/1 delivery counts, repeat attempts, compensation registration | FR-383-AC-8, AC-9, AC-16; FR-384; FR-381; FR-390-AC-1, AC-2 | `protocol_compensation.rs:145`, `protocol_controls.rs:453`, `protocol_controls.rs:875`, `qsl-eval/tests/it/protocol_slots.rs:530` | open: FR-390 ACs under QSL-852; delivery counts are consumer binding (FR-383-AC-16, FR-384) |
| AC-7 public reader, canonical bytes, mutation refusal | FR-383-AC-19, AC-20, AC-21 | `qsl-package/src/checked_v2/tests/protocol_corpus.rs:8`, `protocol_controls.rs:238`, `protocol_compensation.rs:91` | covered for the I2 reader; the legacy reader goes with B9 (QSL-838) |
| AC-8 partial, unsupported, incomplete retain independent results | FR-383-AC-18, AC-27 | `owner_model.rs:342` | GAP-8: the spine refuses the whole unit on one S3 refusal (FR-383-AC-18); retaining an admitted subject beside an unsupported projection has no successor. Owner ruling needed |
| AC-9 work-limit vectors | FR-383-AC-21, AC-25; FR-390-AC-9, AC-10; FR-218-AC-30 | `large_cycle.rs:7`, `qsl-eval/tests/it/protocol_slots.rs:711`, `acceptance.rs:1307` | partial; GAP-9: emit-side source, dependency, table, reference and canonical-output dimensions have no per-dimension zero/exact/one-short test |
| AC-10 handoff consumed by quire-protocol | FR-383-AC-19; QSL-839 consumer repoint | none in this repository | GAP-10: no test proves the quire-protocol consumer reads the checked package (IT-001) |
| AC-11 domain-package identity round trip | FR-056-AC-9; FR-383-AC-23, AC-26 | `owner_model.rs:119`, `owner_model.rs:209` | covered |
| AC-12 explicit null domain package | FR-383-AC-23 (no `model` dependency, empty replication) | `owner_model.rs:119` | covered structurally; the nullable wire member retires with the wire |
| AC-13 population and closure pairs, domain operation export | FR-380; FR-383-AC-4 | `qsl-package/src/protocol_activation.rs:14`, `protocol_graph.rs:178` | partial; GAP-13: no test reads back one pair per covering declaration |
| AC-14 domain parameters and `result` binders | FR-383-AC-10; FR-390-AC-1, AC-23 | `protocol_controls.rs:453`, `acceptance.rs:380` | covered |
| AC-15 `write_v1`/`write_v2` handoff directory | none: the checked package is the handoff | none | GAP-15: owner to confirm the checked-package v2 read by I2 replaces the directory and `expected.json` |

### Deletion staging per legacy file

| Legacy file or group | Deleted when | Blocker tickets |
| --- | --- | --- |
| `native_compensation_emission.rs`, `native_refund_emission.rs`, `native_payment_retry_emission.rs`, `native_split_shipment_emission.rs` (AC-6) | FR-390-AC-1, AC-2, AC-9, AC-10 green; the timed `/2` test stays until QSL-1414 closes | QSL-852, QSL-1414 |
| `native_choice_*`, `native_query_*`, `native_population_*`, `native_mixed_*`, `native_composite_*`, `native_domain_event_*`, `native_owned_attempt_progress_*` emission tests (AC-4, AC-5, AC-7, AC-9) | each test has a named equivalent that asserts bytes and read-back | spec and test ticket for GAP-5, GAP-9 |
| `native_numeric_domain_emission.rs`, `domain_protocol_emission.rs` (AC-2, AC-11 to AC-14) | GAP-2 and GAP-13 closed | spec and test ticket |
| `native/temporal_v2.rs`, `temporal_v3.rs`, `compiled_protocol_v2.rs` temporal half, `checked_temporal*` successors | FR-050 on the spine | QSL-1384, QSL-1446, QSL-1414 |
| `handoff/writer.rs`, `handoff.rs`, the two examples, `examples/protocol-handoff`, committed snapshots, `tests/it/handoff_writer.rs` (AC-10, AC-15) | GAP-10 and GAP-15 closed | consumer repoint under QSL-839; spec ruling |
| `native/mod.rs`, `controls.rs`, `channels.rs`, `families*`, `populations.rs`, `values.rs`, `types.rs`, `layout.rs`, `context.rs`, `metadata.rs`, `runtime*` | every group above deleted | all of the above |
| B9 readers and `tests/it/protocol_artifact.rs` | QSL-838 | out of QSL-840 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-500-AC-1 | Each FR-042 criterion AC-1 to AC-15 appears in the map with a successor row or a named GAP. | Inspection (TC-1294) |
| FR-500-AC-2 | Every successor test named in the map exists at the stated `file:line` and carries the successor criterion tag. | Test (TC-1294) |
| FR-500-AC-3 | No legacy file in the staging table is deleted while a GAP its criteria name is open. | Test (TC-1294) |

## Dependencies

- [FR-499](FR-499-delete-each-retired-family-only-behind-its-named-successor.md),
  [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md),
  [FR-390](FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md).
