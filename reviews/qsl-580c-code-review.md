---
id: "SR-12265"
title: "code-review of QSL-580 remainder: timed lasso evaluation (FR-231, ADR-026 CT-2/CT-3) and FR-235 V-8 punctual settlement (PR #1153)"
type: "SpecReview"
analysis: "code-review"
scope: "agent-ix/quire-spec-language@63a8bd86a9711dcec28fbe5fff6aa83558e94c70; PR #1153; git diff origin/main...HEAD (qsl-eval/src/temporal/{timed.rs,outcome.rs}, qsl-eval/tests/it/timed_trace.rs, qsl-replay/src/{outcome.rs,proof_result.rs}, qsl-replay/src/spine/clause/temporal.rs); spec FR-231 Evaluation, FR-235 V-8 and AC-4, FR-100-AC-8, ADR-026 CT-2, CT-3, DF-5, TV-1; QSpec FR-331 unsupported causes, FR-416; QSL-580"
review_set: "subset"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: references
---

## Summary

Ticket: QSL-580. This is the code review of PR #1153, with the rust-review lane folded in. The PR adds the `TimedTrace::lasso` constructor and the lasso arm of `evaluate_timed_at`. The lasso arm unrolls the loop and then continues a future scan through the last copy with stamps shifted by `D`. The PR also adds the `UnavailabilityCause::PunctualInterval` cause. One high finding: the scan's stop rule for the tail returns a wrong verdict when the loop holds equal stamps. This was reproduced. There is also one medium (the wire record has no locus) and two lows.

## Verdict

**FAIL (blocked)** on FND-001 (high). FND-002 is medium and FND-003 and FND-004 are low. Fix them in this PR.

Examined with no defect:

- **`TimedTrace::lasso` refusals.** It refuses a `loop_start` outside the represented positions (`EmptyLoop`), `D <= 0` (`NonPositiveLoopDelay`), and `stamps[start] + D < last` (`LoopReentersBeforeLastStamp`). These are exactly CT-2's `D > 0` and the non-decreasing stamp rule across the re-entry step. Re-entry exactly at the last stamp is admitted, and that is correct because equal stamps keep step order. The four cases are tested at their boundaries. Indexing `stamps[start]` comes after the range check.
- **Unroll layout.** Copy `c` holds `stamps[s..n] + c·D`. Atom values past `n` are copied from `s + (k − s) mod L`. A position past the represented trace maps to `N − L + (k − s) mod L`. The last copy starts at class 0, so this is the matching class. The first tail position is `copy = N − L` with `shift = 1`, which is `τ_N`. Stamps are therefore periodic and exact.
- **Copy count.** Copies = `⌊Σ upper / D⌋ + #temporal + 3`. Future nodes keep their operand's periodicity point. Each past node moves its point back by at most `⌈b/D⌉ + 1` periods along its nesting chain. The count is at least `Σ b/D + #temporal + 2`, so every node and the root have settled by the last copy. I judge it sufficient, although the code states this without proof (FND-004).
- **Termination and meter.** Every tail iteration charges one unit. Its offset grows by `D > 0` per wrap, so a bounded window ends at `!reaches` and an unbounded one ends at `covers_a_period`. `unrolled` charges each copied stamp before it extends the stamp vector, so memory is bounded by the meter. Overflow of the copy count refuses `TraceTooLong`.
- **Kleene and Undefined.** On an undefined atom, every atom slot from that position on is `Unknown`, unrolled copies included, and the copy step is skipped. That is conservative. Kleene min and max are monotone, so `Unknown` never becomes `False`.
- **Past operators** scan the unrolled history `0..=i` with exact stamps, so they need no tail. An unbounded claim on a lasso is two-valued rather than `Pending` (tested).
- **Panics and typed errors.** No `unwrap`, `expect` or `panic!` in production code. `values[image]` indexing is in range because `image < represented <= len`. The new refusals map to `invalid_runtime_input`/`invalid-value`. They fault in the step-only spine path, as the existing timed refusals do.
- **FR-100-AC-8.** The `qsl-replay` change adds arms to the private `trace_refusal` and imports `qsl_semantics::family::UnsupportedItem` (not a `qsl_eval` path). No public `spine` item or re-export names `qsl_eval`.

## Validation

There was one focused run, `locked-build.sh … cargo test -p qsl-eval --test it timed_trace`. The PR's 8 timed_trace tests pass. A temporary repro test was appended for that run, reverted afterwards and never committed. It failed as FND-001 describes. `target/` was deleted. No full gate was run, as briefed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `future_past_the_trace` stops once `offset >= lower + D` (`covers_a_period`). Loop positions before `i` that share `i`'s stamp reach offset exactly `D` on their first tail visit. So the scan stops after reading the first of them, and the later equal-stamp classes are never read. Reproduced with stamps `[0,0,0]`, levels `[0,2,0]`, loop 0 and delay 1 ms: `always eventually holds(c.level = 2)` at 0 returns `Violated(0)`, and `eventually holds(c.level = 2)` at 5 returns `Violated(5)`. Both are `Holds`. An open lower end loses its offset-`a` visit the same way. Fix: end the tail scan only after `L` consecutive positions have been read inside the window (or at `!reaches`), and add this vector as a test. | qsl-eval/src/temporal/timed.rs:229-235, 362 |
| FND-002 | medium | QSpec FR-331 spells the cause `punctual-interval{locus}`, where `locus` is a checked-package `{node, occurrence}`. `OutcomeItem` emits `punctual-interval` with no locus and drops the span. A record built from this cause would be non-conformant on the wire. Nothing produces it today (FND-001 in SR-12266). The doc comments say "naming the interval", but that is untrue at the wire. Either keep the cause unwired and record the locus as Remaining work on QSL-580, or emit the locus once QSpec's node identity is available. Do not wire settlement before then. | qsl-replay/src/outcome.rs:559-567; qsl-replay/src/proof_result.rs:366-372 |
| FND-003 | low | The display text of `NonPositiveLoopDelay` reads "a timed lasso's loop has a positive total delay", which is the opposite of the condition it reports. It should read "has no positive total delay". | qsl-eval/src/temporal/outcome.rs:127 |
| FND-004 | low | The `unrolled` doc says "and two more" but the code adds 3. Neither the module doc nor `unrolled` gives the argument that this many copies settle every node. Fix the doc and state the bound: each past node adds at most `⌈b/D⌉ + 1` periods, and future nodes add none. | qsl-eval/src/temporal/timed.rs:625-629, 664 |
