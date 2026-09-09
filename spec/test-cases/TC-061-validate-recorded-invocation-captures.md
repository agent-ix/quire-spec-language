---
id: TC-061
title: "Validate recorded invocation captures"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-061: Validate recorded invocation captures

## Description

Integration, priority P1. Verifies FR-007-AC-3, FR-007-AC-9, FR-007-AC-10, FR-008-AC-6. Validation criteria are qualified at 45ed1b4 by SR-097; FR-008-AC-6's
predicate/captured-read behavior remains planned with TC-071. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Validate actual pre/post step invocations with ordered parameters, optional declared result and exact frame. Add missing/extra/duplicate/misnamed parameter/result roots, absent observations, missing required State roots and self deletion permitted by a separately admitted frame. Compare precondition self with postcondition self; read a pre-captured reference to an object deleted at post.

## Expected Results

Missing data is incomplete; mismatched supplied bindings refuse. Preconditions can validate a permitted deletion of self with self present in pre; postconditions require post self. Parameter/reference capture stays pre, result stays post and pre(alias) does not retag. Stated values are evaluated only after complete validation.
