---
id: SR-15940
title: "spec review of STD-282 exact product build settlement (QSL PR #1256)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@5e2e83b84c2fb95e02c4d620f46894a0d989c5e3; spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md, spec/functional/FR-196-build-the-probabilistic-product.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/test-cases/TC-1166-every-exact-product-build-result-settles.md"
review_set: subset
evaluated_revision: "5e2e83b84c2fb95e02c4d620f46894a0d989c5e3"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1166
    type: reviews
---
# SR-15940: spec review of STD-282 exact product build settlement (QSL PR #1256)

## Summary

Ticket: STD-282 (unblocks the QSL-573 remainder and QSL-574). PR quire-spec-language#1256, head 5e2e83b84 (matches `git ls-remote`), reviewed as a merge with origin/main 39823fe73 (1ae52f482). Four files, spec Markdown only. FR-203's "Build refusals and stops" table gains rows for the limit-naming expansion stops, each `ClauseOutcome` under `PredicateRefused`, `Translation`, and the request refusals; `ExactProbLimit` gains `MaxCandidates`; FR-196 points to the table; FR-192 uses FR-255 setting spellings; FR-203-AC-7 and TC-1166 check the map.

These claims hold:

- The rows cover every `ProductRefusal` member and every `ClauseOutcome` variant of `qsl_eval::simulation::model::ClauseOutcome` (KernelUndefined, KernelRefused, FamilyUndefined, FamilyRefused, Incomplete, NotBoolean). The broken-invariant reading is grounded: FR-196 returns an undefined read as `ProbBuild::Undefined`, never `PredicateRefused`, and a checked claim predicate completes a Boolean.
- The refusal rule (EN-5 settles `failed` with the refusal's own diagnostic, a predicate `Incomplete` settles V-7) is right; see FND-002 for why EN-4 should follow it.
- `expansion.max_candidates` is #1244's FR-255 engine-table row. The `Translation` row is sound: FR-196 stops an automaton translation limit as `Stopped(Limit{MaxAutomatonStates})`, so a translation error is never a reached limit.
- Ids: TC-1166 and FR-203-AC-7 appear on no other open PR head. No status wording or compat layer. The gate on the merge exits 0 with no diagnostic on a touched file.

Conflict with #1244, exactly: a trial merge of #1256 onto #1244 (73d07a397) conflicts only in FR-203's AC table. #1244 rewrites FR-203-AC-6 to `Incomplete(LimitReached{MaxAutomatonStates, value, exact_prob.max_automaton_states})`; #1256 keeps main's AC-6 (`... max_automaton_states})`) and appends AC-7 on the next line. #1244's MaxStates/MaxTransitions/MaxAutomatonStates build row (`exact_prob.` setting) and Budgets merge cleanly, and #1256 adds no `exact_probabilistic.` spelling. What #1256 must change: rebase after #1244, keep #1244's AC-6 verbatim and append AC-7; and settle FND-003 and FND-004 on the meter and candidates forms.

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: textual conflict with #1244. #1256 carries main's FR-203-AC-6 (setting `max_automaton_states`) directly above its new AC-7, and #1244 rewrites that row to `exact_prob.max_automaton_states`, so the two PRs conflict there and nowhere else. Fix: merge #1244 first, then rebase #1256 keeping #1244's AC-6 verbatim and appending AC-7. | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:162-163 |
| FND-002 | medium | major: after this PR, EN-4 and EN-5 settle the same `PredicateRefused(ClauseOutcome)` differently. FR-192 still drops it as a request refusal with no terminal record, while FR-203 settles it `failed`, V-7 or `runtime_invariant` by variant. EN-5's rule is the right one. A refused predicate at a reachable state is an evaluation outcome of the item, as FR-192's own "expansion that names no limit ... failed carrying that cause" row already treats a refused state-constraint family. A predicate `Incomplete` is a clause-meter exhaustion, which QSpec FR-408 says stops the run `incomplete`, `limit-reached`. The qsl-replay code's own comment says "FR-192's map has no row for it". FR-189:142 also ties EN-4 to EN-5. Fix: split FR-192's row into the same per-variant rows in this PR, or have the planner file the FR-192 follow-up. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:113; spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:141-142; qsl-replay/src/statistical/settle.rs:127-129,233 |
| FND-003 | medium | minor: the meter-stop form `LimitReached{ClauseMeter, B, work_units}` names a limit with no wire spelling and a setting whose spelling is open. #1244's FR-255 Behavior 11 has the outcome name `limit` as an engine-table limit name, and the table has no meter row. Behavior 9 has the outcome name the counter. EN-1 and EN-4 name the same stop `EvaluationMeter` (FR-127-AC-4, FR-192). The bare `work_units` setting fails QSpec's dotted `SettingName` pattern, which is the open QSpec question STD-323. AC-7 and TC-1166 hard-code both. Fix: state that the meter stop's wire `limit` and `setting` are the exhausted counter's name per FR-255 Behavior 9, and write the vectors against that rule; if STD-323 dot-qualifies the name, only FR-255 changes. | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:127,129,163; spec/test-cases/TC-1166-every-exact-product-build-result-settles.md:38-42 |
| FND-004 | low | minor: the `MaxCandidates` row writes `limit: MaxCandidates`, the typed variant, but FR-203 gives no wire spelling for `ExactProbLimit`. #1244 Behavior 11 requires the engine table's limit name, `max_candidates`. Fix: say the row's limit is written `max_candidates` (the `expansion.max_candidates` row's limit name). | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:55,126 |
| FND-005 | low | minor: the `work_units` 0 vector does not say which charge is denied first. If the build reads an initial state's predicates before it expands that state, the refusal is `PredicateRefused(Incomplete)`, not `Expansion(EvaluationMeter)`. Both rows settle to the same record, so the `Expansion(EvaluationMeter)` row can go unexercised. Fix: assert the build's `ProductRefusal` variant before settling, or pick a fixture whose first charge is an expansion charge. | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:163; spec/test-cases/TC-1166-every-exact-product-build-result-settles.md:22-24 |

## Verdict

Mergeable after #1244 merges and FND-001's AC-6 conflict is resolved as "#1244's AC-6 plus AC-7". FND-002 and FND-003 need a planner decision. FND-004 and FND-005 are minor. The row coverage over `ProductRefusal` and `ClauseOutcome`, the broken-invariant reasoning, the `Translation` row, the `expansion.max_candidates` name, id uniqueness and the gate are verified. Owner or planner decisions: (1) EN-4 vs EN-5 for `PredicateRefused`. EN-5's per-variant rule is right, and FR-192's request-refusal row should change to match it, in this PR or in a follow-up ticket. (2) Whether the meter-stop vectors wait for STD-323 or are written against FR-255 Behavior 9's rule (FND-003). (3) Acknowledge lead default 2 (add `MaxCandidates` to `ExactProbLimit`, keep `ClauseMeter`).
