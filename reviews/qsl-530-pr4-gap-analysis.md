---
id: SR-13352
title: "Gap analysis of QSL-530 E9 PR 4 against FR-133 (AC-1..AC-3) and TC-534 (PR #1047)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9ae691752; spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md (AC-1..AC-3, Behavior), spec/test-cases/TC-534-strong-fairness-hint.md, spec/decisions/ADR-019-strong-fairness.md (SV-6, §6, context), spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md (§6, context); qsl-replay/src/model_check/replay.rs, qsl-replay/src/model_check/settle.rs, qsl-eval/src/simulation/model.rs, qsl-analyze/tests/it/strong_hint.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-534
    type: reviews
---
# SR-13352: Gap analysis of QSL-530 E9 PR 4 against FR-133 (PR #1047)

## Summary

Ticket: QSL-530 (E9, PR 4 of 4). Tools: `quoin matrix --json` (quoin
0.28.3), exit 0. Plan completion: not assessed.

Matrix state of FR-133's acceptance criteria at 9ae691752:

| AC | Status | Tagged by |
| --- | --- | --- |
| AC-1 | untagged | none (deferred in the test-module doc) |
| AC-2 | tagged | `a_weak_refutation_names_the_strong_each_that_would_exclude_it` |
| AC-3 | tagged | `a_stutter_loop_and_a_prefix_carry_no_hint` |

`an_operation_the_loop_takes_in_part_is_named_each_with_the_identity_it_leaves`
is tagged `TC-534` alone. It backs FR-133 "Behavior" (the `Whole` and `Each`
rules), which no AC states on its own.

FR-133 "Behavior" against the code:

- Hints after a reproduced lasso, from the replayed enabled sets:
  implemented (`strong_hints`, called only on the reproduced arm).
- `Whole` when enabled and never taken, otherwise `Each` with the first
  never-taken identity in canonical transition order: implemented. Tested
  by the `Counter` test for both shapes.
- First enabled position: implemented as a trace index. Tested only with
  an empty stem (FND-004).
- Suppression when the clause already holds the constraint: implemented,
  but cannot be reached while replay refuses every strong clause. The test
  module says so.
- Copied onto the record, verdict unchanged: implemented in
  `settle_model_check`. Tested through AC-2 (value, label and counterexample
  equal to the engine's).
- No hint for safety, bounded, deadlock-freedom or stutter: implemented
  (empty vectors on those arms). Only part of it is tested (FND-001).
- SMT path's writer (the orchestrating driver): not in this repository. The
  PR body says so.

No stubs, no coverage inflation, and no production code without an owning
requirement were found in the diff.

## Verdict

**Not mergeable yet on coverage and spec drift: three medium findings, one
low.** FR-133-AC-3's named rows are only partly exercised (FND-001), and
FR-133-AC-1 is deferred whole, though half of it is buildable now, under a
ticket that does not own it (FND-002). FR-133-AC-2's text disagrees with
the code and with the test tagged to it, and only the spec owner can settle
that (FND-003). The code findings, including the merge-blocking conflict
with main, are in SR-13351.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-133-AC-3 names three refutations: FR-126-AC-3's terminal stutter over `Counter`, its deadlock-freedom refutation, and FR-126-AC-2's bounded refutation. TC-534 step 3 names the same three. The test tagged to AC-3 covers only the stutter one. It has no deadlock-freedom case. In place of FR-126-AC-2's bounded refutation (`eventually[0,5]` on the example, or `eventually[0,1] holds(c.value = 3)` `on each` over `Counter`) it uses an unbounded safety refutation, `always holds(c.value < 2)`. The deadlock and bounded arms return empty vectors in the code, but no test exercises them. Fix: add the `DeadlockFreedom` item's `kind: Deadlock` refutation and FR-126-AC-2's `eventually[0,1]` `Counter` refutation, both through replay. | qsl-analyze/tests/it/strong_hint.rs:204-229 |
| FND-002 | medium | FR-133-AC-1 is deferred whole: "needs a strong-fairness engine" (strong_hint.rs module doc, PR body). Only its second sentence (the `strong` whole refutation) needs strong fairness, which replay refuses today. The `weak each` refutation of ADR-019 §6's mutex should already run: EN-1 checks `weak each`, and parametric operations exist (`Counter`'s `setTo`). It has one hint, `{Strong, acquire, Each}`, identity `acq(1)`, position 0, and `release` gets none. That is the one AC that checks "taken in the loop, so no hint". The deferral also cites QSL-895, which is the Holds-certificate wiring, not strong-fairness replay. FR-131 replay belongs to QSL-532 (E10a), which Linear shows Done while replay still refuses a strong clause. Fix: test the `weak each` half of AC-1 here, and point the `strong` half at the ticket that will build FR-131 replay. The lead should reopen or split QSL-532, or file that ticket. | qsl-analyze/tests/it/strong_hint.rs:7-10 |
| FND-003 | medium | FR-133-AC-2 (and TC-534 step 2) says the loop is `upd(a)` three times, with identity `upd(b)`. The code and the test tagged `FR-133-AC-2` give a lasso bound to `a` that loops on `b`, with identity `upd(a)`, because FR-126 as amended by STD-203 breaks the tie toward the earlier binding. The test therefore asserts the opposite of the AC it claims to cover, and the matrix shows AC-2 as tagged. The PR body raises this as a spec question. Until FR-133-AC-2 and TC-534 are amended to match FR-126's ranking, the requirement and its evidence disagree. Fix: the spec owner amends FR-133-AC-2 and TC-534 step 2 (loop `upd(b)`, identity `upd(a)`). No code change is needed. | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:90; qsl-analyze/tests/it/strong_hint.rs:112-146 |
| FND-004 | low | `first_enabled_position` is a trace index: `LoopState::position` is the `enumerate` index before `.skip(lasso.entry)`. FR-133's use case says "the loop position", which could be relative to the loop. The two readings agree only when the stem is empty, and every test (and every AC) has loop entry 0. So a loop-relative implementation would pass every test. The PR body raises this as a spec question. Fix: the spec owner fixes the reading in FR-133 "Behavior", and a test adds a lasso with a non-empty stem (for example `Counter` with a weak `inc` constraint started at 1 under a `setTo` loop) asserting the chosen index. | qsl-replay/src/model_check/replay.rs:795-806; spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:70-71 |
