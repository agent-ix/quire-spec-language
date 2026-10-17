---
id: FR-362
title: "Produce the checked graph and the type report from one source and bind them by digest"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-033
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
---
# FR-362: Produce the checked graph and the type report from one source and bind them by digest

Status: Proposed. It becomes Accepted on merge. It states ADR-033 Option A+D.

## Description

QSL SHALL read one source text once, with one grammar, and from that single
parse produce both the `CheckedGraph` and the `TypeReport`. A checked unit SHALL
bind a graph, a type report and a selected formal source only when the
`quire.source.bytes/v1` digest of each is the same digest.

## Target design

1. **One grammar.** The complete-V1 grammar is the only source grammar. No
   other parser reads a source for the graph, the type report or the source
   inventory, and no function converts one syntax tree into another to feed a
   stage. The type report is produced by the S3 binding phase and the family
   checkers (ADR-011) over the S2 forms of the same parse; the composed linker
   and checker are not re-read over those forms.
2. **One reading.** The producer takes one `Source` (its authority, identity,
   revision and the digest of its exact bytes) and one parse of it. The
   graph's `RawSourceRef` and the type report unit's `Source::reference()` are
   the `Source`'s own reference.
3. **One producer, one `CheckedUnit`.** The producer is the spine front end
   (parse, build, assemble, check over one `Source`) extended by the type
   phase. It takes the source, the selected formal identity, the spine's
   `LockEvidence` and the stage limits, and returns the checked unit or
   the typed refusal of the first stage that refused. Exactly one public
   `CheckedUnit` type exists: the spine's, which carries the graph's
   requirement records, the type report, the formal identity and the source
   references. The type report covers one unit by construction. The type the
   source inventory used to define is deleted, and `qsl_inventory` with its
   `CheckedUnit` lives in `qsl_replay`.
4. **Digest-bound unit.** The checked unit is built only by that producer or by
   a constructor that takes a formal source, a graph and a type report and
   compares their source references as whole references: authority, identity
   and digest. Equal labels with a different digest are a refusal.
5. **Typed refusal.** The refusal names the side that differs (the graph or the
   type report), the selected reference and the differing one.
6. **No label-only match.** No type, function or flag compares only the
   authority and identity of a graph, a type report or a formal source.
7. **Profile selection.** A source selects a profile by identity alone
   (FR-110). The one grammar refuses a profile line that carries a `version`
   or a `digest` after the identity. Profiles resolve through `DefinitionLock::pinned`
   and the spine's `LockEvidence`; a revision or digest never belongs to the
   source.
8. **Moved and unmoved families.** The producer admits a family only once its
   type phase runs in S3 over S2 forms. A construct of a family whose type phase has not
   moved (`relation`, `hyper`) is refused with `unsupported_construct` at the construct, and no unit
   is read by two parsers.
9. **Composed requirements during the migration.** FR-035, FR-036, FR-040 and
   FR-049 stay normative for the families whose type phase has not moved to S3.
   The PR that moves a family retires or amends the parts of those
   requirements and of their test cases that name that family, and deletes that
   family's composed parser arm, syntax, linker and checker. The PR that moves
   the last family retires them. This requirement amends none of them.
10. **The inventory.** The owner-issued source inventory of QSL's checked
   requirements and pending obligations is built from the checked unit and
   from nothing else. A unit that the S3 type phase refuses is an error outcome
   of `check` and yields no partial inventory rows. This is a different case
   from the exhaustion stops: when the type phase stops on `TypeWorkExhausted`
   or `TypeAdmissionUnfinished`, the inventory stays `Incomplete`, retains the
   partial rows and carries the typed cause (FR-052-AC-10). The inventory's
   identity binds the whole `RawSourceRef` (authority, identity and digest);
   the encoding of a row's identity belongs to the FR that owns the inventory
   rows.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-362-AC-1 | For a source the grammar admits, the producer returns a graph and a type report from one parse. The graph's source reference, the type report unit's source reference and the formal source's reference are equal, and their digest is the `quire.source.bytes/v1` digest of the input bytes. | Test (TC-1026) |
| FR-362-AC-2 | For a source whose profile line carries `version` or `digest` after the identity, `qsl_cst::parse` returns a recovered `ParsedSource` with an `invalid_syntax` diagnostic at that token, and `build_unit` and the producer refuse it with `invalid_syntax`; no other parser admits it. | Test (TC-1026) |
| FR-362-AC-3 | Given a graph from one text and a type report from a second text that differs by one byte and has the same authority and identity, the checked-unit constructor refuses with a cause naming the type report side, the selected reference and the differing one. The same holds with the graph from the other bytes. | Test (TC-1026) |
| FR-362-AC-4 | A checked unit built from a graph and a type report of the same bytes yields the inventory rows of the producer's own unit; the report type holds one unit, so a two-unit report cannot be built. | Test (TC-1026) |
| FR-362-AC-5 | No type or function compares only the authority and identity of a graph, a type report or a formal source: the label-only types and the label-only constructor do not exist. | Inspection (TC-1027) |
| FR-362-AC-6 | The producer and the type phase it runs call no composed entry point (`parse_native`, `parse_native_source`) and name no composed type (`ComposedUnit`, `syntax::composed`), and no conversion from `ParsedUnit` to a composed tree exists. After the last family has moved, those symbols and the composed linker and checker no longer exist. | Inspection (TC-1027) |
| FR-362-AC-7 | During the migration sequence, a unit that holds a `predicate` and a `relation` clause is refused with `unsupported_construct` at the `relation` clause, and produces no graph and no type report; the same unit without the `relation` clause yields both. A `hyper` clause is refused the same way. Once a family has moved, its constructs are admitted and the test for that family is retired with it. No second parser reads the unit. | Test (TC-1026) |

## Dependencies

- ADR-033 states the options, the measurements and the recommendation this
  requirement follows.
- FR-087 and FR-091 state the checked graph and the assembler; FR-110 states
  profile selection by identity.
- FR-035, FR-036, FR-040 and FR-049 state the composed lane's parse, link,
  check and evaluation inputs. They stay normative for unmoved families and are
  retired family by family (item 9).
- ADR-011 states the stage order and the deletion of the composed lane.
