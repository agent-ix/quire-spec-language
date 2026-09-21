---
id: TC-230
title: "A relationship end naming an undeclared type refuses the whole relationship record"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-085
    type: verifies
---
# TC-230: A relationship end naming an undeclared type refuses the whole relationship record

## Description

Verify that a relationship whose target end names a type absent from the
admitted domain package is refused with a dangling-reference cause naming
the missing type and the `target` end, admitting no binding for it, while a
relationship whose both ends resolve is admitted normally. Scope:
FR-085-AC-1.

Catches an implementation that resolves the source end, fails to resolve the
target end, and either silently admits a one-ended (partial) relationship
binding or admits the relationship with the target end left `None`/absent
rather than refusing the record outright — invisible to a test that never
checks what happens to a relationship's *other*, correctly-resolving end
when one end is dangling.

## Test Procedure

1. Admit a domain package declaring object type `Customer` and a
   relationship record whose source end names `Customer` and whose target
   end names `Order`, where `Order` is not declared anywhere in this domain
   package.
2. Resolve this relationship record's ends.
3. Admit a second domain package declaring both `Customer` and `Order`, with
   the same relationship shape, and resolve its ends.
4. For step 2, if any binding-like value is produced, inspect whether it
   carries a source end.

## Expected Results

Step 2 refuses with a dangling-reference cause naming `Order` and `target`,
and produces no relationship binding at all — step 4 finds nothing to
inspect because no binding value exists, not because its target field is
empty on an otherwise-present binding. Step 3 resolves successfully with
both ends bound. A mutant that admits a one-ended binding for step 2
produces a value with a populated source end and an absent target, passing a
shallow "did resolution throw" check but failing the no-binding-at-all
assertion.
