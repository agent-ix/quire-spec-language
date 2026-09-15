---
id: Plan-011
title: "Opaque native-temporal trigger identity"
type: Plan
status: complete
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-053
    type: references
---
# Plan-011: Opaque native-temporal trigger identity

## Scope

Deliver FR-053 as a strict, versioned v2 native-temporal request/result pair.
The slice retains an observation-owner supplied opaque byte identity in canonical
preimages and strict readers while preserving v1 unchanged. QProtocol consumes
the published v2 boundary separately; QSL neither imports QObs nor constructs a
protocol obligation.

## Dependency and ownership

FR-052's validated subject, canonical encoding, evaluator and result lineage are
the only implementation dependencies. FR-053 owns the v2 adapter and its
version separation. QObs owns observation identity selection; QProtocol owns
FR-300 activation-site binding.

## Test plan

TC-141 first establishes non-UTF-8 round-trip, mutation and substitution
controls, correction immutability, malformed/empty rejection and v1/v2 refusal.
Existing FR-052 owner tests remain the v1 regression control.

## Execution

| Task | Track | Status |
| --- | --- | --- |
| [Task-039](tasks/Task-039-opaque-temporal-trigger.md) | serial | done |

One serial task is required: the v2 request type must exist before result
evaluation and strict reading can bind it. PR-time Rust/spec/gap review occurs
after the complete slice and local gates, not between substeps.
