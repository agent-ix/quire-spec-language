---
id: SR-12318
title: "Gap analysis of QSL-570 part E against FR-057, FR-075-AC-10..14, FR-280-AC-4, FR-185-AC-9, FR-189-AC-3/AC-15, FR-192-AC-6 (PR #1204)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@65a31e407; spec/functional/FR-057-*.md, FR-075-*.md (evidence kinds, AC-10..AC-14), FR-280-*.md (AC-4), FR-185-*.md (Random parameters, AC-9), FR-189-*.md (AC-3, AC-15), FR-192-*.md (quantile, AC-6); qsl-route/src/lib.rs, qsl-route/tests/it/evidence_kinds.rs, qsl-analyze/src/statistical/{manifest.rs,tests.rs}, qsl-analyze/tests/it/{statistical_manifest.rs,statistical_model.rs}, qsl-semantics/src/check/probabilistic/model.rs, qsl-semantics/tests/it/probabilistic_declarations.rs, tests/it/composed_admission_stages.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
---
# SR-12318: Gap analysis of QSL-570 part E against FR-057, FR-075-AC-10..14, FR-280-AC-4, FR-185-AC-9, FR-189-AC-3/AC-15, FR-192-AC-6 (PR #1204)

## Summary

Ticket: QSL-570. PR quire-spec-language#1204, branch qsl-570e-impl, head 65a31e407. The scope is `git diff origin/main...HEAD`. This analysis is planless, so plan completion was not assessed. Traceability comes from the trace tags in the diff. The spec is the STD-251 (#1185) and STD-252 (#1186) text at the PR head.

Measured on the PR head:

- Every new test passes. `cargo test -p qsl-route -p qsl-analyze -p qsl-semantics --test it` with the new test names ran 14 tests, all passed.
- Mutation runs, `cargo test -p qsl-route`:
  - Each of these mutations is caught by a test: an unknown label skipped instead of refused; an absent or an empty set admitted; a conflict check that ignores the evidence set; `statistical` and `exact` substituting for each other; an absent or unknown item evidence matching a backend; a non-probabilistic backend that must supply evidence labels; item evidence not kept on the `CandidateSet`.
  - Three mutations survive: labels kept on a non-probabilistic backend, a repeated label refused, and evidence judged before `domains` (FND-004).
- Mutation run, full `cargo test -p qsl-semantics` (733 + 501 + 23 tests): with the new reference-support guard removed, every test still passes (FND-001).

How the tests bind to the criteria:

- **FR-075-AC-10 to AC-14.** The tests in `qsl-route/tests/it/evidence_kinds.rs` bind correctly and their oracles are real: the exact retained set, refusal causes keyed by identity, exact-kind candidacy both ways, the named-backend empty set against the unknown-backend marker, and the evidence-difference conflict in both orders.
- **FR-280-AC-4.** `statistical_manifest.rs` binds correctly. The "FR-331 manifest reader" step is the driver's, so the test enters at FR-288's conversion, which is the same entry FR-280-AC-1 uses.
- **FR-057-AC-11 and AC-13.** These bind correctly at the `families` table and at `fr290_family` (FND-005).
- **FR-189-AC-3.** `two_symmetric_servers_run_one_test_under_any_operation_weights` binds correctly.
- **FR-189-AC-15.** Partial (FND-003, FND-006).
- **FR-192-AC-6.** Partial (FND-002).
- **FR-185-AC-9.** Wrong (FND-001).

## Verdict

**No major finding.** The registry admission and routing semantics are implemented and pinned by tests. No test substitutes `exact` for `statistical`, and the conflict and refusal paths are each caught by a mutation.

The STD-252 tags overclaim. FR-185-AC-9's refused clause is never exercised. FR-192-AC-6 has four untested clauses. FR-189-AC-15's quantity clause is recorded as Remaining work, but the reason given is stale.

