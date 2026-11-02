---
id: FR-514
title: "Specify the spine type report, the inventory on exhaustion and the multi-unit cycle refusals"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-contract-ir/FR-052
    type: depends_on
---
# FR-514: Specify the spine type report, the inventory on exhaustion and the multi-unit cycle refusals

## Description

FR-362 and FR-480 were written against the composed lane's `TypeReport`, its
pending-obligation record and its work-exhaustion stops. The spine has none of
these: `spine::CheckedUnit` holds the in-process checked package, the unit's
source and its resolved libraries; `check` returns a unit, a typed refusal, or
a stage limit outcome (`StageFailure::Limit`), and never a partial unit; no
entry point takes a formal identity; and the assembler reports a unit or
dimension cycle with a cause that carries no code (informational only). This
requirement states what the spine holds and does in each case, keeping the
observable obligation of each FR-362 and FR-480 criterion.

## Inputs

- A `ParsedUnitSource`, the `Result` of `check` over it, and, for the
  inventory, the selected formal identity.

## Outputs

- `CheckedUnit::type_report()`; `qsl_replay::qsl_inventory(parsed, outcome,
  formal_identity)`; the cycle refusals.

## Behavior

### The type report

- `check` SHALL produce, with the checked package and from the same parse, a
  `TypeReport` that `CheckedUnit::type_report()` returns. It holds:
  1. **declarations**: for each declaration of the unit in ascending `NodeRef`
     order, its node, its kind and its checked value type (the type node the
     package gives it), or none for a declaration that has no value type;
  2. **requirements**: one `RequirementRecord` for each requirement the unit
     declares (the graph's requirement records), with its node and source
     span;
  3. **obligations**: one `PendingObligation` for each claim node (FR-097) and
     for each protocol requirement obligation S3 records (FR-218), with its
     node, occurrence, the capability kind its claim form requests (FR-057-AC-10)
     and the state `Pending`; a retained obligation is pending because its
     discharge belongs to analysis or replay, not to `check`.
- The report SHALL carry no source reference of its own; its spans resolve
  through the checked unit's `RawSourceRef` (FR-362-AC-5).
- A unit of several source units (FR-480) SHALL have one checked unit per
  source unit, each with the type report of its own parse and none shared.

### The inventory on exhaustion

- `qsl_inventory` SHALL take the `parsed` unit, the `check` outcome and the
  selected formal identity, and SHALL return:
  - for `Ok(unit)`, a `Complete` inventory: one row per requirement record
    and one per pending obligation of the report;
  - for a stage limit outcome (`StageFailure::Limit`) at any stage, an
    `Incomplete` inventory that holds one requirement row for each
    requirement form S2 built from `parsed` (none when the limit stopped S1 or
    S2 before any form existed), no obligation row, and the typed cause: the
    stage, the limit kind and the FR-255 setting name; this cause is the IR's
    `TypeWorkExhausted` (IR FR-052-AC-10);
  - for any other refusal, an error and no inventory rows.
- The spine produces no stop for "admission unfinished"; the IR's
  `TypeAdmissionUnfinished` cause is therefore never produced by QSL.

### The formal identity

- The selected formal identity SHALL be a parameter of `qsl_inventory`. It is
  not a parameter of any `check*` entry point and `CheckedUnit` does not carry
  it. The inventory's identity binds the checked unit's source reference and
  the formal identity; a `Complete` and an `Incomplete` inventory of the same
  source and formal identity are the same inventory identity.

### Unit and dimension cycles across source units (FR-480-AC-6)

- A refusal for an FR-142 unit-graph defect found in the assembler SHALL carry
  the code and cause FR-091's unit-graph topology row gives it: a unit target
  cycle (row 6) and a dimension cycle `invalid_package`/`definition-cycle`
  with the dependency edges as payload; a zero scale (row 1) and a second
  root (row 5) `invalid_package`/`invalid-value`; an unresolved name
  `missing_declaration`/`missing-name`.
- The refusal SHALL locate the defect at the declaration in its own document,
  naming that document's source reference and the declaration span, and SHALL
  report the first defect in the order the units are presented.

### FR-480 pieces

FR-480's criteria belong to four pieces of work (QSL-1508):

| Piece | Criteria |
| --- | --- |
| (a) admission layer: the unit list, source and header admission | FR-480-AC-4; the library-conflict sentence of FR-480-AC-2 |
| (b) multi-unit assembler: cross-unit declarations, types, cycles, type reports | FR-480-AC-2 (types and ambiguity), AC-5 (ownership of nodes), AC-6, AC-8 |
| (c) cross-unit dispatch, linking and ownership | FR-480-AC-1, AC-3 |
| (d) FR-098 multi-source replay | FR-480-AC-7 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-514-AC-1 | For a unit with a function, a state clause with a `post` claim and a protocol with one `check` and one `requirement`, `type_report()` lists the declarations in ascending `NodeRef` order with their value types, one `RequirementRecord` per requirement and one `PendingObligation` per claim and per protocol requirement obligation, each `Pending` with the capability kind of its claim form; it holds no source reference field. | Test (TC-1309) |
| FR-514-AC-2 | The two units of FR-480-AC-2's package give two `CheckedUnit`s whose type reports list only their own declarations; no API accepts a report or source for rebinding. | Test (TC-1309) |
| FR-514-AC-3 | With `s3.work_units` set one below the unit's measured need, `check` returns the stage limit outcome and `qsl_inventory` returns `Incomplete` with one requirement row per requirement form, no obligation row and cause (S3, work budget, `s3.work_units`); with `s1.nodes` set to stop S1, `Incomplete` with zero rows and cause (S1, node count, `s1.nodes`); with the unit refused `ill_typed`, an error and no rows; with sufficient limits, `Complete`. | Test (TC-1309) |
| FR-514-AC-4 | `qsl_inventory` over one source with formal identities `F1` and `F2` gives two inventories with different identities, and over `Incomplete` and `Complete` outcomes of one source and `F1` the same identity; no `check*` signature has a formal-identity parameter and `CheckedUnit` has no such member. | Test (TC-1309) |
| FR-514-AC-5 | The FR-480-AC-6 documents give, for `[1, 2]`, `invalid_package`/`invalid-value` at `z` in document 1; for `[2, 1]`, `invalid_package`/`definition-cycle` at `u_self` in document 2 with edge `u_self` to `u_self`; a two-unit cycle `z`, `z2` across documents gives `invalid_package`/`definition-cycle` with both edges; a dimension cycle across documents gives the same code; each names the document's source reference. | Test (TC-1309) |
| FR-514-AC-6 | Every FR-480 criterion appears in exactly one row of the pieces table, and the Linear tickets of pieces (a) to (d) each cite their criteria. | Inspection |

## Dependencies

- **Upstream:** FR-362, FR-480, FR-091 (topology table), FR-255 (setting
  names), FR-097, FR-218, FR-057-AC-10.
- **Contract question (Contract IR FR-052):** the inventory's cause set names
  `TypeAdmissionUnfinished`; QSL never produces it. Whether FR-052 keeps or
  drops the cause is the IR owner's decision; the obligation FR-052-AC-10
  states (partial rows and a typed cause on exhaustion) is kept here.
