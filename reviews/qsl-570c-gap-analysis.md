---
id: SR-12268
title: "Gap analysis of QSL-570 part 2b against FR-193-AC-2..4, FR-189-AC-3/AC-5, FR-192 estimate and TC-624, TC-628, TC-640 (PR #1154)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@bf9ea7519549139b2985fd03827c6d702aad6779; spec/functional/FR-193-*.md (Behavior, AC-1..AC-4), FR-189-*.md (Tests, AC-3, AC-5), FR-192-*.md (estimate, the map); ADR-024 ST-3, RX-1; ADR-021 SYM-4/5/8 (context); qsl-analyze/tests/it/{statistical_witness_replay.rs,statistical_model.rs,prob_world.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
---
# SR-12268: Gap analysis of QSL-570 part 2b against FR-193-AC-2..4, FR-189-AC-3/AC-5, FR-192 estimate and TC-624, TC-628, TC-640 (PR #1154)

## Summary

Ticket: QSL-570. Planless. Plan completion: not assessed. Traceability comes from the trace tags in the diff. The tagged tests were run (`cargo test -p qsl-analyze --test it statistical`: 23 passed).

How the tests bind to the criteria:

- **FR-193-AC-2.** Two tests bind to it: `a_kept_witness_replays_and_each_alteration_is_caught` and `a_witness_with_another_value_or_length_is_inconclusive`. Both bindings are correct. They cover: three witnesses replaying `Reproduced`; `d = 2` refusing `invalid_runtime_input`/`invalid-value`; a changed seed giving `Inconclusive(Draw(Step))`; a doubled path probability giving `Inconclusive(Draw(PathProbability{recomputed}))` with the exact recomputed value; the verdict unchanged; a changed value and a short step list giving `Inconclusive`. The seed clause is weaker than the AC text (FND-003).
- **FR-193-AC-3, `a_witness_is_the_counterexample_of_the_qualitative_claim`.** The binding is partial. It covers the §7.1 fault-claim conversion (initial state, binding, steps, post digests, `Formula` kind, `UnknownTest`) and the undefined-witness conversion. "Replays through FR-128 and settles `refuted`" is blocked by STD-250, which was verified (S4 does not emit probabilistic packages, and `replay_model_trace` recompiles). The `M <= 2 ms` measure-claim conversion is not blocked, and it is not tested (FND-004). "`LongRun` keeps no witness" is covered by part 2a.
- **FR-193-AC-4.** Two tests bind to it: `an_undefined_witness_replays_and_a_changed_cause_is_inconclusive` and `a_long_run_undefined_witness_replays`. Both bindings are correct.
- **FR-189-AC-3, `two_symmetric_servers_run_one_test`.** The binding is partial. It checks `m = 1`, `α' = 1/10` and `N = 116` under the declaration, `m = 2` without it, and `NotSymmetric`, which settles no record. The two-initial-state Bonferroni clause is covered by part 2a. The non-invariant clause cannot be built (STD-252, SR-12267 FND-004). The orbit path across two initial states is not tested (FND-001).
- **FR-189-AC-5, `an_undecided_contract_conjunction_leaves_its_test_undecided`.** The binding is correct for the undecided-successor clause: Undecided `UndecidedSuccessor`, test and claim, 0 samples, no interval, no witness. The model really reaches `ContractUndetermined` (`6 mod (2 - v)` at candidate `v = 2`).
- **FR-192 estimate, `a_quantile_claim_reports_its_empirical_quantile`.** The binding is correct for the finite-value case: P95's 0.95-quantile is 3 ms (exact CDF 0.898 at 2 ms and 0.967 at 3 ms, with N = 23,026), and a mean claim gets `None`. The censored case is not tested (FND-002).
- **Not built, declared and ticketed:** the EN-4 manifest (STD-251, verified as a cross-repo FR-290 vocabulary gap), the FR-098 recompile (STD-250), and the weighted-fraction `h + 1` dedicated test (part 2a remainder).

## Verdict

**No major in the gap analysis.** Every tag binds to a test with real seeded oracles. No test is tautological or a stub. The gaps are untested branches of new code (orbit carry, censored quantile) and partial tags whose missing clauses are blocked and ticketed. Fix them in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The orbit rule's cross-initial-state path is not tested. `two_symmetric_servers_run_one_test` has one initial state, so `canonicalise`'s carry is the identity and only `block_key` merges. The `carry.image` branch is where a direction error would give a wrong merge, and no test makes it merge two distinct initial states. Add a fixture with two initial states that are images of each other under the class: `a` busy and `b` idle, then `a` idle and `b` busy, with an `over` binding. The four pairs should run `m = 2` orbits, `(s1,a)~(s2,b)` and `(s1,b)~(s2,a)`, each with `α' = α/2`, on the least pair. | qsl-analyze/src/statistical/model.rs:231-327; qsl-analyze/tests/it/statistical_model.rs:530-572 |
| FND-002 | medium | `Tally::quantile`'s censored handling is not tested. Nothing exercises the rank landing among censored samples (`Some(Censored)`), censored samples counting in `n`, an exact-integer `q·n` boundary (for example `q = 1/2`, `n = 4`, which should give the 2nd value and not the 3rd), or `n = 0` giving `None`. These are the parts of the FR-192 field the coder introduced. Add unit tests on `Tally` in `statistical/tests.rs`. | qsl-analyze/src/statistical.rs:322-340 |
| FND-003 | low | FR-193-AC-2's seed clause says "with the seed changed so a recomputed draw selects `1 ms`". The test accepts the first seed in 8..264 that gives any `SampledDisagreement::Step`. Assert that the disagreement is on the attempt step's `d` draw, recomputing `1 ms`, or a reordered-step disagreement passes. | qsl-analyze/tests/it/statistical_witness_replay.rs:136-153 |
| FND-004 | low | FR-193-AC-3's first clause converts a `2 ms` measure witness ("`M <= 2 ms` on every activated behaviour"). The conversion is not blocked (only the FR-128 settlement is, STD-250), but only the event and undefined witnesses are converted in the test. Convert a `WitnessValue::Measure` witness of the `Two` claim and assert its prefix and `Formula` kind. Note on QSL-570 that FR-193-AC-3 and FR-189-AC-3 stay open (STD-250, STD-252) although tags bind to them, so the matrix count does not close them. | qsl-analyze/tests/it/statistical_witness_replay.rs:253-313 |
