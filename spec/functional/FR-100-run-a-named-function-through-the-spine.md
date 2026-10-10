---
id: FR-100
title: "Run a named function of a 1-draft program through the spine"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-014
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-038
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-301
    type: depends_on
  - target: ix://agent-ix/quire-specification/interface_005
    type: depends_on
---
# FR-100: Run a named function of a 1-draft program through the spine

## Description

When an author invokes quire-spec run with a native-run/1 request file, the
run command shall route the program source by the edition its header
declares, as compile does
([FR-027](FR-027-export-compiled-native-package.md)). A `0-draft` source,
or one declaring no edition, runs its selected clause through native run
([FR-026](FR-026-run-standalone-native-workflow.md)). A `1-draft` source
compiles through the spine (S1 to S4) and the command calls the one
function the request names through S6a (`CheckedPackage::call`), with the
arguments the request supplies. This is ADR-011 §5's spine `run`.

The spine run entry is `qsl_replay::spine::run`. It takes the caller's
`&Cancel` (ADR-029). A cancel observed at any stage, S1 to S4 or during the
S6a call, returns `RunRefusal::Cancelled` with the handle's cause
(`Requested` or `Deadline`), category incomplete, exit 22; it is never a
fault (FR-100-AC-12). It compiles the source with
the FR-278 composition, selects the function by name lookup in the
compiled package, binds the arguments, calls the function and returns the
call's outcome. The root crate reaches it through `qsl_replay` and names
`qsl-eval` in none of its dependency tables (TC-390).

The request handling is the QSL library operation `command`, which the
driver's `quire run` reaches (FR-287). A `1-draft` native-run/1 request is
the command encoding of the `execute` operation with a function selection
(FR-279). Every exit status this requirement states is FR-285's exit code of
the outcome's ADR-013 O-16 category.

## Inputs

### Typed public call

The public Rust call contract SHALL use these existing shared carriers:

| Boundary member | Carrier and meaning |
| --- | --- |
| `CallArgument.parameter` | The declared parameter name, as before |
| `CallArgument.value` | `quire_exact::Value`, replacing `i64`; this entry admits Boolean, Integer, Reference and Population arguments only |
| `Call.accounting` | The caller's complete `quire_exact::ScalarLimits`, with all ten existing counters |
| `run` object context | An explicit borrowed `&qsl_semantics::model::object_environment::ObjectEnvironment`, immediately before the existing `&Cancel` argument; the other run inputs retain their roles |
| Completed Reference | `CallValue::Reference(quire_exact::ObjectReference)` in the existing `CallOutcome::Completed` arm |

The existing `Result<(PackageId, CallOutcome), Box<RunRefusal>>` return
type SHALL remain the facade's return type. If actual model-context
admission refuses, then `RunRefusal` SHALL carry
`Admission { position: usize, refusal: qsl_semantics::model::normalize::ModelRefusal }`;
its code/category are the original refusal's, its stage is `call`, and its
typed cause/payload are retained. If a call argument is dangling, then
`RunRefusal` SHALL carry
`DanglingReference { position: usize }`, stage `call`, code
`dangling_reference`, cause `absent-target-in-complete-population`, category
refusal. These are typed facade refusal arms; no qsl_eval or private helper
type is exposed. Existing `WrongValueKind`, binding, compile, cancellation
and fault variants retain their behavior. An earlier failure in external
model/population/object admission remains that operation's failure and
does not manufacture a `run` result.

For call-context correspondence, the Admission arm carries the actual
owning `ForeignModelSelection`, `ForeignType` or `ForeignUniverse` refusal,
not a synthesized cause selected from a failed Boolean check. Other
model-admission failures remain the originating admission operation's
outcomes. The command projections below are closed per cause; they do not
add a request shape. `ModelRefusal` itself has no locus member, so these
three projections add no fabricated location. An earlier located
constructor/reader/admission failure retains its location in its own
operation; a lookup refusal arising during evaluation retains FR-096's
actual lookup locus instead of becoming one of these call-stage errors.

This is a replacement of the existing public signature, not an additional
entry. Population arguments carry only `Value::Population(PopulationId)`;
FR-084 admission produces the binding and FR-089 records its identity in
the borrowed context. Reference arguments carry
`Value::Reference(ObjectReference)` with the original universe,
most-specific effective type and object identity. The facade SHALL NOT
mint either identity, copy an environment, construct a trusted effective
view, or expose an evaluator API.

The context's object closure SHALL come from actual model-aware admission
against the selected model and checked type environment. Its population
bindings SHALL come from `admit_binding` or `admit_invocation` against the
actual normalized selected domain package. Registration uses each binding's
own `population_id`; an unequal binding collision retains the existing
`PopulationConflict` failure. A caller declaration that these inputs are
trusted is insufficient. The call boundary SHALL check correspondence to
the compiled model selections, registered population, declared maximum,
declared object type and actual universe before evaluation, including when
the function never reads an argument. It SHALL retain each owning typed
admission failure, its cause and available fields/locus.

Object-world closure and query population membership are different scopes.
A Reference argument must resolve in the admitted object world; it need
not belong to the selected query population. A real object outside that
population reaches `lookup` and its absence mode. An unresolved object
refuses admission as `dangling_reference` /
`absent-target-in-complete-population`, with its declared parameter position,
rather than as a lookup absence or an internal fault. An unregistered
PopulationId or mismatched declared maximum retains `WrongValueKind` and
its position under FR-089. Wrong-kind input retains the same refusal;
foreign model/universe input retains the owning `foreign_reference` cause
and actual expected/supplied identities. Malformed identities retain their
own construction/reader refusal before a typed value can be supplied.
`with_unresolved_references` SHALL NOT admit a model call here.

