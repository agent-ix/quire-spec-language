---
id: SR-416
title: "Scope-boundary review of ConfigVersion numeric backends"
type: SpecReview
analysis: scope-boundary
scope: "IT-010 ownership across SL, IR, codegen, runtime and Kani"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/IT-010
    type: reviews
---

## Summary

Responsibilities are allocated to the component that owns each semantic decision. IT-010 verifies
the external generator and solver contracts but does not absorb them into SL or widen scalar support
to object/graph semantics.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Closed: FR-033 still owns projection only; generated backend acceptance is explicitly an IT-010 consumer observation, not a lowering guarantee. | FR-033, IT-010 |
| FND-002 | medium | Closed: SL owns model-domain admission before oracle invocation, while codegen owns exact expression lowering/strategy/Kani generation and cargo-kani owns proof execution. | IT-010-SC-02 through IT-010-SC-05 |
| FND-003 | medium | Closed: ParentOrder/NoCycle remain outside the representable scalar IR and must refuse at the earliest owning boundary; this test cannot invent codegen-level graph semantics. | IT-010-SC-06 |

## Responsibility Allocation

| Boundary | Owner | Class | Assumed or verified |
| --- | --- | --- | --- |
| Native model/source/package, runtime input and `runtime::execute` | quire-spec-language | core | Verified directly |
| Executable expression/type/source-span contract | quire-contract-ir | infrastructure | Verified by strict read and one locked revision |
| Oracle, constructive strategy, Kani source and proof graph | quire-contract-codegen | core | Guaranteed through IT-010 contract execution |
| Rust compilation, proptest runner and cargo-kani solver execution | External local tools | infrastructure | Guaranteed by actual invocation and exact version/digest checks |
| Object dereference/reachability backend semantics | Future IR/codegen design | excluded | Neither assumed nor claimed |
