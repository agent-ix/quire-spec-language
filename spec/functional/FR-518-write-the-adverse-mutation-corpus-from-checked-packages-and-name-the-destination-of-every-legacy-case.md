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
refusal code). The table below records every legacy identity. quire-protocol's
IT-001 submits every case to its reader and asserts the declared refusal
(`tests/it001_v2_handoff.rs`). Nothing in the checked-package path produces them.
Moving them is the QSL-1501 gate for GAP-10 and GAP-15: **no case is dropped
silently**. Each is moved to a named successor.

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
`clause.node`, `clause.activation`, `clause.binding`, `definition.identity`,
`definition.revision`, `definition.authority`, `clock.kind`, `clock.name`,
`clock.sequence_authority`, `clock.epoch`, `clock.period`, `clock.unit`,
`clock.timestamp_unit`, `package.nodes`). `clause.binding`, `clock.kind`,
`clock.name` and `package.nodes` are the axes `Selection::check` returns for a
difference that is not one of the others (a clause with another binding row, a
row of another clock kind or clock name, a package whose node set differs), so
an unrelated difference never reports as `clause.node`; no row of the table
mutates them.

## Behavior

### The corpus table

Stage `reader` codes and loci are the ones Contract IR's reader gives today
(`temporal_binding.rs` `validate`, FR-038 of Contract IR); `selection` axes are
the ones in the sidecar. The expected values are written by hand in the QSL
writer's source; the writer never copies them from the reader's output.

Verification status: rows 1 to 4, 8 to 13, 16 to 21, 29 and 30 restate codes
and axes the current Contract IR reader and `Selection::check` give. Rows 5 to 7,
the renamed-member rows 23, 25 and 27 and row 31 are
written from Contract IR's rules and are not yet given by the reader. The writer
self-check below verifies them; where it shows a code or pointer differs, the
author confirms the reason in Contract IR's reader source and amends this table
in the same change.

