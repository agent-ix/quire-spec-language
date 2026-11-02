---
id: TC-1290
title: "Project Quantity units into protocol evaluation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1290: Project Quantity units into protocol evaluation

## Description

Verify that reader-backed protocol evaluation preserves Quantity unit
identity and treats Dimension declarations as type metadata. This exercises
FR-390-AC-25.

## Test Procedure

Compile a fixture with `Time`, units `s` and `ms`, and a `Quantity<s>` value.
Add a protocol `finish.constraint` that compares two quantities with the same
unit and explicitly converts a value from `ms` to `s` before comparison. Emit
and admit the package. Evaluate the slot with the exact typed values and
inspect the reader-backed projection for the declared unit identity; verify
that the projected runtime type is Quantity with that `UnitId`, not a
standalone Dimension value. Attempt a direct comparison between the two
distinct unit types without conversion.

## Expected Results

A Quantity in `s` retains the `s` unit identity, while the explicit conversion
produces a value in the selected target unit and follows FR-142's conversion
rule. `Time` itself supplies no runtime value. Matching dimension names or
exponent vectors do not merge `s` and `ms`; a direct mixed-unit comparison is
refused before evaluation, while the explicit conversion comparison returns
the same result and ordered charge log as the in-process package.
