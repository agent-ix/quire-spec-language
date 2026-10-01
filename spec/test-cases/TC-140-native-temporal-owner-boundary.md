---
id: TC-140
title: "Produce, read and evaluate canonical native temporal requests"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-052, type: verifies }
---
# TC-140: Produce, read and evaluate canonical native temporal requests

## Description

**RETIRED (M-6d).** Complete owner-boundary tests for both FR-052
contracts and every acceptance criterion. The executable Rust suite was
`tests/native_temporal_owner.rs`; each test carried `TC-140` and
criterion-level trace tags. `protocol_artifact::native_temporal::{request,
result}` and `tests/it/native_temporal_owner.rs` are deleted: with the spine `ProtocolClause` frame slices
landed, the spine `ProtocolClause` path no longer needs this
producer/consumer round trip. FR-052's acceptance criteria this test case
verified are retired for the same reason (see FR-052-AC-1). There is no
successor test case.

## Test Procedure

**RETIRED (M-6d).** ~~Construct future event-position and
fixed-sample requests plus pure-past O/H/Y/S/T requests from real FR-051
checked subjects. Produce and strict-read requests, evaluate and strict-read
results, then enumerate formula semantics, all independent axis/non-value
combinations, identities, position/valuation bijections, direct corrections
and every bounded dimension. Mutate every canonical field independently,
replace a formula result with a leaf value, and run the existing FR-043
semantic discriminators through the new owner surface.~~

## Expected Results

**RETIRED (M-6d).** ~~Only exact canonical requests and
re-evaluated results yield constructor-private views. Future and past
truth/support match FR-043; every invalid contract, identity, axis,
population, correction or resource mutation returns one typed failure and no
partial request, result or Boolean.~~

## Status

Retired (M-6d): `tests/it/native_temporal_owner.rs` is deleted,
and FR-052 (the requirement this test case verified) is retired in full.
Previously: all eleven traced controls passed locally with the full serial
no-default-feature suite, strict Clippy and formatting.