| # | Legacy identity (axis) | Legacy expected | Stage and mutation of **R** | Expected |
| --- | --- | --- | --- | --- |
| 1 | `header-wire-version-substitution` (`header.version`) | `v2.header.*` | reader: replace `contract_version` by `quire.checked-package/v1` | `unknown_contract_version` |
| 2 | `binding-missing` (`temporal_bindings.inventory`) | `v2.binding.*` | reader: delete a clause's row | `invalid_package`/`timed-node-reference` at that clause node's `/semantic_graph/nodes/{N}/body` |
| 3 | `binding-surplus` (`temporal_bindings.inventory`) | `v2.binding.*` | reader: append a row whose declaration is an id the graph does not hold | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/declaration` |
| 4 | `binding-duplicate` (`temporal_bindings.uniqueness`) | `v2.binding.*` | reader: repeat a row | `invalid_package`/`timed-node-reference` at the second `/temporal_bindings/{i}` |
| 5 | `binding-reordered` (`temporal_bindings.order`) | `v2.binding.*` | reader: swap two rows | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/declaration` of the row out of `NodeRef` order |
| 6 | `declaration-index` (`temporal_binding.declaration`) | `v2.binding.*` | reader: a row's declaration becomes the id of a non-clause node | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/declaration` |
| 7 | `definition-index` (`temporal_binding.definition`) | `v2.binding.*` | reader: remove the lock `profile_selections` row of one clause's `temporal_profile` law | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/clock/kind` |
| 8 | `declaration-name` (`declaration.name`) | `v2.declaration.*` | reader: add a `declaration` member to a `temporal_clause` node, which carries none; the reader refuses an unexpected declaration member, and the successor for a differing clause name is FR-518-AC-11 | `invalid_semantic_graph` (no cause) at `/semantic_graph/nodes/{N}/declaration` |
| 9 | `declaration-requirement` (`declaration.requirement`) | `v2.declaration.*` | selection: the protocol `requirements` entry that names the clause points at another clause | `clause.requirement` |
| 10 | `declaration-clause` (`declaration.clause`) | `v2.declaration.*` | selection: the clause node id differs from the sidecar's | `clause.node` |
| 11 | `declaration-execution` (`declaration.execution`) | `v2.declaration.*` | selection: the activation form differs (`origin` for `each`) | `clause.activation` |
| 12 | `definition-identity` (`definition.identity`) | `v2.definition.*` | selection: the profile law identity of a clause differs; the clauses, the lock `profile_selections` row and the `temporal_bindings` rows all move to another registered profile, because the reader refuses a package whose law and lock move without the binding rows (`invalid_package`/`timed-node-reference` at `/temporal_bindings/0/clock/kind`, or `malformed_wire` at `/temporal_bindings` under `infinite-trace`) | `definition.identity` |
| 13 | `definition-revision` (`definition.revision`) | `v2.definition.*` | reader: the profile law revision is another revision; Contract IR registers one revision per profile, so the reader refuses every other revision, and the successor for a revision difference between an admitted package and the sidecar is FR-518-AC-12 | `unknown_profile`/`unsupported-selection` at `/semantic_graph/nodes/{N}/body/operation/laws/0/definition` |
| 14 | `definition-artifact-digest` (`definition.artifact.digest`) | `v2.definition.*` | moved to row 12, which refuses a definition whose identity differs from the sidecar's; closed-shape successor 14b | see 12, 14b |
| 14b | by absence: a `digest` member added to a `definition_selections` entry | none | reader: add the member | `unknown_member` at `/lock/definition_selections/0/digest` |
| 15 | `definition-artifact-digest-domain` (`definition.artifact.digest`) | `v2.definition.*` | moved to row 29, which refuses a definition whose authority differs; closed-shape successor 15b | see 29, 15b |
| 15b | by absence: a `digest_domain` member added to a `definition_selections` entry | none | reader: add the member | `unknown_member` at `/lock/definition_selections/0/digest_domain` |
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
| 28 | `definition-original-bytes` (`definition.original_bytes`) | `invalid.seal` | dropped: the checked package vouches for no source bytes, so the case has no successor | none |
| 29 | `definition-artifact-wire-identity` (QSL main only) | `v2.definition.*` | selection: the lock definition authority differs | `definition.authority` |
| 30 | `clock-name` (`temporal_bindings.clock_name`, QSL main only) | `v2.clock.*` | reader: a row's `clock_name` differs from the clause's clock literal | `invalid_package`/`timed-node-reference` at `/temporal_bindings/{i}/clock_name` |
| 31 | by absence of a default: an empty `clock_name` (new) | none | reader: set `clock_name` to the empty text | `malformed_wire` at `/temporal_bindings/{i}/clock_name` |

Rows 1 to 28 are the legacy identities. Rows 29 and 30 are cases QSL main has
that the legacy list does not, and rows 14, 15 and 28 are legacy identities main
no longer has. The manifest holds a case for every row that names a mutation:
rows 1 to 13, 14b, 15b, 16 to 27, 29, 30 and 31 (30 cases). Of
these, 20 are stage `reader` (rows 1 to 8, 13, 14b, 15b, 16, 22 to 27, 30,
31) and 10 are stage `selection` (rows 9 to 12, 17 to 21, 29).

A `temporal_clause` node carries no declaration, so the checked package holds
no clause name, and Contract IR registers one revision per profile, so the
reader admits no package whose revision differs. `Selection::check` takes the
package the reader admits, so neither the `clause.name` axis nor a revision
difference between two admitted packages can be reached by mutating a package.
The `definition.revision` axis is reached by editing the sidecar's revision
against an admitted package (FR-518-AC-12). The `clause.name` axis is gated on
Contract IR IR-1278 (the checked package carries no temporal clause name):
FR-518-AC-11 is verified only with a reader that admits a declared name on a
temporal clause, and until then the axis is defined in `Selection::check` and
has no passing test.

### Writer self-check

Before writing, the writer submits each case to the stage it names and requires
the declared refusal, exactly: a case admitted by both stages, refused by the
other stage, or refused with another code or pointer returns
`Error::CaseMismatch { identity }` and writes nothing. If a stage's actual
refusal differs from this table because Contract IR gives the mutation an
earlier refusal, the table is amended with that code in the same change; the
case is not removed.

> **INTERIM.** This requirement is ahead of the code. The writer in
> `src/checked_handoff.rs` and `src/checked_handoff/corpus.rs` still carries the
> `source_integrity` reader check and writes the 28m case (31 cases), and
> `tests/it/checked_handoff.rs` still tests it. A coder change removes the
> emission and the 28m case so the code matches the 30 cases above; the
> source-integrity work of QSL-1559 and QSL-1560 is canceled.

