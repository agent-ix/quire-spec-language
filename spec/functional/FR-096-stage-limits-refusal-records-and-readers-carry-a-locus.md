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

Package declarations SHALL be checked under the `RawSourceRef` of the unit
they were assembled from (FR-001). Their source owner is that reference's
`SourceOwner{authority, identity}`.

A `check::Location` names a declaration origin and a child-index path. For a
declaration the assembler built from the unit, it SHALL resolve to a
`SourceRegion` under the unit's `RawSourceRef`, as follows:

- `Origin::Body{index}` starts at the body of function `index`, and
  `Origin::Measure{index}` at its `decreases` measure.
- Each path step `i` moves to child `i` of the current node, numbered as
  `Expression::children` numbers them.
- The region is the span the node reached carries (FR-091-AC-10).

A declaration as a whole SHALL be located at its form's span (FR-091-AC-1).

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
code is its kind's. A stage limit is never reported as `resource_exhausted`,
which the catalog keeps for the caller's work-budget meter.

Each producer's locus is the position at which its charge failed:

| Producer | Limits | Locus |
| --- | --- | --- |
| S1 (`qsl-cst`) | nesting depth; CST node and token count, both node count (a token is a CST leaf) | `Locus::Region` over the span of the token or node whose entry failed the charge, under the source's `RawSourceRef` |
| S2 (`forms`) | nesting depth (FR-091-AC-9) | `Locus::Region` over the span of the first node past the bound |
| S3, a family `check`, for a declaration as a whole | the contract's nesting entry, and the declaration's preimage input bytes, node count and work charge | `Locus::Region` over that declaration's span |
| S3, `Typer` | expression nesting depth (`CheckingLimits.depth`) and the package-wide node count (`CheckingLimits.nodes`) | `Locus::Region` over the node whose entry failed the charge, resolved from its `check::Location` |
| I2 reader, IR's reported limits | IR's `Bytes`, `Depth`, `Nodes` and `Work` as input bytes, nesting depth, node count and work budget | `Locus::Artifact` with the `raw-artifact-digest` digest record of the supplied bytes (FR-201, O-18) and the RFC 6901 pointer IR reports for the value at which the charge failed |

`LimitExceeded`'s locus SHALL be absent in exactly these cases:

1. The position resolves to no region (the two cases above).
2. The I2 reader's own artifact byte ceiling. It refuses before any byte is
   read, and the only name for the artifact is a digest over the bytes the
   ceiling refuses to read.
3. An IR limit for which IR reports no pointer: IR's byte budget, which it
   charges against the whole input rather than at a value.

### A refusal record carries the code, category, locus and catalog fields

`CatalogCoded` SHALL have a second required method, `catalog_fields`. It
returns the structured payload the catalog row requires for the cause that
`catalog_code()` names, with the same map shape as `UndefinedRecord.fields`:

- one entry for each required payload item other than a location;
- keyed by QSL's name for that item, with the key set fixed for each cause;
- valued by the item's rendering.

A cause type SHALL carry every payload item the catalog requires for each of
its causes in the cause's own variant. `catalog_fields` reads them from the
variant and never from a message. A location item of a row, such as the
`lookup` locus or the call locus, is the record's locus and not a field.

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

### O-22: the one QSL reader this slice covers

O-22's QSL reader in S-5b SHALL be the I2 reader
(`qsl-package`'s `read_checked_package_v2`) alone. The other versioned
readers are outside this slice:

- The #231 replay envelope readers (the replay request,
  `quire.native-runtime/v1`, and the proof result,
  `quire.backend-provider/v1`) own their version refusal under O-22's #231
  row. Each already refuses another version as
  `unknown_wire`/`unsupported-wire` and keeps the actual version.
- `decode_function_package_v2` reads `quire.checked-function-package/v2`,
  which is not a QSpec contract identifier. It is the layer-5 prototype
  codec that stays in `qsl-eval` only until QSL-6 S3 (ADR-011 X-7, X-8).
- The native-v1 readers are lane-private (ADR-013 §6, R-09) and gain no
  consumer.

IR's `read_checked_package` decides the version (O-22 Package schema). When
IR refuses the version, the I2 reader SHALL return `StageFailure::Refused`
with code `unknown_wire`/`unsupported-wire`. Its fields name the actual
contract version IR read and the expected `quire.checked-package/v2`. Its
locus is `Locus::Artifact` with the supplied bytes' `raw-artifact-digest`
record and the pointer `/contract_version`. Every other IR refusal SHALL be
located at `Locus::Artifact` with that digest and the RFC 6901 pointer IR
reports.

### What makes FR-062-AC-7 constructible

