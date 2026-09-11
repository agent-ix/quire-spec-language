---
id: Task-034
title: "Enforce the native sequence declaration ceiling"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-041
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-065
    type: verifies
---
## Scope

Compiler #30, the 2026-09-09 source-profile ruling: enforce the declared sequence
maximum of 10,000 in shared native model admission. Test used, unused and nested
declarations through real Rust and source-derived inputs, first demonstrating
the missing guard and then its correction. No actual large runtime allocation
is needed. Keep source identity, typed admission refusals and byte encoding.
Retain TC-065's actual hard Unicode/work exhaustion controls using individually
admitted nested sequence maxima instead of an oversized declaration.

This task addresses the sequence boundary only. FS01's definition/identity
amendment, deferred operators, rational support and the ConfigVersion reference
gate remain under #30 and their existing workstreams. It does not change the
meaning of already admitted sequence values or claim complete ruling conformance.

Use QUOIN specify and the selected all-set review, actual code/Rust review and
local serial checks at PR readiness. No intermediate review or hosted CI.

Implemented with three admission boundary tests and the adapted TC-065 stress
fixture. Both full feature lanes pass; SR-255–264 record the scoped PR review
and remaining matrix/profile qualification limits.
