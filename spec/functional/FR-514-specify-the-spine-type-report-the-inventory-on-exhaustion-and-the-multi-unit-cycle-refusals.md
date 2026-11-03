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
     node, occurrence, the capability kind its claim form requests (FR-057-AC-10),
     its source span and the state `Pending`; a retained obligation is pending because its
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
    stage, the limit kind and the FR-255 setting name, with the configured
    bound, the observed value and the limit's source region when it has one;
    the IR reports this cause as `TypeWorkExhausted` (IR FR-052-AC-10) and the
    mapping of "The IR identities" below applies;
  - for any other refusal, an error and no inventory rows.
- The spine produces no stop for "admission unfinished"; the spine never
  produces the IR's `TypeAdmissionUnfinished` cause. The legacy composed-lane
  inventory does until QSL-830 deletes it (IR-1304 removes the cause).

### Requirement forms

A **requirement form** is a declaration form of the unit's S2 form list (the
`ParsedUnitSource`) of a kind the checked graph records a requirement for: a
`function`, a state clause, an operation body, a `protocol`, a `temporal`
clause, a `terminal`, a `refinement`, a hyper-behaviours or hyper-finite
declaration, a model relation, a probabilistic claim, a state-graph claim, a
`schedulable` declaration, a `response` and a time invariant. Declarations that
state no requirement (a `union`, `record`, `tuple`, `enum`, `alias`,
`dimension`, `unit` or `bound`) are not requirement forms. The set is decided
from the S2 form's kind alone, before any type work, so a `function` that
declares no requirement is a requirement form that has no requirement record in
a `Complete` inventory. A partial inventory therefore lists the forms that can
carry a requirement; it is never read as the final row set, and its `Incomplete`
outcome says so.

A requirement form's **identity** is its position among the unit's requirement
forms in source order (a non-negative integer, the form ordinal), with the
declared name when the form has one. It names no checked node, because none
exists when a limit stopped the stage, and it is a different kind of identity
from the requirement record of a `Complete` row, which it never equals.

### The IR identities

