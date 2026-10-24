---
id: TC-1192
title: "The conformance runner gives equal report and output bytes across runs and test thread counts"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: verifies
---
# TC-1192: The conformance runner gives equal report and output bytes across runs and test thread counts

## Description

Run the FR-350 determinism sample of the published QSpec corpus in four
child test processes, two with one test thread and two with two test
threads and a concurrent companion run, and show the report and each passed
vector's output bytes do not change. Inspect the
runner for thread creation.

Scope: FR-350-AC-6, FR-350-AC-7.

## Test Procedure

1. Select the determinism sample from the corpus at `QSPEC_DIR`. Within
   each capability of the default scope, for each vector class, take the
   bytewise-smallest id among the vectors of that class whose own
   applicability names a QSL consumer type.
2. From one parent test, start four child `cargo test` processes with the
   same build and default limits, each writing to its own output
   directory:
   - two with `RUST_TEST_THREADS=1`, each running the sample test alone;
   - two with `RUST_TEST_THREADS=2`, each running the sample test together
     with a companion test that runs the runner over the same sample. Both
     tests wait at one barrier before starting, so their runs overlap.
   Each child writes the FR-352 JSON report of the sample test's run and,
   for each `passed` outcome, the output bytes the outcome holds (the
   encoded checked package for `compile`, the FR-286 outcome document for
   `run` and `replay`, the formatted text for `format`), one file per
   vector id.
3. In the parent, compare the four reports byte for byte. For each vector
   `passed` in all four children, compare its four output files byte for
   byte.
4. Inspect the runner's source for thread creation, a thread pool, an
   async executor, or a vector call started before the previous vector
   settled.

Tag the tests `#[trace("TC-1192", "FR-350-AC-6")]`.

## Expected Results

- Step 1: the sample is nonempty and holds at most one vector per
  (capability, class) pair, each one a vector the runner executes.
- Step 2: each `RUST_TEST_THREADS=2` child records that the companion test
  was running while the sample test ran.
- Step 3: the four reports are byte-equal, and each passed vector's four
  output files are byte-equal. A difference names the vector, the two
  children and the first differing byte offset.
- Step 4: none is found.
