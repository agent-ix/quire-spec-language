---
id: TC-1395
title: "A registered process plugin changes no matrix outcome and only the candidate sets it advertises"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-566
    type: verifies
---
# TC-1395: A registered process plugin changes no matrix outcome and only the candidate sets it advertises

## Description

Verify FR-566's invariance over the matrix cases with a `Process`-origin
descriptor and provider.

Scope: FR-566-AC-1 and FR-566-AC-2.

## Test Procedure

1. Build the test plugin manifest for `qsl.test-plugin` advertising only
   (`value-validity`, `unbounded`); build its descriptor with
   `descriptor_from_manifest` and origin `Process`, and add the stand-in
   provider to `Engines` under it.
2. For every `Case`, call `run_case_with` over configurations 1, 2 and 3 of
   FR-566; also compare FR-275-AC-1's `package` bytes in the three. In
   configuration 3, run FR-281-AC-14's item settled
   `Supported(qsl.test-plugin)`.
3. Compute the candidate set of FR-281-AC-1's item and of an item of a kind
   the plugin advertises, in configurations 2 and 3, registering the plugin
   descriptor first and then last.

Tag the tests `#[trace("TC-1395", "<AC id>")]`.

## Expected Results

- Step 2: byte-equal documents or items and equal exit codes across the
  three configurations; byte-equal package bytes; the control item's record
  names `qsl.test-plugin` with certification `trusted`.
- Step 3: FR-281-AC-1's candidate set is unchanged; the advertised item's set
  gains exactly `qsl.test-plugin`; registration order changes nothing.
