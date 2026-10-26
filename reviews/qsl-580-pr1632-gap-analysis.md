---
id: SR-1632002
title: "Gap analysis of quire-spec-language PR #1632: FR-230 clock-reading declarations (580e)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ae196e6638e32dcf549b0619f7f92b1534d6f3ae; PR #1632 against FR-230-AC-9, AC-10, AC-14 and FR-230 Clock-typed positions (Parameters, Results, Clock-reading declarations, Bounds, Records), TC-685; QSpec shared grammar clock-bound (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1632

## Summary

Ticket: QSL-580, slice 580e. Plan completion: not assessed. This review used
the trace attributes on the PR head, the AC text and the code.

Backed by tests: AC-9's precondition expansion, the absence of a node for
`near`, and the four refusals. AC-10's `pick` and `g(): Clock { 0 }` are
backed but untagged. AC-14's second half is backed but untagged.

Deferrals checked against the spec:
- **AC-9's time-invariant vector (STD-493 item 10): real spec gap.** The
  `time-invariant` body is a list of `clock-bound` (`postfix, ('<'|'<='),
  quantity-literal`), so a call cannot be written there.
- **AC-9's `within`: real gap.** `within` is a reserved token in
  `qsl-cst/src/token.rs`.
- **AC-9's "body that resets `c`": real spec gap.** A function body is an
  expression, so `c = 0 ms` there reads as the constraint `c = 0`, not as a
  reset.
- **AC-10 `peer_clock` (item 11): not a spec gap** (FND-001).
- **AC-14's first half (package `t: Timer` under tick or untimed, `ts:
  Sequence<Timer>`)** is already required to be built (AC-11..14), so it is
  not flagged here.

## Verdict

**CONDITIONAL.** AC-10's deferral reason does not hold. The tagged AC-9 tests
leave out vectors without saying which ones in the tag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-230-AC-10's `peer_clock` is deferred because "FR-230 does not define which instance a non-self place denotes". FR-230 "Parameters" does define it: "A path through a reference field ... SHALL check as one place for each object ... it can reach, each with the data condition that selects it, and a constraint over it SHALL record the disjunction". AC-10 fixes the universe `{c1, c2}`. That universe is available where the subject expands places (FR-231 `TimedSystem` already lists places per object), or S3 can record the reference place symbolically. This is an implementation gap that can be built now, not a spec gap. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:315-323, 589; qsl-semantics/src/check/timed_model/reading.rs:150-170 |
| FND-002 | low | The two tests tagged FR-230-AC-9 cover only part of AC-9. The time-invariant vector and the reset vector are missing (spec gaps), and the predicate is named `near`. The tag claims the whole AC, and only the doc comments record what is missing. | qsl-semantics/tests/it/timed_model.rs:560-640 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Moved to slice 580h (coordinator ruling, 2026-10-26). STD-493 comment 582cd0a5 withdraws item 11 as an implementation gap. Until then, `peer_clock` refuses `TimedNotChecked { ReferencedClock }`. |
| FND-002 | fixed | Both AC-9 tests carry only `#[trace("TC-685")]`, and their doc comments say "Part of FR-230-AC-9" and name the missing vectors. |
