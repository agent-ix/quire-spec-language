---
id: FR-054
title: "Publish control-to-temporal activation mappings"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-052, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-449, type: references }
  - { target: ix://agent-ix/quire-specification/FR-464, type: references }
  - { target: ix://agent-ix/quire-specification/FR-478, type: references }
---
# FR-054: Publish control-to-temporal activation mappings

## Description

**RETIRED.** No requirement requires a `quire.compiled-protocol/3` artifact.
The control-to-temporal activation map is derived from the checked package's
`protocol_clause` node by [FR-449](FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md)
(QSpec FR-464 "Activation relation"), and an obligation is bound to its map
row only by QSpec FR-478's activation binding. The text below records the
retired `/3` contract; the table under "Coverage by FR-449" states where each
of its rules holds on the spine. This file and TC-143 are deleted in the change
that deletes the `/3` producer (`protocol_artifact::native::temporal_v3`) and
reader (`protocol_artifact::v3`), as QSL #952 deleted FR-009, FR-029, FR-033
and FR-034 with the `lower` command.

When an admitted protocol control can establish an event-triggered obligation,
the compiler SHALL publish its exact authored control-to-temporal-activation
mapping in a new strict `quire.compiled-protocol/3` artifact. The `/1` and `/2`
wire identities and temporal semantics stay as FR-042 and FR-050 define them.

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
| FR-054-AC-1 | **RETIRED** (see Coverage by FR-449). A valid authored mapping round-trips through canonical `/3` production and strict reading with each control and temporal declaration unchanged. | Retired |
| FR-054-AC-2 | **RETIRED** (see Coverage by FR-449). Missing, duplicate, foreign, out-of-range, non-event-triggered, reordered and substituted mappings refuse before a view is admitted. | Retired |
| FR-054-AC-3 | **RETIRED** (see Coverage by FR-449). `/1` and `/2` remain byte-identical and reject `/3`; `/3` rejects `/1` and `/2` rather than upgrading or inferring a mapping. | Retired |
| FR-054-AC-4 | **RETIRED** (see Coverage by FR-449). A profile/clock match, display name, source position, timestamp or observation cannot substitute for an independently expected mapping row. | Retired |

## Coverage by FR-449

A `/3` row named one event-control handle and one temporal declaration. An
FR-449 row names the `protocol_clause` node key, the activation that
establishes the obligation (`Each` with its trigger node key, which is the
event-triggered control of the `/3` row, or `Origin`) and the required
temporal clause node key. The 1-draft source relates a protocol to a temporal
clause only through `requires temporal`, so the protocol's activation is the
one control that establishes an obligation of that clause.

| FR-054 rule | On the spine |
| --- | --- |
| AC-1: a valid mapping round-trips through `/3` production and strict reading unchanged | The map derived from the in-process package equals the map derived after the I2 read-back, and two compiles give equal maps (FR-449-AC-1, FR-449-AC-4; QSpec FR-464-AC-13) |
| AC-2: a missing mapping refuses | A `requires temporal` name with no temporal clause refuses at S3 `missing_declaration`/`missing-name` (FR-449-AC-6); a requirement or trigger absent from the clause's `dependencies` refuses at the I2 reader `missing_declaration`/`missing-name` with no map (FR-449-AC-5) |
| AC-2: a duplicate mapping refuses | A clause required twice is one `requirements` entry and one row (FR-218, FR-449-AC-2); a set repeating an entry refuses `invalid_semantic_graph` (QSpec FR-464-AC-3) |
| AC-2: a foreign or out-of-range handle refuses | A requirement binds only a local temporal clause and refuses an imported name `missing_declaration`/`missing-name` (FR-218); a key no node of the graph carries refuses `missing_declaration`/`missing-name` (FR-449-AC-5, FR-383-AC-17) |
| AC-2: a non-event-triggered mapping refuses | Extended: an `origin` protocol's rows carry `Origin` (FR-449-AC-2), and QSpec FR-478 refuses a `trigger_event` origin on an `Origin` row and a `whole_execution` origin on an `Each` row with `invalid_runtime_input`/`conflicting-identity` (FR-478-AC-5) |
| AC-2: a reordered mapping refuses | Rows are in ascending node-id digest order; a `requirements` set out of that order refuses `invalid_semantic_graph` (QSpec FR-464-AC-3) |
| AC-2: a substituted mapping refuses | The clause node is the only carrier; retargeting a requirement changes the clause key and the `package_id` (QSpec FR-464-AC-3), and a binding whose row the package's relation does not hold refuses `missing_declaration`/`missing-name` (QSpec FR-478-AC-4) |
| AC-3: `/1` and `/2` stay byte-identical and the versions refuse each other | No versioned map wire exists: the checked package v2 nodes are the only carrier (FR-383), so there is no version to upgrade, translate or infer |
| AC-4: a profile or clock match, display name, source position, timestamp or observation cannot substitute for a row | A temporal clause sharing the protocol's clock that the protocol does not require gives no row (FR-449-AC-3); no name, position, profile, clock, timestamp or observation adds, removes or substitutes a row (FR-449 Behavior; QSpec FR-464 "Activation relation", FR-478-AC-4) |

## Dependencies

- [FR-050](FR-050-publish-authenticated-temporal-artifacts.md) owns the frozen
  `/2` temporal-selection contract.
- QSpec FR-052 owns the authored relation; QSpec FR-478 constructs the
  obligation activation binding from an FR-449 map row.