The input guard SHALL NOT reinterpret other Value variants as one of these
four admitted kinds. Binding-name checks run in full before value checks,
in the existing order: unknown/duplicate arguments, missing parameters,
then admission in declared parameter order. For each parameter the call
boundary SHALL check, in order: value shape, actual selected-model/universe
correspondence, selected-model type conformance, then complete-object-world
resolution. If a check fails, then the boundary SHALL stop admission with
its owning typed cause and SHALL NOT evaluate. Population handle resolution and declared-maximum
checks retain FR-089's `WrongValueKind` guard: an unregistered handle
provides no binding on which to perform model correspondence. A registered
binding must have the parameter's declared maximum and belong to the
actual compiled model selection before it can be used. These checks apply
even to unused parameters and do not replace the lookup's later membership
and absence decision.

When admitting a declared `Reference<T>` parameter, the boundary SHALL
admit a supplied Reference with most-specific type `S` exactly when `S`
and `T` are admitted object
types in the actual selected-model type environment and `S` conforms to
`T`: equality or a declared supertype path from `S` to `T` (QSpec FR-151).
The boundary SHALL use the owning shared model-aware
`quire_semantic_value::declaration::TypeEnvironment::admits`/`conforms`
semantics on that real admitted environment, not a caller-asserted ancestry
graph. A genuine proper subtype is admitted without retagging, constructing
an upcast reference or changing any component of its most-specific identity
triple. An in-model unrelated type refuses `WrongValueKind` at that
parameter's position; equal shape does not establish conformance.

The kernel `quire_exact::ValueType::admits` still requires exact effective
type equality for Reference values. This amendment does not change that
kernel API or claim it admits proper subtypes. The existing native call
validator's use of that exact predicate for References must be adapted at
the model-aware call admission boundary, using the authoritative shared
types, not worked around by retagging a value or by a second evaluator.
If a Reference is both foreign-universe and nonconforming, then the actual
universe-correspondence refusal SHALL win before type conformance:
`foreign_reference` / `foreign-universe`, carrying its real required and
supplied bytes. A foreign model selection likewise retains its actual
`foreign-model-selection` cause before in-model conformance is considered.
Neither case becomes a generic type-equality or dangling refusal.

Integer assignments
`0` and `1` to Boolean parameters retain their canonical false/true meaning;
any other integer refuses. A typed Boolean binds a Boolean parameter
directly. Integer parameters retain exact integer/domain admission.

### File request encoding

The native-run/1 argument encoding below remains a signed-i64 assignment
encoding. Its reader converts those assignments to the shared values for
the same public entry. It carries no Population payload or model-call
context member. This amendment defines no alternative request dialect,
compatibility entry or caller-minted population handle. Typed TC-792
execution is an in-process public call; admitting an I05 envelope's model
inputs remains the owning strict reader/admission operation. A file/driver
adapter cannot claim typed-input support without that consumer qualification.

A native-run/1 request (FR-026's closed envelope and shared limits). For a
`1-draft` program the request carries:

- `program`: the program's source selection (file and the two [FR-001](FR-001-read-exact-source.md) labels, `document` and
  `formal_revision`), with no `clauses` and no `extraction`.
- `models`: zero or more `semantic-ir/2.0.0` domain package documents, each
  read and handed to spine `compile` as FR-056's
  package input, exactly as FR-027 does for compile.
- `libraries`: zero or more `{identity, source}` objects, read and
  handed to spine `compile` as its dependency input, exactly as FR-027 does
  for compile ([FR-099](FR-099-compile-against-supplied-libraries.md),
  ADR-015 D-1).
- `call`: a closed object naming the call.
  - `function`: the called function's name, a string of one or more
    segments separated by `.`, the separator source paths use. Each segment
    is an identifier: an ASCII letter or `_`, then ASCII letters, digits or
    `_`.
  - `arguments`: an array of `{parameter, value}` objects, one per
    parameter, in any order. `parameter` is the parameter's declared name.
    `value` is a JSON integer in the signed 64-bit range: the canonical
    integer assignment FR-098 replays, read by the same rule. An integer
    type's parameter takes the integer. A `Boolean` parameter takes `0`
    (`false`) or `1` (`true`).
  - `work_units`: optional, a JSON integer from 0 to 18446744073709551615
    (`u64::MAX`). It is the S6a `quire.value.accounting/v1` `work_units`
    limit of the call. An omitted `work_units` is 1,000,000. The call's
    other nine accounting counters are each `u64::MAX`.

A `0-draft` request carries FR-026's members and no `call` or `libraries`.

## Outputs

### Typed result and outcome document

The public entry SHALL admit declared Boolean, Integer/Int and Reference
results. Other result kinds retain `unsupported_construct` before a call.
A completed Reference SHALL retain the actual returned identity triple in
`CallValue::Reference`, without dereferencing, re-hashing or narrowing it.
An unexpected completed kind remains an internal invariant failure.

Where a completed Reference is serialized in the existing function outcome
document, its value SHALL reuse the existing canonical typed Reference form:
`{"kind":"reference","universe":U,"type":T,"object_identity":O}`.
`U` and `T` are the actual UniverseId and most-specific EffectiveId, each
64 lowercase hexadecimal characters; `O` is the exact nonempty UTF-8 object
identity. No population binding or object fields are serialized as a
Reference result. This does not add a Population argument wire encoding.

