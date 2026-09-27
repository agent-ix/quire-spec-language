---
id: TC-512
title: "S3 refuses ambiguous anchors and shadowing binders, in builder order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: verifies
---
# TC-512: S3 refuses ambiguous anchors and shadowing binders, in builder order

## Description

Verify the `ambiguous_declaration`/`ambiguous-name` refusals for two
declarations of one name in one scope and for a shadowing binder, and the
refusal order.

Scope: FR-113-AC-4 to FR-113-AC-6.

## Test Procedure

Use TC-510's `RecoveryFlow` fixture.

1. Add a second `effect Applied of Tried ...` in `Main`. Check it; then swap
   the two `Applied` declarations and check again.
2. Inside `Undo`, add `capture forward: Boolean = saved;` (the template binds
   `forward`). Check it. Separately, rename `attempt Tried`'s record binder
   to `M`. Check it. Separately, add a second protocol to the unit whose
   `attempt` record binder is also `attempted`. Check it.
3. Combine `effect Applied of Absent` with the shadowing `forward` capture.
   Check it twice.

Tag the tests `#[trace("TC-512", "FR-113-AC-n")]`.

## Expected Results

- Step 1: `ambiguous_declaration`/`ambiguous-name` at both `Applied`
  declarations and at the `Main::Applied` anchor, naming both in source
  order; after the swap, the same refusals with the two loci in the new
  source order.
- Step 2: `ambiguous_declaration`/`ambiguous-name` at the capture naming the
  bound parameter `forward`; at the binder `M` naming the model alias; the
  second protocol checks.
- Step 3: both refusals, ordered by builder state then source position, no
  checked protocol node, and equal results on both runs.
