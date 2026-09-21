---
id: TC-224
title: "An ambiguous family produces no dispatch table, not even for subtypes that resolved cleanly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: verifies
---
# TC-224: An ambiguous family produces no dispatch table, not even for subtypes that resolved cleanly

## Description

Verify that when one subtype in a redefinition family is ambiguous, the
family's linking outcome carries no dispatch table at all — not a table
missing only the ambiguous subtype's entry, and not a table whose ambiguous
entry is filled with a placeholder or a first-applicable guess. Scope:
FR-083-AC-3. This is the test the "no partial substitute" exit condition
names directly: it must fail against an implementation that returns a
partially-populated result on an ambiguity failure.

Catches an implementation that builds the dispatch table incrementally,
subtype by subtype, and simply omits (or fills with a default/first-found
candidate) the entry for the one subtype that fails to resolve, returning
the rest of the table as if the family overall succeeded. A test that only
inspects the ambiguous subtype's own entry would call this passing "because
it's ambiguous"; only inspecting the *presence and type* of the table itself
for the whole family catches it.

## Test Procedure

1. Declare a redefinition family with two concrete subtypes: `Clean`, for
   which exactly one undominated candidate applies, and `Ambiguous`, for
   which two undominated candidates apply with neither dominating the
   other (as in TC-223's `C`).
2. Link the dispatch table for the family covering both `Clean` and
   `Ambiguous`.
3. Inspect the outcome's variant/type, not just its per-subtype contents.
4. If the outcome is (or contains) something resembling a table, attempt to
   read `Clean`'s entry from it.

## Expected Results

The outcome is the ambiguity variant, carrying only the named per-subtype
failures (as in TC-223), and no dispatch-table value exists in the outcome
at all — step 4 has nothing to read because no table object is present, not
because `Clean`'s entry happens to be absent from an otherwise-present
table. A mutant that returns a table containing `Clean`'s correct entry and
omits or defaults `Ambiguous`'s produces a readable table in step 4, failing
this test even though `Clean`'s entry in it would be individually correct.