### Moved cases for rows 14, 15 and 28

Rows 14, 15 and 28 tested a digest, a digest domain and a source-byte seal. The
checked package carries no content digest (QSL-1270) and no source-byte claim.
The behavior of rows 14 and 15, refusing a definition whose content claim is not
bound to its identity, is not dropped; row 28 tested a source-byte seal that no
longer exists, so it is dropped with no successor. Rows 14 and 15 move onto rows 12 and 29, whose selection axes `definition.identity`
and `definition.authority` (FR-517's `Selection::check`, FR-513-AC-9) refuse a
definition whose identity or authority is not the one the sidecar declares; a
separate case would repeat them, so none is added. Row 8 moves to the
reader refusal of its row and, for a differing clause name, to FR-518-AC-11,
which is gated on IR-1278; row 13 moves to the reader refusal of its row and to
FR-518-AC-12. Until IR-1278 lands the legacy property of row 8 (a clause name
that differs) has no passing test.
Rows 14b and 15b keep the closed-shape assertion.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-518-AC-1 | `write(R, dir)` with `corpus = Standard` writes `mutations/manifest.json` of format `quire.checked-handoff.mutations/1` whose cases are in table order, each with a unique identity, a stage of `reader` or `selection`, and a non-empty case file; the cases include 14b and 15b. | Test (TC-1313) |
| FR-518-AC-2 | Every case with stage `reader` is refused by `read_checked_package` with the code, cause and pointer of its table row, and no such case is admitted. | Test (TC-1313) |
| FR-518-AC-3 | Every case with stage `selection` is admitted by `read_checked_package` and refused by `Selection::check` with the `SelectionAxis` of its row, and the unmutated **R** passes both. | Test (TC-1313) |
| FR-518-AC-4 | Over **R**, the cases of rows 12 and 29 are admitted by `read_checked_package` and refused by `Selection::check` with `definition.identity` and `definition.authority` respectively (the destinations of legacy rows 14 and 15). | Test (TC-1313) |
| FR-518-AC-5 | A case whose declared refusal the stage does not give (a stage double returning another code or pointer), and a case whose bytes equal the base, each make `write` return `Error::CaseMismatch` naming that identity, and nothing is written. | Test (TC-1313) |
| FR-518-AC-6 | Two `write` calls with `Standard` give byte-identical manifests and case files, and the manifest lists the cases in table order. | Test (TC-1313) |
| FR-518-AC-7 | No case's `expected` field names a digest, a seal or a `v2.*` refusal string, and `mutations/` of a `None` corpus does not exist. | Test (TC-1313) |
| FR-518-AC-8 | Every `mutations/<identity>.json` is named by exactly one manifest case and every case names one existing file; the crate exports no case-count or identity-list constant, so a consumer reads the case list from the manifest. | Test (TC-1313) |
| FR-518-AC-9 | **RETIRED (source-byte integrity removed).** Withdrawn with no replacement: no case mutates source bytes. | Test (TC-1313), retired |
| FR-518-AC-10 | **RETIRED (source-byte integrity removed).** Withdrawn with no replacement. | Test (TC-1313), retired |
| FR-518-AC-11 | With a reader that implements IR-1278, a package whose temporal clause declares a name that differs from the sidecar's is admitted by `read_checked_package` and refused by `Selection::check` with `clause.name`. | Test (TC-1313) |
| FR-518-AC-12 | Over **R**, a sidecar whose profile law revision differs from the revision of the admitted package is refused by `Selection::check` with `definition.revision`. | Test (TC-1313) |

## Dependencies

- **Upstream:** FR-517, FR-513-AC-9 (emission-time selection refusals: the
  emission side of rows 9 to 12 and 17 to 21 is covered there), Contract IR
  FR-038 (temporal binding rows), QSL-1270.
- **Successors:** rows 14 and 15 are carried by rows 12 and 29 and by rows 14b and 15b; row 28 has no successor.
- **Contract IR:** IR-1278, the checked package carries no temporal clause name (FR-518-AC-11).
