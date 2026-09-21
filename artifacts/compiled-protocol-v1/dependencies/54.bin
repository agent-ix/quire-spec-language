---
id: FR-090
title: "Select an exact temporal profile and clock"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-008
    type: implements
  - target: ix://agent-ix/quire-specification/FR-030
    type: depends_on
---
## Description

When admitting a native temporal clause, the frontend shall bind the clause to
one exact temporal semantic-profile definition and one exact clock
interpretation before evaluation or lowering.

## Inputs

An admitted native clause; exact language edition and temporal profile
definition identity, revision and digest; typed interval bounds; and one clock
binding supplied through the shared observation interface.

## Outputs

A temporal subject carrying the selected profile, clock and interval meaning,
or a located typed refusal that names the missing, unknown, inconsistent or
unsupported dimension.

## Behavior

The temporal profile shall select exactly one of these v1 interpretations:

| Profile identity | One interval tick means | Closed-boundary rule |
| --- | --- | --- |
| `quire.temporal.event-position.false-extension/v1` | One admitted semantic-event position in an authoritative sequence | Atomic predicates are false after closure; constants retain their value |
| `quire.temporal.fixed-sample.false-extension/v1` | One sample of the declared positive rational period from the declared epoch | Atomic predicates are false after closure; constants retain their value |
| `quire.temporal.timestamped-event.finite-window/v1` | One tick in the declared timestamp unit | Quantification ranges over admitted event instants in a complete bounded window |

The event-position profile shall not claim elapsed time.

The fixed-sample profile shall retain epoch, period and unit identity.

The fixed-sample profile shall classify an absent runtime sample under a valid
binding as incomplete. The frontend shall refuse a malformed, incompatible or
missing fixed-sample clock definition or binding before evaluation.

The timestamped-event profile shall retain clock identity, unit, admitted order
and the completeness premise for every evaluated window.

The temporal evaluator shall not use timestamp order alone to establish
causality or break a tie for an order-sensitive operator.

The frontend shall not infer a profile, unit, period, origin, order or boundary
rule from backend availability, file shape, ingestion order or a similarly
spelled historical profile.

When a temporal meaning changes, the frontend shall require a new profile
definition identity; old profile bytes and results remain historical subjects.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-090-AC-1 | The same predicate values and numeric interval under event-position, fixed-sample and timestamped-event selections retain three distinct temporal subjects and meanings. | Test (TC-110) |
| FR-090-AC-2 | Unknown, stale, digest-mismatched, multiply selected or internally inconsistent profile/clock definitions refuse before evaluation and name the affected clause. | Test (TC-110) |
| FR-090-AC-3 | Changing a sample period, epoch, timestamp unit, sequence authority or closed-boundary rule changes the binding identity and cannot reuse an earlier result. | Test (TC-113) |
| FR-090-AC-4 | Equal timestamps without an admitted causal/sequence order refuse `until`, `release`, `since` and `triggered`; ingestion order is not substituted. | Test (TC-113) |
| FR-090-AC-5 | Historical profile definitions remain unchanged and a backend capability request cannot silently select a different executable profile. | Test (TC-117) |

## Dependencies

- [FR-030](./FR-030-bind-composed-definitions.md) admits the exact language
  edition and temporal definition closure consumed here. FR-020 reports
  composed release capability and is not an admission prerequisite.
- Agent F's observation contract supplies clock bindings and completeness
  assertions; this requirement defines their temporal interpretation, not
  their transport or authority schema.
