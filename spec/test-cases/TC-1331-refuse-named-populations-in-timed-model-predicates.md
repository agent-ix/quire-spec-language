---
id: TC-1331
title: "Refuse named populations in timed model predicates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: verifies
---
# TC-1331: Refuse named populations in timed model predicates

## Description

Verify that timed model predicates do not acquire an ambient named-population
scope, while their `self` state object and unit-level state-clause population
scope remain available.

## Test Procedure

Create a timed model whose domain package declares a `Call` type, a `calls`
population, a dense clock `x`, and an operation with a precondition. Compile
variants whose `time invariant` predicate ranges over `Config::calls` with
`forall` and whose `urgent when` predicate ranges over `Config::calls` with
`exists`. Compile a control variant with predicates that read only `self`, and
a precondition that ranges over `Config::calls`.

## Expected Results

Each timed-model variant with the named-population read is refused as
`unsupported_construct`/`not-yet-implemented` at `Config::calls`, with the
diagnostic naming the missing timed-predicate population scope and no checked
`TimedModel`. The self-only timed predicates and the unit-level precondition
check successfully.
