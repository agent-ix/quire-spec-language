---
id: TC-294
title: "An unresolved PopulationId refuses with a typed cause, not a panic or Undefined"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-294: An unresolved PopulationId refuses with a typed cause, not a panic or Undefined

## Description

Verify FR-089-AC-4: a `Value::Population(population_id)` whose `population_id`
names no binding recorded in the current evaluation's correspondence produces
a typed evaluator refusal naming the unresolved identity, never a panic and
never `Undefined`. Scope: FR-089-AC-4.

Known gap: no `PopulationId` type, no recorded correspondence, and no
resolution step exist yet. This test case fails against current code (there
is nothing to construct an "unresolved identity" from). Remaining work:
QSL-131 Slice B.

Catches an implementation that unwraps the correspondence lookup
unconditionally (panicking on a miss) or that treats a lookup miss as
`Undefined` — indistinguishable, to a caller, from a genuinely
indeterminate population value rather than a real defect (a `PopulationId`
that was never admitted in this evaluation at all, for example one replayed
from a different evaluation's identity space).

## Test Procedure

1. Construct a `Value::Population(population_id)` whose `population_id` was
   never produced by any `admit_binding`/`admit_invocation` call in the
   current evaluation (a distinct, independently-admitted identity from a
   separate evaluation run).
2. Evaluate an expression that consumes this value at one of the
   `evaluate.rs:921`/`:934` sites.

## Expected Results

Evaluation returns a typed refusal naming the unresolved `population_id`,
with no panic and no `Undefined` outcome. A mutant that maps a lookup miss
to `Undefined` passes any test that only checks "evaluation does not crash"
but fails this test's assertion that the outcome is specifically a named
refusal; a mutant that unwraps the lookup panics, failing under a harness
that treats a panic as a test failure rather than a passing refusal.
