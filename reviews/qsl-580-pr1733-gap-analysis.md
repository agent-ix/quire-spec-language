---
id: SR-1733002
title: "Gap analysis of quire-spec-language PR #1733: a Clock payload in a union case (QSL-580q part 2)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@aec5be25f; PR #1733 against FR-230-AC-12 (union part), FR-230-AC-41 (union part), FR-230 Records/tuples/unions, EN-6, FR-360 item 9"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1733

## Summary

Ticket: QSL-580 (580q part 2). Plan completion: not assessed.

**Covered:**

- **FR-230-AC-12, union part.** `clock_places::a_union_case_clock_is_a_guarded_place` (S3) and `timed_subject::a_union_case_clock_is_live_while_its_case_holds` (eval) cover:
  - `Armed(Clock)` as the guarded place `state.Armed`, with no bit;
  - the guard part requiring the case;
  - the constructor reset;
  - the drop by `Idle`;
  - `unproved-presence`;
  - liveness following the case, and distinct keys.
- **FR-230-AC-41, union part.** `a_snapshot_names_a_union_clock_case_by_its_variant_form` covers the snapshot case member.
- **EN-6.** `zone_clock_collections::the_zone_search_drops_a_union_case_clock_with_its_case` shows time-lock freedom holds when `disarm` drops the clock and is violated without it.

**Out of this slice:** tuple `Clock` components (QSL-1168, IR-1059); `Option<Clock>` beyond a top-level field; a union case under another guard or a collection; and `clock_place` emission, the place id and `Reference<Clock>`. The PR body lists them as pending.

**Untested, though the probes pass:**

- A union with two clock cases: a switch resets one and drops the other.
- A rewrite of the same case with a new constant.
- A union write outside the frame refusing `clock-write`.
- FR-230's refusal of a whole read of a clock-bearing union (`self.state = M::Mode::Idle` in a guard, `pre(self.state)`).
- The case-guarded time invariant.

**Untested, and wrong today:** the conflicting double write (SR-1733001 FND-001).

## Verdict

**The union part of AC-12 and AC-41 is covered.** The listed tests are needed alongside the SR-1733001 FND-001 fix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Add tests for six cases. (a) Two clock cases in one union: a switch resets the target case and drops the other. (b) `Armed(2 ms)` from `Armed` resets to 2. (c) A union write outside the frame refuses `clock-write`. (d) A whole read of the clock-bearing union refuses `clock-read-outside-constraint` (a guard `self.state = M::Mode::Idle`; `self.state = pre(self.state)`). (e) `when { present(self.state.Armed) }` bounds only the armed state. (f) Two writes of the field in one postcondition refuse `conflicting-clock-write` (SR-1733001 FND-001). Probes for (a) to (e) pass. | qsl-semantics/tests/it/clock_places.rs; qsl-eval/tests/it/timed_subject.rs; probe_1733.diff; FR-230 "Records, tuples and unions" |

## Dispositions

Round 1, fix commit 742127c84. The gates are those in SR-1733001's dispositions; workspace clippy fails there (SR-1733001 FND-004).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `clock_places::a_union_field_is_written_once_and_never_read_whole` covers the conflicting double writes (same case, a case switch plus reset, two clock cases), a union write outside the frame refusing `clock-write`, and the whole-read refusals. `timed_subject::a_union_write_resets_its_case_and_drops_the_others` covers two clock cases, where a switch resets one and drops the other; a rewrite of the same case; and the case-guarded invariant. Both pass. |
