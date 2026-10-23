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

Scope: FR-230-AC-1 to FR-230-AC-5 and FR-230-AC-7 to FR-230-AC-28. Every unit of steps 1 to 7 and 10 to 25 carries the header `profile t = "quire.temporal.complete/v1";`; step 8 carries none, and step 9 checks the header unit beside the unit that selects `quire.model.complete/v1` alone.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`, whose object type `Call` comes from the admitted domain package `acme/rpc` with the field `x` typed `ix://quire/native/Clock` (filament-core-data FR-146); its `s`-unit variant; a tick variant; the refusal fixtures of FR-230-AC-2, each clock a package-admitted clock field; a 64-clock model; the domain package `acme/rpc` with `dimensions` `[Time]` and `units` `[s, ms]` (`ms` targeting `s` at scale `1/1000`); its variant `acme/rpc` with `dimensions` `[Time, Rate]` (`Rate` a base dimension with no unit); and a second domain package `acme/flow` with `dimensions` `[Time]` and `units` `[sec]`, selected beside `acme/rpc` after it in lock order.

For each input in steps 1 to 5, read its S2 forms before the S3 check. Assert the forms, spans and absence of timed semantic refusals stated in FR-230-AC-1 to AC-5; then assert the S3 result below.

1. Check `Rpc` and its `time dense unit s` variant.
2. Check each refusal fixture of FR-230-AC-2.
3. Check the tick model with period `1/2 ms`, with period `0 ms`, and with an unresolved tick operation.
4. Check the disjunctive guard and the 64-clock model with the `10^40 ms` constant.
5. Check, with units declared in the unit's own source, `time dense unit ms` and `unit s`, the quantity `3 ms` under `unit s`, `unit hour`, a graph with no base `Time`, two dimensions named `Time` in the source, a unit of `Duration = Time`, a time unit with a nonzero offset, and two graphs declaring `ms` differently.
6. Check, with `Time`, `s` and `ms` supplied by `acme/rpc` and none in the source, `time dense unit M::ms`, `time dense unit M::s` and `3 M::ms` under `unit M::s`; then a source that also declares `unit ms : Time = 1/1000 * M::s;` and `time dense unit ms`; then a source that also declares `dimension Time;`; then, over the `Rate` variant of `acme/rpc`, a source that declares `dimension Rate;`; then `acme/rpc` and `acme/flow` both selected, with no source `Time`.
7. With `acme/rpc` (alias `R`) and `acme/flow` (alias `F`) both selected and a source that declares `dimension Time; unit s : Time = 1;`, check `time dense unit R::ms`, `time dense unit F::sec` and `time dense unit s`; under `time dense unit R::ms` check `x <= 3 R::ms`, `x <= 3 F::sec` and `x <= 3 s`; check `time tick O period 1/2 R::s unit R::ms` and `time tick O period 1 F::sec unit R::ms`; check `time dense unit len` and a unit of `Duration = R::Time` (FR-230-AC-18).
8. Check a unit that selects no `quire.temporal.complete/v1` and writes in its own source `time dense unit ms;`, a `time invariant`, an `urgent when P;` declaration, `f(c: Clock)`, `g(): Clock` and a record with a `Clock` field, `type Alias = Clock;` and `Option<Clock>` (FR-230-AC-7).
9. Check a unit whose header selects `quire.temporal.complete/v1` and writes `time dense unit ms;`, a `time invariant`, an `urgent when P;` declaration and a `Clock` field of an object type of the admitted domain package; read its emitted `definition_selections`. Then check the same unit with the header `profile t = "quire.model.complete/v1";` alone (FR-230-AC-7, FR-230-AC-8).
10. Over step 7's packages and source with `dimension R::Time ~ F::Time by 1 R::s = 1 F::sec;` declared, check `time dense unit R::ms` with `x <= 3 F::sec`, with a reset of `x` to `2 F::sec` and with `x <= 3 s`; check `time tick O period 1 F::sec unit R::ms`; check the `len` and `Duration = R::Time` units again; compare the package identities with those of step 7 and of the same models without the equivalence; then with `acme/clock` (alias `G`, root `msec`) check `x <= 3 G::msec`, a reset to `2000 G::msec` and `time tick O period 1 G::sec unit R::ms` under `by 1 R::s = 1000 G::msec` and under `by 1 R::s = 1 G::msec`.
11. Check the `within` predicate of FR-230-AC-9 in a precondition, a time invariant body, a conjunction, with `self.limit` as the bound, with `5` as the clock, with a body that assigns the clock and one that resets it, and in an `invariant` clause.
12. Check `pick` and `peer_clock` of FR-230-AC-10 in guards, and `g(): Clock { 0 }`.
13. Check the `Timer` record, the tuple field and the three-object universe of FR-230-AC-11, including the whole read `self.u = self.t`, the `let` literal and the checked `Timer` node.
14. Check the `Alias`, `Option<Clock>` and union-case fields of FR-230-AC-12 with the three guards, the two writes and the time invariant.
15. Check `reset(c: Clock)` with a frame naming `Call.x` and with an empty frame, and `check(c: Clock)` (FR-230-AC-13).
16. Check the `time tick` and untimed fields `t: Timer` and `ts: Sequence<Timer>` of FR-230-AC-14 and the unit declarations that raise no refusal.
17. Check the `Quantity<ms>` positions of FR-230-AC-15, the domain package of FR-230-AC-16 and the unit positions of FR-230-AC-17, each with its refused variants.
18. Check the clock collections, the static dimension and the `Map<Timer, Int>` key of FR-230-AC-19.
19. Check the collection-valued writes and their refusals of FR-230-AC-20.
20. Check the selectors, quantified reads, resets by selection, time invariant bodies and refused queries of FR-230-AC-21.
21. Check the `Option<Clock>` positions of FR-230-AC-22.
22. Check the package clock collections of FR-230-AC-23, then the same package with the maximum changed.
23. Check the place references, `ref` and `deref` terms, resets through a reference, comparisons and state keys of FR-230-AC-24.
24. Check the clock constraints of FR-230-AC-25 under `time dense`, `time tick` and an untimed unit.
25. Check the unit constraints and the unit `reference` target of FR-230-AC-26.

