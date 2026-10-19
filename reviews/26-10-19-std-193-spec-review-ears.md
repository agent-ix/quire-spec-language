---
id: SR-12924
title: 'EARS conformance of STD-193 (PR #1012): FR-134 manifest fairness_kinds refusals'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@e75698ed9; spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md; spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md; context: spec/functional/FR-057-admit-shared-capability-kinds.md, qsl-route/src/lib.rs (#996), PR #1011; agent-ix/quire-specification@a2230e1 FR-368, FR-272, FR-331'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-134
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-535
  type: reviews
---
# SR-12924: ears-conformance review of STD-193 (PR #1012)

## Summary

Ticket: STD-193 (QSpec follow-up STD-194). Scope is `git diff origin/main...HEAD` at e75698ed9: FR-134's new Inputs/Outputs lines, the 'Manifest `fairness_kinds` admission' section, FR-134-AC-3/4/5, the Remaining work section, and TC-535 steps 3-4 with vectors V1-V10. Read against QSL FR-057 (registration causes and their order), merged #996 (`candidates_for_fairness`, `descriptor_from_manifest`), open #1011 (QSL-877), and QSpec main a2230e1 FR-368, FR-272 and FR-331.

What holds: the three causes are honestly marked QSL-proposed, in the section intro and in Remaining work with STD-194, and the rationale is accurate. QSpec FR-272's closed `invalid_capability` row has no fairness cause, defines `unknown-kind` by the supplied FR-290 kind, and keeps evidence kinds apart as `absent-evidence-kind`/`unknown-evidence-kind`. FR-368 and FR-331 both say a backend advertising `temporal-satisfaction` carries `fairness_kinds`, so refusing an absent member reads FR-368 faithfully, and admitting a present empty member matches 'a subset of {weak, strong}'. The precedence rules agree with FR-057: a pair or `domains` defect wins, then unknown before repeat, and the refusal happens before insertion so a held registration stands, the same as the kind/mode/`invalid-domains` causes and unlike `duplicate-backend`. V3's expected result (candidate only for an item with an empty fairness set) matches #996's filter, which keeps a backend whose kinds contain every kind the item uses. V1-V10 are reachable through an unbounded-only temporal manifest, which FR-057 admits without `domains`. V10 puts the unknown label after the repeat, so it tests precedence rather than scan order. No id collides: FR-134-AC-3..5 are new and defined only here, TC-535 keeps its id, and #1011 tags no FR-134-AC-3..5 test. Both files pass `quire validate` (quire 0.36.2). `quire matrix` reports FR-134-AC-3/4/5 `untagged`, which is expected until #1011 follows the spec.

What does not hold, for this lens: the new rules name no system and the precedence rules have no SHALL.

Units examined (each listed in the Linear comment's `scope` with its text): FR-134, FR-134-AC-3, FR-134-AC-4, FR-134-AC-5, TC-535. Context only: FR-057, quire-specification/FR-272, quire-specification/FR-331, quire-specification/FR-368.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The three new rules are subjectless passives ('A label that ... SHALL be refused'), and the precedence and no-insertion rules ('is refused as unknown', 'the refusal carries that earlier cause', 'made before registry insertion') have no SHALL. EARS unwanted-behaviour form, which FR-057 uses for its sibling causes, is 'If <condition>, then the registry SHALL refuse the registration with ...'. Fix: rewrite the three bullets as 'If ..., then the registry SHALL refuse ...', and state the order as 'The registry SHALL judge `fairness_kinds` after every advertised pair and `domains`, and SHALL report an unknown label before a repeated one'. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:84-105 |

## Verdict

Mergeable for this lens; one low wording finding.
