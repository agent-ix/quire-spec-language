---
id: TC-1291
title: "Project Alias types transparently into protocol evaluation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1291: Project Alias types transparently into protocol evaluation

## Description

Verify that an Alias has its target's effective evaluator type without adding
a runtime wrapper or alias-name identity. This exercises FR-390-AC-26.

## Test Procedure

Compile a fixture defining aliases to String, UUID, Timestamp, `Quantity<s>`
and a Union. A protocol `finish.constraint` reads each alias-typed field and
compares it with the corresponding target-typed value using only the
operations allowed for that target. Emit and admit the package, then evaluate
the slot through both the in-process and reader-backed views.

## Expected Results

Each alias projects as its admitted target type and has the target's existing
evaluator behavior. No alias wrapper or alias-name identity appears. Each
reader-backed evaluation returns the same result and ordered charge log as the
in-process evaluation.
