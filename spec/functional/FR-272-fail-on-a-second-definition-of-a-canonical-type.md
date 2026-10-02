---
id: FR-272
title: "Fail on a second definition or copy of a canonical type"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-028
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-032
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-271
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-061
    type: depends_on
---
# FR-272: Fail on a second definition or copy of a canonical type

## Description

QSL SHALL provide `cargo xtask canonical-types`, a gate that reads the
canonical set from FR-271's tags and fails when a canonical type has a
namesake inside the workspace, a re-export from a crate other than its
owner, or a same-shaped copy in an ecosystem dependency, as ADR-032 DT-2 to
DT-6 decide. It extends `xtask::definition_scan`. It runs on demand while
namesakes remain, and it joins `make ci` once it reports nothing over the
QSL workspace (ADR-032 R-1, FR-273-AC-1).

## Inputs

- `cargo metadata` for the workspace the gate runs in: the source root of
  every target of every workspace member (for example `tools/arch-lint`'s
  `[[bin]] path = "main.rs"`), and every resolved package that FR-061's ecosystem
  classification assigns to an ecosystem repository, read from its manifest
  directory.
- The shipped source of those directories (test code excluded, as
  `definition_scan` excludes it).

## Outputs

- One finding per violation, naming the canonical type, its definition's
  file and line, the second item's file and line, and the rule
  (`identifier`, `re-export`, `copy` or FR-271's `tag`).
- Exit status 0 when there is no finding, non-zero otherwise.

## Behavior

- If a module-level `struct`, `enum`, `union`, `trait` or `type` alias in a
  workspace member's shipped code has a canonical type's identifier and is
  not that type's tagged definition, then the gate SHALL report an
  `identifier` finding, whatever the item's visibility or shape.
- The gate SHALL apply the `identifier` rule to module-level items only; an
  associated type inside an `impl` or `trait` block is not a module-level
  item.
- If a workspace member other than a canonical type's owner crate has a
  `pub use` of that type whose path does not name the owner crate's
  defining module (a re-export of a re-export), or that renames it with
  `as`, then the gate SHALL report a `re-export` finding.
- If an item in an ecosystem dependency's shipped code has a canonical
  type's identifier, the same item kind and the same member names in the
  same order (field names of a struct, variant names of an enum, method
  names of a trait), then the gate SHALL report a `copy` finding. The
  comparison ignores documentation, attributes, visibility and the paths of
  member types.
- The gate SHALL take the scanned directories from `cargo metadata` and
  FR-061's classification, so the same subcommand run in a backend's
  workspace takes QSL's tagged crates as ecosystem dependencies and applies
  the `copy` rule to the backend's own code.
- The gate SHALL report every finding it meets before it exits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-272-AC-1 | Over a fixture workspace whose crate K tags `pub enum Value`: a private `struct Value` in crate W and a `pub(crate) enum Value` in a nested module of W each give an `identifier` finding naming K's definition and theirs; a serde visitor's `type Value = T;` inside an `impl` in W, and a `struct Value` inside W's `#[cfg(test)]` module, give none. In crate W, `pub use v::Value`, where crate V has `pub use k::Value`, and `pub use k::Value as Datum` each give a `re-export` finding; `pub use k::Value` in W, naming K's defining module, gives none. A `struct Outcome` in a workspace member whose only target is `[[bin]] path = "main.rs"` outside `src/` gives an `identifier` finding when K tags `Outcome`. | Test (TC-747) |
| FR-272-AC-2 | Over a fixture `cargo metadata` whose ecosystem package E sits beside K: an `enum ComparisonOperator` in E whose variants match K's tagged one in name and order gives a `copy` finding, and still does with E's docs, derives, visibility and member type paths changed; with one variant renamed (`LessEqual` for `LessOrEqual`), with two variants swapped, or as a `struct` with those field names, it gives none. A package outside the ecosystem holding the same copy gives none. | Test (TC-747) |
| FR-272-AC-3 | Backend run: a fixture backend workspace that depends on K as an ecosystem package and defines its own `pub enum Value` with K's variant names gives a `copy` finding in the backend's run. Over the QSL workspace, `quire-contract-model`'s `CollectionType`, `ComparisonOperator`, `EnumDeclaration`, `IntegerDomain` and `ValueType` give no `copy` finding, because their members differ from the tagged types'. | Test (TC-747) |
| FR-272-AC-4 | Two planted namesakes give two findings in one run, each naming both locations and its rule, and the gate exits non-zero; a fixture with no namesake, re-export or copy exits 0. | Test (TC-747) |

## Dependencies

- [ADR-032](../decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md)
  DT-2 to DT-6 and ruling R-1.
- [FR-271](FR-271-tag-canonical-types-at-their-definition.md) (the tag and
  the canonical set), [FR-273](FR-273-resolve-each-canonical-type-namesake.md)
  (the namesakes the gate reports on main),
  [FR-061](FR-061-check-duplicate-ecosystem-revisions.md) (ecosystem
  classification).

## References

- Owning ticket: Linear QSL-391. Implementation: Linear QSL-13.
- The backend lint gates that run the subcommand:
  agent-ix/quire-contract-runtime#56 and agent-ix/quire-contract-codegen#89.
