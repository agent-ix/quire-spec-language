---
id: FR-041
title: "Evaluate bounded queries with occurrence-preserving semantics"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When evaluating an admitted bounded sequence query, the evaluator SHALL apply the selected query's ordered occurrence and exact-result semantics to the validated supplied domain.

## Inputs

A complete supplied Seq(T,N), N at most 10,000, a checked binder body, declared cardinality/total type where required, exact scope identity and execution budgets.

## Outputs

The specified Boolean, cardinality, sequence or exact sum value, or a typed refusal/incomplete outcome without a fabricated completed result.

## Behavior

The state contract defines size, contains, forall, exists, filter, map, count and sum. Order and duplicate occurrences remain observable. Missing data is not empty. Filter/map/count/sum process each occurrence; Boolean membership/quantifiers short-circuit only as specified. Sum explicitly selects a same-representation/same-unit result domain admitting zero, projected values and every accumulated prefix. No implicit unit/numeric conversion, deduplication, flattening, reassociation or ambient population lookup occurs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-041-AC-1 | For explicitly authored Amount 1..20/U, N=5, Total 0..100/U and Count 0..5, amounts [2,2,3] retain duplicate map/filter outputs, count=2 for amount=2, sum=7 and size=3. | Test (TC-041) |
| FR-041-AC-2 | Empty domains yield the declared query identities: forall=true, exists/contains=false, filter/map empty, count/size/sum zero where the selected result type admits them. | Test (TC-041) |
| FR-041-AC-3 | Wrong cardinality/total domain, unit or numeric representation refuses; an N=6 domain is not admitted to the example Total merely because observed values are small. | Test (TC-041) |
| FR-041-AC-4 | Contains/exists/forall stop at their specified decisive occurrence; filter/map/count/sum retain every reached occurrence and its source binding. | Test (TC-041) |
| FR-041-AC-5 | An incomplete domain or unknown required membership cannot be treated as empty or produce aggregate success; resource exhaustion returns no completed query value. | Test (TC-041) |
| FR-041-AC-6 | An out-of-range intermediate sum prefix is not rescued by an in-range final mathematical sum or by reordering operands. | Test (TC-041) |

## Dependencies

- [Detailed draft contract](../../proposals/quire-v1/state-contract.md).
- [Shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed composed-v1 obligation. Historical definitions and their acceptance
remain separate; this artifact does not establish implementation coverage.
