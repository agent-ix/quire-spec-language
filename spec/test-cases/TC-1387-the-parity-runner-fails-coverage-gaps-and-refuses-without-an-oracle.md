---
id: TC-1387
title: "The parity runner fails coverage gaps and empty comparisons, refuses without an oracle, and excuses nothing"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-563
    type: verifies
---
# TC-1387: The parity runner fails coverage gaps and empty comparisons, refuses without an oracle, and excuses nothing

## Description

Verify the failure policy: coverage gaps, prepare failures and empty
comparisons fail; a faulting reference, an empty scope and an unreadable
corpus refuse; and no setting excuses a finding.

Scope: FR-563-AC-4, FR-563-AC-5, FR-563-AC-6 and FR-563-AC-8.

## Test Procedure

1. Run `run_parity` with a test backend whose coverage lists `let` and which
   lists every `let` entry unsupported; then with the same backend and `let`
   removed from its coverage.
2. Run it with a backend whose `prepare` returns `PrepareFailure::Limit` for
   one generated unit, and with a backend that lists every entry unsupported
   and supplies no coverage.
3. Run it with a reference that returns `Fault` for one entry; with no
   non-interpreter backend; with a corpus location that does not exist.
4. Inspect `ParityInputs`, `ParityLimits` and `make parity`'s options for a
   member that allows, skips, retries or tolerates a finding; run
   `make parity` with an unknown option.

Tag the tests `#[trace("TC-1387", "<AC id>")]`.

## Expected Results

- Step 1: one `coverage-gap` per `let` entry; then a pass, with those entries
  listed by their `UnsupportedEntry`.
- Step 2: `prepare-failed` naming the unit; `empty-comparison`.
- Step 3: `reference-fault`, `empty-scope` before any unit, and the corpus
  refusal, each with no verdict and exit 2.
- Step 4: no such member exists; the unknown option refuses, exit 2.
