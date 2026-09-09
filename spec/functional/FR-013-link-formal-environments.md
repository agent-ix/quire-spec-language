---
id: FR-013
title: "Resolve native units against exact formal environments"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: depends_on
---
# FR-013: Resolve native units against exact formal environments

## Description

When a native unit is linked under the formal-environment binding profile, the linker shall produce an immutable correspondence to its exact selected Contract IR declarations.

## Inputs

An owned ParsedUnit, a borrowed slice of validated Contract IR
DeclarationEnvironment values and caller-lowered LinkLimits. The public
`link(unit, environments, limits)` API is specified in
[the binding contract](../../docs/formal-environment-linking.md).

## Outputs

An immutable LinkedPackage borrowing the selected environments and owning the
original ParsedUnit, or the existing Box<Diagnostic> with a link phase and no
package. Declaration identities retain RequirementRef ownership, declaration
kind/path and original IR SourceSpan. Native occurrence spans retain exact source
bytes and opaque identity/revision labels. No numeric revision cast occurs.

## Behavior

This profile uses existing IR canonical declaration bytes as its explicit formal
model artifact. Its package is the owner's exact PackageId namespace; its version is the exact
decimal spelling of the positive IR requirement revision. Its import byte digest
is SHA-256 of the existing V1 canonical declaration bytes, distinct from IR's
domain-separated CanonicalDigest. This profile does not reinterpret historical
Filament artifact digests or add a model decoder. All declarations/dependencies
inside an environment participate in that canonical artifact.

Every import is resolved before exposing a package. Missing package, stale
revision/digest, ambiguous exports and missing declarations have distinct codes.
Repeated aliases contribute candidate exports; no first/last precedence is
inferred. A context needs one record and an explicit state value named self with
that record type. Field, value, enum and local references retain their respective
owners/scopes. Record/option/collection shape propagation enables field lookup;
it is not a Boolean/definedness judgment and cannot authorize evaluation.

The initial formal profile resolves current invariants, lexical let/quantifier
bindings, grouping, conditionals, fields, scalar operators and the present/value/
size/pre names. Native reference dereference/reachability and operation clauses
require their concrete semantic mapping and return unsupported_construct in this
profile. Snapshot availability, guarded unwraps, scalar inference, unit/type
equality and Boolean roots remain FR-006's subsequent checking phase.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-013-AC-1 | LinkedPackage preserves exact source identity, bytes and all native/formal declaration loci after successful exact import selection. | Test (TC-030) |
| FR-013-AC-2 | A canonical semantic digest or another environment's byte digest cannot substitute for the selected canonical-declaration byte digest. | Test (TC-030) |
| FR-013-AC-3 | Lexical scopes, record fields, values and enum variants resolve to their exact local or owner-qualified identities; a missing reference refuses without a package. | Test (TC-031) |
| FR-013-AC-4 | Unsupported reference/operation mappings refuse with unsupported_construct and never yield a package or Boolean result. | Test (TC-032) |
| FR-013-AC-5 | Exact caller limits are admitted, a required next operation above a lowered or hard limit returns resource_exhausted, and zero does not disable a limit. | Test (TC-033) |
| FR-013-AC-6 | Ambiguity diagnostics retain every conflicting formal declaration locus, and prior calls or candidate order cannot expose partial linkage. | Test (TC-034) |

## Dependencies

- [FR-005](FR-005-link-shared-model.md) owns the existing exact/missing/ambiguous/stale/atomic criteria TC-020–024.
- [FR-006](FR-006-check-defined-expressions.md) owns actual typing and definedness after linkage.
- [FR-010](FR-010-report-native-outcomes.md) owns the shared native diagnostic envelope.
- [IT-005](../integration/IT-005-qualify-native-model-consumption.md) requires real public IR consumption.

## Status

Concrete LC02 linking contract. Successful resolution alone is not the complete
healthy/violating/refused state workflow, reference evaluation or backend proof.
