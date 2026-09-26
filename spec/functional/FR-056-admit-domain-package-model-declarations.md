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
  declaration's domain package identity, version and `sha256-jcs` digest. Each
  selection names exactly one domain package.
- The package input: a map from `sha256-jcs` digest to domain package bytes.
- The selected `quire.model.complete/v1` definition and the Quire meaning ids
  of Quire specification FR-208.
- `ModelNormalizationLimitsV1` of `quire.value.accounting/v1`.
- For the bundle entry point: the explicit artifact inventory (paths and bytes)
  and the exact module manifests that type it, selected by module identity and
  version. No filesystem discovery, network retrieval or installed module
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

A document does not parse when its bytes are not UTF-8 or not JSON, carry a
lone UTF-16 surrogate escape (a `\uD800`-`\uDFFF` escape that is not a high
surrogate followed by a low surrogate), or carry a number with no finite IEEE
754 double value (such as `1e400`). RFC 8785 encodes none of these inputs, so
such a document has no `sha256-jcs` digest.

If the selected bytes do not parse, then the compiler SHALL take check 3's
digest over the raw bytes, and SHALL read no package identity or version for
check 4. Such bytes therefore refuse
`stale_dependency`/`byte-digest-mismatch` under any `sha256-jcs` digest, and
`invalid_model_binding`/`wrong-model-selection` when the selected digest
happens to equal their raw digest. No declaration is admitted.

When a document carries an integer whose magnitude exceeds 2^53, the compiler
SHALL encode it for the digest as the IEEE 754 double nearest to it, ties to
even, as RFC 8785 reads every number. Every spelling of one double therefore
has one digest: `18446744073709551615` and `18446744073709551616` both digest
as `18446744073709552000`. Declarations read an integer within the 64-bit
signed or unsigned range exactly as written; only the digest reads the double.

If the selected bytes exceed `agent-ix-semantic-ir`'s input-size limit, or
carry a value enclosed by that reader's depth limit or more arrays and objects,
then the compiler SHALL refuse the package with
`resource_exhausted`/`intake-limit-exceeded`, naming the limit and its bound.
This refusal comes after FR-154's check 2 and before its check 3.

### Admission

The compiler SHALL apply FR-154's admission checks in FR-154's order before
reading any declaration: `stale_dependency`/`digest-domain-mismatch`,
`missing_import`/`missing-selection`, `stale_dependency`/`byte-digest-mismatch`
and `invalid_model_binding`/`wrong-model-selection`.

If `agent-ix-semantic-ir` refuses the selected bytes, then the compiler SHALL end
intake with no declaration and SHALL retain every reader diagnostic with its IR
node, artifact id and span.

The reader owns contract-version checks and resolution of each module-qualified
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

When a complete-V1 (`1-draft`) unit declares `model M = "<identity>" version
"<version>" digest "sha256-jcs:<hex>";`, spine intake (ADR-011 §2.1 I1) SHALL
admit that selection against the package input under this requirement, read its
records and normalize them, in the unit's declaration order, before the FR-091
assembler runs. The assembler SHALL declare each object type of the admitted
package in the package's type environment as `M::<artifact id>`, with its
declared supertypes and fields, and SHALL resolve `M::<artifact id>` and
`Reference<M::<artifact id>>` to that object type. A field is named by its
member name. A native `Boolean` or `Integer` value type, an object type (as
`Reference`) and a scalar type (as its `Int[lower, upper]`) are its value type.
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

