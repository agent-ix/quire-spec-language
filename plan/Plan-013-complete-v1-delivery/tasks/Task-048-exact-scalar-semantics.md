---
id: Task-048
title: "Implement exact scalar, numeric, text, enum and unit semantics"
type: Task
status: in_progress
track: A02
priority: P0
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-047
    type: depends_on
  - { target: ix://agent-ix/quire-specification/FR-140, type: references }
  - { target: ix://agent-ix/quire-specification/FR-141, type: references }
  - { target: ix://agent-ix/quire-specification/FR-142, type: references }
  - { target: ix://agent-ix/quire-specification/FR-147, type: references }
  - { target: ix://agent-ix/quire-specification/FR-148, type: references }
  - { target: ix://agent-ix/quire-specification/TC-185, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-186, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-187, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-192, type: verifies }
  - { target: ix://agent-ix/quire-specification/TC-193, type: verifies }
---
# Task-048: Implement exact scalar, numeric, text, enum and unit semantics

## Scope

Execute QSL #118 across unbounded/bounded integers, normalized rationals, exact
decimals, text profiles, enum identity, quantities/units and explicit IEEE
binary32/binary64 profiles.

## Subtasks

- [ ] Write TC-185–187 and TC-192–193 as deterministic property/vector tests.
- [ ] Implement types, conversions, equality, division and exceptional-value matrices.
- [ ] Preserve undefined, refused and incomplete outcomes without narrowing or float substitution.
- [ ] Pass local Rust gates and PR-time Rust/gap review; merge before Task-049.

## Deliverables

- Exact scalar value/check/evaluation surface and conformance corpus.
- Stable typed outcomes for every sign, boundary, precision, unit and cross-type case.

## Boundary

Task-048 supplies the scalar/text/enum/quantity/IEEE equality primitives needed
by its own types. Task-049 owns V1-EXPR-016, complete FR-149/TC-194 closure and
the record, tuple and collection rows; Task-048 must not claim that later
complete-matrix evidence.
