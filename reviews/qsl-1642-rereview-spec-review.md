---
id: SR-20261104130241
title: "Spec review (PR #2294 re-review) — QSL-1642 timed population scope"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2294 head dd4c417ee: spec/functional/FR-230, FR-391, FR-480; spec/test-cases/TC-1334, TC-685"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
---

## Summary

Ticket: QSL-1642. The ticket asks the spec to decide whether timed predicates may read named populations, then either implement it or record a refusal. The PR refuses. This is consistent with the owning requirements. FR-391 grants population scope only to unit-level state clauses and `holds` atoms. The `time invariant when` and `urgent when` predicates are model-level declarations whose subject is `self`. The QSpec timed-model requirement (FR-415, cited by ID only) calls `P` a data predicate and says nothing about population scope. FR-480-AC-17's `convert<Config::Call>(self)` was a type-name resolution test, not a workaround that FR-480 relies on, and the PR now says that. The new text is located, typed and cross-referenced in FR-230, FR-391 and FR-480, and TC-1334 is added.

## Verdict

**PASS with findings** — the disposition is right, but the spec does not make clear whether it is final.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-391 now states as a rule that timed predicates "are not population positions", yet FR-230 assigns the refusal the `not-yet-implemented` tag, which by the cause's own definition means "a form S3 does not check yet". Two implementers would read this differently: one as a permanent exclusion, the other as a deferral. Fix: pick one. Either keep the tag and say in FR-230/FR-391 that timed-predicate population scope is deferred (and name its ticket), or treat the exclusion as permanent and state it as the rule without the "yet". | spec/functional/FR-391-name-a-population-in-a-state-clause.md:66-72; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:290-298 |
| FND-002 | low | Wording: "A named declared named population" repeats "named". | spec/functional/FR-391-name-a-population-in-a-state-clause.md:69 |

## Scope

- Examined: FR-230 new bullet on timed-predicate population scope and FR-230-AC-58.
- Examined: FR-391 new paragraph.
- Examined: FR-480-AC-17 amended text.
- Examined: TC-1334 (new) and TC-685 scope-line edit.
- Context only: QSpec FR-415 (timed models: `P` is a data predicate; silent on population scope) and QSpec TC-369 TS-13 (population `lookup` inside a `holds` claim, which FR-391 still scopes).
- Context only: the QSL-1642 ticket text, treated as data.
