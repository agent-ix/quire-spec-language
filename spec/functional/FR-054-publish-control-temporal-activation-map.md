---
id: FR-054
title: "Publish control-to-temporal activation mappings"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-052, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-300, type: depends_on }
---
# FR-054: Publish control-to-temporal activation mappings

## Description

When an admitted protocol control can establish an event-triggered obligation,
the compiler SHALL publish its exact authored control-to-temporal-activation
mapping in a new strict `quire.compiled-protocol/3` artifact. The compiler SHALL
leave the frozen `/1` and `/2` bytes, readers and semantics unchanged.

## Inputs

One FR-050-admitted `/2` package, the exact authored activation relation for
each eligible declaration-local control, and independent reader expectations
for every relation. A control position, temporal declaration position, matching
profile/clock, display name, timestamp or observation is not a relation input.

## Outputs

One canonical `/3` document and constructor-private admitted view retaining the
exact control handle and temporal declaration selected by each relation, or one
typed invalid, unsupported or resource-incomplete refusal with no partial view.

## Behavior

The `/3` producer SHALL retain every admitted `/2` member unchanged and append
one canonical mapping table. Each row SHALL name one in-graph control handle and
one in-graph temporal declaration. The producer SHALL reject a missing,
duplicate, foreign, out-of-range, non-event-triggered or noncanonical relation.

The `/3` strict reader SHALL compare the complete mapping table against an
independently supplied expected relation population before granting admission.
The `/3` strict reader SHALL reject any substitution, reordering, surplus or
inferred relation and SHALL NOT derive a relation from profile/clock equality or
source-table order.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-054-AC-1 | A valid authored mapping round-trips through canonical `/3` production and strict reading with each control and temporal declaration unchanged. | Test (TC-143) |
| FR-054-AC-2 | Missing, duplicate, foreign, out-of-range, non-event-triggered, reordered and substituted mappings refuse before a view is admitted. | Test (TC-143) |
| FR-054-AC-3 | `/1` and `/2` remain byte-identical and reject `/3`; `/3` rejects `/1` and `/2` rather than upgrading or inferring a mapping. | Test (TC-143) |
| FR-054-AC-4 | A profile/clock match, display name, source position, timestamp or observation cannot substitute for an independently expected mapping row. | Test (TC-143) |

## Dependencies

- [FR-050](FR-050-publish-authenticated-temporal-artifacts.md) owns the frozen
  `/2` temporal-selection contract.
- QSpec FR-052 owns the authored relation and FR-300 consumes the strict `/3`
  mapping when constructing an obligation activation binding.
