---
id: FR-021
title: "Derive the native static package identity"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: depends_on
---
# FR-021: Derive the native static package identity

## Description

When a native package identity is derived, the compiler shall hash the complete reconstructed static manifest under the native bound-package canonical profile.

## Inputs

The actual CheckedPackage, from which the identity derivation reads the static
manifest defined in the shared wire contract, including original source,
selected models, semantic definitions, feature set, authored clauses, resolution
and runtime obligations. The exact projection and byte algorithm are defined in
[the package identity contract](../../docs/native-linked-packages.md#native-static-identity).

## Outputs

A typed NativePackageIdentity with domain quire.native.bound-package, version v1,
algorithm sha256 and a bare lowercase 64-hex digest. This identity does not use
the IR CanonicalDigest type or the source/package ByteDigest role.

## Behavior

Canonical input excludes only the canonical_identity member and each clause's
projection-availability inventory. Display paths, runtime data/results and
producer/run claims are already absent from the static manifest. All other
manifest content participates. Exact source identity and bytes therefore matter;
this is a source-bound static identity, not a claim of general logical equivalence.

The reader recomputes identity after actual source/model rechecking and still
compares excluded projection metadata with the actual producer disposition.
Matching identity alone cannot authorize a forged capability. Registering this
new domain in B's shared SemanticRef reader remains explicit FS05 consumer work;
existing closed shared-reference schemas are not silently extended here.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-021-AC-1 | Independently authored canonical bytes and domain-prefix hash vectors match the producer, including Unicode escaping and a revision above the binary64 exact-integer range. | Test |
| FR-021-AC-2 | Reordered JSON members and unique feature order preserve reconstructed canonical identity while their differing raw artifact bytes retain distinct ByteDigests. | Test |
| FR-021-AC-3 | Each changed semantic definition, source binding, selected model declaration/role, authored clause, resolution or runtime obligation changes the independently computed static identity. | Test |
| FR-021-AC-4 | The canonical input omits projection availability, and a forged excluded projection claim still refuses package reconstruction despite an unchanged canonical identity. | Test |
| FR-021-AC-5 | An unknown canonical domain/version/algorithm refuses selection; a wrong digest or raw/IR/JCS digest substitution refuses correspondence without a package. | Test |
| FR-021-AC-6 | Canonical derivation obeys its separately reported package-pass limits before excess work; runtime populations, evaluator budgets and local display paths do not enter its bytes. | Test |

## Dependencies

- [FR-016](FR-016-check-native-clauses.md) supplies checked inputs; deriving identity does not require an already encoded NativePackage.
- [FR-019](FR-019-package-checked-native-clauses.md) consumes identity when emitting the complete artifact.
- [FR-020](FR-020-read-and-rebind-native-packages.md) owns strict reconstruction.
- [NFR-007](../non-functional/NFR-007-bound-native-packages.md) bounds each pass.
- Existing IR/JCS algorithms retain their separate domains and golden vectors.

## Status

Draft producer contract. Review, implementation, independent vectors and shared
consumer adoption are required before their corresponding qualification claims.
