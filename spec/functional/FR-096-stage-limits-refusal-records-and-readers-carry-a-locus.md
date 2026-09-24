---
id: FR-096
title: "S-5b: stage limits, refusal records and the I2 reader name their locus"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/US-005
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/US-013
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: traces_to
---
# FR-096: S-5b: stage limits, refusal records and the I2 reader name their locus

## Description

ADR-013 §7 slice **S-5b** (Linear QSL-160) has no owning FR before this one.
It carries T-4's `LimitExceeded` and its limit-kind enum, O-17's
`RefusalRecord`, and O-22's QSL readers, each naming its position by T-5's
`Locus`. This requirement states where each producer's locus comes from,
which producers cannot know one and why, and how a refusal record gets the
structured fields the catalog requires.

The types are layer F's, in `qsl_foundation::diagnostic` (ADR-011 §6.1).

## Inputs

- ADR-013 T-4, T-5, O-12, O-16, O-17, O-18 and O-22.
- `quire.native.diagnostics/v1` revision `1-draft.6`
  (`ix://agent-ix/quire-specification`,
  `proposals/quire-v1/definitions/native-diagnostics.md`, cited by
  reference): the common structured context ("original source region and
  typed input location where known"; "Unavailable context is explicitly
  unavailable; byte zero, an empty path or a name-search match cannot
  masquerade as a located failure"), and the `stage_limit_exceeded`,
  `unknown_wire`, `invalid_runtime_input` and `wrong_snapshot` rows.
- [FR-001](FR-001-read-exact-source.md): the unit's `RawSourceRef`.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  AC-1 and AC-10: every `Value` parsed form carries its declaration's span,
  and every `Expression` node the span of its CST node.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): `Locus`,
  `SourceRegion` and `JsonPointer`.

## Outputs

- `LimitKind` and `LimitExceeded` in F `diagnostic`.
- `RefusalRecord` in F `diagnostic`, and a second `CatalogCoded` method,
  `catalog_fields`.
- The resolution of a check-stage `check::Location` to a `SourceRegion`.
- The I2 reader's refusals and limits, each with its locus.

## Behavior

### A check location resolves to a region of the unit it was read from

One set of package declarations holds the declarations of one source unit.
They SHALL be checked under that unit's `RawSourceRef` (FR-001), and their
source owner is that reference's `SourceOwner{authority, identity}`.

A `check::Location` names a declaration origin and a child-index path. For a
declaration the assembler built from the unit, it SHALL resolve to a
`SourceRegion` under the unit's `RawSourceRef`, as follows:

- `Origin::Body{index}` starts at the body of function `index`, and
  `Origin::Measure{index}` at its `decreases` measure.
- Each path step `i` moves to child `i` of the current node, numbered as
  `Expression::children` numbers them.
- The region is the span the node reached carries (FR-091-AC-10). When the
  unit is a body embedded in a document, that span is mapped to the
  document region by C-21, under the document's `RawSourceRef`.

A declaration as a whole SHALL be located at its form's span (FR-091-AC-1),
mapped the same way.

A location SHALL resolve to no region in exactly two cases. In each, the
tree holding the position was not read from the unit, so no region of the
unit names it:

1. A position in a function that the `model` bridge synthesized for FR-151
   dispatch (a function not callable by name). It is built from a
   domain-package declaration.
2. A position with `Origin::Expression`. This is a standalone expression
   handed to `check`, a tree `check` builds itself (the FR-094
   field-refinement guards), or a position `check` names for a table the
   caller built (dispatch tables and model selections).

The checked package SHALL resolve a `check::Location` of its own
declarations by the same rule, so a consumer holding an S6a `Evaluation`
resolves `Evaluation.location` without the forms.

### A stage limit names its kind, bound, counter and locus

`LimitKind` SHALL have four variants: input bytes, nesting depth, node
count and work budget. Its `catalog_code()` SHALL be
`stage_limit_exceeded` with cause `input-bytes-exceeded`,
`nesting-depth-exceeded`, `node-count-exceeded` or `work-budget-exceeded`
respectively.

`LimitExceeded` SHALL carry its `LimitKind`, the configured bound, the
actual counter at the failed charge, and an optional `Locus`. Its catalog
code is its kind's.

