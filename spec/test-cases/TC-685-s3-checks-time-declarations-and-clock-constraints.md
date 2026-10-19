---
id: TC-685
title: "S3 checks time declarations, clocks, clock constraints, time invariants and urgency"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: verifies
---
# TC-685: S3 checks time declarations, clocks, clock constraints, time invariants and urgency

## Description

Verify that S3 admits the `time` member, `Clock` fields, resets, clock constraints, time invariants and urgency, records them on the checked state model, and refuses every misuse at its span.

Scope: FR-230-AC-1 to FR-230-AC-5.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; its `s`-unit variant; a tick variant; the refusal fixtures of FR-230-AC-2; a 64-clock model.

1. Check `Rpc` and its `time dense unit s` variant.
2. Check each refusal fixture of FR-230-AC-2.
3. Check the tick model with period `1/2 ms`, with period `0 ms`, and with an unresolved tick operation.
4. Check the disjunctive guard and the 64-clock model with the `10^40 ms` constant.
5. Check `time dense unit ms` and `unit s` in a package declaring the time dimension with `s` and `ms`, the quantity `3 ms` under `unit s`, `unit hour`, a package with no time dimension, two packages declaring `ms` differently, and a `Clock` field of a package-admitted object type.

Tag the tests `#[trace("TC-685", "FR-230-AC-n")]`.

## Expected Results

- Step 1: `TimeSource::Dense{ms}`, clock `Call.x`, two time invariants, `send`'s reset, `reply`'s one convex part `x >= 1`; the `s` variant has bound `1/1000` and a different package identity.
- Step 2: each refuses with the code and span FR-230-AC-2 names.
- Step 3: `TimeSource::Tick` with period `1/2`; `invalid_model_binding`/`malformed-declaration`; `missing_declaration`/`missing-name`.
- Step 4: two convex parts each with `self.ready`; the large model checks with no refusal.
- Step 5: both models check and `3 ms` converts to `3/1000`; `missing_declaration`/`missing-name` naming `hour`; `missing_declaration`/`missing-name` naming `time`; different package identities; `unsupported_construct`/`not-yet-implemented` at the field. Steps 1, 2 (clock refusals) and 4 run once STD-192 lands.
