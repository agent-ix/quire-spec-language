---
id: SR-12923
title: 'Integrity analysis of STD-193 (PR #1012): FR-134 manifest fairness_kinds refusals'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@e75698ed9; spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md; spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md; context: spec/functional/FR-057-admit-shared-capability-kinds.md, qsl-route/src/lib.rs (#996), PR #1011; agent-ix/quire-specification@a2230e1 FR-368, FR-272, FR-331'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-134
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-535
  type: reviews
---
# SR-12923: integrity review of STD-193 (PR #1012)

## Summary

Ticket: STD-193 (QSpec follow-up STD-194). Scope is `git diff origin/main...HEAD` at e75698ed9: FR-134's new Inputs/Outputs lines, the 'Manifest `fairness_kinds` admission' section, FR-134-AC-3/4/5, the Remaining work section, and TC-535 steps 3-4 with vectors V1-V10. Read against QSL FR-057 (registration causes and their order), merged #996 (`candidates_for_fairness`, `descriptor_from_manifest`), open #1011 (QSL-877), and QSpec main a2230e1 FR-368, FR-272 and FR-331.

What holds: the three causes are honestly marked QSL-proposed, in the section intro and in Remaining work with STD-194, and the rationale is accurate. QSpec FR-272's closed `invalid_capability` row has no fairness cause, defines `unknown-kind` by the supplied FR-290 kind, and keeps evidence kinds apart as `absent-evidence-kind`/`unknown-evidence-kind`. FR-368 and FR-331 both say a backend advertising `temporal-satisfaction` carries `fairness_kinds`, so refusing an absent member reads FR-368 faithfully, and admitting a present empty member matches 'a subset of {weak, strong}'. The precedence rules agree with FR-057: a pair or `domains` defect wins, then unknown before repeat, and the refusal happens before insertion so a held registration stands, the same as the kind/mode/`invalid-domains` causes and unlike `duplicate-backend`. V3's expected result (candidate only for an item with an empty fairness set) matches #996's filter, which keeps a backend whose kinds contain every kind the item uses. V1-V10 are reachable through an unbounded-only temporal manifest, which FR-057 admits without `domains`. V10 puts the unknown label after the repeat, so it tests precedence rather than scan order. No id collides: FR-134-AC-3..5 are new and defined only here, TC-535 keeps its id, and #1011 tags no FR-134-AC-3..5 test. Both files pass `quire validate` (quire 0.36.2). `quire matrix` reports FR-134-AC-3/4/5 `untagged`, which is expected until #1011 follows the spec.

What does not hold, for this lens: QSL's registration contract (FR-057) is not amended to carry the member, and the Remaining work wording and the cause shape could be tighter.

Units examined (each listed in the Linear comment's `scope` with its text): FR-134, FR-134-AC-3, FR-134-AC-4, FR-134-AC-5, TC-535. Context only: FR-057, quire-specification/FR-272, quire-specification/FR-331, quire-specification/FR-368.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-134's Inputs now say the registry gets the manifest's `fairness_kinds` 'as supplied, which may be absent', but FR-057 owns QSL's registration contract and is unchanged. It says the driver registers a backend with its identity, (kind, mode) labels, `domains` 'present or absent' and evidence-kind labels; `fairness_kinds` is not in the list, so as written the registry never learns whether the member was absent, and AC-5 cannot be reached. FR-057's 'Diagnostic code' and registration paragraphs also list the registry causes and say they are catalogued in `quire.native.diagnostics/v1`; the three new ones are neither. #996's `descriptor_from_manifest` takes no fairness input (#1011 adds `Option<&[&str]>`, which also cannot carry the non-string label AC-3 names). QSpec FR-331 also says out-of-domain values and missing required properties 'refuse before consumption', so a second refusal point exists upstream. Fix: amend FR-057 to list `fairness_kinds`, present or absent, in the driver handoff, and to name FR-134's causes and their place after `domains`. Record the FR-331 schema-refusal question in STD-194. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:44-47; spec/functional/FR-057-admit-shared-capability-kinds.md (Stage ownership, Diagnostic code) |
| FND-002 | low | The Remaining work note ends 'Until QSpec rules, QSL keeps them here and a different QSpec cause name replaces these three', which reads as if a rename will certainly happen. Meanwhile AC-3/4/5, which tests and the matrix quote on their own, state the causes as plain `invalid_capability` causes with no QSL-proposed mark, although QSpec FR-272-AC-1 holds the row to exactly its listed variants. Fix: reword to 'if QSpec adopts different names, those replace these three', and add '(QSL-proposed, STD-194)' to each new AC, or a short note under the AC table. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:127-131,113-115 |
| FND-003 | low | The repeat rule cites the `domains` member as its precedent ('as a repeated `domains` kind is'), but QSpec and FR-057 refuse every `domains` defect (empty, unknown or non-boundable kind, repeat, absent when `bounded`) with one cause, `invalid-domains`. FR-134 proposes three causes for the same kinds of defect on the sibling member. Both shapes are defensible; the divergence should be a deliberate choice for STD-194, not an accident. Fix: in Remaining work, name the single-cause alternative (for example `invalid-fairness-kinds`) as the shape QSpec may prefer. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:88-91; spec/functional/FR-057-admit-shared-capability-kinds.md (invalid-domains) |

## Verdict

Not mergeable as is: FND-001 is medium. FND-002 and FND-003 are low; FND-003 is an input for the STD-194 ruling, not a blocker.