If an admitted package declares a record the type environment does not
represent (a record value type, a systems part, port or allocation, or a
field whose value type or multiplicity has no kernel type), then the
assembler SHALL refuse it with
`unknown_required_feature`/`unsupported-feature` at the `model` declaration.
The assembler SHALL declare each operation of an object type, with its
parameters, result and frame effect, as [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
states (amended by QSL-273; before it, operations refused here).
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
| FR-056-CON-1 | Intake SHALL depend on `agent-ix-extraction-frontend` and `agent-ix-semantic-ir` by exact git revision, with `publish = false` preserved. | Dependency | Inspection |
| FR-056-CON-2 | Compiler source SHALL contain no IR reader, IR schema copy, construct rule, module registry or installed-manifest lookup. | Maintainability | Inspection |
| FR-056-CON-3 | Compiler source SHALL select meaning, exports and checks without branching on a kind `name`, a module identity or a string literal equal to a module object-type name. | Maintainability | Inspection |
| FR-056-CON-4 | A domain package digest SHALL occupy only `sha256-jcs` slots, never a raw-byte or compiled-artifact digest slot. | Security | Test (TC-145) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-056-AC-1 | Lifted bytes of a valid domain package passed to the intake seam are read by `agent-ix-semantic-ir` and yield exactly one original declaration per IR node, ascending by (domain package identity, IR node identity), each with its bound meaning, export records and artifact id and span. | Test (TC-145) |
| FR-056-AC-2 | A wrong digest domain, a missing package, a stale digest and a package whose identity or version differs each refuse with FR-154's named cause, in FR-154's order, before any declaration; a reader-refused document retains every reader diagnostic and admits no declaration. The one parse refuses a lone high or low surrogate escape, a reversed pair, a lone surrogate in a member name, and `1e400`; a lone-surrogate document offered under the `sha256-jcs` digest of the same document with U+FFFD in its place refuses `stale_dependency`/`byte-digest-mismatch`; unparseable bytes offered under their own raw digest with an empty identity and version refuse `invalid_model_binding`/`wrong-model-selection`; bytes over the reader's size or depth limit refuse `resource_exhausted`/`intake-limit-exceeded` naming the limit; a document carrying `18446744073709551615` or `18446744073709551616` admits under the digest of the same document carrying `18446744073709552000`, and not under the digest of its own exact digits. | Test (TC-145) |
| FR-056-AC-3 | An IR node whose kind names no `constructs` entry refuses `invalid_model_binding`/`malformed-declaration`, reported in FR-154's declaration refusal order; a construct with no meaning id or one outside FR-208 refuses each IR node of its kind with `invalid_model_binding`/`malformed-declaration`, naming meaning id, kind, node, artifact and span; a node not valid for its construct's meaning under FR-154 refuses `invalid_model_binding`/`malformed-declaration`; renaming a kind while keeping its meaning id changes no meaning or export. | Test (TC-146) |
| FR-056-AC-4 | A type's key is its artifact id: changing only `title` or `displayName` leaves every key, export, ordering and binding unchanged, and two artifacts with equal titles stay distinct declarations; an artifact id matching FR-154's id rule, such as `sys_pump`, is admitted and named `M::sys_pump`, while one that does not, such as `sys-pump`, refuses `invalid_model_binding`/`malformed-declaration` with node, artifact and span, a reference to that node reports no `missing_declaration`/`missing-name`, and a second failing check on that node reports after the id refusal (FR-154-AC-8). | Test (TC-145) |
| FR-056-AC-5 | Each relationship member yields one `relationship` export with its name and span; a relationship member missing either refuses `invalid_model_binding`/`malformed-declaration`; a relationship member or reference to a node absent from the package refuses `missing_declaration`/`missing-name`; any declaration refusal leaves the whole package unadmitted with every refusal reported in node order. | Test (TC-145) |
| FR-056-AC-6 | Intake at its exact `normalize.record` bound completes, the one-less run is incomplete at `normalize.record` with no declaration, and admission refusals are decided before the first charge. | Test (TC-147) |
| FR-056-AC-7 | The same selection, package input, definition and limits yield byte-identical results from the intake seam and from the bundle entry point, and a domain package digest offered in a raw-byte or compiled-artifact digest slot refuses. | Test (TC-145, TC-147) |
| FR-056-AC-9 | A `1-draft` unit selecting a domain package by its `sha256-jcs` digest assembles with each of the package's object types declared as `M::<artifact id>` with its declared supertypes and fields, a subtype conforming to its declared supertype, and checks with functions over `M::T` and `Reference<M::T>`; inherited field access checks (`deref(g).code` over a `Gadget` whose supertype `Widget` declares `code`), and `deref(g).nope` and `g = r` over an unrelated `Rock` refuse `ill_typed` at check. A `model` declaration with no admitted package refuses at the assembler as `missing_import`. `Reference<M::Nope>` refuses at the assembler as `missing_declaration` at `M::Nope`; with no package supplied the declaration refuses at intake as `missing_import`, and a `sha256:` digest refuses at intake as `invalid_model_binding`, each at the `model` declaration. Past the I2 read, a package holding the field access resolves the model member through the lock-selected domain package (IR-285, QSpec STD-100) and returns Verified exporting `code`; one holding `g = w` over conforming references returns Verified exporting `same`, admitted by IR-285's QVC checked-operation catalog (STD-101/102) for `quire.op.reference.eq`; TC-442 verifies both. | Test (TC-442) |
| FR-056-AC-10 | A field's multiplicity `[1, 1]` gives its declared value type outright; any other multiplicity gives the `ordered`/`unique`-selected collection (`Set`, `Bag`, `Sequence` or `OrderedSet`) bounded by it, unbounded only at a lower bound of `0`, and an unbounded upper bound with a lower bound above `0` has no kernel type (QSpec FR-322's "Model-owned members" step 4). The field's own declared `presence` -- never a multiplicity lower bound of `0` -- decides whether it is optional (QSpec's `model-complete.md` Presence row): `required`/`optional` map to the assembled declaration's own presence one to one, and any other value refuses `invalid_model_binding`/`malformed-declaration` at intake, naming the offending value. | Test (TC-443) |
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
