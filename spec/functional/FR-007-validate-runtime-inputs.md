---
id: FR-007
title: "Validate runtime snapshots and invocations"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-003"
    type: traces_to
---
# FR-007: Validate runtime snapshots and invocations

## Description

When evaluation is requested, the compiler runtime shall validate the selected snapshots and invocation bindings before evaluating the predicate.

## Inputs

Exact typed populations, completeness, observations, invocation, frame and object-delta records.

## Outputs

Validated immutable runtime context or refused/incomplete input.

## Behavior

Validation includes skipped predicate fields, type/universe/reference closure, required observations and authored frame permissions. Unavailable data is not an empty population. Static linked packages exclude runtime observations. The portable result envelope remains B-owned.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-007-AC-1 | A dangling target in a complete universe receives dangling_reference. | Test |
| FR-007-AC-2 | An incomplete population receives incomplete_population. | Test |
| FR-007-AC-3 | A mismatched invocation receives wrong_snapshot. | Test |
| FR-007-AC-4 | An unauthorized field change receives frame_violation. | Test |
| FR-007-AC-5 | Failed validation yields no predicate Boolean. | Test |

## Dependencies

- [US-003](../usecase/US-003-evaluate-bounded-state.md) supplies the user need.
- [Detailed contract or implementation evidence](../../README.md) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
