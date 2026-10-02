---
id: TC-220
title: "A conformance ancestor walk that exhausts its edge limit stops incomplete instead of truncating"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
---
# TC-220: A conformance ancestor walk that exhausts its edge limit stops incomplete instead of truncating

## Description

Verify that a conformance walk whose `ancestor_steps` charge is denied stops
`Incomplete` naming the limit, its value and the count reached, and reports
neither conformance nor non-conformance, while the same walk completes once
the limit fits. Scope: FR-082-AC-3, FR-082-AC-6 for the expression checker
sharing the same limit (at check time a stage limit, `LimitExceeded` edge
count, ADR-014 B-3), and FR-082-AC-7 for the checker's admission work
budget: a chain too large for the budget stops at a work-budget
`LimitExceeded` naming it, and a linear chain's admission work stays within
four units per flattened slot.

Catches an implementation that silently truncates the ancestor walk at the
limit and returns `false` (non-conformant) for the unreached remainder — a
defect indistinguishable from a correct "does not conform" result unless the
test checks that the outcome is the distinct `Incomplete` variant, not a
`false` conformance verdict.

## Test Procedure

1. Declare a linear chain of object types (each `supertypes: [previous]`)
   whose deepest type's conformance walk to the shallowest follows `n`
   edges.
2. Query that conformance with `ancestor_steps` at `n − 1`, then at `n`.
3. Admit the chain into the expression checker's type environment with
   `ancestor_steps` at `n − 1`, then at `n`, and ask the same question.
4. Admit object types whose closure and flattening cost more than the
   admission `work_units` budget; admit a linear chain and count its work
   per flattened slot.
5. On a thread with a 512 KiB stack, query the conformance of a
   10,000-long chain at the default `ancestor_steps`.

Tag the tests `#[trace("TC-220", "<AC id>")]`.

## Expected Results

- Step 2: at `n − 1` the result is `Incomplete` naming `ancestor_steps`,
  bound `n − 1` and count `n`, not a boolean; at `n` it is a completed
  verdict of `true`. A truncate-and-return-`false` mutant fails the
  result-type assertion.
- Step 3: at `n − 1` admission returns `LimitExceeded` with limit kind edge
  count naming `ancestor_steps`; at `n` it admits and answers `true`, as
  step 2 does.
- Step 4: `LimitExceeded` with limit kind work budget naming `work_units`;
  the linear chain stays within four units per flattened slot.
- Step 5: the walk completes with `true`, and no outcome names a depth.
