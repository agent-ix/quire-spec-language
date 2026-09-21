---
id: NFR-020
title: "Bound protocol linking and assessment"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: ix://agent-ix/quire-specification/FR-052
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-059
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-061
    type: constrains
---
## Statement

The protocol implementation SHALL bound linking and assessment by authored protocol limits and caller-supplied execution limits, returning a resource-incomplete result before exceeding any limit.

## Scope

Protocol definitions, causal nodes and edges, roles, messages, branches, iteration,
active workflow instances, observations, retained result items and derived
analysis artifacts in production or possible qualification Rust paths.

## Rationale

Protocol structure and supplied observations are untrusted. A finite source
profile does not itself bound concurrent instances, observation volume or an
optional analysis backend's resource use.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Unbounded caller-controlled collections or traversals | 0 | 0 | Inspection (TC-065) |
| Limit-crossing cases returned as typed resource-incomplete results | 100% of declared limits | 100% | Test (TC-062) |
| Business actions or compensations executed by assessment | 0 | 0 | Test |

## Verification

Retain a complete source-bound inventory of every caller-controlled collection,
queue and traversal, its owning request or result field, limit authority, charge
point, termination argument, resource-incomplete code and corresponding TC-062
boundary case. Reconcile that inventory independently against every reviewed
Rust interface and production path; TC-065 fails when an entry or source site is
unmatched. Execute exact-boundary and one-over-boundary cases independently for
every inventoried structural, instance, observation and result limit. Mutate
each limit check so equality is rejected or excess is accepted; TC-062 must
discriminate both changes.

## Dependencies

- [FR-052](../functional/FR-052-represent-bounded-control.md).
- [FR-059](../functional/FR-059-assess-finite-global-conformance.md).
- [FR-061](../functional/FR-061-report-orthogonal-results.md).
