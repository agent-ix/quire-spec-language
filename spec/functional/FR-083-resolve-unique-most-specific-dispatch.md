---
id: FR-083
title: "Resolve unique most-specific dispatch with no partial substitute"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
---
# FR-083: Resolve unique most-specific dispatch with no partial substitute

## Description

When linking a dispatch table for an operation's redefinition family, the
model checker SHALL select, for each concrete conforming subtype of the
operation's receiver type, the unique undominated redefinition applicable to
it. The model checker SHALL produce a complete dispatch table only when every
subtype in the family resolves to exactly one such candidate. An ambiguous or
incomplete family SHALL produce no dispatch table, including no entry for a
subtype that itself resolved cleanly.

## Inputs

- The effective view and conformance relation
  [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  and [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md)
  produce.
- The redefinition family of one root operation: the operation and every
  member that transitively redefines it, reached only through declared
  `redefines` edges.
- The `abstract` member of each effective object type.
- `ModelNormalizationLimitsV1`.

## Outputs

A `DispatchLinkOutcome`: a linked dispatch table mapping each concrete
conforming subtype to its unique undominated candidate, or an ambiguity
result naming every subtype whose resolution failed and the competing
candidates for each, or a typed incomplete result naming the exhausted charge
point.

## Behavior

### Applicability and dominance, not registration order

For each concrete (non-abstract) effective subtype conforming to the root
operation's receiver type, the model checker SHALL compute the family
members whose owner the subtype conforms to as that subtype's applicable set.
It SHALL determine that a candidate `p` dominates a candidate `q` exactly
when `p`'s owner is a proper descendant of `q`'s owner in the conformance
relation. The model checker SHALL select the subtype's undominated set as its
applicable set minus every candidate some other applicable candidate
dominates. Registration order, source order or declaration order SHALL NOT
influence applicability, dominance or selection.

### Unique undominated candidate, or a named ambiguity

If a subtype's undominated set has exactly one member, the model checker
SHALL select that member as the subtype's linked dispatch candidate. If a
subtype's applicable set is empty, the model checker SHALL record a
no-applicable-candidate failure for that subtype. If a subtype's undominated
set has more than one member, the model checker SHALL record a
multiple-undominated-candidates failure for that subtype, naming every member
of the undominated set and the dominance relation among the family's
candidates.

### No partial substitute on ambiguity or incompleteness

If any subtype in the family fails to resolve to a unique undominated
candidate, the model checker SHALL produce no dispatch table for that family:
it SHALL NOT return a table containing entries only for the subtypes that did
resolve, and it SHALL NOT default an unresolved subtype's entry to an
arbitrary or first-applicable candidate. The ambiguity result SHALL carry
only the named per-subtype failures, never a table.

### Abstract types dispatch to no direct instance

An effective object type declared `abstract` SHALL contribute no candidate
receiver to the family's concrete-subtype enumeration; dispatch resolves only
over its concrete descendants.

### Family enumeration is bounded

The model checker SHALL bound the depth of the redefinition-family walk. If
the walk would exceed the bound, the model checker SHALL return a typed
incomplete result naming the bound and SHALL NOT report a linked table or an
ambiguity result for that family.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-083-AC-1 | Given a redefinition family with two branches applicable to one concrete subtype where one branch's owner is a proper descendant of the other's, the linked table selects the descendant's redefinition for that subtype; permuting the family members' declaration order does not change the selection. | Test (TC-222) |
| FR-083-AC-2 | Given a concrete subtype with no applicable family member, linking reports a no-applicable-candidate failure for it; given a concrete subtype with two undominated candidates, linking reports a multiple-undominated-candidates failure naming both candidates and the dominance relation among the family. | Test (TC-223) |
| FR-083-AC-3 | Given a family where one subtype is ambiguous and a second subtype in the same family would resolve cleanly on its own, the outcome carries no dispatch table at all — not even an entry for the second, cleanly-resolving subtype. | Test (TC-224) |
| FR-083-AC-4 | Given a redefinition family whose chain of `redefines` edges exceeds the bound, linking returns a typed incomplete result naming the bound and reports neither a linked table nor an ambiguity result; a family at exactly the bound links successfully. | Test (TC-225) |

## Dependencies

- **Upstream:** [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md)
  and [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md)
  supply the effective declarations and conformance relation this requirement
  dispatches over; quire-specification FR-151 owns the normative dispatch
  rule ("registration, source or declaration order never resolves
  ambiguity, and no linearization is applied") and AD-006 owns the model-view
  decision to keep closed dispatch sets in the checked model view.
- **Downstream:** the value evaluator consumes a linked dispatch table to
  resolve a dispatched call's implementation; it never receives a table for a
  family this requirement reports ambiguous or incomplete.
- This requirement's behavior touches the same compiler surface as
  [FR-068](FR-068-split-expression-checking-into-check-stage.md), which
  declares a bounded, temporary `model` → `check` import edge confined to
  exactly the names FR-068-AC-9 and FR-068-CON-5 list. This requirement does
  not alter, widen or contradict that edge; it specifies dispatch-linking
  behavior only.
