---
id: FR-518
title: "Write the adverse mutation corpus from checked packages and name the destination of every legacy case"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-500
    type: references
---
# FR-518: Write the adverse mutation corpus from checked packages and name the destination of every legacy case

## Description

The legacy v2 handoff writer produced `mutations/manifest.json`
(`quire.protocol.v2-mutations/1`, cases of identity, axis, input and expected
refusal code) and 28 cases at the quire-protocol pin (27 on QSL main after the
digest removal, QSL-1270). quire-protocol's IT-001 submits every case to its
reader and asserts the declared refusal (`tests/it001_v2_handoff.rs`,
`DECLARED_CASES = 28`). Nothing in the checked-package path produces them.
Moving them is the QSL-1501 gate for GAP-10 and GAP-15: **no case is dropped
silently**. Each is moved to a checked-package mutation with a named refusal,
or stated as not movable with the concept it tested and the ruling it needs.

## Inputs

The fixture **R** of FR-517 and `corpus = Standard`.

## Outputs

`mutations/manifest.json`, format `quire.checked-handoff.mutations/1`:
`{ format, base_package: "checked-package.json", selection: "selection.json",
cases: [ { identity, axis, input: { file }, stage, expected } ] }`, and one
`mutations/<identity>.json` per case holding the mutated package bytes. `stage`
is `reader` (refused by `quire_contract_model::read_checked_package`) or
`selection` (admitted by the reader and refused by `Selection::check`).
`expected` is, for `reader`, a code, an optional cause and a JSON pointer; for
`selection`, a `SelectionAxis` (`clause.name`, `clause.requirement`,
`clause.node`, `clause.activation`, `definition.identity`, `definition.revision`,
`definition.authority`, `clock.sequence_authority`, `clock.epoch`,
`clock.period`, `clock.unit`, `clock.timestamp_unit`).

## Behavior

### The corpus table

Stage `reader` codes and loci are the ones Contract IR's reader gives today
(`temporal_binding.rs` `validate`, FR-038 of Contract IR); `selection` axes are
the ones in the sidecar. The expected values are written in the QSL writer's
source, never derived from the reader's output in the same run.

