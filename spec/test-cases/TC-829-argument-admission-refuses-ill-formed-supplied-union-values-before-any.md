---
id: TC-829
title: "Argument admission refuses ill-formed supplied union values before any charge"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-321
    type: verifies
---
# TC-829: Argument admission refuses ill-formed supplied union values before any charge

## Description

Scope: FR-321-AC-1, FR-321-AC-2.

## Test Procedure

Run `area` (FR-318-AC-1) with each supplied argument, recording meter charges:

1. a union value keyed by a union of another package;
2. a value whose `VariantId` is another union's member, and one that is an
   enum member's;
3. `Rect` with one payload value;
4. `Circle` with payload `true`;
5. `Rect(2, 3)`.
6. Over `union Holder { Some(Reference<M::T>), Nothing }`, supply
   `Holder::Some` with a reference that has no target in a declared-complete
   population.

Tag each test `#[trace("TC-829", "<AC id>")]`.

## Expected Results

- Steps 1 to 4: `invalid_runtime_input`/`wrong-value-kind`; no evaluation
  and no charge recorded.
- Step 5: admitted; the run returns 6.
- Step 6: refused by the reference walk with its existing code, before
  evaluation.

## Status

Planned.
