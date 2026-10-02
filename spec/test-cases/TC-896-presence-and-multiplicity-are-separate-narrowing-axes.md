---
id: TC-896
title: "A field redefinition narrows presence and multiplicity on separate axes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: verifies
---
# TC-896: A field redefinition narrows presence and multiplicity on separate axes

## Description

Verify that the model checker engages the `field-presence` obligation for a
presence narrowing, whatever the multiplicities are, and engages none
through presence when both fields declare the same presence. Scope:
FR-082-AC-8.

## Test Procedure

Object type `Sub` specializes `Base`. `Base` declares a Boolean field `f`;
`Sub` redefines it. An exposed operation `set` of `Sub` writes `f`.

1. `Base.f` is `optional` `[1, 1]`; `Sub.f` is `required` `[1, 1]`; `set`
   has no postcondition.
2. As step 1, with `set`'s postcondition `present(self.f)`.
3. `Base.f` and `Sub.f` are both `required` `[0, 1]`; `set` has no
   postcondition.
4. `Base.f` and `Sub.f` are both `optional` `[1, 1]`; `set` has no
   postcondition.

## Expected Results

- Step 1: refused `undefined_expression`/`unproved-refinement` with
  obligation `field-presence`, naming `Sub.f`.
- Step 2: the redefinition is admitted.
- Steps 3 and 4: the redefinition is admitted, and no refinement obligation
  is recorded for `Sub.f`.

## Status

🚧 Planned.
