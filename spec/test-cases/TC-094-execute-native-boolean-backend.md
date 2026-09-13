---
id: TC-094
title: "Compare generated Boolean truth and source activation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: verifies
  - target: ix://agent-ix/quire-spec-language/IT-008
    type: references
---
# TC-094: Compare generated Boolean truth and source activation

## Description

Execute every step of [IT-008](../integration/IT-008-native-boolean-backend-parity.md)
on `(a implies b) implies (c implies (not a or not b and c))` and a second constant
clause, for all eight Boolean assignments.

## Test Procedure

Assert the historical `ix:native` / `0-draft` / `state-finite/0-draft` source
selection. Compile the actual emitted artifacts and codegen's generated finite
proptest strategy once. Require its deterministic campaign to cover all eight
assignments, then execute each assignment with separate measured coverage.
Compare the exact LLVM 3.1.0 counts at the generated source-map probes with
native source events and independently specified control equations, including
the implication nested in an antecedent. Retain the pinned reusable coverage
consumer's explicit unsupported-profile result as a downstream boundary.

## Expected Results

All eight native, independent, generated executable and generated-proptest truth
results agree, as do all per-source activation counts. Complete clause identity
survives both IR pins. Missing tools, unavailable coverage and failed native
validation fail the test rather than count as a passing or skipped case.