Tag the tests `#[trace("TC-685", "FR-230-AC-n")]`.

## Expected Results

- Step 1: S2 returns the dense source, two invariants, reset and guard with spans and no timed semantic refusal. S3 gives `TimeSource::Dense{ms}`, clock `Call.x`, the two time invariants, `send`'s reset and `reply`'s one convex part `x >= 1`; the `s` variant has bound `1/1000` and a different package identity.
- Step 2: S2 builds each grammar-valid offending form and returns no timed semantic refusal; S3 refuses each with the code and span FR-230-AC-2 names.
- Step 3: S2 returns a tick source form with period `1/2 ms`, a zero-period form and the unresolved operation name, all with spans and no timed semantic refusal. S3 checks `TimeSource::Tick` with period `1/2`; `invalid_model_binding`/`malformed-declaration` at zero; `missing_declaration`/`missing-name` at the unresolved operation.
- Step 4: S2 preserves the guard tree `(x < 1 or x > 2) and self.ready` and exact `10^40 ms` bound with no timed semantic refusal. S3 gives two convex parts each with `self.ready`; the large model has 6,400 clock places and checks with no refusal.
- Step 5: S2 preserves all dense-unit and quantity forms and names/spans without unit or dimension resolution. S3 checks both source-unit models and `3 ms` converts to `3/1000`; `missing_declaration`/`missing-name` names `hour`; `missing_declaration`/`missing-name` names `Time`; source duplicate `Time` gives `ambiguous_declaration`/`ambiguous-name` at assembly; the `Duration` unit and offset unit give `ill_typed`/`type-mismatch`; the two `ms` scale graphs have different package identities.
- Step 6: both package-unit models check and `3 M::ms` converts to `3/1000`; the source `ms` model checks over the source unit with no ambiguity; the source `Time`, the source `Rate` and the two-package selection each check with no refusal, the declarations being distinct dimensions (FR-230-AC-5).
- Step 7: the three `time` members check with three different package identities; `x <= 3 F::sec` and `x <= 3 s` under `R::ms` and the `1 F::sec` period refuse `ill_typed`/`type-mismatch` at the constant and the period; `3 R::ms` and `1/2 R::s` check; `len` and the `Duration` unit refuse `ill_typed`/`type-mismatch` (FR-230-AC-18).
- Step 8: each construct refused `unsupported_construct`/`declaration-form` at its span in source order, retaining family, owning declaration, selected layers and the required facet, with no other refusal.
- Step 9: the first unit resolves at E3, S3 raises no `unsupported_construct` and produces a checked state model with the `TimedModel`, and `definition_selections` hold the six layers of FR-110-AC-13; the second unit refuses each timed construct `unsupported_construct`/`declaration-form`.
- Step 10: `x <= 3 F::sec` checks with the constant `3000` in `R::ms`, the reset converts to `2000` and the tick period is `1000`; `x <= 3 s`, the `len` unit and the `Duration` unit refuse `ill_typed`/`type-mismatch`; the three models have three different package identities, each different from the same model without the equivalence (FR-230-AC-27); under ratio 1000 the `acme/clock` constant is `3`, the reset `2000` and the period `1000`, under ratio 1 the constant is `3000`, and the two declarations give two model identities (FR-230-AC-28).
- Step 11: the expansions of FR-230-AC-9 with no `within` node in the checked package; `clock-bound-not-constant`, `ill_typed`/`type-mismatch`, `clock-read-outside-constraint`, `clock-write` and `clock-read-outside-constraint` at the call, each at its span.
- Step 12: two convex parts with `self.l` and `not self.l`; two parts with `self.peer = c1` and `self.peer = c2`; `ill_typed`/`type-mismatch` at the body.
- Step 13: three clocks `Call.t.x`; the guard, the reset and the data write; `clock-read-outside-constraint` at `self.t`; the data write; `clock-write`; the place `Call.p.0`; a `Timer` node listing `n` only.
- Step 14: `w` a clock; the guard with `present(self.v)`; `undefined_expression`/`unproved-presence`; the reset that makes `v` present; the drop; the guarded invariant bound; the guarded place `Call.state.Armed` and a payload-free case.
- Step 15: one instance per place of `Call.x`, naming it; `invalid_timed_model`/`clock-write`; one instance per clock place.
- Step 16: `clock-without-dense-time` for the tick and untimed fields; no refusal for the declarations.
- Step 17: every position checks and is typed by the one unit node; `ill_typed`/`type-mismatch`; `missing_declaration`/`missing-name`; the package clocks of FR-230-AC-16; the `Quantity<M::ms>` positions, and `missing_declaration`/`missing-name` naming each member path.
- Step 18: the places and shapes, the dimension `11` for one object with `s` adding none, `s` with at most one element, `ts` typed over the `Timer` data node, and `ill_typed`/`operator-ineligible` at the map key.
- Step 19: the provenance records of AC-20 and `clock-write` at each refused occurrence and position.
- Step 20: the resets, the guard entries by size, the convex parts for `exists`, `unproved-presence` at `1`, `time-invariant-shape` for the `exists` body and `clock-read-outside-constraint` for `=`, `contains` and `count`, with `size` and `e.n` admitted.
- Step 21: each `Option<Clock>` position as stated, with the instances of `arm`, the place expressions of `maybe` and the refusals.
- Step 22: the clock collections of the package with dimension `5` for one `Call`, and a different package identity under the changed maximum.
- Step 23: the four guarded convex parts, the data write of `ref(self.x)`, `clock-write` at the reset outside the frame, the data comparison, `unproved-presence` without and no refusal with `present(self.r)`, equal state keys for equal data and different keys for different `r`, and slot `0` kept across a reorder.
- Step 24: the `x <= 3` invariants on the typed places, the bound on every clock, `time-invariant-shape` for `min`, `malformed-declaration` for the other keywords and operands, and `clock-without-dense-time` under `time tick` and untimed units.
- Step 25: `5 ms` checks, `11 ms` refuses `ill_typed`/`type-mismatch`, and `malformed-declaration` for `minLength` and for the unit `reference` target.
