---
id: TC-141
title: "Preserve opaque semantic-trigger identity through native temporal v2"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-053, type: verifies }
---
# TC-141: Preserve opaque semantic-trigger identity through native temporal v2

## Description

**RETIRED (M-6d).** Verify that native-temporal v2 retains the
observation-owner semantic trigger as opaque bytes, has no textual fallback,
and remains strictly version-separated from v1. `protocol_artifact::
native_temporal` (FR-052) and its only test, `tests/it/native_temporal_owner.rs`,
are deleted: with the spine `ProtocolClause` frame slices landed, the spine
`ProtocolClause` path no longer needs this producer/consumer round trip.
FR-053's acceptance criteria this test case verified are retired for the same
reason (see FR-053-AC-1). There is no successor test case.

## Test Procedure

**RETIRED (M-6d).** ~~Construct event-triggered requests with valid
UTF-8 and non-UTF-8 trigger byte populations. Produce, evaluate and strict-read
each v2 request/result. Mutate one byte, exchange request/result triggers,
alter a correction trigger, empty the value, replace its canonical binary
encoding, and offer v1 documents to v2 and v2 documents to v1. Compare the
strict request view's decision and surrounding progress, decision and
surrounding closure, execution, and completeness reference, state, and fact
population with the exact admitted inputs. Attempt substitutions using every
non-identity field the current v1 API exposed.~~

## Expected Results

**RETIRED (M-6d).** ~~Only byte-exact v2 trigger values round-trip.
Every mutation changes identity or refuses at the source boundary. Neither
reader translates versions, invents a string nor returns partial temporal
truth. Every typed axis and completeness view retains the exact evidence
reference and value admitted into the request.~~

## Status

Retired (M-6d): `tests/it/native_temporal_owner.rs` is deleted,
and FR-053 (the requirement this test case verified) is retired in full.
