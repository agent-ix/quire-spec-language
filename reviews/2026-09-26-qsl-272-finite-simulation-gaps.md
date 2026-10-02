---
id: SR-673
title: "QSL-272 gap analysis of the FR-101 finite simulation implementation against FR-101 and TC-453 to TC-455"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@05db9adf6e2c557f30c2911ab286d02d946bdcb2; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md; spec/test-cases/TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes.md; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md; spec/test-cases/TC-455-stopped-explorations-stay-incomplete-and-unbounded-requests-require-a-bound.md; spec/test-cases/TC-390-family-outcome-and-refusal-layering.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (X-8); spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md (TR-1, TR-7); spec/tests.md; qsl-eval/src/simulation/*.rs; qsl-eval/tests/it/finite_simulation.rs; tests/it/family_outcome_layering.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-453
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-454
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-455
    type: reviews
---

## Summary

Ticket: QSL-272 (PR agent-ix/quire-spec-language#467, head 05db9adf).
This analysis traces each FR-101 AC and each TC-453 to TC-455 step to a test,
and checks whether the test's oracle is a literal value. It also checks the
FR-101 disposition table and the ADR-011 X-8 and ADR-014 TR-1/TR-7 contracts
against the code. Code-level findings are in SR-672.

AC-to-test trace (`qsl-eval/tests/it/finite_simulation.rs` unless noted):

| AC | Tests | Literal oracle | State |
| --- | --- | --- | --- |
| AC-1 | `canonical_order_sorts_successors_by_transition_identity_bytes`, `…integer_arguments…`, `…breaks_ties…`, `cancellation_frontier_keeps_fifo_order_not_key_order` | ordered digest lists | step 1's fixture does not discriminate (SR-672 FND-004) |
| AC-2 | `float64_state_keys_pin_their_digests_and_stay_distinct`, `key_equal_coalescing_merges_two_paths_to_the_same_state` | four hex digests, counts | covered; the digests recompute |
| AC-3 | `pinned_sampler_reproduces_tc_210_five_way_vector`, `…further_vectors`, `…advances_the_step_counter…`, `sample_on_a_state_with_no_successors_ends_immediately`, in-crate `tc_454_step_0_preimage_and_digest_match_the_pinned_vector`, `tc_454_one_successor_chain_computes_no_digest` | index vectors, preimage bytes, digest | covered; the vectors recompute |
| AC-4 | `seeded_sampling_reproduces_the_same_trace`, `multi_initial_sample_starts_at_trace_index_mod_m` | start digests | provenance contents not asserted (FND-005) |
| AC-5 | `sample_then_replay_round_trips_and_refuses_tampered_traces` | `ReplayError` values with digests | the shared-identity clause is vacuous (SR-672 FND-003) |
| AC-6 | `cancellation_stops_the_run_and_returns_the_frontier` | stats, frontier, category | **cause missing (FND-001)** |
| AC-7 | three limit tests, `state_limit_mid_successors…`, `exhaustive_small_graph…` | full `Outcome` values | covered |
| AC-8 | `requires_bound_refuses_before_any_transition_system_call`, `bounded_domain_request_explores` | variant, call count | weak (FND-003) |
| AC-9 | `several_initial_states…`, `duplicate_initial_states…`, `state_limit_on_initial_states_admits_in_canonical_order` | full `Outcome` values | step 4 not as written (FND-004) |
| AC-10 | `sample_request_refuses_a_generator_mismatch_before_any_call`, `sample_request_reports_empty_initial_and_the_step_limit` | error values, call count | covered |

Disposition table: each of the 23 rows is handled as stated. Rows marked
retag are kept or merged into the new tests. The two replace rows
(`canonical_order_is_the_systems_authored_successor_order`,
`multi_initial_sample_picks_the_sampler_selected_start`) are replaced.
`counter_sampler_produces_a_pinned_index_sequence` is deleted along with
`CounterSampler`. TC-439 is unchanged. One exception: the duplicate
transition-identity replay retag lost its discriminating assertion (SR-672
FND-003).

X-8 and TC-390: `qsl-eval`'s `[dependencies]` match the X-8 set exactly, and
`tests/it/family_outcome_layering.rs` asserts that set. TR-1:
`SampleProvenance{seed, trace, sampler: DefinitionRef, stopped}` replaces
`sampler_version: String`. TR-7: frontiers are digests, but `Cancelled` has
no `cause` (FND-001).

Coder deviation 2 (`GeneratorMismatch` maps to
`invalid_runtime_input`/`invalid-value`): the mapping is normative text in
FR-101 Behavior, line 128: "`GeneratorMismatch` maps to
`invalid_runtime_input`/`invalid-value`." No AC and no TC step covers it.
It is a gap in this PR because the PR's Status paragraph says the
requirement is implemented. It is also a spec traceability gap, because a
normative sentence has no AC.

## Verdict

**GAPS FOUND.** FR-101-AC-6 is not implemented (FND-001). The normative
`GeneratorMismatch` catalog mapping is not implemented (FND-002). Several tests
pass without discriminating what their TC step asks (FND-003, plus SR-672
FND-003 and FND-004). The remaining gaps are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-101-AC-6 has no implementation and no oracle for its `cause` clause. `Outcome::Cancelled` has no `cause` field, and neither cancellation test asserts `CatalogCode::new("cancelled", "caller-cancelled")`. ADR-014 TR-7 names `Cancelled{stats, frontier, cause}`. tests.md marks TC-455 "Implemented", and FR-101's Status says the cause is carried. TC-455's own Status ("The cancellation cause … not implemented") is the accurate one. Fix: SR-672 FND-001, plus an equality assertion on `cause` in both cancellation tests. | qsl-eval/src/simulation/explore.rs:90-96; qsl-eval/tests/it/finite_simulation.rs:385-416,611-647; spec/tests.md:226-228 |
| FND-002 | medium | FR-101 Behavior says "`GeneratorMismatch` maps to `invalid_runtime_input`/`invalid-value`". No code implements the mapping: `NotSimulated` has no `CatalogCoded` impl or equivalent, and no test covers it. `qsl-route/src/request.rs:174` already follows this pattern. Failure: a caller surfacing a sampler mismatch has no catalog code to report, and the FR-101 Status overclaims. Fix: implement `CatalogCoded for NotSimulated` (at least the `GeneratorMismatch` arm), and add a unit test with the literal code. Separately, FR-101 should carry the mapping in an AC (AC-10) and in TC-454 step 8, so the sentence is traceable. | qsl-eval/src/simulation/not_simulated.rs:18-45; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:128 |
| FND-003 | medium | The TC-455 step 4 test does not check what the step states. (a) The loop over `Limits` runs twice on the same value: `generous_limits()` is already `usize::MAX` in every field, so the "raising exploration `Limits` does not change a `RequiresBound` result" clause of AC-8 is never exercised with a smaller `Limits`. (b) It asserts only `requires_bound.domains.len() == 1`, not the domain's key (`WireNodeId [7;32]`) and kind `Integer`, which AC-8 and TC-455 require. (c) The `position_limit` 0 case asserts `Extent(_)`, not a node-count stage limit. Fix: make the first iteration tight (for example all ones), assert the key and kind literally, and match the `ClassifyFailure` stage-limit variant. | qsl-eval/tests/it/finite_simulation.rs:857-912 |
| FND-004 | low | TC-453 step 4 is not implemented as written. The step lists three distinct initial states in descending key order, repeats one, and uses `max_states` 3 and `max_depth` 0. It expects `Bounded` at `Limit::Depth`, with the three digests in ascending order. The mapped tests list their states in ascending order (`"0","1","2"` and `"0","0","1"`), use generous limits, and assert `Exhaustive` counts only, so no frontier order is checked. Step 8's test does cover ascending admission order, so this is low. | qsl-eval/tests/it/finite_simulation.rs:331-379 |
| FND-005 | low | AC-4 and TC-454 step 5 require the provenance to record the seed, the trace index and the supplied `DefinitionRef`. The test only checks `assert_ne!` on provenance after a seed change and after a trace-index change. It never asserts `provenance.sampler == sampler_ref()`, `seed == 42` or `trace == 0`. TC-454 step 7 also asks for the step-5 traces to be replayed, which the test does not do. Fix: assert the provenance fields literally and replay `first`. | qsl-eval/tests/it/finite_simulation.rs:1236-1298 |
| FND-006 | low | `cancellation_frontier_keeps_fifo_order_not_key_order` is tagged `#[trace("TC-453", "FR-101-AC-6")]`. TC-453's scope is AC-1, AC-2 and AC-9, and the FIFO-within-a-level rule it checks is an AC-1 clause. Fix: tag it `FR-101-AC-1`. | qsl-eval/tests/it/finite_simulation.rs:385-387 |
| FND-007 | low | Status drift. `spec/tests.md` marks TC-453 to TC-455 "✅ Implemented", but each TC file's own `## Status` still reads "🚧 Planned (QSL-272)". FR-101's Status claims the `Cancelled` cause and the full `NotSimulated` behaviour, which FND-001 and FND-002 contradict. Fix: once FND-001 and FND-002 land, update the three TC Status sections to match. | spec/test-cases/TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes.md:77-81; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md:80-85; spec/test-cases/TC-455-stopped-explorations-stay-incomplete-and-unbounded-requests-require-a-bound.md:56-60; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:280-299 |

## Dispositions

Disposition pass at 54732520 (rebased onto main with #465 / FR-096; fixes in
e8c5f6c7 and 54732520).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e8c5f6c7: `cause` is implemented and asserted literally in `cancellation_stops_the_run_and_returns_the_frontier` and in `cancellation_frontier_keeps_fifo_order_not_key_order`. |
| FND-002 | fixed | e8c5f6c7 and 54732520: `CatalogCoded for NotSimulated`. `GeneratorMismatch` gives `invalid_runtime_input`/`invalid-value`, asserted in the TC-454 step 8 test. `catalog_fields` follows the FR-096 key table: `Extent(Limit)` delegates to `LimitExceeded` (`kind`/`bound`/`actual` row), and every other variant returns `None`, since no key-table row exists for `invalid-value` or `runtime_invariant`. No new code or field is invented. See R1-FND-002 for the `RequiresBound` mapping. |
| FND-003 | fixed | e8c5f6c7: the first iteration is tight (1/1/1) and the second generous. The test asserts domain key node `[7;32]`, path `[]` and `DomainKind::Integer`, and asserts `Extent(ClassifyFailure::Limit)` with `LimitKind::NodeCount` and zero calls. |
| FND-004 | fixed | e8c5f6c7: `several_initial_states_in_descending_order_stop_bounded_at_depth_zero` lists `c,b,a,a` with `max_states` 3 and `max_depth` 0, and expects `Bounded` at `Depth` with frontier a,b,c. |
| FND-005 | fixed | e8c5f6c7: the test asserts provenance seed 42, trace 0 and `sampler == sampler_ref()`, and `replay(&system, &first) == Ok(())`. |
| FND-006 | fixed | e8c5f6c7: the test is retagged `#[trace("TC-453", "FR-101-AC-1")]`. |
| FND-007 | fixed | e8c5f6c7: TC-453 to TC-455 Status now reads Implemented, and the FR-101 Status matches the code. |

New findings, round 1:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| R1-FND-002 | medium | `NotSimulated::RequiresBound` gets catalog code `invalid_runtime_input`/`invalid-value` (category refusal), and the FR-101 amendment writes that into Behavior. ADR-014 §4 and QSpec FR-181 treat `requires-bound` as a per-item negotiation disposition that the caller answers with a bounded request, not as a malformed input, and "`requires-bound` is never an evaluator outcome". qsl-route keeps it as a disposition distinct from `invalid-request` (`qsl-route/src/lib.rs:21`). The catalog has no `requires-bound` code. The mapping reuses an existing code, so it is not an invented one, but a `RefusalRecord` built from it would report "invalid value" for a request that is valid and only needs a bound. The ADR-014 TR-2 analogy the amendment cites is about an out-of-range trace position and fits `EmptyInitial`/`KeyEncoding`, not `RequiresBound`. Nothing reads `NotSimulated::catalog_code` today except one test, so this is non-blocking and needs a leader ruling: either keep `RequiresBound` out of the refusal mapping (for example `CatalogCoded` on a narrower type, or a spec-owned disposition code), or record the choice in FR-101 as QSL's own. | qsl-eval/src/simulation/not_simulated.rs:55-80; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:143-150 |
| R1-FND-003 | low | FR-101-AC-11 is compound: key-encoding refusal plus "`GeneratorMismatch`'s catalog code". It is traced to TC-453, but the catalog-code assertion lives in the TC-454 step 8 test tagged `FR-101-AC-10`, and TC-454's Expected Results for step 8 do not mention the code. No test covers `catalog_code`/`catalog_fields` for the other variants, including the `Extent(Limit)` delegation that FR-096-AC-7 would require. Fix: move the code sentence into AC-10, add it to TC-454 step 8's expected results, and add one assertion on `Extent(Limit).catalog_fields()` giving `kind`/`bound`/`actual`. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:250; qsl-eval/tests/it/finite_simulation.rs:1552-1590 |

## R1 Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| R1-FND-002 | fixed in ce67367b | `NotSimulated` no longer implements `CatalogCoded` (that trait is documented as always `Category::Refusal`); `catalog_code`/`catalog_fields` are inherent `Option`-returning methods, `None` for `RequiresBound`. Neither the diagnostics catalog nor `qsl-route`'s `Disposition` types define a requires-bound code (`qsl-route`'s own `Disposition::RequiresBound` carries none either), so none is invented. FR-101:143-158 states the rule; the TR-2 analogy now covers only `EmptyInitial` and `KeyEncoding`. |
| R1-FND-003 | fixed in ce67367b | `GeneratorMismatch`'s catalog-code sentence moved from AC-11 to AC-10 and into TC-454 step 8's expected results. `not_simulated_catalog_code_and_fields_cover_every_variant` asserts `catalog_code`/`catalog_fields` for every `NotSimulated` variant, including `Extent(Limit)`'s delegation to `LimitExceeded`'s `kind`/`bound`/`actual` fields. |
