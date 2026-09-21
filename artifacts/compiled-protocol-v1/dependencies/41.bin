---
id: FR-049
title: "Bind native choreography syntax to exact protocol subjects"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
  - target: ix://agent-ix/quire-specification/FR-034
    type: references
  - target: ix://agent-ix/quire-specification/FR-050
    type: references
  - target: ix://agent-ix/quire-specification/FR-052
    type: references
  - target: ix://agent-ix/quire-specification/FR-056
    type: references
  - target: ix://agent-ix/quire-specification/FR-090
    type: references
---
## Description

When linking a native protocol declaration, the frontend SHALL retain its typed
roles, channels, causal control, obligations and compensation bindings under the
exact selected source, model and semantic definitions.

## Inputs

The located composed syntax tree, exact imported declaration/profile closure,
checked shared expressions, and required protocol/observation binding roles.

## Outputs

A typed source-bound protocol representation with explicit runtime input
requirements, or a located syntax, scope, type or admission refusal.

## Behavior

The frontend SHALL parse every protocol guard, key, capture, check and recovery
expression through the shared value-expression grammar.

The linker SHALL resolve model, native declaration and protocol-node references
in their distinct typed namespaces.

The linker SHALL reject a node value that is not definitely available on its
authored causal path.

The linker SHALL preserve the separate registration and activation environments
of a compensation template.

The frontend SHALL retain the typed protocol structure when a requested backend
cannot execute or project it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-049-AC-1 | Role, channel and relationship positions bind their required declaration kinds; substituting a payload type, endpoint or scalar identity refuses at its source location. | Test (TC-170) |
| FR-049-AC-2 | Send, receive, delivery, operation attempt, effect and compensation references remain distinct; a duplicate receipt cannot mint another effect node. | Test (TC-170) |
| FR-049-AC-3 | Sequence, owned choice, all-branch join and bounded repeat produce the declared causal structure; missing/duplicate join labels and unbounded or zero-progress repetitions refuse. | Test (TC-171) |
| FR-049-AC-4 | A required predecessor value is available to its successor; sibling-before-join, optional-branch escape, iteration-result escape and await-result-on-timeout references refuse. | Test (TC-171) |
| FR-049-AC-5 | A successful forward effect creates one template registration with registration captures; the first admitted eligible trigger creates one activation with separate immutable captures. | Test (TC-172) |
| FR-049-AC-6 | Retry bounds count the initial attempt, and exact commit/recovery references survive linking; compensation before the effect, after commit, or based only on transport success cannot satisfy recovery. | Test (TC-172) |
| FR-049-AC-7 | O1/O2 and split-shipment references require the exact declared relationship and endpoint direction even when they share a provider, trace ID or equal scalar labels. | Test (TC-173) |
| FR-049-AC-8 | A recovery aggregate retains the finite view, snapshot/window membership and completeness dependencies; missing members or duplicate business effects cannot be erased to make the aggregate pass. | Test (TC-173) |
| FR-049-AC-9 | A bound response at the inclusive deadline takes the response path; timeout requires complete authorized progress and never substitutes for a false constraint or missing input. | Test (TC-174) |
| FR-049-AC-10 | State-contract and temporal-requirement references retain their own profile and invocation/subject bindings; an unsupported projection leaves the admitted global protocol available for other requested capabilities. | Test (TC-170) |
| FR-049-AC-11 | Every lowered source node, including synthesized control edges and bounded occurrences, retains its declaration span and exact derivation/iteration identity. | Test (TC-171) |
| FR-049-AC-12 | A finish record without required closure, or with unsettled required obligations, cannot establish complete global success; it does not cause any business operation or compensation to execute. | Test (TC-174) |

## Dependencies

- [One composed grammar](../../proposals/quire-v1/shared-grammar.md).
- [Choreography surface and binding contract](../../proposals/quire-v1/choreography-surface.md).
- [B's protocol contract](../../proposals/quire-v1/protocol-contract.md).
- [Explicit ecosystem subjects](FR-035-bind-ecosystem-subjects.md).
- [Temporal clock selection](FR-090-select-temporal-profile-and-clock.md).
- [Typed observation binding](FR-110-bind-typed-observations.md).
- [Planned common matrix](../composed-foundation/tests.md).

This source-integration requirement does not discharge the producer contracts,
family consumer requirements or complete baseline review. D's exact model and
configuration contribution remains an explicit integration dependency.
