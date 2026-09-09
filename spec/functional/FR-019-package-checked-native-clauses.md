---
id: FR-019
title: "Package the complete checked native clause inventory"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: depends_on
---
# FR-019: Package the complete checked native clause inventory

## Description

When native package construction is requested, the compiler shall emit a versioned artifact retaining the complete checked clause inventory and its exact static dependencies.

## Inputs

A constructor-produced CheckedPackage and caller-lowered PackageLimits. The
closed artifact and Rust API are specified in
[the native package contract](../../docs/native-linked-packages.md).
No runtime population, observed result or caller-supplied support claim is an
input to package construction.

## Outputs

An immutable NativePackage owning the CheckedPackage, complete artifact bytes,
their ByteDigest and a role-specific NativePackageRef, or a structured PackageError
with actual package-stage usage and no partial package.

## Behavior

The artifact identifies original native/formal source, selected native models,
the exact adopted semantic definitions, every authored clause and resolved
occurrence, and every runtime input obligation. Its model artifacts preserve
typed declarations, nominal scalar roles, units, bounds, object universes and
operation/frame declarations, including unused declarations in selected models.
The artifact contains a source/dependency manifest; original source bytes and
admitted model objects remain explicit reader inputs.

Every clause has one native-reference disposition and one executable-IR
disposition. The currently qualified native reference path is available only
after runtime validation. Existing IR lowering is unlowered until its separate
producer qualification is implemented. No clause disappears because that
projection is unavailable, and no definedness proof is relabeled executable.

The artifact's exact byte identity is separate from authored identities and
the native static identity derived by [FR-021](FR-021-derive-native-package-identity.md).
Existing IR/JCS canonical domains remain unchanged. Independent FS05 consumer
adoption remains an acceptance gate. No portable reference,
method, plan, result or evidence schema is introduced.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-019-AC-1 | A package built from real checked invariant/pre/post clauses retains every authored owner/clause, source-order position, kind and execution point. | Test |
| FR-019-AC-2 | The package retains exact native/formal source identity and byte digest while excluding the local display path from artifact identity. | Test |
| FR-019-AC-3 | Every selected import retains its alias, owner, complete native-model artifact and digest; changed unused declarations or roles in a selected model change package bytes. | Test |
| FR-019-AC-4 | Every formal/local resolved occurrence and runtime context/universe/observation/operation/frame obligation matches the retained CheckedPackage. | Test |
| FR-019-AC-5 | The package identifies both adopted base-profile and semantic-rules definitions by exact revision/digest without claiming that the old profile-file digest covers the amendment. | Test |
| FR-019-AC-6 | Required features are the exact duplicate-free sorted set derived from all selected model declarations and source syntax, including unused declarations and unreachable syntax. | Test |
| FR-019-AC-7 | Every clause retains available native-reference and unlowered executable-IR dispositions with original clause identity and location; no IR proof view is exported as executable code. | Test |
| FR-019-AC-8 | Repeated construction with identical checked inputs emits identical bytes and digests; changing runtime populations or evaluator budgets cannot change the static artifact. | Test |
| FR-019-AC-9 | Package byte digests, native static identity, authored identifiers and IR canonical/bound digests retain distinct typed roles and selected domains. | Test |
| FR-019-AC-10 | Construction honors each selected content/entry limit before excess work and leaves caller-retained source/model bytes unchanged on success or failure. | Test |

## Dependencies

- [FR-015](FR-015-project-native-model-semantics.md) owns admitted native models.
- [FR-016](FR-016-check-native-clauses.md) owns typed clauses and conditional proofs.
- [FR-007](FR-007-validate-runtime-inputs.md) remains required before execution.
- [FR-020](FR-020-read-and-rebind-native-packages.md) owns strict reconstruction.
- [FR-021](FR-021-derive-native-package-identity.md) owns native static identity.
- [NFR-007](../non-functional/NFR-007-bound-native-packages.md) owns package limits.
- [FR-009](FR-009-lower-qualified-projections.md) retains actual executable lowering.

## Status

Reviewed draft for LC02 issue #3: all-eight review at 69588ad, with the
source-setup correction/re-review at 2c6b9b8/1c3aa50. Task-016 has an initial
producer implementation; complete qualification, reconstruction and independent
canonical/interchange/B/C acceptance remain pending under Plan-007.
