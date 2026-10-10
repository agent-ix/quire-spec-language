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

ADR-013 §7 slice **S-5b** has no owning FR before this one.
It carries T-4's `LimitExceeded` and its limit-kind enum, O-17's
`RefusalRecord`, and O-22's QSL readers, each naming its position by T-5's
`Locus`. This requirement states where each producer's locus comes from,
which producers cannot know one and why, and how a refusal record gets the
structured fields the catalog requires.

The types are layer F's, in `qsl_foundation::diagnostic` (ADR-011 §6.1).

The key table has no row for the other `ModelQueryRefusal` causes
(including `type-mismatch`) or for `qsl-route`'s `BoundRefusal`; they build
no record.

## Inputs

- ADR-013 T-4, T-5, O-12, O-16, O-17, O-18 and O-22.
- `quire.native.diagnostics/v1`
  (`ix://agent-ix/quire-specification`,
  `proposals/quire-v1/definitions/native-diagnostics.md`, cited by
  reference): the common structured context ("original source region and
  typed input location where known"; "Unavailable context is explicitly
  unavailable; byte zero, an empty path or a name-search match cannot
  masquerade as a located failure"); the `stage_limit_exceeded`,
  `unknown_wire`, `invalid_runtime_input` and `wrong_snapshot` rows; the ten
  value-refusal rows (`inexact_decimal`,
  `decimal_out_of_domain`, `division_out_of_domain`,
  `modulo_out_of_domain`, `text_length_out_of_domain`,
  `integer_out_of_domain`, `rational_out_of_domain`, `ieee_not_exact`,
  `ieee_nan_payload_not_representable`, `ieee_rational_out_of_domain`); and
  its payload value spellings.
- QSpec FR-044 (a bounded integer or `Rational[..]` arithmetic result's
  membership at evaluation), FR-140, FR-141, FR-142 (an integer-target unit
  conversion), FR-145 (`sum` as a fold whose every running total lies in
  `N`'s domain), FR-147, FR-148 and `quire.value.accounting/v1`.
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
- The resolution of a check-stage `quire_semantic_value::location::Location` to a `SourceRegion`.
- The I2 reader's refusals and limits, each with its locus.
- The kernel value refusals' target domains and widths, carried in their
  `quire-exact` variants, and the kernel undefined reason
  `Undefined::SumOutOfDomain`.

## Behavior

### A check location resolves to a region of the unit it was read from

One set of package declarations holds the declarations of one source unit.
They SHALL be checked under that unit's `RawSourceRef` (FR-001), and their
source owner is that reference's `SourceOwner{authority, identity}`.

A `quire_semantic_value::location::Location` names a declaration origin and a child-index path. The
path SHALL be a parent chain shared with the location it was taken from, so
taking a child costs the same at any depth and a location at depth `d` holds
no copy of the `d` steps above it (ADR-030 D-1, FR-258). The index sequence is
built only on output: the FR-269 cause wire writes a location as its
declaration and the child-index array, built from the chain, and reads that
array back into a chain. Resolving every location of one checked unit to a
region SHALL remember the node each chain link reaches, so the work is the
size of the unit, not its size times its depth. Clone, equality, ordering,
hash, debug and drop of a location SHALL NOT recurse with its depth. For a
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

A location SHALL resolve to no region in exactly four cases. In the first
three, the tree holding the position was not read from the unit, so no
region of the unit names it; in the fourth, the position was read from the
unit, but its span does not map to a single region of the document it is
embedded in:

1. A position in a function that the `model` bridge synthesized for FR-151
   dispatch (a function not callable by name). It is built from a
   domain-package declaration.
2. A position with `Origin::Expression`. This is a standalone expression
   handed to `check`, a tree `check` builds itself (the FR-094
   field-refinement guards), or a position `check` names for a table the
   caller built (dispatch tables and model selections).
3. A position in the expression checker's type environment built from a
   domain package's object types (FR-082). Those types were read from the
   domain package, not the unit.
4. A position in a body embedded in a document (C-21), whose mapped span
   spans more than one document region (a layout deletion the C-21 map
   admits splits it). No single region of the document names it.

The checked package SHALL resolve a `quire_semantic_value::location::Location` of its own
declarations by the same rule, so a consumer holding an S6a `Evaluation`
resolves `Evaluation.location` without the forms.

### A stage limit names its kind, bound, counter and locus

`LimitKind` SHALL have seven variants: input bytes, token count, node
count, edge count, occurrence count, diagnostic count and work budget. Its
`catalog_code()` SHALL be `stage_limit_exceeded` with cause
`input-bytes-exceeded`, `token-count-exceeded`, `node-count-exceeded`,
`edge-count-exceeded`, `occurrence-count-exceeded`,
`diagnostic-count-exceeded` or `work-budget-exceeded` respectively
(ADR-030 D-1).

`LimitExceeded` SHALL carry its `LimitKind`, the configured bound, the
actual counter at the failed charge, the limit's setting name
([FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md)), and an
optional `Locus`. Its catalog code is its kind's.

Every ceiling of a stage's own limits type in compiler stages S2 to S4, the
I2 reader and a family `check`'s own per-declaration precheck (the row
"S3, a family `check`, for a declaration as a whole" below, including a
declaration's preimage input bytes) is a stage limit and SHALL be reported
as `LimitExceeded`, never as `resource_exhausted`. `PackageDeclarations::
check` re-reports that same `LimitExceeded` as a `CheckRefusal` with code
`stage_limit_exceeded` for its own caller -- one producer, one path, not a
second ceiling. A package-level `CheckingLimits` stop with no `LimitExceeded`
producer of its own -- `Typer`'s package-wide node count, and lowering's
own work charge (the other S3 rows below) -- is a stage limit
by the same rule, but is reported directly as a `CheckRefusal` with code
`stage_limit_exceeded`, carrying the locus this FR gives its row; it is
never `resource_exhausted`. A ceiling of an
accounting-contract limits type (`ModelNormalizationLimitsV1`,
`PopulationAdmissionLimitsV1`, quire-specification FR-150) is the caller's
meter wherever it is read and keeps `resource_exhausted`
([ADR-014](../decisions/ADR-014-temporal-trace-and-boundedness-architecture.md)
§1). The `quire.native.diagnostics/v1`
`stage_limit_exceeded` row names these surfaces,
together with S1, `replay` and `route`, and the catalog keeps `resource_exhausted` for the caller's
work-budget meter, adding that a semantic maximum is not a caller work
budget. The check stage's `CheckingLimits` ceilings (node count, declaration
input bytes and work, NFR-011) are therefore stage
limits. S1's syntax budgets (`SyntaxLimit`, FR-035) report
`stage_limit_exceeded` with their kind's cause, the token budget as
`token-count-exceeded`; S1 names its position by its diagnostic's region.
The `replay` envelope readers' encoded-byte bound (`BoundExceeded`) is
`input-bytes-exceeded`. This requirement does not rule on the complete-V1
package-graph resolution limits
(`ResolutionCause::InsufficientNextCharge`). The native-v1 parser's
budgets (FR-002-AC-4) report through the lane-private native-v1 `Code`
(ADR-013 §6 Diagnostics row). S4 and the `route` module have no stage limit
yet; their owners specify one with its locus when they add it.

Each producer's locus is the position at which its charge failed:

| Producer | Limits | Locus |
| --- | --- | --- |
| S3, a family `check`, for a declaration as a whole | the declaration's preimage input bytes and work charge | `Locus::Region` over that declaration's span |
| S3, `Typer` and lowering, under `CheckingLimits` | the package-wide node count (NFR-011) | `Locus::Region` over the node whose entry failed the charge, resolved from its `quire_semantic_value::location::Location`. For the package-wide node count this is the node of whichever declaration was being checked when the running count passed the bound |
| S3, lowering's own work charge, under `CheckingLimits` (NFR-011) | the shared work meter lowering charges per node past a declaration's own precheck | `Locus::Region` over the node whose lowering charge crossed the bound, resolved from its `quire_semantic_value::location::Location` |
| I2 reader, IR's reported limits | IR's `Bytes`, `Nodes`, `Edges`, `Occurrences`, `Diagnostics` and `Work` as input bytes, node count, edge count, occurrence count, diagnostic count and work budget | `Locus::Artifact` with the `raw-artifact-digest` digest record of the supplied bytes (FR-201, O-18) and the RFC 6901 pointer IR reports for the value at which the charge failed |

`LimitExceeded`'s locus SHALL be absent in exactly these cases:

1. The position resolves to no region (the four cases above).
2. The I2 reader's own artifact byte ceiling. The reader refuses without
   hashing the oversized bytes, which is the ceiling's purpose, and a digest
   over them is the only name the artifact has.
3. An IR limit for which IR reports no pointer: IR's byte budget, which it
   charges against the whole input rather than at a value.

`ValueFunctionFamily::check` SHALL return `Typer`'s node-count stop as
`StageFailure::Limit` with kind node count, the configured `CheckingLimits`
node limit as bound, the actual count, setting `s3.nodes`, and the locus of
the node whose entry failed. `CheckContext` is not threaded through `Typer`: the
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

The keys of the family and kernel causes S6a raises are these. Each key
names the catalog payload item beside it (`quire.native.diagnostics/v1`):

| Cause type | Code / cause | Key: catalog payload item |
| --- | --- | --- |
| `ProtocolClauseSnapshot` | `wrong_snapshot` / `wrong-anchor` | `required`: the required anchor selection; `supplied`: the supplied anchor selection |
| `ProtocolClauseSnapshot` | `wrong_snapshot` / `forbidden-pre-read` | `read`: the exact prohibited read |
| `ModelQueryRefusal` | `invalid_runtime_input` / `absent-key` | `binding`: the population binding; `key`: the requested key |
| `ModelQueryRefusal` | `foreign_reference` / `foreign-universe` | `required`: the required object universe (the binding's), as lowercase hex; `supplied`: the supplied object universe, as lowercase hex |
| `LimitExceeded` | `stage_limit_exceeded` / the kind's cause | `kind`: the exceeded limit kind's cause tag; `bound`: the configured bound; `actual`: the actual counter |
| `BoundExceeded` (`replay`) | `stage_limit_exceeded` / `input-bytes-exceeded` | `kind`, `bound`, `actual`, as for `LimitExceeded` |
| kernel `Refusal::CardinalityOutOfBound` | `cardinality_out_of_bound` / `below-minimum` or `above-maximum` | `collection`: the collection kind; `bound`: the inclusive bound `[minimum, maximum]`; `count`: the formed count |
| kernel `Refusal::ForeignReference` | `foreign_reference` / `foreign-universe` | `required`: the universe already in force (an equality's left operand, or membership's collection or already-kept member), as lowercase hex; `supplied`: the universe of the value tested against it, as lowercase hex |
| kernel `Refusal::InexactDecimal` | `inexact_decimal` / `nonzero-discarded-digit` | `expected`: the target type's declared domain, `Decimal[lo, hi; smin, smax]` for a decimal target and `Int[lo, hi]` for an integer target (scale zero) |
| kernel `Refusal::DecimalOutOfDomain` | `decimal_out_of_domain` / `outside-domain` | `expected`: the target `Decimal[lo, hi; smin, smax]` domain |
| kernel `Refusal::DivisionOutOfDomain` | `division_out_of_domain` / `quotient-outside-domain` or `remainder-outside-domain` | `expected`: the bounded consumer's `Int[lo, hi]` domain |
| kernel `Refusal::ModuloOutOfDomain` | `modulo_out_of_domain` / `outside-domain` | `expected`: the bounded consumer's `Int[lo, hi]` domain |
| kernel `Refusal::TextLengthOutOfDomain` | `text_length_out_of_domain` / `outside-domain` | `expected`: the declared `Text[min, max; profile]` bounds and profile |
| kernel `Refusal::IntegerOutOfDomain` | `integer_out_of_domain` / `outside-domain` | `expected`: the target `Int[lo, hi]` domain |
| kernel `Refusal::RationalOutOfDomain` | `rational_out_of_domain` / `outside-domain` | `expected`: the `Rational[lo, hi; dmin, dmax]` result domain |
| kernel `Refusal::IeeeNotExact` | `ieee_not_exact` / `rounding-required` | `expected`: the target IEEE width; `flags`: the would-be flag set, the flags the same operation returns under `nearest-even` (QSpec FR-148) |
| kernel `Refusal::IeeeNanPayloadNotRepresentable` | `ieee_nan_payload_not_representable` / `payload-exceeds-target` | `expected`: the explicit width conversion's target IEEE width; `actual`: its source IEEE width |
| kernel `Refusal::IeeeRationalOutOfDomain` | `ieee_rational_out_of_domain` / `outside-domain` | `expected`: the grammar-named `Rational[lo, hi; dmin, dmax]` conversion target |

A cause another family adds to S6a adds its row here, with the key for each
payload item its catalog row requires. A cause with no row here has no
fields to give: `catalog_fields` returns `None` for it, and no record is
built from it. An empty map is never a stand-in.

### A kernel value refusal carries what its record renders

The ten kernel value refusals above are category refusal: the operation is
defined, and the rounding policy, target width or declared target domain
does not admit its result. None retains the refused value. Each field is
one exact ASCII string, spelled as the catalog spells it:

- A domain is its type name and bounds: `Int[lo, hi]`,
  `Decimal[lo, hi; smin, smax]`, `Rational[lo, hi; dmin, dmax]` or
  `Text[min, max; profile]`. Each bound is written as QSpec FR-038 writes an
  integer: a leading `-` when negative, and no other sign, leading zero or
  exponent. Bounds within a group are separated by `, ` and groups by `; `.
  The text profile is its grammar spelling. No rounding mode is written.
- An IEEE width is `binary32` or `binary64`.
- A flag set is its member flag names in the order `invalid`,
  `divide_by_zero`, `overflow`, `underflow`, `inexact`, joined by `,` with
  no space.

Each of these ten `quire-exact` `Refusal` variants SHALL carry, in the
variant, the target domain or IEEE width its `expected` field renders from,
as `ForeignReference` carries its two universes. `IeeeNotExact`
SHALL also carry the would-be flags, and `IeeeNanPayloadNotRepresentable`
the source width its `actual` field renders from. `kernel_refusal_record`
reads each field from the variant and never from a message or from the
checked tree.

`DivisionOutOfDomain` SHALL take its cause from the member the expression
exposes: `quotient-outside-domain` for `div` and `remainder-outside-domain`
for `rem`. Both members are computed exactly; the kernel raises it only when
the exposed member is outside the consumer's domain.

`Refusal::code()` and `Refusal::cause()` SHALL return, for every kernel
cause other than `CheckedInvariant`, the code and cause the key table gives
it, and `kernel_refusal_record` builds its record with them.

### A `sum` running total outside its domain is undefined, not refused

A `sum<N>` seed or running total outside `N`'s domain is none of the ten
refusals. QSpec FR-145 defines `sum` as a fold whose every running total
lies in `N`'s domain, so a running total outside it leaves the fold with no
value in `N`, even when a later summand would bring the total back inside
(QSpec FR-044, FR-140, FR-145 and `quire.value.accounting/v1`).

When S6a evaluates a `sum<N>` whose seed or running total is not a member
of `N`'s domain, it SHALL return `Outcome::Undefined` with the kernel reason
`Undefined::SumOutOfDomain`, never `Outcome::Refused`, and builds no refusal
record. The outcome is located: `Evaluation.location` is the summand's node
when the seed fails, and the `sum` node when an addition's running total
fails. The sum exposes no total, and makes no charge after the failed
membership decision: for an integer `N`, an addition fails after
`integer-arithmetic.arithmetic` and before `integer-arithmetic.result-retain`.
This holds whatever `N`'s numeric family is. The final total is a running
total, so S6a makes no separate `integer_out_of_domain` decision on it.

`Undefined::SumOutOfDomain` names no catalog undefined reason: the
`quire.native.diagnostics/v1` "Undefined reasons" table admits only the
family reasons `absent-key` and `precondition-false`, so it builds no
`UndefinedRecord`. Like the other kernel undefined reasons, it reaches the
consumer as `FamilyOutcome::Evaluated(Outcome::Undefined(_))` with
`Evaluation.location`. QSL's kernel reason spelling for it is
`sum-out-of-domain`, the row in
[FR-100](FR-100-run-a-named-function-through-the-spine.md)'s kernel
undefined-reason table, which owns the spelling (FR-100-AC-10).

A linked `sum` whose prefixes are not all proved members is refused at
checking (QSpec FR-145, `undefined_expression`/`unproved-range`), so only a
`sum` checked under `CheckMode::Kernel` meets this outcome.

`RefusalRecord` SHALL carry the `CatalogCode`, the O-16 category, an
optional `Locus`, and the catalog fields. It is built from a `CatalogCoded`
cause and a locus. Its code is the cause's `catalog_code()`, its category
is `refusal`, and its fields are the cause's `catalog_fields()`.

At S6a, a consumer SHALL build a record as follows:

- From `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))`, it
  builds the record from the family cause.
- From `FamilyOutcome::Evaluated(Outcome::Refused(refusal))`, it builds the
  record from the kernel `Refusal` through QSL F's map of the kernel cause
  (O-17), `kernel_refusal_record`. The map gives a record for each of the
  twelve kernel causes in the key table, and for no other.
  A kernel `CheckedInvariant` SHALL become an `InternalFault`
  (`runtime_invariant`), never a refusal record: a record is always
  category refusal.

In both cases the locus is `Evaluation.location`, resolved by the checked
package. It is absent when the location is `None` or resolves to no region.

### The I2 reader locates its refusals in the artifact

IR's `read_checked_package` decides the version (ADR-013 O-22 Package
schema). When IR refuses the version, the I2 reader SHALL return
`StageFailure::Refused` with code `unknown_wire`/`unsupported-wire`. Its
fields are `actual`, the contract version IR read, and `expected`,
`quire.checked-package/v2`. Its locus is `Locus::Artifact` with the supplied
bytes' `raw-artifact-digest` record and the pointer `/contract_version`.

When IR refuses the bytes as not the RFC 8785 bytes of the document they
hold (`noncanonical_wire`), the I2 reader SHALL refuse with the native code
`noncanonical_wire`, not `invalid_package`.

Every other IR refusal that IR reports at a value SHALL be located at
`Locus::Artifact` with that digest and the RFC 6901 pointer IR reports. A
refusal IR reports at no value, such as malformed JSON, SHALL carry no
locus: the whole-document pointer would stand in for a position it does not
name.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-096-AC-1 | For function `0` with body `if a then b else c + d` and measure `n`, under a unit reference `r`: location (`Body{0}`, `[]`) resolves to the region of the `If` node's span, (`Body{0}`, `[2]`) to the span of `c + d`, (`Body{0}`, `[2, 1]`) to the span of `d`, and (`Measure{0}`, `[]`) to the span of `n`. Each region is under `r`. The checked package of those declarations resolves the same four locations to the same four regions. When the unit is a body embedded at byte offset `k` of a document with reference `d`, with no layout deletions, the same locations resolve under `d` to the same spans shifted by `k`. When the embedding's map splits one of those spans into more than one document region (a layout deletion), the location resolves to no region. A location in an FR-151 synthesized function and a location with `Origin::Expression` resolve to no region. | Test (TC-426) |
| FR-096-AC-2 | `LimitKind`'s `catalog_code()` gives `stage_limit_exceeded` with causes `input-bytes-exceeded`, `token-count-exceeded`, `node-count-exceeded`, `edge-count-exceeded`, `occurrence-count-exceeded`, `diagnostic-count-exceeded` and `work-budget-exceeded` for its seven variants, and a `LimitExceeded` gives its kind's code and carries its setting name. | Test (TC-427) |
| FR-096-AC-3 | S1 over a function body of `not`×8 `a`, with `s1.nodes` one below the unit's syntax-node count, returns `stage_limit_exceeded`/`node-count-exceeded` with that bound, the count reached and setting `s1.nodes`, at the region S1's diagnostic names under the unit's `RawSourceRef`; S2 over the same body, parsed at the default S1 limits, builds its form. | Test (TC-427) |
| FR-096-AC-4 | A declaration whose preimage input bytes exceed a configured bound `B` returns `StageFailure::Limit` with kind input bytes, bound `B`, actual equal to the measured bytes, and `Locus::Region` over the declaration's span. The same limit reached for an FR-151 synthesized function carries no locus. | Test (TC-427) |
| FR-096-AC-5 | A declaration whose work charge is denied by a work budget `W` returns `StageFailure::Limit` with kind work budget, bound `W`, actual equal to the spend the denied charge would have reached, and `Locus::Region` over the declaration's span. | Test (TC-427) |
| FR-096-AC-6 | A family refusal of `lookup<T>(p, r) absent refused` with no member for `r` builds a `RefusalRecord` with code `invalid_runtime_input`/`absent-key`, category refusal, fields `binding` and `key` naming the population binding and the requested key, and `Locus::Region` over the span of the `lookup` expression. An `Evaluation` whose `location` is `None` builds a record with no locus. | Test (TC-428) |
| FR-096-AC-7 | For each `CatalogCoded` cause in the key table (every row but the twelve kernel `Refusal` rows, which `kernel_refusal_record` maps and AC-8 checks), `catalog_fields()` holds exactly the keys the table lists for it. | Test (TC-428) |
| FR-096-AC-8 | For each of the twelve kernel causes in the key table, a `RefusalRecord` built from an S6a `Evaluation` whose outcome is that kernel `Refused` carries the table's code and cause, category refusal, exactly the table's field keys and the evaluation's resolved locus, and `Refusal::code()` and `Refusal::cause()` return the same code and cause. The fields are spelled exactly: `IntegerOutOfDomain` for target `Int[-5, 9]` gives `expected` `Int[-5, 9]`; `DecimalOutOfDomain` for `Decimal[-100, 100; 0, 2]` gives `expected` `Decimal[-100, 100; 0, 2]`; `RationalOutOfDomain` for `Rational[-9, 9; 1, 9]` gives `expected` `Rational[-9, 9; 1, 9]`; `TextLengthOutOfDomain` for `Text[1, 8; nfc]` gives `expected` `Text[1, 8; nfc]`; `InexactDecimal` for an integer target `Int[0, 9]` gives `expected` `Int[0, 9]`; `IeeeNotExact` for a `binary32` result whose `nearest-even` flags are inexact and overflow gives `expected` `binary32` and `flags` `overflow,inexact`; `IeeeNanPayloadNotRepresentable` for a `binary64` to `binary32` conversion gives `expected` `binary32` and `actual` `binary64`. A kernel `CheckedInvariant` builds no record. | Test (TC-428) |
| FR-096-AC-9 | The I2 reader, given bytes whose `contract_version` is `quire.checked-package/v3`, returns `StageFailure::Refused` with code `unknown_wire`/`unsupported-wire`, `actual` `quire.checked-package/v3`, `expected` `quire.checked-package/v2`, and `Locus::Artifact` whose digest is the `raw-artifact-digest` of those bytes and whose pointer is `/contract_version`. Given bytes that are not JSON, it refuses with no locus. A refusal IR reports at a value, such as a `package_id` digest domain, is located at `Locus::Artifact` with the bytes' `raw-artifact-digest` and the pointer of that value. | Test (TC-429) |
| FR-096-AC-10 | The I2 reader, given a v2 wire whose graph has more nodes than its node bound `B`, returns `StageFailure::Limit` with kind node count, bound `B`, IR's consumed counter as actual, and `Locus::Artifact` with the bytes' `raw-artifact-digest` and the pointer IR reports. The same holds for IR's edge, occurrence, diagnostic and work limits, each with its own kind. Given bytes longer than its artifact byte ceiling, it returns kind input bytes with no locus. | Test (TC-429) |
| FR-096-AC-11 | A function whose body is the source text `not not not true` (four expression nodes), parsed under a unit reference so its forms carry spans and checked through `ValueFunctionFamily::check` with `CheckingLimits` node limit 3, returns `StageFailure::Limit` with kind node count, bound 3, actual 4, setting `s3.nodes`, and `Locus::Region` over the span of the node whose entry failed, reported as `stage_limit_exceeded`/`node-count-exceeded`. With node limit 4 and nothing else changed, it returns no limit. The same stop inside an FR-151 synthesized function carries no locus. | Test (TC-378) |
| FR-096-AC-12 | `Code::RuntimeInvariant`, the code of `InternalFault` (T-4, O-16 internal-failure category), resolves to FR-301 exit status 30 (tool failure) through its category's FR-285 exit code, and every other `Code` resolves to 20, 21 or 22. A native `run` whose evaluation refuses with `runtime_invariant` exits 30. A report holding a `runtime_invariant` diagnostic beside invalid, unsupported or incomplete ones exits 30. | Test (TC-470) |
| FR-096-AC-13 | A kernel `DivisionOutOfDomain` for consumer domain `Int[0, 9]` builds a record with `expected` `Int[0, 9]` and cause `quotient-outside-domain` when the exposed member is the quotient and `remainder-outside-domain` when it is the remainder. | Test (TC-428) |
| FR-096-AC-14 | With `type Small = Int[0, 3]` checked under `CheckMode::Kernel`, S6a evaluation of `sum<Small>(x in q: x)` for `q` of `Sequence<Int[0, 3]>[0, 2]` holding `2, 2` returns `FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))`, located at the `sum` node, with no refusal record and no charge after `integer-arithmetic.arithmetic`; it is not `Outcome::Refused(Refusal::IntegerOutOfDomain)`. The same `sum` for `q` holding `1, 2` completes with `3`. `sum<Small>(x in q: x)` for `q` of `Sequence<Int[0, 9]>[0, 2]` holding `5, 0` returns the same undefined outcome, located at the summand node, with no addition. | Test (TC-500) |
| FR-096-AC-15 | An S6a evaluation of `not x` for `x: Boolean`, called through `qsl_semantics::check::ValueFunctionFamily::evaluate` with an Integer argument that admission would have refused, stops on a kernel `CheckedInvariant`. It returns `Err(InternalFault)` naming stage `S6a` and invariant `checked-program-invariant` (category internal failure, code `runtime_invariant`); it returns no `Evaluation` and builds no refusal record. | Test (TC-428) |
| FR-096-AC-16 | A lowering work-budget stop -- the shared work meter denying a per-node charge past a declaration's own precheck -- is a `CheckRefusal`/`stage_limit_exceeded` with kind work budget, `region: None` on its `StageLimitCause`, and `DeclarationRegions::refusal_region` resolving to the specific node whose lowering charge crossed the bound, not the declaration span. | Test (TC-427) |
| FR-096-AC-17 | Two declarations `g1`, `g2`, each with an individually-under-bound node count, checked together under a package-wide node bound one past `g1`'s own count: `g1` types fully, and `Typer`'s package-wide counter, seeded from `g1`'s final count, crosses the bound partway through `g2`'s own body walk -- a `CheckRefusal`/`stage_limit_exceeded` with kind node count located at the specific node of `g2` where the running count passed the bound, never at either declaration's span. | Test (TC-427) |
| FR-096-AC-18 | The I2 reader, given a v2 wire whose bytes are valid JSON for a valid envelope but carry one space after the opening brace, refuses with the native code `noncanonical_wire`, not `invalid_package`. | Test (TC-429) |

The fourth
no-region case (a span the map splits) is backed directly against the
resolver; no production caller builds a multi-segment C-21 map today, so
that path is untested by any end-to-end flow. No production caller sets
`embedding` yet either: `qsl-source`'s own document map
(`qsl-source/src/lib.rs` ~304-344) does not reach the FR-091 assembler, so
a real Markdown-embedded unit still resolves under its own body reference
until that wiring lands.

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  T-4, T-5, O-12, O-16, O-17 and O-22 (whose Implementing ticket row names
  the I2 reader as S-5b's one QSL reader); C-21; §7 slices S-4b and S-5b;
  §8 QC-28.
- [FR-001](FR-001-read-exact-source.md) AC-5 to AC-7 (slice S-4b): the
  unit's `RawSourceRef`.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  AC-1, AC-9 and AC-10: the declaration and expression spans every
  check-stage locus resolves through. AC-1, AC-3 to AC-6 need them.
- [FR-062](FR-062-implement-checked-family-contract.md) AC-5 and AC-11: the
  per-declaration limits and the package node budget.
- [FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md):
  `CatalogCoded`, `UndefinedCoded`, `Evaluation` and `FamilyResult`.
- The QSpec diagnostics catalog (QSpec STD-95, STD-110): its
  `token-count-exceeded`, `edge-count-exceeded`, `occurrence-count-exceeded`
  and `diagnostic-count-exceeded` causes, the ten kernel value-refusal codes
  and the `invalid_source_identity` causes `blank-label` and `empty-path`
  ([FR-001](FR-001-read-exact-source.md)).
- [NFR-011](../non-functional/NFR-011-bound-value-checking-work.md): the
  `CheckingLimits` ceilings.
- IR (`agent-ix/quire-contract-ir`, `quire-contract-model`'s
  `checked_package`, IR-281): `CheckedPackageRefusal.path` is the RFC 6901
  pointer of the value the refusal concerns, absent for a refusal at no
  value; the `UnknownContractVersion` refusal carries the contract version
  it read; and `CheckedPackageIncomplete` carries the pointer of the value
  at which the charge failed, for every limit other than `Bytes`.

## Open Questions

- **FR-096-OQ-1 (answered):** IR's `Edges`, `Occurrences` and `Diagnostics`
  limits carry `edge-count-exceeded`, `occurrence-count-exceeded` and
  `diagnostic-count-exceeded`, which `quire.native.diagnostics/v1` carries (ADR-013 QC-28, STD-95).
- **FR-096-OQ-2 (open):** `inexact_decimal`'s `expected` is the target's
  declared domain. The catalog spells a domain only with both
  bounds, so it gives no spelling for an unbounded `Integer` target of a
  unit conversion under strict `exact`. QSpec owns the ruling. QSL cannot
  reach that case today: `QuantityTarget::Integer` always carries a
  two-bounded `IntegerInterval` (`qsl-semantics/src/value/quantity.rs`,
  `convert_quantity`), and the kernel's `InexactDecimal` raise sites place
  into a `DecimalType` target, which has both bounds. An integer target is
  placed as a temporary `DecimalType` `Decimal[lo, hi; 0, 0]`; its record
  SHALL still render the declared `Int[lo, hi]`, not that placement, which
  FR-096-AC-8's `InexactDecimal` example checks.

