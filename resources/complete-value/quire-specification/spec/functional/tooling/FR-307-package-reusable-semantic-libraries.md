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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-307-AC-1 | A compatible dependency diamond resolves one exact export identity and reproducible lock. | Test (TC-227) |
| FR-307-AC-2 | Conflicting definitions, import cycles or ambiguous unqualified names refuse with dependency paths. | Test (TC-227) |
| FR-307-AC-3 | Migration preserves old/new identities and does not relabel historical evidence. | Test (TC-227) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
