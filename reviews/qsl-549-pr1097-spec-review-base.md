---
id: SR-13540
title: 'Spec review of PR #1097: FR-149 meter stops and expansion stop, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@100f4dc1d; git diff refs/remotes/origin/main...100f4dc1d (PR #1097, base main 7e28a47ae): spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md; read against FR-120 Contract clauses and Invariants, FR-126 limits, TC-556, and on main qsl-eval/src/value/expression/clause_set.rs (combine, run_group, verdict), qsl-eval/src/simulation/model/contracts.rs (conjunction), qsl-eval/src/simulation/model/walk.rs (exhausted), qsl-eval/src/simulation/expansion.rs (ExpansionStop, ExpansionLimit), qsl-replay/src/certificate/check.rs (expansion_halt), and PR #1089 head f8f40d586 qsl-replay/src/certificate/simulation.rs; ticket QSL-549'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-149
  type: reviews
---

## Summary

Ticket: QSL-549. PR: quire-spec-language#1097, head 100f4dc1d, scoped to `git diff refs/remotes/origin/main...100f4dc1d` (one commit, one file, +9/-5). Spec-only. Base review of the three changed units: the Meter stops bullet, the Expansion stop bullet and FR-149-AC-5 (c).

The PR's claim was checked against the code on main. `clause_set::combine` returns `Incomplete` whenever any clause verdict is `Incomplete`. That check comes before `False` and `Undecided`, and `run_group` ends the set at the first `Incomplete`. `contracts::conjunction` maps `ClauseSetOutcome::Incomplete` to `Err(exhausted(ExpansionLimit::EvaluationMeter))`, an `ExpansionStop` with `limit: Some(EvaluationMeter)`. `ContractUndetermined` is built only on the `Undecided` arm. So an incomplete contract is never a finding, and the old 'or a `BadState` finding is' clause of Meter stops was vacuous, as the PR says. FR-120 lines 384-397 say the same. `check.rs::expansion_halt` on main maps `Some(EvaluationMeter)` to `LimitReached{EvaluationMeter, budgets.meter}`, `Some(MaxCandidates)` to `LimitReached{MaxCandidates, ...}`, and `None` to `ResourceExhausted` unless the stop is an internal failure. That is the new Expansion stop bullet, and FR-126's 'exhausted the subject's evaluation meter' -> `{EvaluationMeter, value}` agrees with it. #1089's `undecided_state` `MeterStop` arm is dead for the same reason; its own test comment says so. That is an implied code clean-up, not a spec defect. TC-556 step 6 and its expected 'four `Stopped(LimitReached{EvaluationMeter, B})`' agree with the new AC-5 (c), and step 11 still matches AC-10. Relationships: FR-149 already has `depends_on` FR-120 and FR-126; no edge is added or needed. The text describes behaviour and its reason; it has no status or changelog wording.

One soundness gap (FND-001). The new sentence says that FR-120 turns 'an incomplete contract' into an expansion stop, never a finding. It does this for every incomplete contract, not only for one that ran out of the meter: `clause_set::verdict` treats any `Outcome::Incomplete` as `Incomplete`, and quire-exact's `LimitKind` also has `IntegerBits`, `DecimalDigits` and other counters. The unchanged Undecided state rule still sends 'a reason other than the evaluation meter' to `BadState`, so it covers a case the checker can never see as a finding.

Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same count as on main 7e28a47ae, none in FR-149. tools/check-index-completeness.sh exits 0. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13540) and spec-review/ears (SR-13541). code-review and gap-analysis were not run, because no code or test file changed. spec-review/integrity was not run, because no structural or relationship change was made.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-149-meter-stops | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-expansion-stop | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-5 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-undecided-state | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | context_only |
| FR-149-AC-7 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | context_only |
| FR-149-AC-10 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | context_only |
| FR-120 | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | context_only |
| TC-556 | spec/test-cases/TC-556-the-simulation-certificate-checker-accepts-and-rejects.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new Expansion stop sentence holds for every incomplete contract, not only one out of meter. On main, `clause_set::verdict` treats any `Outcome::Incomplete` (quire-exact `LimitKind::IntegerBits`, `DecimalDigits`, ... as well as work units) as `Incomplete`, and `contracts::conjunction` maps every `ClauseSetOutcome::Incomplete` to `exhausted(EvaluationMeter)`. The unchanged Undecided state rule still says a contract 'undecided there for a reason other than the evaluation meter' SHALL be `BadState`, and a counter-limit Incomplete is such a reason. That SHALL can never be met, because the expansion stops first, so the two rules disagree for that case. The PR body says BadState now means refused/undefined, and AC-7 says 'undefined or refused (not out of meter)', but the rule's own text does not say that. Fix: narrow Undecided state to 'undefined or refused' (FR-120's `ContractUndetermined` cases), so every incomplete contract is the Expansion stop's. FR-120 calling a counter-limit Incomplete `EvaluationMeter` is a separate FR-120 question and is outside this diff. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:157-160, spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:196-202, qsl-eval/src/value/expression/clause_set.rs:70-71, qsl-eval/src/simulation/model/contracts.rs:164 |

## Verdict

One medium soundness finding. The PR's central claim is true on main: an incomplete contract is an FR-120 expansion stop with `limit: Some(EvaluationMeter)`, never a `ContractUndetermined` finding. `expansion_halt` turns that stop into `Stopped(LimitReached{EvaluationMeter, B})`, so dropping the BadState clause from Meter stops loses nothing. AC-5 (c) and TC-556 step 6 agree. The remaining gap is that the unchanged Undecided state rule is wider than the meaning the PR gives it: it still names non-meter incomplete contracts, which the expansion stop always takes first. Not mergeable until FND-001 is fixed or the lead dispositions it.
