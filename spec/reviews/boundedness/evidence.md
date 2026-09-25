---
id: SR-627
title: "Evidence analysis of ADR-014 temporal, trace and boundedness architecture"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md (new); its amendments to ADR-012 §1.1, ADR-013 (O-20, O-21, S-6, Q222 table), FR-057, FR-082 and spec/spec.md, at bbe92fec on spec/17-boundedness-adr"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: reviews
---
# SR-627: Evidence analysis of ADR-014

## Summary

Round 1. Reviewed commit bbe92fec against `origin/main` (`fc27aacc`), with
QSpec at `eb4234f` as the read-only authority. ADR-014 is a design record and
has no AC rows. Its obligations are the rulings in §1 to §9, and the ticket
lists in §11 (QSL-140, QSL-42, QSL-43) that implement them. This review asks
three things of each ruling: is it verifiable, which method verifies it, and
which ticket produces the evidence.

Most rulings can be verified by test: B-1 to B-6 separation, the absent-bound
table, TR-2/TR-3/TR-4, A-2 to A-4, the §4 predicate, the §6 step 4 route guard,
the §7 `explore::Outcome` map, and N-1. But §11 names only the types each
ticket builds. It names no test for any of them. The CG-settled scenarios (1
and 3) and the replay scenario (5) have no QSL-side evidence and no owner for
it.

