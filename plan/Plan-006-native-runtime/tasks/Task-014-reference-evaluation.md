---
id: Task-014
title: "Execute the native reference semantics and exact cost model"
type: Task
status: blocked
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-006
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-003
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-067
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-068
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-069
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-070
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-071
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-072
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-073
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-074
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-075
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-076
    type: verifies
---
# Task-014: Execute the native reference semantics and exact cost model

## Scope

Implement independent native AST execution over ValidatedContext, retaining captured observations, concrete truth, exact work counters and actual implication events.

## Subtasks

- [ ] Author independent parent/aggregate, signed-arithmetic, sequence, observation/capture and event/cost expectations through the actual checked native API.
- [ ] Implement immutable runtime value views and evaluate-once locals/conditionals, admitted Boolean/scalar/record/reference operations and ordered sequence quantifiers.
- [ ] Implement positive-length reaches over one validated observation with a local visited set; compare every small functional graph to the separate closure oracle.
- [ ] Preserve pre/post captures and original expression lineage; preflight expression/depth/event entry together, retaining actual event prefixes on later stops.
- [ ] Enforce native-ref-cost/1-draft exact vectors plus auxiliary value/text/depth/event limits, deterministic cancellation and fresh retry state.
- [ ] Run supported and actual frontend-unsupported controls; do not introduce a collect/helper implementation or use the IR proof graph as the reference evaluator.

## Deliverables

Native Completed/Incomplete/Refused reports tied to exact context; qualified healthy/violating truth, independent graph/cost evidence and source-bound implication events.

## Notes

No B-owned TechniqueResult or generated backend interpreter is added. Memory/work limits do not become authored number, sequence or path bounds. Defensive runtime-invariant tests stay private and cannot create a public unchecked-context bypass.
