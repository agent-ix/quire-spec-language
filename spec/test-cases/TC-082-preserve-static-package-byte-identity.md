---
id: TC-082
title: "Preserve static package byte identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: verifies
---
# TC-082: Preserve static package byte identity

## Description

Property, priority P1. Verifies FR-019-AC-8, FR-019-AC-9, FR-020-AC-10, FR-021-AC-2. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Generate repeated builds and one-axis source/model/authored-binding mutations. Independently compare exact emitted bytes and SHA-256. Run several runtime populations and budgets against the same package. Reencode object member order and unique feature order for readback with their own correct byte selectors, then reorder clauses/imports/occurrences.

## Expected Results

Unchanged static inputs reproduce bytes; runtime changes do not alter them. Accepted representation variants retain their own raw bytes/digest but reconstruct the same checked meaning. Ordered-array changes refuse, with no global array sorting.

Accepted member/feature representation variants have the same NativePackageIdentity
and independently different raw ByteDigests; neither role substitutes for the other.
