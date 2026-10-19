---
id: SR-12214
title: "Gap analysis of QSL-920 part 1 FR-185 S3 for random, reward and workload declarations against FR-185, TC-620, TC-621 (PR #1067)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f60a3bdb3; spec/functional/FR-185-*.md (Exact values, Random parameters, Workloads, Rewards, AC-1..AC-4), spec/test-cases/TC-620-*.md, spec/test-cases/TC-621-*.md (context); qsl-semantics/src/check/probabilistic/{model,exact}.rs, qsl-semantics/tests/it/probabilistic_declarations.rs, qsl-package/src/emit/tests/probabilistic.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-620
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-621
    type: reviews
---
# SR-12214: Gap analysis of QSL-920 part 1 FR-185 S3 for random, reward and workload declarations against FR-185, TC-620, TC-621 (PR #1067)

## Summary

Ticket: QSL-920. Planless. Plan completion: not assessed. The traceability below was read from the trace tags in the diff. A `quoin matrix --json` run did not finish within the review window, so its output is not used here.

How the tests bind to the criteria:

- FR-185-AC-1: untagged. The test is `the_service_model_checks_and_records_its_declarations`, tagged `TC-620` alone. It substitutes the fixture: `d` is a `Millis` integer 1..12 with integer support `1, 3, 12`, and `duration` is asserted to be `Int`. AC-1 requires `1 ms, 3 ms, 12 ms` on a `Quantity<Time>` parameter and `duration: Quantity<Time>`.
- FR-185-AC-2: untagged. Its rows are covered by `random_parameter_variants_refuse_naming_the_parameter`, `a_precondition_that_reads_a_random_parameter_refuses_at_the_read`, `exact_values_are_bounded_as_written` and `a_bare_negative_weight_is_refused_at_s1`, each tagged `TC-620` alone. Three rows differ from the AC as written:
  - `true` is tested against an integer parameter, not `Quantity<Time>`.
  - The precondition is `d < 5`, not `d < 5 ms`.
  - Over-ceiling values assert `not-yet-implemented`, not the limit refusal (SR-12213 FND-002).
- FR-185-AC-3: `workload_variants_refuse_naming_the_workload_and_the_operation`, tagged `TC-620, FR-185-AC-3`. The binding is correct, and all four rows are asserted, with cause and located text.
- FR-185-AC-4: `reward_variants_refuse_naming_the_reward`, tagged `TC-620, FR-185-AC-4`. The binding is correct, and all three refusals plus `cost` checking are asserted.
- TC-621 (FR-186): out of this PR's scope. Claims stay `not-yet-implemented` past their form checks, and the existing TC-621 tests are kept except for the two in FND-003.

Leaving AC-1 and AC-2 untagged is truthful: the matrix does not credit them, since the Quantity rows are not built.

## Verdict

**PASS for the PR's own scope; gaps at ticket scope.** No gap analysis finding blocks merge. FND-001 is real unfinished QSL-920 scope: the ticket names `Quantity<Time>` parameters as needed by the `Service` fixture. It must stay open on QSL-920, or get its own ticket, before QSL-920 closes. The merge blocker for this PR is SR-12213 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-185-AC-1 and the Quantity rows of AC-2 and AC-4 are not built. Domain-package intake admits no `Quantity<…>` parameter type: nothing under `qsl-semantics/src/model` maps one. So `Server::attempt(d: Quantity<Time>)`, its `1 ms` support, `duration: Quantity<Time>`, `true` outside a quantity type and `d < 5 ms` cannot be checked. The tests stand in with an integer `Millis`. QSL-920's description lists "`Quantity<Time>` parameters (needed by the `Service` fixture)" as part of this work. Record it as remaining work on QSL-920 (or a child ticket), so part 2 (FR-187 over `Service`) does not inherit the substitution. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md (AC-1, AC-2, AC-4); qsl-semantics/tests/it/probabilistic_declarations.rs:140-204, 337-397 |
| FND-002 | low | AC-1's support values are checked weakly. The test checks `values.len() == 3` and that the `Debug` strings of values 0 and 2 contain `'1'` and `"12"`. A support recorded as `1, 7, 12` or `10, 3, 12` would pass. Assert the three values exactly (`Value::Integer(1)`, `3`, `12`), as the probabilities are. | qsl-semantics/tests/it/probabilistic_declarations.rs:350-359 |
| FND-003 | medium | Two TC-621 regression tests were deleted, though the code they cover is unchanged. One is the claim half of `huge_written_exact_values_are_read_without_expansion`: `with alpha decimal(1, 4294967295), …` must be read without expansion (TC-621 step 4). The other is `an_event_window_is_not_refused_before_profiles_resolve`, which covers four unbounded and nested event windows refusing only `not-yet-implemented`. The claim path still runs `magnitude`/`ratio` with `MAX_EXPANDED_SCALE`, which nothing now tests against a 2^32 scale. Restore both. | qsl-semantics/tests/it/probabilistic_declarations.rs (origin/main lines 231-249, 404-417) |
| FND-004 | low | `a_checked_probabilistic_model_refuses_emission` (the S4 refusal) is tagged `TC-620`, but TC-620 has no S4 step and FR-185 does not state the S4 refusal. The behaviour is right; the binding claims TC-620 coverage it does not provide. Tag it to the requirement that owns S4 emission scope, or leave it untagged with a doc comment naming the decision. | qsl-package/src/emit/tests/probabilistic.rs:30-48 |
| FND-005 | low | Spec gap: FR-185 "Workloads" makes a workload a named node with an identity that a claim binds by name, but gives no cause for two workloads of one name (on one model or across models). Without one, the code admits both (SR-12213 FND-004). Ask the planner for a cause in FR-185, a QSL-proposed one like `duplicate-reward` if QSpec FR-272 has none, plus an AC-3 row. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md (Workloads) |
| FND-006 | low | Spec gap: FR-185 bounds each exact value as written, but not the exact arithmetic S3 does itself: the sum of a distribution's weights and each probability. FR-203's `max_rational_bits` covers only the exact engine. The result is SR-12213 FND-001, where in-limit weights produce a sum of n·65,536 bits. State that the rational-bits ceiling (or the checking work budget) also bounds S3's derived probabilities, with the same limit refusal, and add an AC-2 row. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md (Exact values, Random parameters) |
