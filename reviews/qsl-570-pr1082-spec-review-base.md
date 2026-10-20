---
id: SR-13392
title: 'Spec review of PR #1082: statistical Stopped shape, provenance, estimate, edge cases, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@73b9df742; git diff refs/remotes/origin/main...73b9df742 (PR #1082): spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md, spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md, spec/test-cases/TC-626-statistical-limits-and-reproduction.md; compared with PR #1079 head 149fdd4b7 and QSpec main 0ad376803 FR-331, FR-408, FR-409; ticket QSL-570'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-189
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-191
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-624
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-626
  type: reviews
---

## Summary

Ticket: QSL-570. PR: quire-spec-language#1082, head 73b9df742, scoped to `git diff refs/remotes/origin/main...73b9df742` (one commit on main 47047b1df, five files, +49/-11). Spec-only. It decides six spec gaps G1 to G6 that terra's code PR #1079 (head 149fdd4b7, branch qsl-570-impl) raised. Base review: whether each decision is sound against FR-186 to FR-193, QSpec and terra's engine, plus hand checks of the margin rule and the `max_samples` boundary, and whether each AC can be tested. The owner decisions in the PR body are labelled 'author's defaults', so they still need the owner's confirmation. G1 to G6 provenance: the QSL-570 comment says they are 'listed in the #1079 PR body'. That body lists only the PR 2 remaining work, the PR has no comments or reviews, and the body has no edit history. So the gaps' text is not recoverable, and the author worked from the six titles. The decisions were compared with terra's code: qsl-replay/src/statistical.rs and qsl-analyze/src/statistical.rs, which hold the Stopped and Unsupported shapes, the per-outcome provenance, `TestResult` and `StatisticalVerdict` `estimate: Option<Rational>`, `least_favourable`, the `max_samples` check order and the refusal set. QSpec was read at main 0ad376803: FR-331-AC-28, FR-408 Admission and Activation, FR-409 combination rules, FR-363, FR-391 and FR-399. QSL context: ADR-018 V-6, ADR-024 SV-2 and SV-4, ADR-028 XV-4, FR-126, FR-190. Methods: spec-review/base (SR-13392), spec-review/ears (SR-13393), spec-review/integrity (SR-13394). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-189, FR-191, FR-192, TC-624 or TC-626. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189-outputs | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-refusals | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-estimate-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-least-favourable-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-no-initial-state-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-undefined-stop-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-non-activated-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-AC-8 | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-191-limit-enum | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-191-outputs | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-191-max-samples-timing-bullet | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-191-AC-5 | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-192-verdict-estimate | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-estimate-bullet | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| TC-624-step-7 | spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md | examined |
| TC-626-step-5 | spec/test-cases/TC-626-statistical-limits-and-reproduction.md | examined |
| FR-190-zero-weight-bullet | spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md | context_only |
| QSpec-FR-331-AC-28 | quire-specification:spec/objects/interfaces/FR-331-backend-provider-envelope.md | context_only |
| QSpec-FR-409-combination | quire-specification:spec/objects/protocol/FR-409-measured-decision.md | context_only |
| QSpec-FR-408-admission | quire-specification:spec/functional/analysis/FR-408-settle-probabilistic-claims-statistically.md | context_only |
| ADR-018-V-6 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | context_only |
| ADR-024-SV-4 | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md | context_only |
| FR-126-stopped | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | context_only |
| PR-1079-code-shapes | agent-ix/quire-spec-language@149fdd4b7 qsl-replay/src/statistical.rs, qsl-analyze/src/statistical.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | An empty subject is refused `NoInitialState`, so FR-192's 'EN-4 refusal' row settles it `unsupported`. QSpec FR-331-AC-28 says a model-subject item with no initial state records `inconclusive` with cause `no-initial-state`. QSL settles the same case that way on every other engine: ADR-018 V-6, FR-126 `Undecided(NoInitialState)`, FR-142, FR-168, FR-181, ADR-022 GV-5 and ADR-023 HM-7. ADR-028 XV-4 says `NoInitialState` keeps its meaning. The PR body says the alternative was 'a new undecided cause, which would need QSpec FR-409'. That premise is wrong: the cause belongs to the FR-331 `inconclusive` value and already exists. It is not on FR-409's measured axis. QSpec FR-408 also reserves `unsupported` for capability gaps (missing-confidence, every-scheduler, exact-only-form, not-markov, ...), and an empty subject is not one. Fix: settle an empty subject `inconclusive`, `no-initial-state`, as an outcome (an `Undecided`/`Inconclusive` variant, or a refusal that FR-192 maps to that row), add the FR-192 map row, and change AC-8 and TC-624 step 7 to match. It still must never be Accepted. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:88-93, 162-164 |
| FND-002 | high | FR-192's `StatisticalVerdict.estimate` is still `Rational`, but it is 'the least favourable test's', and that test's `estimate` can now be `None`. Two cases do this. The first is FR-189-AC-6's `Coin` when its first sample is undefined: one test, Rejected, with no counted sample. The second is a regenerative test whose closed cycles all weigh 0. So the verdict has no value to carry, which contradicts G3. ADR-024 SV-2 also calls `estimate` 'the exact rational point estimate'. Terra's code makes `StatisticalVerdict.estimate: Option<Rational>` and takes it from the chosen test. Fix: make FR-192's `estimate` `Option<Rational>`, `None` exactly when the least favourable test's is, and say how the settled record spells an absent estimate. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:56 |
| FND-003 | medium | G5 differs from what #1079 built, and the spec does not say why. Terra's `least_favourable` ranks tests by decision (Rejected, then Undecided, then Accepted). Within a rank it takes the estimate furthest on the failing side of the bound, ranks a test with no estimate worst, and breaks ties by index. FR-189 takes the first rejecting or first Undecided test, and measures an accepting test's margin by its interval end, not its estimate. The two rules give different answers in three cases. With two rejecting tests, the spec takes the first and the code takes the lower estimate. Regenerative accepting tests have different half-widths, so ranking by interval end and ranking by estimate can disagree. When an undefined sample rejects after an earlier Okamoto rejection, the spec reports the earlier test's estimate, but the rejection cause comes from the later test. Okamoto's equal-width intervals make the margin rules agree. The margin text also never names the strict reduced bound `< q` of `quantile q of M >= c`, which the code treats as an upper bound. Fix: pick one rule with the owner, state why, and name the `<` case. The PR body lists this as an implied code change, so the owner should confirm. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:154-161 |
| FND-004 | medium | G1 leaves the limit enum and the non-budget stops undefined. `ReachedStatisticalLimit{limit: StatisticalLimit, ...}` can name `StatisticalLimit::Cancelled`, but the spec says a cancelled handle carries `None`. `Time` and `ClauseMeter` have no `statistical.*` setting to put in `setting`. Nothing says whether an `ExpansionStop` (for example the clause meter) carries `Some` or `None`, although ADR-024 SV-4 settles it `limit-reached{limit, value, setting}`. FR-126's precedent has no cancelled member in `ModelCheckLimit`. Terra's code answers both points: `StatisticalLimit` has only MaxSamples, MaxDraws and MaxCycleSteps, with `setting()`, and the stop is the enum `Limit{limit, value} \| Expansion(ExpansionStop) \| Cancelled(CancelCause)`. Fix: remove `Cancelled` (and decide `Time` and `ClauseMeter`) from `StatisticalLimit`, and state the `limit` of an expansion stop and its setting. Or adopt the code's shape and explain any difference from FR-126's. | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md:44, 49-52 |
| FND-005 | medium | 'A refusal carries none, since no run began' is not true for every refusal. FR-189 refuses `GeneratorMismatch` when a sample stops with `SampleStop::GeneratorMismatch`, which happens after draws. Terra's code also refuses `FractionForEvent` and `Internal` mid-run. Those refusals drop the provenance of the draws already made. Fix: either say that a refusal carries no provenance even after draws, and why that is acceptable for reproduction, or return the mid-run cases as an outcome that carries provenance. | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md:53-56 |
| FND-006 | medium | '`Some` once it has counted a sample or a cycle' contradicts FR-190: 'a run whose closed cycles all have weight 0 has no estimate yet and continues'. `Σ Y_k / Σ L_k` is 0/0 there. Terra's code returns `None` 'while every closed cycle has weight 0'. 'count / samples' also does not fit a mean of a fraction, whose estimate is the sample mean. Fix: `Some` once the method's estimate is defined, meaning one counted sample for Okamoto and SPRT and `Σ L_k > 0` for the regenerative method, and name each method's estimate. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:150-153 |
| FND-007 | medium | FR-189-AC-8 cannot be checked as written. It says 'Over `Service`' but its first clause uses `Coin`. AC-6's `Coin` at seed 7 does not make the first sample undefined: AC-6 and terra's test look for the first trace index that reaches `v = 1`, which need not be index 0. No fixture is named for the two-test rejecting run, the all-accepting run or the tie, the empty subject, or the all-non-activated quantile claim, which also needs a `max_draws` value (the default is 2^26 draws). The verdict has no field naming the least favourable test, so 'the least favourable test is test 1' can only be seen through the verdict's `estimate` and `interval`, and the AC should assert those. The 'first Undecided' branch has no check. TC-624 step 7's expected result only says 'as AC-8 states'. Fix: name each fixture and seed, assert observable values, and split the five behaviours into separate ACs. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:199 |

## Verdict

Not mergeable. FND-001 and FND-002 (high) must be fixed. FND-001: settling an empty subject `unsupported` contradicts QSpec FR-331-AC-28 and QSL's own `NoInitialState` settlement, which is `inconclusive`. FND-002: FR-192's verdict estimate cannot hold a `None` test estimate. FND-003 to FND-007 (medium) should be fixed or explicitly deferred. FND-003, the least-favourable rule, needs an owner decision. What holds: G2 (provenance on Stopped and Unsupported) and G6 match terra's code. In G6 the method's decision is checked before the `max_samples` and `max_draws` checks, and both are checked before each draw. Checked by hand against Okamoto's N = 9,210,341: `max_samples` 9,210,341 completes and 9,210,340 stops after 9,210,340 counted samples. FR-191-AC-5 is correct. The undefined-stop rule matches terra's code, which runs tests in sequence: earlier results and the rejecting test are kept, and no later test runs. The all-non-activated rule matches QSpec FR-408 Activation (`incomplete`, `limit-reached` naming `max_draws`). The accepting-margin arithmetic is right for `>=` and `<=`. The text reads as system behaviour, not as a changelog.