For a call that reaches S6a, stdout is one `spine-run-result/1` JSON
document, newline-terminated, and nothing else.

`spine-run-result/1` is the current output of QSL's `quire-spec` CLI. It is
retired with that CLI when the driver's verbs land (ADR-029 CB-1). The
outcome document of the `execute` operation is FR-286's `quire-outcome/1`,
whose `result` member carries the completed value, undefined reason or
exhausted limit this document's `outcome` member carries. Its members are:

- `format`: `"spine-run-result/1"`.
- `package_id`: the compiled package's `package_id`, lowercase hex, equal
  to the one the FR-278 composition computes for the same source and
  inputs.
- `source`: the program source as native-run-result/1 renders one: its two
  FR-001 labels, its `sha256:` source digest and its authored path.
- `function`: the `function` string of the request.
- `outcome`: exactly one of
  - `{"kind": "completed", "value": V}`, where `V` is
    `{"kind": "boolean", "value": true|false}` or
    `{"kind": "integer", "decimal": "<ASCII decimal>"}`, the integer written
    exactly, with a leading `-` when negative and no other sign, leading
    zero or exponent ([FR-038](FR-038-encode-exact-protocol-numbers.md)'s
    integer spelling, at any magnitude), or the Reference form above;
  - `{"kind": "refused", ...}`, with the members the refusal table below
    states;
  - `{"kind": "undefined", "reason": "<reason>"}`;
  - `{"kind": "incomplete", "limit": "<accounting counter>"}`, the
    exhausted counter's `quire.value.accounting/v1` member name.

The command renders every S6a outcome through one total mapping to this
document and an FR-301 exit status. Each outcome's category is its
ADR-013 O-16 category, except the kernel `Refusal::CheckedInvariant`, which
O-16 lists as a refusal and which is an internal failure by
[FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
(ADR-013 T-4 `InternalFault`).

| S6a outcome | `outcome` | Exit |
|-------------|-----------|------|
| `Outcome::Completed(v)` | `completed`, `value` from `v` | 0, whatever value it completes |
| `FamilyResult::Refused(c)` for which FR-096 builds a record, or `Outcome::Refused(r)` other than `CheckedInvariant` | `refused`, as the refusal table's record row | the record code's exit status (below) |
| `FamilyResult::Refused(c)`, for which FR-096 builds no record | `refused`, as the refusal table's family row | `c`'s code's exit status (below) |
| `Outcome::Refused(Refusal::CheckedInvariant)`, or `CheckedPackage::call` returning `CallFailure::Fault` | none: an internal failure (below) | the internal-failure exit status (below) |
| `Outcome::Undefined(u)`, kernel reason `u` | `undefined`, `reason` from the undefined table below | 10 |
| `FamilyResult::Undefined(u)` | `undefined`, `reason` the catalog's `UndefinedReason` spelling (`precondition-false`, `absent-key`) | 10 |
| `Outcome::Incomplete(i)` | `incomplete`, `limit` `i`'s counter | 22 |

The refused outcome's members:

| Refusal | Members |
|---------|---------|
| Record: FR-096 builds a `RefusalRecord` for the call's `Evaluation` (`Evaluation::refusal_record`: the family cause's `refusal_record`, or `kernel_refusal_record`) | `code` and `cause`, the record's `CatalogCode`; `fields`, an object holding each of the record's catalog fields as a JSON string, the field's FR-096 rendering; `locus`, when the record carries one; `location`, when `Evaluation.location` is present |
| Family, no record: a family cause for which FR-096 builds no record (it has no FR-096 key-table row) | `code` and `cause`, the cause's `catalog_code()`; no `fields` member; `location`, when `Evaluation.location` is present |

- `locus` renders the record's `Locus::Region` (the only locus S6a builds,
  FR-096): `{"file": "<authored path of the region's source>", "span":
  S}`, where `S` is the region's span in the
  native-run-result/1 span form (`start` and `end`, each `byte`, `line` and
  `column`). The byte offsets are the region's. The line and column are
  computed over the bytes of the `Source` whose reference equals the
  region's reference: the program's source or one supplied library's,
  whichever the refusal arose in. A locus naming no supplied source is an
  internal failure (below).
- `location` renders `Evaluation.location`, the `quire_semantic_value::location::Location` the
  outcome arose at: `{"origin": O, "path": [<child index>, ...]}`, where `O`
  is `{"kind": "body", "function": "<name>", "index": <declaration index>}`,
  `{"kind": "measure", "function": "<name>", "index": <declaration
  index>}`, `{"kind": "expression"}` or `{"kind": "type-declaration",
  "name": "<name>"}`.
- A record's or cause's exit status is FR-285's exit code of the O-16
  category of the record's or cause's typed code: 20 for a refusal, 21 for a
  profile-gated unsupported construct and 22 for an exhausted resource.

In the catalog (QSpec `native-diagnostics.md`,
`quire.native.diagnostics/v1`), `kernel_refusal_record` builds a record for
every kernel refusal but `CheckedInvariant`, with the code, causes and fields
of FR-096's key table, and each exits 20:

