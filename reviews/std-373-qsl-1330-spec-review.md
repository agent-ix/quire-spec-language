---
id: SR-16317
title: "spec-review of deterministic component certificate order and FR-339 charges (quire-spec-language PR 1330)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@db094fa7e4; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-130-decide-strong-fairness-on-the-explicit-state-product.md; spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md; spec/functional/FR-339-check-an-en-1-component-certificate.md; spec/test-cases/TC-521-explicit-state-model-check.md; spec/test-cases/TC-526-an-en-1-component-certificate-is-checked.md; spec/test-cases/TC-646-the-product-closure-checker-rejects-false-certificates.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-130
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-163
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-521
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-526
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-646
    type: references
---

# Spec review of deterministic component certificate order and FR-339 charges (quire-spec-language PR 1330)

## Summary

Ticket: STD-373. One spec-review pass over `git diff origin/main...db094fa7e4` (agent-ix/quire-spec-language#1330), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- The order rules make the certificate a total function of subject, item and limits: closure and component states ascending by `ProductStateRef`; bindings in FR-125 order; components by Kahn order keyed on least FR-101 breadth-first index; constraint lists ascending by resolved index; the checkers accept any topological order. FR-163-AC-9 (byte-equal reruns and the swapped-unjoined-components case) and the `UnfairStrong` before `UnfairVisibility` witness order agree with FR-130.
- FR-339 no longer carries the sentence that charged "those same existing budgets" and now reconciles with FR-338 ("does not reset ... or charge the same fact twice"); Cancel is checked at each charge, component, listed state and computed edge; the stop payloads are typed.
- No deferral or follow-up wording in the diff; the "Code notes for the QSL-895 coder" in the body are implementation hints, not spec changes.
- Ids FR-126-AC-18, FR-163-AC-9, FR-339-AC-6 are unused on main and other heads.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The charging rules the PR adds are not what AC-6 tests. The "one below" half stops at `max_states` while the closure check, which runs first and charges every product state, is still in progress, so it passes for an implementation that charges component work to the wrong counters. The exact-count half shows only that component work adds no charge to the three counters. No vector exercises the rule that the FR-120 evaluation meter is charged once per model state for enabled-set or fairness-enabledness evaluations the component rules perform, or that a value the closure check already evaluated is reused with no charge. Fix: add a vector with a meter limit equal to the closure check's count that is accepted, and one with the limit one below that stops during component work with `LimitReached` naming the meter setting. | spec/functional/FR-339-check-an-en-1-component-certificate.md | missing-requirement |
| FND-002 | medium | The Fork vector claims "every component holding a phase-2 state precedes every component holding a phase-1 state whatever their vertex indices", but the stated rule only orders components joined by edges. Derive the product for `always eventually holds(f.phase = 1)`: negation automaton states q0 (any) and q1 (not phase 1 forever, accepting). The retained product holds (2,q1), reachable from (0,q0) and (0,q1) by `right`, with no successor because `back` would read a phase-1 letter in q1. Canonical breadth-first indices (initial (0,q0)=0 and (0,q1)=1, then (1,q0)=2, (2,q0)=3, (2,q1)=4) give Kahn order (0,q0), (0,q1), (2,q0), (1,q0), (2,q1): the phase-1 component (1,q0) precedes the phase-2 component (2,q1), so the claim fails for a correct implementation. Fix: state the pairwise precedence that the rule implies ((2,q0) before (1,q0)) or give the full component list in order for the fixture. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | wrong-requirement |

## Verdict

Mergeable after FND-001, FND-002 (medium) are fixed or dispositioned by the lead.

Conflicts with open head #1327 (FR-163 AC rows); clean against origin/main.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | The Ring vector now exists and is correct as a graph: 5 product states, 9 edges, 2 automaton states, components {(0,q0),(1,q0),(2,q0)} MissingAcceptance, {(2,q1)} Trivial, {(0,q1)} UnfairWeak naming a, in topological order (rederived by hand). The meter half is not realizable as written. FR-339 says the closure check evaluates "the successor expansion and the atoms of each state and no constraint's enabled set", so the third component charges E once. But FR-130 Enabled sets defines a state's enabled set as "the labels of the state's FR-120 transitions, at no extra contract evaluation", and FR-126 says an identity is enabled when FR-120's successor relation gives it a successor. The closure check already expands every model state, including v 0, so by those definitions the enabled set of v 0 is already evaluated and component work reuses it at no charge: E is 0 and M + E - 1 stops inside the closure check, not during the third component. Fix: either state that FR-339's checker evaluates an enabled set separately from the expansion it already reads (and amend FR-130/FR-126 for the checker), or drop the meter-stop half and keep the exact-count half. |
| FND-002 | fixed | Round 1, head 01165a07. with canonical breadth-first indices 0, 1, 2, 3, 4 in that order, and a certificate whose five singleton components are listed `(0,q0)`, `(0,q1)`, `(2,q0)`, `(1,q0)`, `(2,q1)`: the edge `(2,q0)` to `(1,q0)` lists `(2,q0)` before `(1,q0)` although `(1,q0)` has the smaller vertex index, and `(2,q1)`,  |
