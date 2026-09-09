---
id: TC-096
title: "Retain mapped compiler refusals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-022
    type: verifies
---
## Description

Refuse invalid mapped requests and retain located native or package causes.

## Test Procedure

Change the declared language, native syntax, clause name/count and formal model
selection. Exercise Unicode and an indentation/CRLF map. Lower each stage's
budget to exhaustion, then retry with defaults.

## Expected Results

No request yields a partial package. Native errors map to exact original regions;
package errors retain their encoding path and have no invented native span.
Retries succeed under fresh budgets and retained source bytes do not change.
