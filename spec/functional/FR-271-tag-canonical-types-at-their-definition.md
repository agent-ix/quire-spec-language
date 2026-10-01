---
id: FR-271
title: "Tag each canonical public type at its definition"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-028
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-032
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
---
# FR-271: Tag each canonical public type at its definition

## Description

Each canonical public type SHALL carry the doc-comment tag
`/// quire:canonical` at its one definition, as ADR-032 DT-1 decides. The
tags are the canonical set: `cargo xtask canonical-types` (FR-272) reads the
set from them and from nothing else. A type is canonical when an ADR-013 §3
owner row names it as its owner's public type.

## Inputs

- The shipped source of every crate the gate scans (FR-272).

## Outputs

- The canonical set: each tagged type's identifier, kind, member names,
  file and line.
- A `tag` finding for each misplaced or duplicated tag.

## Behavior

- A canonical type's definition SHALL carry, in its outer doc comment, one
  line whose text after the `///` marker and surrounding whitespace is
  exactly `quire:canonical`.
- The tag SHALL be documentation only: the crate that carries it keeps its
  dependency list, and the compiled item is unchanged.
- The gate SHALL read the tag from the item's outer `doc` attributes,
  wherever the line sits among the item's other doc lines.
- If the tag sits on anything other than a `pub` `struct`, `enum`, `union`,
  `trait` or `type` alias, then the gate SHALL report a `tag` finding naming
  the file, line and item.
- If two scanned definitions of one identifier both carry the tag, then the
  gate SHALL report a `tag` finding naming both locations.
- The canonical set SHALL be exactly the set of tagged definitions, so
  tagging a type adds it to the set.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-271-AC-1 | Over a fixture of two crates, `pub struct Meter` in crate A whose doc comment holds the tag line between two prose lines, and a `struct Meter` in crate B, the gate reports one identifier finding naming A's definition and B's. With A's tag removed, written as `// quire:canonical`, or written as `/// quire:canonical type`, the gate reports nothing. | Test (TC-746) |
| FR-271-AC-2 | The tag on a `pub fn`, on a private `struct`, and on a struct field each give one `tag` finding naming the file, line and item. The tag on `pub struct Meter` in both A and B gives one `tag` finding naming both locations. | Test (TC-746) |

## Dependencies

- [ADR-032](../decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md)
  DT-1 and ruling R-3.
- ADR-013 §3 (the owner rows that make a type canonical).
- [FR-272](FR-272-fail-on-a-second-definition-of-a-canonical-type.md) (the
  gate that reads the set).

## References

- Owning ticket: Linear QSL-391. Implementation: Linear QSL-13.
