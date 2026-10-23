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

Scope: FR-230-AC-1 to FR-230-AC-5 and FR-230-AC-7 to FR-230-AC-17. Steps 1 to 6 and 9 to 15 need a header that can select `quire.temporal.complete/v1` and wait for it. Steps 7 and 8 run now.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`, whose object type `Call` comes from the admitted domain package `acme/rpc` with the field `x` typed `ix://quire/native/Clock` (filament-core-data FR-146); its `s`-unit variant; a tick variant; the refusal fixtures of FR-230-AC-2, each clock a package-admitted clock field; a 64-clock model; the domain package `acme/rpc` with `dimensions` `[Time]` and `units` `[s, ms]` (`ms` targeting `s` at scale `1/1000`); its variant `acme/rpc` with `dimensions` `[Time, Rate]` (`Rate` a base dimension with no unit); and a second domain package `acme/flow` with `dimensions` `[Time]` and `units` `[sec]`, selected beside `acme/rpc` after it in lock order.

1. Check `Rpc` and its `time dense unit s` variant.
2. Check each refusal fixture of FR-230-AC-2.
3. Check the tick model with period `1/2 ms`, with period `0 ms`, and with an unresolved tick operation.
4. Check the disjunctive guard and the 64-clock model with the `10^40 ms` constant.
5. Check, with units declared in the unit's own source, `time dense unit ms` and `unit s`, the quantity `3 ms` under `unit s`, `unit hour`, a graph with no base `Time`, two dimensions named `Time` in the source, a unit of `Duration = Time`, a time unit with a nonzero offset, and two graphs declaring `ms` differently.
6. Check, with `Time`, `s` and `ms` supplied by `acme/rpc` and none in the source, `time dense unit M::ms`, `time dense unit M::s` and `3 M::ms` under `unit M::s`; then a source that also declares `unit ms : Time = 1/1000 * M::s;` and `time dense unit ms`; then a source that also declares `dimension Time;`; then, over the `Rate` variant of `acme/rpc`, a source that declares `dimension Rate;`; then `acme/rpc` and `acme/flow` both selected, with no source `Time`.
7. Check a unit that selects no `quire.temporal.complete/v1` and writes in its own source `time dense unit ms;`, a `time invariant`, an `urgent when P;` declaration, `f(c: Clock)`, `g(): Clock` and a record with a `Clock` field, `type Alias = Clock;` and `Option<Clock>` (FR-230-AC-7).
8. Check a unit whose header selects `quire.temporal.complete/v1` (FR-230-AC-8).

9. Check the `within` predicate of FR-230-AC-9 in a precondition, a time invariant body, a conjunction, with `self.limit` as the bound, with `5` as the clock, with a body that assigns the clock and one that resets it, and in an `invariant` clause.
10. Check `pick` and `peer_clock` of FR-230-AC-10 in guards, and `g(): Clock { 0 }`.
11. Check the `Timer` record, the tuple field and the three-object universe of FR-230-AC-11, including the whole read `self.u = self.t`, the `let` literal and the checked `Timer` node.
12. Check the `Alias`, `Option<Clock>` and union-case fields of FR-230-AC-12 with the three guards, the two writes and the time invariant.
13. Check `reset(c: Clock)` with a frame naming `Call.x` and with an empty frame, and `check(c: Clock)` (FR-230-AC-13).
14. Check the collection, `time tick`, untimed and multiplicity `[0, 3]` fields of FR-230-AC-14 and the unit declarations that raise no refusal.
15. Check the `Quantity<ms>` positions of FR-230-AC-15, the domain package of FR-230-AC-16 and the unit positions of FR-230-AC-17, each with its refused variants.

Tag the tests `#[trace("TC-685", "FR-230-AC-n")]`.

## Expected Results

- Step 1: `TimeSource::Dense{ms}`, clock `Call.x`, two time invariants, `send`'s reset, `reply`'s one convex part `x >= 1`; the `s` variant has bound `1/1000` and a different package identity.
- Step 2: each refuses with the code and span FR-230-AC-2 names.
- Step 3: `TimeSource::Tick` with period `1/2`; `invalid_model_binding`/`malformed-declaration`; `missing_declaration`/`missing-name`.
- Step 4: two convex parts each with `self.ready`; the large model checks with no refusal.
- Step 5: both models check and `3 ms` converts to `3/1000`; `missing_declaration`/`missing-name` naming `hour`; `missing_declaration`/`missing-name` naming `Time`; `ambiguous_declaration`/`ambiguous-name` at assembly; `ill_typed`/`type-mismatch` for the `Duration` unit; `ill_typed`/`type-mismatch`; different package identities.
- Step 6: both package-unit models check and `3 M::ms` converts to `3/1000`; the source `ms` model checks over the source unit with no ambiguity; the source `Time` and the source `Rate` each refuse `conflicting_declaration`/`duplicate-declaration` at the source declaration's span, naming both declarations; the two-package selection refuses `conflicting_declaration`/`duplicate-declaration` at `/lock/model_selections/1`, naming both `Time` declarations (FR-230-AC-5).
- Step 7: each construct refused `unsupported_construct`/`declaration-form` at its span in source order, retaining family, owning declaration, selected layers and the required facet, with no other refusal.
- Step 8: `unknown_profile`/`unsupported-selection` at E3 and no S3 result.
- Step 9: the expansions of FR-230-AC-9 with no `within` node in the checked package; `clock-bound-not-constant`, `ill_typed`/`type-mismatch`, `clock-read-outside-constraint`, `clock-write` and `clock-read-outside-constraint` at the call, each at its span.
- Step 10: two convex parts with `self.l` and `not self.l`; two parts with `self.peer = c1` and `self.peer = c2`; `ill_typed`/`type-mismatch` at the body.
- Step 11: three clocks `Call.t.x`; the guard, the reset and the data write; `clock-read-outside-constraint` at `self.t`; the data write; `clock-write`; the place `Call.p.0`; a `Timer` node listing `n` only.
- Step 12: `w` a clock; the guard with `present(self.v)`; `undefined_expression`/`unproved-presence`; the reset that makes `v` present; the drop; the guarded invariant bound; the guarded place `Call.state.Armed` and a payload-free case.
- Step 13: one instance per place of `Call.x`, naming it; `invalid_timed_model`/`clock-write`; one instance per clock place.
- Step 14: `clock-in-collection` for `s`, `q` and the `[0, 3]` field; `clock-without-dense-time` for the tick and untimed fields; no refusal for the declarations.
- Step 15: every position checks and is typed by the one unit node; `ill_typed`/`type-mismatch`; `missing_declaration`/`missing-name`; the package clocks of FR-230-AC-16; the `Quantity<M::ms>` positions, and `missing_declaration`/`missing-name` naming each member path.
