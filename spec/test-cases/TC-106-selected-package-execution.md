---
id: TC-106
title: "Execute or refuse selected compiled package files"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-028
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
## Description

Integration, P1: select exported native package files at the actual run-command
boundary and inspect runtime outcomes, original byte identity and reader errors.

## Test Procedure

Run state/operation examples with exact and alternate-layout package bytes.
Mutate reconstructed claims, source/bindings and request shape;
exercise intake/runtime limits and fresh retries. Run existing command tests.
Require the selected
package's bytes counted toward `dependent_bytes`. Run selected files through absolute and parent-relative operands.

## Expected Results

Only verified packages reach runtime execution. Reader failures preserve their typed details without
truth or fallback. Source-only run and compile remain available.