| Kernel refusal | Code | Cause | Fields |
|----------------|------|-------|--------|
| `CardinalityOutOfBound` | `cardinality_out_of_bound` | `below-minimum` or `above-maximum` | `collection`, `bound`, `count` |
| `ForeignReference` | `foreign_reference` | `foreign-universe` | `required`, `supplied`, each lowercase hex |
| `InexactDecimal` | `inexact_decimal` | `nonzero-discarded-digit` | `expected`: the target type's declared domain |
| `DecimalOutOfDomain` | `decimal_out_of_domain` | `outside-domain` | `expected`: the target `Decimal[..]` domain |
| `DivisionOutOfDomain` | `division_out_of_domain` | `quotient-outside-domain` or `remainder-outside-domain`, from the variant's exposed member | `expected`: the consumer's integer domain |
| `ModuloOutOfDomain` | `modulo_out_of_domain` | `outside-domain` | `expected`: the consumer's integer domain |
| `TextLengthOutOfDomain` | `text_length_out_of_domain` | `outside-domain` | `expected`: the declared `Text[..]` bounds and profile |
| `IntegerOutOfDomain` | `integer_out_of_domain` | `outside-domain` | `expected`: the target `Int[..]` domain |
| `RationalOutOfDomain` | `rational_out_of_domain` | `outside-domain` | `expected`: the result `Rational[..]` domain |
| `IeeeNotExact` | `ieee_not_exact` | `rounding-required` | `expected`: the target IEEE width; `flags`: the variant's would-be flags |
| `IeeeNanPayloadNotRepresentable` | `ieee_nan_payload_not_representable` | `payload-exceeds-target` | `expected`: the target IEEE width; `actual`: the source IEEE width |
| `IeeeRationalOutOfDomain` | `ieee_rational_out_of_domain` | `outside-domain` | `expected`: the target `Rational[..]` domain |

