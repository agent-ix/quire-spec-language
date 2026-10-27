---
id: SR-1631002
title: "gap analysis of PR #1631 (QSL-581c, FR-242 TT-2 BoundedWindow claim automaton)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1631 git diff origin/qsl-581b-zone-certificate...edd7fa765; FR-242 AC-1 to AC-3; qsl-analyze/tests/it/zone_search.rs; STD-493 comment 5b5714c3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-242
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-697
    type: reviews
---
# SR-1631002: gap analysis of PR #1631 (QSL-581c, FR-242 TT-2 BoundedWindow claim automaton)

## Summary

Ticket: QSL-581. Head reviewed: edd7fa765. Method: a manual AC-to-test check against the diff, scoped to FR-242 and TC-697. I also re-measured the spec-gap claims in Linear. Plan completion: not assessed.

| AC | PR claim | Measured |
| --- | --- | --- |
| FR-242-AC-1 | partial: 3 of 4 formulas, TT-3 deferred to QSL-583 | Partial, as claimed. `the_window_automaton_agrees_with_the_evaluator_on_random_lassos` runs the three TT-2 formulas on 1,000 lassos each (stamps in 1/4 ms steps, length at most 8, unrolled past the window), panics on any disagreement, and requires both verdicts to occur. The deferral is sound: the fourth formula has an unbounded `always` root, so it is not `BoundedWindow`, and QSL-583 states "Implements: FR-237, FR-240, FR-241, FR-242" and owns MightyL. The test is correctly left untagged for AC-1. |
| FR-242-AC-2 | covered | Covered. `a_punctual_window_is_decided_by_a_horizon_clock`: `go` forced at exactly 2 ms gives `Holds` with `automaton_states > 0`; at 5/2 ms it gives `Violated`, with `go` as the step and delay 5/2. |
| FR-242-AC-3 | covered | Covered. `a_timed_invariant_builds_no_automaton` asserts `automaton_states == 0` and that every certificate node has `automaton_state == 0`. Its model is an Rpc-like fixture, not ADR-026 §11's `Rpc`, which TC-697 names (FND-005). |

Can the random-lasso test catch a wrong automaton? For the transition function, yes. Its stamps fall on 0, 1, 2 and 3 ms and between them, so they hit each interval end, open and closed. Zero delays occur, `p` and `q` are drawn independently, and both verdicts are required. A swapped closedness, a wrong `until` guard order, or a missing position-0 guard check would each fail it. The probe in SR-1631001 also showed that `always`, `release`, the past operators and boolean combinations agree. But the test calls `bounded_window_verdict`, which drives `WindowClaim` with exact stamps. It never runs the zone product: the horizon clock, the pieces, the LU bound for `h`, the stutter and idle tail, or tick sources. So it cannot catch SR-1631001 FND-001, a wrong `Holds` under a tick source (FND-001 below).

Spec gaps on STD-493 comment 5b5714c3, posted 2026-10-26 by this lane:

1. **Nested windows.** The refusal is real behaviour, but the stated reason overstates the spec (FND-003).
2. **No idle-tail stutter delay in the counterexample.** This gap is real. FR-241's `Lasso { entry, terminal_stutter }` has no delay field, and `Concrete.prefix` adds nothing for a stutter, so replay cannot recover the delay.
3. **Certificate edges per horizon piece, and stutter edges.** This gap is real. FR-244's edge names only the transition, and one `(transition, part)` leads to different automaton states on different pieces.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-242's agreement criterion ("the automaton accepts exactly the timed traces on which the evaluator returns `false`") is tested only on `WindowClaim`'s transition, through `bounded_window_verdict` with exact stamps. The part of the translation that the product adds (the horizon clock, the piece decomposition, the LU bound for `h`, stutter and idle tail, tick sources) has only two zone-level tests: the punctual `[2,2]` pair and one idle-tail case. That is why SR-1631001 FND-001 (wrong `Holds` under a tick source) passes the suite. Add zone-level tests: a tick-source window (Violated), an open and a closed end at the product level (`always[0,2)` Holds and `always[0,2]` Violated over the `go`-at-2 ms model), and a lower-bounded window reached only by idle-tail stutters. | qsl-analyze/tests/it/zone_search.rs:1084-1104,1263-1330 |
| FND-002 | medium | The translation implements `always`, `release`, origin atoms, `once`/`historically`/`since`/`triggered` on origin, and `not`/`and`/`or`/`implies` over obligations, but the tests exercise only `eventually` and `until`. These forms do agree with the evaluator (the probe in SR-1631001: 13 formulas, 1,000 lassos each, no disagreement), so the fix is cheap: add a representative set to the agreement test's formula list, for example `always[1 ms, 2 ms]`, `release(1 ms, 2 ms]`, `holds(p) and eventually[1 ms, 1 ms]`, `once[0 ms, 1 ms] ... or ...` and `since[0 ms, 1 ms]`. | qsl-analyze/tests/it/zone_search.rs:1263-1330; qsl-analyze/src/zone_check/window.rs:497-590 |
| FND-003 | medium | The `NestedWindow` refusal is sound, but STD-493 gap 1 says "FR-242 translate[s] TT-2 with one horizon clock", which the spec does not say. FR-242 asks for "a timed automaton with one added horizon clock", and its Outputs give the claim automaton "its own clocks". FR-234 also defines the TT-2 horizon as "the sum of the formula's upper bounds", which only makes sense for nested forms. So refusing a nested `BoundedWindow` is an implementation scope cut against FR-242's "A `BoundedWindow` formula SHALL translate ...", not a contradiction in the spec. The cut is listed neither in the PR's "Deferred ACs and owners" nor on any ticket: QSL-583 owns MightyL but does not mention nested TT-2. Fix: correct the STD-493 comment, so the gap reads "FR-242 does not say how a nested TT-2 form gets its per-activation clocks". Record nested TT-2 on QSL-583, or name the ticket that owns it, and list it in the PR body. | qsl-analyze/src/zone_check/window.rs:603-627; spec/functional/FR-242-translate-a-timed-formula-into-a-claim-automaton.md |
| FND-004 | low | `an_idle_tail_passes_a_window` asserts only `terminal_stutter` and `entry == prefix.len()`. It does not check the verdict cause or that the violation needed the stutter. Since STD-493 gap 2 means the stutter's delay is not in the counterexample, nothing checks that the violating stutter lies past 3 ms. Assert `kind == Formula` and an empty prefix, and add a lower-bounded variant (for example `eventually[2 ms, 3 ms]` with `q` true from the start), whose violation depends on the stutter's piece. | qsl-analyze/tests/it/zone_search.rs:1335-1350 |
| FND-005 | low | TC-697 names ADR-026 §11's `Rpc` unit (`T = 3 ms` and `T = 4 ms`) as the FR-242-AC-3 fixture. The test builds its own Rpc-like model with a 3 ms deadline. The behaviour checked is the same, so this is a fixture mismatch, not a coverage gap. Either use the shared `rpc(t)` fixture already in `zone_search.rs`, or reword TC-697. | qsl-analyze/tests/it/zone_search.rs:1105-1133; spec/test-cases/TC-697-timed-formulas-translate-to-claim-automata.md |

