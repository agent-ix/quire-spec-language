---
id: SR-1587002
title: 'Gap analysis of PR #1587: HP-2 k-induction settlement (FR-182), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@2d47d5bce; git diff origin/main...2d47d5bce (PR #1587, branch qsl-991-kinduction-settle): qsl-replay/src/model_check/hyper_settle.rs, qsl-replay/src/model_check/hyper_smt_outcome.rs, qsl-replay/tests/it/hyper_smt_settle.rs; ticket QSL-991'
review_set: subset
date: '2026-10-26'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-182
  type: reviews
---

## Summary

Ticket: QSL-991. PR: quire-spec-language#1587, head 2d47d5bce. Method: gap-analysis, scoped to the diff. Plan completion: not assessed. `quoin matrix --json` over the whole repository did not finish within 300 s, so the criterion-to-test check was done by hand from the `#[trace]` tags in the diff and a repository-wide grep for `FR-182-AC-*` tags. The three new tests pass (`cargo test -p qsl-replay --test it hyper_smt_settle`, 3 passed). FR-182-AC-7's k-induction sentence is backed by `an_inductive_outcome_with_an_empty_match_settles_proved_uncertified`. The other two tests carry the same tag but assert other criteria, and the `NoInitialState` guard has no test, although the PR description says it does.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-182-AC-7 | spec/functional/FR-182-settle-a-hyper-verdict.md | examined |
| FR-182-AC-10 | spec/functional/FR-182-settle-a-hyper-verdict.md | examined |
| FR-182-AC-11 | spec/functional/FR-182-settle-a-hyper-verdict.md | examined |
| FR-182-AC-1 | spec/functional/FR-182-settle-a-hyper-verdict.md | context_only |
| hyper_smt_settle tests | qsl-replay/tests/it/hyper_smt_settle.rs | examined |
| inductive_row | qsl-replay/src/model_check/hyper_settle.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Tag inflation. `an_inductive_outcome_is_not_proved_without_a_match_or_for_another_instance` is tagged FR-182-AC-7, but it asserts `VacuousMatch` for a non-empty `μ_U` with no witness (FR-182-AC-10's first sentence) and `ProductMismatch` naming `instance` (FR-182-AC-11). Neither is in AC-7. | qsl-replay/tests/it/hyper_smt_settle.rs:75-92 |
| FND-002 | medium | Tag inflation. `an_unclosed_induction_is_inconclusive` is tagged FR-182-AC-7, but it asserts the V-6 `InductionNotClosed` row (FR-182-AC-1's table, and FR-182-AC-11's last clause). AC-7 does not mention `InductionNotClosed`. | qsl-replay/tests/it/hyper_smt_settle.rs:94-109 |
| FND-003 | medium | The V-3 `NoInitialState` guard has no test, so deleting it fails nothing. The PR description says a test covers it; none does. FR-182-AC-10's last sentence gives the vector: an empty initial list settles `NoInitialState`. | qsl-replay/src/model_check/hyper_settle.rs:338-346 |

## Verdict

Not mergeable as is. Retag FND-001 and FND-002 so each test names only criteria it fully asserts (AC-1 for the table rows, TC-607). Add the `NoInitialState` test (FND-003). The unbuilt witness path of AC-10 and AC-11 belongs to the subticket in SR-1587001 FND-002. Settling a non-empty `μ_U` as `VacuousMatch` is not a stub: with no witness, FR-182 Behavior requires exactly that result, and the code never settles `proved` there.
