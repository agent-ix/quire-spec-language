---
id: SR-12922
title: 'Failure-domain analysis of STD-193 (PR #1012): FR-134 manifest fairness_kinds refusals'
type: SpecReview
analysis: failure-domain
scope: 'agent-ix/quire-spec-language@e75698ed9; spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md; spec/test-cases/TC-535-en-1-advertises-fairness-kinds.md; context: spec/functional/FR-057-admit-shared-capability-kinds.md, qsl-route/src/lib.rs (#996), PR #1011; agent-ix/quire-specification@a2230e1 FR-368, FR-272, FR-331'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-134
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-535
  type: reviews
---
# SR-12922: failure-domain review of STD-193 (PR #1012)

## Summary

Ticket: STD-193 (QSpec follow-up STD-194). Scope is `git diff origin/main...HEAD` at e75698ed9: FR-134's new Inputs/Outputs lines, the 'Manifest `fairness_kinds` admission' section, FR-134-AC-3/4/5, the Remaining work section, and TC-535 steps 3-4 with vectors V1-V10. Read against QSL FR-057 (registration causes and their order), merged #996 (`candidates_for_fairness`, `descriptor_from_manifest`), open #1011 (QSL-877), and QSpec main a2230e1 FR-368, FR-272 and FR-331.

What holds: the three causes are honestly marked QSL-proposed, in the section intro and in Remaining work with STD-194, and the rationale is accurate. QSpec FR-272's closed `invalid_capability` row has no fairness cause, defines `unknown-kind` by the supplied FR-290 kind, and keeps evidence kinds apart as `absent-evidence-kind`/`unknown-evidence-kind`. FR-368 and FR-331 both say a backend advertising `temporal-satisfaction` carries `fairness_kinds`, so refusing an absent member reads FR-368 faithfully, and admitting a present empty member matches 'a subset of {weak, strong}'. The precedence rules agree with FR-057: a pair or `domains` defect wins, then unknown before repeat, and the refusal happens before insertion so a held registration stands, the same as the kind/mode/`invalid-domains` causes and unlike `duplicate-backend`. V3's expected result (candidate only for an item with an empty fairness set) matches #996's filter, which keeps a backend whose kinds contain every kind the item uses. V1-V10 are reachable through an unbounded-only temporal manifest, which FR-057 admits without `domains`. V10 puts the unknown label after the repeat, so it tests precedence rather than scan order. No id collides: FR-134-AC-3..5 are new and defined only here, TC-535 keeps its id, and #1011 tags no FR-134-AC-3..5 test. Both files pass `quire validate` (quire 0.36.2). `quire matrix` reports FR-134-AC-3/4/5 `untagged`, which is expected until #1011 follows the spec.

What does not hold, for this lens: the member's own shape, a member on a non-temporal backend, and which label a multi-defect member names are not decided.

Units examined (each listed in the Linear comment's `scope` with its text): FR-134, FR-134-AC-3, FR-134-AC-4, FR-134-AC-5, TC-535. Context only: FR-057, quire-specification/FR-272, quire-specification/FR-331, quire-specification/FR-368.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The rules cover labels and an omitted member, but not a member whose value is not a list: `"fairness_kinds": null`, `"weak"` or `{}`. One implementer reads `null` as absent (FR-057 treats a JSON `null` capability label as `absent-kind`); another reads it as an unknown value. A bare string `"weak"` could be refused unknown, refused absent, or read as the one-element set. Fix: state that a present member that is not a JSON array (including `null`) is refused with a named cause (most simply `unknown-fairness-kind` naming the received bytes, or `absent-fairness-kinds` for `null`), and add vectors. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:84-97 |
| FND-002 | medium | A backend that advertises no `temporal-satisfaction` pair but carries a `fairness_kinds` member is decided three ways. The unknown and repeat bullets are not scoped to temporal backends, so `["eventual"]` or `["weak","weak"]` is refused. AC-3 is scoped to 'A manifest advertising `temporal-satisfaction`', so it is silent on that case, while AC-4 is unscoped. The absent bullet says such a manifest 'needs no member' but not whether a valid member, such as `["weak"]`, is admitted or is an error (FR-368 and FR-331 attach the member only to temporal backends). Fix: state one rule for a member on a non-temporal backend (refuse it, or validate it and ignore it), scope AC-3 and AC-4 the same way, and add a step-4 vector. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:84-97,113-115 |
| FND-003 | low | AC-3 'naming the received bytes' and AC-4 'naming the repeated label' do not say which one when a member holds two unknown labels (`["eventual","Weak"]`) or two repeated ones (`["weak","strong","weak","strong"]`). The first in member order, all of them, and the bytewise least are all readings, and they give different refusal payloads. Fix: say which (for example the first in member order) and add a vector. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:113-114 |

## Verdict

Not mergeable as is: FND-001 and FND-002 are medium. Each needs one sentence and one vector in FR-134; neither changes the approach.