The PR body's "Landed" list should match what is tested. Fix FND-001 to FND-004 in this PR. FND-005 and FND-006 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-185-AC-9 test never reaches the new reference guard. `a_reference_valued_random_parameter_refuses_value_outside_type` lists the integer `3` for the `Server`-typed parameter `target`. That refusal comes from the existing `IllTyped => None` path. The AC's refused case is a reference literal naming a `Server` key. S3 has no reference-literal form (see `qsl-semantics/tests/it/symmetric_population.rs:7`), and the typer never yields a `Literal(Value::Reference)`, so the `.filter(..)` guard cannot fire. With the guard removed, all of `cargo test -p qsl-semantics` still passes. The test's doc comment ("whatever support value is listed") and the PR body ("FR-185-AC-9 landed") claim more than was tested. Fix: tag the test to the ill-typed clause it actually covers. Record FR-185-AC-9's reference-literal clause as Remaining work, naming the ticket that adds the reference literal (the same gap FR-150-AC-3 records). Drop "landed" from the PR body. | qsl-semantics/tests/it/probabilistic_declarations.rs:590-625; qsl-semantics/src/check/probabilistic/model.rs:806-815 |
| FND-002 | medium | `the_verdict_carries_the_least_favourable_tests_quantile` covers three clauses of FR-192-AC-6: `Value(2)`, `Censored`, and a probability bound carrying no `quantile`. It does not test four others: a quantile claim whose least favourable test has no estimate carries no `quantile`; the mean-of-a-fraction verdict carries none; the `LongRun` verdict carries none; and `quantile 1/2 of (M) >= 3` over estimates `2/5` and `9/20` gives the verdict `estimate` `9/20`. TC-627 step 5 names all of them. Add them to the test. | qsl-analyze/src/statistical/tests.rs:2260-2333 |
| FND-003 | medium | FR-189-AC-15's `Quantity<ms>` clause is untested, and the Remaining-work reason given for it is stale. The PR says the fixtures cannot type `Quantity<Time>`. In fact QSL-946's `Quantity<u>` type form (c33f6a867) is on this branch, and a quantity reward already types at S3 (the `LATENCY = 1 ms` fixture, probabilistic_declarations.rs:1033). What actually blocks the per-sample values `1 ms`, `3 ms`, `2 ms`, `5 ms` is the model sampler: FR-120 has no encoding for a quantity-typed parameter value (`prob_world.rs:7-11`, STD-113). The clause also cannot be met through the stub seam at all, because `Sample::Measured` carries a bare `Rational`. The unit property lives in `model.rs`'s `magnitude` (`Value::Quantity`), which no qsl-analyze test reaches. Fix: state the real blocker and its owning ticket in Remaining work. If the sampler accepts it, cover `magnitude`'s `Value::Quantity` arm with a constant quantity reward. | qsl-analyze/src/statistical/tests.rs:2172-2240; qsl-analyze/src/statistical/model.rs:640-651 |
| FND-004 | medium | The three rules this PR adds to FR-057 have no test. Each mutation of one survives `cargo test -p qsl-route`: (1) evidence labels supplied to a non-probabilistic backend are ignored, so the descriptor holds none (mutated to keep them: still green); (2) a repeated label is one member (mutated to refuse: still green); (3) evidence is judged after pairs, `domains` and `fairness_kinds` (mutated to judge it before `domains`: still green). Add one case for each, as routing.rs does for the `domains` ordering. | qsl-route/src/lib.rs:253-260,300-305,604-618; spec/functional/FR-057-admit-shared-capability-kinds.md:286-291 |
| FND-005 | low | FR-057-AC-11 says a `probabilistic-satisfaction` or `schedulability` request on a current declaration is inapplicable, names the declaration family, and leaves the body reaching its family checker. Only the `families()` table is tested for the two new kinds. No request of either kind goes through the composed linker. The path is the same one the `finite-replay` case exercises, so the risk is low. | tests/it/composed_admission_stages.rs:542-568 |
| FND-006 | low | FR-189-AC-15 says the no-sample quantile test runs "through the `BehaviourSampler` seam". The test instead builds it directly with `test_result(.., &Tally::new(), ..)`, so the run loop's no-sample path is not exercised for `quantile`. | qsl-analyze/src/statistical/tests.rs:2228-2239 |
