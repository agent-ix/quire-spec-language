---
id: SR-1631001
title: "code-review and rust-review of PR #1631 (QSL-581c, FR-242 TT-2 BoundedWindow claim automaton)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1631 git diff origin/qsl-581b-zone-certificate...edd7fa765: qsl-analyze/src/zone_check/{window.rs,search.rs,certificate.rs,concretise.rs}, qsl-analyze/src/zone_check.rs, qsl-eval/src/simulation/model/timed.rs, qsl-analyze/tests/it/zone_search.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-242
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: reviews
---
# SR-1631001: code-review and rust-review of PR #1631 (QSL-581c, FR-242 TT-2 BoundedWindow claim automaton)

## Summary

Ticket: QSL-581. Head reviewed: edd7fa765, which is 844386769 rebased onto the fixed #1627 (34bc06f68). `git range-diff` shows that 581c's own diff is unchanged apart from one context line in the tests. I traced the claim automaton (`WindowClaim`: obligations, `position`, `pieces`, `read_root`), its use in the product search (`expand`, `window_position`, `judge`, `scaled_bounds`, `dim`), the stutter and idle-tail handling, the counterexample lasso, concretisation with the added clock, and the certificate's per-piece and stutter edges. I compared all of this with the reference evaluator `evaluate_timed_at` (qsl-eval/src/temporal/timed.rs) and with FR-231, FR-234, FR-242 and ADR-026 EZ-9.

Soundness for dense time:

- **Acceptance.** The automaton's transition matches the evaluator's `future` scan exactly. A position whose stamp is in `I` with `right` true decides true. Otherwise, a position past `I` or with `left` false decides false, in the evaluator's order: the witness is read before the guard at the same position. A Kleene-false root is the violation.
- **Endpoints.** Closed and open ends are decided by `TimedInterval::contains`/`reaches` on the exact stamp. Every lower and upper end is a breakpoint, so `contains` and `reaches` are constant on each piece: a breakpoint `h = c`, or an open stretch between breakpoints. One representative per piece is therefore exact.
- **Horizon.** `h` is never reset. Its LU bound is the largest breakpoint on both sides, and the piece above the last breakpoint is uniform, so the extrapolation is sound.
- **Past operators on origin.** `once`, `historically`, `since` and `triggered` are read at position 0 and offset 0, and they are correct.

I confirmed the transition beyond the three tested formulas with a probe that ran 13 more formulas over 1,000 random lassos each, all agreeing with `evaluate_timed_at`: `always` with closed and open intervals, `release` with closed and half-open intervals, a punctual `eventually` under `and`, `implies always[..)`, `not eventually`, `once ... or eventually(..)`, `until[0,3]`, `eventually and always`, `since`, `triggered`, and `historically implies eventually`. At the product level, a second probe gave the right verdicts for `always[0,3]`, `[0,1]`, `[0,2)` and `[0,2]` over the `go`-at-2 ms model: Violated, Holds, Holds, Violated.

The automaton is not sound under a tick source (FND-001). Under digital time `elapses` is false, so the zone never lets `h` grow. A tick step does not advance it, and no stutter is taken. Every position is therefore read at `h = 0`. Reproduced at edd7fa765: a tick source with period 1 ms and `tick` always enabled, with `eventually[0 ms, 3 ms] holds(c.retried)` where `retried` is never set, returns `Holds` with `automaton_states = 1`. The fourth tick has stamp 4 ms, past the window, so the right verdict is `Violated`. The probes ran in my detached worktree and were reverted. Nothing was pushed.

