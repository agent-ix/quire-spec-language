---
id: TC-779
title: "Each non-interpreter backend matches the interpreter over the conformance corpus and generated programs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: verifies
---
# TC-779: Each non-interpreter backend matches the interpreter over the conformance corpus and generated programs

## Description

The parity gate: run each non-interpreter execution backend against the interpreter, and show the gate catches a deliberately wrong backend.

Scope: FR-295-AC-1, FR-295-AC-5 and FR-295-AC-6.

## Test Procedure

1. For each linked non-interpreter backend, run every conformance-corpus program and the 10,000 programs the parity program generator yields at the default seed, with their argument tuples, on it and on the interpreter, comparing outcome, typed cause, `Location` and loss records.
2. Run the gate with a test backend that rounds one decimal operation.
3. Run the generator twice at the default seed and count 10,000, and once at another seed; check each unit with `check`, measure its size and forms, and run the interpreter over its tuples. Regenerate the unit step 2's report names, at its seed and index.
4. Run the gate at the default seed with a test backend that cannot translate one expression form, then with the same backend plus step 2's rounding fault.

Tag the tests `#[trace("TC-779", "<AC id>")]`.

## Expected Results

- Step 1: every comparison is equal and the gate passes.
- Step 2: the gate fails, naming the seed, the unit's index, its source bytes, the entry, the arguments and both outcomes.
- Step 3: the two default-seed runs are byte-equal and the other seed's first unit differs; every unit is admitted and within 1 to 8 functions, 0 to 4 parameters and 64 body nodes, calling only earlier functions; each of the eight forms and five scalar types occurs in at least 100 units; bounded `Int` tuples include both range ends; the interpreter's outcomes include `Completed` and `Undefined`; the regenerated unit is byte-equal to the reported source.
- Step 4: entries using the form are listed with `UnsupportedEntry` and cause `Untranslatable` naming `ExpressionForm` with the form's name at its first use and every other entry is compared, and the gate passes; with the rounding fault the gate fails.
