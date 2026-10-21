---
id: "SR-12249"
title: "code-review of QSL-580 FR-234 S3 timed profile, timed intervals, timed forms and timed evaluator (PR #1133)"
type: "SpecReview"
analysis: "code-review"
scope: "agent-ix/quire-spec-language@0b6a18d3176e0f4183854d382bb32459c6e6672b; PR #1133; git diff origin/main...HEAD (qsl-foundation/src/bound.rs, qsl-foundation/src/diagnostic.rs, qsl-semantics/src/check/{temporal_clause.rs,temporal_clause/timed.rs,time_units.rs,profile.rs,refusal.rs,check.rs,mod.rs,hyper_clause.rs,probabilistic.rs,probabilistic/claim.rs,lowering/temporal.rs}, qsl-semantics/src/family/{mod.rs,requirements.rs}, qsl-route/src/{lib.rs,request.rs}, qsl-eval/src/temporal/{timed.rs,engine.rs,mod.rs,outcome.rs,plan.rs}, qsl-eval/src/automaton/{formula.rs,translate.rs}, qsl-replay/src/spine.rs, qsl-replay/src/spine/clause/temporal.rs, tests); spec FR-230, FR-233, FR-234, FR-361 item 8, ADR-014 TR-3/A-4, ADR-018 IV-1/IV-4, ADR-026 DF-1..DF-7, TC-689; QSL-580"
review_set: "subset"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-233
    type: references
---

## Summary

Ticket: QSL-580. Code review with the rust-review lane folded in, of PR #1133: the FR-234 S3 slice (timed profile in the TemporalTrace catalog, timed interval check and FR-230 time-unit conversion, TT-1..TT-4 classification, punctual-interval disposition in the request writer, `CheckedInterval::{Steps, Timed}`) and the qsl-eval timed reference evaluator. Soundness was checked first and holds. No high finding. One medium (the punctual short-circuit in `candidates_of` hides an unknown named backend) and four lows.

## Verdict

**PASS (mergeable)**: no high finding. FND-001 is medium and FND-002..FND-005 are low; none blocks.

Soundness, examined with no defect:

- **Timed evaluator** (`qsl-eval/src/temporal/timed.rs`). `future` stops at the first position whose offset the window cannot reach. Stamps never decrease, so every later position, represented or not, is past the window too. `contains` and `reaches` read each end's openness with exact `Rational` comparison. The `until` guard covers `i <= k < j` because `held` is updated after the witness at `j` is read. `since` covers `j < k <= i` the same way in reverse. `release` and `triggered` are the duals `¬(¬p U ¬q)` and `¬(¬p S ¬q)`, with witness `right` and guard `left`, both flipped. A finite trace is read as a prefix. After the represented positions, the continuation adds `Unknown` exactly when the window still reaches `last − τ_i`. That is right, because a later position may repeat the last stamp. So `[a, b]` with `last − τ_i = b` stays undecided. Kleene min/max are monotone, so `Unknown` never becomes `True` or `False`. A `False` result is false on every extension, so `Violated` is sound. Past operators read only represented positions and nothing before position 0. No floating point appears anywhere. A punctual `[2, 2]` is read as exact equality. All AC-4 and AC-3 vectors were traced by hand and agree.
- **Bound conversion** (`check/time_units.rs`). `magnitude × scale(from) × 1/scale(to)` is exact. quire-semantic-value admits one root per dimension node (`unit.rs` root count check), so two time units always share a root, and the composed scales compare like with like. Nothing is rounded, so no interval is widened or narrowed. `0 s, 3/1000 s` gives the same `TimedInterval` and key as `0 ms, 3 ms` (tested). An affine unit, a unit of another dimension, an unknown unit and a missing `Time` base each refuse, in FR-230's order.
- **TimedInterval** (`qsl-foundation/src/bound.rs`). The only constructor refuses `a < 0`, `a > b` and an open end with `a = b`. That makes `is_punctual()`'s `lower == upper` test equivalent to "both ends closed". The key holds the interval with its openness, so `[0,3)` and `[0,3]` differ.
- **Classification** (`temporal_clause/timed.rs::property_form`). The rules run in FR-234's order: TT-1 (unbounded `always` over a `holds` atom), then TT-2 (`on origin` with every temporal node carrying a timed interval, horizon the sum of upper bounds), then TT-3 (`in_safety_fragment`, where a timed interval counts as closed per DF-2 and IV-4), then TT-4. `always eventually[0 ms, 5 ms] p` classifies `TimedSafety`. That is correct by the rule order and by DF-4 TT-3 with IV-4. The conflict with FR-234-AC-1 is a spec defect, recorded in SR-12250 FND-001.
- **No silent step reading of a timed clause.** The package check refuses a checked timed clause as `NotEmitted` (`is_emitted` tests `time_binding`; tested). `lowering::temporal::step_interval` faults on `Timed`. The step evaluator (`engine.rs` Shape) and the automaton translator (`translate_formula`) refuse the profile. `plan::Window::of` and `automaton::formula::Window::of` fault on a timed interval. `check_clauses` has no production caller.
- **Making timed/v1 selectable lets nothing through.** `quire.temporal.complete/v1` is still in no catalog (FR-230-AC-8, FR-361 item 8). So the assembler builds no `time_sources`, and every production timed clause refuses at S3: `model-time-on-untimed-model`, `TimedProfileWithoutModelTime`, or `TimedTraceSubject`. Other readers of the profile under timed/v1 also refuse: hyper clauses (`not_emitted`), probabilistic events (`UnboundedEvent`, FND-003), refinements (`WrongProfile`) and the model-check path (InfiniteTrace only). FND-002 and FND-003 concern diagnostics only.
- **Punctuality.** `unsupported_punctual` is set only under the timed profile and only for TT-1, TT-3 and TT-4. A `BoundedWindow` admits punctual intervals.
- **Panics, unsafe, compat layer.** No `unwrap`, `expect`, `panic!` or indexing in the added production code. Position conversions use `try_from` with a fault or `unwrap_or`. No `unsafe`. No compatibility layer, shim or fallback.
- **FR-100-AC-8.** The `qsl-replay` change matches `qsl_eval::TemporalRefusal` arms inside the private `trace_refusal`. No public `spine` item and no re-export names a `qsl_eval` path.
- **Rust idioms.** `CheckedInterval` replaced `Copy` with `Clone`, and every call site clones explicitly. The single `Time` spelling read is marked `#[qsl_attrs::string_edge]`. The `TimeQuantity` cause is boxed. Each new match over `FormulaNode` and `TemporalProfile` is exhaustive, with no wildcard.

