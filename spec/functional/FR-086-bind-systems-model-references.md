---
id: FR-086
title: "Bind systems-model references: Interface, Part, Port, Connection and Allocation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-085
    type: references
  - target: ix://agent-ix/quire-specification/FR-152
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
---
# FR-086: Bind systems-model references: Interface, Part, Port, Connection and Allocation

## Description

When classifying and admitting the systems-model structure of an admitted
domain package, the model binder SHALL resolve every component, endpoint,
relationship and allocation record to exactly one of the five FR-152 systems
kinds or to no kind, by its declared capability and end kinds alone. The model
binder SHALL admit a Connection or an Allocation only when every one of its
independent admission conditions holds, and SHALL report every failing
condition.

## Inputs

- Component records, each carrying whether it supplies the part-signature
  capability.
- Endpoint records, each carrying an optional declared port direction and
  its owning component's declaration key.
- Object-type export records carrying whether they supply interface
  features.
- Relationship records, each carrying source and target ends, a direction
  and per-end multiplicity.
- Allocation records, each carrying a source element and a target element.
- The conformance relation [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md)
  produces.
- `ModelNormalizationLimitsV1`.

## Outputs

A systems classification mapping every component, endpoint, relationship and
allocation record's declaration key to its resolved kind (`Part`, `Port`,
`Interface`, `Connection`, `Allocation` or no kind), every no-kind cascade
refusal in declaration-key order; and, per Connection or Allocation checked,
an admitted outcome, a refusal naming every failing condition, or a typed
incomplete result.

## Behavior

### Classification is by declared capability and end kind, never by display name

The model binder SHALL classify a component as `Part` only when it supplies
the part-signature capability; otherwise it SHALL record a no-kind cascade
naming the missing capability. It SHALL classify an endpoint as `Port` only
when it declares a port direction and its owning component classifies as
`Part`; otherwise it SHALL record a no-kind cascade naming the missing
direction or the wrong owner kind. It SHALL classify an object-type export as
`Interface` only when it supplies interface features. It SHALL classify a
relationship as `Connection` only when both ends resolve to declared
endpoints that themselves classify as `Port`; a relationship whose both ends
resolve to declared object types classifies to no systems kind under this
requirement (see [FR-085](FR-085-resolve-relationship-end-references.md)).
No classification decision SHALL depend on a display name: two records that
share a display identity but differ in declaration key SHALL classify, and
resolve, independently.

### Every no-kind cascade is reported, not only the first

The model binder SHALL classify every component, then every endpoint, then
every relationship, then every allocation, each in ascending declaration-key
order, charging one classification unit per record before resolving it, and
SHALL collect every no-kind cascade into the classification's refusal list
rather than stopping at the first.

### A Connection admits only when every condition holds

For a relationship already classified `Connection`, the model binder SHALL
check, independently: that the resolved ports' declared directions are
compatible with the relationship's declared direction; that the flow-source
port's interface type conforms to the flow-target port's interface type
(equal types for a bidirectional relationship); and that each end's declared
multiplicity conforms to its port's declared multiplicity. It SHALL charge
one unit before each of these three conditions and SHALL report every
failing condition in this order rather than stopping at the first. A
Connection whose relationship is not itself classified `Connection`, or
whose declared end does not resolve to a declared endpoint, SHALL be refused
outright before any condition charge.

### An Allocation admits only when its target is a Part

For an allocation already classified `Allocation`, the model binder SHALL
admit it only when its declared target element classifies as `Part`;
otherwise it SHALL refuse with a wrong-export cause naming the required kind
(`Part`) and the target's actual resolved kind. An allocation whose
`relationship_key` does not itself classify as `Allocation` SHALL be refused
outright before the target-kind charge.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-086-AC-1 | Given a domain package with one component missing the part-signature capability, one endpoint missing a declared direction, and one interface-qualified object type, classification records the two no-kind cascades (naming the missing capability and the missing direction respectively) and the `Interface` kind, all three in the classification's refusal list and kind map, not only the first found. | Test (TC-233) |
| FR-086-AC-2 | Given a Connection-classified relationship whose port-direction condition and whose per-end multiplicity condition both fail while the interface-type condition holds, `check_connection` reports both failing conditions (naming their own cause) and does not report the interface-type condition as failing; a relationship satisfying all three conditions is admitted. | Test (TC-234) |
| FR-086-AC-3 | Given an Allocation whose target element classifies as `Port` rather than `Part`, `check_allocation` refuses wrong-export naming required kind `Part` and actual kind `Port`; given a target that classifies as `Part`, the allocation is admitted. | Test (TC-235) |
| FR-086-AC-4 | Given two declarations that share the same display title but have distinct declaration keys — one a genuine `Part` and one that supplies no part-signature capability — resolving each by its own key yields the correct, independent kind for each; neither declaration's classification is affected by the other's display title. | Test (TC-236) |

## Dependencies

- **Upstream:** [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  supplies the declaration keys this requirement classifies by; [FR-056](FR-056-admit-domain-package-model-declarations.md)
  admits the component, endpoint, relationship and allocation records with
  no export beyond binding to their FR-152 kind; [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md)
  supplies the conformance relation the interface-type condition checks
  against. quire-specification FR-152 owns the normative kind-mapping,
  connection and allocation rules this requirement binds, and AD-006 owns
  the decision to resolve systems structure, interfaces first, in the
  checked model view.
- **Downstream:** none within this ticket's scope; a later requirement may
  bind systems-model navigation over an object population, which this
  requirement's scope explicitly excludes.
- Relates to [FR-085](FR-085-resolve-relationship-end-references.md), which
  resolves a plain object-to-object relationship's ends; this requirement's
  Connection classification applies only once both ends already resolve to
  declared endpoints.
- Systems-model records receive no effective-declaration identity under
  [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md):
  this requirement resolves each record's own original declaration key and
  kind only, consistent with AD-006's export table, which names no export
  record for the five systems meanings beyond binding to their FR-152 kind.
