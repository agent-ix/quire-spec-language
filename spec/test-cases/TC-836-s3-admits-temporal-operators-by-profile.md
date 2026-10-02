---
id: TC-836
title: "S3 admits temporal operators by the unit's temporal profile and records the requirement"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: verifies
---
# TC-836: S3 admits temporal operators by the unit's temporal profile and records the requirement

## Description

Verify the S3 TemporalTrace check's profile identity rule, the bounded and
infinite-trace interval rules, the horizon and its overflow, and the
requirement record.

Scope: FR-326-AC-1 to FR-326-AC-5.

## Test Procedure

1. Check `eventually holds(c.value = 3)` under infinite-trace and under
   event-position false-extension.
2. Check `always[0,2] eventually[1,3] holds(c.value = 3)`,
   `eventually[3,*] holds(p)` and `eventually[5,3] holds(p)` under
   event-position, and the last two under infinite-trace.
3. Check `eventually[0,18446744073709551615] eventually[0,1] holds(p)` under
   event-position.
4. Check a unit with a temporal clause and no temporal profile selection,
   and one selecting two temporal profiles.
5. Check a bounded clause over `over (c: Counter)` and compare the `[0,2]`
   interval keys under event-position and fixed-sample.

Tag the tests `#[trace("TC-836", "FR-326-AC-n")]`.

## Expected Results

- Step 1: infinite-trace checks with `None`, no horizon and record
  (`temporal-satisfaction`, `Unbounded`) with one infinite-trace-formula
  domain at the clause node; event-position refuses
  `unsupported_construct`/`expression-form` at `eventually`.
- Step 2: horizon 5; `[3,*]` refuses `unsupported_construct`/
  `expression-form` under event-position and checks with `Some{3, Open}`
  under infinite-trace; `[5,3]` refuses `ill_typed`/`type-mismatch` under
  both, each at the interval's span.
- Step 3: `stage_limit_exceeded`, limit kind work budget, at the overflowing
  operator.
- Step 4: `unknown_profile`/`unsupported-selection` at the clause, naming the
  selections found.
- Step 5: record (`temporal-satisfaction`, `Bounded`); unequal keys.
