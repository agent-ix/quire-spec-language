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

Status: Proposed. This text is the proposal of ADR-033 Option A.

## Description

QSL SHALL read one source text once, with one grammar, and from that single
parse produce both the `CheckedGraph` and the `TypeReport`. A checked unit SHALL
bind a graph, a type report and a selected formal source only when the
`quire.source.bytes/v1` digest of each is the same digest.

## Target design

1. **One grammar.** The complete-V1 grammar is the only source grammar. No
   other parser reads a source for the graph, the type report or the source
   inventory, and no function converts one syntax tree into another to feed a
   stage.
2. **One reading.** The producer takes one `Source` (its authority, identity,
   revision and the digest of its exact bytes) and one parse of it. The
   graph's `RawSourceRef` and the type report unit's `Source::reference()` are
   the `Source`'s own reference.
3. **One producer.** One public function takes the source, the selected formal
   identity and the stage limits, and returns the graph, the type report and
   the checked unit of both, or the typed refusal of the first stage that
   refused. The function name is the implementation's.
4. **Digest-bound unit.** The checked unit is built only by that producer or by
   a constructor that takes a formal source, a graph and a type report and
   compares their source references as whole references: authority, identity
   and digest. Equal labels with a different digest are a refusal.
5. **Typed refusal.** The refusal names the side that differs (the graph or the
   type report), the selected reference and the differing one. A type report
   that does not cover exactly one unit is a refusal of its own kind.
6. **No label-only match.** No type, function or flag compares only the
   authority and identity of a graph, a type report or a formal source.
7. **Profile selection.** A source selects a profile by identity alone
   (FR-110). The one grammar refuses a profile line that carries a `version`
   or a `digest`, and the definition inventory is the caller's input to the
   binding phase.
8. **The inventory.** The owner-issued source inventory of QSL's checked
   requirements and pending obligations is built from the checked unit and
   from nothing else.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-362-AC-1 | For a source the grammar admits, the producer returns a graph and a type report from one parse. The graph's source reference, the type report unit's source reference and the formal source's reference are equal, and their digest is the `quire.source.bytes/v1` digest of the input bytes. | Test (TC-1026) |
| FR-362-AC-2 | A source whose profile line carries `version` or `digest` is refused by the one grammar with `invalid_syntax` at the line, by the producer and by `qsl_cst::parse`, and no other parser admits it. | Test (TC-1026) |
| FR-362-AC-3 | Given a graph from one text and a type report from a second text that differs by one byte and has the same authority and identity, the checked-unit constructor refuses with a cause naming the type report side, the selected reference and the differing one. The same holds with the graph from the other bytes. | Test (TC-1026) |
| FR-362-AC-4 | A checked unit built from a graph and a type report of the same bytes yields the inventory rows of the producer's own unit, and a type report that covers two units is refused as not one unit. | Test (TC-1026) |
| FR-362-AC-5 | No type or function compares only the authority and identity of a graph, a type report or a formal source: the label-only types and the label-only constructor do not exist. | Inspection (TC-1027) |
| FR-362-AC-6 | The root crate has no composed parser, no composed syntax tree and no `ParsedUnit`-to-composed-tree conversion; the binding and type phase read the output of S2. | Inspection (TC-1027) |

## Dependencies

- ADR-033 states the options, the measurements and the recommendation this
  requirement follows.
- FR-087 and FR-091 state the checked graph and the assembler; FR-110 states
  profile selection by identity.
- ADR-011 states the stage order and the deletion of the composed lane.
