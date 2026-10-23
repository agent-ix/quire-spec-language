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

Scope: FR-230-AC-1 to FR-230-AC-5, FR-230-AC-7, FR-230-AC-8 and FR-230-AC-18. Steps 1 to 7 need a header that can select `quire.temporal.complete/v1` and wait for it. Steps 8 and 9 run now.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`, whose object type `Call` comes from the admitted domain package `acme/rpc` with the field `x` typed `ix://quire/native/Clock` (filament-core-data FR-146); its `s`-unit variant; a tick variant; the refusal fixtures of FR-230-AC-2, each clock a package-admitted clock field; a 64-clock model; the domain package `acme/rpc` with `dimensions` `[Time]` and `units` `[s, ms]` (`ms` targeting `s` at scale `1/1000`); its variant `acme/rpc` with `dimensions` `[Time, Rate]` (`Rate` a base dimension with no unit); and a second domain package `acme/flow` with `dimensions` `[Time]` and `units` `[sec]`, selected beside `acme/rpc` after it in lock order.

1. Check `Rpc` and its `time dense unit s` variant.
2. Check each refusal fixture of FR-230-AC-2.
3. Check the tick model with period `1/2 ms`, with period `0 ms`, and with an unresolved tick operation.
4. Check the disjunctive guard and the 64-clock model with the `10^40 ms` constant.
5. Check, with units declared in the unit's own source, `time dense unit ms` and `unit s`, the quantity `3 ms` under `unit s`, `unit hour`, a graph with no base `Time`, two dimensions named `Time` in the source, a unit of `Duration = Time`, a time unit with a nonzero offset, and two graphs declaring `ms` differently.
6. Check, with `Time`, `s` and `ms` supplied by `acme/rpc` and none in the source, `time dense unit M::ms`, `time dense unit M::s` and `3 M::ms` under `unit M::s`; then a source that also declares `unit ms : Time = 1/1000 * M::s;` and `time dense unit ms`; then a source that also declares `dimension Time;`; then, over the `Rate` variant of `acme/rpc`, a source that declares `dimension Rate;`; then `acme/rpc` and `acme/flow` both selected, with no source `Time`.
7. With `acme/rpc` (alias `R`) and `acme/flow` (alias `F`) both selected and a source that declares `dimension Time; unit s : Time = 1;`, check `time dense unit R::ms`, `time dense unit F::sec` and `time dense unit s`; under `time dense unit R::ms` check `x <= 3 R::ms`, `x <= 3 F::sec` and `x <= 3 s`; check `time tick O period 1/2 R::s unit R::ms` and `time tick O period 1 F::sec unit R::ms`; check `time dense unit len` and a unit of `Duration = R::Time` (FR-230-AC-18).
8. Check a unit that selects no `quire.temporal.complete/v1` and writes in its own source `time dense unit ms;`, a `time invariant`, an `urgent when P;` declaration, `f(c: Clock)`, `g(): Clock` and a record with a `Clock` field, `type Alias = Clock;` and `Option<Clock>` (FR-230-AC-7).
9. Check a unit whose header selects `quire.temporal.complete/v1` (FR-230-AC-8).

Tag the tests `#[trace("TC-685", "FR-230-AC-n")]`.

## Expected Results

- Step 1: `TimeSource::Dense{ms}`, clock `Call.x`, two time invariants, `send`'s reset, `reply`'s one convex part `x >= 1`; the `s` variant has bound `1/1000` and a different package identity.
- Step 2: each refuses with the code and span FR-230-AC-2 names.
- Step 3: `TimeSource::Tick` with period `1/2`; `invalid_model_binding`/`malformed-declaration`; `missing_declaration`/`missing-name`.
- Step 4: two convex parts each with `self.ready`; the large model checks with no refusal.
- Step 5: both models check and `3 ms` converts to `3/1000`; `missing_declaration`/`missing-name` naming `hour`; `missing_declaration`/`missing-name` naming `Time`; `ambiguous_declaration`/`ambiguous-name` at assembly; `ill_typed`/`type-mismatch` for the `Duration` unit; `ill_typed`/`type-mismatch`; different package identities.
- Step 6: both package-unit models check and `3 M::ms` converts to `3/1000`; the source `ms` model checks over the source unit with no ambiguity; the source `Time`, the source `Rate` and the two-package selection each check with no refusal, the declarations being distinct dimensions (FR-230-AC-5).
- Step 7: the three `time` members check with three different package identities; `x <= 3 F::sec` and `x <= 3 s` under `R::ms` and the `1 F::sec` period refuse `ill_typed`/`type-mismatch` at the constant and the period; `3 R::ms` and `1/2 R::s` check; `len` and the `Duration` unit refuse `ill_typed`/`type-mismatch` (FR-230-AC-18).
- Step 8: each construct refused `unsupported_construct`/`declaration-form` at its span in source order, retaining family, owning declaration, selected layers and the required facet, with no other refusal.
- Step 9: `unknown_profile`/`unsupported-selection` at E3 and no S3 result.
