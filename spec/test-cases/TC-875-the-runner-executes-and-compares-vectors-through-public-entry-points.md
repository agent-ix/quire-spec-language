---
id: TC-875
title: "The conformance runner executes vectors through public entry points and compares typed results"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: verifies
---
# TC-875: The conformance runner executes vectors through public entry points and compares typed results

## Description

Verify that the runner runs each vector through its subject's public entry point and compares the result with the expected disposition, including the code and cause of a refusal.

Scope: FR-350-AC-1, FR-350-AC-2.

## Test Procedure

Build a fixture corpus in the test's temporary directory in the QSpec FR-336 vector shape, holding:

1. a positive compile vector whose expected disposition is the checked package identity of a two-function unit; a positive `run` vector calling one function with its expected value; a `replay` vector over a frame counterexample with its expected agreement; a `format` vector with its expected bytes;
2. a refusal vector whose source adds a `Meter` to a `Second` and which expects `ill_typed`/`unit-mismatch`; the same vector expecting `ill_typed`/`type-mismatch`; a refusal vector expecting `ill_typed`/`type-mismatch` over source QSL admits.

Run the runner over the corpus. Tag the tests `#[trace("TC-875", "FR-350-AC-n")]`.

## Expected Results

- Step 1: four `passed` outcomes. Each vector's result equals the result of calling `compile`, `run`, `replay` or `format` directly on the same input.
- Step 2: `passed`; `failed` holding expected `ill_typed`/`type-mismatch` and actual `ill_typed`/`unit-mismatch`; `failed` holding the expected refusal and the actual success.
