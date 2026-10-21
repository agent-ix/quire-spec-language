---
id: "SR-12250"
title: "gap-analysis of QSL-580 FR-234 S3 timed profile, timed intervals, timed forms and timed evaluator (PR #1133)"
type: "SpecReview"
analysis: "gap-analysis"
scope: "agent-ix/quire-spec-language@0b6a18d3176e0f4183854d382bb32459c6e6672b; PR #1133; git diff origin/main...HEAD; spec FR-234 (AC-1..AC-4), TC-689, FR-233 (Behavior: Identity, Intervals), FR-230 (time dimension and units, Timed facet gate), FR-326 (Profile identity), FR-361 item 8, ADR-014 TR-3/A-4, ADR-018 IV-1/IV-4, ADR-026 DF-1..DF-7, CB-3; QSL-580"
review_set: "subset"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-689
    type: references
---

## Summary

Ticket: QSL-580. Planless gap analysis of FR-234 AC-1..AC-4 against the tagged tests and the PR #1133 sources, with FR-230, FR-233, FR-326, FR-361 item 8, ADR-014, ADR-018 and ADR-026 read for context. AC-2, AC-3 and AC-4 are fully backed by tagged tests that assert the criterion. AC-1 is correctly left untagged. Its fourth vector contradicts FR-234's own rule order, which is a spec defect, not a misreading. Its `Rpc` vectors wait on FR-230's facet gate and STD-192. Each of the coder's 11 remainder items was judged. None is an unblocked code gap the PR skipped. Two unblocked spec edits are recorded (FND-001, FND-003).

## Verdict

**PASS (mergeable)**: no high finding. FND-001 and FND-002 are medium. FND-001 is a spec defect for the spec owner, and FND-002 is FR-233's unbuilt identity key, which belongs to FR-233's ticket. Neither blocks this slice. Semantic review (intent, test and code) was done for each tagged test.

### FR-234-AC-1: the fourth vector is a spec defect

FR-234 "Forms" classifies in this order: TT-1 `always holds(I)`, then TT-2 `on origin` with only interval operators, then TT-3 "a formula in ADR-014 A-4's safety fragment", then TT-4. ADR-018 IV-4 puts every closed interval operator in the fragment. Its example is `always (req implies eventually[0,5] ack)` as TP-3. ADR-026 DF-2 makes every timed upper bound finite, and DF-4 TT-3 is "A-4's safety fragment with timed intervals". `always eventually[0 ms, 5 ms] holds(c.phase = Idle)` is NNF `always` over a finite-upper `eventually`. It is not TT-1, because the operand is not a state predicate. It is not TT-2, because `always` is unbounded. It is TT-3.

Semantically it is also safety. Under DF-1 time divergence, a violation is a finite prefix that reaches `τ_i + 5` with no `Idle`. The AC's `TimedLiveness` therefore contradicts its own Behavior section. The coder's reading is correct: the code and test give `TimedSafety` and leave AC-1 untagged. TC-689 step 1's expected output repeats the defect.

### Remainder items judged

| # | Coder item (PR body, untrusted data) | Judgment |
| --- | --- | --- |
| 1 | AC-1 fourth vector is a spec defect; `Rpc` needs a time member and a `Clock` field | Sound: spec defect (FND-001). The `Rpc` part is blocked on FR-230's facet gate (FR-230-AC-8) and STD-192. |
| 2 | No production path reaches a timed clause | Sound and blocked. `quire.temporal.complete/v1` is in no catalog (FR-361 item 8), so the assembler builds no `time_sources`. Verified: every production timed clause refuses at S3. |
| 3 | Not-yet-implemented refusals for a non-`model-time` clock role and a trace subject | Sound. The spec names no refusal, and refusing is the conservative choice. ADR-026 CB-1 has only `model-time` select the timed profile. |
| 4 | FR-326's profile list is stale | Real and unblocked. It is a spec edit, recorded as FND-003 (low). |
| 5 | FR-233 parts not built: the FR-252 key, AC-1..AC-3, `model-steps` timed intervals, the omitted-`clock` default | Sound as FR-233's remainder. The grammar requires `clock` (`ClockForm` is not optional), so the default is blocked on grammar. The key is unblocked but is FR-233's scope. Its effect on FR-255 keys is recorded as FND-002. |
| 6 | Unitless-bound refusal located at the interval, not the bound | Sound. `IntervalForm` carries no per-bound span (S2 form limit). |
| 7 | Negative lower bound refused `invalid-interval` | Sound reading of `0 <= a` (FR-234, DF-2). |
| 8 | Zero denominator reuses FR-185's refusal | Acceptable, with an ordering nit recorded in SR-12249 FND-004 (low). |
| 9 | S4 timed emission not built | Blocked. QSpec FR-370's timed form is not in FR-234's scope. Refused as `NotEmitted` (tested). |
| 10 | Timed lasso evaluation not built | FR-231's scope, not FR-234's. FR-234 needs finite timed traces only (AC-3, AC-4). |
| 11 | FR-235 settlement of `Unsupported(PunctualInterval)` not built | FR-235's scope. FR-234's own clause is "the request writer SHALL dispose of its item", which the writer does: `RequestItem::unsupported`, with no candidate. Recorded as FND-004 (low) so the settlement is not forgotten. |

