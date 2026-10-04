---
id: FR-056
title: "Admit domain packages as Quire model declarations"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-002, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: traces_to }
  - { target: ix://agent-ix/quire-specification/AD-006, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-150, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-151, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-152, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-153, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-154, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-208, type: depends_on }
  - { target: ix://agent-ix/filament-core-data/FR-142, type: depends_on }
  - { target: ix://agent-ix/filament-core-data/FR-143, type: depends_on }
---
# FR-056: Admit domain packages as Quire model declarations

## Description

When a native package imports a domain package, the compiler SHALL admit that
one domain package under Quire specification FR-154 and SHALL build one Quire
model declaration for each IR node of its Semantic IR 2.0.0 document.

The domain model is declared in spec artifacts (AD-006). Each domain object is
one artifact whose object type, such as `entity`, comes from a module. Intake is
one path:

```text
spec artifact bundle
  -> quire_rs::semantic::extract_semantic with the modules' real manifests
  -> agent-ix-extraction-frontend lift (RFC 8785 JCS bytes, sha256-jcs digest)
  -> domain package bytes
  -> intake seam: agent-ix-semantic-ir read, FR-154 admission, declarations
  -> model linker (FR-036)
```

quire-rs is the only artifact parser, filament-core-data owns the lowering and
the IR reader, and the Quire specification owns model meaning and the Quire meaning
ids (FR-208). Generated Rust, TypeScript and Python types and Quire model
declarations come from one fingerprinted lowering of the same artifacts.

## Inputs

- One ModelSelection (Quire specification FR-321): the source `model`
  declaration's domain package identity and `sha256-jcs` digest. Each
  selection names exactly one domain package.
- The package input: a map from `sha256-jcs` digest to domain package bytes.
- The selected `quire.model.complete/v1` definition and the Quire meaning ids
  of Quire specification FR-208.
- `ModelNormalizationLimitsV1` of `quire.value.accounting/v1`.
- For the bundle entry point: the explicit artifact inventory (paths and bytes)
  and the exact module manifests that type it, selected by module identity.
  No filesystem discovery, network retrieval or installed module
  supplies an omitted artifact.

## Outputs

The original declarations of the one ModelSelection, each keyed by (domain
package identity, IR node identity) under digest domain `sha256-jcs`, with its
bound Quire meaning, its export records or FR-152 systems kind, and the source
artifact id and span it came from; or the located refusals and incomplete
result that FR-154 defines. The model linker
([FR-036](FR-036-link-composed-native-packages.md)) consumes only this result.

## Behavior

### Entry points

The compiler SHALL expose an intake seam that takes one ModelSelection, the
package input and limits, and SHALL read the selected bytes only through
`agent-ix-semantic-ir`.

The compiler SHALL expose a bundle entry point that runs
`quire_rs::semantic::extract_semantic` with the selected manifests, lifts the
result with `agent-ix-extraction-frontend`, and passes the lifted bytes to the
intake seam.

### Package document and its digest

The compiler SHALL parse the selected bytes once, and SHALL take the package's
`sha256-jcs` digest as SHA-256 over the RFC 8785 encoding of that one parsed
document, computed by `quire-canonical`.

A document does not parse when its bytes are not UTF-8 or not JSON, begin
with a byte order mark, carry a lone UTF-16 surrogate escape (a
`\uD800`-`\uDFFF` escape that is not a high surrogate followed by a low
surrogate), or repeat a member name within one object (RFC 8785 requires
I-JSON, RFC 7493). RFC 8785 encodes none of these inputs, so such a document
has no `sha256-jcs` digest.

If the selected bytes do not parse, then the compiler SHALL take check 3's
digest over the raw bytes, and SHALL read no package identity for
check 4. Such bytes therefore refuse
`stale_dependency`/`byte-digest-mismatch` under any `sha256-jcs` digest, and
`invalid_model_binding`/`wrong-model-selection` when the selected digest
happens to equal their raw digest. No declaration is admitted.

A number with no finite IEEE 754 double value (such as `1e400`) is not one
of these: the reader refuses it with its RFC 6901 pointer and exact source
text, and the compiler classifies it from that text under the two cases
below. Unless an earlier reader fault decides, it is never a parse failure,
never digested raw, and never refuses `stale_dependency`/`byte-digest-mismatch`;
`[{"a":1,"a":2},1e400]` is digested raw, because the repeated name is the
reader's first refusal. `1e400`, `-1e400` and every other
whole value beyond ±2^53 refuse `inexact-integer`; `1e-400`, which has a
finite double (zero) and is not whole, refuses `inexact-number`. When the bytes carry
several reader faults (a number with no finite double, a repeated member name
the reader detects when the object closes, truncation), the refusal is the
first one `quire-canonical`'s read returns. `{"a":1,"a":2,"n":1e400}` refuses
`noncanonical_wire`/`inexact-integer` at `/n`, and `[1e400` refuses it at
`/0`, while `{"a":1,"a":2,"n":1e-400}` has a repeated name the reader refuses
first, so it is digested raw. A number with no finite double is also named
ahead of an earlier inexact number.

A document that parses can still carry a number with no exact RFC 8785
spelling. RFC 8785 writes every number as the shortest round-trip text of
its nearest IEEE 754 double, ties to even, so the digest of such a document
would be the digest of a different value, shared with every document that
differs from it only in that number. A number has no exact spelling in two
cases, and the compiler SHALL decide both on the number's text, by exact
decimal reasoning over its sign, integer digits, fraction digits and
exponent, in time linear in the text's length:

- It denotes a whole value whose magnitude exceeds 2^53 (9007199254740992),
  however it is spelled: an integer past the 64-bit range such as
  `18446744073709551616`, an exponent form such as `1e20`, or a decimal form
  such as `9.007199254740993e15` (2^53 + 1). This is decided from the text
  alone, never from a parsed double or integer. 9007199254740992 and
  -9007199254740992 are not refused for it, and 9007199254740993 and
  -9007199254740993 are.
- Otherwise, the exact decimal value of its text differs from the exact
  decimal value of the text RFC 8785 writes for its nearest double: the
  shortest round-trip text, which is ECMAScript `Number::toString`'s, and
  where two shortest texts are equally close to the double, the one with the
  even last digit. That is the text quire-canonical writes for the number;
  the compiler takes it from there, not from any other formatter. The two
  texts are compared by their digits and scale, never as doubles. `0.1`,
  `1.0`, `-0` (zero has no sign) and `5e-324` are exact;
  `9007199254740993.5`, `0.1000000000000000000001` and `1e-400`, which
  underflows to zero, are not.

If the selected bytes carry such a number, then the compiler SHALL refuse
the read with `noncanonical_wire` before any `sha256-jcs` digest is
computed, with cause `inexact-integer` for the first case and
`inexact-number` for the second. A number that fits both cases, as
9007199254740993 does, refuses `inexact-integer`; `inexact-number` covers
every other inexact number, and a whole number within ±2^53 is refused
under neither. The refusal carries `document_pointer`: the RFC 6901 pointer
of the first such number in document order, except that a number with no
finite double is named when the reader reaches it, before any tree exists, so
it can be named ahead of an earlier inexact number. Each member name is
escaped (`~` as `~0`, `/` as `~1`) and each array element named by its decimal
index.
The rule covers every number of the document, at any depth. The code is
QSpec FR-271's `noncanonical_wire` (its `## Values` row), and the causes and
`document_pointer` are QSpec FR-272's (the `noncanonical_wire` row of
`## Closed cause variants by code family`, and `document_pointer` in
`## Canonical cause payload contract`). This refusal comes where the
byte-limit refusal does, after
FR-154's check 2 and before its check 3: admission refuses such bytes
`noncanonical_wire` with the same cause and pointer under any selected
digest, never `byte-digest-mismatch` or `wrong-model-selection`, as
quire-contract-ir FR-038-AC-93 refuses a selected model document.

The compiler SHALL encode each number of a document it admits for the digest
as RFC 8785 does. Every such number is exactly the value of its double's
shortest round-trip text, so the digest spells the value the declarations
read: `100`, `1e2` and `100.0` all digest as `100`. Declarations read an
integer exactly as written.

If the selected bytes exceed the intake byte limit (`intake.input_bytes`),
then the compiler SHALL refuse the package with
`resource_exhausted`/`intake-limit-exceeded`, naming the limit, its bound and
its setting. A document of any nesting depth is read through
`quire-canonical`'s shared reader and judged on its content
([FR-260](FR-260-admit-semantic-ir-documents-at-any-depth.md)).
This refusal comes after FR-154's check 2 and before its check 3.

If reading the selected bytes or encoding their digest cannot reserve
memory, then the compiler SHALL refuse the package with
`resource_exhausted`/`allocation-failed`, carrying the size in bytes of the
reservation that failed
([FR-259](FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md)
B6). It names no limit and no setting, and comes where the byte-limit
refusal does.

### Admission

The compiler SHALL apply FR-154's admission checks in FR-154's order before
reading any declaration: `stale_dependency`/`digest-domain-mismatch`,
`missing_import`/`missing-selection`,
`stale_dependency`/`byte-digest-mismatch` and
`invalid_model_binding`/`wrong-model-selection`. Check 3 recomputes the
supplied document's canonical `sha256-jcs` digest and compares it with the
selection's identity; a difference refuses
`stale_dependency`/`byte-digest-mismatch` with the selected and the
recomputed digest (QSpec FR-154 check 3 and the
FR-272 cause table, quire-specification#174 and #176).

If `agent-ix-semantic-ir` refuses the selected bytes, then the compiler SHALL end
intake with no declaration and SHALL retain every reader diagnostic with its IR
node, artifact id and span.

The reader owns resolution of each module-qualified
`kind` through the document's embedded `constructs` table.

If the reader reports an IR node whose `kind` names no entry of the `constructs`
table, then the compiler SHALL refuse that node with
`invalid_model_binding`/`malformed-declaration`, retaining the IR node identity,
the artifact id and the span, reported in FR-154's declaration refusal order:
node order, and within one node every failing check in FR-154's table order,
the object id check first.

### Meaning binding

The compiler SHALL bind each IR type definition and population node to a Quire
meaning by the exact `meaning` id of its construct, as Quire specification
FR-208 lists, and SHALL NOT
select a meaning by kind name, module, shape or members.

If a construct has no `meaning`, or a `meaning` that is not an FR-208 value, then
the compiler SHALL refuse each IR node of that kind with
`invalid_model_binding`/`malformed-declaration` (FR-154-AC-7), retaining the
meaning id when present, the module-qualified kind, the IR node identity, the
artifact id and the span.

If an IR node is not valid for its construct's meaning under FR-154's construct
validity rule, then the compiler SHALL refuse it with
`invalid_model_binding`/`malformed-declaration`.

The compiler SHALL derive type export records from the bound meaning as this
table fixes:

| Construct meaning | Export records |
| --- | --- |
| `quire.meaning.model.object-type/v1` | one `object` |
| `quire.meaning.model.value-type/v1` | one `scalar` naming the value type and its bound native value type |
| `quire.meaning.model.variant-type/v1` | one `enum`, one `variant` per case |
| `quire.meaning.model.population/v1` | one `population` |
| `quire.meaning.systems.interface/v1`, `quire.meaning.systems.part/v1`, `quire.meaning.systems.port/v1`, `quire.meaning.systems.connection/v1`, `quire.meaning.systems.allocation/v1` | none; the node binds to its FR-152 kind |

The compiler SHALL derive member export records from each type's members, which
carry no meaning id: one `field` per field member, one `reference` per field
member whose type is an object type, one `operation` per operation member and
one `relationship` per relationship member.

If a clause names a declaration bound to `quire.meaning.model.variant-type/v1`,
then the compiler SHALL refuse the clause with
`unsupported_construct`/`declaration-form` (FR-150).

The compiler SHALL key each `relationship` export by its owner's declaration
identity and declared `name`, with its `sourceSpan` as the export locus.

If a relationship member has no declared name or no source span, then the
compiler SHALL refuse it with `invalid_model_binding`/`malformed-declaration`.

When a construct's meaning id is one of the five `quire.meaning.systems.*`
values, the compiler SHALL bind the IR node to its FR-152 kind (Interface, Part,
Port, Connection or Allocation) and SHALL apply FR-152's kind mapping,
connection and allocation rules to it.

When an IR node binds as a Port, the compiler SHALL retain its owning part,
direction, interface type and typed multiplicity.

### Declarations

The compiler SHALL build declarations under FR-154: one original declaration per
IR node, ascending by declaration key, with FR-154's declaration refusals
`invalid_model_binding`/`malformed-declaration`,
`missing_declaration`/`missing-name`, `invalid_model_binding`/`conflicting-binding`
and `invalid_model_binding`/`unpreserved-model-meaning`.

Each IR type definition, including every Part, Port, Interface, Connection and
Allocation, has identity `ix://<package identity>/<artifact id>`; the owner of a
Part or Port and the ends of a Connection or Allocation are references and
never part of identity.
A member's identity is its owner's identity, `/` and the member name (Quire
specification FR-154).

The artifact id of each artifact that declares a type definition or population
node is an object id under Quire specification FR-154's id rule.
A clause names the declaration by that id, such as `M::sys_pump`.

If an IR type definition or population node carries an artifact id that does
not match FR-154's id rule, such as `sys-pump`, then the compiler SHALL refuse
it with `invalid_model_binding`/`malformed-declaration` under FR-154, retaining
the IR node identity, the artifact id and the span.

The compiler SHALL NOT use `title` or `displayName` in any identity, key,
digest, ordering or resolution decision.

If any declaration refusal occurs, then the compiler SHALL report every
declaration refusal in node order and SHALL admit no declaration of the package.

The compiler SHALL charge `normalize.record` under `ModelNormalizationLimitsV1`
once per declaration, ascending by declaration key, after all admission checks.

If the `normalize.record` limit is exhausted, then the compiler SHALL return an
incomplete result at `normalize.record` and SHALL admit no declaration.

The compiler SHALL produce byte-identical results for the same selection,
package input, definition and limits, whether it starts at the intake seam or
at the bundle entry point that lifts those bytes.

### Complete-V1 units (spine I1)

When a complete-V1 (`1-draft`) unit declares `model M = "<identity>" digest
"sha256-jcs:<hex>";` (QSpec shared grammar, quire-specification#176), spine intake (ADR-011 §2.1 I1) SHALL
admit that selection against the package input under this requirement, read its
records and normalize them, in the unit's declaration order, before the FR-091
assembler runs. The assembler SHALL declare each object type of the admitted
package in the package's type environment as `M::<artifact id>`, with its
declared supertypes and fields, and SHALL resolve `M::<artifact id>` and
`Reference<M::<artifact id>>` to that object type. A field is named by its
member name. A native `Boolean` or `Integer` value type, an object type (as
`Reference`) and a scalar type (as its `Int[lower, upper]` or its
`Text[min, max; profile]`) are its value type.
Multiplicity `[1, 1]` gives the value type outright, and any other
multiplicity gives the collection its `ordered` and `unique` flags name,
bounded by it unless the lower bound is `0` and the upper is unbounded, in
which case the collection is unbounded too; an unbounded upper bound with a
lower bound above `0` has no kernel type (QSpec FR-322's "Model-owned
members" step 4). The field's own declared
`presence` -- never a multiplicity lower bound of `0` -- decides whether it
is optional (QSpec's `model-complete.md` Presence row): a lower bound of `0`
makes an empty collection legal, and an `optional` presence wraps the
multiplicity's own value type in `Option`. A `redefines` names the redefined
field by its owner and name.

A `typeRef` under `ix://quire/native/` names one of QSL's closed native
value types, spelled as QSpec's shared grammar spells it: `Boolean`,
`Integer`, `Rational`, `Decimal`, `Float32`, `Float64` and `Text`. Text is
`Text`, the QSpec grammar's `Text[min, max; profile]` and the native name
QSpec's checked-package vectors write. A scalar type bound to `Text` with
bounds `0 <= min <= max` and a profile gives the value type
`Text[min, max; profile]`. The profile is one of the six QSpec FR-141 defines
(`unicode-scalars`, `nfc`, `nfd`, `nfkc`, `nfkd` and `binary-utf8`), and it
selects how the text is compared, measured and bounded, as FR-141's "Text and
enumeration profiles" states. If a scalar type bound to `Text` has bounds
outside `0 <= min <= max`, then intake SHALL refuse it with
`unsupported_construct`/`declaration-form` at the scalar type, naming the
bounds. If it names any other profile, then intake SHALL refuse it with
`unsupported_construct`/`declaration-form` at the scalar type, naming the
profile. A member whose `typeRef` is
`ix://quire/native/Text` itself carries no bounds or profile, which QSpec's
`Text[min, max; profile]` requires, so intake SHALL refuse it with
`unsupported_construct`/`declaration-form` at the member, naming `profile`.
If a `typeRef` names any other native name, then intake SHALL refuse it with
`invalid_model_binding`/`malformed-declaration` at the member, naming the
`typeRef`. If it names `Rational` or `Decimal`, whose required `dmin` and
`dmax` the semantic-IR constraint vocabulary cannot carry, then intake SHALL
refuse it with `unsupported_construct`/`declaration-form` at the member,
naming the parameter.

If an admitted package declares a record the type environment does not
represent (a record value type, a systems part, port or allocation, or a
field whose value type or multiplicity has no kernel type), then the
assembler SHALL refuse it with
`unknown_required_feature`/`unsupported-feature` at the `model` declaration.
The assembler SHALL declare each operation of an object type, with its
parameters, result and frame effect, as [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
states (as amended; operations were previously refused here).
A `Float32` or `Float64` field SHALL refuse with `unknown_required_feature`/`unsupported-feature`; a floating type in a function signature or body is admitted (FR-091-OQ-4), and admitting the field is QSL-285.

If a unit's `model` declaration spells a `sha256:` digest, then spine intake
SHALL refuse it with `invalid_model_binding` at the declaration: that slot
selects a compiled-model artifact (FR-056-CON-4). If a selection does not
admit, spine intake SHALL refuse with the FR-154 refusal's code at the
declaration. If a type name names no object type of an admitted package, then
the assembler SHALL refuse it with `missing_declaration` at the name.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-056-CON-1 | Intake SHALL depend on `agent-ix-extraction-frontend` and `agent-ix-semantic-ir`, with `publish = false` preserved. | Dependency | Inspection |
| FR-056-CON-2 | Compiler source SHALL contain no IR reader, IR schema copy, construct rule, module registry or installed-manifest lookup. | Maintainability | Inspection |
| FR-056-CON-3 | Compiler source SHALL select meaning, exports and checks without branching on a kind `name`, a module identity or a string literal equal to a module object-type name. | Maintainability | Inspection |
| FR-056-CON-4 | A domain package digest SHALL occupy only `sha256-jcs` slots, never a raw-byte or compiled-artifact digest slot. | Security | Test (TC-145) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-056-AC-1 | Lifted bytes of a valid domain package passed to the intake seam are read by `agent-ix-semantic-ir` and yield exactly one original declaration per IR node, ascending by (domain package identity, IR node identity), each with its bound meaning, export records and artifact id and span. | Test (TC-145) |
| FR-056-AC-2 | A wrong digest domain, a missing package, a stale digest and a package whose identity differs each refuse with FR-154's named cause, in FR-154's order, before any declaration; a reader-refused document retains every reader diagnostic and admits no declaration. The one parse refuses a lone high or low surrogate escape, a reversed pair, a lone surrogate in a member name. It refuses `1e400` and `-1e400` `noncanonical_wire`/`inexact-integer` and `1e-400` `noncanonical_wire`/`inexact-number`, each with `document_pointer` naming it, at any depth, through intake and through admission under any digest, never `stale_dependency`. A lone-surrogate document offered under the `sha256-jcs` digest of the same document with U+FFFD in its place refuses `stale_dependency`/`byte-digest-mismatch`; unparseable bytes offered under their own raw digest with an empty identity refuse `invalid_model_binding`/`wrong-model-selection`; bytes intake refuses at one of its limits refuse `resource_exhausted`/`intake-limit-exceeded`, naming that limit and its bound. | Test (TC-145) |
| FR-056-AC-3 | An IR node whose kind names no `constructs` entry refuses `invalid_model_binding`/`malformed-declaration`, reported in FR-154's declaration refusal order; a construct with no meaning id or one outside FR-208 refuses each IR node of its kind with `invalid_model_binding`/`malformed-declaration`, naming meaning id, kind, node, artifact and span; a node not valid for its construct's meaning under FR-154 refuses `invalid_model_binding`/`malformed-declaration`; renaming a kind while keeping its meaning id changes no meaning or export. | Test (TC-146) |
| FR-056-AC-4 | A type's key is its artifact id: changing only `title` or `displayName` leaves every key, export, ordering and binding unchanged, and two artifacts with equal titles stay distinct declarations; an artifact id matching FR-154's id rule, such as `sys_pump`, is admitted and named `M::sys_pump`, while one that does not, such as `sys-pump`, refuses `invalid_model_binding`/`malformed-declaration` with node, artifact and span, a reference to that node reports no `missing_declaration`/`missing-name`, and a second failing check on that node reports after the id refusal (FR-154-AC-8). | Test (TC-145) |
| FR-056-AC-5 | Each relationship member yields one `relationship` export with its name and span; a relationship member missing either refuses `invalid_model_binding`/`malformed-declaration`; a relationship member or reference to a node absent from the package refuses `missing_declaration`/`missing-name`; any declaration refusal leaves the whole package unadmitted with every refusal reported in node order. | Test (TC-145) |
| FR-056-AC-6 | Intake at its exact `normalize.record` bound completes, the one-less run is incomplete at `normalize.record` with no declaration, and admission refusals are decided before the first charge. | Test (TC-147) |
| FR-056-AC-7 | The same selection, package input, definition and limits yield byte-identical results from the intake seam and from the bundle entry point, and a domain package digest offered in a raw-byte or compiled-artifact digest slot refuses. | Test (TC-145, TC-147) |
| FR-056-AC-9 | A `1-draft` unit selecting a domain package by its `sha256-jcs` digest assembles with each of the package's object types declared as `M::<artifact id>` with its declared supertypes and fields, a subtype conforming to its declared supertype, and checks with functions over `M::T` and `Reference<M::T>`; inherited field access checks (`deref(g).code` over a `Gadget` whose supertype `Widget` declares `code`), and `deref(g).nope` and `g = r` over an unrelated `Rock` refuse `ill_typed` at check. A `model` declaration with no admitted package refuses at the assembler as `missing_import`. `Reference<M::Nope>` refuses at the assembler as `missing_declaration` at `M::Nope`; with no package supplied the declaration refuses at intake as `missing_import`, and a `sha256:` digest refuses at intake as `invalid_model_binding`, each at the `model` declaration. Past the I2 read, a package holding the field access resolves the model member through the lock-selected domain package (IR-285, QSpec STD-100) and returns Verified exporting `code`; one holding `g = w` over conforming references returns Verified exporting `same`, admitted by IR-285's QVC checked-operation catalog (STD-101/102) for `quire.op.reference.eq`; TC-442 verifies both. | Test (TC-442) |
| FR-056-AC-10 | A field's multiplicity `[1, 1]` gives its declared value type outright; any other multiplicity gives the `ordered`/`unique`-selected collection (`Set`, `Bag`, `Sequence` or `OrderedSet`) bounded by it, unbounded only at a lower bound of `0`, and an unbounded upper bound with a lower bound above `0` has no kernel type (QSpec FR-322's "Model-owned members" step 4). The field's own declared `presence` -- never a multiplicity lower bound of `0` -- decides whether it is optional (QSpec's `model-complete.md` Presence row): `required`/`optional` map to the assembled declaration's own presence one to one, and any other value refuses `invalid_model_binding`/`malformed-declaration` at intake, naming the offending value. | Test (TC-443) |
| FR-056-AC-11 | A domain package whose field `label` is typed by a scalar type `Label` bound to `Text` with bounds `0` and `64` and profile `nfc` is admitted, and `label`'s value type is `Text[0, 64; nfc]`; each of the other five QSpec FR-141 profiles is admitted the same way. Bounds satisfy `0 <= min <= max`: with `Label`'s bounds `-1` and `64`, or `65` and `64`, intake refuses `unsupported_construct`/`declaration-form` at `Label`, naming the bounds. With `Label`'s profile `nfx`, intake refuses `unsupported_construct`/`declaration-form` at `Label`, naming `nfx`. With `label`'s `typeRef` `ix://quire/native/Text`, it refuses `unsupported_construct`/`declaration-form` at `label`, naming `profile`; with `ix://quire/native/String`, it refuses `invalid_model_binding`/`malformed-declaration` at `label`, naming `ix://quire/native/String`. The same outcomes hold for an operation parameter and an operation result. | Test (TC-897) |
| FR-056-AC-12 | The one parse refuses a document that repeats a member name within one object, at the repeated name's byte offset, and a document that begins with a byte order mark, at byte 0. A document repeating a member name, offered under the `sha256-jcs` digest of its last-wins value, refuses `stale_dependency`/`byte-digest-mismatch` with its raw digest as the recomputed digest. | Test (TC-145) |
| FR-056-AC-13 | The one parse refuses a document holding `18446744073709551616`, `-18446744073709551616`, `1e20`, `9.007199254740993e15`, `9007199254740993` or `-9007199254740993` at `/package/count` with `noncanonical_wire`/`inexact-integer` and `document_pointer` `/package/count`; one holding `1e20` at member `c~d` of the first element of member `a/b` names `/a~1b/0/c~0d`, and one holding such numbers at `/b` and then `/a/0` names `/b`. The same document holding `9007199254740992` or `-9007199254740992` at `/package/count` is admitted. Documents that differ only in holding `18446744073709551615` or `18446744073709551616`, each offered under the `sha256-jcs` digest of the same document holding `18446744073709552000`, which they shared before this rule, refuse `noncanonical_wire`/`inexact-integer` at `/package/count`, never `stale_dependency`/`byte-digest-mismatch`; a field whose multiplicity `upper` is 2^60 + 1 refuses the same way at that bound's pointer under the digest it used to admit under and under its raw digest. | Test (TC-145) |
| FR-056-AC-14 | The one parse refuses a document holding `9007199254740993.5`, `0.1000000000000000000001`, `1e-400`, `-1e-400` or `4.9e-324` at `/package/count` with `noncanonical_wire`/`inexact-number` and `document_pointer` `/package/count`; the same document holding `9007199254740993`, which fits both cases, refuses `inexact-integer`, as does `1e20`; and a document holding `0.5` at `/a/0` and `1e-400` at `/a/1` names `/a/1`. The same document holding `0.1`, `1.0`, `-0`, `-0.0`, `5e-324`, `1e15` or `9007199254740991` at `/package/count` is admitted, with the digest of the RFC 8785 text of the same value. A document holding `0.1000000000000000000001` at `/package/count`, offered under the `sha256-jcs` digest of the same document holding `0.1`, refuses `noncanonical_wire`/`inexact-number`, and the document holding `0.1` admits under it. The double nearest `1125899906842624.25` has two equally close shortest texts: a document holding `1125899906842624.2` (the even digit, RFC 8785's text) at `/package/count` is admitted, as are `1500000000000000.2` and `2.9802322387695312e-8`, and one holding `1125899906842624.3`, `1500000000000000.3` or `2.9802322387695313e-8` refuses `noncanonical_wire`/`inexact-number`. | Test (TC-145) |
| FR-056-AC-15 | The derived view of an admitted document holds, for each non-whole number, the correctly rounded double of its text, the double its `sha256-jcs` digest encodes: a document holding `1.5e-300` admits, and its tree holds the double `1.5e-300` reads as, whatever `serde_json`'s own float parse reads. | Test (TC-145) |
| FR-056-AC-8 | End to end, the filament-core-data#173 architecture fixture runs bundle → quire-rs → lift → intake seam; its ports resolve with owning part, direction, interface type and multiplicity, a connection between them is admitted under FR-152, and the model linker binds a native package's references to those declarations. | Test (TC-148, IT-012) |

## Open Questions

| Question | Owner | Blocked criteria |
| --- | --- | --- |
| Component, endpoint, participant contract and configuration constructs have no FR-208 meaning id, so a construct naming one is refused under FR-154-AC-7 until FR-208 adds ids for them. | Architect decision | None in this requirement; FR-048 positive component, endpoint and participant cases |

## Dependencies

- **Upstream:** [US-002](../usecase/US-002-link-exact-models.md); Quire
  specification AD-006 and FR-150–154 own model intake, normalization,
  conformance, systems kinds and closed lookup; FR-208 and FR-154
  own the Quire meaning ids and artifact-id identity; filament-core-data
  FR-142/FR-143 and agent-ix/filament-core-data#172 own Semantic IR 2.0.0, the
  embedded `constructs` table and the reader; quire-rs owns artifact extraction.
- **Downstream:** [FR-036](FR-036-link-composed-native-packages.md) links native
  declarations against the admitted declarations;
  [FR-042](FR-042-publish-compiled-protocol-artifacts.md) names the domain
  package in the compiled-protocol `Model`;
  [IT-012](../integration/IT-012-domain-package-model-intake.md) exercises the
  real crates end to end on the agent-ix/filament-core-data#173 fixture.
- Implementation: agent-ix/quire-spec-language#131. Compiled-protocol `Model`
  wire change: agent-ix/quire-spec-language#132.

## References

- Linear QSL-290 (AC-11: a `Text` scalar type is admitted with its QSpec FR-141 profile).
- Linear QSL-219 (AC-13, AC-14: a number with no exact RFC 8785 spelling refuses `noncanonical_wire`); QSpec FR-271 and FR-272 catalog the code, its causes and `document_pointer` (quire-specification#180).
- QSpec FR-141, "Text and enumeration profiles": the six text profiles.