Every ceiling of compiler stages S1 to S4, the I2 reader and a family
`check` is a stage limit and SHALL be reported as `LimitExceeded`, never as
`resource_exhausted`. The `quire.native.diagnostics/v1`
`stage_limit_exceeded` row (revision `1-draft.6`) names exactly these
surfaces, and the catalog keeps `resource_exhausted` for the caller's
work-budget meter, adding that a semantic maximum is not a caller work
budget. The check stage's `CheckingLimits` ceilings (nesting depth, node
count, declaration input bytes and work, NFR-011) and S1's syntax budgets
(FR-035) are therefore stage limits. S0's source-byte ceiling
(FR-001-AC-4) is not: S0 is not one of the row's surfaces. The native-v1
parser's budgets (FR-002-AC-4) are lane-private (ADR-013 §6) and keep
`resource_exhausted`. S4, the `replay` facade and the `route` module have
no stage limit yet; their owners specify one with its locus when they add
it.

Each producer's locus is the position at which its charge failed:

| Producer | Limits | Locus |
| --- | --- | --- |
| S1 (`qsl-cst`) | nesting depth; CST node and token count, both node count (a token is a CST leaf) (FR-035-AC-5) | `Locus::Region` over the span of the token or node whose entry failed the charge, under the source's `RawSourceRef` |
| S2 (`forms`) | nesting depth (FR-091-AC-9) | `Locus::Region` over the span of the first node past the bound, under the unit's `RawSourceRef` |
| S3, a family `check`, for a declaration as a whole | the contract's nesting entry, and the declaration's preimage input bytes, node count and work charge | `Locus::Region` over that declaration's span |
| S3, `Typer` and lowering, under `CheckingLimits` | nesting depth, and the package-wide node count (NFR-011) | `Locus::Region` over the node whose entry failed the charge, resolved from its `check::Location` |
| S3, package checking, under `CheckingLimits` | a declaration's input bytes and the package's work (NFR-011) | `Locus::Region` over the declaration being charged |
| I2 reader, IR's reported limits | IR's `Bytes`, `Depth`, `Nodes` and `Work` as input bytes, nesting depth, node count and work budget | `Locus::Artifact` with the `raw-artifact-digest` digest record of the supplied bytes (FR-201, O-18) and the RFC 6901 pointer IR reports for the value at which the charge failed |

`LimitExceeded`'s locus SHALL be absent in exactly these cases:

1. The position resolves to no region (the two cases above).
2. The I2 reader's own artifact byte ceiling. The reader refuses without
   hashing the oversized bytes, which is the ceiling's purpose, and a digest
   over them is the only name the artifact has.
3. An IR limit for which IR reports no pointer: IR's byte budget, which it
   charges against the whole input rather than at a value.

`ValueFunctionFamily::check` SHALL return `Typer`'s nesting-depth stop as
`StageFailure::Limit` with kind nesting depth, the configured
`CheckingLimits` depth as bound, the actual depth, and the locus of the node
whose entry failed. `CheckContext` is not threaded through `Typer`: the
family maps `Typer`'s own stop. This backs FR-062-AC-7.

### A refusal record carries the code, category, locus and catalog fields

`CatalogCoded` SHALL have a second required method, `catalog_fields`. It
returns the structured payload the catalog row requires for the cause that
`catalog_code()` names, with the same map shape as `UndefinedRecord.fields`:
one entry for each required payload item other than a location, valued by
the item's rendering. A location item of a row, such as the `lookup` locus
or the call locus, is the record's locus and not a field.

A cause type SHALL carry every payload item the catalog requires for each of
its causes in the cause's own variant. `catalog_fields` reads them from the
variant and never from a message.

The keys of the family causes S6a raises are these. Each key names the
catalog payload item beside it (`quire.native.diagnostics/v1` revision
`1-draft.6`):

| Cause type | Code / cause | Key: catalog payload item |
| --- | --- | --- |
| `ProtocolClauseSnapshot` | `wrong_snapshot` / `wrong-anchor` | `required`: the required anchor selection; `supplied`: the supplied anchor selection |
| `ProtocolClauseSnapshot` | `wrong_snapshot` / `forbidden-pre-read` | `read`: the exact prohibited read |
| `ModelQueryRefusal` | `invalid_runtime_input` / `absent-key` | `binding`: the population binding; `key`: the requested key |

A cause another family adds to S6a adds its row here, with the key for each
payload item its catalog row requires.

`RefusalRecord` SHALL carry the `CatalogCode`, the O-16 category, an
optional `Locus`, and the catalog fields. It is built from a `CatalogCoded`
cause and a locus. Its code is the cause's `catalog_code()`, its category
is `refusal`, and its fields are the cause's `catalog_fields()`.

