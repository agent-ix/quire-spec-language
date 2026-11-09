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
   FR-281-AC-2's item, FR-280-AC-6's request with the undeclared-population
   binding, FR-281-AC-11's EN-1 `RequiresBound` item and FR-281-AC-6 (a)'s
   budgeted item, each over bytes and a provision; build one FR-286 item from
   each record or refusal. A cell FR-564 marks † that is not yet a `Case` is
   not run and is checked to be a `PendingEngine` cell (FR-564-AC-8).
2. Build FR-291-AC-2's `incomplete` `TimedOut` and `incomplete` cancelled
   records and FR-291-AC-1's `failed` record with `ToolFailure::Crash`;
   build one FR-286 item from each.
3. Map each item's category through FR-285's function.

Tag the tests `#[trace("TC-1393", "FR-564-AC-6")]`.

## Expected Results

`proved` exit 0; `refuted` exit 10; the binding refusal exit 20;
`unsupported` with `unsupported_projection`/`unbounded-extent` exit 21; `incomplete` `timed-out` and `incomplete` cancelled,
exit 22; `incomplete` with `LimitReached{max_states}`, exit 22; `failed` with
`tool-failure{crash}`, exit 30.
