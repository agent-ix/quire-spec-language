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

Implemented (QSL-131 Slice B): `CheckedPackage::call`/`evaluate`'s own
argument-admission `validate` (`qsl-eval/src/value/expression/mod.rs`) resolves
every `Population<T>[N]` argument through `ObjectEnvironment::
resolve_population` before any charge, and refuses with
`InputRefusal::WrongValueKind`, naming the parameter position (the same
shape every other `InputRefusal` variant in this crate uses -- `Arity`,
`DanglingReference` -- trusting the caller to correlate the position back
to the identity it supplied), whenever the identity does not resolve. This
holds whether or not the checked body actually reads the parameter (PR #326
review finding F1). The evaluator's own `Machine::resolve_population`
(`qsl-eval/src/value/expression/evaluate.rs`'s `AllInstances`/`Lookup` sites) performs
the identical resolution, but no longer as a kernel refusal: since FR-090-
AC-10 (QSL-174), an unresolved identity reaching it is an S6a invariant
break, `Err(InternalFault)` naming stage `"S6a"` (never `Refusal::
UnresolvedPopulation`, which is deleted along with its one production call
site). It is unreachable through either public entry point for a checked
program once `validate` already refuses first, and is kept only as defence
in depth; `tc_294_unresolved_population_id_refuses_typed` exercises
`validate`'s own admission refusal, not this now-unreachable fault path.

Catches an implementation that unwraps the correspondence lookup
unconditionally (panicking on a miss), that treats a lookup miss as
`Undefined` — indistinguishable, to a caller, from a genuinely
indeterminate population value rather than a real defect (a `PopulationId`
that was never admitted in this evaluation at all, for example one replayed
from a different evaluation's identity space) — or that admits an
unresolved identity by presence alone whenever the checked body never
consumes it.

## Test Procedure

1. Construct a `Value::Population(population_id)` whose `population_id` was
   never produced by any `admit_binding`/`admit_invocation` call in the
   current evaluation (a distinct, independently-admitted identity from a
   separate evaluation run).
2. Evaluate an expression that consumes this value at one of the
   `evaluate.rs:921`/`:934` sites (`tc_294_unresolved_population_id_refuses_typed`),
   and separately an expression that never reads it at all
   (`tc_294_unresolved_population_id_refuses_even_when_unconsumed`) and a
   `lookup<T>(p, r)`-shaped consumer (`tc_294_lookup_refuses_an_unresolved_population_id`).

## Expected Results

Every case refuses at argument admission (`InputRefusal::WrongValueKind`,
naming the parameter), with no panic and no `Undefined` outcome. A mutant
that maps a lookup miss to `Undefined` passes any test that only checks
"evaluation does not crash" but fails this test's assertion that the
outcome is specifically a refusal; a mutant that unwraps the lookup panics,
failing under a harness that treats a panic as a test failure rather than a
passing refusal; a mutant that checks resolution only when the body
consumes the parameter fails the "never reads it at all" case.
