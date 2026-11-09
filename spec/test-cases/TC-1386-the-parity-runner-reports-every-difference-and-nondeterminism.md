---
id: TC-1386
title: "The parity runner reports every difference and every nondeterministic result"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-563
    type: verifies
---
# TC-1386: The parity runner reports every difference and every nondeterministic result

## Description

Verify that `run_parity` passes a backend equal to the interpreter, fails a
backend that differs in any one compared member, fails a nondeterministic
backend, and reports findings up to its limit while reaching every unit.

Scope: FR-563-AC-1, FR-563-AC-2, FR-563-AC-3, FR-563-AC-7 and FR-563-AC-9.

## Test Procedure

1. Run `run_parity` with a test backend that delegates to the interpreter,
   over the corpus and the default generated units.
2. Run it once with each of eight test backends that alter exactly one
   compared member on one chosen entry: `Ok` to `Err`, outcome kind,
   completed value, typed cause, `Location`, loss records, `Incomplete`
   charge point, `CallFailure` payload. Run it with a backend that returns
   `Fault` on one entry.
3. Run it with a backend whose result alternates on repeated calls for one
   entry.
4. Run it with a backend that alters 150 entries, at the default
   `max_reported`, counting the units the runner reaches.
5. Run it over the FR-563-AC-9 corpus fixture with a backend that returns 6
   for `inc(4)`.

Tag the tests `#[trace("TC-1386", "<AC id>")]`.

## Expected Results

- Step 1: no finding, a compared count above zero, exit code 0.
- Step 2: each run has exactly one `difference` naming the altered member and
  carrying the seed, unit index, source bytes, entry, arguments and both
  results; the `Fault` run has one `difference`.
- Step 3: one `nondeterministic` finding naming the entry and arguments.
- Step 4: 100 findings held, total 150, and every unit reached.
- Step 5: one parity case, id the `run` vector's, tuple `(4)`; two vectors
  counted not selected; one `difference` carrying the vector id and `(4)`.
