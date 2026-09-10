---
id: TC-101
title: "Compile rule-model source through the public frontend"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-025
    type: verifies
---
## Description

Preserve the existing qualified model semantics and original source occurrences.

## Test Procedure

Read the actual rule-model fixture through the production frontend. Compare
existing fixed artifact expectations and inspect declarations/roles at their
original occurrences. Exercise supported type forms, Unicode/escapes and native
state/operation execution. Keep fixture wrappers limited to setup and calls.

## Expected Results

Model correspondence and actual outcomes agree with the existing qualified
cases. Original source and declared metadata survive; no second lowering exists
in test helpers.
