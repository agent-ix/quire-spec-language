---
id: TC-075
title: "Bound evaluation and immutable retries"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-075: Bound evaluation and immutable retries

## Description

Property, priority P1. Verifies FR-008-AC-5, FR-008-AC-17, FR-008-AC-18, FR-008-AC-19. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Generate deep eligible records, long equal Unicode prefixes, nested expressions and event-heavy quantifiers. Exercise each auxiliary/depth/event limit at exact, one below, zero and hard clamping, plus deterministic cancellation/panic boundaries. Re-run after success using smaller limits and after refused/incomplete runs; retain input-byte snapshots.

## Expected Results

Limits stop before excess work with the actual event prefix and no Boolean. Text advances count each side and final end checks. Depth 64/65 controls distinguish work limits from graph path length. Panic unwinds without a success report; all later calls have fresh usage, locals and visited sets. Inputs are unchanged.
