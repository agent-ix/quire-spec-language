---
id: TC-1017
title: "The model profile is selected by the header only"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1017: The model profile is selected by the header only

## Description

Verify the unit-level behaviour of the gate: every refusal is reported, per unit, with one refusal per construct, from a header selection and nothing else; and the `unknown_profile` behaviour. Scope: FR-361-AC-6 to FR-361-AC-12, FR-361-AC-14 and FR-361-AC-15. Steps map to criteria: step 1 AC-6, step 2 AC-6, step 3 AC-7, step 4 AC-8, step 5 AC-9, step 6 AC-10, step 7 AC-11, step 8 AC-12, step 9 AC-14, step 10 AC-15.

## Test Procedure

1. Compile a value-profile unit holding one construct of each family of item 5, one of them inside a declaration nothing uses.
2. Compile a library that selects the model profile imported by a unit that does not, writing a `body` against the import alias; then the reverse.
3. Compile a unit with a `model` declaration, a supplied domain package and a model construct, whose only header profile is `quire.value.complete/v1`.
4. (Retired with FR-361-AC-8: the layer selection rows are withdrawn.)
5. Compile a unit holding model constructs with the header profile set to the `ieee_profile` definition's identity, then to `test:unknown-profile`.
6. (Retired with FR-361-AC-10: no layer-table comparison.)
7. Read the S2 forms of the unit of step 1.
8. Compile `body b using V on M::A::add(x) { x }` with `V` the only profile (`quire.value.complete/v1`); then with a second profile `W` naming `quire.model.complete/v1`, still `using V`.
9. Compile a unit whose only profile `t` names `quire.temporal.complete/v1`, holding a `model` declaration and one construct of each family of item 5 written `using t`; then the same unit with the profile naming `quire.value.complete/v1`; then a unit whose profile names `quire.model.complete/v1` and that writes a `time` member.
10. (Retired with FR-361-AC-15: no layer-table comparison.)

## Expected Results

1. One `unsupported_construct` refusal per construct, in source order, each retaining the selected and required layers.
2. The unit lacking the selection gets its refusals, including for its own forms written against the import alias; the other unit does not.
3. The construct is refused; the unit never compiles.
4. Retired (the layer selection rows are withdrawn).
5. `unknown_profile`/`wrong-selection-role`, then `unknown_profile`/`unsupported-selection`; no S3 refusal.
6. Retired.
7. S2 holds a form for every construct and reports no refusal.
8. The first compile has one refusal (the family refusal); the second has FR-110's layer refusal alone.
9. The first unit compiles with no model-gate refusal because the temporal layer's `requires` closure includes `quire.model.complete/v1`; the second refuses each model construct once; the third refuses the `time` member alone (`unsupported_construct`/`declaration-form`, FR-230) and no model construct.
10. Retired.
