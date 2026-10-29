---
id: SR-1763002
title: "gap analysis of PR #1763 (QSL-580v): per-slot Option<Clock> bits and PR-03 quantified presence"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1763 at bf657815b against FR-230-AC-41/42/46/49..51, STD-526 PR-03, FR-415-AC-20/21"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1763002: gap analysis of PR #1763 (QSL-580v)

## Summary

These are covered:
- FR-230-AC-41, AC-42 and AC-46: `a_sequence_of_option_clocks_holds_one_bit_per_element` (7 initial states, live equals bit, present clocks at 0, distinct keys, option shells).
- FR-230-AC-19 and AC-41 at S3: the place `q.[slot]` with QSpec's presence-place rows; unbounded refuses.
- FR-415-AC-20 and PR-03: S3 data predicates in a `when`, in urgency, in a guard and over `self.peer.q`; eval over all 7 states, the empty `q` included.
- AC-49: the emitted `v` and `q` places equal QSpec's presence-place nodes.

Gaps:
- **FND-001.** No test resets elements of `q` (`forall(e in self.q : e = 0 ms)`). Such a test would have caught SR-1763001 FND-002.
- **FND-002.** `an_option_element_keeps_its_bit_without_a_write` asserts that an added element takes either bit. That pins SR-1763001 FND-001, so it changes with that fix.
- **FND-003.** No test drops a present slot by shrinking and then regrows it. FR-230: a dropped clock is unconstrained, and a later write resets it afresh.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test of a reset over `q`'s elements and their bits | FR-230 Presence bit, FR-415 Reset by selection |
| FND-002 | low | `an_option_element_keeps_its_bit_without_a_write` asserts added elements take either bit (pins SR-1763001 FND-001) | FR-415-AC-21 |
| FND-003 | low | No test of a slot dropped by a shrink and regrown (unconstrained, then reset afresh) | FR-230 Presence bit (drop) |

## Verdict

Changes requested together with SR-1763001. All three findings are low.

## Dispositions

Round 1, head 07cfab04e.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `a_reset_over_option_elements_makes_each_present`: from every initial state, `forall(e in self.q : e = 0 ms)` leaves every element present at 0 ms. The reset-free mutant makes it fail. |
| FND-002 | fixed | `an_option_element_keeps_its_bit_without_a_write` now asserts that every added element is absent and that `q` still grows. The added-free mutant makes it fail. |
| FND-003 | fixed | `a_dropped_element_regrows_absent`: `[true] -> [] -> [false]` only. The added-free mutant makes it fail. |

## Round 2 (delta review)

Head 92831e891, which carries QSL-1209.
- **FR-230-AC-52** (new) is covered by `the_clock_presence_frame_vectors_step_as_published`, traced `#[trace("TC-685", "FR-230-AC-52")]`:
  - all 14 FP vectors run, with `ran == 14`;
  - FP-11 and FP-12 give the S3 refusal, checked against the vector's `refused`;
  - every CK-31 pair outcome is asserted.
- **TC-685 step 35** matches the test.

No new gaps.
