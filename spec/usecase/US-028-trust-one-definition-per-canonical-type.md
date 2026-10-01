---
id: US-028
title: "Trust that a canonical type has one definition across QSL and its backends"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-271
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-272
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-273
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-028: Trust that a canonical type has one definition across QSL and its backends

## Story

**As a** QSL or backend contributor reading or changing code that handles a
shared concept such as a kernel value, an evaluation outcome or a checked
package
**I want** each canonical type to have exactly one definition, marked where
it is declared, with every same-named item inside QSL either deleted in its
favour or renamed to say what it is, and every copy of it in a backend
caught by that backend's own lint gate
**So that** a name in the code always means the one canonical concept, I
never have to work out which of several `Value`s or `Meter`s a signature
takes, and a copied type cannot drift from its source unnoticed.

## Context

ADR-013 §3 names one canonical owner and one public type for each shared
concept, and ADR-011 §4 requires exactly one QSL type per stage output. On
main, many canonical names are also declared elsewhere in the workspace:
generic names in the native-v1 seam modules, layer-crate types with a
different meaning, and two definitions of one concept. ADR-032 tags each
canonical type with `/// quire:canonical` and adds a gate that reads the
tags, so the set needs no list of names.

## Acceptance Examples (Illustrative)

### US-028-EX-1: A second definition fails the gate

- **Given** `quire_exact::Meter` carries the canonical tag.
- **When** a contributor adds a `struct Meter` to another QSL crate.
- **Then** `cargo xtask canonical-types` fails, naming both definitions,
  and the contributor names the new type for what it meters.

### US-028-EX-2: A backend copy fails the backend's gate

- **Given** a backend workspace that builds against `quire-exact`.
- **When** a contributor copies `ComparisonOperator` into the backend with
  the same variants.
- **Then** the backend's run of the same subcommand fails with a `copy`
  finding, and a same-named boundary type with different members passes.

### US-028-EX-3: Tagging a type is the whole registration

- **Given** an ADR-013 owner row that names a new public type.
- **When** its owner adds the tag to the type's doc comment.
- **Then** the gate covers it from then on, with no list edited anywhere.

## Priority and Risk (Informative)

Priority: High. Duplicate definitions are how byte-for-byte copies came to
sit in two crates before; without a gate, a renamed or moved type can leave
its old definition behind and both stay in use.

## Traceability (Informative)

- [FR-271](../functional/FR-271-tag-canonical-types-at-their-definition.md)
- [FR-272](../functional/FR-272-fail-on-a-second-definition-of-a-canonical-type.md)
- [FR-273](../functional/FR-273-resolve-each-canonical-type-namesake.md)
