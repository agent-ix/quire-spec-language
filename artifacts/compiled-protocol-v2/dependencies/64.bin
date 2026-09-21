---
id: NFR-010
title: "Bound composed processing without fabricating results"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-specification/FR-036
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-041
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-043
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-044
    type: constrains
---
## Statement

When a composed processing request cannot perform its next required work within a declared resource limit, the
implementation SHALL stop the affected work with an explicit incomplete outcome
before publishing an unestablished result or silently dropping required input.

## Scope

Source/token/node admission, model and profile dependency expansion, numeric
proof/normalization, query traversal/materialization and finite-graph traversal.
F/E/B extend this policy to their owned monitor/protocol accounting contracts.
Semantic domain limits and caller-lowered work controls are different inputs;
zero does not disable a limit. Every selected processing boundary must declare
its accounting version and effective limits before accepting work.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Completed affected results after an insufficient limit | 0 | 0 | Test |
| Required inputs or live work silently dropped to report success | 0 | 0 | Test |
| Allocations/expansions beyond the declared admission/work charge | 0 | 0 | Test |

## Verification

[TC-048](../test-cases/TC-048-bound-composed-processing.md) plans zero, sufficient,
exact-boundary and one-below-sufficient controls for every implemented boundary.
Its budget, adverse-input and instrumented allocation controls respectively
measure the three metrics above; the Method cells use the catalog's Test class.
The domain includes nested sequences whose individual maxima are valid but
aggregate work is large, acyclic predicate fan-out, reducible rational inputs,
and long/cyclic graphs. A prior phase may fail first and must retain that actual
cause. No unimplemented boundary is claimed covered by another phase's tests.
Do not prescribe a universal memory ceiling from the historical compiler's
current defaults or confuse the local Cargo job limit with language semantics.

## Dependencies

- [State/query/graph contract](../../proposals/quire-v1/state-contract.md).
- [Typed producer causes](../functional/FR-047-emit-typed-located-causes.md).
- [Historical exhaustion distinction](NFR-002-classify-resource-incompleteness.md).
