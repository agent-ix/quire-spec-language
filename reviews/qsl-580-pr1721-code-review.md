---
id: SR-1721001
title: "Code review of quire-spec-language PR #1721: Option<Clock> presence bits, the absent literal and the drop (QSL-580q part 1)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4a66a7b16; PR #1721 diff against qsl-580p-clock-positions (81fa6e048), 21 files, against FR-230 as amended by #1689 (Guards, Presence bit, AC-12/22/25/39/41/42), STD-513, and the STD-493 `absent` ruling (option 1)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1721

## Summary

Ticket: QSL-580 (580q part 1).

**Scope.** The diff is `git diff origin/qsl-580p-clock-positions...4a66a7b16`: 21 files across three commits plus two merges of the base. The base is 81fa6e048, the #1709 head I reviewed.

**Gates** (locked-build, jobs 4, debug info off, `--all-features`; QSPEC_DIR is the checkout at `/home/peter/dev/quire-specification`, now at 48373530):
- qsl-semantics, qsl-eval, qsl-forms and qsl-cst: every test binary passes. That includes qsl-semantics lib 804, it 1331 and qsl-eval it 809.
- qsl-analyze and qsl-replay `zone`/`timed`/`refinement`: 208 + 49 + 71 + 12, all pass.
- Workspace clippy `--all-targets -D warnings`: exit 0.
- The probes are in `probe_1721.diff`. They were not committed.

**The soundness checklist I was given:**

- **`value(self.v)` without proved presence refuses**, in guards and invariants.
  - It refuses alone, in `value(v) >= 1 or self.late`, and in `not present(v) and value(v) >= 1`. Each refuses `Unproved(Presence)`, both in guards and in `when true`.
  - The refusal has no locus (SR FND-003).
- **An absent `v` is never live and is never constrained.**
  - `live` returns the bit. `over` filters on `live`, so a `max` bound skips an absent `v`.
  - Guard parts and invariants check `presences_hold`.
  - It does still keep a valuation (FND-004).
- **The drop clears the bit, and the frame is checked.** `bits_follow` holds the dropped place absent. A drop outside the frame refuses `clock-write`, and a reset outside the frame refuses too.
- **The state key encodes presence.** Absent writes `{"type":"option","value":null}` and present writes the Clock shell. The keys differ.
- **Initial presence ranges over both values.** `ranged` uses `[false, true]` per object. The present clock starts at 0.
- **`absent` is typed only by its context.**
  - `contextual` plus `absent_literal`: an Option hint gives `OptionValue::none`.
  - Any other hint gives type-mismatch, and no hint gives ambiguous-literal.
  - It lowers to `LiteralValue::None`, which is FR-322's `none`.

**Probes beyond that list:**

- **A reset and a drop on the same step** (`self.v = 0 ms and self.v = absent`, in either order): from both initial states, one successor that resets `v` to 0 and leaves it absent. See FND-001.
  - `self.v = 0 ms and self.v = 1 ms` likewise gives one successor with resets `[(v, 0), (v, 1)]`, ending at 1.
- **`present(self.v)` inside a quantifier** (`forall(c in M::calls : present(self.v))`), and `present(c.v)` of another object: both refuse `ClockReadOutsideConstraint`. So do these:
  - an invariant `when { not present(self.v) }`;
  - `when { present(self.v) or self.late }`;
  - `urgent U when { present(self.v) }`, with or without `not`;
  - a postcondition conjunct `present(self.v)`;
  - a guard `self.v = absent` or `self.v != absent`.

  See FND-002.
- **`not present(self.v)` as a guard** is correct: it is enabled only from the absent state.

## Verdict

