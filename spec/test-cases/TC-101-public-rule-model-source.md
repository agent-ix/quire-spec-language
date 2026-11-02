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

Read the actual rule-model fixture through the retained production frontend.
Inspect admitted declarations and roles at their original source occurrences.
Exercise supported type forms and Unicode/escapes. Keep fixture wrappers
limited to setup and calls. Native compilation and state/operation execution
are retired with the native package/runtime path.

## Expected Results

Model correspondence agrees with the retained cases. Original source and
declared metadata survive; no second lowering exists in test helpers.
