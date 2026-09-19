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
  - target: ix://agent-ix/quire-specification/FR-208
    type: references
---

# FR-152: Bind complete systems-model structures

## Description

When a clause references a systems-model element, the binder SHALL resolve its
typed part, port, interface, connection or allocation identity through the
declarations of the admitted domain package.

## Inputs

Checked package, the admitted domain package with its ModelSelection (FR-321),
the FR-150 effective view, the selected `quire.model.complete/v1` definition
and a qualified element reference with its required kind.

## Outputs

A typed resolved model element/relationship or located kind, identity,
direction or foreign refusal.

## Behavior

Parts, ports, interfaces, connections and allocations retain distinct kinds and
their declared end, direction and type constraints. Display names and equal
shapes are not identities. Navigation follows only declared relationships and
connections and preserves original/effective declaration provenance.

## Systems-model roles

Rule `quire.model.systems.kind-mapping/v1` of the selected
`quire.model.complete/v1` definition maps each IR node to at most one systems
kind by the Quire meaning id (FR-208) of its construct: the `meaning` of the
`constructs` table entry that the node's `{module, name}` kind names. The kinds
are disjoint. A node's kind name and module never select a systems kind: a node
whose construct names no systems meaning has no systems kind, whatever its kind
name, and a node whose construct names a systems meaning has that kind under
any kind name.

| Kind | Construct meaning | Required identity and constraints |
| --- | --- | --- |
| Part | `quire.meaning.systems.part/v1` | construct declaration key, `owner` reference to the owning composite type, declared type and typed multiplicity |
| Port | `quire.meaning.systems.port/v1`, whose `owner` names a Part | construct declaration key, `owner` reference to the owning part, direction `in`, `out` or `inout`, interface type (an Interface) and typed multiplicity |
| Interface | `quire.meaning.systems.interface/v1` | construct declaration key and ordered feature signatures |
| Connection | `quire.meaning.systems.connection/v1`, whose two ends both name Ports | construct declaration key, effective member owned by the source end-owner type, compatible source and target port keys and a declared `direction` |
| Allocation | `quire.meaning.systems.allocation/v1` | construct declaration key, effective member owned by the source end-owner type, plus source element (Part, Port or operation) and target element (Part) keys |

Each part, port, interface, connection and allocation is declared by its own
artifact and is a construct, not a member. Its IR node identity is
`ix://<package identity>/<artifact id>`, like any type definition (FR-154).
Ownership is a reference of the construct, not a member, and never part of its identity:
a part's `owner` names its owning type; a port's `owner` names its part, and
the port is owned by that part's owning type; and a connection or allocation
end's end-owner type is the element owner of the declaration its source or
target end names. Parts, ports, connections and allocations are qualified into
the FR-150 effective view as effective members `(owner effective type,
construct declaration key)` under those owners; an interface is an effective
type. Changing an `owner` or an end changes the effective member, never the
construct's declaration key. An end `e` of type `T` exists exactly when `T` has the
relationship member and `r.name` matches `e`'s role name. A relationship whose
two ends both name object types is a navigation relationship with no kind.

Kind mapping runs over interfaces, then parts, then ports, then connections,
then allocations, each ascending by declaration key, charging `systems.kind` before each node, and
is exhaustive under the limits: a port whose owner has no kind or another kind
is `wrong-export` (required Part); a port whose interface type is not an
Interface is `wrong-export` (required Interface); a connection end naming a
node that is not a Port is `wrong-export` (required Port, once per end, source
first). Refusals cascade: a node whose dependency was refused is reported with
actual kind `none`. Every refusal names the IR node identity, the artifact and
the span.

A binder request names the required kind and a qualified name. A qualified
name names a systems construct by the model alias and its artifact id, such as
`M::pump_out`, never through its owner. It resolves to the construct's
effective declaration: an Interface's effective type, a Part's or Port's
effective member under its owner, and a Connection's or Allocation's effective
member under its source end-owner type. A resolved
declaration of another kind is refused `invalid_model_binding`/`wrong-export`
with the required kind, actual kind and both declaration keys; equal names
never pass a kind check.

Rule `quire.model.systems.connection/v1` admits a connection only when all of
these hold, each charged as `systems.connection-condition` in table order and
reported independently before the connection is exposed:

| Condition | Refusal |
| --- | --- |
| For declared direction `source-to-target`, the source port is `out` or `inout` and the target port is `in` or `inout`; for `target-to-source`, the same with source and target exchanged; for `bidirectional`, both ports are `inout`; `undirected` is never a connection direction | `invalid_model_binding`/`port-direction` |
| The flow-source port's interface type conforms under FR-151 to the flow-target port's interface type (for `bidirectional`, the two interface types are the same effective type) | `ill_typed`/`type-mismatch` |
| Each connection end's typed multiplicity conforms under FR-151 to its port's typed multiplicity | `ill_typed`/`multiplicity-narrowing` |

Rule `quire.model.systems.allocation/v1` admits an allocation only when its
target element is a Part, charging `systems.allocation`; any other target is
`invalid_model_binding`/`wrong-export` (required Part, actual kind or `none`).
Allocation compatibility has no other rule.

## Navigation

Static navigation (`qualified-name` resolution through members and ends) is
resolved at checking and charged as `systems.resolve` per qualified-name
segment after the alias and per runtime navigation site, in source order. A
qualified name whose alias names another ModelSelection than the one that owns
the navigated declaration is refused at checking as
`foreign_reference`/`foreign-model-selection`. Runtime navigation `r.name` over
`r: Reference<T>` resolves statically to exactly one field or relationship end
of `T` in the effective view; a name matching both is
`ambiguous_declaration`/`ambiguous-name`. Traversal from source to target
requires declared direction `source-to-target`, `bidirectional` or
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
| FR-152-AC-1 | Valid references to every supported systems-model kind resolve with exact declaration key and effective-declaration identity. | Test (TC-197) |
| FR-152-AC-2 | Substituting a part for a port or a connection for an allocation refuses `invalid_model_binding`/`wrong-export` even when names match. | Test (TC-197) |
| FR-152-AC-3 | A dangling end reference or an ambiguous effective target refuses without ambient lookup. | Test (TC-197) |
| FR-152-AC-4 | Port direction, port interface type and multiplicity are all enforced before a connection is exposed. | Test (TC-197) |
| FR-152-AC-5 | Relationship navigation never crosses ModelSelections or substitutes display-name equality for identity. | Test (TC-197) |
| FR-152-AC-6 | Each declared relationship direction admits exactly the port-direction pairs and traversals stated above, and every other pair or traversal refuses with its named cause. | Test (TC-197) |
| FR-152-AC-7 | Runtime navigation returns the multiplicity-determined type in canonical reference-key order and charges exactly the `model.navigate` schedule. | Test (TC-197) |
| FR-152-AC-8 | A node whose construct meaning is not a systems meaning has no systems kind even when its kind name is `port`: a binder request for a Port and a connection end naming it each refuse `invalid_model_binding`/`wrong-export` with required kind `Port` and actual kind `none`. | Test (TC-197) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [FR-208](../../objects/foundation/FR-208-quire-meaning-vocabulary.md) owns
  the systems meaning ids.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
