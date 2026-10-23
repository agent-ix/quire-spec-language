---
id: SR-16355
title: "spec-review of model-check EvaluationMeter stops naming the FR-255 setting (quire-spec-language PR 1396)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@90aeae32; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md; spec/test-cases/TC-521-explicit-state-model-check.md; spec/test-cases/TC-522-model-check-verdicts-settle-as-terminal-records.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: references
---

# Spec review of STD-445 model-check meter stops (quire-spec-language PR 1396)

## Summary

Ticket: STD-445. One spec-review pass over `git diff origin/main...90aeae32`.
Checked against FR-120 (ExpansionLimit::EvaluationMeter{counter}, AC-21), FR-192 and
FR-203 (meter stops write the counter and accounting.<counter>), FR-255 (table
and Behaviors 9 and 11), and FR-338/FR-339 as on main. The new FR-127 table,
AC-1/4/7/16 and FR-126-AC-19 agree with them. Hand check: the accounting
defaults (work_units 1000000, value_occurrences and integer_bits u64::MAX) match
FR-255, so the AC-16 vectors are well formed. No deferral or narrowing language.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-19's first vector names no claim. FR-126 creates the product state of position 0, with its letter, before any expansion, and AC-19's own second vector shows a letter atom stops at position 0 under work_units 0. Take the first vector with any letter-bearing claim (for example `always holds(c.value <= 3)`): the run stops at the letter of position 0, not at the `inc` precondition, so the stated stop point is wrong; it holds only for a claim that reads no atom (DeadlockFreedom). The same gap is in the third vector (max_candidates 1 over the AC-1 subject, no claim named) and in FR-126-AC-5's meter sentence and TC-521 steps 5 and 11. Fix: name the claim in each vector (DeadlockFreedom for the precondition stop; the letter claim for the letter stop). | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:500 | wrong-requirement |
| FND-002 | low | The Purpose paragraph still says the engine reports a limit 'with the limit's value and the request member that raises it'. The Outputs paragraph now says the FR-255 setting raises it (accounting.<counter> is not a request member). Fix: say 'the FR-255 setting that raises it'. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:51 | wrong-requirement |
| FND-003 | low | The certificate-checker stop text still says 'the corresponding existing MaxStates/MaxTransitions/MaxAutomatonStates and request setting' and 'actual charged value and request setting'. FR-127 (as edited) now defines the setting as the FR-255 name (model_check.*, accounting.<counter>). Fix: say 'FR-255 setting' in both places. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:654-658 | wrong-requirement |

## Verdict

Mergeable after FND-001 (medium) is fixed; FND-002 and FND-003 (low) should be fixed in the same round.
