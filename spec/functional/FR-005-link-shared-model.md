---
id: FR-005
title: "Link native names to explicit formal declarations"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-002"
    type: traces_to
---
# FR-005: Link native names to explicit formal declarations

## Description

When linking is requested, the compiler shall resolve native name occurrences against the selected formal declaration environment.

## Inputs

ParsedUnit, exact import/source bindings, explicit formal declarations constructed
through Contract IR's public DeclarationEnvironment API, and the reviewed native
projection contract for those declarations.

## Outputs

A separately constructed LinkedPackage or located linkage diagnostics.

## Behavior

Contract IR FR-013 owns the formal type system; its FR-019 public Rust API is
available now. Accepted ADR-0054 at IR revision
690bde7f2dc58662cf9ff0595c2c0e3b17107c6f supersedes the earlier requirement for
a Filament reader or universal typed-model adapter. Filament defines archetype
schemas and generates datatypes; those facts do not supply formal semantics.

A owns native resolution and the explicit source-to-formal correspondence. It
checks declaration identity, exact revisions, source loci and selected closure.
It does not infer formal meaning from Rust layouts, populations, bare names or
successful datatype generation. Generic language linking/checking proceeds
against the existing public IR constructors and validation interface.

A clause requiring archetype-specific meaning needs only its concrete reviewed
projection, owned by that modeling language. In particular, a one-sided integer
bound and optional self-reference are not representable as a finite integer and
recursive inline record. Refuse that projection until its finite-bound and
reference/population semantics are specified and qualified. This local semantic
gap does not gate unrelated supported clauses. No second Contract IR binder or
general Filament model authority is introduced.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-005-AC-1 | An exact qualified import resolves to the selected formal environment's exported type identifier. | Test |
| FR-005-AC-2 | A missing import receives missing_import. | Test |
| FR-005-AC-3 | Ambiguous exports receive ambiguous_declaration. | Test |
| FR-005-AC-4 | A stale package closure receives stale_dependency. | Test |
| FR-005-AC-5 | A failed link yields no LinkedPackage. | Test |

## Dependencies

- [US-002](../usecase/US-002-link-exact-models.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
