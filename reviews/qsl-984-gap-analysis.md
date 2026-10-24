---
id: SR-16626
title: "Gap analysis of QSL-984 FR-125-AC-1/2 and FR-133-AC-5 coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #1429 (branch qsl984-en1-test-gaps, head f909c7201); FR-125-AC-1, FR-125-AC-2, FR-125-AC-7, FR-133-AC-1, FR-133-AC-5, FR-127-AC-7; TC-520, TC-534; qsl-analyze/tests/it"
review_set: subset
---

## Summary

Ticket: QSL-984. This was a planless gap analysis. Plan completion: not
assessed.

`quoin matrix --json` (quoin 0.28.4) at head f909c7201 gives these binders:

- FR-125-AC-1 is `tagged`, bound to
  `liveness::the_counter_behaviour_is_positions_anchored_by_initialization_then_each_inc_and_a_stutter`.
- FR-125-AC-2 is `tagged`, bound to
  `liveness::a_bounded_always_reads_past_closure_as_false_only_under_event_position_false_extension`.
- FR-133-AC-5 is `tagged`, bound to
  `strong_hint::the_strong_whole_mutex_refutation_carries_the_same_hint`.
- FR-133-AC-1 keeps its two binders.
- FR-125-AC-7 and FR-127-AC-7 stay `untagged`.

On origin/main, none of FR-125-AC-1, FR-125-AC-2 or FR-133-AC-5 appears in any
`.rs` file. The two new untagged tests cite only "FR-125's Instancing" in their
doc comments, and neither cites an AC id. The matrix counts doc-comment ids
too, so FR-125-AC-7 is not claimed.

### FR-125-AC-1, clause by clause

| # | Clause | Asserted? |
| --- | --- | --- |
| 1 | Counter subject, universe `{c}`, initial value 0, no `terminal` | yes: the fixture `counter(document(false), ...)`, with `values[0] == 0` |
| 2 | one behaviour, positions 0, 1, 2, 3 | yes: one successor at each step, values `[0,1,2,3]`, and no model successor at 3 |
| 3 | anchors `initialization`, `inc`, `inc`, `inc` | yes: `AnchorKind::Initialization`, then three equal `Handler` anchors naming `Counter/inc` |
| 4 | under infinite-trace, a terminal stutter repeating position 3 | yes: `Product::infinite` gives exactly `TerminalStutter`, with value 3 and the same state key |
| 5 | the stutter's identity differs from every `inc` identity | by type only: the `assert_ne!` is tautological (SR-16625 FND-001) |

The tag is complete. Clause 5 holds by construction in the product.

### FR-125-AC-2, clause by clause

| # | Clause | Asserted? |
| --- | --- | --- |
| 1 | `always[0,5] holds(c.value <= 3)` false under event-position false-extension | yes: the evaluator on `Finite{len:4}` gives `Violated`, and EN-1 gives a `Formula` violation. The red check (sentinel True) fails here |
| 2 | the same formula true under infinite-trace | yes: the evaluator on `Lasso{3,1}` gives `Holds`, and EN-1 gives `Holds` |
| 3 | `eventually holds(c.value = 3)` true under infinite-trace | evaluator only, because EN-1 does not decide `eventually`. This is a sound reading of the AC |
| 4 | `always eventually holds(c.value = 0)` false | yes: the evaluator gives `Violated`, and EN-1 gives a terminal-stutter lasso violation |

The tag is complete. Atom values are supplied by the test (SR-16625 FND-003).
The behaviour's values are read from the model's own successor relation.

### FR-133-AC-5, clause by clause

| # | Clause | Asserted? |
| --- | --- | --- |
| 1 | mutex refuted under `strong` (whole) on `acquire` | yes: `TerminalValue::Refuted` |
| 2 | hint `{Strong, acquire, Each}` | yes: kind, granularity and operation name (`setTo` is the fixture's `acquire`) |
| 3 | identity `acq(1)` | yes: the operation `Counter/setTo` and the literal integer argument 1 |
| 4 | no hint for the held `Whole` constraint | yes: `len == 1` and `Each`. The extra `all(!= Whole)` is redundant (SR-16625 FND-002) |
| 5 | FR-131-AC-1's lasso under `strong each` refused `UnfairLasso` by replay | yes: `mutex_lasso_for(STRONG_EACH)` gives `Err(UnfairLasso)`. The red check (UnfairLasso return dropped) fails here |
| 6 | carries no hint | yes: the replay has no result, and the EN-1 settlement has empty `hints` and no counterexample |

The tag is complete.

### Deliberately untagged

- FR-125-AC-7: the instancing clauses are exercised by
  `state_graph::a_state_graph_claim_is_instanced_over_the_subjects_universe`
  and `model_instances::a_state_graph_claim_is_instanced_like_a_clause`. The
  clause "the item reports `NoInstance`, settled `inconclusive`" is not
  implemented for state-graph items on main, and the empty-universe case only
  asserts an empty instance list. The QSL-891 comment (2026-10-24) records
  that the tag waits on that settlement. Leaving the AC untagged is correct.
- FR-127-AC-7: the spec says `model_check.max_states`, while
  `ModelCheckLimit::MaxStates.setting()` returns `max_states`. The zero
  `work_units` meter case and the `requested` cancellation case are not
  settled in the candidate tests. QSL-1043 (Backlog) tracks this. Leaving the
  AC untagged is correct.

No production code changed, so no code without an owning requirement was
introduced.

## Verdict

**APPROVED.** The three new tags are complete against their AC text on main,
and two of them are red-checked. The two ACs the PR does not cover are
correctly left untagged and are tracked in tickets. One low process finding
remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | QSL-984's title and Work list still name FR-125-AC-7 and FR-127-AC-7, but the ticket has no Linear relation to QSL-1043 (FR-127-AC-7) or to QSL-891 (the FR-125-AC-7 tag follow-up, which is only a free-text comment there). If QSL-984 closes when this PR merges, ix-board's structured data loses both follow-ups. Add `related` relations (QSL-984 to QSL-1043 and to QSL-891), and narrow the ticket title or record the hand-off before closing. | QSL-984; QSL-891; QSL-1043 |
