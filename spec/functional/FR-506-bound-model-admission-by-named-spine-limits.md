---
id: FR-506
title: "Bound model admission by named spine limits"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: references
---
# FR-506: Bound model admission by named spine limits

## Description

`src/native_model` bounded a model by `ModelLimits` (roles, entries, nodes,
depth, artifact bytes, IR canonicalization) and by the fixed
`MAX_SEQUENCE_ITEMS` of 10,000 (FR-015-AC-2, FR-015-AC-6, FR-041-AC-7). The
ruling on QSL-858 is to delete a legacy limit only where the spine already
bounds the same resource, and to port the rest. This requirement records, for
each legacy limit, the spine limit that bounds the same resource (so the
legacy limit is deleted with its carrier), and it ports the one limit with no
spine equivalent, the declared collection maximum.

## Inputs

- A domain package admitted under FR-056 and the stage limits of FR-255.
- A declared collection maximum: the upper bound of a field's, parameter's or
  result's multiplicity, or of a sequence value type, in the model document.

## Outputs

- For each legacy dimension, the FR-255 setting that bounds it (table below).
- A new FR-255 setting `model.sequence_maximum`.

## Behavior

| Legacy limit (code) | Spine limit for the same resource | Disposition |
| --- | --- | --- |
| roles (`model roles limit exceeded`) | `model.declaration_records` | deleted with the carrier |
| entries (scalar sites, parameters, frame fields, created, deleted) | `model.declaration_records` | deleted with the carrier, provided the charge rule below holds |
| formal nodes (types, values, fields, variants) | `s3.nodes` and `model.declaration_records` | deleted with the carrier |
| artifact bytes: input document | `intake.input_bytes` | deleted with the carrier |
| artifact bytes: string content bound | `s3.input_bytes` and `identity.input_bytes` | deleted with the carrier |
| artifact JSON writer cap | `identity.input_bytes` | deleted with the carrier |
| IR canonicalization limit | `identity.input_bytes` | deleted with the carrier |
| nested value-type depth (hard maximum 64) | none: ADR-030 D-1 removes depth as a limit | deleted, recorded decision |
| `MAX_SEQUENCE_ITEMS` 10,000 | none | ported: `model.sequence_maximum` |

- Model normalization SHALL charge `model.declaration_records` one record for
  each declared type, field, operation, operation parameter, operation result
  and frame entry (`modifies`, `creates`, `deletes`), so that an entry
  counted by the legacy `entries` limit is counted here.
- Intake SHALL refuse a declared collection maximum above
  `model.sequence_maximum` (default 10,000, used as given, raisable and
  lowerable by the caller, never disabled by zero) with `resource_exhausted`
  naming the setting `model.sequence_maximum`, its value and the declared
  maximum, before any declaration is published. The limit applies to every
  collection wrapper in used or unused declarations, including nested options
  and sequences, and is independent of any runtime length (FR-015's wording,
  kept). The refusal is a stage limit outcome (FR-255 Behavior 1) and exits 22
  (FR-285).
- Intake SHALL refuse a Text value type whose `maxLength` exceeds 1,048,576
  scalars (the Contract IR text ceiling) with
  `unsupported_construct`/`declaration-form` naming the bound; the ceiling is
  not a caller setting.
- A refusal by any limit above SHALL publish no declaration and no partial
  package (FR-015-AC-6).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-506-AC-1 | A package of N model records, with one field per type, one operation with one parameter, one result and one `modifies`, one `creates` and one `deletes` entry, charges `model.declaration_records` exactly its count of declared types, fields, operations, parameters, results and frame entries: with the limit set to that count the package admits, with the count less one it is `resource_exhausted` at `model.declaration_records` and nothing is published, and with 0 it is `resource_exhausted` (zero never disables). | Test (TC-1301) |
| FR-506-AC-2 | A field with multiplicity upper bound 10,000, a nested option of a sequence of maximum 10,000, and an unused declaration with maximum 10,000 all admit at the default; the same shapes with maximum 10,001 each refuse `resource_exhausted` naming `model.sequence_maximum`, 10,000 and 10,001, the unused one included; setting `model.sequence_maximum` to 10,001 admits all three. The refusal exits 22. | Test (TC-1301) |
| FR-506-AC-3 | `model.sequence_maximum` 0 refuses every package that declares any collection; a limit of 3 admits a maximum 3 and refuses 4 at the exact boundary. | Test (TC-1301) |
| FR-506-AC-4 | A Text value type with `maxLength` 1,048,576 admits and with 1,048,577 refuses `unsupported_construct`/`declaration-form` naming the bound, whatever the limits. | Test (TC-1301) |
| FR-506-AC-5 | At each of `intake.input_bytes` (document bytes), `s3.nodes` (formal nodes) and `identity.input_bytes` (identity preimage bytes), a model whose usage equals the setting admits, usage plus one refuses at that setting naming it, and zero refuses; none of the legacy messages (`model roles limit exceeded`, `model entries limit exceeded`, `model nodes limit exceeded`) is produced. | Test (TC-1301) |
| FR-506-AC-6 | A model nested 200 levels deep in option and sequence value types admits under default limits (ADR-030 D-1: depth is not a limit). | Test (TC-1301) |

## Dependencies

- **Upstream:** FR-255 (setting table; this requirement adds the row
  `model.sequence_maximum`), FR-056, ADR-030 D-1, ADR-014 B-6 (the fixed
  native-v1 ceiling that this requirement ports as a setting).
- **Spec/code note:** FR-015's text says the sequence limit is `resource_exhausted`
  and raisable; the deleted code refused with `unrepresentable_constraint` at
  exit 20, not raisable. This requirement keeps FR-015's text. Planner
  confirmation requested.
