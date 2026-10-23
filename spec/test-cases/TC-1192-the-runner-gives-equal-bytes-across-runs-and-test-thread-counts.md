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

Run the FR-350 determinism sample of the published QSpec corpus four times,
twice with one test thread and twice at the harness default, and show the
report and each passed vector's output bytes do not change. Inspect the
runner for thread creation.

Scope: FR-350-AC-6, FR-350-AC-7.

## Test Procedure

1. Select the determinism sample from the corpus at `QSPEC_DIR`: within
   each capability of the default scope, for each vector class it holds,
   the vector with the smallest id.
2. Run the sample twice in a test process started with
   `RUST_TEST_THREADS=1`, with the same build and default limits. Keep each
   run's FR-352 JSON report and, for each `passed` vector, its entry
   point's output bytes (the encoded checked package for `compile`, the
   FR-286 outcome document for `run` and `replay`, the formatted text for
   `format`).
3. Run the sample twice more in a test process at the harness's default
   thread count, with other tests of the suite running alongside, keeping
   the same records.
4. Compare the four reports byte for byte, and for each vector `passed` in
   all four runs compare its four output byte strings.
5. Inspect the runner's source for thread creation, a thread pool, an
   async executor, or a vector call started before the previous vector
   settled.

Tag the tests `#[trace("TC-1192", "FR-350-AC-6")]`.

## Expected Results

- Step 1: the sample is nonempty and holds exactly one vector per
  (capability, class) pair present in the default scope.
- Step 4: the four reports are byte-equal, and each passed vector's four
  output byte strings are byte-equal. A difference names the vector, the
  two runs and the first differing byte offset.
- Step 5: none is found.
