---
id: SR-446
title: "Dependency review of the complete-V1 QSL adoption plan"
type: SpecReview
analysis: dependency
scope: "FR-055, IT-011 and Plan-013 Task-046 through Task-054"
review_set: all
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-055, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/IT-011, type: references }
---
# Dependency review of the complete-V1 QSL adoption plan

## Summary

The plan is an acyclic nine-node chain. QSpec enables adoption; source, scalar,
composite, model, runtime and tooling work precede WASM parity; complete
qualification consumes all merged results last.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No cycle or hidden enablement edge remains after the A07/A08 label correction. Every task after Task-046 has exactly one serial predecessor and separately names its external producer or consumer gates. | FR-055-AC-4; Plan-013; Task-046..Task-054 |

## Classification and order

| Stage | Class | Prerequisite |
| --- | --- | --- |
| Task-046 adoption | Enablement | QSpec Task-010 / frozen revision `8d0fbad` |
| Task-047 source/package | Enablement | Task-046 |
| Task-048 scalar semantics | Enablement | Task-047 |
| Task-049 composite/functions | Feature | Task-048 |
| Task-050 model graph | Feature | Task-049 plus accepted I03 producer pin |
| Task-051 runtime | Feature | Task-050 plus merged temporal/protocol consumers |
| Task-052 tooling | Feature | Task-051 |
| Task-053 WASM parity | Feature | Task-052 merged in QSL |
| Task-054 qualification | Assurance | Task-053 plus all named integration pins |

Topological order is therefore Task-046 through Task-054. External contracts
are one-way consumed pins and introduce no return edge into QSL admission.
