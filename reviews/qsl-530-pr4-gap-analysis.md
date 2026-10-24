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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The fix round dropped `FR-133-AC-3` from the AC-3 test's `#[trace]`. `a_stutter_loop_a_bounded_prefix_and_a_deadlock_carry_no_hint` is now `#[trace("TC-534")]`, where its predecessor was `#[trace("TC-534", "FR-133-AC-3")]`. `quoin matrix` (exit 0) still counts it toward AC-3 (`static_status: tagged`, binder at line 210), apparently from the doc comment's leading `FR-133-AC-3:`, so coverage is not lost. But TC-534 says to tag the tests `#[trace("TC-534", "FR-133-AC-n")]`, the AC-2 test still does, and a doc-comment rewording would silently untag AC-3. Fix: restore `#[trace("TC-534", "FR-133-AC-3")]`. | qsl-analyze/tests/it/strong_hint.rs:208 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | `operations_are_ordered_by_declaring_type_then_name`, tagged `FR-133-AC-4`, cannot fail for AC-4's supertype leg. That leg is an admitting subtype `U` whose `EffectiveId` sorts *below* its declaring supertype `S`, where the hints must come as `U.m`, `S.a`, not `a`, `m`. In the fixture, the declaring `Base` (`276f74...`) sorts below the inheriting `Counter` (`5ee4fc...`), and `Base` is in `applications` in all four configurations. So the pre-fix behaviour gives the same `inc, setTo, bump` and the test passes. The pre-fix behaviour is admitting-type order with first-seen dedup, as at 5bbe8d1b1. Reproduced: with `operations()` changed to that (`retain` over a `BTreeSet` of `(declaring, name)`, no sort), the test passes, printing `inc, setTo, bump` for every configuration. (Removing only the sort, as the fix round checked, fails for another reason: `dedup_by` then leaves adjacent duplicates.) The oracle `operations == sorted` also restates the implementation's key. It does match the spec (derived `Ord` on `[u8; 32]` is digest-byte order), but it asserts no literal expected order. A hint-level two-type vector is not required. `strong_hints` walks `operations()` in order and only filters, and `hints_come_in_the_models_operation_order` already checks that pass-through at hint level. Fix: add a function-level configuration where the inheriting subtype's identity sorts below the declaring supertype's. Rename the identities until the digests order that way, and print or assert their order in the test. Assert the literal expected name order there (`bump` before `inc`, `setTo`, with `bump` declared by the subtype), so the pre-fix behaviour fails. | qsl-analyze/tests/it/strong_hint.rs:335-413; qsl-eval/src/simulation/model.rs:602-619 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The AC-3 test now replays FR-126-AC-2's bounded refutation (`bounded_fixture("eventually[0,1] holds(c.value = 3)")`, prefix length 1, no lasso), as well as the stutter. It also replays the `DeadlockFreedom` item's `kind: Deadlock` counterexample through `model_settle::replay`, which settles `Reproduced`. All three carry no hint. (Its tag regressed; see FND-005.) |
| FND-002 | fixed | `the_weak_each_mutex_refutation_names_acq_one_at_loop_position_zero` builds ADR-019 §6's mutex over `Counter` (`setTo(n)` as `acquire`, n in 1..2; `inc` as `release`). Under `fair weak each`, the lasso is `0 -acq(2)-> 2 -rel-> 0`. It carries exactly one hint, `{Strong, setTo, Each}`, at position 0, with an identity the loop never takes, so `release` gets none. The deferral of the `strong` half now cites FR-131 / QSL-532 instead of QSL-895. QSL-532 is still Done in Linear while replay refuses strong clauses. That ticket state is for the lead. |
| FND-003 | deferred | Spec-owner amendment. FR-133-AC-2 and TC-534 step 2 still say loop `upd(a)`, identity `upd(b)`, while the code and the tagged test give loop `b`, identity `a` (FR-126's tie to the earlier binding). This needs a spec change, which this code PR cannot make. The lead routes it to the spec owner. |
| FND-004 | fixed | `first_enabled_position` is now loop-relative (`position: index - lasso.entry`), matching FR-133's "first loop position". The doc says so. The mutex test replays the same loop behind a two-step stem (`entry: 2`) and asserts position 0, where a trace index would give 2. |
| FND-003 | fixed | Spec PR #1068 (merged to main 891825c45) amended FR-133-AC-2. It now reads: bound to `a`, loop `upd(b)` three times, identity `upd(a)`. That matches the code and `a_weak_refutation_names_the_strong_each_that_would_exclude_it`. The branch merges cleanly into that main. |
| FND-005 | fixed | ee5bbed62 restores `#[trace("TC-534", "FR-133-AC-3")]` on `a_stutter_loop_a_bounded_prefix_and_a_deadlock_carry_no_hint`. |
| FND-006 | fixed | d3e52c875 rewrites `operations_are_ordered_by_declaring_type_then_name`. The subtype declares `zap`, which sorts last by name. Eight supertype names are tried, and the test asserts that the identities fall both ways round. For each one it asserts the literal order (`zap, inc, setTo` when the subtype's id sorts first, `inc, setTo, zap` otherwise) across admitting configurations `c`, `bc` and `cb` and both declaration orders. Reproduced: with `operations()` restored to the pre-fix logic (`retain` over a `BTreeSet`, no sort), the test fails at `Root reversed false admitting c` (left `inc, setTo, zap`, right `zap, inc, setTo`). With the shipped logic it passes. |
