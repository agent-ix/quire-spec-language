---
id: TC-1078
title: "S1 and S2 parse and build time declarations, quantity literals and timed intervals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: verifies
---
# TC-1078: S1 and S2 parse and build time declarations, quantity literals and timed intervals

## Description

Verify that S1 parses and S2 builds the `time-decl`, `time-invariant`, `urgent-decl`, `Clock` type, `quantity-literal` and `timed-interval` forms, dispatches each leading token to its one production, and defers every meaning check to S3.

Scope: FR-393-AC-1 to FR-393-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit written in the shared grammar's productions; a tick variant; a unit with two `time` declarations; claims with the intervals of FR-393-AC-3 and FR-393-AC-4.

1. Build `Rpc` and the tick variant and read back each form, its order and its spans.
2. Dispatch units beginning `time invariant`, `time` and `urgent`.
3. Parse the four claims of FR-393-AC-3 and read each operator form.
4. Build `eventually[0 s, 0.003 s]`, `(2 ms, 2 ms]`, `[3 ms, 2 ms]` and the two-`time` unit; parse `eventually[0 ms, x]`.

Tag the tests `#[trace("TC-1078", "FR-393-AC-n")]`.

## Expected Results

- Step 1: the forms of FR-393-AC-1, each with its source span; `Tick{send, 1/2 ms}` for the variant.
- Step 2: exactly one call each, to `time_invariant`, `time_decl` and `urgent_decl`.
- Step 3: the closed/open flags, magnitudes and `None` members of FR-393-AC-3.
- Step 4: upper magnitude `3/1000`; the last three build; `eventually[0 ms, x]` fails at S1 at `x`'s span with no form.
