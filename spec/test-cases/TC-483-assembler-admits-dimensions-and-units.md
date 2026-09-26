---
id: TC-483
title: "The assembler admits source dimensions and units into a UnitGraph and refuses each source error"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-483: The assembler admits source dimensions and units into a UnitGraph and refuses each source error

## Description

Verify that the assembler normalizes dimension terms, reduces exact
numbers, mints dimension and unit keys over the unit's source owner by
QSpec FR-142's nominal preimages, admits them through `UnitGraph::admit`,
and refuses each source error with its own cause.

This catches: unnormalized or unsorted terms; an unreduced scale or offset
(which `UnitGraph::admit` would refuse); a unit keyed before its target; a
unit name read as a type; `UnitGraph::admit` reached only from tests; and a
topology error reported as a reader's `invalid_semantic_graph`.

Scope: FR-091-AC-32, FR-091-AC-33, FR-091-AC-34, FR-091-AC-35.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`. The owner is authority `a`, identity `u`.

1. Assemble one unit holding the sources of FR-091 vectors Q1 to Q10. Read
   `units` and `nominal_spans`. Assemble a unit with no dimension or unit
   form and read `units`.
   With the decimal-scale bound set to `4`, assemble
   `dimension Length; unit m : Length = rational(1, 1);` with
   `unit c : Length = decimal(1, 5) * m;`, and again with `decimal(1, 4)`.
2. Assemble `dimension Length;`, `unit m : Length = rational(1, 1);` and
   `function f using v(x: m): Boolean pure { true }`.
3. Assemble FR-091-AC-34's unit.
4. Assemble each of FR-091-AC-35's five units.

Tag the tests with the AC ids they back and `TC-483`.

## Expected Results

- Step 1: dimensions keyed Q1 to Q5 and units keyed Q6 to Q10; Q3's and
  Q4's terms as the vectors list; `km`'s scale `1000`, `cm`'s `1/100`,
  `C`'s offset `5463/20`; `C` affine and `km` not; a span for each key in
  `nominal_spans`. The unit with no dimension or unit form has the empty
  `units`. `decimal(1, 5)` gives a limit refusal,
  `stage_limit_exceeded`/`work-budget-exceeded`, naming bound `4`, actual
  `5` and its span; `decimal(1, 4)` assembles.
- Step 2: an unresolved-type-name error naming `m`.
- Step 3: one refusal holding exactly the duplicate-name error (`Mass`), the
  unresolved-name error (`Width`), the two cycle errors (`P`/`Q`, `b`/`c`)
  and the zero-denominator error, each with the code FR-091-AC-34 names,
  and no `PackageDeclarations`.
- Step 4: each unit refuses with exactly one unit-graph topology error naming the
  declarations it concerns, and no `PackageDeclarations`. Its catalog code
  is the STD-112 cause.

## Status

Not implemented. QSL-275 adds the assembler's `UnitGraph` admission; today
S2 refuses `dimension` and `unit` with `NoDispatchEntry`. Step 4's catalog
code waits on STD-112 (FR-091-OQ-12).