## Verdict

**Not mergeable** together with SR-1631001: FND-001 here is the missing test that would have caught SR-1631001 FND-001. FR-242-AC-2 and AC-3 are covered as claimed. AC-1 is partial as claimed, and its TT-3 formula is soundly deferred to QSL-583. The random-lasso test is a real gate for the automaton's transition function, but not for the zone product. `NestedWindow` is a sound refusal with an inaccurate reason and no owner (FND-003). STD-493 gaps 2 and 3 are real.

## Coverage

Bindings checked: `a_punctual_window_is_decided_by_a_horizon_clock` → FR-242-AC-2 (correct), `a_timed_invariant_builds_no_automaton` → FR-242-AC-3 (correct). The random-lasso test carries `TC-697` only, which is correct for a partial AC. `quire matrix` was started and did not finish within the review, so the bindings were read from the `#[trace]` attributes directly.

## Dispositions

Round 1, fix commit 0f579f44c. Each finding was verified against the code and Linear. The two new STD-493 gaps (comment 520d0b26) were judged as follows:

- **Gap 1, horizon cut vs whole-trace undefinedness:** a real conflict between two FR-242 SHALLs.
- **Gap 2, idle-tail stutter delays:** a real gap, not a code bug. FR-231 makes each stutter delay any positive value, and FR-239 checks every behaviour. So an idle behaviour with stutters at, say, 1 ms and 3.5 ms has no position in `[2 ms, 3 ms]`. Under the pointwise meaning (FR-234, QSpec FR-416), `eventually[2 ms, 3 ms] holds(q)` is false on it, so `Violated` is the spec's verdict as written. The gap is the conflict with `TimedBehaviour::timed_trace`, which reads the idle tail with one fixed delay (the tick period, or 1) and would make it hold, plus whether that pointwise artefact is intended. Under a tick source the stutter is a tick and the verdict is `Holds`, which I probed.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Zone-level tests were added: `a_window_over_a_tick_source_is_read_at_each_tick` (would have caught SR-1631001 FND-001), `an_open_and_a_closed_end_are_read_on_their_own_pieces`, `a_lower_bounded_window_is_decided_by_the_idle_tail`, and `an_undefined_atom_inside_a_window_refutes_it`. |
| FND-002 | fixed | The agreement test now also runs `always[1,2]`, `release(1,2]`, a punctual window under `and`, `once ... or eventually`, `since`, and `historically implies always(0,1]`. It compares whole outcomes, with 1 reading in 25 undefined or short, and requires held, failed and odd readings to occur. |
| FND-003 | fixed | STD-493 comment 5b5714c3 was edited to drop the "one horizon clock" claim; it now says FR-242 does not specify per-activation clocks. QSL-583 comment a2a9789a takes nested TT-2 into its scope, and the PR body lists it under deferrals. `NestedWindow` stays as a sound refusal until QSL-583 lands. |
| FND-004 | fixed | `an_idle_tail_passes_a_window` asserts `kind == Formula`, an empty prefix and `entry == 0`. The lower-bounded idle variants are in `a_lower_bounded_window_is_decided_by_the_idle_tail`. |
| FND-005 | fixed | FR-242-AC-3 now runs over ADR-026 §11's `Rpc` through `rpc_claim`, at `T = 3` and `T = 4`, with zero automaton states asserted for both and `Holds` at 4. |
