---
id: TC-228
title: "lookup never defaults an unresolved closure to the declared absence mode"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
---
# TC-228: lookup never defaults an unresolved closure to the declared absence mode

## Description

Verify that a `lookup<T>` query whose key genuinely matches no member
returns the declared absence mode, while a query against a binding whose
subtype closure for `T` is not established returns the distinct
incomplete/unknown-closure outcome, never the absence mode standing in for
it. Scope: FR-084-AC-3.

Catches an implementation that treats "I can't establish whether this key
resolves" the same as "this key resolves to nothing" — an easy conflation,
since both currently mean "no reference is returned," but conflating them
lets a caller mistake an unproven absence for a proven one and reason
unsoundly about it downstream.

## Test Procedure

1. Declare a population with extent `closed`, member type `Item`, fully
   covering `Item`'s subtypes, and admit a binding with two `Item` members,
   neither matching key `r`.
2. Query `lookup<Item>(p, r) absent Undefined` against this binding.
3. Declare a second population whose declared member types do not cover
   every effective subtype conforming to `Item` (as in TC-227's step 3
   setup), and query `lookup<Item>(p, r) absent Undefined` against it for a
   key `r` that would, if closure held, plausibly not match any member.
4. Compare the result variants of step 2 and step 3.

## Expected Results

Step 2 returns `Undefined` (the declared absence mode), a genuine, completed
result. Step 3 returns the distinct incomplete/unknown-closure outcome, a
different result variant from step 2's, even though both "found no match."
A mutant that returns `Undefined` in both cases collapses the two, failing
the variant-distinctness assertion.
