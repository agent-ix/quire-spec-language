---
id: FR-053
title: "Enforce choice ownership and information visibility"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
---
## Description

When admitting a labeled protocol choice, the protocol linker SHALL require one owning role and evidence that the owner can distinguish every selected branch from facts available at its decision point.

## Inputs

A finite choice node, owning role, labeled guards, predecessor graph and declared
role-visible facts/messages.

## Outputs

An admitted owned choice with explicit visibility premises, or a located typed
unobservable-choice refusal.

## Behavior

The linker SHALL reject ownerless, multiply owned, unlabeled and overlapping
choices. The linker SHALL reject a choice when its distinguishing fact is
available only to another role and no causal message exposes it to the owner.
Rejecting an unobservable choice SHALL NOT imply that every global trace is
invalid or that a general projection theorem has been attempted.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-053-AC-1 | A fulfillment-owned ship/cancel choice is admitted when an earlier message supplies the distinguishing payment outcome. | Test (TC-053) |
| FR-053-AC-2 | The same choice is refused when the payment outcome remains visible only to the external provider. | Test (TC-053) |
| FR-053-AC-3 | Ownerless, multiply owned, duplicate-label and guard-overlap mutants are refused independently. | Test (TC-053) |
| FR-053-AC-4 | The refusal identifies the exact choice, owner and unavailable distinguishing fact without reporting global conformance success. | Test (TC-053) |

## Dependencies

- [FR-052](./FR-052-represent-bounded-control.md).
- Agent A's typed predicate and scope rules.
