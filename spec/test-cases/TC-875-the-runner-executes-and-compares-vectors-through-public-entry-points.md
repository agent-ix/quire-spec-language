---
id: TC-875
title: "The conformance runner executes vectors through public entry points and compares typed results"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-336
    type: verifies
---
# TC-875: The conformance runner executes vectors through public entry points and compares typed results

## Description

Verify QSL's corpus adapter, dispatch and typed-result settlement through the public entry points. QSpec FR-336 owns the corpus schema, input encodings and expected-result vocabulary; this test uses its contract through a test helper and does not maintain a second schema or vector definition.

Scope: FR-350-AC-1, FR-350-AC-2, FR-350-AC-8, FR-350-AC-9, FR-350-AC-10.

## Test Procedure

1. Use the shared test helper and QSpec FR-336 contract to prepare QSL-applicable positive vectors for compile, run with object context, replay and format. Include call inputs that exercise scalar and object-context arguments, and a replay input whose byte provisions are available to the vector.
2. Include a refusal vector for an ill-typed unit operation, one copy whose expected refusal differs from QSL's actual refusal, and one expected refusal over source QSL admits.
3. Run the qualification entry point over this corpus and inspect each per-vector outcome and the public-operation invocation records.

Tag the tests `#[trace("TC-875", "FR-350-AC-1", "FR-350-AC-2", "FR-350-AC-8", "FR-350-AC-9", "FR-350-AC-10", "QSpec-FR-336-AC-4", "QSpec-FR-336-AC-7", "QSpec-FR-336-AC-8")]`.

## Expected Results

- Step 1: each positive vector is `passed` by the corresponding public operation. The run operation receives the typed `Call` and the object context admitted for that vector; replay reaches FR-098 with its supplied bytes and settles under FR-072. No vector is routed through an internal operation that its caller cannot reach.
- Step 2: `passed`; `failed` holding expected `ill_typed`/`type-mismatch` and actual `ill_typed`/`unit-mismatch`; `failed` holding the expected refusal and the actual success.
