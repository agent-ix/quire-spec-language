---
id: TC-609
title: "max_witness_set and max_relation_tuples are caller budgets with published defaults"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: verifies
---
# TC-609: max_witness_set and max_relation_tuples are caller budgets with published defaults

## Description

Verify the two hyper budgets: their defaults, stopping before they are passed, and V-7 records that name the limit, its value and how to raise it.

Scope: FR-184-AC-1 to FR-184-AC-3.

## Test Procedure

Fixtures: §8.2's leaky `Opaque`; FR-179-AC-1's `Det`.

1. Run a request that sets neither member, once to a proof (`Det`) and once
   to a stop, and read the limits each terminal record states.
2. `Opaque` with `max_witness_set` 0 and 1.
3. `Det` with `max_relation_tuples` 63 and 64.

Tag the tests `#[trace("TC-609", "FR-184-AC-n")]`.

## Expected Results

- Step 1: both records state 65,536 and 16,777,216, marked as published
  defaults.
- Step 2: `Stopped(ResourceExhausted, MaxWitnessSet)`, value 0, count 1, no counterexample; FR-177-AC-1's outcome.
- Step 3: `Stopped(ResourceExhausted, MaxRelationTuples)`, value 63, count 63; `Holds`.
