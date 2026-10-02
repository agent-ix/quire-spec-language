---
id: TC-818
title: "S3 refuses ill-formed union declarations and bounds them only by checking ceilings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-316
    type: verifies
---
# TC-818: S3 refuses ill-formed union declarations and bounds them only by checking ceilings

## Description

Scope: FR-316-AC-2, FR-316-AC-3, FR-316-AC-4.

## Test Procedure

1. Check `union L { Nil, Cons(Integer, L) }`.
2. Check `record Box { inner: U; }` with `union U { Wrap(Box) }`.
3. Check `union U { A, A(Integer) }` and `union U { A(Missing) }`.
4. Run QSpec TC-263's refused cases (tag `QSpec-TC-263`).
5. Check a generated union of 5,000 nullary members, and a union whose one
   payload type nests `Option<...>` 1,000 levels, under default ceilings.
6. Check the 5,000-member union with the work ceiling one below its charge,
   then with the ceiling raised by one.

Tag each test `#[trace("TC-818", "<AC id>")]`.

## Expected Results

- Step 1: `ill_typed`, recursion subgraph non-escaping, cycle `L, L`.
- Step 2: `ill_typed` naming the cycle through `Box` and `U`, at both.
- Step 3: `invalid_semantic_graph` naming `A`; the unresolved-type refusal
  naming `Missing` at its span.
- Step 4: TC-263's verdicts.
- Step 5: both admitted.
- Step 6: `StageFailure::Limit`, kind work budget, the configured bound and
  the `CheckingLimits` field; then admitted. No nesting-depth limit kind
  appears anywhere.

## Status

Planned.