At S6a, a consumer SHALL build a record as follows:

- From `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))`, it
  builds the record from the family cause.
- From `FamilyOutcome::Evaluated(Outcome::Refused(refusal))`, it builds the
  record from the kernel `Refusal`. QSL F implements `CatalogCoded` for the
  kernel `Refusal` as its map of the kernel cause (O-17).

In both cases the locus is `Evaluation.location`, resolved by the checked
package. It is absent when the location is `None` or resolves to no region.

### The I2 reader locates its refusals in the artifact

IR's `read_checked_package` decides the version (ADR-013 O-22 Package
schema). When IR refuses the version, the I2 reader SHALL return
`StageFailure::Refused` with code `unknown_wire`/`unsupported-wire`. Its
fields are `actual`, the contract version IR read, and `expected`,
`quire.checked-package/v2`. Its locus is `Locus::Artifact` with the supplied
bytes' `raw-artifact-digest` record and the pointer `/contract_version`.

Every other IR refusal that IR reports at a value SHALL be located at
`Locus::Artifact` with that digest and the RFC 6901 pointer IR reports. A
refusal IR reports at no value, such as malformed JSON, SHALL carry no
locus: the whole-document pointer would stand in for a position it does not
name.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-096-AC-1 | For function `0` with body `if a then b else c + d` and measure `n`, under a unit reference `r`: location (`Body{0}`, `[]`) resolves to the region of the `If` node's span, (`Body{0}`, `[2]`) to the span of `c + d`, (`Body{0}`, `[2, 1]`) to the span of `d`, and (`Measure{0}`, `[]`) to the span of `n`. Each region is under `r`. The checked package of those declarations resolves the same four locations to the same four regions. When the unit is a body embedded at byte offset `k` of a document with reference `d`, with no layout deletions, the same locations resolve under `d` to the same spans shifted by `k`. A location in an FR-151 synthesized function and a location with `Origin::Expression` resolve to no region. | Test (TC-426) |
| FR-096-AC-2 | `LimitKind`'s `catalog_code()` gives `stage_limit_exceeded` with causes `input-bytes-exceeded`, `nesting-depth-exceeded`, `node-count-exceeded` and `work-budget-exceeded` for its four variants, and a `LimitExceeded` gives its kind's code. | Test (TC-427) |
| FR-096-AC-3 | With S2 nesting-depth bound `L = 8`, S2 over a body of `not`×8 `a` returns a limit with kind nesting depth, bound 8, actual 9, and `Locus::Region` over the span of the node at depth 9, under the unit's `RawSourceRef`. | Test (TC-427) |
| FR-096-AC-4 | A declaration whose preimage input bytes exceed a configured bound `B` returns `StageFailure::Limit` with kind input bytes, bound `B`, actual equal to the measured bytes, and `Locus::Region` over the declaration's span. The same limit reached for an FR-151 synthesized function carries no locus. | Test (TC-427) |
| FR-096-AC-5 | A declaration whose work charge is denied by a work budget `W` returns `StageFailure::Limit` with kind work budget, bound `W`, actual equal to the spend the denied charge would have reached, and `Locus::Region` over the declaration's span. | Test (TC-427) |
| FR-096-AC-6 | A family refusal of `lookup<T>(p, r) absent refused` with no member for `r` builds a `RefusalRecord` with code `invalid_runtime_input`/`absent-key`, category refusal, fields `binding` and `key` naming the population binding and the requested key, and `Locus::Region` over the span of the `lookup` expression. An `Evaluation` whose `location` is `None` builds a record with no locus. | Test (TC-428) |
| FR-096-AC-7 | For each cause in the key table, `catalog_fields()` holds exactly the keys the table lists for it. | Test (TC-428) |
| FR-096-AC-8 | A `RefusalRecord` built from an S6a `Evaluation` whose outcome is a kernel `Refused` carries the code QSL's kernel map gives that cause, category refusal, and the evaluation's resolved locus. | Test (TC-428) |
| FR-096-AC-9 | The I2 reader, given bytes whose `contract_version` is `quire.checked-package/v3`, returns `StageFailure::Refused` with code `unknown_wire`/`unsupported-wire`, `actual` `quire.checked-package/v3`, `expected` `quire.checked-package/v2`, and `Locus::Artifact` whose digest is the `raw-artifact-digest` of those bytes and whose pointer is `/contract_version`. Given bytes that are not JSON, it refuses with no locus. | Test (TC-429) |
| FR-096-AC-10 | The I2 reader, given a v2 wire whose graph has more nodes than its node bound `B`, returns `StageFailure::Limit` with kind node count, bound `B`, IR's consumed counter as actual, and `Locus::Artifact` with the bytes' `raw-artifact-digest` and the pointer IR reports. Given bytes longer than its artifact byte ceiling, it returns kind input bytes with no locus. | Test (TC-429) |
| FR-096-AC-11 | A function whose body is `not not not true` (four nodes deep), checked through `ValueFunctionFamily::check` with `CheckingLimits` depth 3, returns `StageFailure::Limit` with kind nesting depth, bound 3, actual 4, and `Locus::Region` over the span of `true`, reported as `stage_limit_exceeded`/`nesting-depth-exceeded`. With depth 4 and nothing else changed, it returns no nesting-depth limit. | Test (TC-378) |
| FR-096-AC-12 | S1 over a source nested one bracket pair past its nesting bound `N` returns a limit with kind nesting depth, bound `N`, actual `N + 1`, and `Locus::Region` over the span of the bracket that opened the pair past the bound, under the source's `RawSourceRef`. | Test (TC-427) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  T-4, T-5, O-12, O-16, O-17 and O-22 (whose Implementing ticket row names
  the I2 reader as S-5b's one QSL reader); C-21; §7 slices S-4b and S-5b;
  §8 QC-28.
