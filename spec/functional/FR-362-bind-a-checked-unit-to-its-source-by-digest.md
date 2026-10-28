---
id: FR-362
title: "Produce the checked graph and the type report from one source"
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
# FR-362: Produce the checked graph and the type report from one source

Status: Proposed. It becomes Accepted on merge. It states ADR-033 Option A+D.

## Description

QSL SHALL read each source text once, with one grammar, and from that single
parse produce both the `CheckedGraph` and the `TypeReport`. The producer SHALL
return one `CheckedUnit` carrying the `Source`'s single `RawSourceRef`; its graph
and type report are outputs of that same parse, not independent inputs bound by
comparing copied source digests.

## Target design

1. **One grammar.** The complete-V1 grammar is the only source grammar. No
   other parser reads a source for the graph, the type report or the source
   inventory, and no function converts one syntax tree into another to feed a
   stage. The type report is produced by the S3 binding phase and the family
   checkers (ADR-011) over the S2 forms of the same parse; the composed linker
   and checker are not re-read over those forms.
2. **One reading.** The producer takes one `Source` (its authority, identity,
   revision and the digest of its exact bytes) and one parse of it. The graph
   and type report are both derived from this parse; their source locations
   resolve through the checked unit's single source reference.
3. **One producer, one `CheckedUnit`.** The producer is the spine front end
   (parse, build, assemble, check over one `Source`) extended by the type
   phase. It takes the source, the selected formal identity, the spine's
   `LockEvidence` and the stage limits, and returns the checked unit or
   the typed refusal of the first stage that refused. Exactly one public
   `CheckedUnit` type exists: the spine's, which carries the graph's
   requirement records, the type report, the formal identity and the source's
   one `RawSourceRef`. The type report covers one unit by construction. The type the
   source inventory used to define is deleted, and `qsl_inventory` lives in
   `qsl_replay` and reads `spine::CheckedUnit`.
4. **One source reference.** `CheckedUnit` carries the single `RawSourceRef`
   minted for the `Source` read by the producer. Its graph and type report come
   from the same parse; the producer does not accept separately supplied graph,
   report and source references, and no constructor compares copied references
   to re-establish that association. The graph and report do not carry separate
   unit-level source digest fields; source locations use the checked unit's
   reference.
5. **No mismatch outcome.** A graph/report/source mismatch is not a producer
   refusal case: the single producer constructs all three from one `Source` and
   one parse, so no public mismatch constructor or typed refusal exists.
6. **No duplicate binding guard.** No type, function or flag copies the
   `CheckedUnit` source reference to compare it against graph or type-report
   digests. Source intake still verifies the bytes against the `Source`'s own
   reference.
7. **Profile selection.** A source selects a profile by identity alone
   (FR-110). The one grammar refuses a profile line that carries a `version`
   or a `digest` after the identity. Profiles resolve through `DefinitionLock::pinned`
   and the spine's `LockEvidence`; a revision or digest never belongs to the
   source.
8. **Moved families.** The producer admits a family only once its
   type phase runs in S3 over S2 forms. S2 builds a form for a `relation`
   clause and for a `hyper` clause over `behaviours`, and S3 checks them
   (FR-171, FR-172); S2 refuses neither with `NoDispatchEntry` or
   `UnrepresentedConstruct`. A checked `relation` clause over model
   operations, a step relation (FR-173), is emitted as one QSpec FR-477
   `claim`/`step_relation` node (FR-180) and is never refused `NotEmitted`.
   A checked `hyper` clause is refused only as not emitted, the typed
   temporal cause `NotEmitted` that a checked `temporal` clause S4 does not
   write (FR-337 "Clauses S4 does not emit": one under
   the timed profile or with a protocol-target fairness constraint) is refused with, because no S4
   emission writes it. A clause with a fault keeps
   the refusal of its own fault (FR-172, FR-173), a typed hyper cause or, for
   its temporal operators, fairness and profile selection, a typed temporal
   cause, and is never refused `NotEmitted`. A `hyper` clause over a finite
   trace domain and a `relation` over names that are not model operations keep
   the finite reading of QSpec FR-191 (FR-171, FR-172). A construct of a family
   whose type phase has not moved is refused with `unsupported_construct` at
   the construct, and no unit is read by two parsers.
