---
id: TC-223
title: "No-applicable-candidate and multiple-undominated-candidates are named separately"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: verifies
---
# TC-223: No-applicable-candidate and multiple-undominated-candidates are named separately

## Description

Verify that a subtype with zero applicable family members reports a
no-applicable-candidate failure, and a subtype with two undominated
candidates reports a multiple-undominated-candidates failure naming both
candidates and the dominance relation, and that these two failure shapes are
not collapsed into one generic "dispatch failed" report. Scope:
FR-083-AC-2.

Catches an implementation that reports both failure modes under one
undifferentiated cause (losing the distinction a caller needs to tell "no
one handles this" from "more than one candidate could, pick one"), and
separately catches one that names only one of two competing undominated
candidates in the ambiguity report rather than the full undominated set.

## Test Procedure

1. Declare a root operation on abstract type `A`, redefined independently on
   two unrelated concrete subtypes `B1` and `B2` (neither a supertype nor
   subtype of the other), both conforming to a common concrete subtype `C`
   through multiple generalization, with no redefinition on `C` itself and
   neither `B1` nor `B2` dominating the other.
2. Declare a second concrete subtype `D` conforming to `A` through a branch
   that supplies no redefinition applicable to `D` at all.
3. Link the dispatch table for the family covering both `C` and `D`.
4. Inspect the per-subtype failures the outcome names for `C` and for `D`.

## Expected Results

`D`'s failure is no-applicable-candidate. `C`'s failure is
multiple-undominated-candidates, naming both `B1`'s and `B2`'s redefinitions
in its candidate set. The two failures are distinguishable by type/cause,
not merged into one shape. A mutant that reports both as one generic
"ambiguous" cause fails the distinguishability assertion; a mutant that
names only `B1` (or only `B2`) in `C`'s failure fails the candidate-set
completeness assertion.