- [FR-001](FR-001-read-exact-source.md) AC-5 to AC-7 (slice S-4b): the
  unit's `RawSourceRef`.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  AC-1, AC-9 and AC-10 (QSL-141): the declaration and expression spans every
  check-stage locus resolves through. AC-1, AC-3 to AC-6 need them.
- [FR-062](FR-062-implement-checked-family-contract.md) AC-5 and AC-11: the
  per-declaration limits and the package node budget.
- [FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md):
  `CatalogCoded`, `UndefinedCoded`, `Evaluation` and `FamilyResult`.
- The catalog revision QSL claims. `quire.native.diagnostics/v1` states
  that a producer claiming revision `1-draft.5` or earlier emits no
  `stage_limit_exceeded`. QSL's complete-source diagnostics claim `1-draft.3`
  (`qsl-cst/src/diagnostic.rs`, `qsl-semantics/src/complete/package.rs`).
  Emitting any `LimitExceeded` this requirement specifies is gated on QSL's
  claim reaching `1-draft.6`, the revision FR-322 selects (ADR-013 O-17).
- [FR-035](FR-035-parse-composed-native-units.md) AC-5 and
  [NFR-011](../non-functional/NFR-011-bound-value-checking-work.md): the S1
  syntax budgets and the `CheckingLimits` ceilings.
- IR (`agent-ix/quire-contract-ir`, `quire-contract-model`'s
  `checked_package`), conformance work with no ticket (ADR-013 §7). AC-9
  and AC-10 need all three:
  1. `CheckedPackageRefusal.path` is the RFC 6901 JSON pointer of the value
     the refusal concerns, and is absent for a refusal at no value. Today it
     is a bare member name (`contract_version`) or `document`.
  2. The `UnknownContractVersion` refusal carries the contract version it
     read.
  3. `CheckedPackageIncomplete` carries the RFC 6901 pointer of the value at
     which the charge failed, for every limit other than `Bytes`.

## Open Questions

- **FR-096-OQ-1: What stage limit kind do IR's `Edges`, `Occurrences` and
  `Diagnostics` limits map to?** IR's `CheckedPackageLimit` has seven
  kinds. `stage_limit_exceeded` has four causes, and none names edges,
  occurrences or diagnostics. The catalog forbids a producer from choosing
  a broader cause to discard a distinction it knows. ADR-013 QC-28 carries
  the question, and S-5b's I2 limit conversion waits on it.

## Status

Specified under QSL-160. Not implemented. `LimitExceeded`, `StageFailure`
and `Staged` are `qsl-semantics`'s `family::outcome` types, with no locus
and no actual counter. `RefusalRecord` and `catalog_fields` do not exist.
The `CheckingLimits` ceilings and S1's syntax budgets surface as
`resource_exhausted`, and `Typer`'s depth stop as `StageFailure::Refused`.
`ModelRefusalCause::AbsentKey` carries the key but not the population
binding, and `WrongSnapshotCause::WrongAnchor` carries neither anchor
selection, so both causes gain the payload the key table names.
