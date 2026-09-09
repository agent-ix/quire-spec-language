---
id: TC-066
title: "Repeat immutable runtime validation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
---
# TC-066: Repeat immutable runtime validation

## Description

Property, priority P1. Verifies FR-007-AC-15. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

For a generated set of valid/adverse snapshots record model/source/artifact bytes, then run success, refusal, exhaustion and cancellation followed by another request. Revalidate with a smaller work limit after success and with reversed inventory order at sufficient budget.

## Expected Results

Input/model/source bytes remain unchanged. Each request repeats its own accounting; a cached success never bypasses a smaller limit. Successful contexts retain their exact checked-package/input correspondence, independently of later requests.
