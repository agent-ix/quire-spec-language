---
id: SR-12124
title: "Gap analysis of quire-spec-language PR #953: qsl-eval automaton and fairness kernels against ADR-018, ADR-019, FR-126 and FR-130"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b4d2e0400c3c706869fa42c7b8a6bd5e1087582e; PR #953: ADR-018 SM-6, IV-3, IV-5, IV-6, EN-1 SCC order; ADR-019 FS-2 to FS-6; FR-126 automaton and second-phase SHALLs; FR-130 filter SHALLs and AC-1 to AC-5; against qsl-eval/src/automaton/*, qsl-eval/src/fairness/* and their tests"
review_set: subset
---
# Gap analysis of quire-spec-language PR #953

## Summary

Ticket: QSL-524. Plan completion: not assessed. Planless, read-only; no
`quoin matrix` run, since the brief excludes building and running tools on the
lane. The AC-to-test check below was done by reading the code and the spec.

- ADR-018 SM-6 (bad-prefix monitor, GBA for the negation): implemented
  (`Monitor`, `Gba`) and tested.
- ADR-018 IV-3 lower-bound-0 counters, the past age counter, the shift
  counter, and the offset sets for `eventually`/`always`: implemented and
  tested exhaustively against the expansion.
- ADR-018 IV-6 / FR-126 `max_automaton_states`: `Numbering` and `Monitor`
  refuse at the limit, tested at limits 0, 2, 3 and 50 (FR-126-AC-6's 50).
- FR-126 and FR-130 SCC order (amended to least vertex index): implemented and
  tested (`a_self_loop_is_an_edge_and_components_follow_their_least_vertex`).
- FR-130 filter (a)/(b)/(c) and refinement, and the FS-6 depth-first first pass:
  tested on the FR-130-AC-1, AC-2 and AC-5 product graphs, built from the
  engine's exploration rule. The witness trees match the AC text: AC-4's
  `UnfairStrong[acq(1)]` with sub `{(2,q1)}` `Trivial`, and AC-5's
  `UnfairStrong[acq(1), acq(2)]` with sub `{(3,q1)}` `Trivial`.
- FR-130 Cancel before each decomposition: tested.
- CX-3 lasso fairness: tested for weak, strong and stutter.
- Trace tags: none added. Correct, because every FR-126/FR-130 AC is an
  engine verdict that these kernel tests do not fully check.
- Spec edits: the ADR-018/FR-126/FR-130 wording now states the target design
  (least-vertex SCC order, `b + 2` past values) and matches the code.

## Verdict

Approve. Not blocking: two mediums and one low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | IV-3 says any occurrence with a nonzero lower bound, or with an interval-operator operand, keeps its open obligations as a set of offsets. The PR has offset sets only for `eventually` and `always`. There is none for `until[a,b]`/`release[a,b]` in general, where a window is also killed when the left operand fails, so `EventuallyWindows` does not model it. There is none for past operators with a nonzero lower bound either: `once[a,b]` with `a > 0` cannot be decided from the most recent age that `PastAgeCounter` keeps. The translation that consumes these kernels will have no counter for those occurrences. | qsl-eval/src/automaton/counters.rs:251-396; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:552 |
| FND-002 | medium | The ticket's Delivers line (ticket text, data) names a DFA and MightyL in `qsl-eval::automaton`. Neither is in the PR. The module doc says the bounded profile's finite monitor goes through `Monitor`, but no test builds a TP-2 monitor, and `Monitor::bad_prefix` trims to states with an infinite run. That is only right for a bounded automaton built with accepting sinks. MightyL (FR-242, ADR-026 EZ-9) is not in the ticket's acceptance FR list. Record the scope either as delivered later, under a ticket, or as covered by `Monitor` with a TP-2 test. | qsl-eval/src/automaton/mod.rs:14-19; qsl-eval/src/automaton/monitor.rs:115-135 |
| FND-003 | low | ADR-019 FS-6 still says sub-SCCs are "enumerated in discovery order within their parent". ADR-018 EN-1, FR-126 and FR-130 (which cites FS-6) now say least vertex index. Amend FS-6 in the same way so the record FR-130 cites states the order the kernel uses. | spec/decisions/ADR-019-strong-fairness.md:110 |
