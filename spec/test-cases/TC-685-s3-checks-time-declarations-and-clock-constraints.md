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

Scope: FR-230-AC-1 to FR-230-AC-5 and FR-230-AC-7 to FR-230-AC-23. Every unit of steps 1 to 7 and 10 to 21 carries the header `profile t = "quire.temporal.complete/v1";`; step 8 carries none, and step 9 checks the header unit beside the unit that selects `quire.model.complete/v1` alone.

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
9. Check a unit whose header selects `quire.temporal.complete/v1` and writes `time dense unit ms;`, a `time invariant`, an `urgent when P;` declaration and a `Clock` field of an object type of the admitted domain package; read its emitted `definition_selections`. Then check the same unit with the header `profile t = "quire.model.complete/v1";` alone (FR-230-AC-7, FR-230-AC-8).
10. Check the `within` predicate of FR-230-AC-9 in a precondition, a time invariant body, a conjunction, with `self.limit` as the bound, with `5` as the clock, with a body that assigns the clock and one that resets it, and in an `invariant` clause.
11. Check `pick` and `peer_clock` of FR-230-AC-10 in guards, and `g(): Clock { 0 }`.
12. Check the `Timer` record, the tuple field and the three-object universe of FR-230-AC-11, including the whole read `self.u = self.t`, the `let` literal and the checked `Timer` node.
13. Check the `Alias`, `Option<Clock>` and union-case fields of FR-230-AC-12 with the three guards, the two writes and the time invariant.
14. Check `reset(c: Clock)` with a frame naming `Call.x` and with an empty frame, and `check(c: Clock)` (FR-230-AC-13).
15. Check the `time tick` and untimed fields `t: Timer` and `ts: Sequence<Timer>` of FR-230-AC-14 and the unit declarations that raise no refusal.
16. Check the `Quantity<ms>` positions of FR-230-AC-15, the domain package of FR-230-AC-16 and the unit positions of FR-230-AC-17, each with its refused variants.
17. Check the clock collections, the static dimension and the `Map<Timer, Int>` key of FR-230-AC-19.
18. Check the collection-valued writes and their refusals of FR-230-AC-20.
19. Check the selectors, quantified reads, resets by selection, time invariant bodies and refused queries of FR-230-AC-21.
20. Check the `Option<Clock>` positions of FR-230-AC-22.
21. Check the package clock collections of FR-230-AC-23, then the same package with the maximum changed.

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
- Step 9: the first unit resolves at E3, S3 raises no `unsupported_construct` and produces a checked state model with the `TimedModel`, and `definition_selections` hold the six layers of FR-110-AC-13; the second unit refuses each timed construct `unsupported_construct`/`declaration-form`.
- Step 10: the expansions of FR-230-AC-9 with no `within` node in the checked package; `clock-bound-not-constant`, `ill_typed`/`type-mismatch`, `clock-read-outside-constraint`, `clock-write` and `clock-read-outside-constraint` at the call, each at its span.
- Step 11: two convex parts with `self.l` and `not self.l`; two parts with `self.peer = c1` and `self.peer = c2`; `ill_typed`/`type-mismatch` at the body.
- Step 12: three clocks `Call.t.x`; the guard, the reset and the data write; `clock-read-outside-constraint` at `self.t`; the data write; `clock-write`; the place `Call.p.0`; a `Timer` node listing `n` only.
- Step 13: `w` a clock; the guard with `present(self.v)`; `undefined_expression`/`unproved-presence`; the reset that makes `v` present; the drop; the guarded invariant bound; the guarded place `Call.state.Armed` and a payload-free case.
- Step 14: one instance per place of `Call.x`, naming it; `invalid_timed_model`/`clock-write`; one instance per clock place.
- Step 15: `clock-without-dense-time` for the tick and untimed fields; no refusal for the declarations.
- Step 16: every position checks and is typed by the one unit node; `ill_typed`/`type-mismatch`; `missing_declaration`/`missing-name`; the package clocks of FR-230-AC-16; the `Quantity<M::ms>` positions, and `missing_declaration`/`missing-name` naming each member path.
- Step 17: the places and shapes, the dimension `11` for one object with `s` adding none, `s` with at most one element, `ts` typed over the `Timer` data node, and `ill_typed`/`operator-ineligible` at the map key.
- Step 18: the provenance records of AC-20 and `clock-write` at each refused occurrence and position.
- Step 19: the resets, the guard entries by size, the convex parts for `exists`, `unproved-presence` at `1`, `time-invariant-shape` for the `exists` body and `clock-read-outside-constraint` for `=`, `contains` and `count`, with `size` and `e.n` admitted.
- Step 20: each `Option<Clock>` position as stated, with the instances of `arm`, the place expressions of `maybe` and the refusals.
- Step 21: the clock collections of the package with dimension `5` for one `Call`, and a different package identity under the changed maximum.
