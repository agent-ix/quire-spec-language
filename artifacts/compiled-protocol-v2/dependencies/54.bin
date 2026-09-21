---
id: FR-060
title: "Separate conformance from projection and realizability claims"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-031
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-059
    type: depends_on
---
## Description

When multiple protocol-analysis capabilities are requested, the dispatcher SHALL produce one independent disposition for global conformance, monitorability, local projection, refinement, realizability and composition without transferring premises or success among them.

## Inputs

An admitted protocol, exact requested claims, selected backend capabilities and
per-claim communication, visibility, environment and correspondence premises.

## Outputs

One result per requested claim, each with execution disposition, premises,
derived-artifact identities and limitations.

## Behavior

Global conformance MAY execute when projection is unavailable. A satisfiable
finite trace SHALL NOT establish projectability or a causal strategy. Local
contracts with mutually circular assumptions SHALL NOT establish realizability
or composition. An unsupported advanced claim SHALL NOT erase a valid independent
global result. If a selected analysis backend fails, then the dispatcher SHALL
return a failed disposition for that requested claim without invoking an
unreviewed fallback or changing independent results.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-060-AC-1 | A monitorable but unprojectable protocol returns global replay/conformance results and an explicit projection refusal. | Test (TC-060) |
| FR-060-AC-2 | A satisfiable protocol with no causal implementation strategy does not report realizability. | Test (TC-060) |
| FR-060-AC-3 | Circular local assumptions cannot establish composition success without an independently discharged premise. | Test (TC-060) |
| FR-060-AC-4 | Every advanced result identifies its admitted fragment, backend, assumptions and source/derived-artifact correspondence. | Test (TC-060) |
| FR-060-AC-5 | A backend failure yields a failed disposition for its exact claim and does not invoke a fallback or erase an independent result. | Test (TC-060) |

## Dependencies

- [Shared capability dispatcher](./FR-031-report-requested-capabilities.md).
- [FR-059](./FR-059-assess-finite-global-conformance.md).
- E03 library qualification for any adopted specialized implementation.
