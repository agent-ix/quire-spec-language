---
id: SR-12164
title: "Gap analysis of quire-spec-language PR #1000: FR-311 and FR-392 forms, FR-185 and FR-186 S3 checks"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@af798d76e2f850955074e0bf48bcd75cd5ee7899; PR #1000: qsl-cst probabilistic grammar, qsl-forms probabilistic forms and dispatch, qsl-semantics check/probabilistic.rs, qsl-foundation invalid_probabilistic_model; against QSL FR-185, FR-186, FR-195, FR-311, FR-392, FR-393 (overlap), TC-620, TC-621, TC-890, TC-1077, and QSpec FR-272, FR-405, FR-407 and the shared grammar"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1000

## Summary

Ticket: QSL-568. Planless. Plan completion: not assessed. Read-only; the
criterion-to-test check was done by hand over every `#[trace]` the PR adds.
QSpec read at its current main, cited by id.

Trace tags checked, one by one:

- FR-311-AC-1 (`the_service_model_builds_its_random_reward_and_workload_forms`):
  both `RandomForm`s with pairs in order, the `RewardForm` and the three
  workload weights in order, with spans. Can fail on the AC. Correct.
- FR-311-AC-2 (`each_leading_word_builds_only_its_own_form` and the `syn`
  test `each_probabilistic_word_dispatches_to_its_one_production`): the
  variant built for each word, and each arm's one call to its own builder.
  Correct.
- FR-311-AC-3 (`a_zero_weight_and_an_incomplete_workload_build_at_s2`):
  asserts the S2 build only. See FND-001.
- FR-392-AC-1 (`the_adr_024_claims_build_their_probabilistic_forms`): the four
  body shapes and spans are asserted. Correct tag; see FND-002 for the parts
  asserted for one claim only.
- FR-392-AC-2 (forms test plus the `syn` dispatch test): names as written,
  unresolved, one call. Correct.
- FR-392-AC-3 (`mean_expected_steps_and_elapsed_build_their_bodies`): all five
  shapes and both horizon kinds. Correct.
- FR-392-AC-4 (`over_each_and_fairness_items_keep_what_was_written`): `over`,
  `Each{parameter, when}` and two fairness items with kind and granularity.
  Correct.
- FR-392-AC-6 (`the_claim_form_is_the_same_whether_or_not_the_unit_selects_the_complete_facets`):
  equal forms, spans included, under a core and a complete header. Correct.
- FR-392-AC-5, FR-185-AC-2/3/4 and FR-186-AC-2/3/4 are partly covered and
  carry TC ids only (TC-1077, TC-620, TC-621). S1 tests carry TC ids only.
  Correct.

Coverage of the S3 checks that exist: every cause (`invalid-weight`,
`repeated-support-value`, `duplicate-distribution`, `duplicate-weight`,
`state-read-in-reward`, `qualitative-threshold` in five notations,
`partial-confidence` omitted and repeated, `invalid-confidence` on each
parameter and both θ ± ι bounds, the unbounded event, `unproved-nonzero`)
has a located test, and the blanket `not-yet-implemented` refusal is tested
for all four declarations. No production code lacks an owning requirement:
`quantity-literal`, `ExactValue` and `ExprNode::Quantity` overlap FR-393
(QSL-580), as the PR declares.

Declared spec conflicts, recorded and not re-raised: a decimal literal in an
`expr` position (tests spell `rational(999, 1000)`); `ExactValue` held as
`(kind, first, second)`; `EventForm::Temporal` holding a
`TemporalFormulaForm`; FR-185-AC-2 (`-1`), FR-185-AC-4 (no `on`) and
FR-186-AC-3 (fairness under a workload) not writable as stated; no catalog
cause for a confidence range (`invalid-confidence` used).

## Verdict

PASS. No blocking gap. FND-001 is a medium trace defect, FND-002 and FND-003
are lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-311-AC-3 is tagged on an S2-only test. The AC reads "both build at S2 and are refused only at S3 (FR-185-AC-2, FR-185-AC-3)"; no FR-311-AC-3 test asserts the S3 refusal, so S3 could admit a zero weight and the tagged test would still pass. The workload half is refused today only by the blanket `not-yet-implemented` (FR-185-AC-3's `incomplete-workload` is not built). This is the same shape as FR-392-AC-5, which the PR correctly leaves at TC-1077 only. Tag TC-890 only, or add the S3 assertion under the AC tag. | qsl-forms/tests/it/probabilistic_forms.rs:234-247 |
| FND-002 | low | FR-392-AC-1 states "each with `Workload(Steady)`, its confidence items in declared order". The test asserts both for `NoFault` only; `P95` checks only that there are three confidence items, and `LongRun` and `Monthly` check neither the scheduler nor the confidence. Assert both for all four claims. | qsl-forms/tests/it/probabilistic_forms.rs:264-396 |
| FND-003 | low | The quantity-literal refusal is tested in a function body only. No test puts `5 ms` in a `temporal` clause's `holds(...)` or a state clause, and none shows a `random`/`probabilistic` declaration inside a protocol body is refused. The code paths are sound (one `Typer` gate; unit-level grammar only); the tests would keep them so. | qsl-semantics/tests/it/probabilistic_declarations.rs:355-360 |

## New findings (disposition pass 1)

Reviewed at `ed2402498e5fe74036bc2c378d6ad62c54fac195`. Every `#[trace]`
rechecked: FR-311-AC-1, FR-311-AC-2 and FR-392-AC-1/2/3/4/6 stay correct
(FR-392-AC-1 now asserts `Workload(Steady)`, `on origin` and the three
confidence items for all four claims); FR-311-AC-3 now carries TC-890 only.
The new S3 tests (`a_second_reward_of_one_name_on_one_operation_refuses`,
`a_negative_rational_weight_refuses_invalid_weight`,
`a_quantile_over_a_fraction_refuses`,
`an_event_window_is_not_refused_before_profiles_resolve`,
`huge_written_exact_values_are_read_without_expansion`) carry TC-620/TC-621
only, which is right: FR-185-AC-2/3/4 and FR-186-AC-2/3/4 each still need
typing, resolution, units or the QSL-883 limit outcome.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-185 now states that a bare weight `-1` is not an `exact-value` and S1 refuses it `invalid_syntax`. No test writes `random Server::attempt(d) ~ { 1 ms: -1 };` or a `-1` workload weight; the malformed-declaration table in the S1 tests has no signed weight. Add one row so the S1 half of that rule cannot regress. | qsl-cst/tests/it/probabilistic.rs:191-230 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c29cae523 |
| FND-002 | fixed | c29cae523 |
| FND-003 | deferred | low; not fixed in this PR, recorded under the lane's majors-only fix policy |
| FND-004 | deferred | low; new this round, recorded under the lane's majors-only fix policy |
