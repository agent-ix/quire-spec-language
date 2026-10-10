---
id: TC-768
title: "Core operations read no ambient input and write nothing to the process streams"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-284
    type: verifies
---
# TC-768: Core operations read no ambient input and write nothing to the process streams

## Description

Verify the qualified core's results do not depend on the process environment and that it writes nothing to stdout or stderr.

Scope: FR-284-AC-2, FR-284-AC-3, FR-284-AC-4.

## Test Procedure

1. Run TC-755 step 1's chain and FR-098's replay of a fixture counterexample in the test's own environment and record the outcomes.
2. Run the same calls in a child process whose environment holds 200 generated variables with arbitrary values, whose working directory is an empty temporary directory and whose `HOME` is unset, capturing stdout and stderr.
3. Run the ambient-input scan over a source tree whose core crate's shipped code holds one site of each category FR-284-AC-4 names, and over one that holds those sites only in comments, string literals and `#[cfg(test)]` code.

Tag the tests `#[trace("TC-768", "<AC id>")]`.

## Expected Results

- Step 1: the outcomes are recorded.
- Step 2: the outcomes equal step 1's, the captured stdout and stderr are empty, and the child exits only when the harness ends it.
- Step 3: the first scan names each site's file, line and category; the second finds nothing.
