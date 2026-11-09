---
id: SR-20261109172912000058
title: "Gap analysis of acyclic until and always certificates"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1826 PR2625; FR-201-AC-15 through AC-18, FR-282-AC-14 and AC-15; changed implementation and tests"
review_set: subset
---

## Summary

Scoped manual requirements-to-tests, reverse-ownership and stub review at frozen 65363e360fb3054ca4adf3a3b64a4dc2926d9f45. No scoped implementation gap found; no repository-wide matrix or qualification claim is made.

Plan completion: not assessed

## Verdict

**CONDITIONAL** — the requested scoped manual review is clean; computed repository-wide matrix, aggregate checks and the wider assurance campaign were excluded by dispatch. Engineering delivery is mergeable within this scope. Broader objective, cyclic, timed, fair, lower/upper, reward and new refutation capabilities remain explicitly unsupported or separately owned rather than claimed complete.

## Assurance Context

No selected AssuranceProfile found in the repository guidance or dispatch. AGENTS.md separates engineering delivery from assurance. Quoin 0.29.1 and the installed SpecReview schema are available; full computed matrix was deliberately not invoked per dispatch. Active exception: scoped manual gap analysis and existing focused logs replace a repository-wide audit for this PR. Other unavailable selected context: none identified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Trace and behavior evidence

All new state-predicate integration tests carry actual #[trace("TC-636", ...)] attributes. FR-201-AC-15/16 and FR-282-AC-14 are exercised by until_and_always_check_exact_direct_minimum_maximum_and_workload_values through native S1-S3 admission, analyzer proof generation and a fresh core-only executable. Both threshold sides, minimum/maximum and resolved workload have exact value assertions and beyond-side threshold rejection. The test checks byte round trip and complete binding-bearing FR-196 key components.

until_precedence_and_infinite_terminal_stutter_stop_at_real_boundaries and initial_terminal_predicates_use_infinite_stutter assert initial B precedence, initial failure, unresolved until terminals, nonterminal always continuation, prefix failure and terminal P. stopped value coverage excludes states beyond boundaries. altered_objective_binding_keys_values_ranks_and_threshold_do_not_prove covers FR-201-AC-17 with changed event, profiles, target/failure sets, optimization, complete keys, actual source/formula and threshold, missing coverage and broken ranks. Existing closed-profile tests retain reachability bytes and verdicts.

FR-201-AC-18 and FR-282-AC-15 have original_cycles_remain_unsupported_even_behind_initial_success_or_failure, caller_limits_and_cancellation_never_return_truth_or_evidence, and every_independent_checker_poll_preserves_cancellation_including_graph_expansion. The latter cancels at every observed poll for both requested and deadline causes. failed_state_predicate_bounds_have_no_old_eventual_refutation confirms unsupported native refutation rather than routing old witnesses. Existing source/roundtrip refusal tests cover the retained admission restrictions.

Owner solver regressions until_minimum_retains_absorbing_failure_in_qualitative_analysis and until_avoidance_cannot_cross_an_already_reached_target carry TC-633/FR-201-AC-15 tags and assert actual rational outcomes. Reviewed changed tests in bounded_closure.rs, bounded.rs and roundtrip.rs for preserved bounded and reachability paths. No source stub, ignored new test, test-only semantic bypass or unowned capability was found. New event/profile ownership is FR-201, source facade ownership FR-282, reductions FR-198, product identity FR-196, and infinite-terminal semantics FR-126/ADR-018/ADR-028.

Accepted focused logs are the six logs enumerated in SR-20261109172912000057; 25 fresh-process, 39 engine, 13 adapter and 17 core tests passed. No full matrix completion percentage is inferred from trace tags or those passing suites.

## Examined units

Each AC was read in full; excerpts identify the reviewed units.

```yaml
scope:
  - id: FR-201-AC-15
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'A reached B-and-not-A state has value 1; initial B accepts at 1, initial neither-A-nor-B at 0, and terminal A-and-not-B at 0.'
  - id: FR-201-AC-16
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'A prefix not-P has value 0 despite later P; initial terminal P has value 1 and initial not-P value 0.'
  - id: FR-201-AC-17
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'Changed event/optimization/targets/failures refuses objective mismatch; a foreign S3 binding or altered complete key is not an alias.'
  - id: FR-201-AC-18
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'No state-predicate proof document is treated as a refutation witness.'
  - id: FR-282-AC-14
    path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
    role: examined
    excerpt: 'Existing reachability/v1 bytes and verdicts are preserved.'
  - id: FR-282-AC-15
    path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
    role: examined
    excerpt: 'No analyzer/provider linkage, emitted-package bytes, fabricated replay request, solver labels, local state ids or new witness wire are used.'
  - id: FR-201
    path: spec/functional/FR-201-check-a-probability-certificate.md
    role: examined
    excerpt: 'The checker SHALL solve no equations and reuse no EN-5 graph, labels or complement result as trusted evidence.'
  - id: FR-196
    path: spec/functional/FR-196-build-the-probabilistic-product.md
    role: context_only
    excerpt: 'Build the probabilistic product'
  - id: FR-198
    path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
    role: context_only
    excerpt: 'Decide unbounded reachability and expected rewards'
  - id: FR-126
    path: spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md
    role: context_only
    excerpt: 'Check a temporal clause over every behaviour of a model'
  - id: ADR-028
    path: spec/decisions/ADR-028-exact-probabilistic-engine.md
    role: context_only
    excerpt: 'Exact probabilistic engine'
  - id: ADR-018
    path: spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md
    role: context_only
    excerpt: 'Temporal properties over every behaviour'
```
