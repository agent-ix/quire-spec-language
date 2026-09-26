---
id: FR-111
title: "Link a complete-V1 definition bundle over a caller-supplied definition catalog"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/IT-011
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-131
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-339
    type: depends_on
---
# FR-111: Link a complete-V1 definition bundle over a caller-supplied definition catalog

## Description

QSL SHALL link a set of root definition selections, against a
caller-supplied definition catalog, into a dependency-closed complete-V1
bundle, or refuse with a catalogued code and cause (QSpec FR-131, and
FR-339-AC-3's backend-independence; QSpec V1-SRC-003 and V1-SRC-004,
which QSpec's delivery manifest assigns to QSL and IT-011 adopts).

This is the closure, cycle, facet and bundle capability of
`complete::resolve_source_package`, kept when `ResolvedSourcePackage`
retires (FR-087, ruling on QSL-234, 2026-09-26). It lives in layer-3
`library` as `library::bundle`, the owner ADR-011 §6.2 gives
`complete::package`. It computes a bundle only: it builds no checked
package, no `quire.checked-package/v2` bytes and no backend artifact, and
no stage of the S1 to S4 spine calls it.

## Inputs

- `roots: &[DefinitionRef]`: the root selections, each an exact
  identity/version/raw-byte digest (`qsl_foundation::selection`).
- `catalog: &DefinitionCatalog`: the caller's definitions. Each
  `Definition` holds its exact `DefinitionRef` computed from its bytes, its
  `DefinitionRole` (which gives its facet), its dependency edges
  (`DefinitionRef`s), its `CapabilityId`s and its exact bytes. A
  `Definition` is built with a `ReaderAuthority`.
- `limits: PackageLimits`.

## Outputs

- `library::bundle::link_bundle(roots, catalog, limits)
  -> Result<LinkedBundle, BundleRefusal>`.
- `LinkedBundle`: the closed definitions by `DefinitionRef`, the
  `CompleteBundle` (facets, capabilities and its `SemanticDigest` identity)
  and the `PackageLimits` it was checked against.
- `BundleRefusal`: the `Code`, the `ResolutionCause`, the typed
  `PackageError` and the index of the root whose closure refused.

## Behavior

### Closure

- `link_bundle` SHALL close the roots over their dependency edges, depth
  first, and refuse a root or a reached definition that the catalog does
  not hold exactly.
- If two selections of one identity differ, among the roots or anywhere in
  the closure, then `link_bundle` SHALL refuse
  `ambiguous_declaration`/`conflicting-authority`, naming both.
- If the dependency edges form a cycle, then `link_bundle` SHALL refuse
  `invalid_package`/`definition-cycle`, naming the cycle's definitions in
  path order.

### Bundle

- The bundle's facets SHALL be the facets of the closed definitions'
  roles. If one of the nine complete-V1 facets is missing, then
  `link_bundle` SHALL refuse `invalid_package`/`feature-set-mismatch`,
  naming it.
- The bundle's capabilities SHALL be the union of the closed definitions'
  capabilities. If a capability of the closed 176-entry inventory is
  missing, then `link_bundle` SHALL refuse
  `unknown_required_feature`/`unsupported-feature`; a capability outside
  the inventory SHALL refuse `unknown_required_feature`/`unknown-feature`.
- The bundle's identity SHALL be the `quire.complete.resolved-graph/2`
  `SemanticDigest` of the preimage `{capabilities, definitions, models}`:
  the capabilities, the closed definitions (exact reference, role,
  dependencies, capabilities) and `models`, which is always the empty
  array, because model selections resolve only through I1 (FR-056). The
  label stays `/2`, so the identity of an existing bundle is unchanged. No backend, installed
  runtime or support set is an input, so none changes the identity or the
  admission.

### Refusals

| Case | Code | Cause |
| --- | --- | --- |
| a root's identity is in no catalog definition | `unknown_profile` | `unsupported-selection` |
| a root's identity is held at another version | `stale_dependency` | `revision-mismatch` |
| a root's identity and version are held with other bytes | `stale_dependency` | `byte-digest-mismatch` |
| a dependency edge names a definition the catalog does not hold | `missing_import` | `missing-selection` |
| two selections of one identity differ | `ambiguous_declaration` | `conflicting-authority` |
| a dependency cycle | `invalid_package` | `definition-cycle` |
| a missing facet | `invalid_package` | `feature-set-mismatch` |
| a missing inventory capability | `unknown_required_feature` | `unsupported-feature` |
| a capability outside the inventory | `unknown_required_feature` | `unknown-feature` |
| a catalog holding one exact definition twice | `invalid_package` | `duplicate-member` |
| dependency-chain depth above `limits.depth` | `stage_limit_exceeded` | `nesting-depth-exceeded` |
| any other `PackageLimits` ceiling | `resource_exhausted` | `insufficient-next-charge` |

### Limits

Every `PackageLimits` field applies, exactly as the caller supplies it:
`definitions` (catalog size and closed definitions), `dependency_edges`
(catalog edges and traversed edges), `depth` (the active chain),
`artifact_bytes` (total bytes of the catalog and of the closure) and
`single_artifact_bytes` (each definition). `DefinitionCatalog::with_limits`
applies them to the catalog and `link_bundle` to the closure.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-111-CON-1 | `library::bundle` depends only on layers F and K, on `semantic_value` (`value::semantic_node::IDENTITY_LIMITS`, the canonical-encoding limits its identity digest uses) and on `library`'s own items, and on the external `quire-canonical` and `serde` crates it encodes the identity through, as ADR-011 §6.1's layer-3 order permits. It names no `check`, `package`, `model`, emitter or backend type, and returns no checked or emitted package. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-111-AC-1 | Roots whose closure covers the nine facets and the 176 capabilities link. The `LinkedBundle` holds every closed definition and the nine facets, and its identity is unchanged when the caller's known-backend set is broader or narrower (`CompleteBundle::validate_known_capabilities` alone refuses an unknown capability). | Test (TC-491) |
| FR-111-AC-2 | A root whose identity the catalog lacks refuses `unknown_profile`/`unsupported-selection`; one held at another version refuses `stale_dependency`/`revision-mismatch`; one held with other bytes refuses `stale_dependency`/`byte-digest-mismatch`; a dependency edge to an absent definition refuses `missing_import`/`missing-selection`. Each names the root's index. | Test (TC-491) |
| FR-111-AC-3 | Two roots, or a root and a reached definition, selecting one identity at two exact selections refuse `ambiguous_declaration`/`conflicting-authority` naming both; a dependency cycle refuses `invalid_package`/`definition-cycle` naming the cycle in path order. | Test (TC-491) |
| FR-111-AC-4 | Removing the definitions of one facet refuses `invalid_package`/`feature-set-mismatch` naming that facet; removing one capability refuses `unknown_required_feature`/`unsupported-feature`. | Test (TC-491) |
| FR-111-AC-5 | The bundle identity matches the `quire.complete.resolved-graph/2` golden vector (preimage `{"capabilities":[…],"definitions":[],"models":[]}`), and changing one definition's role, dependencies, capabilities or bytes changes it. | Test (TC-491) |
| FR-111-AC-6 | Each `PackageLimits` field admits at its bound and refuses one past it, at catalog construction and at link, with the table's code and cause; a caller-raised `definitions` ceiling admits a catalog the default refuses, and the link records the limits it ran under. | Test (TC-491) |
| FR-111-AC-7 | Every refusal's (code, cause) pair is one `quire.native.diagnostics/v1` revision `1-draft.7` lists. | Test (TC-491) |

## Dependencies

- QSpec FR-131 (AC-1 to AC-3), FR-339 (AC-3), `docs/v1-delivery-ticket-manifest.md`
  (V1-SRC-003, V1-SRC-004) and `native-diagnostics.md`.
- [IT-011](../integration/IT-011-adopt-complete-v1-baseline.md), which adopts them.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md) §6.1, §6.2 (`complete::package` → layer-3 `library`).
- [FR-087](FR-087-typestate-and-cross-package-node-key.md), whose AC-7 moves this capability here when `ResolvedSourcePackage` retires.

## Status

Specified under QSL-234 (ruling 2026-09-26). The capability is implemented
today inside `complete::resolve_source_package` and backed there by
`tests/it/complete_package.rs` and `complete::package_tests`, tagged QSpec
FR-131-AC-1 to AC-3 and FR-339-AC-3. `library::bundle` does not exist yet,
so FR-111's criteria are unbacked -- TC-491 planned. The relocation lands
in the QSL-269 change that retires `ResolvedSourcePackage`: the code and
its tests move together, keeping their QSpec tags, so the capability is
backed throughout.