Gates, run under locked-build with jobs 4 and `--all-features`. At 844386769: `cargo fmt --all --check` exit 0, and `clippy -p qsl-analyze -p qsl-eval --all-targets -D warnings` exit 0. At edd7fa765: `zone_search` 22/22 plus my probe, and the zone_check lib tests 7/7 (run at 844386769).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Under a tick source the horizon clock never advances, so a TT-2 clause over a digital-time model is decided as if all of time were 0. `elapses` is false under a tick source, so the zone is never `up()`'d. A tick step does not move `h`, and no idle-tail stutter is taken at all. FR-231 "Digital time" stamps position `i` as the period times the ticks so far, and counts the terminal stutter as a tick. Reproduced: `time M tick M::Call::tick period 1 ms`, `tick` always enabled, `eventually[0 ms, 3 ms] holds(c.retried)` with `retried` never true. The result is `Holds` (automaton_states 1), but the right verdict is `Violated` at the 4th tick. A wrong `Holds` is the worst outcome a checker can give. Fix: under a tick source, advance the stamp by the period on each tick step and on the terminal stutter, for example by keeping the tick count, capped past the last breakpoint, in the automaton state instead of reading `h`. Or refuse TT-2 over a tick source until this is done. Add the probe as a regression test. | qsl-analyze/src/zone_check/search.rs:547-549,851-940 |
| FND-002 | medium | FR-242 and ADR-026 EZ-9 say "the product search SHALL stop each path when the horizon clock exceeds the formula's horizon". It does not. Every path keeps expanding past the last breakpoint until the whole model state space is covered, though the claim is decided by then. This is not wrong, but a claim decided within its horizon can still end `Stopped(LimitReached)` on a large model, and the deviation from a SHALL is neither stated nor filed. Fix: stop expanding a node whose automaton state has every obligation decided, or whose `h` lies in the last piece. | qsl-analyze/src/zone_check/search.rs:851-940; qsl-analyze/src/zone_check/window.rs:211-247 |
| FND-003 | medium | Undefined atoms are handled differently from the evaluator, and no TT-2 test reaches that path. `position` reads both operands of every open `until` at every position. So an undefined `left` at the position where `right` decides true, or an undefined `right` before the window, gives `Badness::Undefined`, although the value does not depend on it. The other way round, once an obligation is decided the automaton reads nothing more, while `read_atoms` reports the first undefined atom at any represented position. Every `UndefinedCause`, including `InsufficientData`, is treated as an undefined counterexample, whereas the evaluator reads `InsufficientData` as Kleene-unknown. Fix: read `right` only when `h` is in `I` and `left` only before a decision, decide which reading FR-242 wants (it says only "agrees with the evaluator"), and add a TT-2 undefined-atom test. | qsl-analyze/src/zone_check/window.rs:278-347; qsl-eval/src/temporal/timed.rs:465-530 |
| FND-004 | low | Stutter positions are read on the pieces of the node's settled zone, which includes zero delay (`h` equal to the arrival stamp). FR-231 and `concretise` require a positive delay before each stutter (`times.constrain(at - 1, at, lt 0)`). Since a zero-delay stutter re-reads the same state at the same stamp, it is idempotent and gives no wrong verdict. But a symbolic path through such a stutter piece, for example the punctual piece `h = c` entered at its arrival instant, can fail concretisation and end as `Fault("concretisation")`. Fix: exclude the arrival instant from the stutter's piece, or let concretisation drop a zero-delay stutter. | qsl-analyze/src/zone_check/search.rs:916-940; qsl-analyze/src/zone_check/concretise.rs:142-148 |
| FND-005 | low | `CertEdge` does not record the horizon piece, so one `(from, transition, part)` can lead to several `to` nodes with different automaton states, and an FR-245 checker cannot re-derive which automaton transition an edge took. A stutter edge carries the sentinel `part: 0` instead of `None`. This is filed as STD-493 gap 3, which is right, but the code states it only in prose. Fix: add `piece: Option<Vec<PlaceConstraint>>` (or the piece index together with the breakpoints) once FR-244 names it, and make `part` an `Option`. | qsl-analyze/src/zone_check/certificate.rs:75-87,170-193 |
| FND-006 | low | `certificate::follow` is a recursive depth-first backtracking search over per-piece edges. Its recursion depth is the witness path length. When pieces dead-end, the worst case is pieces^length. A long `admitted` path, or a model with many breakpoints, can overflow the stack or stall. Fix: make it iterative and memoise the `(node, move index)` pairs already found dead. | qsl-analyze/src/zone_check/certificate.rs:261-284 |
| FND-007 | low | `bounded_window_verdict` stringifies its discriminants. An undefined reading becomes `ZoneCheckRefusal::Atom(format!("{:?}", cause))`, and the state limit becomes `Fault("automaton-states")`. A caller cannot tell an undefined atom from a stopped letter except by parsing the message. It is a `pub` function that exists only as a test seam. Fix: return the `UndefinedCause` typed (for example `Ok(Verdict::Undefined(cause))`), or make it `#[doc(hidden)]` and test-only. | qsl-analyze/src/zone_check/window.rs:373-405 |
| FND-008 | low | `expand` calls `window.pieces(self.horizon())` for every expanded node, which allocates the whole piece list each time, though the list is fixed per run. Compute it once when `Run::new` builds the window. | qsl-analyze/src/zone_check/search.rs:861-865 |

## Verdict

**Not mergeable** until FND-001 is fixed. Over a tick-source model a TT-2 clause can return a wrong `Holds`. Fix FND-002 and FND-003 in the same round. FND-004 to FND-008 are low.

What is correct, judged against the three questions in the review brief:

