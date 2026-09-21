---
id: TC-226
title: "Population admission distinguishes unknown closure from a genuine refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
---
# TC-226: Population admission distinguishes unknown closure from a genuine refusal

## Description

Verify that an `open`-extent population reports the distinct unknown-closure
admission outcome carrying no binding, while a member of an undeclared type
is a genuine refusal, and a fully closed, fully covered population admits.
Scope: FR-084-AC-1.

Catches an implementation that folds the unknown-closure case into the same
result variant as a genuine refusal (so a caller cannot tell "this
population just isn't closed" from "this input is actually malformed") —
plausible because both currently "fail to produce a binding," so a test
checking only "admission did not succeed" cannot tell them apart. It also
catches an implementation that treats `open` extent as an ordinary success
with an implicitly-assumed-closed binding.

## Test Procedure

1. Declare a population artifact with extent `open`, listing one member type
   `Order`, and admit a binding with one well-typed `Order` member.
2. Declare a second population artifact, extent `closed`, listing member
   type `Order`, and admit a binding containing one member whose
   most-specific type is `LineItem`, not `Order` and not covered by the
   population's declared member types.
3. Declare a third population artifact, extent `closed`, member type
   `Order`, with every member correctly typed as `Order`, and admit it.
4. Inspect the result variant (not just success/failure) of each of the
   three admissions.

## Expected Results

Step 1 yields the distinct unknown-closure outcome, carrying no binding.
Step 2 yields a genuine refusal (foreign type), also carrying no binding but
of a different result variant than step 1. Step 3 yields `Admitted` with a
usable binding. A mutant that reports steps 1 and 2 as the same refusal
variant fails the variant-distinctness assertion even though both correctly
"fail."
