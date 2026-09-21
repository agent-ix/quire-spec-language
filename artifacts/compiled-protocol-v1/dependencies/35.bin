---
id: FR-043
title: "Evaluate identity and reachability in explicit finite graphs"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When evaluating an admitted finite-graph relation, the evaluator SHALL use the declared typed universe and qualified snapshot without substituting structural records, ambient lookups or a path cutoff.

## Inputs

A complete finite typed object environment, exact universe/snapshot identities, references, resolved edge field and explicit work limits.

## Outputs

An identity/reachability result with retained bindings, or a typed model/binding/refusal or incomplete execution.

## Behavior

Identity equality uses universe, stable object type and object ID, preserving snapshot qualifiers for reads. Traversal requires the same type/universe/snapshot and one-or-more edges over Ref(T), Option(Ref(T)) or ordered Seq(Ref(T),N). Check a discovered target before suppressing repeated expansion; expand each identity at most once. Reference cycles are permitted while structural containment remains acyclic. No opaque scalar gains navigation or implicit heterogeneous relation semantics.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-043-AC-1 | The same typed object identity compares equal across pre/post of one universe while its field reads keep their respective observations; matching labels in different universes/types cannot substitute. | Test (TC-043) |
| FR-043-AC-2 | An isolated object does not reach itself; a self-loop or cycle back to the start does. Empty/absent outgoing edges stop expansion. | Test (TC-043) |
| FR-043-AC-3 | Branching ordered edges and duplicates preserve the reachability result without repeated expansion; a path longer than an implementation convenience threshold is not reported unreachable. | Test (TC-043) |
| FR-043-AC-4 | Duplicate objects, dangling targets in a declared-complete universe, mismatched traversal snapshots/types and unsupported edge shapes refuse; unavailable completeness stays incomplete. | Test (TC-043) |
| FR-043-AC-5 | Work exhaustion returns incomplete rather than false; an opaque UUID or recursive record cannot act as the missing graph representation. | Test (TC-043) |

## Dependencies

- [Detailed draft contract](../../proposals/quire-v1/state-contract.md).
- [Shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed composed-v1 obligation. Historical definitions and their acceptance
remain separate; this artifact does not establish implementation coverage.
