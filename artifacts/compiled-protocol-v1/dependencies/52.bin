---
id: FR-059
title: "Assess finite global protocol conformance"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-054
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-058
    type: depends_on
---
## Description

When requested for an admitted finite protocol and bound observation history, the conformance evaluator SHALL determine global conformance only against the protocol's exact causal, channel, activation, compensation and closure premises.

## Inputs

A linked finite protocol, exact request, qualified observations, decision-scope
and surrounding-execution progress/closure facts, the complete-global-conformance
closure record, resource limits and selected evaluator capability.

## Outputs

Per-obligation and global conformance results with an independently retained
complete-global-conformance closure state, exact violation or incompleteness
causes and provenance.

## Behavior

The evaluator SHALL accept any observation order consistent with the authored
partial order and channel premises. It SHALL report unexpected, missing,
misordered, duplicated and wrongly correlated events without repairing them.
Open or incomplete histories SHALL NOT establish complete global success. Batch
replay and settled incremental assessment over the same admitted facts SHALL
produce the same semantic results.

The complete-global-conformance closure record SHALL retain one state from
`closed`, `open`, `incomplete` or `contradicted` plus the exact required
execution, branch and workflow closure-premise identities. It is independent of
decision-scope closure, surrounding-execution closure, assessment execution and
completeness. Complete global success requires `closed` and every exact required
premise; a closed decision scope, settled per-obligation truth, completed
assessment or closed surrounding execution cannot substitute for a missing or
non-closed global premise. A strict reader SHALL refuse an omitted, duplicated,
cross-wired or scope-mismatched required premise.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-059-AC-1 | Valid split-shipment and payment/refund observations conform under each permitted branch interleaving. | Test (TC-059) |
| FR-059-AC-2 | Unexpected, missing, causally misordered, duplicate-effect and cross-workflow observations identify their exact violated obligation. | Test (TC-059) |
| FR-059-AC-3 | An open or incomplete history remains pending/incomplete even when no violation has yet been observed. | Test (TC-059) |
| FR-059-AC-4 | Settled incremental and batch replay results agree for the same protocol, inputs, progress and closure identities. | Test (TC-059) |
| FR-059-AC-5 | Complete global success is available only with a `closed` complete-global-conformance closure record carrying every exact required execution, branch and workflow premise; independently omitting, duplicating, cross-wiring or changing each premise prevents complete success or refuses the record without changing an independently settled per-obligation result. | Test (TC-059) |

## Dependencies

- [FR-052](./FR-052-represent-bounded-control.md) through [FR-058](./FR-058-preserve-retry-and-partial-recovery.md).
- E/F temporal and observation contracts.
