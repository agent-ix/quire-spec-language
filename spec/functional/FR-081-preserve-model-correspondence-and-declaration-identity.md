---
id: FR-081
title: "Preserve immutable model correspondence and original/effective declaration identity"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/quire-specification/FR-150
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
  - target: ix://agent-ix/quire-specification/interface_004
    type: references
---
# FR-081: Preserve immutable model correspondence and original/effective declaration identity

## Description

When the model binder normalizes the original declarations
[FR-056](FR-056-admit-domain-package-model-declarations.md) admits from one
domain package, it SHALL derive one immutable correspondence between each
original declaration and the declaration derived from it. Every original
declaration key SHALL survive unchanged in that correspondence. Every derived
declaration SHALL carry a normalized identity computed only from the
correspondence's own preimage. No identity, key or ordering the binder
produces SHALL depend on a display name, a registration order or any state
held outside the one `DomainPackage` value and limits the caller supplies.

This requirement is QSL's own binding of quire-specification
[FR-150](ix://agent-ix/quire-specification/FR-150) ("Preserve original and
effective model declarations") and [AD-006](ix://agent-ix/quire-specification/AD-006)'s
model-view decisions into the compiler's actual types and behavior; FR-150
states the normative rule, this requirement states QSL's binder contract for
it.

## Inputs

- The original declarations [FR-056](FR-056-admit-domain-package-model-declarations.md)
  builds, each keyed by `(domain package identity, IR node identity,
  sha256-jcs)`.
- The declared `supertypes`, `subsets` and `redefines` edges each original
  declaration carries as an inline property (never a second producer record).
- The selected `quire.model.complete/v1` definition and its rule manifest
  `quire.model.complete.rules/v1`.
- `ModelNormalizationLimitsV1` of `quire.value.accounting/v1`.

## Outputs

An immutable original-to-effective view — one `EffectiveId`-keyed entry per
resolved effective declaration, each retaining its original declaration key
and an ordered derivation-fact trail — or a typed refusal or incomplete
result.

## Behavior

### Original identity never moves

The model binder SHALL treat each original declaration's key as fixed input:
it SHALL NOT mint, rename or merge a declaration key, and SHALL NOT derive an
edge the domain package does not declare. A supertype, subset or redefine
edge exists in the correspondence only when the admitted declaration itself
carries it.

### Effective identity is a normalized digest over the correspondence, never over display text

The model binder SHALL compute each effective declaration's identity as a
normalized digest over exactly `{owner_effective_type, original declaration
key, derivation facts}`. It SHALL NOT read a `title`, `displayName` or any
other presentation field when computing an identity, a key, a digest, an
ordering or a resolution decision. Two admitted declarations with equal
declared shape but distinct original declaration keys SHALL always yield
distinct effective declarations; the binder SHALL NOT collapse them by an
identity-equivalence rule.

### Losing redefinitions are retained, not deleted

When two or more declared redefinitions of one member reach the same
effective type and one redefining owner is a proper descendant of every
other, the binder SHALL resolve that descendant's redefinition as effective
and SHALL retain every other reaching redefinition's entry in the
correspondence, marked as not the member a name resolves to. A losing
redefinition's original key, its
derivation facts and its own effective identity SHALL remain readable from
the correspondence for provenance and diagnostics.

### The binder holds no ambient registry

The model binder SHALL be a pure function of the one `DomainPackage` value
and the one limits value its caller supplies. It SHALL hold no static,
thread-local or process-global table of declarations, and no binder run
SHALL observe or be observed by another run's declarations. Given the same
`DomainPackage` and the same limits, every call SHALL produce byte-identical
original keys, effective identities and correspondence ordering, whether the
calls occur in the same process or in independent processes.

### Charging and incompleteness

The model binder SHALL charge normalization work under
`ModelNormalizationLimitsV1` before exposing a derived fact. If a charge is
denied, the binder SHALL return an incomplete result naming the exhausted
charge point and SHALL expose no effective declaration for the run.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-081-CON-1 | The model binder SHALL NOT read `title`, `displayName` or any other presentation field of an admitted declaration for any identity, key, digest, ordering or resolution purpose. | Design | Test (TC-216) |
| FR-081-CON-2 | The model binder's public entry point SHALL take the `DomainPackage` and limits as explicit parameters. | Design | Inspection |
| FR-081-CON-3 | The model binder SHALL declare no static, `OnceCell`, `thread_local` or other ambient state carrying declarations between calls. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-081-AC-1 | Given a domain package admitted by FR-056, every original declaration key the binder's correspondence names is byte-identical to the key FR-056 admitted for it; no key in the correspondence is absent from, or added beyond, the admitted set. | Test (TC-213) |
| FR-081-AC-2 | Two effective declarations derived from distinct original declaration keys are never equal, even when their declared shape (fields, types, multiplicities) is identical; each effective identity differs because its preimage's `original` component differs. | Test (TC-214) |
| FR-081-AC-3 | Given a three-level chain where a base type declares a member and two of its descendants each independently redefine it, so that both redefinitions reach the most specific type's effective member, the more specific (descendant) redefiner's entry is the one a name resolves to, and the less specific redefiner's entry remains present in the correspondence with its own original key, derivation facts and effective identity readable rather than being deleted. | Test (TC-215) |
| FR-081-AC-4 | Given two admitted domain packages that are byte-identical except for a `displayName`/title field on one declaration, every original key, effective identity and correspondence ordering the binder produces is byte-identical between the two runs. | Test (TC-216) |
| FR-081-AC-5 | Given the same `DomainPackage` value and the same limits, two independent binder invocations — including invocations in two separate process runs — produce byte-identical original keys, effective identities and correspondence ordering, with no observable interaction between the runs. | Test (TC-217) |

## Dependencies

- **Upstream:** [FR-056](FR-056-admit-domain-package-model-declarations.md)
  supplies the original declarations this requirement normalizes;
  quire-specification FR-150 and AD-006 own the normative rule and model-view
  decisions this requirement binds; quire-specification interface_004 (I04)
  is the immutable checked-package boundary the resulting correspondence is
  carried across, as a `correspondence` node.
- **Downstream:** [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md),
  [FR-083](FR-083-resolve-unique-most-specific-dispatch.md) and
  [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md) resolve
  conformance, dispatch and population membership over the correspondence
  this requirement produces.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-03 and O-05 record QSL's ownership of original declaration identity and
  effective declaration identity; this requirement states their behavioral
  contract, not their Rust module placement.
