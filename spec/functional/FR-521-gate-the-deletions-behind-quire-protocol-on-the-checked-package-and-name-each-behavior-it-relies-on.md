---
id: FR-521
title: "Gate the deletions behind quire-protocol on the checked package and name each behavior it relies on"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-500
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-518
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-511
    type: references
---
# FR-521: Gate the deletions behind quire-protocol on the checked package and name each behavior it relies on

## Description

QSL-1501 blocks QSL-840 (the B8 sink and handoff writer), QSL-838 (the B9
`protocol_artifact` remainder) and QSL-858 (`src/temporal`). FR-499 says "KEEP
until QSL-1501" without saying what has to be true when QSL-1501 is done. This
requirement states it, and gives each behavior quire-protocol uses (every
`use quire_spec_language::...` item) one row: a successor requirement with its
test case, or an explicit keep-until-green condition. There is no permanent keep
and no row reads "retired" without a green successor (FR-499).

## Behavior

### The gate

A change that deletes a carrier named in the table below SHALL NOT merge until
all of the following hold on its head:

1. The row's successor test is green and tagged with the legacy criterion tags
   the row names (so `quire matrix` lists none untagged; FR-493-AC-5).
2. quire-protocol's default branch has merged QSL-1549: it pins QSL at a
   revision that contains the successor, its
   `Cargo.toml` has no `handoff-writer` feature, `cargo check --locked
   --all-targets` exits 0, and its IT-001, IT-002, TC-003 and TC-011 tests pass
   on the checked-package route.
3. The row's own condition, where it states one.

No table, hash or allowlist of rows is kept in code: the gate is the tags and the
consumer's build.

### Rows

