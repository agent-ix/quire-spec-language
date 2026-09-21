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
admission outcome carrying no binding, that a closed-extent population whose
generalization graph is itself not closed reports the same distinct
unknown-closure outcome (naming the unresolved type), that a member of an
undeclared type is a genuine refusal, and that a fully closed, fully
covered population with a closed generalization graph admits. Scope:
FR-084-AC-1. This also covers the case moved from TC-228: the
generalization-graph-closure scenario is only constructible at admission,
never against an already-admitted binding, so it belongs here.

Catches an implementation that folds the unknown-closure case into the same
result variant as a genuine refusal (so a caller cannot tell "this
population just isn't closed" from "this input is actually malformed") —
plausible because both currently "fail to produce a binding," so a test
checking only "admission did not succeed" cannot tell them apart. It also
catches an implementation that treats `open` extent, or an unclosed
generalization graph, as an ordinary success with an implicitly-assumed-closed
binding.

## Test Procedure

1. Declare a population artifact with extent `open`, listing one member type
   `Order`, and admit a binding with one well-typed `Order` member.
2. Declare a second population artifact, extent `closed`, member type
   `Order`, over a domain package whose generalization graph is not itself
   closed, and admit it with `GeneralizationClosure::Open` asserted.
3. Declare a third population artifact, extent `closed`, listing member
   type `Order`, and admit a binding containing one member whose
   most-specific type is `LineItem`, not `Order` and not covered by the
   population's declared member types.
4. Declare a fourth population artifact, extent `closed`, member type
   `Order`, with every member correctly typed as `Order`, over a domain
   package with a closed generalization graph, and admit it.
5. Inspect the result variant (not just success/failure) of each of the
   four admissions.

## Expected Results

Step 1 yields the distinct unknown-closure outcome, carrying no binding.
Step 2 likewise yields the distinct unknown-closure outcome, carrying no
binding, naming the type whose subtypes are unresolved — the same result
variant as step 1, from a different cause (generalization-graph closure,
not extent). Step 3 yields a genuine refusal (foreign type), also carrying
no binding but of a different result variant than steps 1 and 2. Step 4
yields `Admitted` with a usable binding. A mutant that reports step 3 with
the same variant as steps 1/2 fails the variant-distinctness assertion even
though all three correctly "fail."
