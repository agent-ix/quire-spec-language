---
id: FR-152
title: "Bind complete systems-model structures"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-035
    type: references
  - target: ix://agent-ix/quire-specification/AD-006
    type: references
---

# FR-152: Bind complete systems-model structures

## Description

When a clause references a systems-model element, the binder SHALL resolve its
typed part, port, interface, connection or allocation identity through the exact
producer correspondence.

## Inputs

Checked package, admitted producer interface `1.3.0` export with its
ModelSelection key (FR-321), the FR-150 effective view, the selected
`quire.model.complete/v1` definition and a qualified element reference with its
required kind.

## Outputs

A typed resolved model element/relationship or located kind, identity,
direction, closure or foreign refusal.

## Behavior

Parts, ports, interfaces, connections and allocations retain distinct kinds and
their declared endpoint/direction/type constraints. Display names and equal
shapes are not identities. Navigation follows only explicit producer relations
and preserves original/effective declaration provenance.

## Systems-model roles

Rule `quire.model.systems.kind-mapping/v1` of the selected
`quire.model.complete/v1` definition maps each producer record to exactly one
kind; the kinds are disjoint.

| Kind | Producer record | Required identity and constraints |
| --- | --- | --- |
| Part | FCD FR-114 component with `part-signature` | qualified declaration identity, owning composite (`owningTypeIdentity`), declared type (`typeIdentity`) and typed multiplicity |
| Port | FCD FR-114 endpoint with `port-direction` whose owning component is a Part | qualified declaration identity, owning part (`owningComponentIdentity`), direction `in`, `out` or `inout`, interface type (`typeIdentity`, an Interface) and typed multiplicity |
| Interface | `object` type export with `interfaceFeatures` | qualified declaration identity and ordered feature signatures |
| Connection | FCD FR-115 relationship whose two ends both name endpoint records that are Ports and whose `semantics.category` is not exactly `allocation` | effective member owned by the source end-owner type, plus compatible source and target port identities |
| Allocation | FCD FR-115 relationship whose `semantics.category` is exactly the bytes `allocation` | effective member owned by the source end-owner type, plus source element (Part, Port or operation member) and target element (Part) identities, each named by its end's `typeIdentity` |

Ownership follows `quire.model.complete/v1`: a component is owned by its
`owningTypeIdentity`; an endpoint by the owning type of its component; and a
relationship end's end-owner type is the type named by, or owning the component
or endpoint named by, the end's `typeIdentity`. Endpoints and relationships are
qualified into the FR-150 effective view under those owners. An end `e` of type
`T` exists exactly when `T` has the relationship member derived from the
opposite end's end-owner type and `r.name` matches `e`'s role name. A
relationship whose two ends both name types is a navigation relationship with
no kind.

Kind mapping runs over components, then endpoints, then relationships, each
ascending by producer key, charging `systems.kind` before each record, and is
exhaustive under the limits: a component lacking `part-signature` is
`unsupplied-producer-record` and has no kind; an endpoint with
`port-direction` whose component has no kind or another kind is
`wrong-export` (required Part); a non-allocation relationship end naming an
endpoint that is not a Port is `wrong-export` (required Port, once per end,
source first). Refusals cascade: a record whose dependency was refused is
reported with actual kind `none`.

A binder request names the required kind and a qualified name. A resolved
record of another kind is refused `invalid_model_binding`/`wrong-export` with
the required kind, actual kind and both producer identities; equal names never
pass a kind check. A record that lacks the capability its kind requires is
refused `invalid_model_binding`/`unsupplied-producer-record`.

Rule `quire.model.systems.connection/v1` admits a connection only when all of
these hold, each charged as `systems.connection-condition` in table order and reported
independently before the connection is exposed:

| Condition | Refusal |
| --- | --- |
| For FCD direction `source-to-target`, the source port is `out` or `inout` and the target port is `in` or `inout`; for `target-to-source`, the same with source and target exchanged; for `bidirectional`, both ports are `inout`; `undirected` is never a connection direction | `invalid_model_binding`/`port-direction` |
| The flow-source port's interface type conforms under FR-151 to the flow-target port's interface type (for `bidirectional`, the two interface types are the same effective type) | `ill_typed`/`type-mismatch` |
| Each relationship end's typed multiplicity conforms under FR-151 to its port's typed multiplicity | `ill_typed`/`multiplicity-narrowing` |

Rule `quire.model.systems.allocation/v1` admits an allocation only when its
target element is a Part, charging `systems.allocation`; any other target is
`invalid_model_binding`/`wrong-export` (required Part, actual kind or `none`).
Allocation compatibility has no other rule, and no producer profile prose
supplies one.

## Navigation

A correspondence bundle is exactly one admitted FCD FR-117 static bundle key
joined to one FR-321 ModelSelection key. Static navigation (`qualified-name`
resolution through members and ends) is resolved at checking and charged as
`systems.resolve` per qualified-name segment after the alias and per runtime
navigation site, in source order. An end whose `typeIdentity` names an export of
another correspondence bundle is refused at checking as
`foreign_reference`/`foreign-model-selection`. Runtime navigation `r.name` over `r: Reference<T>` resolves
statically to exactly one field member or relationship end of `T` in the
effective view; a name matching both is
`ambiguous_declaration`/`ambiguous-name`. Traversal from source to target
requires relationship direction `source-to-target`, `bidirectional` or
`undirected`, and traversal from target to source requires `target-to-source`,
`bidirectional` or `undirected`; any other traversal is
`ill_typed`/`operator-ineligible`. The static result type follows the end's
typed multiplicity: `[0,1]` gives `Option<Reference<U>>`; `[1,1]` gives
`Reference<U>`; any other finite `[l,u]` gives `Set<Reference<U>>[l,u]` when
`unique` is true and `Bag<Reference<U>>[l,u]` otherwise, in canonical
reference-key order. An `unbounded` upper bound or `ordered: true` is
`unsupported_construct`/`expression-form`. At runtime, targets are taken in
canonical reference-key order and a target absent from a complete population is
`dangling_reference`, decided before the count bound; then a target count
outside the bound is `cardinality_out_of_bound`/`below-minimum` or
`above-maximum`. Charges are the `model.navigate` schedule of
`quire.value.accounting/v1`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-152-AC-1 | Valid references to every supported systems-model kind resolve with exact producer and effective-declaration identity. | Test (TC-197) |
| FR-152-AC-2 | Substituting a part for a port or a connection for an allocation refuses `invalid_model_binding`/`wrong-export` even when names match. | Test (TC-197) |
| FR-152-AC-3 | Missing relationship closure or an ambiguous effective target refuses without ambient lookup. | Test (TC-197) |
| FR-152-AC-4 | Port direction, endpoint type and multiplicity are all enforced before a connection is exposed. | Test (TC-197) |
| FR-152-AC-5 | Relationship navigation never crosses correspondence bundles or substitutes display-name equality for identity. | Test (TC-197) |
| FR-152-AC-6 | Each FCD relationship direction admits exactly the port-direction pairs and traversals stated above, and every other pair or traversal refuses with its named cause. | Test (TC-197) |
| FR-152-AC-7 | Runtime navigation returns the multiplicity-determined type in canonical reference-key order and charges exactly the `model.navigate` schedule. | Test (TC-197) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