9. **Composed requirements during the migration.** FR-035, FR-036, FR-040 and
   FR-049 stay normative for the composed lane until it is retired. The PR that
   moves a family retires or amends the parts of those requirements and of
   their test cases that name that family, and deletes that family's composed
   parser arm, syntax, linker and checker. This requirement amends none of them.
   Retiring the composed lane is open work tracked as QSL-868, not owed by the
   change that moved `relation` and `hyper`: the composed lane
   (`src/linking/composed`, `src/checking/composed`, `src/parser/composed*`,
   `src/syntax/composed*` and the callers of `parse_native` and
   `parse_native_source`) is still present and is retired family by family.
10. **The inventory.** The owner-issued source inventory of QSL's checked
   requirements and pending obligations is built from the checked unit and
   from nothing else. A unit that the S3 type phase refuses is an error outcome
   of `check` and yields no partial inventory rows. This is a different case
   from the exhaustion stops: when the type phase stops on `TypeWorkExhausted`
   or `TypeAdmissionUnfinished`, the inventory stays `Incomplete`, retains the
   partial rows and carries the typed cause (IR FR-052-AC-10). The inventory's
   identity binds the whole `RawSourceRef` (authority, identity and digest);
   the encoding of a row's identity belongs to the FR that owns the inventory
   rows.
11. **Several source units.** A package of several source units (FR-480) has
    one checked unit per source unit. Each carries its own source reference and
    the type report of its own parse. The shared graph lists every unit's
    `RawSourceRef` in `lock.sources`; each type report covers one unit by
    construction.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-362-AC-1 | For a source the grammar admits, the producer returns one `CheckedUnit` containing the graph, type report and the source's single `RawSourceRef`; the graph and type report come from that source's one parse. The carried reference's digest is the `quire.source.bytes/v1` digest of the input bytes. | Test |
| FR-362-AC-2 | For a source whose profile line carries `version` or `digest` after the identity, `qsl_cst::parse` returns a recovered `ParsedSource` with an `invalid_syntax` diagnostic at that token, and `build_unit` and the producer refuse it with `invalid_syntax`; no other parser admits it. | Test |
| FR-362-AC-3 | The producer does not accept separately supplied graph, type report and formal source inputs, and exposes no graph/report/source mismatch constructor or typed refusal. Its sole `CheckedUnit` is created from the one source parse. | Inspection |
| FR-362-AC-4 | A producer-created checked unit yields the inventory rows derived from its own graph and type report. | Test |
| FR-362-AC-5 | No type, function or flag copies the checked unit's source reference to compare it against graph or type-report digests; source intake verifies the bytes against the source's own reference. | Inspection |
| FR-362-AC-6 | The producer and the type phase it runs call no composed entry point (`parse_native`, `parse_native_source`) and name no composed type (`ComposedUnit`, `syntax::composed`), and no conversion from `ParsedUnit` to a composed tree exists. Retiring those symbols and the composed linker and checker is open work tracked as QSL-868. | Inspection |
| FR-362-AC-7 | A unit that holds a `predicate` and a `relation` clause over model operations whose body checks yields both a graph and a type report, the graph holding the clause's `claim`/`step_relation` node (FR-180). A unit that holds a `hyper` clause whose body checks is refused only as `NotEmitted` for that clause (a typed temporal cause naming the clause and its span), and produces no graph and no type report; the same unit without the `hyper` clause yields both. S2 refuses neither clause with `NoDispatchEntry` or `UnrepresentedConstruct`. A clause with a fault is refused with its typed hyper cause, or its typed temporal cause for temporal operators, fairness and profile selection, and never with `NotEmitted`. No second parser reads the unit. | Test |
| FR-362-AC-8 | A unit that the S3 type phase refuses is an error outcome of `check` and yields no inventory rows. A unit whose type phase stops on `TypeWorkExhausted` or `TypeAdmissionUnfinished` yields an `Incomplete` inventory that retains the partial rows and carries the typed cause. | Test |

## Dependencies

- ADR-033 states the options, the measurements and the recommendation this
  requirement follows.
- FR-087 and FR-091 state the checked graph and the assembler; FR-110 states
  profile selection by identity.
- FR-035, FR-036, FR-040 and FR-049 state the composed lane's parse, link,
  check and evaluation inputs. They stay normative for unmoved families and are
  retired family by family (item 9).
- ADR-011 states the stage order and the deletion of the composed lane.