The FR-082 amendment matters most. AC-6 and AC-7 are rewritten, and so is
AC-3's check-time arm: a reached check-time ceiling is now
`StageFailure::Limit(LimitExceeded)` / `stage_limit_exceeded`. The tests that
back them (TC-220) still assert `resource_exhausted`, and they pass. In
`spec/tests.md`, TC-220 still reads "Passed locally". The ACs are therefore
unbacked while the build is green. The only place this is recorded is the
`spec/spec.md` index row. TC-219's rows (unknown supertype, cycle) do not
change and stay backed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-082-AC-6 and AC-7, and AC-3's check-time arm, now require `LimitExceeded` / `stage_limit_exceeded` (nesting depth, work budget). TC-220's backing tests still assert the superseded code and pass: `type_environment_model.rs::divergence_chain_past_the_ceiling_refuses_at_check_and_at_evaluation` (`refusal.code() == "resource_exhausted"`, `DeclarationCause::AncestorSteps`), `::a_deep_chain_refuses_the_admission_work_budget` (`resource_exhausted`), `::a_linear_chain_costs_admission_work_linear_in_its_flattened_slots` and `::wide_multiple_inheritance_costs_admission_work_bounded_in_its_slots` (`DeclarationCause::WorkUnits`), and `model_conformance.rs::an_ancestor_chain_at_the_configured_bound_is_admitted_and_one_longer_refuses` and `::normalization_admits_an_ancestor_path_at_the_bound_and_refuses_one_longer` (`Code::ResourceExhausted`). `spec/tests.md` still marks TC-220 "✅ Passed locally", and the FR-082 AC rows still say "Test (TC-220)" with no pending mark. Fix: mark TC-220 in `spec/tests.md` as backing pre-ADR-014 behaviour for AC-3 (check-time arm), AC-6 (past-ceiling row) and AC-7, retargeted in QSL-140, and note the same on those AC rows. List these six tests in QSL-140's §11 entry, each to assert `StageFailure::Limit` with `nesting-depth-exceeded` or `work-budget-exceeded`. `model_population.rs::binding_admission_walks_member_types_under_the_callers_ancestor_steps` (S6a, B-2) keeps `resource_exhausted` and stays backed. | FR-082-AC-3, FR-082-AC-6, FR-082-AC-7; TC-220; ADR-014 §1, §12 |
| FND-002 | medium | The FR-082 body now contradicts itself. The "Ancestor and conformance walks are bounded" paragraph still says the model checker SHALL refuse every walk with a resource-exhaustion cause (`ancestor-steps`) as a `Refused` outcome. The paragraph added after it says a check-stage walk, model normalization included, SHALL return `StageFailure::Limit`. No test can satisfy both. Fix: limit the first paragraph's resource-exhaustion and `Refused` wording to S6a population admission, and point to the new paragraph for check-stage walks. | FR-082 §"Ancestor and conformance walks are bounded" |
| FND-003 | medium | A `LimitExceeded` must carry a `Locus` (ADR-013 T-5, FR-096), plus an `actual` counter. FR-096's locus table has no row for model normalization, the conformance walk or type-environment admission. `ancestor_steps` is "read, never charged", so nothing defines its `actual` either. QSL-140's tests cannot assert a complete `LimitExceeded`. Fix: add an FR-096 locus row, or state in FR-082, which locus these walks carry (for example `Locus::Region` over the starting type's declaration span) and that `actual` is the ceiling + 1 for `ancestor_steps` and the denied spend for `work_units`. | FR-096 locus table; FR-082 amended paragraph; ADR-014 §12 |
| FND-004 | medium | §11 names types but no evidence, so no ruling has a named test. Fix: add a test list to each ticket entry. **QSL-140:** (a) no `From`/`Into`/`as` path between B-1, B-2, B-3, B-4 and B-6 types (inspection, recorded in review); (b) `explore::Outcome` → O-16 map: `Exhaustive` → success, `Bounded`/`Cancelled` → incomplete with frontier kept; (c) a `DeclaredDomain.domain` that does not parse as `ProofBound` is refused; `run_limits` rename round-trips. **QSL-42:** (d) `K<T>` and `K<T>[a,b]` produce different collection type identities under `1-draft.2`, and any other root revision is refused (N-1); (e) S6a builds an unbounded collection of any size with no cardinality refusal and stops only with `Incomplete`/`resource_exhausted` (§8, scenario 2); (f) `requirements()` returns `Unbounded` for each domain in the §4 extent rule and `Bounded` otherwise; (g) the §6 step 4 route guard: `Unbounded` settled `supported` on a descriptor without (kind, `Unbounded`) is refused `invalid_capability`/`inconsistent-candidates`. **QSL-43:** (h) A-2: an unbounded form admits only under `quire.temporal.infinite-trace/v1`, and otherwise refuses `unsupported_construct`/`expression-form` at the operator; (i) TR-3 constructor refusals (`lower > upper`, `Unbounded` under a bounded profile, `None` except for a bare infinite-trace operator); (j) TR-4 horizon overflow refuses at check; (k) A-4 as a property test: over any finite trace, S6a never yields `proved` or true for an infinite-trace formula, yields violation only on a complete bad prefix, and gives exact results over a lasso; (l) TR-2: the decimal encoding has no leading zero, and a position past the end refuses at reconstruction. | ADR-014 §1, §3, §4, §5, §6, §7, §8, §9, §11 |
| FND-005 | medium | Scenarios 1 and 3 and the §4 predicate's outcomes (`requires-bound` with a `ProofBound`, `unsupported`/`unbounded-extent` without one, Kani advertising bounded only) are settled in CG `negotiate_*` (quire-contract-codegen#86). §11 says QSL-42 and QSL-43 "settle their exit cases through" it, but names no evidence that QSL can run. This is the same gap SR-478 found for ADR-012 §7.3. Fix: name the evidence per side. QSL: `Requirements{kind, extent}` recorded, a request built with and without `ProofBound`, and the route guard (FND-004 g). CG, cited by ticket: a negotiation test covering both predicate arms, that defaults, `limits` and profiles never make a bound available, and that Kani's manifest advertises only `bounded`. Add a QSL test that the bounded re-request's result never joins the unbounded item's `request_index` (§6 step 5). | ADR-014 §4, §6, §10 scenarios 1 and 3; ADR-012 §1.1; FR-057 |
| FND-006 | medium | Scenario 5 has no owner in §11. The steps it needs: the replay facade recompiles, refuses on an interval-key or temporal-profile mismatch, re-evaluates over the lasso at `TemporalPosition`, and settles `reproduced-with-evaluated-witness` or `inconclusive`. QSL-43's list covers only the payload and the evaluator. The layer-6 `replay` refusal falls between QSL-43 and the #231/TK-01 replay work. Fix: assign it in §11, to QSL-43 or TK-01, with a test that covers agreement, interval-key mismatch, profile mismatch and disagreement. | ADR-014 §3 TR-2, §10 scenario 5, §11 |
| FND-007 | low | §8's rule is a universal negative with no named consumer, so as written it cannot be verified: "No consumer reads an `Exhaustive` exploration, a `tested` result or a bounded `proved` as proof of an item whose extent is `Unbounded`". Fix: name the consumers that could do so (`route`, the `request_index` join, O-16 aggregation) and say what check each one performs. That turns the rule into the testable cases in FND-004 (g) and FND-005. | ADR-014 §8 |
| FND-008 | low | The stage-limit scope differs between documents. ADR-014 §1 classifies only "the check-stage type environment" ceilings as B-3. The FR-082 amendment also puts model normalization in scope. Neither names the conformance-check walk (`ConformanceCheckOutcome::Refused`). QSL-140 has no single list to retarget. Fix: in §1, list every check-stage reader of `ancestor_steps` and `work_units` (normalization, conformance check, type-environment admission), matching FR-082. | ADR-014 §1, §12; FR-082 |
