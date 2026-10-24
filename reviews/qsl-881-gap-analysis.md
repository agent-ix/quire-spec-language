---
id: SR-12178
title: "Gap analysis of quire-spec-language PR #1003: FR-393 time language S1/S2 forms"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e65c0d627fea52f1cdf034401bf7cd4f91f5c5a3; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md, spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md, spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md, spec/test-cases/TC-835-s2-parses-temporal-operators-with-an-optional-interval.md, qsl-cst/src/grammar.rs, qsl-cst/src/parser.rs, qsl-forms/src/time_model.rs, qsl-forms/src/temporal_clause.rs, qsl-forms/tests/it/time_forms.rs, qsl-forms/tests/it/temporal_clause_forms.rs, qsl-semantics/src/check/temporal_clause.rs, qsl-semantics/tests/it/timed_declarations.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1003

## Summary

Ticket: QSL-881 (QSL-580a). Planless gap analysis over the PR diff. Plan
completion: not assessed. The computed matrix (`quoin matrix --json`, quoin
0.28.3, at the reviewed head) shows FR-393-AC-1, 2, 3, 5, 6 and 7 as `tagged`,
each bound to one test in `qsl-forms/tests/it/time_forms.rs`. FR-393-AC-4 is
`untagged`, as declared. No run evidence is bound, because no gate was run in
this review. Each binding below was read against the AC text.

Bindings:
- `the_rpc_declarations_build_their_time_forms_in_order_with_spans` and
  FR-393-AC-1: correct. Checks the dense source, both invariants in order with
  guard text and non-strict `3 ms`/`4 ms` bounds, `Clock` at the word's span,
  both urgency targets, `Tick{Call::send, rational(1,2) ms}`, and each
  whole-form span.
- `each_time_head_dispatches_to_its_one_production_and_the_words_stay_identifiers`
  and FR-393-AC-2: correct. A wrong dispatch builds a different variant and
  fails the test. Fields `time`/`urgent` and parameters `period`/`dense` build
  as names.
- `timed_step_and_absent_intervals_build_as_written` and FR-393-AC-3:
  correct. Checks both flag pairs, magnitudes and unit, `None` for
  `always (holds(..))`, and `Steps{0, Finite(3)}`.
- `exact_magnitudes_build_and_meaning_checks_wait_for_s3` is tagged TC-1078
  only. It covers AC-4's S1/S2 half (`0.003` held as `DecimalLiteral(3,3)`,
  the three fixtures that S3 must refuse build at S2, `[0 ms, x]` fails at
  `x`). Leaving AC-4 untagged is correct, because its S3 refusals do not
  exist yet.
- `mixed_interval_bounds_fail_at_s1_at_the_upper_bound` and FR-393-AC-5:
  correct. It checks the exact span of `3 ms`, `3` and `*`, that the unit is
  not admissible, and that S2 refuses.
- `the_rpc_unit_builds_equal_forms_under_every_facet_selection` and
  FR-393-AC-6: correct. It fails if S2 gated a time form on a facet. The
  fixture wording is noted in SR-12177 FND-002.
- `clock_is_built_in_only_unqualified_and_cannot_be_declared` and
  FR-393-AC-7: correct. It checks the built-in, the qualified named type, and
  the name-located S1 refusal for all five type-introducing declarations.
- FR-325-AC-1..AC-6 (TC-835): the existing tests are adapted through
  `steps()`. A `Timed` value fails them, so they still assert step intervals.
  FR-325-AC-3's `[0,x]`, upper-overflow and lower-overflow cases now rely on
  `interval_bounds` and are unchanged.

Out of scope as declared: FR-230 to FR-236 (QSL-580 remainder, blocked on
STD-192, QSL-882 and the zone engine), and FR-393-AC-4's S3 refusals
(`invalid-interval`, `duplicate-time-source`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test puts a timed interval on a binary operator. The new `TemporalRelation` alternative `binary(), interval(), TemporalUnary` with a `TimedInterval` (for example `holds(p) until[0 ms, 3 ms) holds(q)` or `since(1 ms, 2 ms]`) is not exercised by `time_forms.rs`, the corpus, or `temporal_clause_forms.rs`, which writes only step intervals. FR-393's behaviour applies to "a temporal operator" without limiting it to unary operators. Add one binary case to the AC-3 test. | qsl-cst/src/grammar.rs:1397-1404, qsl-forms/tests/it/time_forms.rs:283 |
| FND-002 | low | Widening the shared `Interval` production (lower `uint`, upper `QuantityLiteral` allowed) also changes protocol `within` and `await` intervals and state-graph intervals, which now rely on `interval_bounds` for both the quantity-upper refusal and the overflow refusal. No test covers a quantity or over-`u64` bound in those positions. The code path is shared and sound by reading, but nothing pins the location of those refusals. | qsl-cst/src/grammar.rs:1248, qsl-cst/src/grammar.rs:1562, qsl-cst/src/grammar.rs:1717, qsl-cst/src/parser.rs:843 |

## Verdict

PASS. Every tagged AC has a test that can fail on the behaviour that AC
describes. No AC is over-claimed, and AC-4 is correctly left untagged. The
code has no stubs: every new production has a real builder, and every new S3
construct refuses `unsupported_construct`/`not-yet-implemented` at its own
span (see SR-12177). Both findings are low coverage gaps and do not block.
