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
4. Run a populated model with inherited reference collections and optional references constrained by `subsets`, then with the target field redefined. Both valid snapshots pass. Change either the subsetting collection or optional reference to an admitted object absent from the target field: native admission refuses `subsetting-violation`. The original snapshot member order is retained. At the exact admission work bound the valid case passes, one unit short stays incomplete; repeating the population for a second parameter consumes the same cumulative admission budget.

5. Supply a canonical, schema-valid corpus whose independently authored
   expected value contains 50,000 nested singleton arrays and differs from
   the operation's result. Exercise the complete preflight, typed decoding,
   comparison, report serialization and release path on a bounded test stack.
   It reports `failed` with that unchanged oracle, and a repeated run retains
   the same report bytes; it neither aborts nor silently truncates the oracle.

Tag the tests `#[trace("TC-875", "FR-350-AC-1", "FR-350-AC-2", "FR-350-AC-8", "FR-350-AC-9", "FR-350-AC-10", "QSpec-FR-336-AC-4", "QSpec-FR-336-AC-7", "QSpec-FR-336-AC-8")]`.

## Expected Results

- Step 1: each positive vector is `passed` by the corresponding public operation. The run operation receives the typed `Call` and the object context admitted for that vector; replay reaches FR-098 with its supplied bytes and settles under FR-072. No vector is routed through an internal operation that its caller cannot reach.
- Step 2: `passed`; `failed` holding expected `ill_typed`/`type-mismatch` and actual `ill_typed`/`unit-mismatch`; `failed` holding the expected refusal and the actual success.
- Step 4: no invalid subset is admitted by dropping field values; inherited and redefining storage supply both original field identities, and no argument resets the shared meter.
- Step 5: the mismatching oracle is retained in the failed report, byte equal across repeated runs, without recursive stack failure.
