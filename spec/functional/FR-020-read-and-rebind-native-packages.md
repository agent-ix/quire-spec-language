---
id: FR-020
title: "Read native packages through exact reconstruction"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: depends_on
---
# FR-020: Read native packages through exact reconstruction

## Description

When a native package is read, the compiler shall reconstruct its checked meaning from the explicitly supplied source, authored bindings and admitted models before accepting its serialized claims.

## Inputs

Bounded raw package bytes, an exact expected NativePackageRef, externally
selected CheckBindings containing the original FormalSource, an offered native
model inventory, PackageSupport and separate package/parser/linker/checker limits.
The [package contract](../../docs/native-linked-packages.md) fixes the intake
order and closed representation.

## Outputs

An immutable NativePackage retaining both the accepted raw artifact and its
freshly reconstructed CheckedPackage, or PackageError with no accepted package.
An upstream native refusal retains its original Diagnostic and phase; a package
wire error does not fabricate a native clause span.

## Behavior

Serde owns JSON recognition. Closed typed wire records reject unknown, duplicate
decoded and omitted required fields without a generic host map erasing them.
The reader rejects unsupported versions/profiles/features before interpreting
unrecognized meaning. It performs no network lookup, decoder fallback, revision
coercion, source rewriting, model construction from generated layouts or implicit
authored-ID adoption.

After exact dependency and external authored-binding checks, the reader invokes
the actual parser, link_native and check APIs with fresh caller-selected limits.
It regenerates the package manifest and compares every serialized semantic or
provenance claim. Required-feature order alone is set-like; original clause,
import and occurrence orders remain significant. An attacker recomputing a raw
digest cannot bypass derivation checks.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-020-AC-1 | A produced package reads through actual parse/link/check with the exact externally selected source/models/authored bindings and yields the expected healthy/violating observations through actual runtime validation/evaluation. | Test |
| FR-020-AC-2 | Invalid UTF-8, BOM, trailing JSON, malformed numbers/escapes, unpaired surrogates, duplicate decoded members and unknown/missing closed fields refuse without an accepted package. | Test |
| FR-020-AC-3 | Unknown format in recognized valid duplicate-free JSON receives unknown_wire before version-specific field checks; unsupported language/edition/profile or changed selected semantic-definition digest refuses in the contract's selection order without fallback. | Test |
| FR-020-AC-4 | Reordered unique required features preserve acceptance; duplicates receive invalid_package, and unknown features or features outside the declared consumer feature set receive unknown_required_feature. | Test |
| FR-020-AC-5 | Changed package/source/model bytes or exact native/formal identities refuse at the corresponding binding stage, including conflicting unselected inventory identities. | Test |
| FR-020-AC-6 | Missing, duplicate, extra, foreign or changed authored clause bindings refuse even when the attacker updates the package byte digest. | Test |
| FR-020-AC-7 | Mutated resolutions, locations, runtime obligations, omitted features or projection dispositions refuse after successful setup rather than being trusted as checked facts. | Test |
| FR-020-AC-8 | A validly encoded package cannot bypass actual syntax/link/type/definedness refusal when its selected source requires that refusal. | Test |
| FR-020-AC-9 | Exhaustion in package intake or any selected frontend stage remains incomplete, retaining the actual stage/cause with no package; later retries have fresh accounting. | Test |
| FR-020-AC-10 | Equivalent object-member spelling/order yields the same reconstructed meaning while retaining the accepted raw bytes/digest; reordered clauses/imports or arrays with ordered semantics refuse correspondence. | Test |
| FR-020-AC-11 | Unsupported executable projection remains addressable after read/rebind and cannot be extracted as a BoundPackage or substituted for a native runtime result. | Test |

## Dependencies

- [FR-019](FR-019-package-checked-native-clauses.md) owns emitted meaning.
- [FR-004](FR-004-verify-source-maps.md) and [FR-014](FR-014-bind-native-formal-source.md) retain explicit source correspondence.
- [NFR-007](../non-functional/NFR-007-bound-native-packages.md) bounds intake.
- [IT-007](../integration/IT-007-native-package-reconstruction.md) qualifies reconstruction through actual runtime observations.

## Status

The source bindings this reader checks carry the four source labels of FR-001; a package or binding naming two labels refuses. Specified under QSL-233 (ADR-013 §7 S-4b), not implemented.

Draft. This is compiler payload intake, not B's shared-reference reader, an
independent consumer qualification or a general formal-model decoder.
