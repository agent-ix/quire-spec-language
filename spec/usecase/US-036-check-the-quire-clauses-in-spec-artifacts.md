---
id: US-036
title: "Check the Quire clauses written in spec artifacts"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-355
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-036: Check the Quire clauses written in spec artifacts

## Story

**As a** spec author who writes domain objects as spec artifacts
**I want** every `quire` fence in my artifacts parsed, resolved against the
object it belongs to and type-checked, with each error naming the file and
line
**So that** a clause that calls `present` on a required field, puts an
integer where a condition belongs, names a field the object does not
declare or names an enumeration fails the check with a precise code,
instead of passing every gate unread.

## Context

A spec artifact declares a domain object: its frontmatter names the object
type, its Properties table declares the fields, and each clause under its
Invariants section owns one `quire` fence holding a Quire expression over
`self`. quire-rs records each fence's language as `quire` and reads nothing
inside it. QSL already compiles the same artifacts into a domain package
(FR-056) and checks state invariants over that package's object types
(FR-104), so it can check each fence as an invariant of the object its
artifact declares.

## Acceptance Examples (Illustrative)

### US-036-EX-1: A well-formed artifact passes

- **Given** an entity artifact whose Properties table declares
  `verified: Boolean 1..1` and `first_order: Integer 1..1` with presence
  `optional`, and whose one clause reads
  `present(self.first_order) implies self.verified`.
- **When** the author checks the artifact.
- **Then** the check reports no diagnostic.

### US-036-EX-2: Each error names its fence

- **Given** the same artifact with a second clause
  `present(self.verified)` and a third clause `self.nickname`.
- **When** the author checks the artifact.
- **Then** the check reports one diagnostic for each clause, at the
  artifact's path and the line inside each fence where the error is, and
  the first clause reports nothing.
