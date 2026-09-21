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
kinds or to no kind, by the Quire meaning id (FR-208) of its construct and,
for a Port, Connection or Allocation, by its declared end kinds. The model
binder SHALL admit a Connection or an Allocation only when every one of its
independent admission conditions holds, and SHALL report every failing
condition.

This requirement is QSL's own binder contract for quire-specification
[FR-152](ix://agent-ix/quire-specification/FR-152)'s kind-mapping rule
`quire.model.systems.kind-mapping/v1`, which maps each IR node to at most one
systems kind by the meaning id of its construct, never by a kind name, a
module or a separate wire capability. [FR-056](FR-056-admit-domain-package-model-declarations.md)
already binds each construct's meaning id to its FR-152 kind at intake
(FR-056 "Meaning binding"); this requirement is QSL's own binder contract for
resolving that binding, and the cross-record conditions FR-152's kind-mapping
and admission rules layer on top of it, into the compiler's own types.

## Inputs

- Component records: every admitted `ComponentRecord` already corresponds,
  by FR-056's meaning binding, to a construct whose meaning id is
  `quire.meaning.systems.part/v1`; this requirement classifies it Part
  unconditionally, never from a separate wire capability, because none
  exists to read.
- Endpoint records, each carrying a declared interface-type reference, an
  optional declared port direction and its owning component's declaration
  key.
- Object-type export records: every admitted export whose construct's
  meaning id is `quire.meaning.systems.interface/v1` carries its ordered
  feature signatures; this requirement classifies it Interface
  unconditionally, by the same meaning-binding argument as Part.
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

### Classification is by declared meaning id and end kind, never by display name

The model binder SHALL classify a component as `Part` whenever its construct's
meaning id is `quire.meaning.systems.part/v1`; every `ComponentRecord`
[FR-056](FR-056-admit-domain-package-model-declarations.md) admits already
carries that meaning, so this classification is total over the admitted set
and produces no no-kind cascade for a component. It SHALL classify an
object-type export as `Interface` whenever its construct's meaning id is
`quire.meaning.systems.interface/v1`, by the same total argument. It SHALL
classify an endpoint as `Port` only when it declares a port direction, its
owning component classifies as `Part`, and its declared interface-type
reference classifies as `Interface`; otherwise it SHALL record a no-kind
cascade naming the missing direction, the wrong owner kind, or the wrong
interface-type kind, respectively. It SHALL classify a relationship as
`Connection` only when both ends resolve to declared endpoints that
themselves classify as `Port`; a relationship whose both ends resolve to
declared object types classifies to no systems kind under this requirement
(see [FR-085](FR-085-resolve-relationship-end-references.md)). No
classification decision SHALL depend on a display name: two records that
share a display identity but differ in declaration key SHALL classify, and
resolve, independently.

### Every no-kind cascade is reported, not only the first

The model binder SHALL classify every interface, then every part, then every
endpoint, then every relationship, then every allocation, each in ascending
declaration-key order, charging one classification unit per record before
resolving it — quire-specification FR-152's own kind-mapping order
("interfaces, then parts, then ports, then connections, then allocations") —
and SHALL collect every no-kind cascade into the classification's refusal
list rather than stopping at the first.

### A port's interface type must itself classify as Interface

A Port additionally requires its declared interface-type reference to
classify as `Interface`; when it does not, the model binder SHALL record a
no-kind cascade with a wrong-export cause naming the required kind
(`Interface`) and the reference's actual resolved kind. This is
quire-specification FR-152's own kind-mapping exhaustiveness rule ("a port
whose interface type is not an Interface is `wrong-export` (required
Interface)").

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
| FR-086-AC-1 | Given a domain package with one endpoint missing a declared direction, a second endpoint whose owning component key names no declared node, and one interface-qualified object type, classification records both no-kind cascades (naming the missing direction and the missing owner respectively) and the `Interface` kind, all three in the classification's refusal list and kind map, not only the first found. | Test (TC-233) |
| FR-086-AC-2 | Given a Connection-classified relationship whose port-direction condition and whose per-end multiplicity condition both fail while the interface-type condition holds, `check_connection` reports both failing conditions (naming their own cause) and does not report the interface-type condition as failing; a relationship satisfying all three conditions is admitted. | Test (TC-234) |
| FR-086-AC-3 | Given an Allocation whose target element classifies as `Port` rather than `Part`, `check_allocation` refuses wrong-export naming required kind `Part` and actual kind `Port`; given a target that classifies as `Part`, the allocation is admitted. | Test (TC-235) |
| FR-086-AC-4 | Given two declarations that share the same display title but have distinct declaration keys and distinct kinds — a component classifying `Part` and an endpoint classifying `Port` — resolving each by its own key against required kind `Part` yields the correct kind for the component and a wrong-export refusal naming the endpoint's own key for the endpoint; neither declaration's classification is affected by the other's display title. | Test (TC-236) |
| FR-086-AC-5 | Kind mapping charges and classifies every interface before any part, every part before any port, every port before any connection, and every connection before any allocation; a port whose declared interface-type reference does not itself classify as `Interface` records a no-kind cascade with a wrong-export cause naming required kind `Interface` and the reference's actual kind. | Test (TC-241) |

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
- **Downstream:** none within this ticket's scope. Systems-model navigation
  over an object population — FR-152's "Navigation" section — is out of this
  requirement's scope and is tracked by
  [#147](https://github.com/agent-ix/quire-spec-language/issues/147), which
  names FR-152-AC-1/3/5/7 as its unbacked criteria.
- Relates to [FR-085](FR-085-resolve-relationship-end-references.md), which
  resolves a plain object-to-object relationship's ends; this requirement's
  Connection classification applies only once both ends already resolve to
  declared endpoints.
- Systems-model records receive no effective-declaration identity under
  [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md):
  this requirement resolves each record's own original declaration key and
  kind only, consistent with AD-006's export table, which names no export
  record for the five systems meanings beyond binding to their FR-152 kind.
- `src/model/systems.rs::classify` does not yet implement FR-086-AC-5's
  ordering or its port interface-type condition: interface types are
  collected inline while scanning every record and are never charged
  through `charge_kind`, so they are not classified as their own, first
  charged phase; and the endpoint loop (lines 246-273) checks only
  direction and owning-component kind, never whether the endpoint's
  declared interface-type reference itself classifies as `Interface`.
  FR-086-AC-5 does not ship today. Remaining work: #120.
