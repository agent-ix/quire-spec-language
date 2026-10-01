---
id: US-032
title: "Model a closed set of alternatives with a union and handle every case"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-316
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-317
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-318
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-319
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-320
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-321
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-322
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-323
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-324
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-032: Model a closed set of alternatives with a union and handle every case

## Story

**As a** specification author writing complete-V1 source
**I want** to declare a `union` whose members carry their own payloads, build
its values, and take them apart with a `case` that the compiler proves
handles every member
**So that** a value that is one of several shapes is modelled directly,
a forgotten or misspelled member is refused at compile time with a located
cause, and the checked meaning of each value survives packaging, evaluation
and replay unchanged.

## Context

QSpec specifies the source forms and their meaning: `union-decl`, union
construction, `case` and its exhaustiveness obligations (QSpec AD-015,
FR-143, FR-146). QSL compiles and evaluates them through the `SumCase`
family of the shared family contract (ADR-012 §16). QSpec owns the wire
spelling, the member identity preimage, the accounting charge points, the
canonical key, the `case` result-type rule and the value-profile admission
(the QSpec halves in References).

## Acceptance Examples (Illustrative)

### US-032-EX-1: A union is declared, built and matched

- **Given** source that declares
  `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }` and a
  function whose body is a `case` over a `Shape` parameter with one arm per
  member.
- **When** the caller compiles the source and runs the function with
  `Shape::Rect(2, 3)`.
- **Then** the function is admitted, and the run takes the `Rect` arm with
  its two payload values bound in order.

### US-032-EX-2: A missing arm is refused at compile time

- **Given** the same function with the `Empty` arm removed.
- **When** the caller compiles the source.
- **Then** the compiler refuses the `case` with
  `undefined_expression`/`unproved-exhaustiveness`, naming `Shape`, the arm
  set and the `missing-arm` obligation, located at the `case`.

### US-032-EX-3: A recursive union describes a finite tree

- **Given** `union Tree { Leaf, Node(Integer, Option<Tree>, Option<Tree>) }`.
- **When** the caller compiles it and evaluates a function that sums a tree
  of any depth.
- **Then** the declaration is admitted, and evaluation of a deep tree
  completes or stops only at a caller-configured resource limit that names
  itself.

### US-032-EX-4: Reordering arms changes nothing

- **Given** two sources that differ only in the order of one `case`'s arms.
- **When** both are compiled and packaged.
- **Then** both give the same `case` node identity and the same package
  identity.

## Priority and Risk (Informative)

Priority: High. `case` over a declared union is the only pattern-matching
form in complete V1 (QSpec FR-146). Without it, a closed set of alternatives
can only be approximated with enums and parallel optional fields, which the
checker cannot prove exhaustive.

## Traceability (Informative)

- FR-315 to FR-324.
- QSpec AD-015, FR-143, FR-146.

## References

- QSpec half: wire spelling QSpec FR-440 (SC-G1, SC-G2); member key
  FR-441 (SC-G3); accounting charge points FR-143, FR-146 and
  `value-accounting.md` (SC-G4); canonical key FR-144 (SC-G5); `case`
  result type FR-146 "Case result type" (SC-G6); value profile FR-143
  "Value profile" (SC-G7). Specification tickets STD-142, STD-115.
- Owning ticket: QSL-383.
