---
id: SR-12158
title: "Gap analysis of quire-spec-language PR #994: FR-336 model subject bounded by universes"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3719fc81355c02cf403bf78151983e5dc99a81eb; qsl-analyze/src/model_check/check.rs, qsl-analyze/tests/it/model_check.rs, qsl-analyze/tests/it/counter.rs, qsl-eval/src/simulation/model.rs, qsl-semantics/src/check/temporal_clause.rs, spec/functional/FR-336-bound-a-model-subject-by-its-universes.md, spec/test-cases/TC-846-bound-a-model-subject-by-its-universes.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #994

## Summary

Ticket: QSL-589. This is a planless gap analysis scoped to the PR diff and
FR-336 (TC-846), with QSpec FR-361-AC-8 and FR-125 as context. It checks each
FR-336 criterion against a test that would fail if the behaviour were wrong,
checks that each `#[trace]` claims only what its test verifies, and checks the
coder's "not covered" claims. Plan completion: not assessed. Static review
only: no test was run.

Examined:
- FR-336-AC-1. No test. `TemporalClaim::requirements` takes no universe, so a
  record cannot depend on one. But it keys only the clause's parameter type
  positions and the infinite-trace formula. Nothing writes a
  `DomainKey::Population` domain for a population with no maximum into a
  clause record, so "with the population's domain" is unbuilt, not just
  untested.
- FR-336-AC-2. `a_population_with_no_universe_is_refused_before_exploring`
  covers the refusal half. The FR-126-AC-1 outcome half is over-tagged; see
  SR-12157 FND-001.
- FR-336-AC-3. `a_proof_bound_does_not_make_an_unranged_parameter_finite`.
  Binding correct.
- FR-336-AC-4. No test, and no code. No path computes a model subject's
  obligation identity with its universes (ADR-013 O-09), and `qsl-route` has
  no universe input. The "each `proved` joins only its own request index"
  part needs a result join in `qsl-replay`.
- FR-336 Behavior "EN-1 SHALL classify every root ... before any expansion":
  `check_model_bounds` runs before `Run` is built. Verified.
- FR-336 Behavior "A `ProofBound` SHALL NOT make an EN-1 root finite":
  `check_model` never reads `subject.proof_bounds`. Verified.
- QSpec FR-361-AC-8 (requires-bound before no-instance) and FR-125, context
  only. QSL has no `NoInstance` cause yet (QSL-873, Backlog). Today
  `check_model` returns `RequiresBound` before `Undecided(NoInitialState)`,
  which is the FR-361-AC-7 order. QSL-873 must put its no-instance settlement
  after `check_model_bounds` to keep AC-8.

The coder says FR-336-AC-1 and AC-4 live in the requirement-record and
request-writer layer, outside `qsl-analyze` model_check. That is accurate.
AC-1 belongs in `qsl-semantics` `check/temporal_clause` (the clause record,
with a ConfigVersion fixture whose `config_history` has no maximum). AC-4
belongs in the temporal-item request writer that computes the obligation
identity over the model subject, plus `qsl-replay`'s result join. The problem
is ownership, not honesty: QSL-589 says "Implements: FR-336" and no other
ticket names these two ACs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-336-AC-1 and FR-336-AC-4 have no test, and in part no code. No ticket owns them: QSL-589 says "Implements: FR-336" and lists `qsl-replay` under Touches, and a Linear search for FR-336 finds only QSL-589 and QSL-385 (the spec ticket, Done). If QSL-589 closes on this PR, both ACs are orphaned. File a follow-up ticket for AC-1 (`qsl-semantics` clause record over a population with no maximum, independent of any universe) and AC-4 (obligation identity binds the universes; per-request result join in `qsl-replay`), linked from QSL-589. | spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:91-94 |
| FND-002 | low | `ModelSystem::domains` skips a root when its naming digest fails (`let Ok(digest) = ... else { return; }`). An unbounded root would then vanish from EN-1's pre-check and the run would explore, instead of returning a fault. This code is from before this PR and is likely unreachable under `CanonicalLimits::new(u64::MAX)`, but FR-336's soundness claim rests on every root being classified. It should return an `InternalFault`. | qsl-eval/src/simulation/model.rs:601-604 |

## Verdict

PASS for the code path FR-336 assigns to EN-1. The direct `RequiresBound`
refusal, its order before any expansion, and the rule that proof bounds are
never read are all present and backed (AC-3 fully, AC-2's refusal half). The
blocking trace issue is recorded once, in SR-12157 FND-001. FND-001 here is a
ticket-ownership gap to resolve before QSL-589 closes. It does not block this
PR.
