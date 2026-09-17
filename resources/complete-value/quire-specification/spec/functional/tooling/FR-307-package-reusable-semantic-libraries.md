---
id: FR-307
title: "Package reusable semantic and domain libraries"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-002
    type: references
  - target: ix://agent-ix/quire-specification/FR-132
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

# FR-307: Package reusable semantic and domain libraries

## Description

When importing a reusable Quire library, the linker SHALL resolve its exported
declarations, semantic definitions, model dependencies and compatibility/migration
contracts through the package lock.

## Inputs

Library package, export table, dependencies, version constraints and migrations.

## Outputs

Resolved qualified exports and dependency closure, or conflict/cycle/version/
migration refusal.

## Behavior

Libraries may export types, functions, predicates, protocols, plans and view
definitions without acquiring a second source authority. Imports are qualified;
diamond dependencies must select byte-identical definitions or an explicit
compatible unification. Migration emits a new subject and correspondence.

## Library resolution contract

A library manifest binds package identity/version, edition/profile, exported
qualified declarations, dependency constraints and canonical content digest.
Resolution selects one exact version per package identity and produces a closed
acyclic lock graph. Diamond paths unify only when they reach the same canonical
definition digest or an explicit compatibility declaration with a checked
migration. Imports never inject unqualified names implicitly, and migration
creates a new package identity plus element correspondence.

## Import binding and export identity

The complete-unit `import "L" version "v" digest "d" as a;` names library
identity `L`, version string `v` and `d`, which is the 64-lowercase-hex
`quire.package.semantic/v2` `package_id` of the library's CheckedPackage V2. It
is not a raw source or manifest byte digest. A supplied library whose identity,
version or `package_id` differs from the import is
`refused { code: stale_dependency, cause: revision-mismatch }` for a version
difference, or `cause: byte-digest-mismatch` for a `package_id` difference.
An export identity is (library package key, export node key in that library's
graph). Imported declarations are named only as `a::Name`. The grammar makes
`as` optional: an import without `as` still selects and verifies the library,
but binds no qualifier and brings no name into scope. No import brings
unqualified names into scope, so a use of an imported name without its
qualifier, including every use of a name from an import without `as`, is
`refused { code: missing_declaration, cause: missing-name }`. When two imports
bind the same qualifier, each use is
`refused { code: ambiguous_declaration, cause: ambiguous-name }` and lists both
import paths.

Resolution takes the transitive import closure in declaration order. When two
dependency paths reach library identity `L`, they unify only when they name the
same version and `package_id`. Any other combination is
`refused { code: invalid_package, cause: conflicting-definition }`, and the
refusal lists both dependency paths. Complete V1 has no source form for a
compatibility declaration, so distinct `package_id`s never unify. An import
cycle is `refused { code: invalid_package, cause: definition-cycle }`, whose
dependency-edge payload lists the cycle path. A resolved closure is recorded in the importing package lock as one
selection per library identity, in ascending identity order.

A library migration is a new library package with a new `package_id`. A package
that imports the migrated library is a new package with a new `package_id`.
Because `package_id` is the digest of the package's identity preimage, a reader
recomputes it before resolution. A migrated package that reuses its source's
`package_id` while its preimage differs is
`refused { code: invalid_package, cause: invalid-value }` at member path
`/package_id`, with no import, lock selection or evidence binding it. A
"migration" whose preimage is unchanged yields the identical package and
identity, and is not a migration.
Evidence keyed by the old package or export identity keeps that key and is never
relabeled. No library-level migration correspondence record is defined beyond
FR-322's CheckedPackage V1-to-V2 migration record.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-307-AC-1 | A compatible dependency diamond resolves one exact export identity and reproducible lock. | Test (TC-227) |
| FR-307-AC-2 | Conflicting definitions, import cycles or ambiguous unqualified names refuse with dependency paths. | Test (TC-227) |
| FR-307-AC-3 | Migration preserves old/new identities and does not relabel historical evidence. | Test (TC-227) |
| FR-307-AC-4 | An import binds the library `package_id`, a mismatched import is `stale_dependency`, a conflicting diamond or import cycle is `invalid_package` with its paths, and a duplicate qualifier is `ambiguous_declaration`; an import cycle uses cause `definition-cycle`, and a name from an import without `as` is `missing_declaration`. | Test (TC-227) |
| FR-307-AC-5 | A migrated package that reuses its source `package_id` with a different identity preimage is `invalid_package` with cause `invalid-value` at `/package_id`. | Test (TC-227) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
