---
id: SR-16301
title: "Spec review of quire-spec-language PR 1360: STD-400"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ffb65e4c; spec/decisions/ADR-026-dense-time.md; spec/functional/FR-056-admit-domain-package-model-declarations.md; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md; spec/test-cases/TC-147-account-and-reproduce-domain-package-intake.md; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-147
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-685
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1078
    type: references
---

# SR-16301: Spec review of quire-spec-language PR 1360 (STD-400)

## Summary

Ticket: STD-400. Head ffb65e4c (PR 1360, contains 1355 head f74f95ac). Own delta: commits ab10bc0d, 316d64d1 and ffb65e4c (ADR-026, FR-056, FR-230, FR-393, TC-147, TC-685, TC-1078). The clock-place model (call by place, expansion of clock-reading declarations, guarded `Option<Clock>`, slots for collections with clock activity) is stated consistently across FR-230-AC-9 to AC-23: the static dimension of AC-19 is 3 + 2 + 0 + 2 + 2 + 2 = 11 and AC-23's is 3 + 2 = 5, `Set<Clock>` holds one element by data part, `=` / `contains` / `count` over clock collections refuse, and a `Map<Timer, Int>` key refuses. The `clock-in-collection` refusal and the 'Not yet specified' paragraph are removed with no stale reference at the tip. Cause names (`clock-bound-not-constant`, `clock-read-outside-constraint`, `clock-write`, `time-invariant-shape`, `clock-without-dense-time`) exist in the QSpec stack (QSpec PR 410).

Also checked:

- FR-230-AC-9..17, 19..23, FR-393-AC-8, FR-056-AC-44/45 are unique across the other open PR heads. Gate exit 0, 20 warnings (no new). Merge-tree with main clean; conflicts with open PRs 1335, 1333, 1314, 1304, 706.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | TC-685's scope line says steps 1 to 7 and 10 to 21 'need a header that can select `quire.temporal.complete/v1` and wait for it'. At this head all of FR-230-AC-9 to AC-23 are specified but cannot run until STD-421 (PR 1374, next in the chain) makes the identity selectable. The scope itself is complete and PR 1374 removes the wording, so this is an ordering dependency and not a dropped feature; it is flagged because the PR body does not state that 1360 and 1374 must land together. Fix: state the merge-together dependency in the body. | spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md:15 | missing-requirement |
| FND-002 | low | Lead defaults 7 and 8 keep three refusals: a `reference` `target` of `Clock` (FR-056-AC-44, `malformed-declaration`), a constraint `appliesTo` of `Clock` or a unit, and a package variant `payloadType` of `Clock`. The ticket lists parameters, results, records, aliases and options, none of these, and a reference names an object while a bounded quantity is not a type form, so none removes a capability the ticket asks for. Recorded so the lead can rule on them under the standing no-narrowing rule. | spec/functional/FR-056-admit-domain-package-model-declarations.md:1055 | missing-requirement |

## Verdict

**Mergeable after** FND-001 (medium) are fixed or dispositioned by the lead.