## Validation

No gates and no full build, as briefed (the machine is overloaded). The review is code reading plus one single-suite run through `~/dev/worktrees/locked-build.sh`: `cargo test -p qsl-eval --test it timed_trace` exited 0 (4 passed: the AC-3 and AC-4 vectors, until/since/duals, and the trace and evaluator refusals). `target/` was deleted afterwards. The matrix is in SR-12250.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `candidates_of` returns `Candidates(CandidateSet::none(kind, named))` for an item with an `unsupported` cause before it consults the registry. A request that names a `BackendId` the registry does not hold therefore gets an empty candidate set carrying that unknown id, instead of `CandidateOutcome::UnknownBackend`. FR-075-AC-3 (`CandidateOutcome` doc) says that case is never reported as, or confused with, an empty set. Fix: check `named` against the registry first, or return `UnknownBackend` before the short-circuit, and add the named-unknown case to `a_punctual_item_is_disposed_unsupported_and_routed_to_no_engine`. | qsl-route/src/request.rs:525-534; qsl-route/src/lib.rs:620-631 |
| FND-002 | low | `check_behaviours` reads a hyper body with `IntervalReading::Steps { bounded: false }` whatever the selected profile. Under timed/v1 a hyper clause admits step intervals as steps, and a timed interval refuses `TimedIntervalNotChecked`. FR-234 refuses a step interval under the timed profile `ill_typed`/`type-mismatch`. The clause is still refused as `not_emitted`, so nothing leaks, but the diagnostic is wrong. Refuse a hyper clause under the timed profile, or route it through the timed reading. | qsl-semantics/src/check/hyper_clause.rs:926-931 |
| FND-003 | low | A probabilistic temporal event under timed/v1 refuses `UnboundedEvent`, because `is_bounded` is false for `Timed`. The cause says the event is unbounded when the actual problem is that a timed profile is not a probabilistic event profile. Nothing leaks. A profile-specific refusal would name it correctly. | qsl-semantics/src/check/probabilistic/claim.rs:1046-1052, 1106-1117 |
| FND-004 | low | `rational(n, 0)` in a timed bound returns `IntervalFailure::Stop` with a `ProbabilisticCause::ZeroDenominator` refusal. It skips the clause's earliest-span selection (`min_by_key(span.start)`), so a zero denominator in a later interval wins over an earlier `invalid-interval`. The cause is also filed under the probabilistic family in a temporal clause. Push it as a temporal cause into `refusals` (code `undefined_expression`/`unproved-nonzero` is fine). | qsl-semantics/src/check/temporal_clause/timed.rs:168-179 |
| FND-005 | low | `TimedPropertyForm::BoundedWindow` is assigned to an `on origin` formula with no temporal operator (for example `holds(p)`), because `all` over an empty iterator is true. The horizon is then 0. This matches FR-234's literal wording and is semantically harmless, but it is untested. Pin it in a test or exclude it. | qsl-semantics/src/check/temporal_clause/timed.rs:247-259 |