FR-062-AC-7's nesting-depth limit is `Typer`'s `CheckingLimits.depth`, the
bound that real recursive descent charges. `ValueFunctionFamily::check`
SHALL return `Typer`'s depth refusal as `StageFailure::Limit` with kind
nesting depth, the configured bound, the actual depth and the locus of the
node whose entry failed. `CheckContext` is not threaded through `Typer`: the
family maps `Typer`'s own refusal. The locus is what keeps the location that
PR #303's review required. It needs the unit's `RawSourceRef` (FR-001,
slice S-4b) and the forms' spans (FR-091-AC-10).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-096-AC-1 | For function `0` with body `if a then b else c + d` and measure `a`, under a unit reference `r`: location (`Body{0}`, `[]`) resolves to the region of the `If` node's span, (`Body{0}`, `[2]`) to the span of `c + d`, (`Body{0}`, `[2, 1]`) to the span of `d`, and (`Measure{0}`, `[]`) to the span of the measure. Each region is under `r`. A location in an FR-151 synthesized function and a location with `Origin::Expression` resolve to no region. The checked package of those declarations resolves the same four locations to the same four regions. | Test (TC-425) |
| FR-096-AC-2 | `LimitKind`'s `catalog_code()` gives `stage_limit_exceeded` with causes `input-bytes-exceeded`, `nesting-depth-exceeded`, `node-count-exceeded` and `work-budget-exceeded` for its four variants, and a `LimitExceeded` gives its kind's code. | Test (TC-426) |
| FR-096-AC-3 | A function whose body is `not not not true` (four nodes deep), checked through `ValueFunctionFamily::check` with `CheckingLimits.depth` 3, returns `StageFailure::Limit` with kind nesting depth, bound 3, actual 4 and `Locus::Region` over the span of `true`. Its catalog code is `stage_limit_exceeded`/`nesting-depth-exceeded`, not `resource_exhausted`. With depth 4 and nothing else changed, it returns no nesting-depth limit. | Test (TC-426) |
| FR-096-AC-4 | A declaration whose preimage input bytes exceed a configured bound `B` returns `StageFailure::Limit` with kind input bytes, bound `B`, actual equal to the measured bytes, and `Locus::Region` over the declaration's span. The same limit reached inside an FR-151 synthesized function carries no locus. | Test (TC-426) |
| FR-096-AC-5 | S1 over a source nested one bracket pair past its nesting bound `N` returns a limit with kind nesting depth, bound `N`, actual `N + 1`, and `Locus::Region` over the span of the bracket that opened the pair past the bound, under the source's `RawSourceRef`. | Test (TC-426) |
| FR-096-AC-6 | A family refusal of `lookup<T>(p, r) absent refused` with no member for `r` builds a `RefusalRecord` with code `invalid_runtime_input`/`absent-key`, category refusal, fields naming the population binding and the requested key, and `Locus::Region` over the span of the `lookup` expression. An `Evaluation` whose `location` is `None` builds a record with no locus. | Test (TC-427) |
| FR-096-AC-7 | For every cause of every `CatalogCoded` type, `catalog_fields()` holds exactly the keys fixed for that cause, and those keys name every payload item the catalog row requires for that cause except its locations. | Test (TC-427) |
| FR-096-AC-8 | A `RefusalRecord` built from an S6a `Evaluation` whose outcome is a kernel `Refused` carries the code QSL's kernel map gives that cause, category refusal, and the evaluation's resolved locus. | Test (TC-427) |
| FR-096-AC-9 | The I2 reader, given bytes whose `contract_version` is `quire.checked-package/v3`, returns `StageFailure::Refused` with code `unknown_wire`/`unsupported-wire`, fields naming actual `quire.checked-package/v3` and expected `quire.checked-package/v2`, and `Locus::Artifact` whose digest is the `raw-artifact-digest` of those bytes and whose pointer is `/contract_version`. | Test (TC-428) |
| FR-096-AC-10 | The I2 reader, given a v2 wire whose graph has more nodes than its node bound `B`, returns `StageFailure::Limit` with kind node count, bound `B`, IR's consumed counter as actual, and `Locus::Artifact` with the bytes' `raw-artifact-digest` and the pointer IR reports. Given bytes longer than its artifact byte ceiling, it returns kind input bytes with no locus. | Test (TC-428) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  T-4, T-5, O-12, O-16, O-17 and O-22; §7 slices S-4b and S-5b.
- [FR-001](FR-001-read-exact-source.md) AC-5 to AC-7 (slice S-4b): the
  unit's `RawSourceRef`.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  AC-1 and AC-10 (QSL-141): the declaration and expression spans every
  check-stage locus resolves through. AC-1, AC-3, AC-4 and AC-6 need them.
- [FR-062](FR-062-implement-checked-family-contract.md) AC-7: backed by
  this requirement's AC-3 fixture.
- [FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md):
  `CatalogCoded`, `UndefinedCoded`, `Evaluation` and `FamilyResult`.
- IR (`agent-ix/quire-contract-ir`, `quire-contract-model`'s
  `checked_package`), conformance work with no ticket (ADR-013 §7). AC-9
  and AC-10 need all three:
  1. `CheckedPackageRefusal.path` is an RFC 6901 JSON pointer. Today it is a
     bare member name (`contract_version`) or `document`.
  2. The `UnknownContractVersion` refusal carries the contract version it
     read.
  3. `CheckedPackageIncomplete` carries the RFC 6901 pointer of the value at
     which the charge failed, for every limit other than `Bytes`.

## Open Questions

- **FR-096-OQ-1: What stage limit kind do IR's `Edges`, `Occurrences` and
  `Diagnostics` limits map to?** IR's `CheckedPackageLimit` has seven
  kinds. `stage_limit_exceeded` has four causes, and none names edges,
  occurrences or diagnostics. The catalog forbids a producer from choosing
  a broader cause to discard a distinction it knows. Until this is decided,
  the I2 reader's conversion of those three kinds is unspecified.

## Status

Specified under QSL-160. Not implemented. `LimitExceeded`, `StageFailure`
and `Staged` are `qsl-semantics`'s `family::outcome` types, with no locus
and no actual counter. `RefusalRecord` and `catalog_fields` do not exist.
`Typer`'s depth refusal surfaces as `resource_exhausted`.
