---
id: TC-089
title: "Qualify native package runtime reconstruction"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: verifies
---
# TC-089: Qualify native package runtime reconstruction

## Description

Integration, priority P1. Verifies FR-020-AC-1, FR-019-AC-1, FR-019-AC-7. Planned; no implementation or execution is claimed. Setup uses actual admitted models and compiler APIs before the target boundary.

## Test Procedure

Execute every step of IT-007 with the real admitted model and current/pre/post inputs. Drop the initially constructed package before readback, use exact external bindings and invoke actual parser/link/check/validate/evaluate APIs. Retain source/model/package/runtime references and independent truth/event/cost expectations.

## Expected Results

Healthy/violating, refused/incomplete, operation capture/frame and fresh-budget results match the independent expected observations after reconstruction. No original object cache, shared lowering oracle or authored expected JSON supplies the execution result.
