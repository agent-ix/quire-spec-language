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
requirement states it, and gives each behavior quire-protocol uses at pin
`9395be4` (30 files, 167 references) one row: a successor requirement with its
test case, or an explicit keep-until-green condition. There is no permanent keep
and no row reads "retired" without a green successor (FR-499).

## Behavior

### The gate

A change that deletes a carrier named in the table below SHALL NOT merge until
all of the following hold on its head:

1. The row's successor test is green and tagged with the legacy criterion tags
   the row names (so `quire matrix` lists none untagged; FR-493-AC-5).
2. quire-protocol's default branch has merged QSL-1549: it pins QSL `main`
   (a branch or a rev that contains the successor, not `9395be4`), its
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
| 2 | `handoff::{write_v1, write_v2}`, `expected.json`, `expected-v2.json`, `mutations/manifest.json`, the six `PUBLISHED_*` file constants, `MUTATION_MANIFEST_FORMAT`, feature `handoff-writer`, the two examples, `examples/protocol-handoff` | FR-517, FR-518, TC-1312, TC-1313; FR-042-AC-10 and AC-15 | gate; the ported recipe **R** is green | QSL-840 |
| 3 | `native_temporal` request/result v1 and v2 (already deleted by QSL-303): `Limits`, `EvidenceRef`, `ActivationState`, `Truth`, `Settlement` | FR-516, TC-1311 | the successor must be green before QSL-1549 merges, since quire-protocol cannot compile without it | QSL-858 |
| 4 | The four contract strings `quire.native-temporal-request/v1`, `/v2`, `quire.native-temporal-result/v1`, `/v2` in quire-protocol's producer-contract registry | FR-516 (no document, no contract id) | the registry rows are removed by QSL-1549, not by QSL | none in QSL |
| 5 | `temporal::{Closure, Completeness, Execution, Evidence, Position, OrderKey}`, `Classification`, `mapping_support_v2`, `evaluate_v2`, `Trace`, `Report` (kernel.rs:1198-1271) | FR-516 types for QSL-1549; Contract IR FR-350 / IR-1005 for the bridge | KEEP the public surface (FR-499, QSL-858 ruling 2) until both the QSL-1549 repoint and IR-1005 are merged; the pin-forward break `Position.instance_valuations` is fixed in the consumer | QSL-858 |
| 6 | `state::{EvaluationReport, StateView, evaluate_v2, ACCOUNTING_VERSION}` (already deleted by QSL-834) | FR-520, TC-1315 | the successor is green before QSL-1549 merges | none (deleted) |
| 7 | `protocol_artifact::read`, `v2::read`, `v3::read`, `Expected`, `ExpectedTemporal`, `AdmittedPackage`, `ActivationSelection`, `activation_mappings`, `encode_candidate`, `Candidate` | FR-515, FR-517 (`Selection::check`, determinism AC-5), FR-449 rows, FR-513-AC-10, FR-513-AC-11 | gate; FR-050-AC-5 canonical-bytes determinism is carried by FR-517-AC-5 and FR-518-AC-6 | QSL-838 |
| 8 | `wire::*` records (66 references) | FR-519 record-to-node map | gate; the `BindingKind::Window` spelling is kept per FR-513 until QSL-1506 | QSL-838 |
| 9 | `ProtocolNumber`, `NumberWire`, `NumberComponent`, `NumberError`, `ExactInteger` | FR-511 (root module `number`) | the `protocol_artifact::ProtocolNumber` re-export is removed after QSL-1549; `number` stays until QSL-1506's port is green | QSL-838 |
| 10 | `NodeRole`, `OccurrenceKey`, occurrence-key schema | FR-384 (specified, no code) | PORT QSL-1506; `occurrence.rs` stays until FR-384's code and tests are green | QSL-838 |
| 11 | `Limits`, `Dimension`, `Exhaustion` (`work.rs`); `Limits` gained an `intake` field | `CheckedPackageReadLimits`, `qsl-package` emit limits; FR-513-AC-10 | PARTIAL: the 13-dimension contract is not mirrored; the decision is listed for ruling (PR body) and `work.rs` stays until it is made | QSL-838 |
| 12 | `Error`, `Invalid`, `Unsupported`, `Report`, the `v2.*` refusal code strings (matched by tc_011 and `handoff::reported_code`) | Contract IR's `CheckedPackageRefusalCode` and the FR-518 table codes | the `v2.*` vocabulary stays until QSL-1506's port is green and quire-protocol's tc_011 matches IR codes (FR-513 row for FR-050-AC-2, AC-3) | QSL-838 |
| 13 | `qsl_foundation::ByteDigest`, `.digest()` of admitted packages, `ArtifactRef.digest`/`revision`, checksum and seal verifying lanes of IT-001 (`it001_v2_handoff.rs:990-1073`) | none: digests and seals were removed by QSL-1270; FR-518 row 28 | QSL-1549 deletes those lanes; listed for ruling (PR body) | none in QSL |
| 14 | `handoff::PUBLISHED_HANDOFF`, `PUBLISHED_V1_HANDOFF` (pinned-source compare) | FR-517-AC-5 (determinism) and FR-518-AC-8 (`CORPUS_CASES`) | the constants are already gone from QSL main | none (deleted) |
| 15 | Format identity constants `quire.compiled-protocol/1` and `/2` | `quire.checked-package/v2` (Contract IR) | quire-protocol stops naming them in QSL-1549 | QSL-838 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-521-AC-1 | Each of rows 1 to 15 names a successor requirement and test case that exist (`quire validate` resolves the ids), or states a condition and its owner, and every item of quire-protocol's `use quire_spec_language::...` lines is named by some row. | Inspection (TC-1316) |
| FR-521-AC-2 | After the successors of rows 1, 2, 3, 6, 7, 8 are merged, `quire matrix` over QSL lists none of FR-042-AC-10, FR-042-AC-15, FR-052-AC-9 to AC-14, FR-053-AC-7 to AC-9, FR-050-AC-5, FR-513-AC-10, FR-513-AC-11 untagged; deleting a file that carries the only tag of one of them makes the matrix test fail. | Test (TC-1316) |
| FR-521-AC-3 | On the head of any change that deletes a row's carrier, quire-protocol built at its default branch pins a QSL revision that contains the change's parent, has no `handoff-writer` feature, and `cargo check --locked --all-targets` and its tests are green; with `main` of QSL as the dependency the 21 lib and 54 lib-test errors recorded on QSL-1501 are zero. | Test (TC-1316) |
| FR-521-AC-4 | FR-499's rows for the handoff writer, the format identity constants and the strict readers read "KEEP until FR-521 rows 2, 15 and 7 are green" and name no other release condition; FR-500's AC-10 and AC-15 rows read "Successor: FR-517, FR-518" with the dispositions Test (TC-1312, TC-1313). | Inspection (TC-1316) |
| FR-521-AC-5 | Rows 11 and 13 list the decisions awaiting a ruling and their stated defaults, and no change deletes `work.rs` or removes the integrity lanes while the ruling is open. | Inspection (TC-1316) |

## Dependencies

- **Upstream:** FR-499, FR-500, FR-513, FR-493-AC-5, QSL-1501, QSL-1549.
