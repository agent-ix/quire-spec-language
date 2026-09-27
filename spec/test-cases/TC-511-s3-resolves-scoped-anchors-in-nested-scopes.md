---
id: TC-511
title: "S3 resolves scoped anchors through nested scopes and refuses a missing anchor or member"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: verifies
---
# TC-511: S3 resolves scoped anchors through nested scopes and refuses a missing anchor or member

## Description

Verify scoped anchor resolution: innermost scope outward, qualified paths,
and the `missing_declaration`/`missing-name` refusal.

Scope: FR-113-AC-1 to FR-113-AC-3.

## Test Procedure

Use TC-510's `RecoveryFlow` fixture.

1. Check `RecoveryFlow`; read each resolved anchor's target identity.
2. Add `sequence Inner { effect Applied of Tried as (inner: Boolean) { inner
   }; await InnerWait after Applied ...; await OuterWait after Main::Applied
   ...; }` inside `Main` after the outer `effect Applied`, and `await MainWait
   after Applied ...` in `Main` after `Inner`. Check it.
3. Check three variants, one defect each: `compensate Undo for Main::Missing`;
   `compensate Undo for Other::Applied`; `effect Applied of Absent`.

Tag the tests `#[trace("TC-511", "FR-113-AC-n")]`.

## Expected Results

- Step 1: no refusal; `Main::Applied` targets the `effect Applied` node,
  `Main::Committed` the `commit Committed` node, `Tried` the `attempt Tried`
  node and `Undo` the `compensate Undo` template, each by node identity.
- Step 2: `InnerWait` targets the inner `Applied`; `OuterWait` and `MainWait`
  the outer one.
- Step 3: `missing_declaration`/`missing-name` at the anchor, naming segment
  `Missing` and scope `[]`; then segment `Other`; then segment `Absent` and
  scope `[Main]`. No checked protocol node in any case.
