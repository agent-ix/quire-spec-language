---
id: TC-1415
title: "Constructs outside the bounded-static profile answer unsupported with a located cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-575
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-174
    type: references
---
# TC-1415: Constructs outside the bounded-static profile answer unsupported with a located cause

## Description

Verify each unsupported cause, the profile admission and that negotiation is
unchanged.

Scope: FR-575-AC-1 to FR-575-AC-4; QSpec FR-174-AC-3.

## Test Procedure

1. For each slug of FR-575, build a protocol using exactly that construct
   and request projection and realizability.
2. Request realizability over a protocol with a construct `qsl-eval`
   refuses.
3. Request a protocol using only profile constructs with profile
   `bounded-static`, then with the profile word `open`.
4. Negotiate each kind with no candidate; list `NegotiationCause`'s
   variants; check a symbolic-capacity protocol that `qsl-eval` also
   refuses.

Tag the tests `#[trace("TC-1415", "FR-575-AC-n")]`.

## Expected Results

- Step 1: `unsupported`, `unsupported_projection` with the slug and the
  construct's node id, for both kinds, with no graph, strategy or
  counterexample.
- Step 2: `evaluator-unsupported` carrying the `qsl-eval` cause.
- Step 3: decided; then refused `invalid_runtime_input`/`invalid-value`
  naming `profile`.
- Step 4: `unsupported_projection`/`unsupported-requested-capability`; the
  three existing variants only; the symbolic-capacity protocol reports
  `symbolic-queue-capacity`, not `evaluator-unsupported`.
