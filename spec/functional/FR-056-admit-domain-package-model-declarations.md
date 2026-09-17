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
the IR reader, and the Quire specification owns model meaning and the meaning-id
registry. Generated Rust, TypeScript and Python types and Quire model
declarations come from one fingerprinted lowering of the same artifacts.

## Inputs

- One ModelSelection (Quire specification FR-321): the source `model`
  declaration's domain package identity, version and `sha256-jcs` digest. Each
  selection names exactly one domain package.
- The package input: a map from `sha256-jcs` digest to domain package bytes.
- The selected `quire.model.complete/v1` definition, including the meaning-id
  registry that agent-ix/quire-specification#85 adds to it.
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

### Meaning binding

The compiler SHALL bind each IR node to a Quire meaning by its construct's
`meaning` id, looked up in the selected meaning-id registry
(agent-ix/quire-specification#85).

If a construct's `meaning` id is absent from the selected registry, then the
compiler SHALL refuse each IR node of that kind with the refusal
agent-ix/quire-specification#85 names, retaining the meaning id, the
module-qualified kind, the IR node identity, the artifact id and the span.

If a construct's `shape` or `identity` is not one the bound meaning accepts, then
the compiler SHALL refuse each IR node of that kind with
`invalid_model_binding`/`malformed-declaration`.

The compiler SHALL derive export records from the bound meaning, `shape` and
`identity` as this table fixes:

| Bound meaning | Accepted `identity` / `shape` | Export records |
| --- | --- | --- |
| object | `identified` / `record` | one `object`, one `field` per member, one `reference` per reference member |
| record | `value` / `record` | one `record`, one `field` per member |
| enumeration | `none` / `enumeration` | one `enum`, one `variant` per member |
| operation | `none` / `interface` | one `operation` per feature signature |
| population | `identified` / `sequence` | one `population` |

The compiler SHALL derive a `scalar` export for each member type that the bound
meaning declares scalar.

The compiler SHALL derive one `relationship` export per relation declaration,
keyed by its source declaration identity and declared `name`, with its
`sourceSpan` as the export locus.

If a relation declaration has no declared name or no source span, then the
compiler SHALL refuse it with `invalid_model_binding`/`malformed-declaration`.

When a construct's meaning id binds a systems-model meaning, the compiler SHALL
bind the IR node to its FR-152 kind (Part, Port, Interface, Connection or
Allocation) and SHALL apply FR-152's kind mapping, connection and allocation
rules to it.

When an IR node binds as a Port, the compiler SHALL retain its owning part,
direction, interface type and typed multiplicity.

When a construct's meaning id binds a component, endpoint, participant or
configuration meaning, the compiler SHALL retain the IR node as a typed
unsupported prerequisite and SHALL derive no export for it.

### Declarations

The compiler SHALL build declarations under FR-154: one original declaration per
IR node, ascending by declaration key, with FR-154's declaration refusals
`invalid_model_binding`/`malformed-declaration`,
`missing_declaration`/`missing-name`, `invalid_model_binding`/`conflicting-binding`
and `invalid_model_binding`/`unpreserved-model-meaning`.

The compiler SHALL use a type's source artifact id as its IR node identity and
an owner's identity plus member name as a member's identity
(agent-ix/quire-specification#85).

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
| FR-056-AC-2 | A wrong digest domain, a missing package, a stale digest and a package whose identity or version differs each refuse with FR-154's named cause, in FR-154's order, before any declaration; a reader-refused document retains every reader diagnostic and admits no declaration. | Test (TC-145) |
| FR-056-AC-3 | A construct whose meaning id is absent from the selected registry refuses each IR node of its kind with the #85 refusal, naming meaning id, kind, node, artifact and span; a shape or identity its meaning does not accept refuses `invalid_model_binding`/`malformed-declaration`; renaming a kind while keeping its meaning id changes no meaning or export. | Test (TC-146) |
| FR-056-AC-4 | A type's key is its artifact id: changing only `title` or `displayName` leaves every key, export, ordering and binding unchanged, and two artifacts with equal titles stay distinct declarations. | Test (TC-145) |
| FR-056-AC-5 | Each relation yields one `relationship` export with its name and span; a relation missing either refuses `invalid_model_binding`/`malformed-declaration`; a relation or reference to a node absent from the package refuses `missing_declaration`/`missing-name`; any declaration refusal leaves the whole package unadmitted with every refusal reported in node order. | Test (TC-145) |
| FR-056-AC-6 | Intake at its exact `normalize.record` bound completes, the one-less run is incomplete at `normalize.record` with no declaration, and admission refusals are decided before the first charge. | Test (TC-147) |
| FR-056-AC-7 | The same selection, package input, definition and limits yield byte-identical results from the intake seam and from the bundle entry point, and a domain package digest offered in a raw-byte or compiled-artifact digest slot refuses. | Test (TC-147) |
| FR-056-AC-8 | End to end, the filament-core-data#173 architecture fixture runs bundle → quire-rs → lift → intake seam; its ports resolve with owning part, direction, interface type and multiplicity, a connection between them is admitted under FR-152, and the model linker binds a native package's references to those declarations. | Test (TC-148, IT-012) |

## Open Questions

| Question | Owner | Blocked criteria |
| --- | --- | --- |
| The meaning-id registry, its spelling and the refusal code for an unknown meaning id; artifact-id identity in place of FR-154's artifact title; systems kinds bound by construct meaning id in place of FR-152's IR kind. | agent-ix/quire-specification#85 | FR-056-AC-3, FR-056-AC-4, FR-056-AC-8 |
| Which Quire meanings, members and export kinds cover component, endpoint, participant contract and configuration constructs. Until decided they stay typed unsupported prerequisites. | Architect decision | None in this requirement; FR-048 positive component, endpoint and participant cases |

## Dependencies

- **Upstream:** [US-002](../usecase/US-002-link-exact-models.md); Quire
  specification AD-006 and FR-150–154 own model intake, normalization,
  conformance, systems kinds and closed lookup; agent-ix/quire-specification#85
  owns the meaning-id registry and artifact-id identity; filament-core-data
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
