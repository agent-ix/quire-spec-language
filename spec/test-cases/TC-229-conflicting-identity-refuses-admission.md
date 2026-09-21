---
id: TC-229
title: "Two members sharing universe and object identity but differing type refuse admission outright"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
---
# TC-229: Two members sharing universe and object identity but differing type refuse admission outright

## Description

Verify that two runtime member records naming the same universe and object
identity but different most-specific effective types refuse population
admission with a conflicting-identity cause and admit no binding, while
records with an equal key and an equal content digest correctly collapse
into one member rather than being treated as a conflict. Scope: FR-084-AC-4.

Catches an implementation that only compares object identity strings for
duplicate detection and ignores the type component, silently letting two
records for "the same object, two different types" collapse into one member
(picking one type arbitrarily) instead of refusing — a real identity-
integrity hole the FR-204 reference triple exists to close, invisible to a
test that only supplies non-conflicting inputs or exact duplicates.

## Test Procedure

1. Construct two runtime member records with the same universe identity and
   the same object identity string, but one naming most-specific type
   `Order` and the other naming most-specific type `LineItem`.
2. Admit a population binding whose runtime member input includes both
   records.
3. Construct a second admission input containing two records that are
   genuinely identical (same key, same content digest).
4. Admit a population binding for the second input.

## Expected Results

Step 2 refuses admission with a conflicting-identity cause, and no binding
results. Step 4 admits successfully with exactly one collapsed member, not
two. A mutant that de-duplicates by object-identity string alone (ignoring
type) collapses step 2's two records into one member instead of refusing,
failing the refusal assertion.