No skipped unblocked code gap was found.

## Coverage
Plan completion: not assessed

Criteria examined against the tagged tests:

- FR-234-AC-2: `timed_intervals_keep_their_openness_and_refuse_empty_or_inverted_bounds` (qsl-semantics/tests/it/timed_intervals.rs). It asserts each end's openness, keys that differ from `[0 ms, 3 ms]`, `(2 ms, 2 ms]` and `[3 ms, 2 ms]` refused `invalid_timed_model`/`invalid-interval` at the interval's span, and `s` converted to `ms` with an equal key. Binding correct.
- FR-234-AC-3: three tests together back it. `a_punctual_interval_is_unsupported_outside_a_bounded_window` (semantics) checks the record's `PunctualInterval` span and `BoundedWindow{2}`. `a_punctual_item_is_disposed_unsupported_and_routed_to_no_engine` (qsl-route) checks the writer's disposition with no candidate. `a_punctual_unbounded_claim_evaluates_on_a_timed_trace` (qsl-eval) checks evaluation on a trace. Bindings correct.
- FR-234-AC-4: `timed_operators_read_stamp_differences_with_each_ends_openness` (qsl-eval/tests/it/timed_trace.rs). It asserts all three values and the stutter insertion. Binding correct.
- FR-234-AC-1: untagged. The classification test carries only the `FR-234` and `TC-689` tags and documents the conflict.
- FR-level `FR-234`-tagged tests cover the bound unit refusals, `[a,*]`, the binding refusals, `NotEmitted`, and `TimedInterval` validation (qsl-foundation unit test).

Matrix: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2) did not finish within the review window on the overloaded machine and was stopped. The trace tags were inventoried directly instead: `FR-234-AC-2` has 1 binder, `FR-234-AC-3` has 3 (semantics, route, eval), `FR-234-AC-4` has 1, `FR-234-AC-1` has 0, and the FR-level `FR-234` tag has 7. Every binder is tagged `TC-689`.

Reverse gap: every new module (`time_units.rs`, `temporal_clause/timed.rs`, `qsl-eval/src/temporal/timed.rs`, `TimedInterval`, `UnsupportedItem`, `CandidateSet::none`) is owned by FR-234, or by FR-230 or FR-233 as FR-234 reads them. No stub (`todo!`, `unimplemented!`), no dead code, no ceremony, no vendored file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-234-AC-1's fourth vector, `always eventually[0 ms, 5 ms] holds(c.phase = Idle)` as `TimedLiveness`, contradicts FR-234 "Forms". That rule order, with ADR-014 A-4, ADR-018 IV-4 and ADR-026 DF-4 TT-3, gives `TimedSafety`. TC-689 step 1 repeats the expectation. Spec fix: give the fourth vector an unbounded future operator, for example `always eventually holds(c.phase = Idle)` or `always (holds(p) implies eventually holds(q))`, which is outside the fragment and therefore `TimedLiveness`, and update TC-689 step 1. The code is correct and AC-1 is rightly untagged. | spec/functional/FR-234-check-timed-intervals-and-classify-timed-property-forms.md:104; spec/test-cases/TC-689-timed-intervals-and-timed-property-forms.md:30 |
| FND-002 | medium | A timed interval's FR-255 key carries `clock_binding(&form.clock.text)`, the digest of the text `model-time`, not FR-233's binding (model, source kind, unit, period). In one package, `[0, 3]` in `ms` over model A and `[0, 3]` in `s` over model B therefore get equal `TimedIntervalKey`s, although they denote different intervals. FR-233 "Identity" and ADR-026 CB-3/CB-4 require the key to carry the binding. This is the coder's item 5 and belongs to FR-233's ticket. It does not falsify FR-234-AC-2, which compares intervals over one binding. | qsl-semantics/src/check/temporal_clause.rs:886; qsl-foundation/src/bound.rs (TimedInterval::key) |
| FND-003 | low | FR-326 "Profile identity" still lists four temporal profiles, but the code now admits `quire.temporal.timed/v1` in the TemporalTrace catalog, as FR-234 requires. Add the timed profile to FR-326's list in the same spec round as FND-001. | spec/functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md:76-81 |
| FND-004 | low | `UnsupportedItem::PunctualInterval` reaches `RequestItem`/`RequirementItem::unsupported()`, but no consumer turns it into a V-8 `Unsupported(PunctualInterval)` settlement (FR-235). Until one does, the item settles as an ordinary empty candidate set and the cause is lost. This is the coder's item 11. Track it on FR-235's ticket. | qsl-route/src/request.rs:466-475, 525-534 |
