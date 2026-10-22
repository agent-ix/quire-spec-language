---
id: SR-12306
title: 'Spec review of PR #1131: FR-181 edits (AC-3, AC-5, AC-7, Outcomes)'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@62b9d71fd; git diff origin/main...62b9d71fd -- spec/ (PR #1131, branch qsl-556-hp5): spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md, with spec/test-cases/TC-606, TC-615 and TC-616 read for consistency; ticket QSL-556'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-181
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-606
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-615
  type: reviews
---

## Summary

Ticket: QSL-556. PR: quire-spec-language#1131, head 62b9d71fd. This is a base spec review with integrity and EARS checks of the FR-181 edits from the fix round: the AC-3 fairness syntax, the AC-5 rewording, the trap/`Undefined` order, the per-instance rule and the new FR-181-AC-7. `quire validate` passes on FR-181. The edits are consistent with the engine and with FR-125/FR-126. The defects are in the test cases the edited ACs point to, which were not updated with them, and in one rule the engine relies on that the FR does not state.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-181-AC-3 | spec/functional/FR-181-...md | examined |
| FR-181-AC-5 | spec/functional/FR-181-...md | examined |
| FR-181-AC-7 | spec/functional/FR-181-...md | examined |
| FR-181 Outcomes (Undefined/trap order, Trapped, instances) | spec/functional/FR-181-...md | examined |
| FR-181 Check (open nodes) | spec/functional/FR-181-...md | examined |
| TC-606 | spec/test-cases/TC-606-single-existential-through-the-possible-family.md | examined |
| TC-615 | spec/test-cases/TC-615-a-single-existential-witness-proves-only-after-exploration.md | examined |
| TC-616 | spec/test-cases/TC-616-a-single-existential-witness-on-a-stopped-run-is-unchecked.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-181-AC-7 is verified by "Test (TC-606)", but TC-606 does not cover it. TC-606's scope reads "FR-181-AC-1 to FR-181-AC-4", and it has no procedure step or expected result for two instances or for a clause with no instance. The tests carry `#[trace("TC-606", "FR-181-AC-7")]` against a TC that does not describe them. Add a TC-606 step and expected result for AC-7 and widen its scope, or name a TC that covers AC-7. | FR-181-AC-7; TC-606 Scope, Test Procedure |
| FND-002 | low | TC-606 step 3 still writes `fair { weak V::Vault::reset }`. FR-181-AC-3 now uses FR-172's grammar, `fair weak V::Vault::reset;`, and the tests do too. | TC-606 Test Procedure step 3 |
| FND-003 | low | TC-615 step 1's expected result still says "each initial state has a witness lasso looping `step(0)` at `(h, 0)` ... in both runs". The revised FR-181-AC-5 says the sampled lasso is found only by phase 0, which is with the default `witness_samples`; with `witness_samples` 0 there is none, and the outcome is `Undefined` in both runs. | TC-615 Expected Results, step 1; FR-181-AC-5 |
| FND-004 | low | FR-181 Check says a node not fully expanded is open, and Outcomes says a closed part is one with no open node. Neither says that a cycle through an open node, or a phase 0 loop through a state whose contract evaluation had no value, does not count as a fair accepting cycle or a witness. The engine relies on that rule (the SR-13627 FND-002 fix, `Decision::lasso`'s `within`, `Run::witness`'s undecided-contract check), and the test `a_contract_with_no_value_leaves_its_state_open` depends on it. Without it, a reader could take FR-181 to allow the stutter lasso the engine used to return. State it under Check and Phase 0. | FR-181 Check, Phase 0 |

## Verdict

No major finding. One medium (the AC-7 to TC-606 trace has no TC content behind it) and three lows.

Clean units:

- FR-181-AC-3's grammar fix is right.
- AC-5's rewording matches the engine.
- The Outcomes order rule is consistent with the engine: a trap ranks at the first product state of its part, and a tie goes to `Undefined` (UE-4).
- `Trapped` naming the first initial state in subject order is consistent with it too.
- The per-instance rule cites FR-125's instance order and FR-126's shared budgets, which is what the engine does.
- AC-7 is atomic enough, observable, and can fail. The mutant that gives each instance the full budget fails its test.

The braced `fair { ... }` text in FR-171-AC-2, FR-173-AC-2, FR-174-AC-1 and FR-177-AC-9 is outside this diff.
