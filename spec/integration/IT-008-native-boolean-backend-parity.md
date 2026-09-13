---
id: IT-008
title: "Execute native Boolean projections through the existing backend"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: verifies
---
# IT-008: Execute native Boolean projections through the existing backend

## Objective

Expose frontend/backend truth or implication-activation drift on actual compiled
output. This bounded Boolean milestone does not close the full ConfigVersion
workflow in [IT-002](./IT-002-native-state-workflow.md).

## Target Integration

Native parse/link/check/package/lower → existing IR wire binder → pinned codegen
→ generated Rust, generated proptest strategy and runtime operators. Native
validation/evaluation supplies the reference result; actual LLVM coverage
supplies generated activation for the exact admitted fixture profile.

## Preconditions

Exact dependencies, Rust 1.98.1 with matching LLVM tools, and cargo-llvm-cov 0.9.0
are available locally. The code generator's
older IR pin is consumed through its own wire reader in qualification only;
production lowering retains the adopted IR pin. Missing prerequisites fail the
test rather than silently skip it. No hosted run is dispatched.

## Inputs

An admitted source model, authored clause bindings, and all assignments of a
small Boolean fixture containing nested implication, conjunction, disjunction
and negation. Use independently specified truth and activation expectations.

## Test Procedure

1. Produce and lower the actual checked native package.
   IT-008-SC-01: every authored clause and native source locus survives binding.
2. Consume exact emitted wire through the code generator's pinned IR reader and
   invoke its public bound generation API.
   IT-008-SC-02: complete generated clauses retain identities and source maps.
3. Validate and evaluate each native runtime assignment; compile and execute the
   generated Rust and generated proptest strategy for the same assignments.
   IT-008-SC-03: all native, independent and generated truth results agree.
4. Measure generated execution with actual coverage and resolve the exact LLVM
   3.1.0 fixture segments through the generated source-map probes. Exercise the
   pinned reusable reader separately and retain its profile refusal.
   IT-008-SC-04: generated consequent activation agrees with native source events,
   including skipped and entered nested consequents.

## Expected Results

Every step passes using actual producer bytes and executions. Unsupported
numeric/reference features retain explicit refusal. Generated attestation bodies
are not described as sealed or retained proof records. This fixture does not
claim that the downstream reusable coverage reader accepts LLVM 3.1.0.