The inventory reports each row and the exhaustion cause as the IR's neutral
identities (Contract IR FR-052-AC-26). The normative content is QSL's own: the
row kinds and cause fields below. Where the IR has no member for a QSL value,
the mapping is an explicit IR dependency (Dependencies, "Asks to the Contract IR
owner"), and the **interim clauses** below say exactly what the producer carries
until the IR adds the member. An interim clause is documented here and in the
producer's rustdoc, is never silent, and ends when its removal condition holds;
`Ok(unit)` is `Complete` and a stage limit is `Incomplete` throughout.

- **Requirement record** row: `SourceRequirementOccurrenceId` of the record's
  structural `CheckedNodeId`, origin role and origin ordinal, as FR-052-AC-26
  states.
- **Requirement form** row (only in an `Incomplete` outcome): the form ordinal
  and declared name above. Interim I-1: until the IR has a requirement-form
  identity, the row is carried as a requirement-record identity with package
  `""`, a single name path segment holding the declared name (or `""`), node
  ordinal and origin ordinal both the form ordinal, and role `form`; no
  `Complete` requirement record has that package, so the two never compare equal.
  Removal: the IR adds the identity (ask 1) and the producer returns it.
- **Pending obligation** row. The normative content is the obligation's
  **capability kind name** (FR-057-AC-10, FR-290), its owning declaration, its
  source unit and its source span. The IR's `SourcePendingObligationId` maps it
  as: `site.declaration` is the owning declaration's ordinal in
  `type_report().declarations()`; `site.unit` is the unit's position in the
  presented unit list (0 for a single unit); `site.expression` is absent; `site.start`
  and `site.end` are the span's bytes resolved through the unit's `RawSourceRef`;
  `kind` is the capability kind name. The IR's `SourcePendingObligationKind`
  names type-level obligations only and has no member for a capability kind
  (ask 2). Interim I-2: until it does, the producer carries `kind`
  `GraphClosure` and `site.declaration` equal to the obligation's position in the
  obligation list, `site.unit` 0, so that distinct obligations stay distinct; the
  capability kind name is retained in QSL's own row beside the IR identity and is
  what a consumer of QSL's row reads. These two interim values are placeholders,
  not measured values, and no QSL test asserts them as the capability kind or the
  declaration. Removal: the IR adds the member (ask 2), the producer maps the
  capability kind and the owning declaration ordinal, and this clause is deleted.
- **Exhaustion cause**, the IR's `TypeWorkExhausted(SourceTypeExhaustion)`: the
  normative content is the stage, the limit kind, the FR-255 setting name, the
  configured bound, the observed value and the limit's source region when it has
  one, held in QSL's own `InventoryCause`. The IR `dimension` is the dimension of
  the limit kind (token and node counts: expressions; edge count: edges;
  occurrence and diagnostic counts: records; work budget: constraints; rational
  bits: normalization; input bytes: bytes) and `limit` is the configured bound.
  The spine holds no charged-before amount, no unpaid increment, no declaration
  handle and no unit handle, and `SourceTypeExhaustion` has no field for the stage,
  the setting or the observed value (ask 3). Interim I-3: until it does, the
  producer carries `prior` equal to the bound, `requested` equal to the observed
  value minus the bound (0 when not above), `site.declaration` and `site.unit` 0,
  and the region as the site's span, as placeholders that no consumer may read as
  measured; the stage and setting are in QSL's `InventoryCause`. Removal: the IR
  adds ask 3 and the producer returns the stage, setting and observed value.

### The `parsed` argument

`parsed` is `None` exactly when the limit stopped S1 or S2 before any form
existed. A stage limit at S3 or later, or any `Ok` outcome, with `parsed` `None`
is a caller defect and returns `InventoryRefusal::MissingParsedUnit`, not an
inventory with zero rows. A limit at S1 or S2 with `parsed` `Some` lists the
forms S2 had built.

### The formal identity

- The selected formal identity SHALL be a parameter of `qsl_inventory`. It is
  not a parameter of any `check*` entry point and `CheckedUnit` does not carry
  it. The inventory's identity binds the checked unit's source reference and
  the formal identity; a `Complete` and an `Incomplete` inventory of the same
  source and formal identity are the same inventory identity.

### Unit and dimension cycles across source units (FR-480-AC-6)

- A refusal for an FR-142 unit-graph defect found in the assembler SHALL carry
  the code and cause FR-091's unit-graph topology row gives it, with the row
  numbers of QSpec FR-142's unit-graph table (checked against it): a unit
  target cycle (FR-142 row 6) and a dimension cycle `invalid_package`/`definition-cycle`
  with the dependency edges as payload; a zero scale (FR-142 row 1) and a second
  root (FR-142 row 5) `invalid_package`/`invalid-value`; an unresolved name
  `missing_declaration`/`missing-name`.
- The refusal SHALL locate the defect at the declaration in its own document,
  naming that document's source reference and the declaration span, and SHALL
  report the first defect in the order the units are presented.

### FR-480 pieces

FR-480's criteria belong to four pieces of work (QSL-1508):

| Piece | Criteria |
| --- | --- |
| (a) admission layer: the unit list, source and header admission | FR-480-AC-4; FR-480-AC-2, its library-conflict sentences (the dependency input that also supplies a library with a unit's authority and identity, and `import A` of an unsupplied library) |
| (b) multi-unit assembler: cross-unit declarations, types, cycles, type reports | FR-480-AC-2, its cross-unit type and ambiguity sentences (the `[B, A]` list and the duplicate `Point`); AC-5 (ownership of nodes), AC-6, AC-8 |
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
| FR-514-AC-6 | A consistency test over this requirement and FR-480 finds every `FR-480-AC-n` id of FR-480 in the pieces table; only FR-480-AC-2 appears in two rows, and the two rows quote disjoint sentences of it. | Test (TC-1309) |
| FR-514-AC-7 | For `T0` with `s3.work_units` one below need, the `Incomplete` inventory lists one requirement-form row per requirement form of `T0`'s S2 list (the function `f`, the state clause and the protocol), each identified by its form ordinal 0, 1, 2 and its declared name when it has one, a `function` declaring no requirement included; none of those identities equals a `Complete` requirement record, and an `enum` declared beside them gives no row. | Test (TC-1309) |
| FR-514-AC-8 | For `T0` with sufficient limits, `qsl_inventory` returns `Complete` with one requirement row per requirement record and one obligation row per pending obligation (FR-362-AC-4), and each obligation row's QSL form holds its capability kind name, owning declaration ordinal in `type_report().declarations()`, unit 0 and the span's bytes; two obligations of one declaration differ by span and kind. The IR identity of each carries the interim values of I-2 and no QSL test asserts them as the capability kind. | Test (TC-1309) |
| FR-514-AC-9 | For the limit outcomes of AC-3, `InventoryCause` holds the stage (S3, S1), the limit kind, the setting (`s3.work_units`, `s1.nodes`), the configured bound and the observed value, and the IR cause carries the dimension (`constraints` for the work budget, `expressions` for `s1.nodes`), `limit` the bound and the interim values of I-3. | Test (TC-1309) |
| FR-514-AC-10 | `qsl_inventory` with `parsed` `None` over a limit at S1 or S2 returns `Incomplete` with zero rows and the cause; with `parsed` `None` over a limit at S3, or over `Ok(unit)`, it returns `InventoryRefusal::MissingParsedUnit` and no inventory. | Test (TC-1309) |

## Dependencies

- **Upstream:** FR-362, FR-480, FR-091 (topology table), FR-255 (setting
  names), FR-097, FR-218, FR-057-AC-10.
### Asks to the Contract IR owner (FR-052)

1. **Requirement-form identity.** A `SourceInventoryIdentity` member (a requirement
   form) beside `SourceRequirementOccurrenceId`, carrying a form ordinal (`u64`)
   and an optional declared name; ends I-1.
2. **Capability-kind member.** A `SourcePendingObligationKind` member carrying
   the FR-057 capability kind name (the enum is `Copy` today, so the member needs
   a non-`Copy` payload or an interned handle); ends I-2.
3. **Exhaustion fields.** In `SourceTypeExhaustion`: a `stage` (S1, S2 or S3
   and later), a `setting` (the FR-255 setting name), an `observed` amount, and
   `prior`, `requested` and the site's declaration and unit handles optional (the
   spine holds a source region only); ends I-3.

- **Contract IR FR-052 (decided):** IR FR-052 as amended by IR#854 keeps the
  `TypeAdmissionUnfinished` cause, scoped to QSL's legacy composed-lane
  inventory; IR-1304 retires it once QSL-830 deletes that lane. The spine
  never produces it. The obligation FR-052-AC-10 (partial rows and a typed
  cause on exhaustion) is kept here.