Each field value is the catalog's exact ASCII spelling: a domain as its type
name and bounds, an IEEE width as `binary32` or `binary64`, and a flag set as
its flag names in the order `invalid`, `divide_by_zero`, `overflow`,
`underflow`, `inexact`, joined by `,`. For `ForeignReference`, `required` is
the universe already in force and `supplied` the one tested against it: for
the equality raise site (`quire-exact`'s `plan_pairs(left, right)`), the left
operand's universe and the right's; for the membership raise site
(`member_equal_stop(candidate, member, ..)`, both `x in c` and collection
construction's dedup), the already-admitted member's or collection's own
universe and the probed candidate's. `CheckedInvariant` is an internal
failure (below).

The kernel defines no spelling method for `Undefined`, and FR-096 spells no
kernel undefined reason, so FR-100 spells them in kebab case:

| Kernel reason | Spelling |
|---------------|----------|
| `Undefined::DivisionByZero` | `division-by-zero` |
| `Undefined::IeeeNotFinite` | `ieee-not-finite` |
| `Undefined::EmptyReduction` | `empty-reduction` |
| `Undefined::NoneValue` | `none-value` |
| `Undefined::SumOutOfDomain` | `sum-out-of-domain`: a `sum<N>` seed or running total outside `N`'s domain, including the seed `0` of an empty `sum` |

`sum-out-of-domain` is a `sum<N>` seed or running total that is not a member
of `N`'s domain. By QSpec FR-145 it is a located undefined outcome, since a
running total is not the expression's result. A failing seed summand is
located at the summand's node, and a failing addition's running total at
the `sum` node (FR-096). It makes no later charge and exposes no total. The
final total is the last running total, so a final total outside `N`'s domain
is the same outcome.

An empty `sum` (`n = 0`) has the seed `0` as its only running total. When
`N`'s domain does not admit `0`, the outcome is `sum-out-of-domain`, located
at the `sum` node. QSpec FR-145 step 1 requires `N`'s domain to admit `0` and
states no outcome when it does not; QSL fixes that outcome this way.

`run` never yields `sum-out-of-domain`. Linked checking proves a `sum`'s
range over every running total, so a program whose `sum` could leave `N`'s
domain refuses at `check` with `unproved-range` (FR-096). The row keeps the
outcome mapping total. Only an expression checked under `CheckMode::Kernel`
and evaluated at S6a reaches it (FR-100-AC-10).

### Internal failure at S6a

A call whose outcome is the kernel `Refusal::CheckedInvariant`, for which
`CheckedPackage::call` returns `CallFailure::Fault(fault)`, or whose
record's locus names a region no supplied source's reference equals, writes
nothing
to stdout. It writes the command-error envelope (FR-026's
`native-run-result/1` until the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`), FR-267's `native-run-result/2` from it)
to stderr with stage `call`, code `runtime_invariant` and `details`
`{"stage": "<fault stage>", "invariant": "<fault invariant>"}`, the
`InternalFault`'s stable identifiers. For `CheckedInvariant` they are stage
`S6a` and invariant `checked-program-invariant`; for `CallFailure::Fault`,
the fault's own; for a locus naming no supplied source, stage `spine-run`
and invariant `locus-source-supplied`. It exits 30, the tool-failure status of QSpec FR-301's exit
contract (`0` completed without violation, `10` logical violation, `20`
invalid/refused input, `21` unsupported, `22` incomplete and `30` tool
failure). A checked-program invariant failing is a tool failure, so this
path exits 30, FR-285's exit code for internal failure.

### Refusals before S6a

Every refusal before S6a writes nothing to stdout. It writes the
command-error envelope (FR-026's `native-run-result/1` until the change that lands FR-100's clause runner (FR-312's reader plus `run_clause`),
FR-267's `native-run-result/2` from it) to stderr, carrying a stage, a
catalog code, a `message` and a `details` object, and
exits with that code's exit status:

| Refusal | Stage | Code | `details` | Exit |
|---------|-------|------|-----------|------|
| A declared edition other than `0-draft` or `1-draft` | `profile` | `unknown_edition` | as FR-027 | 20 |
| A `1-draft` request carrying `selection`, `snapshots`, `invocations`, `package`, a set `validation_work` or `expression_steps`, `clauses`, `extraction`, or a model in a format other than `semantic-ir/2.0.0`; or carrying no `call` | `request` | `invalid-request` | as FR-026 | 20 |
| A `0-draft` request carrying `call` or `libraries` | `request` | `invalid-request` | as FR-026 | 20 |
| A `call` that is not the closed object above; an argument `value` that is not a JSON integer in the signed 64-bit range; a `work_units` that is not a JSON integer from 0 to `u64::MAX` | `request` | `invalid-request` | as FR-026 | 20 |
| A spine compile refusal | the refusing spine stage | that stage's cause code | as FR-027 | that code's |
| A `function` that is empty, holds an empty segment or a segment that is not an identifier, has more than one segment, or names no function of the compiled package | `call` | `missing_declaration` | `{"function": "<the function string>"}` | 20 |
| A function whose declared result is neither `Boolean`, an integer type nor `Reference` | `call` | `unsupported_construct` | `{"function": "<the function string>"}` | 21 |
| An argument naming no parameter, a parameter named twice, or a parameter with no argument | `call` | `invalid_runtime_input` | `{"parameter": "<the parameter name>"}` | 20 |
| A value that is not of its parameter's declared type (`WrongValueKind`) | `call` | `invalid_runtime_input` | `{"position": <the parameter's zero-based position>}` | 20 |
| `Admission` carrying `ForeignModelSelection` with `OfferedSelection::Document(actual)` | `call` | `foreign_reference`, cause `foreign-model-selection` | Exactly `{"position": P, "model_identity": actual, "expected": E}`; `actual` is the original document's modelIdentity string and `E` the cause's full required DomainPackageRef | 20 |
| `Admission` carrying `ForeignModelSelection` with `OfferedSelection::Population(actual)` | `call` | `foreign_reference`, cause `foreign-model-selection` | Exactly `{"position": P, "population": D, "expected": E}`; `D` is the actual offered population DeclarationKey, not a substitute model selection; `E` the cause's full required DomainPackageRef | 20 |
| `Admission` carrying `ForeignType { member, type_name }` | `call` | `foreign_reference`, cause `foreign-type` | Exactly `{"position": P, "member": member, "type": D}`; `D` is the cause's actual uncovered/absent type DeclarationKey | 20 |
| `Admission` carrying `ForeignUniverse { actual, expected }` | `call` | `foreign_reference`, cause `foreign-universe` | Exactly `{"position": P, "required": H, "supplied": A}`; `H` is the actual expected UniverseId as lowercase hex, `A` the actual supplied bytes as lowercase hex, unchanged and unpadded, matching FR-096's cause fields | 20 |
| `DanglingReference { position }` | `call` | `dangling_reference`, cause `absent-target-in-complete-population` | Exactly `{"position": P}`; no lookup binding/key or invented locus | 20 |

In these rows, `P` is the declared parameter's zero-based position. `D`
uses the existing DeclarationKey shape `{"package": package, "node": node}`
with the cause's exact strings. `E` uses the existing DomainPackageRef
shape `{"identity": identity, "version": version,
"digest_domain": "sha256-jcs", "digest": digest}`, with the cause's exact
identity/version and its actual 32-byte digest as 64 lowercase hex
characters. Each row preserves the original `ModelRefusal.detail` as the
message. No row reads expected/supplied values from prose, replaces an
offered population key by a guessed package, uses a fallback cause, or
adds a top-level locus outside FR-267's closed command-error envelope.
The underlying typed Admission still retains the full original refusal.

An unsupported result type exits 21, where FR-098's predicate replay refuses
a non-`Boolean` selection as `NotAPredicate` (`invalid_runtime_input`, 20).
The difference is deliberate: predicate replay requires a predicate, so asking
it to replay a non-`Boolean` selection is a bad request (a non-`Boolean`
function's claim is replayed by FR-357's value-parity entry); run calls any function, and a
result kind the outcome document cannot express is an unsupported construct,
not bad input.

Broken pipes end quietly; other write and output-serialization failures
exit 30.

## Behavior

- The run command shall read the program source's declared edition once,
  from its header, before any model or
  library source is read, with FR-027's edition reader. Each program source
  goes to exactly one runner.
- If the program declares an edition other than `0-draft` or `1-draft`,
  then the run command shall refuse with `unknown_edition` at the edition
  literal, naming the file and the edition.
- If the program declares `0-draft` or no edition, then the run command shall run the program through FR-026's native run.
- If the program declares `0-draft` or no edition and the request carries `call` or `libraries`, then the run command shall refuse the request with `invalid-request`.
- If the program declares `1-draft`, then the run command shall refuse a
  request carrying a member the refusal table names, or carrying neither
  `call` nor `clause`, with `invalid-request` before it reads any model or
  library file.
- From the change that lands FR-100's clause runner (FR-312's reader plus
  `run_clause`, ADR-031 R-1), if the program declares `1-draft` and the
  request carries `clause`, then the run command shall run it as a clause
  run ([FR-312](FR-312-read-a-clause-run-request-and-run-it.md)).
- Until that change, if the program declares `1-draft` and the request
  carries `clause`, then the run command shall refuse it with
  `invalid-request` at stage `request`, naming `clause`.
- If a `call` member, an argument `value` or `work_units` is outside the
  shape Inputs states, then the run command shall refuse with
  `invalid-request` at stage `request`.
- The run command shall hand the `1-draft` source, its domain packages, its
  dependency input, the default spine stage limits, the `call` and its
  accounting limits to `qsl_replay::spine::run`, and render its result by
  the outcome mapping above.
- `qsl_replay::spine::run` shall compile the source with
  the FR-278 composition and carry a compile refusal unchanged, with
  its stage and cause code.
- If `function` is empty, holds an empty segment or a non-identifier
  segment, or has more than one segment, then the spine run entry shall
  refuse with `missing_declaration` at stage `call`.
- `qsl_replay::spine::run` shall resolve a one-segment `function` by name
  lookup in the compiled package's declarations, as `qsl_replay::replay`
  does (FR-098, OQ-5).
- If the name resolves to no function, then `qsl_replay::spine::run` shall
  refuse with `missing_declaration` at stage `call`.
- If the function's declared result is neither `Boolean`, an integer
  type nor `Reference`, then `qsl_replay::spine::run` shall refuse with
  `unsupported_construct` at stage `call` before any call or charge.
- `qsl_replay::spine::run` shall join each argument to the parameter of the
  same declared name, and order the values by declared parameter position,
  whatever order they arrive in.
- If an argument names no parameter, names a parameter already bound, or
  leaves a parameter unbound, then `qsl_replay::spine::run` shall refuse
  with `invalid_runtime_input` at stage `call`, naming the parameter.
- `qsl_replay::spine::run` SHALL bind shared typed values under the Typed
  public call rules above. The signed-i64 file assignments retain FR-098's
  canonical integer/Boolean conversion and its existing refusal oracles.
- If a value is outside its parameter's declared domain (`12` for
  `Int[0, 9]`), then `qsl_replay::spine::run` shall refuse `WrongValueKind`
  at S6a admission, naming the parameter's position.
- `qsl_replay::spine::run` shall call the function through
  `CheckedPackage::call` under the given accounting limits and return the
  compiled package's `package_id` with the call's outcome, converted to the
  `outcome` member by the mapping above, a refusal carrying the FR-096
  record the call's `Evaluation` builds, if any, the family cause's
  `catalog_code()` otherwise, and `Evaluation.location`.
- If the call's outcome is the kernel `Refusal::CheckedInvariant`, then
  `qsl_replay::spine::run` shall return an FR-096 `InternalFault` with stage
  `S6a` and invariant `checked-program-invariant`.
- If `CheckedPackage::call` returns `CallFailure::Fault`, then
  `qsl_replay::spine::run` shall return that `InternalFault`.
- If a record's locus names a region whose reference equals no supplied
  source's, then `qsl_replay::spine::run` shall return an `InternalFault`
  with stage `spine-run` and invariant `locus-source-supplied`.
- The `qsl_replay` crate shall name no `qsl_eval` path in a public item of
  `qsl_replay::spine` or in a re-export. The types `qsl_replay::spine::run`
  takes and returns are `qsl_replay`, `qsl_foundation`, `qsl_semantics` or
  `quire_exact` types.
- The entry SHALL pass the borrowed admitted context to the existing S6a
  call exactly once after admission. It SHALL preserve FR-301's enclosing
  family result, StateModel catalog attribution, fields and location for a
  model refusal. It SHALL NOT use a default context for a typed model call,
  precompute lookup as a facade substitute, or use a second evaluator.
- The entry SHALL retain the caller's original Cancel through each
  applicable phase. Requested and Deadline remain distinct, category
  incomplete, exit 22, with no invented cancel cause or evaluated charge.
  Population admission limits, supplied admission/conversion limits and
  semantic evaluation limits retain their existing owners and units.
  Pre-call failure consumes no evaluation work. QSL-681's proposed A/H
  contract requires its owning settlement and IR-714 public helper
  qualification; no default, private helper or full-Tree qualification is
  inferred by this amendment.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-100-AC-1 | A native-run/1 request selecting `tests/fixtures/spine-compile.native` (`edition "1-draft"`) with `call` `{"function": "seven", "arguments": []}` exits 0, with empty stderr, and writes one newline-terminated document whose `format` is `spine-run-result/1`, whose `package_id` equals the one the FR-278 composition computes for the same source, whose `source` names the request's two labels, source digest and authored path, whose `function` is `seven`, and whose `outcome` is `{"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}`. | Test |
| FR-100-AC-2 | A `0-draft` native-run/1 request writes the stdout bytes and exit status TC-103 and TC-104 fix for it. A program declaring `edition "7-draft"` refuses `unknown_edition` at stage `profile`, exit 20, empty stdout, with a `message` naming the file and the edition. | Test |
| FR-100-AC-3 | A `1-draft` request carrying each member the refusal table names refuses `invalid-request` at stage `request`, exit 20, empty stdout, whatever state its model files are in; a `1-draft` request carrying neither `call` nor `clause` refuses the same way; until the change that lands FR-100's clause runner, a `1-draft` request carrying `clause` refuses the same way; a `0-draft` request carrying `call` or `libraries` refuses the same way. A `work_units` of `18446744073709551616`, `-1` or `1.5` refuses the same way. A `1-draft` request whose `libraries` supplies an imported library and whose `models` supplies a domain package the program selects runs, exit 0. | Test |
| FR-100-AC-4 | For a unit declaring `lt(a: Int[0, 9], b: Int[0, 9]): Boolean { a < b }`, `flag(b: Boolean): Boolean { b }`, `id(x: Int[0, 9]): Int[0, 9] { x }` and `px(p: Point): Digit`: `lt` with `b = 3` given before `a = 5` completes `false`, exit 0; `flag(1)` completes `true`; `id(4)` completes integer `"4"`. `flag(2)`, `id(12)` and `px(1)` each refuse `invalid_runtime_input` at stage `call`, exit 20, empty stdout, with `details` `{"position": 0}`. An argument naming `y`, `x` bound twice, and no argument each refuse `invalid_runtime_input` at stage `call`, with `details` `{"parameter": "y"}`, `{"parameter": "x"}` and `{"parameter": "x"}`. A `value` of `true`, `"7"`, `1.5` or `9223372036854775808` refuses `invalid-request` at stage `request`, exit 20. | Test |
| FR-100-AC-5 | `function` `nope`, `module.seven`, `""`, `seven.`, and `7x` each refuse `missing_declaration` at stage `call`, exit 20, empty stdout, with `details` `{"function": <that string>}`. A function whose declared result is a record refuses `unsupported_construct` at stage `call`, exit 21, before any call. A `1-draft` source with a syntax error refuses at stage `source` (`invalid_syntax`), and one declaring `inv(x: Int[0, 9]): Boolean { 1 / x > 0 }` refuses at stage `check` with `ill_typed` (integer `/` with no `Rational` expected type); each exits 20 with empty stdout (FR-027-AC-8). | Test |
| FR-100-AC-6 | `seven` with `work_units` 0 writes outcome `{"kind": "incomplete", "limit": "work_units"}`, exit 22. | Test |
| FR-100-AC-7 | `qsl_replay::spine::run` called directly over each AC-1, AC-4, AC-5 and AC-6 input returns the same `package_id`, outcome category, value, code, reason, counter, and parameter name or position the CLI renders. The root crate names `qsl-eval` in no dependency table (TC-390). | Test |
| FR-100-AC-8 | No public item of `qsl_replay::spine`, and no `qsl_replay` re-export, names a `qsl_eval` path; a `pub use` of a `qsl_eval` item from `qsl_replay`, or a `qsl_eval` type in `spine::run`'s signature, fails the check. | Test |
| FR-100-AC-9 | The outcome mapping converts a constructed `Outcome::Completed` of each value kind, `Outcome::Refused` of each of the thirteen kernel refusals, `Outcome::Undefined` of each of the five kernel reasons, `Outcome::Incomplete`, `FamilyResult::Refused` with and without an FR-096 key-table row, `FamilyResult::Undefined` of each family reason, and a `CallFailure::Fault` into the `outcome` member and exit status the mapping tables state: each of the twelve kernel refusals other than `CheckedInvariant` renders its record's code, cause, fields (JSON strings) and locus, as the kernel-record table states, exit 20; a family cause with a record renders the same members; a family cause without a record renders its `catalog_code()` with no `fields`, exiting by that code (`AncestorSteps`, `resource_exhausted`, exits 22); and `CheckedInvariant` and `CallFailure::Fault` are `runtime_invariant` command errors with their stage and invariant in `details`, at the internal-failure exit status. | Test |
| FR-100-AC-10 | With `type Pos = Int[1, 9]` checked under `CheckMode::Kernel`, S6a evaluation of `sum<Pos>(x in q: x)` for an empty `q` of `Sequence<Int[1, 9]>[0, 2]` returns `Outcome::Undefined(Undefined::SumOutOfDomain)` located at the `sum` node, and the outcome mapping renders it `{"kind": "undefined", "reason": "sum-out-of-domain"}`, exit 10. The same `sum` for `q` holding `4` completes with `4`. | Test |
| FR-100-AC-11 | Every exit status `run` returns equals FR-285's exit code of the outcome's O-16 category: FR-100-AC-10's `sum-out-of-domain` outcome and a `FamilyResult::Undefined` with reason `precondition-false` each write outcome kind `undefined` and exit 10; FR-100-AC-4's `invalid_runtime_input` refusals exit 20; FR-100-AC-5's record-result function exits 21; FR-100-AC-6 exits 22; a `CallFailure::Fault` exits 30. The `execute` operation (FR-279) over each of these inputs returns an outcome whose FR-285 exit code equals the command's. | Test |
| FR-100-AC-12 | `qsl_replay::spine::run` over FR-100-AC-1's `seven`, with a `Cancel` that trips during the run and not before it, returns `RunRefusal::Cancelled` carrying the handle's cause, category incomplete, exit 22, and never `RunRefusal::Fault`: tripped with `Requested` at the last front-end charge (before S6a starts) and tripped with `Deadline` at the first S6a charge (inside `CheckedPackage::call`), each gives `Cancelled` with that cause. `OutcomeDocument::from_run` over each writes `category` incomplete, `last_stage` `null` and one `cancelled` diagnostic whose cause is `requested` and `deadline` respectively. | Test |
| FR-100-AC-14 | TC-792's exact source-compiled Value lookup with real admitted Population/Reference inputs reaches S6a through `spine::run`; a reference resolving in the object world but absent from the selected population produces the actual enclosing `FamilyResult::Refused`, StateModel `absent-key` cause, refusal category and exit 20, with the owning fields and source location. Compile/select/admit/unsupported-result failures do not satisfy this criterion. | Test |
| FR-100-AC-15 | With the same compiled lookup and model, selecting a population containing that reference completes with `CallValue::Reference` whose universe, most-specific type and object identity equal the supplied reference, exit 0. The Reference result encoding retains all three components exactly. | Test |
| FR-100-AC-16 | Independently supply a wrong-kind argument, an unregistered PopulationId, a binding whose declared maximum differs from the parameter's, a foreign selected model/universe, and a dangling Reference. Each refuses before evaluation with its owning typed cause and parameter/identity fields. TC-792's actual selected-model exact type and genuine proper subtype both pass Reference admission, retaining the most-specific identity; an in-model unrelated type refuses WrongValueKind. Combined foreign-universe plus nonconforming type retains the foreign-universe cause before conformance. Unknown, duplicate and missing parameter failures retain precedence over value admission, and declared parameter order selects the first failing input. A function ignoring its arguments cannot bypass these checks. | Test |
| FR-100-AC-17 | Malformed identity bytes rejected by the actual constructor/reader never become supplied typed references. A well-formed real object absent only from the query population reaches the AC-14 lookup refusal; an object absent from the complete object world produces dangling-reference admission instead. An unresolved-reference context cannot turn the latter into AC-14. | Test |
| FR-100-AC-18 | The scalar inputs, values, refusals, category/exit mapping and internal-fault oracles in AC-1 through AC-12 remain equal after adapting callers to the shared Value field and explicit context. All ten actual accounting limits reach evaluation unchanged. Requested and Deadline stops remain distinct before and during S6a; applicable pre-call denial has zero evaluation consumption. | Test |
| FR-100-AC-19 | Public signatures use the carriers in Typed public call and no qsl_eval path. The source, binding and object context are admitted through their actual owner APIs, with no copied/trusted substitute model view or second engine. The driver executes TC-792 through this reviewed public boundary and retains the absent/present and invalid-admission distinctions. | Test |

The typed-call acceptance criteria supplement all existing scalar oracles;
FR-100-AC-13 remains reserved to the QSL-656 accounting amendment.

Verification uses execution-producing Test evidence for AC-19 together
with inspection of the public signatures and actual owning admission and
engine paths; inspection alone cannot discharge its driver-execution
obligation. The existing test-case associations are unchanged: AC-1–3
use TC-450; AC-4–6 TC-451; AC-7–9 and AC-12 TC-452; AC-10–11 TC-786;
AC-14–19 TC-792. These associations do not replace executable trace tags.

## Dependencies

- [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md),
  [FR-089](FR-089-carry-population-identity-across-the-kernel-boundary.md)
  and [FR-301](FR-301-render-state-model-causes-under-the-enclosing-family.md)
  own admission, the shared Population carrier and StateModel attribution.
  [I05](ix://agent-ix/quire-specification/interface_005) owns the paired
  native-runtime interface contract.
- [FR-026](FR-026-run-standalone-native-workflow.md): native-run/1, its
  intake limits and its command-error envelope; the `0-draft` route.
- [FR-027](FR-027-export-compiled-native-package.md): the edition reader,
  domain package and `libraries` intake, and spine compile refusals.
- [FR-098](FR-098-execute-a-replay-request.md): name lookup and the canonical
  integer argument rule.
- [FR-099](FR-099-compile-against-supplied-libraries.md): the dependency
  input.
- [FR-001](FR-001-read-exact-source.md): the two source labels.
- [FR-038](FR-038-encode-exact-protocol-numbers.md): the integer decimal
  spelling.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §5 and OQ-1: spine `run` calls a named checked function.
- ADR-013 O-16: the outcome categories.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md):
  the refusal record a refused outcome renders, and `CheckedInvariant` as an
  `InternalFault`.
- QSpec FR-301: the six exit codes.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit
  function, with undefined at 10 (ADR-029 CB-4).
- [FR-279](FR-279-execute-a-checked-entry-as-a-library-operation.md),
  [FR-287](FR-287-reach-qsl-through-the-driver-cli.md): `execute` and the
  driver CLI.
- QSpec `native-diagnostics.md`, `quire.native.diagnostics/v1` revision
  `1-draft.8`: the kernel refusal codes, causes and fields.
- QSpec FR-145: a `sum` seed or running total outside `N`'s domain is a
  located undefined outcome.

## Status

The typed-public-call amendment is proposed for independent review under
QSL-665. Its source, admission, result and TC-792 consumer qualification are
UNRUN. Existing implementation statements below describe the scalar entry
only. QSL-656's reviewed accounting amendment is a source-history
prerequisite: its closed `accounting` object replaces the legacy file
`work_units` member, with no compatibility member. The accounting owner
retains that amendment and TC-951; this branch does not duplicate it.
Source consumers must reconcile that exact amendment before implementation
or qualification. No implementation branch is created by this spec.

Remaining work (implementation, with the native-run deletion change, FR-312):
the code still requires a `sha256:` source digest on each source selection
and checks the file against it; both go, and the command reads each named
file's bytes as they are.

Implemented. `qsl_replay::spine::run` and the CLI `run` command render the
outcome mapping, the internal-failure path (`CheckedInvariant`,
`CallFailure::Fault`, an unresolvable locus, each exiting 30 directly), and
locus resolution over the program's and every supplied library's source.
Every kernel refusal but `CheckedInvariant` renders its record (`code`,
`cause`, `fields` and `locus`), and a `sum` seed or running total outside
`N`'s domain, including an empty sum whose `N` does not admit `0`, is
`Undefined::SumOutOfDomain` rather than `IntegerOutOfDomain`.

The conversion of `CheckedInvariant` to an `InternalFault` is
`qsl_replay::spine::run`'s, since the evaluator still returns it as a
refusal (FR-096 Status).

Every exit status is FR-285's `Category::exit_code` of the outcome's
category, and an undefined outcome exits 10 (FR-100-AC-10, FR-100-AC-11,
TC-786).

Remaining work (implementation, Linear QSL-381): the program selection and the `source`
member still carry the revision namespace and revision; they carry the two
FR-001 labels (QSpec STD-150).