| # | Behavior quire-protocol relies on | Successor | Condition for deleting the carrier | Deleting ticket |
| --- | --- | --- | --- | --- |
| 1 | `checked_predicate`, `temporal_subject` (declaration, leaf, predicate leaves, package binding) | FR-515, TC-1310 | gate | QSL-838, QSL-840 |
| 2 | `handoff::{write_v1, write_v2}`, `expected.json`, `expected-v2.json`, `mutations/manifest.json`, the five `PUBLISHED_*` file constants (`PUBLISHED_V1_OFFER_FILE`, `PUBLISHED_V1_SELECTION_FILE`, `PUBLISHED_OFFER_FILE`, `PUBLISHED_SELECTION_FILE`, `PUBLISHED_MUTATION_MANIFEST_FILE`), `MUTATION_MANIFEST_FORMAT`, feature `handoff-writer`, the two examples, `examples/protocol-handoff` | FR-517, FR-518, TC-1312, TC-1313; FR-042-AC-10 and AC-15 | gate; the ported recipe **R** is green | QSL-840 |
| 3 | `native_temporal` request/result v1 and v2 (already deleted by QSL-303): `Limits`, `EvidenceRef`, `ActivationState`, `Truth`, `Settlement` | FR-516, TC-1311 | the successor must be green before QSL-1549 merges, since quire-protocol cannot compile without it | QSL-858 |
| 4 | The four contract strings `quire.native-temporal-request/v1`, `/v2`, `quire.native-temporal-result/v1`, `/v2` in quire-protocol's producer-contract registry | FR-516 (no document, no contract id; the thirteen work counters and the opaque `EvidenceRef` members carry the retired `Limits` and digest fields) | the registry rows are removed by QSL-1549, not by QSL | none in QSL |
| 5 | `temporal::{Closure, Completeness, Execution, Evidence, Position, OrderKey}`, `Classification`, `mapping_support_v2`, `evaluate_v2`, `Trace`, `Report` | FR-516 types for QSL-1549; Contract IR FR-350 / IR-1005 for the bridge | KEEP the public surface (FR-499, QSL-858 ruling 2) until both the QSL-1549 repoint and IR-1005 are merged; the pin-forward break `Position.instance_valuations` is fixed in the consumer | QSL-858 |
| 6 | `state::{EvaluationReport, StateView, evaluate_v2, ACCOUNTING_VERSION}` (already deleted by QSL-834) | FR-520, TC-1315 | the successor is green before QSL-1549 merges | none (deleted) |
| 7 | `protocol_artifact::read`, `v2::read`, `v3::read`, `Expected`, `ExpectedTemporal`, `AdmittedPackage`, `ActivationSelection`, `activation_mappings`, `encode_candidate`, `Candidate` | FR-515, FR-517 (`Selection::check`, determinism AC-5), FR-449 rows, FR-513-AC-10, FR-513-AC-11 | gate; FR-050-AC-5 canonical-bytes determinism is carried by FR-517-AC-5 and FR-518-AC-6 | QSL-838 |
| 8 | `wire::*` records | FR-519 record-to-node map | gate; the `BindingKind::Window` spelling is kept per FR-513 until QSL-1506 | QSL-838 |
| 9 | `ProtocolNumber`, `NumberWire`, `NumberComponent`, `NumberError`, `ExactInteger` | FR-511 (root module `number`) | the `protocol_artifact::ProtocolNumber` re-export is removed after QSL-1549; `number` stays until QSL-1506's port is green | QSL-838 |
| 10 | `NodeRole`, `OccurrenceKey`, occurrence-key schema | FR-384 (specified, no code) | PORT QSL-1506; `occurrence.rs` stays until FR-384's code and tests are green | QSL-838 |
| 11 | `Limits`, `Dimension`, `Exhaustion` (`work.rs`); `Limits` gained an `intake` field | Each of the 13 dimensions has a successor: `payload_bytes` is `identity.input_bytes` and `i2.input_bytes` on `CheckedPackageReadLimits` (FR-513-AC-10); `source_bytes` is `s1.input_bytes`; `entries` and `references` are `i2.nodes`, `i2.occurrences` and `i2.edges`; `sources`, `definitions`, `models` and `declarations` are graph nodes and occurrences bounded by `i2.nodes` and `i2.occurrences`; `dependencies` are graph edges bounded by `i2.edges`; `output_bytes` has no emit-side limit on the retained path (`EMIT_LIMITS` is `u64::MAX`) and is carried by QSL-1782 (caller-owned emit-output ceiling); `content_bytes`, `byte_work` and `depth` are the recursion and cumulative-byte limits of FR-383-AC-24 and AC-25. Each needs a zero, exact and one-short test (QSL-1510); the `i2.edges` and `i2.diagnostics` fixtures are not otherwise valid packages, so their exact case proves only that the ceiling does not stop the read, not that the package verifies | PARTIAL for `output_bytes` only: every other dimension is tested at zero, exact and one-short on its retained ceiling (`checked_v2/tests/read_limit_successors.rs`, `emit/tests/inventory_ceilings.rs`, the `s3.*` and `s1.input_bytes` tests); `output_bytes` stays open until a caller-owned emit-output ceiling exists, and `work.rs` stays until then | QSL-838 |
| 12 | `Error`, `Invalid`, `Unsupported`, `Report`, the `v2.*` refusal code strings (matched by tc_011 and `handoff::reported_code`) | Contract IR's `CheckedPackageRefusalCode` and the FR-518 table codes | the `v2.*` vocabulary stays until QSL-1506's port is green and quire-protocol's tc_011 matches IR codes (FR-513 row for FR-050-AC-2, AC-3) | QSL-838 |
| 13 | `qsl_foundation::ByteDigest`, `.digest()` of admitted packages, `ArtifactRef.digest`/`revision`, checksum and seal verifying lanes of IT-001 | FR-518 rows 12, 29 and 28m (the checksum and seal lanes' behavior); FR-516 (opaque `EvidenceRef` digests); an admitted package is identified by its canonical bytes (FR-056-AC-9), not a content digest (QSL-1270) | gate; row 28m needs the Contract IR source-byte integrity requirement (FR-518); QSL-1549 deletes those lanes after (owner ruling pending; this is the default) | none in QSL |
| 14 | The legacy manifest format identity `quire.protocol.v2-mutations/1` | `quire.checked-handoff.mutations/1` (FR-518) | quire-protocol stops naming it in QSL-1549 | QSL-840 |
| 15 | Format identity constants `quire.compiled-protocol/1` and `/2` | `quire.checked-package/v2` (Contract IR) | quire-protocol stops naming them in QSL-1549 | QSL-838 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-521-AC-1 | Each of rows 1 to 15 names a successor requirement and test case that exist (`quire validate` resolves the ids), or states a condition and its owner, and every item of quire-protocol's `use quire_spec_language::...` lines is named by some row. | Inspection (TC-1316) |
| FR-521-AC-2 | After the successors of rows 1, 2, 3, 6, 7, 8 are merged, `quire matrix` over QSL lists none of FR-042-AC-10, FR-042-AC-15, FR-052-AC-9 to AC-14, FR-053-AC-7 to AC-9, FR-050-AC-5, FR-513-AC-10, FR-513-AC-11 untagged; deleting a file that carries the only tag of one of them makes the matrix test fail. | Test (TC-1316) |
| FR-521-AC-3 | The consumer's gate, owned by QSL-1549 in quire-protocol: its default branch pins a QSL revision that contains the successor, has no `handoff-writer` feature, and `cargo check --locked --all-targets` and its tests exit 0. QSL runs no build of that repository; a deleting change cites the merged QSL-1549 state. | Inspection (TC-1316) |
| FR-521-AC-4 | FR-499's rows for the handoff writer, the format identity constants and the strict readers read "KEEP until FR-521 row 2 is green", "row 15" and "row 7" respectively and name no other release condition (the gate states QSL-1549); FR-500's AC-10 and AC-15 rows begin "Successor: FR-517, FR-518" with the disposition Test (TC-1312, TC-1313). | Inspection (TC-1316) |
| FR-521-AC-5 | Rows 11 and 13 state the successor of every `work.rs` dimension and of the digest and seal lanes, and no change deletes `work.rs` or removes the integrity lanes while a successor test of those rows is not green. | Inspection (TC-1316) |

## Dependencies

- **Upstream:** FR-499, FR-500, FR-513, FR-493-AC-5, QSL-1501, QSL-1549.
