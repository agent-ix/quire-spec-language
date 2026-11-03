---
id: SR-2218001
title: "spec-review of per-occurrence clause names and clock bindings (quire-spec-language PR 2218)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md; spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md; spec/test-cases/TC-1326-temporal-clauses-emit-one-name-row-and-one-clock-binding-per-claim-occurrence.md; spec/test-cases/TC-1327-a-hyper-clause-emits-its-name-row-and-clock-binding-on-its-body-wrapper.md; spec/test-cases/TC-1328-a-hybrid-claim-emits-a-name-row-and-no-clock-binding.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-488
    type: references
---

# Spec review of QSL-1592 clause names and clock bindings (quire-spec-language PR 2218)

## Summary

Ticket: QSL-1592. One spec-review pass over `git diff origin/main...HEAD` at head
fea2ddeafd5cb19d3a1c3a62cd5f75e3da7966cb, checked against main db7b3df24 (PR 2216,
`temporal_binding_rows` in qsl-package/src/emit.rs, `CheckedGraph::hybrid_claim_sites()`) and QSpec
FR-322-AC-342..370. The text matches the implementation on every stated point: one name row per claim
occurrence of temporal, hybrid and hyper clauses (hyper keyed on the body wrapper at the assembled
occurrence ordinal); bindings only for clock-configured profiles, per occurrence; refusal for missing,
unused (SurplusTemporalClockSelection) and wrong-kind selections; both arrays sorted by (declaration,
occurrence); omitted when empty; hybrid claims get name rows and no binding. The hybrid statement is
correct: hybrid sites in `temporal_binding_rows` only push names, and ModelTime/ModelSteps is a
HybridClock, not a ClockConfiguration. TC-1326..1328 are unused on main and on the two other open PR
heads (2196, 2197), which touch no spec ids in this range. Test links (Test (TC-n) cells and
`verifies` relationships) agree. `quire validate` exits 0 on all six files. The new ACs are untagged in
`quoin matrix`, expected before the tests land. Out of scope, noted: QSL `ReplayRequestWire` in
qsl-replay/src/request.rs has no QSpec `package` member. Sub-analyses (EARS, criterion strength) were
applied inline by this pass; no separate artifacts.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-337 lists the clock-configured profiles as "event-position, fixed-sample or timestamped-event false-extension", but the timestamped-event profile is `quire.temporal.timestamped-event.finite-window/v1` (TemporalProfile::TimestampedEventFiniteWindow); only event-position and fixed-sample are false-extension. A reader following the text would not recognise the finite-window profile as clocked. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:471-472 | wrong-requirement |
| FND-002 | medium | The FR says the order is "a function of the checked graph and not of declaration order", but no AC or TC step reorders declarations; with two clauses in AC-16/AC-17 a pass that sorts nothing but happens to emit in node order would satisfy "sorted". Add a case that declares the clauses in the reverse of NodeId order and asserts the same arrays. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:481-482 | missing-requirement |
| FND-003 | low | FR-447 says a hybrid claim writes no binding "when its profile is not clock-configured", which suggests a hybrid claim could have a clock-configured profile and a binding. The implementation never binds a hybrid claim; state it unconditionally. | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md | wrong-requirement |
| FND-004 | low | TC-1328 tags FR-322-AC-346 (a claim occurrence with no name row is refused), but no step exercises a missing row; TC-1326 and TC-1327 step 2 exercise rename (FR-322-AC-343) but do not tag it. | spec/test-cases/TC-1328-a-hybrid-claim-emits-a-name-row-and-no-clock-binding.md | correct-requirement-no-evidence |
| FND-005 | low | FR-337-AC-16 and AC-17 each bundle five or more independent outcomes (names, sort, rename, omission, refusal x3, unclocked) into one criterion, so one failing outcome is not attributable. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | wrong-requirement |

## Verdict

Mergeable once FND-001 and FND-002 are fixed in the PR; no high findings, no scope cuts, no
changelog-style text.
