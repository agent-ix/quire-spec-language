---
id: FR-324
title: "Admit unions and case under the value profile that admits records and tuples"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-316
    type: traces_to
  - target: ix://agent-ix/quire-specification/AD-015
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
---
# FR-324: Admit unions and case under the value profile that admits records and tuples

## Description

When a declaration's `using` alias selects a value profile that admits
records and tuples (`quire.value.complete/v1`), the compiler SHALL admit
union declarations, union constructions and `case` in that declaration on
the same basis, and SHALL add no profile identity, profile version or
per-feature edition gate for them, as QSpec FR-143 "Value profile" defines
(QSpec AD-015; ADR-012 §16.9 "Profiles").

## Inputs

- A complete-V1 unit's header profile selections (FR-110) and its
  declarations.

## Outputs

- Checked declarations that use unions, and the unit's definition lock.

## Behavior

- The compiler SHALL admit a union, a union construction or a `case` in any
  declaration where it admits a record or tuple under the same profile
  selection.
- A unit that uses unions SHALL resolve and record exactly the profile and
  definition selections it would record with each union replaced by a record
  (FR-110).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-324-AC-1 | A unit whose one header profile selects `quire.value.complete/v1` and that declares `Shape` and `area` compiles through spine `compile`, and its emitted lock holds exactly the `profile_selections` and `definition_selections` of the same unit with `Shape` replaced by `record Shape { tag: Integer; }` and `area` returning `0`. | Test (TC-834) |

## Dependencies

- QSpec FR-143 ("Value profile"), AD-015, AD-005.
- FR-110 (header profile selections), FR-316.

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.9, §16.10.
- Value profile naming `union` (SC-G7): QSpec FR-143 "Value profile"
  (specification ticket STD-115).
