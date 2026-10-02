---
id: TC-895
title: "The native checker reads a field's optionality from its presence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-895: The native checker reads a field's optionality from its presence

## Description

Verify that the native checker types a domain-package field from its
declared presence and multiplicity, and never maps a multiplicity lower
bound of `0` to an option. Scope: FR-016-AC-10.

## Test Procedure

The admitted domain package declares an object type `T` with a Boolean
field `f`. A native unit binds `x: T` and is checked once per row:

1. `f` is `required` with multiplicity `[0, 1]`. Check `present(x.f)`.
2. `f` is `optional` with multiplicity `[1, 1]`. Check
   `present(x.f) implies value(x.f)`, and separately `value(x.f)` with no
   guard.
3. `f` is `optional` with multiplicity `[0, 1]`. Check `present(x.f)`.

## Expected Results

- Step 1: `x.f` types as a sequence of at most one Boolean, and
  `present(x.f)` refuses `ill_typed` at `x.f`.
- Step 2: `x.f` types as an option of Boolean. The guarded clause checks;
  the unguarded `value(x.f)` refuses `undefined_expression`.
- Step 3: `x.f` types as an option of a sequence of at most one Boolean,
  and `present(x.f)` checks.

## Status

🚧 Planned.
