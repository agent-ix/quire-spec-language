---
id: SR-1721002
title: "Gap analysis of quire-spec-language PR #1721: Option<Clock> presence bits, the absent literal and the drop (QSL-580q part 1)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4a66a7b16; PR #1721 against FR-230-AC-12, AC-22, AC-25, AC-39, AC-41, AC-42 (as amended by #1689), STD-513 and the STD-493 `absent` ruling"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1721

## Summary

Ticket: QSL-580 (580q part 1). Plan completion: not assessed.

**Covered for a top-level `v: Option<Clock>` field:**

- **FR-230-AC-12, field part.** `an_option_clock_reads_its_presence_bit` and `absent_drops_an_option_clock` (both S3), and `an_option_clock_has_a_presence_bit`, `a_write_makes_an_option_clock_present` and `absent_drops_an_option_clock` (all eval) cover:
  - the guard part `v >= 1` with its presence;
  - `unproved-presence` alone;
  - the reset that makes `v` present;
  - the drop, with no reset;
  - the invariant recorded together with its guard.
- **FR-230-AC-39, guard part.** `not present` is a part requiring absence, and no clock constraint.
- **FR-230-AC-41 and AC-42, field part.** Two initial states, the option shells, distinct keys.
- **The `absent` ruling.** `absent_takes_the_option_type_its_context_expects` covers equality on either side, the function body, type-mismatch and ambiguous-literal.

**Out of this slice, and recorded.** QSL-580 comment 5bba1fe4 lists the deferred 580q rows: AC-22's parameter, result, record component, union and collection positions; the AC-41/42 `Sequence<Option<Clock>>` bits; and AC-25 over an `Option<Timeout>`. The PR body's "Still to come in 580q" matches.

**Not tested in this slice:**

- **`absent` lowering.** The lowering to FR-322's `none` literal (`LiteralValue::None`) is new, but no test asserts the lowered node.
- **The reset outside the frame.** A reset `self.v = 0 ms` outside the frame should refuse `clock-write`. Only the drop's frame refusal is tested.
- **A `max` bound over an absent `Option<Clock>`.** The engine skips it through `over` → `live`, but nothing asserts that for an option place.
- **The reset-and-drop step** (SR-1721001 FND-001) has no test.
- **The `unproved-presence` locus** (AC-12, "at the read") is not asserted (SR-1721001 FND-003).
- **AC-39's `not`** is tested only in a guard. An invariant `when { not present(self.v) }` refuses today (SR-1721001 FND-002).

## Verdict

**The field part of AC-12, AC-39, AC-41 and AC-42 is covered.** The listed gaps need tests alongside the SR-1721001 fixes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Add tests for the behaviours this slice adds that no test covers. (a) `absent` lowers to the FR-322 `none` literal typed by its option node. (b) `self.v = 0 ms` outside the frame refuses `clock-write`. (c) A `max` constraint does not bound an absent `Option<Clock>`. (d) A reset and a drop of one place in one postcondition refuses, or has no successor (SR-1721001 FND-001). (e) The `unproved-presence` refusal is located at the `value(self.v)` read (SR-1721001 FND-003). (f) `present(self.v)` under `not` and `or` in an invariant `when`, and in the other positions SR-1721001 FND-002 lists, checks as a data predicate. | qsl-semantics/tests/it/clock_places.rs; qsl-eval/tests/it/timed_subject.rs; FR-230-AC-12, AC-25, AC-39 |
| FND-002 | low | The `absent` ruling has no spec text to cite yet. The PR body says the STD text is pending, while FR-230 already uses `self.v = absent` in AC-12 and "Presence bit". Once the STD-493 ruling text lands, cite it in the `absent_literal` doc and in the PR body, so that the typing rules tested here (contextual only, `ambiguous-literal` with no type, `type-mismatch` for a non-option) can be traced to a requirement. | qsl-semantics/src/check/check.rs (`absent_literal`); STD-493 |

## Dispositions

Round 1, fix commit c1387e815. The gates are those in SR-1721001's dispositions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Each item now has a passing test. (a) `absent_lowers_to_the_none_literal_of_its_option`: a `none` literal typed by its `Option<Integer>`. (b) `absent_drops_an_option_clock` also refuses a reset outside the frame. (c) `present_reads_the_bit_in_invariants_bounds_and_urgency`: a `max` bound reaches `v` only where it is present. (d) `two_writes_of_an_option_clock_conflict`: either order, and two resets. (e) The read span is asserted in `an_option_clock_reads_its_presence_bit`. (f) `present_is_a_data_predicate_beyond_a_guard_conjunct` covers `!=`/`= absent`, `not … or …` in a `when`, urgency, and the three `not-yet-implemented` positions. In addition, `the_zone_search_frees_an_absent_option_clock` covers SR-1721001 FND-004. |
| FND-002 | still-open | The PR body still says the STD-493 `absent` ruling text and citation are pending. This waits on the spec, not on this PR. Cite the text when it lands. |
