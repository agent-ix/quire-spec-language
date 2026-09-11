---
id: TC-041
title: "Refuse missing or inconsistent native model roles"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: verifies
---
# TC-041: Refuse missing or inconsistent native model roles

## Description

Integration, priority P1. Verifies FR-015-AC-2. Qualified at 0cd679c; SR-085
records the actual public-API controls, local gates and bounded claim. Tests use
the source-derived Rust producer and successfully constructed actual IR inputs.
Task-034 separately qualifies the added sequence-declaration ceiling.

## Test Procedure

Starting from the valid producer, independently remove a scalar site, repeat a site/role, change one site's integer representation, select a nonprimitive site, use unsigned/saturating/rational formal data, exceed the authored text ceiling, supply an invalid object/reference carrier, or break an operation parameter/result/frame target. Carrier controls include extra/missing fields, a non-Text ID, zero ID maximum and conflicting object/carrier roles. Include unused declarations and duplicate frame entries. Construct IR inputs successfully before testing native-only adverse cases.

Construct sequence declarations with maxima 10,000, 10,001 and u32::MAX through
both actual IR constructors and the JSON model frontend. Cover a used field,
an unused record field, an unused value, and inner/outer collection wrappers
under options. Confirm that decoding into a ModelDraft succeeds before judging
admission, and that raising model work limits cannot admit an oversized maximum.

## Expected Results

Each invalid model returns the specified invalid_model_binding or unsupported_construct diagnostic with no NativeModel. IR constructor failures cannot replace the native refusal assertions. A valid bounded text role including maximum zero is accepted; missing text role is refused. Reusing valid source/roles after each failure succeeds.

Sequence maximum 10,000 is retained exactly in admitted declarations. Larger
maxima refuse with unsupported_construct and original model-source provenance,
without an incomplete result or partial artifact. Source intake retains the
typed Admission cause. No runtime population of 10,000 elements is needed to
test a declaration ceiling.