| # | Legacy identity (axis) | Legacy expected | Stage and mutation of **R** | Expected |
| --- | --- | --- | --- | --- |
| 1 | `header-wire-version-substitution` (`header.version`) | `v2.header.*` | reader: replace `contract_version` by `quire.checked-package/v1` | `unknown_contract_version` |
| 2 | `binding-missing` (`temporal_bindings.inventory`) | `v2.binding.*` | reader: delete a clause's row | `invalid_package`/`timed-node-reference` at that clause node's `/semantic_graph/nodes/{N}/body` |
| 3 | `binding-surplus` (`temporal_bindings.inventory`) | `v2.binding.*` | reader: append a row whose declaration is an id the graph does not hold | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/declaration` |
| 4 | `binding-duplicate` (`temporal_bindings.uniqueness`) | `v2.binding.*` | reader: repeat a row | `invalid_package`/`timed-node-reference` at the second `/temporal_bindings/{i}` |
| 5 | `binding-reordered` (`temporal_bindings.order`) | `v2.binding.*` | reader: swap two rows | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/declaration` of the row out of `NodeRef` order |
| 6 | `declaration-index` (`temporal_binding.declaration`) | `v2.binding.*` | reader: a row's declaration becomes the id of a non-clause node | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/declaration` |
| 7 | `definition-index` (`temporal_binding.definition`) | `v2.binding.*` | reader: remove the lock `profile_selections` row of one clause's `temporal_profile` law | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/clock/kind` |
| 8 | `declaration-name` (`declaration.name`) | `v2.declaration.*` | selection: a different well-formed declared name for a clause | `clause.name` |
| 9 | `declaration-requirement` (`declaration.requirement`) | `v2.declaration.*` | selection: the protocol `requirements` entry that names the clause points at another clause | `clause.requirement` |
| 10 | `declaration-clause` (`declaration.clause`) | `v2.declaration.*` | selection: the clause node id differs from the sidecar's | `clause.node` |
| 11 | `declaration-execution` (`declaration.execution`) | `v2.declaration.*` | selection: the activation form differs (`origin` for `each`) | `clause.activation` |
| 12 | `definition-identity` (`definition.identity`) | `v2.definition.*` | selection: the profile law identity of a clause differs | `definition.identity` |
| 13 | `definition-revision` (`definition.revision`) | `v2.definition.*` | selection: the profile law revision differs | `definition.revision` |
| 14 | `definition-artifact-digest` (`definition.artifact.digest`) | `v2.definition.*` | not movable: no digest member exists (QSL-1270); successor row 14b | see 14b |
| 14b | by absence: a `digest` member added to a `definition_selections` entry | none | reader: add the member | `malformed_wire` at the entry |
| 15 | `definition-artifact-digest-domain` (`definition.artifact.digest`) | `v2.definition.*` | not movable (as 14); successor row 15b | see 15b |
| 15b | by absence: a `digest_domain` member added to a `definition_selections` entry | none | reader: add the member | `malformed_wire` at the entry |
| 16 | `clock-alternative` (`clock.kind`) | `v2.clock.*` | reader: a row's clock alternative is another kind | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/clock/kind` |
| 17 | `clock-sequence-authority` (`clock.sequence_authority`) | `v2.clock.*` | selection: another non-empty authority on the event-position row | `clock.sequence_authority` |
| 18 | `clock-epoch` (`clock.epoch`) | `v2.clock.*` | selection: another well-formed epoch on the fixed-sample row | `clock.epoch` |
| 19 | `clock-period` (`clock.period`) | `v2.clock.*` | selection: another positive reduced period on the fixed-sample row | `clock.period` |
| 20 | `clock-unit` (`clock.unit`) | `v2.clock.*` | selection: another non-empty unit on the fixed-sample row | `clock.unit` |
| 21 | `clock-timestamp-unit` (`clock.timestamp_unit`) | `v2.clock.*` | selection: another non-empty unit on the timestamped row | `clock.timestamp_unit` |
| 22 | `clock-missing-sequence-authority` (`clock.sequence_authority`) | `json` | reader: remove the member | `malformed_wire` at the row's `clock` |
| 23 | `clock-renamed-sequence-authority` (`clock.sequence_authority`) | `json` | reader: rename it `sequenceAuthority` | `malformed_wire` at the row's `clock` |
| 24 | `clock-missing-period` (`clock.period`) | `json` | reader: remove `period` | `malformed_wire` at the row's `clock` |
| 25 | `clock-renamed-period` (`clock.period`) | `json` | reader: rename it `sample_period` | `malformed_wire` at the row's `clock` |
| 26 | `clock-missing-timestamp-unit` (`clock.timestamp_unit`) | `json` | reader: remove the member | `malformed_wire` at the row's `clock` |
| 27 | `clock-renamed-timestamp-unit` (`clock.timestamp_unit`) | `json` | reader: rename it `timestampUnit` | `malformed_wire` at the row's `clock` |
| 28 | `definition-original-bytes` (`definition.original_bytes`) | `invalid.seal` | not movable: no seal and no source-byte authentication exists in the package (`CheckedSourceRef` is "named by the producer, with no byte authentication"); ruling needed (default below) | none |
| 29 | `definition-artifact-wire-identity` (QSL main only) | `v2.definition.*` | selection: the lock definition authority differs | `definition.authority` |
| 30 | `clock-name` (`temporal_bindings.clock_name`, QSL main only) | `v2.clock.*` | reader: a row's `clock_name` differs from the clause's clock literal | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/clock_name` |
| 31 | by absence of a default: an empty `clock_name` (new) | none | reader: set `clock_name` to the empty text | `malformed_wire` at `/temporal_bindings/{i}/clock_name` |

Rows 1 to 28 are the 28 cases at the quire-protocol pin; rows 29 and 30 are the
two cases QSL main has that the pin does not, and rows 14, 15 and 28 are the
three the pin has that main does not (27 cases on main). The manifest holds 30
cases: rows 1 to 13, 16 to 27, 29 and 30 (the 27 of main) and the three new
rows 14b, 15b and 31. Row 28 has no case. Of the 30, 18 are stage `reader`
(rows 1 to 7, 14b, 15b, 16, 22 to 27, 30, 31) and 12 are stage `selection`
(rows 8 to 13, 17 to 21, 29).

### Writer self-check

Before writing, the writer submits each case to the stage it names and requires
the declared refusal, exactly: a case admitted by both stages, refused by the
other stage, or refused with another code or pointer returns
`Error::CaseMismatch { identity }` and writes nothing. If a stage's actual
refusal differs from this table because Contract IR gives the mutation an
earlier refusal, the table is amended with that code in the same change; the
case is not removed.

### Defaults stated for ruling

Rows 14, 15 and 28 tested a digest, a digest domain and a source-byte seal. The
checked package carries none of them by decision (QSL-1270), so the default is:
rows 14b and 15b assert the closed shape, and row 28 has no successor case,
recorded here and in the FR-499 row so the loss is visible, not silent. If Peter
rules that source bytes need an integrity check, that is a new requirement on the
checked package (Contract IR), not a QSL corpus case.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-518-AC-1 | `write(R, dir)` with `corpus = Standard` writes `mutations/manifest.json` of format `quire.checked-handoff.mutations/1` with 30 cases whose identities are exactly the set of the corpus table (rows 1 to 13, 14b, 15b, 16 to 27, 29, 30, 31), each identity unique and each file non-empty. | Test (TC-1313) |
| FR-518-AC-2 | Every case with stage `reader` is refused by `read_checked_package` with the code, cause and pointer of its table row, and no such case is admitted. | Test (TC-1313) |
| FR-518-AC-3 | Every case with stage `selection` is admitted by `read_checked_package` and refused by `Selection::check` with the `SelectionAxis` of its row, and the unmutated **R** passes both. | Test (TC-1313) |
| FR-518-AC-4 | The 28 legacy rows each appear in the table with a stage or the words "not movable"; a test lists the legacy identities from the pinned writer and fails when one has no row. | Test (TC-1313) |
| FR-518-AC-5 | Replacing the expected value of any one case in the writer's table by another value makes `write` return `Error::CaseMismatch` naming that identity; an injected admitted mutation (a case whose bytes equal the base) does the same. | Test (TC-1313) |
| FR-518-AC-6 | Two `write` calls with `Standard` give byte-identical manifests and case files, and the manifest lists the cases in table order. | Test (TC-1313) |
| FR-518-AC-7 | The manifest holds no case that expects a digest, seal or `v2.*` refusal string, and `mutations/` of a `None` corpus does not exist. | Test (TC-1313) |
| FR-518-AC-8 | quire-protocol's frozen constants are replaced, not kept: the case count 30 and the axis set of the table are the ones its IT-001 declares after QSL-1549; the QSL test exports them as `CORPUS_CASES` and `CORPUS_AXES` for a consumer to import. | Test (TC-1313) |

## Dependencies

- **Upstream:** FR-517, FR-513-AC-9 (emission-time selection refusals: the
  emission side of rows 8 to 13 and 17 to 21 is covered there), Contract IR
  FR-038 (temporal binding rows), QSL-1270.
- **Rulings:** rows 14, 15, 28 (above).
