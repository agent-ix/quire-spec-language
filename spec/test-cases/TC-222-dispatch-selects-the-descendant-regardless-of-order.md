---
id: TC-222
title: "Dispatch selects the descendant candidate independent of declaration order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: verifies
---
# TC-222: Dispatch selects the descendant candidate independent of declaration order

## Description

Verify that when two redefinition-family branches are applicable to one
concrete subtype and one branch's owner is a proper descendant of the
other's, linking selects the descendant's redefinition, and that permuting
the family members' declaration order never changes that selection. Scope:
FR-083-AC-1.

Catches an implementation that selects by declaration or registration order
("last one wins" or "first one wins" over the family member list) instead of
by dominance — a defect invisible to a test authored with the family members
in only one order, since a fixed order can coincidentally agree with the
correct dominance-based answer.

## Test Procedure

1. Declare a root operation on type `A`, redefined on `B` (`supertypes:
   [A]`) and again on `C` (`supertypes: [B]`), so `C`'s redefinition
   dominates `B`'s for any subtype conforming to `C`.
2. Link the dispatch table for a concrete subtype conforming to `C`, with
   the family members supplied to the linker in declaration order
   (`A`, `B`, `C`).
3. Re-link the same family with the members supplied in reverse order
   (`C`, `B`, `A`) and in one other permutation (`B`, `A`, `C`).
4. Compare the selected candidate for the subtype across all three linking
   runs.

## Expected Results

All three runs select `C`'s redefinition for the subtype. A mutant that
picks the first or last member of the supplied family list (rather than
computing dominance) selects a different candidate under at least one
permutation, failing the comparison.
