---
id: TC-236
title: "Two declarations sharing a display title classify and resolve independently by key"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: verifies
---
# TC-236: Two declarations sharing a display title classify and resolve independently by key

## Description

Verify that resolving a systems-model element by required kind matches the
full declaration key, never a shared display title: two declarations with
equal title but distinct declaration keys, one a genuine `Part` and one
without the part-signature capability, classify and resolve independently.
Scope: FR-086-AC-4.

Catches an implementation that indexes its classification map by title (or
by title as a fallback when a key lookup "seems" to miss), so that
resolving one declaration's key accidentally returns the other's kind
because they share a title string — a defect invisible to any test whose
fixtures use distinct titles for distinct declarations, which is the common
case and therefore easy to leave uncaught.

## Test Procedure

1. Declare component `pump_1` (artifact id `pump_1`, title "Pump") that
   supplies the part-signature capability.
2. Declare a second, unrelated component `pump_2` (artifact id `pump_2`,
   title also "Pump" — the same display title as `pump_1`) that does not
   supply the part-signature capability.
3. Classify the domain package containing both.
4. Resolve `pump_1`'s key as required kind `Part`, and separately resolve
   `pump_2`'s key as required kind `Part`.

## Expected Results

`pump_1` resolves successfully as `Part`. `pump_2`'s resolution refuses
(unsupplied-producer-record), naming `pump_2`'s own key. A mutant that looks
up classification by title, or falls back to a title-keyed index, resolves
`pump_2` as `Part` too (borrowing `pump_1`'s classification because they
share a title), failing the refusal assertion for `pump_2`.
