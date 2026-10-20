---
id: SR-13394
title: 'Spec review of PR #1082: statistical Stopped shape, provenance, estimate, edge cases, integrity'
type: SpecReview
analysis: integrity
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

Ticket: QSL-570. PR: quire-spec-language#1082, head 73b9df742, scoped to `git diff refs/remotes/origin/main...73b9df742` (one commit on main 47047b1df, five files, +49/-11). Spec-only. It decides six spec gaps G1 to G6 that terra's code PR #1079 (head 149fdd4b7, branch qsl-570-impl) raised. Integrity review: consistency across requirements, notation, relationships, and TC coverage of the new ACs. G1 to G6 provenance: the QSL-570 comment says they are 'listed in the #1079 PR body'. That body lists only the PR 2 remaining work, the PR has no comments or reviews, and the body has no edit history. So the gaps' text is not recoverable, and the author worked from the six titles. The decisions were compared with terra's code: qsl-replay/src/statistical.rs and qsl-analyze/src/statistical.rs, which hold the Stopped and Unsupported shapes, the per-outcome provenance, `TestResult` and `StatisticalVerdict` `estimate: Option<Rational>`, `least_favourable`, the `max_samples` check order and the refusal set. QSpec was read at main 0ad376803: FR-331-AC-28, FR-408 Admission and Activation, FR-409 combination rules, FR-363, FR-391 and FR-399. QSL context: ADR-018 V-6, ADR-024 SV-2 and SV-4, ADR-028 XV-4, FR-126, FR-190. Methods: spec-review/base (SR-13392), spec-review/ears (SR-13393), spec-review/integrity (SR-13394). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-189, FR-191, FR-192, TC-624 or TC-626. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-189-refusals | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-outputs | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-stops-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-AC-5 | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-191-outputs | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-191-AC-1 | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-191-AC-5 | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md | examined |
| FR-192-unsupported-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-unsupported-causes | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| TC-624-fixtures | spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md | examined |
| TC-624-step-7 | spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md | examined |
| TC-626-fixtures | spec/test-cases/TC-626-statistical-limits-and-reproduction.md | examined |
| TC-626-step-5 | spec/test-cases/TC-626-statistical-limits-and-reproduction.md | examined |
| PR-1079-code-shapes | agent-ix/quire-spec-language@149fdd4b7 qsl-replay/src/statistical.rs, qsl-analyze/src/statistical.rs | context_only |
| FR-126-stopped | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec's `StatisticalRefusal` list does not cover what #1079's engine refuses, and FR-192 cannot settle refusals the spec does not name. The code also refuses `TooManyTests` (more than u32 tests), `TraceIndexRange` (`max_draws · m` overflows the trace index), `ConfidenceBeyondPrecision` (no normal-quantile bound for the regenerative method), `FractionForEvent` and `Internal`. It names the empty subject `NoTest`, where the spec says `NoInitialState`. The six gaps themselves cannot be checked against terra's report. The QSL-570 comment says G1 to G6 are 'listed in the #1079 PR body', but that body lists only remaining work, the PR has no comments, and the body has no edit history. So the author worked from the titles alone. The titles do match the six topics. Fix: decide which of these refusals belong in the spec (TraceIndexRange and ConfidenceBeyondPrecision follow from FR-191's budgets and QSpec FR-408's precision) and give each an FR-192 settlement. Record the empty-subject name in one place. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:87-93 |
| FND-002 | low | `Unsupported` is now a struct variant `Unsupported { cause, provenance }`. FR-189 Behavior (`Unsupported(NotMarkov{…})`, `Unsupported(TotalAboveDrawRange{total})`), FR-189-AC-5 and AC-7, FR-192's map row and FR-192-AC-1 still write the tuple form. FR-191's Outputs bullet still reads `Stopped{cause, limit, partial}` without `provenance`. Use the new shapes everywhere. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:173-175 |
| FND-003 | low | TC-624's Fixtures line was not extended for step 7. It lacks the `Coin` model, a two-test variant with a rejecting second test, an all-accepting two-test variant and a tied one, an empty subject, and an all-non-activated quantile variant with its `max_draws`. TC-626's Fixtures line names no fixture for the `Unsupported` outcome that step 5 reads (for example the `Health` NotMarkov variant or `Wide`). Add them, so each step can be built from its TC. | spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:19, 27, 39 |

## Verdict

One medium and two low findings. They should be fixed with the base findings. What holds: FR-189-AC-8 is in TC-624's scope and step 7, and FR-191-AC-5 is in TC-626's scope and step 5, one to one. Index completeness passes. No new relationship edge is needed: FR-192 already depends_on FR-189, and FR-191 already depends_on FR-189.
