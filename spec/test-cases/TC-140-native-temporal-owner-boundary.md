---
id: TC-140
title: "Produce, read and evaluate canonical native temporal requests"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-052, type: verifies }
---
# TC-140: Produce, read and evaluate canonical native temporal requests

## Description

Complete owner-boundary tests for both FR-052 contracts and every acceptance
criterion. The executable Rust suite is `tests/native_temporal_owner.rs`; each
test carries `TC-140` and criterion-level trace tags.

## Test Procedure

Construct future event-position and fixed-sample requests plus pure-past
O/H/Y/S/T requests from real FR-051 checked subjects. Produce and strict-read
requests, evaluate and strict-read results, then enumerate formula semantics,
all independent axis/non-value combinations, identities, position/valuation
bijections, direct corrections and every bounded dimension. Mutate every
canonical field independently, replace a formula result with a leaf value, and
run the existing FR-043 semantic discriminators through the new owner surface.

## Expected Results

Only exact canonical requests and re-evaluated results yield constructor-private
views. Future and past truth/support match FR-043; every invalid contract,
identity, axis, population, correction or resource mutation returns one typed
failure and no partial request, result or Boolean.

## Status

All eleven traced controls pass locally with the full serial no-default-feature
suite, strict Clippy and formatting. Remaining work: #95. Review and merge
precede the Contract IR FR-026 consumer handoff.