**Changes requested.** One high soundness defect (FND-001) and two medium findings (FND-002, FND-003). FND-004 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A postcondition that writes one `Option<Clock>` place twice is accepted, and the timed subject builds a successor from it. With `self.v = 0 ms and self.v = absent` in either order, S3 records both a reset and a drop. The step resets `v` to 0 and `bits_follow` holds it absent, so the post-state has `v` absent but reset. That state satisfies neither conjunct: FR-230 says the write "makes the place present" and the drop "sets the presence false". So an operation whose postcondition has no model is enabled, which is a wrong result for deadlock freedom and an invented behaviour for safety. Two constant writes (`self.v = 0 ms and self.v = 1 ms`) are also accepted, and the last one wins. Refuse a postcondition that writes one place more than once with different values or with both a write and a drop (`clock-write` at the second write), or give it no successor. Add a test. | qsl-semantics/src/check/timed_model.rs:1979-2014 (`resets`); qsl-eval/src/simulation/model/timed.rs:789-845 (`bits_follow`); FR-230 "Presence bit" |
| FND-002 | medium | `present(self.v)` is read in two places only: guard DNF parts (`clock_atom`) and top-level conjuncts of an invariant's `when` (`presence_conjuncts`). Anywhere else, `self.v` falls to the data path and refuses `invalid_timed_model`/`ClockReadOutsideConstraint`. That covers `not present` or `present … or …` in a `when`, `urgent when { present(self.v) }`, a postcondition conjunct, a quantifier body (`forall(c in M::calls : present(self.v))`, `present(c.v)`), and `self.v = absent` or `self.v != absent` in a guard. FR-230 "Presence bit" says `present(p)` "SHALL check as a `Boolean` data predicate over the place's presence … read through the data-predicate path and never as a clock read", and AC-39 says it checks as an operand of `and` and `not`. The refusals are sound, but they reject admitted inputs and name a clock read that never happened. Either read the bit through the data path (state predicates, urgency, postconditions, quantifiers, equality with `absent`), or, where a position stays unsupported this slice, refuse `TimedNotChecked` naming it and record it on QSL-580. | qsl-semantics/src/check/timed_model.rs:1447, 2524-2560, 2728; FR-230 "Presence bit"; FR-230-AC-39 |
| FND-003 | medium | The `unproved-presence` refusals carry no locus. `draft_part` and `time_invariant` return the bare `CheckCause::Unproved(Obligation::Presence)`, so the refusal is placed at `Location::root(Expression)` with no index and no span (seen in the probes). FR-230 "Guards" and AC-12 require the refusal "at the read" of `value(p)`. Carry the span of the `value(self.v)` read (or the constraint's span) into the refusal, and assert it in the test. | qsl-semantics/src/check/timed_model.rs:1816, 2509; FR-230 "Guards"; FR-230-AC-12 |
| FND-004 | low | A dropped or absent `v` keeps a valuation that elapses. In the probes, `next.clocks().get(v)` is 1 after a drop from a state at 1 ms, and an absent `v` reads 1 after a delay of 1. The zone search never frees its dimension (search.rs frees only the horizon clock and pre-path resets). FR-230 says a dropped clock has "no zone constraint, time invariant bound or elapse … it has no valuation". No verdict changes, because every read needs presence and every presence-setting write resets the clock. But zone nodes and timed states that differ only in a dead clock are kept apart. Free the place's dimension on a drop, and treat an absent place's valuation as a don't-care in state identity. | qsl-eval/src/simulation/model/timed.rs (`live`, `steps`); qsl-analyze/src/zone_check/search.rs:1360-1390; FR-230 "Presence bit" |

## Dispositions

Round 1, fix commit c1387e815.

Gates (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 48373530):
- qsl-semantics and qsl-eval: all binaries pass, including semantics it 1334, lib 804 and eval it 809.
- qsl-analyze and qsl-replay `zone`/`timed`/`refinement`/`clock`: 12 + 209 + 49 + 71, all pass.
- Workspace clippy `-D warnings`: exit 0.

I re-ran the probes on the head; they are in `probe_1721_r1.diff`, not committed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `ResetCut.option_writes` refuses a second write of one `Option<Clock>` place with `conflicting-clock-write` at the second write. Probes: `self.v = 0 ms and self.v = absent`, its reverse, and `self.v = 0 ms and self.v = 1 ms` each refuse `ConflictingClockWrite` at the second conjunct. A drop alone still steps with `v` absent. |
| FND-002 | fixed | `present(self.v)`, `self.v = absent` and `self.v != absent` are now presence literals in a guard's DNF and in a `when` or `urgent when`, whose state predicate becomes `StateAlternative`s.<br>Soundness: `holding` admits an object when any alternative holds, which is `or` semantics. A bound on `v` needs `(v, true)` in every alternative (`inv-present-or-late-v` refuses `UnprovedPresence`). An unsatisfiable `when` gives no alternative and never applies (`inv-contradiction`: 0 bounds).<br>Probes:<br>- A guard on `= absent` or `!= absent` enables only the matching state.<br>- A guard with `present(v) or late` enables only the present state.<br>- `when { not present(v) }` bounds only the absent state, and `when { present(v) or late }` only the present one.<br>- `urgent when { present(v) }` and its negation are urgent in exactly one state each.<br>Three positions refuse `unsupported_construct`/`not-yet-implemented` (`PresenceNotChecked`) at the read: a postcondition conjunct, a quantifier body, and `present(c.v)`. This deferral is acceptable for this slice. The refusal is sound, carries a correct code and locus, and is no longer a misleading clock-read cause. The PR body lists it as remaining QSL-580 work, but the QSL-580 ticket does not record it yet. |
| FND-003 | fixed | `TemporalCause::UnprovedPresence { span }` (`undefined_expression`/`unproved-presence`) carries the span of the constraint that contains the read, which starts at `value(self.v)`. In a guard `value(self.v) >= 1 ms or self.late` that is 675..696. `an_option_clock_reads_its_presence_bit` asserts it. |
| FND-004 | fixed | `insert` frees every non-live place's dimension after canonicalisation. My checks of the freeing logic:<br>- **Reachable states.** Freeing does not make a reachable state unreachable. Every place that becomes live on a step is reset or carried from a live place: `steps` adds a 0 reset for each place that was not live before and is live after, and `bits_follow` sets an `Option<Clock>` bit only by a reset. So no successor reads the freed value.<br>- **Distinct live states.** Freeing does not merge them. The discrete state key still separates presence, and `free` on a canonical DBM drops only constraints on the freed clock.<br>- **Certificate replay.** It checks inclusion, which a freed (larger) node still satisfies.<br>- **Test.** `the_zone_search_frees_an_absent_option_clock` gives 3 certificate nodes in place of 4.<br>The eval-side `TimedState` still carries a stale value for an absent clock (1 after a delay in the probe). It is unread, since the zone frees the clock and counterexamples filter on `live`. |
