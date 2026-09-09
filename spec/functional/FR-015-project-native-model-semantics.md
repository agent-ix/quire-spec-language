---
id: FR-015
title: "Bind explicit native semantics over formal declarations"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: depends_on
---
# FR-015: Bind explicit native semantics over formal declarations

## Description

When a caller supplies formal declarations and explicit native roles, the native model adapter shall produce an immutable source-bound model for the native finite-state profile.

## Inputs

One exact FormalSource, one existing validated Contract IR DeclarationEnvironment,
NativeRoles and caller-lowered ModelLimits. The concrete Rust contract is
[native-model-checking.md](../../docs/native-model-checking.md). The selected
profile is native-state-model/1. Its inputs are typed Rust values, not generated
Rust layouts or the historical abstract typing hypotheses treated as model wire.

## Outputs

A NativeModel owning those declarations, validated roles, source correspondence
and deterministic artifact bytes/digest. Failure returns the existing located
Box<Diagnostic>, with no model. link_native consumes a borrowed NativeModel slice
and returns a LinkedPackage that retains the exact selected models and source.

## Behavior

The native model adapter shall derive primitive bounds, option structure, record fields and collection maxima from the supplied IR declarations.

The native model adapter shall bind each integer or text declaration site to exactly one explicitly authored nominal scalar role.

If a role is missing, duplicated, inconsistent with its IR representation or outside the native profile, then the native model adapter shall refuse the model.

The native model adapter shall validate every supplied declaration and role locus against its exact bound model source.

When a native model artifact is selected by an import, the native linker shall retain the artifact identity and the native role correspondence for every resolved declaration.

If any import, context, operation or reference-field mapping fails, then the native linker shall refuse the entire unit.

Native roles add semantics absent from IR: nominal scalar identity, exact units,
authored text maximum, identity-bearing object/universe roles, reference identity
domains and operation parameter/result/frame bindings. Scalars reuse IR signed
integer bounds with reject-overflow; no new range engine or numeric widening is
introduced. Collections under this explicit profile are ordered sequences with
duplicates retained and the IR-authored maximum. References use an explicitly
selected closed IR enumeration as a finite identity domain; object fields remain
acyclic IR records. No recursive-record encoding or enum-to-reference coercion
is inferred. Runtime populations and reference existence are separate inputs.

Every metadata role has an explicit source locus. Operations are native effectful
declarations with explicit frame fields and created/deleted object types;
PureFunctionDeclaration signatures are not reinterpreted as operations. The
complete supplied environment is validated, including unused declarations.

The artifact includes existing canonical IR declaration bytes, sorted declaration
loci, the native roles and the exact model source binding. Changes to units,
nominal identities, frames, model source bytes/revision or any declaration affect
the selected artifact. The raw SHA-256 ByteDigest is distinct from IR semantic
CanonicalDigest and the original formal-environment/1 artifact. Existing link
keeps its original digest and refusal behavior; new semantics require link_native.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-015-AC-1 | A real source-derived Rust rule model preserves Version 0..1000, Signed -10..10, Count 0..3, Wide i64 bounds, Distance/metre, Duration/second, Node reference fields, duplicate-preserving items and the step operation. | Test (TC-040) |
| FR-015-AC-2 | Missing/duplicate scalar sites, mismatched representations, unsigned/saturating/rational declarations, missing text bounds and malformed object/operation/frame roles refuse without a model. | Test (TC-041) |
| FR-015-AC-3 | Semantic, provenance and unused-declaration mutations change the model artifact; reordering set-like role/declaration inventories preserves it; a legacy or foreign digest cannot select the model. | Test (TC-042) |
| FR-015-AC-4 | False or foreign model source loci refuse, including constructor-valid IR coordinates; conflicting native bindings for one formal source identity or model owner cannot enter one linked inventory. | Test (TC-043) |
| FR-015-AC-5 | link_native resolves actual dereference fields and operation roles while existing link retains its unsupported reference/operation behavior and original artifact identity. | Test (TC-044) |
| FR-015-AC-6 | Exact lowered model/link limits succeed where the input fits; one required operation beyond a limit, including zero and attempted hard-limit elevation, returns resource_exhausted without partial output. | Test (TC-045) |

## Dependencies

- [FR-013](FR-013-link-formal-environments.md) owns reusable exact import/name resolution.
- [FR-014](FR-014-bind-native-formal-source.md) owns exact source-to-IR coordinates.
- [FR-006](FR-006-check-defined-expressions.md) and [FR-016](FR-016-check-native-clauses.md) own subsequent typing and definedness.
- [NFR-005](../non-functional/NFR-005-rust-verification-paths.md) requires Rust production and qualification.
- [IT-005](../integration/IT-005-qualify-native-model-consumption.md) owns actual IR consumption and the rule-model qualification.

## Status

Specified for implementation. Neither NativeModel nor successful name resolution
establishes a reference-evaluable clause or an executable backend package.
