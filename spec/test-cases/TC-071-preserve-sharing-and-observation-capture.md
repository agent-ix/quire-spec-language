---
id: TC-071
title: "Preserve sharing and observation capture"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-071: Preserve sharing and observation capture

## Description

Integration, priority P1. Verifies FR-008-AC-6, FR-008-AC-14. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Execute let/if and nested pre expressions using the adopted precondition/immutable-parameter examples and pre/post populations with changed optional references. Contrast pre(alias) with pre(self.parent), conditional capture, parameter/result capture, and a pre reference to a deleted object. Bind a large sequence once and use lexical reads under a small comparison budget.

## Expected Results

Let initializer runs once, reads cost one expression step each and skipped branches do no work. Captured values retain their observation; direct reads use the selected observation. Reading large data does not spend deep-comparison work before comparison begins. No substitution duplicates initialization.
