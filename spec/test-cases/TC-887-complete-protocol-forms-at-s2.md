---
id: TC-887
title: "S2 builds the complete protocol forms with their members and spans"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: verifies
---
# TC-887: S2 builds the complete protocol forms with their members and spans

## Description

Verify that S2 builds a typed form for each complete protocol production,
with every member and span, keeps repeated `terminal` and `scheduling`
members, reads contextual keywords by position and refuses a recovering CST.

Scope: FR-308-AC-1 to FR-308-AC-5.

## Test Procedure

1. Build FR-308-AC-1's protocol with a replicated role, a bounded channel
   and `activation on each`; read the forms and spans.
2. Build the `quorum(2)`/`outstanding cancel` `parallel` and the
   `join all` variant.
3. Build the loop-proof `repeat` and the `max 3` `repeat`.
4. Build the protocol with three `terminal`/`scheduling` members, and one
   with an identifier `fork`.
5. Build a protocol whose CST holds a recovery node; build one declaration
   twice.

Tag the tests `#[trace("TC-887", "FR-308-AC-n")]`.

## Expected Results

- Step 1: a `Replicated` role (population `workers`, max 2, `Until`), a
  `ChannelForm` (`AtLeastOnce`, `Literal(1)`, `Block`) and an `OnEach`
  activation; spans cover their text.
- Step 2: `Quorum(2)` with `Some(Cancel)`; `All` with `None`.
- Step 3: no maximum, both proof blocks, no `exhausted`; maximum 3, no
  proof, an `exhausted` control.
- Step 4: three members in source order; `fork` built as an identifier.
- Step 5: `FormsCause::RecoveringCst` with no form; equal forms.
