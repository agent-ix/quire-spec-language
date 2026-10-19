---
id: SR-12194
title: "Gap analysis of quire-spec-language PR #1034: FR-330 run_clause Temporal selection, FR-332 negotiation"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@39ed010e651f0d8cb7df9b60b9c77c943938c9d8; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md, spec/functional/FR-332-settle-an-infinite-trace-item-through-negotiation.md, spec/test-cases/TC-840-run-a-temporal-clause-through-the-spine.md, spec/test-cases/TC-842-settle-an-infinite-trace-item-through-negotiation.md, spec/test-cases/TC-848-a-fairness-premise-over-a-supplied-trace-is-missing.md, qsl-replay/src/spine/clause/temporal.rs, qsl-replay/src/spine/clause.rs, qsl-eval/src/temporal/observed.rs, qsl-replay/src/model_check/settle.rs, qsl-route/src/routing.rs, qsl-replay/src/spine/clause/tests/temporal.rs, tests/it/temporal_negotiation.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1034

## Summary

Ticket: QSL-510 (partial). Planless gap analysis over the PR diff. Plan
completion: not assessed. Each `#[trace]` in the diff was read against the
text of its AC at the reviewed head. The question for each was whether the
test can fail when the behaviour the AC states is wrong. No gate was run, so
no run evidence is bound.

Declared and not counted as findings (per the dispatch brief): FR-331 is not
delivered. FR-330-AC-5 is untested and correctly untagged. `TraceInput::Model`
is not implemented. The `native-run-result/2` writer refuses a temporal
disposition, and FR-312's request wire has no `temporal` selection. The
fixture uses `noop` for `inc`, and `Bounded` and `Reaches` are two units.

Bindings:
- `a_bounded_clause_over_a_finite_trace_violates_at_its_position` and
  FR-330-AC-1: correct. It asserts stage `evaluate`,
  `Violated{at: 0}`, category `violation` and exit 10, from source through
  real admission.
- `an_infinite_trace_clause_reads_a_finite_prefix_as_pending_and_a_lasso_exactly`
  and FR-330-AC-2: correct. It covers all three runs: `Pending`/
  `inconclusive`/exit 0/no truth, the lasso `0, 1` `Violated{at: 0}`, and the
  lasso `0, 1, 2` `Holds`/`success`/exit 0/`truth` true.
- `a_selection_that_cannot_run_reports_its_stage_and_cause` and FR-330-AC-3:
  correct. It covers all four cases with stage and code/cause:
  `MissingName` at `select`, `invalid_runtime_input`/`invalid-value` and
  `invalid_runtime_input`/`wrong-role-mapping` at `admit`, and `EmptyLoop`
  at `evaluate`/`refusal`/`invalid-value`.
- `a_zero_work_budget_is_incomplete_and_a_repeated_run_reports_equal` and
  FR-330-AC-4: correct, with a low gap (FND-004).
- `a_fairness_premise_over_a_supplied_trace_is_unsupported` (TC-848) and
  FR-330-AC-6: correct, with a low gap (FND-003). It asserts
  `MissingFairnessPremise` naming `weak`/`whole`/`noop`, stage `evaluate`,
  `unsupported` and exit 21.
- FR-330-AC-5: untagged, correct (declared).
- `an_infinite_trace_item_is_unbounded_and_refuses_a_bounded_request` and
  FR-332-AC-1: correct. It reads the clause's own requirement record,
  checks `Unbounded{finite_bound_available: false}`, the
  `UnboundableDomain` refusal with `invalid_runtime_input`/`invalid-value`,
  and an unchanged item count.
- `an_infinite_trace_item_routes_only_to_a_backend_negotiation_supports` and
  FR-332-AC-2: over-tagged (FND-001).
- `each_backend_result_label_maps_to_its_category` and FR-332-AC-3: correct.
  All five labels are covered, and `failed` is checked under all three
  executions. Changing any arm of `ResultLabel::category` fails it.
- `a_clause_run_over_a_trace_settles_no_item` and FR-332-AC-4: half
  backed (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `#[trace("TC-842", "FR-332-AC-2")]` claims more than the test checks. The three `route(..)` assertions pass a hand-built `Disposition` and read it back (`route(&[Disposition::Unsupported]) == [None]` twice, `route(&[Supported(c)]) == [Some(c)]`). They do not use the graph, the item or the registry, so they cannot fail on the item's settlement. Nothing asserts that the item settles `unsupported` with a warning naming `temporal-satisfaction`. Nothing asserts the bounded-only `unbounded-extent` settlement either (cause, kind, descriptor, mode). `qsl_route::routing::Disposition` has no `unbounded-extent` variant, so that cause cannot be represented. Only the candidate-set computations (empty, bounded-only, unbounded) can fail. Remove the FR-332-AC-2 tag and ticket the warned settlement, or add the settlement record and assert it. | tests/it/temporal_negotiation.rs:158-187, qsl-route/src/routing.rs:25-37 |
| FND-002 | medium | `#[trace("TC-842", "FR-332-AC-4")]` covers the AC's first half: `Reaches` holds on a lasso (`tested`), with `basis: Unavailable` and no witness. Its second half, "the accounting record for the AC-2 empty-registry request still holds the item `unsupported`", is not run. No accounting record (QSpec FR-331) is built or read, so the test cannot fail if an S6a result were joined to the item. Remove the tag until that record exists (it belongs with the FND-001 settlement work), or bind the test to a narrower AC. | qsl-replay/src/spine/clause/tests/temporal.rs:300-312 |
| FND-003 | low | FR-330-AC-6 (and TC-848 step 3) asks for cause `unsupported_projection`/`missing-fairness-premise`. The test asserts the `MissingFairnessPremise` variant, `unsupported` and exit 21, but no catalog code. The report has no code for this outcome, because the variant only documents the wire cause and the `native-run-result/2` writer refuses temporal dispositions. Assert the code when a mapping from the outcome to its code exists, or note in the test that the cause is implied by the variant. | qsl-replay/src/spine/clause/tests/temporal.rs:283-296 |
| FND-004 | low | FR-330-AC-4's repeat half asks for "any request of AC-1 to AC-3" to be run twice. The repeat list has six of the eight requests and leaves out AC-3's `ghost` and empty-loop requests. Add both to `requests`. | qsl-replay/src/spine/clause/tests/temporal.rs:265-278 |

## Verdict

NOT MERGEABLE as tagged: FND-001 is high, because the matrix counts
FR-332-AC-2 as covered when most of it can never fail. The fix is small:
remove the FR-332-AC-2 tag (and the FR-332-AC-4 tag, FND-002) and ticket the
negotiated settlement record. Once that is done the PR is mergeable on
gap-analysis grounds. FR-330-AC-1 to AC-4 and AC-6, FR-332-AC-1 and
FR-332-AC-3 are each backed by a test that fails on its AC's behaviour.

No production code in the diff is without an owning requirement.
`ResultLabel::category` has no production caller yet, but FR-332's A-5
mapping owns it and FR-332-AC-3 tests it.
