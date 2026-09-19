---
id: FR-150
title: "Preserve original and effective model declarations"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/AD-006
    type: references
  - target: ix://agent-ix/quire-specification/FR-035
    type: references
  - target: ix://agent-ix/quire-specification/FR-321
    type: references
---

# FR-150: Preserve original and effective model declarations

## Description

When normalizing the declarations of an admitted domain package, the model
checker SHALL retain each original declaration key, effective declaration
identity and ordered derivation provenance.

## Inputs

The original declarations that model intake (FR-154) builds from one domain
package, its ModelSelection (FR-321) and the selected
`quire.model.complete/v1` definition with its rule manifest
`quire.model.complete.rules/v1`.

## Outputs

An immutable original-to-effective view, carried as one
`quire.model.effective-view/v1` identity, or a typed refusal.

## Behavior

Each original declaration is keyed by `(domain package identity, IR node
identity, sha256-jcs)`. Supertypes, subsets and redefines are members of the
IR nodes, so every edge normalization reads is part of the admitted package.
QSL derives effective declarations. It never mints, renames or merges a
declaration key, and never derives an edge the package does not declare.
Normalization never replaces the original key or source span. Every derived
member records its source declarations and rule. Equal normalized shapes from
distinct original declarations always remain distinct effective declarations;
no identity-equivalence declaration exists.

The effective member key is `(owner effective type identity, original
declaration key)`. Paths that reach the same original declaration collapse to
one effective member retaining a derivation fact for every path. A source name
that resolves to distinct original declarations is refused
`ambiguous_declaration`/`ambiguous-name` at source resolution only; the
effective view keeps both members.

Each effective declaration identity is `quire.model.effective-declaration/v1`
over `{version, owner_effective_type, original, derivation}`; the effective view
is `quire.model.effective-view/v1` over the ModelSelection, rule manifest and
every effective declaration sorted by identity; and the object universe of a
connected supertype component is `quire.model.object-universe/v1`. The exact
preimages, orders and vectors are defined by the selected
`quire.model.complete/v1` definition,
[`model-effective-declaration.schema.json`](../../../proposals/checked-package-v2/model-effective-declaration.schema.json)
and
[`model-effective-declaration-vectors.json`](../../../proposals/checked-package-v2/model-effective-declaration-vectors.json).
These domains differ from `sha256-jcs` and from the I04 node domain, so no
derived identity equals a domain package digest or an I04 node key. The
effective view is carried in the checked package as a `correspondence` node
with semantic form `model_correspondence`.

## Normalization derivation

The model checker applies the manifest's rules in this fixed phase order:
(1) `quire.model.normalize.intake/v1` and `quire.model.features/v1`, domain
package admission, declaration building and feature checks (FR-154);
(2) `quire.model.normalize.qualify/v1`, qualified-identity formation for
object types, interfaces, fields, relationships, operations, parts, ports
(owned by their part's owning type), connections and allocations (owned by each
end's end-owner type);
(3) `quire.model.normalize.inherit/v1`, inheritance expansion along declared
supertypes; (4) `quire.model.normalize.subset/v1` and
`quire.model.normalize.redefine/v1`, declared subsetting and redefinition; and
(5) `quire.model.normalize.canonicalize/v1`, identity computation. Each derived
fact records its declaration-key inputs, rule identity, rule revision and
ordinal. A later phase may reference but cannot erase an earlier derivation. No
rule declares an override relation, and no precedence exists between rules.

Two phase 4 redefinitions of the same member reaching one effective type
conflict unless one redefining owner is a proper descendant of every other;
subsetting never conflicts with redefinition because it derives no replacement.
A conflict is refused `invalid_model_binding`/`derivation-conflict` with both
derivation paths and exposes no effective member for either. A redefinition
whose target is not an inherited member is refused
`invalid_model_binding`/`redefinition-target`. A supertype cycle is refused
`invalid_model_binding`/`specialization-cycle`. A ModelSelection whose digest
domain is not `sha256-jcs` is refused `stale_dependency`/`digest-domain-mismatch`,
and package bytes whose digest differs are refused
`stale_dependency`/`byte-digest-mismatch`. A model `variant` type referenced by
source is refused `unsupported_construct`/`declaration-form`. Canonical ordering
affects identity bytes only; it never changes a declaration key or resolves
semantic ambiguity.

Normalization is charged under `ModelNormalizationLimitsV1` of the selected
`quire.value.accounting/v1` definition at its named record, fact, cycle-check,
redefinition-check, conflict-check, declaration and preimage-hash charge
points, the last by JCS byte length. Each normalization phase checks
exhaustively and reports every refusal it exposes in charge order; a phase that
reports a refusal ends checking. Exhaustion ends checking as
`resource_exhausted`/`insufficient-next-charge` with no effective view.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-150-AC-1 | An inherited/redefined member links its normalized identity to every contributing original declaration and rule. | Test (TC-195) |
| FR-150-AC-2 | Two equal effective shapes from distinct original identities remain distinguishable. | Test (TC-195) |
| FR-150-AC-3 | A wrong digest domain, a stale package digest or a conflicting phase 4 derivation refuses normalization with its named cause. | Test (TC-195) |
| FR-150-AC-4 | Replaying the ordered derivation from the same domain package reproduces the `quire.model.effective-view/v1` identity, independent of IR node order. | Test (TC-195) |
| FR-150-AC-5 | Conflicting derivations report both rule paths and expose no chosen effective member. | Test (TC-195) |
| FR-150-AC-6 | Diamond paths reaching one original declaration collapse to one effective member retaining every path, and each effective identity equals its published vector. | Test (TC-195) |
| FR-150-AC-7 | Declaration keys are fixed by the domain package identity and IR node identity: two versions of one package with equal nodes yield equal effective declaration identities and different effective views. | Test (TC-195) |
| FR-150-AC-8 | A normalization run at its exact `ModelNormalizationLimitsV1` bound completes, and the one-less run is incomplete at the named charge point. | Test (TC-195) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
