---
id: TC-077
title: "Qualify the native reference API workflow"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: verifies
---
# TC-077: Qualify the native reference API workflow

## Description

Integration, priority P1. Verifies FR-007-AC-5, FR-008-AC-1, FR-008-AC-2, FR-018-AC-3.
Qualified at 4ac3597 by SR-099 through five public pipeline tests and the recorded
full regression. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Execute every step of IT-006 through actual source-derived Rust model admission and public parse/link/check/construct/validate/evaluate APIs. Record exact revisions, source/model/input references and healthy, violating, refused and incomplete outcomes, including operations.

## Expected Results

All IT-006 success criteria hold with explicit stage evidence. Actual source-bound reference results are separate from authored expectations and static proofs. This milestone does not replace IT-002 compiled ConfigVersion/backend qualification or LC02/FS03 acceptance.