- **Soundness for dense time.** The horizon reads each position's exact stamp, because `h` is never reset and is read at the step's instant, before delay. Interval-end closedness is exact, because each end is its own piece. Acceptance is the Kleene-false root, with each obligation decided at the first position past its interval. Every interval is bounded (FR-234 refuses `[a, *]`), so a dense-time behaviour always decides each obligation, through a step or an idle-tail stutter.
- **The `NestedWindow` refusal.** It is sound and conservative: it refuses a form it cannot decide instead of deciding it wrongly. Whether the refusal is a spec gap is judged in SR-1631002.
- **Counterexamples.** The terminal-stutter lasso convention (`entry = prefix.len()`) matches the convention the untimed checkers already use.

Rust lane: production paths have no `unwrap`, `expect` or indexing panic (`statuses[slot]` is in range by construction). Automaton states are interned in a `BTreeMap` and bounded by `max_automaton_states`, checked before insertion. `ZoneCheckItem::Invariant` → `Clause` and `NotTimedInvariant` → `UnsupportedForm` are clean renames with no shim. `PlaceConstraint::new` is a documented `pub` constructor with one external caller.

## Coverage

Methods: code-review with the rust-review lane. Gap analysis is SR-1631002. The probe tests (the tick source, product-level `always`, and 13 formulas against the evaluator) ran in the reviewer's own detached worktree under locked-build, and were then reverted.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | In the product, an instance whose clause value is `False` but which read a short window (`InsufficientData`) is never reported (`violated` skips it). The search then ends `Holds`, where `evaluate_timed_at` gives `InsufficientData` for that trace. Today this is unreachable: a TT-2 clause's atoms are `holds` conditions with no window aggregate, and `Aggregate` is refused. But if it ever becomes reachable, it is a silent `Holds`. Fix: settle such a node as an undecided cause (or refuse the claim), not as nothing. | qsl-analyze/src/zone_check/window.rs:244-250 |

## Dispositions

Round 1, fix commit 0f579f44c. I verified each finding in a fresh detached worktree, re-ran the round-0 probes, and added new probes (tick terminal and tick-plus-`go` models, dense idle and `go` models, Rpc TT-2 windows, and 9 more formulas checked outcome for outcome on 1,000 lassos each, including undefined and short readings: 9,000/9,000 agree). All probes were reverted afterwards. Gates at 0f579f44c: fmt exit 0, clippy `-p qsl-analyze -p qsl-eval --all-targets --all-features -D warnings` exit 0, `zone_search` 26/26, zone_check lib 7/7.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Under a tick source the automaton state now keeps the stamp, capped at the last interval end plus one period. A tick step advances it by the period, and the terminal stutter counts as a tick. The round-0 repro is now `Violated` after four ticks. Under a tick source, `eventually[2,3]` holds and `eventually(2,3)` is violated over a terminal state, and `always[5,5]` after `go` is violated after five ticks, all as FR-231 "Digital time" requires. Regression test: `a_window_over_a_tick_source_is_read_at_each_tick`. |
| FND-002 | fixed | Once every obligation is decided, a step reads no position, keeps `q` and frees `h`, and the zone graph continues so that `admitted` still works. The conflict with the evaluator's whole-trace reading of undefined atoms is filed as STD-493 comment 520d0b26, gap 1. |
| FND-003 | fixed | Obligations now carry the evaluator's `(acc, held)` Kleene scan. Every atom is read at every open position, in clause-node order, and stops at the first undefined one. `bounded_window_verdict` matches `evaluate_timed_at` outcome for outcome, including `Undefined` and `InsufficientData`. A zone-level undefined test was added. The remaining product difference for `InsufficientData` is the new FND-009. |
| FND-004 | fixed | Fixed differently from the suggestion: concretisation now accepts a zero-delay stutter. This is sound because a re-read at the same state and stamp leaves `(acc, held)` unchanged, so a violation can never depend on it. Stutter delays are not in the counterexample (STD-493 gap 2). |
| FND-005 | fixed | `CertEdge.part` is now `Option<usize>` (`None` for a stutter), and `CertEdge.horizon` carries the piece as constraints on `h`. It is `None` without an automaton, past the horizon, and under a tick source. STD-493 gap 3 still names the spec side. |
| FND-006 | fixed | `follow` is now iterative, and dead `(node, depth)` pairs are memoised, so each pair is tried once. |
| FND-007 | fixed | `bounded_window_verdict` is `#[doc(hidden)]` and returns a typed `TemporalOutcome` (`Undefined` with its cause, `InsufficientData`, `Pending`), not a stringified refusal. |
| FND-008 | fixed | The pieces are computed once in `Run::new` (`Run::pieces`). Under a tick source there is a single piece. |
