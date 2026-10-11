---
id: TC-973
title: "Composed admission stage projection preserves stage ownership"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-973: Composed admission stage projection preserves stage ownership

## Description

Verify `composed_admission_stages` agrees with FR-057's stage ownership for all
twelve kinds. Scope: FR-057-AC-14.

## Test Procedure

For every kind in FR-057's table:

1. Admit a requested pair through the composed linker while varying whether a
   registry contains a matching backend; for `probabilistic-satisfaction`,
   give the item the evidence kind `statistical`.
2. Admit a backend descriptor advertising the kind, supplying
   `{statistical}` evidence kinds for `probabilistic-satisfaction`, then
   inspect the registry and its candidate result for the matching item.
3. Compare the reported stage projection with the stage owning each operation.

## Expected Results

- Requested-pair admission is owned by the composed linker and has no registry
  or backend input.
- Advertised-label validation is owned by `BackendDescriptor::admit`.
- Candidate computation occurs in the registry only after descriptor
  admission. The probabilistic fixture's matching `statistical` evidence kind
  yields the backend as a candidate. The stage projection does not move or
  combine these decisions for either new kind.
