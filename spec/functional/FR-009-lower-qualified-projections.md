---
id: FR-009
title: "Lower through the existing executable binder"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-004"
    type: traces_to
---
# FR-009: Lower through the existing executable binder

## Description

When a backend projection is requested, the compiler shall lower only features qualified for the selected existing binder and backend.

## Inputs

A `NativePackage` and caller-lowered projection limits. The initial named target
is `boolean-oracle/v1`, matching the existing code generator's Boolean domain.

## Outputs

An immutable derived projection retaining its originating native package, exact
IR wire bytes, the existing `BoundPackage`, and per-clause read correspondence;
or a located unsupported, resource-exhausted or upstream-binding failure.

## Behavior

When lowering `boolean-oracle/v1`, the compiler shall translate Boolean literals,
parentheses, direct linked Boolean state/input reads, `not`, `and`, `or` and
`implies` through the existing IR constructors and strict executable binder.
The compiler shall preserve clause identities, kinds, anchors, expression source
coordinates, operand order and implication-consequent activation. Parentheses
retain the inner expression's source; they do not create executable operations.

The compiler shall derive each clause environment from its actually referenced
Boolean declarations in the linked context model, retaining declaration kind
and source. Each projected read shall identify its original model owner, model
digest, declaration, native observation and IR observation. Captured invocation
inputs use IR `current` while retaining their native pre-observation; state reads
retain the checked observation. Proof abstraction inputs are not executable reads.

If any clause contains another expression form, a non-Boolean read or an
unrepresentable owner population, then the compiler shall refuse the whole
projection with the authored clause identity and source location. One IR package
requires one authored package and one revision per requirement; empty packages
are refused. The unchanged native package retains every obligation and its
existing unlowered dispositions. No native identity-bearing reference is encoded
as a recursive IR record.

The compiler shall retain native artifact identity separately from the IR bound
identity. The latter covers the derived executable declarations and expressions;
unreferenced native declarations and object roles remain covered by native
identity, not an invented IR declaration. A projection is neither a runtime-input
validation result nor a retained proof attestation.

The compiler shall bound lowering to 10,000 visited native nodes, depth 64 and
16 MiB serialized output, with caller limits able only to lower these ceilings.
If a limit is exhausted, then the compiler shall return no projection; retrying
uses fresh counters. The compiler shall serialize through Serde with writes
stopped before exceeding the byte budget. The existing
binder and backend retain their own additional refusal limits.
The native checker also caps source nodes at 10,000 and depth at 64; lowering
retains these ceilings as an additional guard over already checked packages.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-009-AC-1 | Boolean state and captured input projections preserve their explicit read correspondence and are accepted by the existing strict binder. | Test |
| FR-009-AC-2 | An unqualified reference feature is refused as unsupported. | Test |
| FR-009-AC-3 | A missing executable clause binding refuses the projection. | Test |
| FR-009-AC-4 | Changing a referenced Boolean declaration from state to invocation input changes the bound identity; unrelated native model changes remain visible in native identity. | Test |
| FR-009-AC-5 | Actual generated Rust agrees with native evaluation for the complete Boolean input domain of the named parity fixture, including true and false outcomes and source-mapped implication activation. | Test |
| FR-009-AC-6 | Zero, exact and one-below limits distinguish success from exhaustion, retain all input obligations and permit a fresh successful retry. | Test |
| FR-009-AC-7 | A later unsupported clause, mixed package owners or multiple revisions of one requirement refuses the complete projection without partial output. | Test |

## Dependencies

- [US-004](../usecase/US-004-reuse-existing-toolchain.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.

## Status

Proof-of-concept engineering delivery is permitted with the generated activation
part of FR-009-AC-5 still open, per the owner's 2026-09-09 direction. The full
criterion and its test remain required for backend qualification; passing truth
results do not claim activation parity or production readiness.
