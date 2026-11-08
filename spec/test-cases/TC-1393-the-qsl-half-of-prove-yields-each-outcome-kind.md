---
id: TC-1393
title: "The QSL half of prove yields an item of each outcome kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: verifies
---
# TC-1393: The QSL half of prove yields an item of each outcome kind

## Description

Run the `prove` (QSL half) row of FR-564's matrix: QSL engines served as
providers, and FR-291's plugin settlement records.

Scope: FR-564-AC-6.

## Test Procedure

1. Through `qsl_analyze::provider::run`, run FR-281-AC-1's item,
   FR-281-AC-2's item, FR-281-AC-11's `Admission` refusal, FR-281-AC-3's
   unsupported item and FR-281-AC-6's budgeted item; build one FR-286 item
   from each record.
2. Build the FR-291 terminal records for an FR-462 `incomplete` `timed-out`,
   an `incomplete` cancelled and a `failed` `tool-failure{crash}` settlement;
   build one FR-286 item from each.
3. Map each item's category through FR-285's function.

Tag the tests `#[trace("TC-1393", "FR-564-AC-6")]`.

## Expected Results

`proved` exit 0; `refuted` exit 10; `declined` with `invalid-input` exit 20;
`unsupported` exit 21; `incomplete` `timed-out` and `incomplete` cancelled,
exit 22; `incomplete` with the state-budget cause, exit 22; `failed` with
`tool-failure{crash}`, exit 30.
