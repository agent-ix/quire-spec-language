---
id: SR-12921
title: 'Base review of STD-193 (PR #1012): FR-134 manifest fairness_kinds refusals'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@e75698ed9; spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md; spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md; context: spec/functional/FR-057-admit-shared-capability-kinds.md, qsl-route/src/lib.rs (#996), PR #1011; agent-ix/quire-specification@a2230e1 FR-368, FR-272, FR-331'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-134
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-535
  type: reviews
---
# SR-12921: base review of STD-193 (PR #1012)

## Summary

Ticket: STD-193 (QSpec follow-up STD-194). Scope is `git diff origin/main...HEAD` at e75698ed9: FR-134's new Inputs/Outputs lines, the 'Manifest `fairness_kinds` admission' section, FR-134-AC-3/4/5, the Remaining work section, and TC-535 steps 3-4 with vectors V1-V10. Read against QSL FR-057 (registration causes and their order), merged #996 (`candidates_for_fairness`, `descriptor_from_manifest`), open #1011 (QSL-877), and QSpec main a2230e1 FR-368, FR-272 and FR-331.

What holds: the three causes are honestly marked QSL-proposed, in the section intro and in Remaining work with STD-194, and the rationale is accurate. QSpec FR-272's closed `invalid_capability` row has no fairness cause, defines `unknown-kind` by the supplied FR-290 kind, and keeps evidence kinds apart as `absent-evidence-kind`/`unknown-evidence-kind`. FR-368 and FR-331 both say a backend advertising `temporal-satisfaction` carries `fairness_kinds`, so refusing an absent member reads FR-368 faithfully, and admitting a present empty member matches 'a subset of {weak, strong}'. The precedence rules agree with FR-057: a pair or `domains` defect wins, then unknown before repeat, and the refusal happens before insertion so a held registration stands, the same as the kind/mode/`invalid-domains` causes and unlike `duplicate-backend`. V3's expected result (candidate only for an item with an empty fairness set) matches #996's filter, which keeps a backend whose kinds contain every kind the item uses. V1-V10 are reachable through an unbounded-only temporal manifest, which FR-057 admits without `domains`. V10 puts the unknown label after the repeat, so it tests precedence rather than scan order. No id collides: FR-134-AC-3..5 are new and defined only here, TC-535 keeps its id, and #1011 tags no FR-134-AC-3..5 test. Both files pass `quire validate` (quire 0.36.2). `quire matrix` reports FR-134-AC-3/4/5 `untagged`, which is expected until #1011 follows the spec.

What does not hold, for this lens: the precedence the Behavior states has no criterion or vector, and two clauses of the new criteria have no TC-535 vector.

Units examined (each listed in the Linear comment's `scope` with its text): FR-134, FR-134-AC-3, FR-134-AC-4, FR-134-AC-5, TC-535. Context only: FR-057, quire-specification/FR-272, quire-specification/FR-331, quire-specification/FR-368.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The rule that a capability-pair or `domains` defect (FR-057) wins over a fairness defect is stated only in Behavior. No FR-134 criterion states it and no TC-535 vector exercises it, so an implementation that checks `fairness_kinds` before `domains` passes every AC. FR-057-AC-12 covers its own pair-over-domains order with a criterion. Fix: add a clause to AC-3 (or a new AC) with two vectors, for example a manifest with an unknown mode and `["Weak"]` refused `unknown-mode`, and a `bounded` manifest with no `domains` and member absent refused `invalid-domains`. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:99-105; spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md:26-45 |
| FND-002 | low | Two clauses of the new criteria have no vector. AC-3 names 'a non-string' label, but V4-V6 and V10 are all strings. AC-5 says a refusal 'under AC-3 to AC-5' leaves an earlier registration standing, but step 4 only re-registers V4, so the duplicate and absent refusals are never checked against a held registration. Fix: add a non-string vector (for example `[1]`), and repeat step 4 with V7 and V9. | spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md:26-45; spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:113-115 |
| FND-003 | low | Step 4's non-temporal manifest is 'a pair whose kind is not `temporal-satisfaction`', with no mode and no `domains`. If the tester picks a `bounded` pair, FR-057 refuses it `invalid-domains`, and the expected result 'the non-temporal manifest is admitted' fails for a reason that has nothing to do with fairness. Fix: name the pair, for example (`value-validity`, `unbounded`), or give it `domains`. | spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md:43-45,59 |

## Verdict

Not mergeable as is: FND-001 is medium. No high finding; the QSL-proposed approach is sound and honestly marked. Add a precedence clause to an AC with two vectors, and the missing vectors, in the fix round.
