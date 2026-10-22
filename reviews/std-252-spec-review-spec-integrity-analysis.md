---
id: SR-13950
title: 'Spec review of PR #1186: quantile field and a buildable symmetry case (STD-252), spec-integrity analysis'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@91e502f0e; git diff origin/main...91e502f0e (PR #1186, branch std-qsl570-252, merge base 48b47ab99): spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md, spec/test-cases/TC-627-statistical-results-settle-as-measured.md; ticket STD-252'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-189
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-624
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-627
  type: reviews
---

## Summary

Ticket: STD-252. PR: quire-spec-language#1186, head 91e502f0e, based on main (not stacked). Method: spec-review with the spec-integrity sub-analysis. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

The PR adds `quantile: Option<MeasureValue>` and `MeasureValue { Value(Rational), Censored }` to FR-189's `TestResult` and FR-192's `StatisticalVerdict`. It states the nearest-rank rule and the Censored case, and adds FR-189-AC-15, FR-192-AC-6, TC-624 step 13 and TC-627 step 5. It states that both RX-1 invariance conditions always hold, and rewords FR-189-AC-3's second clause into a buildable case.

Gates at head: `make check-spec-validation` exit 0, with no warning on any touched file; `tools/check-index-completeness.sh` exit 0. The diff touches nothing under `docs/` and no test-parsed vector table (the only spec file a test includes is FR-092). The PR is QSL-only, so the QSpec gates do not apply.

Quantile examples recomputed by script (sorted list, rank `⌈q·n⌉`, censored after every finite value): 1,3,2,5 at q=1/2 gives rank 2 and Value(2); 1,3,2,C at q=9/10 gives rank 4 and Censored; 1,3,2,C at q=1/2 gives rank 2 and Value(2). All three match FR-189-AC-15 and TC-624 step 13. The rule agrees with ADR-024 WA-3 and QSpec FR-410 (`⌈q · n⌉`-th smallest).

RX-1 reasoning checked. Workload weights are per operation (FR-185 Workloads; QSpec FR-405 Workloads), and FR-187 splits an operation's weight uniformly over its enabled scheduled identities (`1 / |E_o(s)|`), so the workload is invariant under any object permutation. A support over non-reference values is invariant too. The code backs "never references": S3's support check accepts only a checked literal or a coerced integer (qsl-semantics/src/check/probabilistic/model.rs:658-672) and refuses everything else `value-outside-type`, and lowering refuses a reference literal `UnbuiltLiteral` (qsl-semantics/src/check/lowering.rs:4462-4470). The spec does not state that rule in the place it cites (FND-001).

Lead defaults: `MeasureValue{Value, Censored}` keeps "censored" apart from "no quantile", and AC-15 and AC-6 can be built with it, apart from the unit clause (FND-002). With no reference-valued support defined, AC-3 is buildable. ADR-024 RX-1's reference-typed clause becomes vacuous, as the PR body says.

IDs: FR-189-AC-15, FR-192-AC-6 and TC-624 step 13 are new and unique. Of the other open PRs, only #1185 (std-qsl570-251) touches FR-189, FR-192 and TC-627. It adds no AC to FR-189 and none to FR-192 (it rewords FR-192-AC-4), so the ids do not collide. Its hunks are separate.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189 `TestResult.quantile`, `MeasureValue` | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:82-87 | examined |
| FR-189 Symmetry bullet (added sentences) | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:111-121 | examined |
| FR-189 quantile bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:153-159 | examined |
| FR-189 least favourable test bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:199-202 | examined |
| FR-189-AC-3 | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:237 | examined |
| FR-189-AC-15 | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:249 | examined |
| FR-192 `StatisticalVerdict.quantile` | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:60 | examined |
| FR-192 `estimate` bullet | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:119-124 | examined |
| FR-192 `quantile` bullet | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:125-128 | examined |
| FR-192-AC-6 | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:146 | examined |
| TC-624 scope, fixtures, steps 3 and 13, expected results 3 and 13 | spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:15,19,22,34,42,52 | examined |
| TC-627 scope, step 5, expected result 5 | spec/test-cases/TC-627-statistical-results-settle-as-measured.md:15,25,35 | examined |
| FR-185 random parameters, workloads | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | context_only |
| FR-187 `weighted_steps` | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:88-93 | context_only |
| ADR-024 RX-1, SV-2, WA-3 | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:152,164,200 | context_only |
| QSpec FR-405 random parameters | quire-specification spec/functional/analysis/FR-405-declare-probabilistic-models.md:46-64 | context_only |
| QSpec FR-410 `estimate` row, AC-2 | quire-specification spec/objects/interfaces/FR-410-statistical-verdict.md:38,96 | context_only |
| S3 support literal check; lowering literal | qsl-semantics/src/check/probabilistic/model.rs:658-672,800-825; qsl-semantics/src/check/lowering.rs:4441-4470 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-189's new symmetry sentence cites FR-185 for "a random parameter's support values are literals or integers, never references", but FR-185 does not say that. It requires only that each value be "of the parameter's type", and QSpec FR-405 says "a literal of the parameter's declared type". The exclusion exists only in code (the S3 support-literal check refuses a non-literal value `value-outside-type`; lowering refuses a reference literal `UnbuiltLiteral`), and ADR-024 RX-1 still describes a reference-typed random parameter into a symmetric population. So the unconditional "both conditions hold for every declaration EN-4 admits" has no stated basis in the spec. A reader of FR-185 would expect a reference-typed support to be admissible. "literals or integers" is also loose, because an integer support value is a literal or a coerced integer literal. Fix: state the rule where it belongs, in FR-185 Random parameters, as a refusal with an AC (for example "a support value SHALL be a literal of a non-reference type; a reference value refuses `value-outside-type`"), and have FR-189 cite it. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:117-121; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md |
| FND-002 | medium | `MeasureValue::Value(Rational)` carries no unit, yet its comment says "a quantity's Rational carries M's unit" and FR-189-AC-15 asserts "A quantity-typed `M` gives a `Value` carrying `M`'s unit". A test cannot observe a unit on a bare `Rational`. Two implementers would differ: one reports the magnitude in `M`'s declared unit, another in a base unit or in the threshold's unit, which is what QSpec FR-410 states ("A quantity estimate carries the threshold's unit"). Fix: either say `Value` holds the magnitude in `M`'s declared unit, give AC-15 an example with a value (such as `M` in `ms` giving `Value(3)` for 3 ms), and reconcile this with FR-410's threshold unit, or make `Value` hold a quantity. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:87,249 |
| FND-003 | medium | TC-624's fixture list still names "a two-server variant with an admitted symmetry declaration and an asymmetric workload". This contradicts the new FR-189 statement that no workload treats one object differently, and AC-3's two workloads (equal operation weights, and unequal operation weights), and TC-624 step 3 now says "the symmetric variant with each workload". An implementer reading the fixtures would look for a workload the spec now says cannot exist. Fix: replace that fixture with the two workloads of FR-189-AC-3. | spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:19 |
| FND-004 | low | ADR-024 SV-2 still says `estimate` is "the conditional probability `Pr(M <= c)` and the empirical nearest-rank quantile for a quantile claim", and its content list `StatisticalVerdict{decision, estimate, interval, basis, provenance, witnesses}` has no `quantile`. FR-192 now reports the quantile in its own field and has removed it from `estimate`. Fix: update SV-2's content list and its `estimate` sentence, or have SV-2 point to FR-192. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:164 |
| FND-005 | low | FR-192's `estimate` bullet, which this PR rewrote, says `estimate` is `Pr(M <= c \| activated)` for every quantile claim. For `quantile q of M >= c`, FR-189 decides `Pr(M < c \| activated) < q`, its margin reads that estimate as an upper bound, and QSpec FR-410 records `Pr(M < c \| activated)`. The wording predates this PR, but it sits in the sentence the PR edited. Fix: "`Pr(M <= c \| activated)` for `quantile q of M <= c` and `Pr(M < c \| activated)` for `quantile q of M >= c`". | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:119-120 |
| FND-006 | low | FR-192's `quantile` bullet lists only "absent for every other claim form" as the absent case, which reads as a complete list. FR-189 also leaves `quantile` absent on a quantile test with no estimate, and that test can be the least favourable one (FR-189-AC-10's "Rejected with no estimate" case). The leading "the least favourable test's `quantile`" implies the right value, so the list is merely incomplete. Fix: add "or when the least favourable test has none", matching the `estimate` bullet. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:125-128 |

## Verdict

Not mergeable as written. Three findings are medium. FND-001: the symmetry argument rests on a rule FR-185 does not state. FND-002: the unit clause of the new value type cannot be tested. FND-003: TC-624's fixture list contradicts the new AC-3. The rest is sound. The nearest-rank rule and all three AC-15 examples check out. The RX-1 argument is right for the language the code builds. The least-favourable-test wiring (FR-189 to FR-192) agrees with itself and with QSpec FR-410-AC-2, which requires both the estimate and the quantile. AC-3 is now buildable. The wording is behaviour-only, and the PR body contains no review-finding paraphrase. Apart from FND-003 and FND-004, no statement under `spec/` outside `reviews/` contradicts the new text.
