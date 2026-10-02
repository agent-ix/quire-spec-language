---
id: TC-688
title: "Model claims bind model-time or model-steps, with defaults, identity keys and interval units"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-233
    type: verifies
---
# TC-688: Model claims bind model-time or model-steps, with defaults, identity keys and interval units

## Description

Verify the default and explicit clock bindings over timed and untimed models, binding identity, and interval units under each binding.

Scope: FR-233-AC-1 to FR-233-AC-3.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the untimed `Counter` unit; a tick variant of `Rpc`.

1. Check `Settles` with and without `clock "model-time"`, `NoLateReply` with `clock "model-steps"`, and a `Counter` clause with no clock clause and with `clock "model-time"`.
2. Check `eventually[0 ms, 3 ms]` and `eventually[0 s, 3/1000 s]` under `model-time`, `eventually[0, 3]` under `model-time`, and `eventually[0 ms, 3 ms]` under `model-steps`.
3. Compare FR-252 keys of the dense and tick `Rpc` models and of two tick models differing only in period, and FR-255 keys of a step and a time interval.

Tag the tests `#[trace("TC-688", "FR-233-AC-n")]`.

## Expected Results

- Step 1: equal bindings and keys; `ModelSteps`; `ModelSteps`; `invalid_timed_model`/`model-time-on-untimed-model` naming `Counter`.
- Step 2: equal bounds and keys; `ill_typed`/`type-mismatch`; `ill_typed`/`type-mismatch`.
- Step 3: different keys in each comparison.
