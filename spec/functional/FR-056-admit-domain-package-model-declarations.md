---
id: FR-056
title: "Admit domain packages as Quire model declarations"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-002, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: traces_to }
  - { target: ix://agent-ix/quire-specification/FR-150, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-151, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-152, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-153, type: depends_on }
  - { target: ix://agent-ix/filament-core-data/FR-142, type: depends_on }
  - { target: ix://agent-ix/filament-core-data/FR-143, type: depends_on }
---
# FR-056: Admit domain packages as Quire model declarations

## Description

When a spec artifact bundle is selected as a model source, the compiler SHALL
derive one Quire model declaration per admitted Semantic IR 2.0.0 type
definition before any native declaration binds to it.

The domain model is authored in spec artifacts. An artifact typed by a module
object type (for example `object: entity`) is one instance; nobody writes a
per-object schema, TypeSpec type or model record. Intake is one path:

```text
spec artifact bundle
  -> quire_rs::semantic::extract_semantic with the modules' real manifests
  -> agent-ix-extraction-frontend lift (deterministic lowering, sha256-jcs)
  -> Semantic IR 2.0.0 domain package, read by agent-ix-semantic-ir
  -> Quire model declarations (this requirement)
  -> model linker (FR-036)
```

quire-rs is the only artifact parser, FCD owns the lowering and the IR reader,
and Quire owns model meaning. Generated Rust/TypeScript/Python types and Quire
model declarations therefore come from one fingerprinted lowering of the same
artifacts.

## Inputs

- The spec artifact bundle: an explicit inventory of artifact paths and bytes.
  No filesystem discovery, network retrieval or installed module supplies an
  omitted artifact.
- The exact module manifests that type the bundle's artifacts, selected by
  module identity and version.
- The `agent-ix-extraction-frontend` and `agent-ix-semantic-ir` crates at the
  git revision selected in `Cargo.lock`.
- The meaning-binding table: the closed set of Quire meaning ids this compiler
  admits, each with its Quire meaning, the export kinds it yields and the
  `shape` and `identity` values it accepts.
- Caller-lowered finite intake limits for bundle bytes, artifacts, type
  definitions, members, relations and constructs-table entries.

A Semantic IR 2.0.0 document carries:

- a module-qualified `kind` on every type definition,
  `{ "module": "<owner>/<repo>", "name": "<object type>" }`;
- an embedded `constructs` table with one entry per used kind: its `identity`
  (`identified | value | none`), `shape`
  (`record | enumeration | interface | state_machine | sequence | namespace`),
  `members`, `references` by role, `rules`, `meaning` id, module version and
  manifest digest;
- relation declarations, each with its declared `name` and `sourceSpan`.

## Outputs

An immutable domain-package admission report containing:

- the domain package identity: package identity, contract version `2.0.0` and
  the `sha256-jcs` digest of the IR document;
- one model declaration per admitted type definition, keyed by
  (domain package identity, IR node identity, digest domain `sha256-jcs`);
- for each declaration, its bound Quire meaning, its export records and the
  source locus of its originating artifact;
- or a typed refusal, unsupported or resource-incomplete disposition that names
  the refused IR node, its artifact and source locus.

The model linker ([FR-036](FR-036-link-composed-native-packages.md)) consumes
only this report. The compiled-protocol `Model` selection names the domain
package identity and digest.

## Behavior

The compiler SHALL obtain the IR document only by running
`quire_rs::semantic::extract_semantic` over the selected bundle with the selected
module manifests and lifting the result with `agent-ix-extraction-frontend`.

The compiler SHALL read the IR document only through `agent-ix-semantic-ir`.

The compiler SHALL NOT contain an IR reader, an IR schema copy or a construct rule.

If `agent-ix-semantic-ir` refuses the document, then the compiler SHALL refuse
the whole domain package and retain every reader diagnostic with its IR node and
source locus.

If the document's contract version is not `2.0.0`, then the compiler SHALL
refuse the domain package with `unsupported-ir-contract-version`.

The compiler SHALL resolve each type definition's module-qualified `kind` to
its entry in the document's embedded `constructs` table and SHALL consult no
module registry, installed manifest or kind list to do so.

If a type definition's `kind` has no entry in the embedded `constructs` table,
then the compiler SHALL refuse that declaration with `unresolved-construct-kind`,
naming the module and kind name.

If two `constructs` entries share one `{module, name}` but differ in
declaration, module version or manifest digest, then the compiler SHALL refuse
the domain package with `conflicting-construct-declaration`.

The compiler SHALL bind each construct to a Quire meaning by looking up the
construct's `meaning` id in the meaning-binding table.

If a construct's `meaning` id is absent from the meaning-binding table, then the
compiler SHALL refuse every declaration of that kind with `unknown-meaning-id`,
naming the meaning id, the module-qualified kind and each refused declaration's
source locus.

If a construct's `shape` or `identity` is not accepted by the Quire meaning its
`meaning` id binds to, then the compiler SHALL refuse every declaration of that
kind with `meaning-shape-mismatch`, naming the meaning id, the declared shape
and identity, and the accepted values.

The compiler SHALL select Quire meaning, export kinds and checks from the bound
meaning id, `shape` and `identity` only. No compiler source SHALL branch on a
kind `name` or a module identity.

The compiler SHALL use a type definition's IR `identity`, which is its source
artifact's id, as the declaration identity.

The compiler SHALL NOT use `displayName` as an identity, a resolution key or an
input to any equality, digest or ordering decision.

The compiler SHALL derive export records from each declaration as the bound
Quire meaning prescribes, using the export kinds
`scalar | enum | variant | record | field | object | reference | operation | relationship | population`
from the [compiled-protocol wire contract](../../docs/compiled-protocol-v1.md).

The compiler SHALL derive one `relationship` export per relation declaration,
keyed by its source declaration identity and declared `name`, with its
`sourceSpan` as the export locus.

If a relation declaration has no declared name or no source span, then the
compiler SHALL refuse that relation with `incomplete-relation-declaration`.

If the target of a reference member or relation is absent from every admitted
domain package, then the compiler SHALL refuse the referring declaration with
`dangling-model-reference`.

If an intake limit is exhausted, then the compiler SHALL return
`resource_exhausted` with the dimension, limit and usage, and SHALL NOT expose a
partial domain package as admitted.

The compiler SHALL produce byte-identical admission reports for the same
bundle bytes, manifests, crate revisions and meaning-binding table.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-056-CON-1 | Intake SHALL depend on `agent-ix-extraction-frontend` and `agent-ix-semantic-ir` by exact git revision, with `publish = false` preserved. | Dependency | Inspection |
| FR-056-CON-2 | Compiler source SHALL contain no string literal equal to a module object-type name used for dispatch. | Maintainability | Inspection |
| FR-056-CON-3 | A domain package digest SHALL use the `sha256-jcs` domain in every slot, never a raw-byte or compiled-artifact digest slot. | Security | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-056-AC-1 | A bundle extracted by quire-rs with its real module manifests and lifted by `agent-ix-extraction-frontend` is read by `agent-ix-semantic-ir` as IR 2.0.0 and yields one model declaration per type definition, keyed by domain package identity, IR node identity and `sha256-jcs`, with its bound Quire meaning and export records. | Test (TC-145) |
| FR-056-AC-2 | A reader-refused document, a non-2.0.0 contract version and an exhausted intake limit each return their typed disposition with located diagnostics and no admitted declaration. | Test (TC-145) |
| FR-056-AC-3 | A construct whose `meaning` id is not in the meaning-binding table refuses every declaration of that kind with `unknown-meaning-id` naming the id, the module-qualified kind and each source locus; a shape or identity the bound meaning does not accept refuses with `meaning-shape-mismatch`. Unrelated declarations stay admitted. | Test (TC-146) |
| FR-056-AC-4 | Each module-qualified `kind` resolves through the embedded `constructs` table with no module registry; a kind missing from the table refuses with `unresolved-construct-kind`, and two conflicting entries for one kind refuse the package with `conflicting-construct-declaration`. Two modules declaring the same kind name under different module ids remain distinct kinds. | Test (TC-147) |
| FR-056-AC-5 | Declaration identity is the artifact id: changing only `displayName` leaves identities, digests, ordering and linking unchanged, and two declarations with equal `displayName` remain distinct. | Test (TC-145) |
| FR-056-AC-6 | Every relation declaration yields one `relationship` export carrying its declared `name` and `sourceSpan`; a relation missing either refuses with `incomplete-relation-declaration`, and a dangling reference or relation target refuses with `dangling-model-reference`. | Test (TC-145) |
| FR-056-AC-7 | End to end, a domain-package bundle whose constructs cover object, record, enum, reference, operation, relationship and population runs bundle → quire-rs → lift → IR 2.0.0 → model declarations, and the model linker binds a native package's model references, relationships and populations to those declarations. | Test (TC-148, IT-012) |

## Dependencies

- **Upstream:** [US-002](../usecase/US-002-link-exact-models.md);
  filament-core-data FR-142/FR-143 and agent-ix/filament-core-data#172 own
  Semantic IR 2.0.0, the embedded `constructs` table and the reader rules;
  quire-rs owns artifact extraction; the Quire specification FR-150–153 own
  model graph, lookup, specialization and dispatch meaning.
- **Downstream:** [FR-036](FR-036-link-composed-native-packages.md) links native
  declarations against the admitted declarations;
  [FR-042](FR-042-publish-compiled-protocol-artifacts.md) names the domain
  package in the compiled-protocol `Model`;
  [IT-012](../integration/IT-012-domain-package-model-intake.md) exercises the
  real crates end to end.
- Implementation: agent-ix/quire-spec-language#131. Compiled-protocol `Model`
  wire change: agent-ix/quire-spec-language#132.
