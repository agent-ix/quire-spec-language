---
id: TC-1414
title: "Decisive protocol counterexamples carry one FR-351 record with a pinned value path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-574
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-351
    type: references
  - target: ix://agent-ix/quire-specification/FR-207
    type: references
---
# TC-1414: Decisive protocol counterexamples carry one FR-351 record with a pinned value path

## Description

Verify the record's presence, components, trace position and value path, and
that the state-clause record still round-trips.

Scope: FR-574-AC-1 to FR-574-AC-6.

## Test Procedure

1. Produce one result of each label for each kind; count records.
2. Read `deciding_quantifier`, `index` and the wire form of a projection and
   of each realizability counterexample; present each of the two on a
   record handed to the strict reader, and remove each of `deciding_element`,
   `value_path` and `trace_position` in turn.
3. Read `deciding_element` for each counterexample.
4. Read `trace_position` for the three counterexample forms, and for two
   counterexamples differing only in the divergent step.
5. Read `value_path`; change display text, path and span.
6. Round-trip a state-clause record (FR-265) with quantifier and index.

Tag the tests `#[trace("TC-1414", "FR-574-AC-n")]`.

## Expected Results

- Step 1: exactly one record per `decisive-counterexample`, none otherwise.
- Step 2: both absent and omitted on the wire; the strict reader refuses
  each altered record.
- Step 3: the obligation's node id string (realizability) or the divergent
  action's node id string (projection).
- Step 4: the zero-based decimal index by the FR-574 rule; the two differ.
- Step 5: `kind: runtime_value`, an `object` root, `steps` empty; equal after
  the presentation changes.
- Step 6: the record is unchanged.
