---
id: FR-303
title: "Keep the model correspondence one-to-one"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-150
    type: depends_on
---
# FR-303: Keep the model correspondence one-to-one

## Description

When the S3 checker records a checked node's domain declaration in the
`ModelCorrespondence`, the checker SHALL keep the correspondence one-to-one
in both directions. If a record would give one `NodeKey` a second, different
`DeclarationKey`, or one `DeclarationKey` a second, different `NodeKey`, the
checker SHALL raise an internal fault and SHALL produce no checked package.

## Inputs

- One `(NodeKey, DeclarationKey)` pair the S3 checker records while lowering
  a model declaration (ADR-013 O-04).
- The `ModelCorrespondence` recorded so far in the same check.

## Outputs

The correspondence with the pair recorded, or an internal fault naming the
existing pair and the new pair.

## Behavior

The S3 checker is the only writer of the correspondence (ADR-013 O-04).
Recording a pair that is already present SHALL leave the correspondence
unchanged. Recording `(n, d2)` when `(n, d1)` is present with `d1 ≠ d2`, or
`(n2, d)` when `(n1, d)` is present with `n1 ≠ n2`, SHALL return an internal
fault `runtime_invariant`/`established-invariant-broken` naming both pairs.
The fault SHALL stop the check: the compile refuses with that cause, its O-16
category is internal failure, and no checked package is produced. A
`NodeKey` SHALL resolve to its `DeclarationKey`, and a `DeclarationKey` to its
`NodeKey`, only through the recorded correspondence (FR-088-AC-2).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-303-AC-1 | Recording `(n, d1)` then `(n, d2)` with `d1 ≠ d2` returns `runtime_invariant`/`established-invariant-broken` naming `n`, `d1` and `d2`, and `n` still resolves to `d1`. | Test (TC-796) |
| FR-303-AC-2 | Recording `(n1, d)` then `(n2, d)` with `n1 ≠ n2` returns `runtime_invariant`/`established-invariant-broken` naming `n1`, `n2` and `d`, and `n2` resolves to no declaration. | Test (TC-796) |
| FR-303-AC-3 | Recording `(n, d)` twice succeeds, and the correspondence holds one entry for `n`. | Test (TC-796) |

## Dependencies

- **Upstream:** ADR-013 O-04 makes `check` the only writer; ADR-016 §2
  ("Original and effective provenance") requires the correspondence to be
  one-to-one; [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
  keys the model-owned nodes; QSpec FR-150 fixes the declaration key.
- **Downstream:** [FR-088](FR-088-clause-name-and-type-identity.md)'s
  `resolve_declaration`, replay's `WireNodeId` → `NodeKey` lookup (ADR-016
  ID-3), and the abstraction relation's key resolution
  ([FR-304](FR-304-check-an-authored-abstraction-relation.md)).

## References

- ADR-016 §2 and §9 G-8.
- Linear QSL-382 (specification), QSL-68 (implementation).
