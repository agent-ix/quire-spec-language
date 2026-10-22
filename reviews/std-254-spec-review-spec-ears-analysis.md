---
id: SR-13771
title: 'EARS review of QSL PR #1182: FR-329 bounded-profile clause over a zero-counted-loop lasso (STD-254)'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@7d7a924b7; git diff origin/main...7d7a924b7 (PR #1182, branch std-254-spec): spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md, spec/test-cases/TC-839-s6a-evaluates-an-infinite-trace-clause-over-a-lasso.md; ticket STD-254'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-329
  type: reviews
---

## Summary

Ticket: STD-254. PR: quire-spec-language#1182, head 7d7a924b7. Method: spec-review, spec-ears-analysis. FR-329's lead statement is unchanged and stays an event-driven EARS statement ("When the TemporalTrace evaluator (FR-327) evaluates a clause under `quire.temporal.infinite-trace/v1` over a lasso, it SHALL ..."). This review checks the new Description sentence, the three bullets of the new Behavior section, and AC-8 to AC-10. It looks for a system subject, one trigger and one response, measurable terms, and behaviour-only wording. `make check-spec-validation` raises no `ears:` warning on FR-329.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-329 Description, new sentence | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:38-41 | examined |
| FR-329 Bounded-profile section, bullets 1-3 | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:136-159 | examined |
| FR-329-AC-8, AC-9, AC-10 | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:172-174 | examined |
| FR-329 lead statement | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:31-38 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new Description sentence "A bounded profile reads a lasso in one case only ... It refuses every other lasso." has no SHALL and makes the profile the actor. The behaviour belongs to the evaluator: "Under a bounded profile, the evaluator SHALL read a lasso only when ...". | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:38-41 |
| FND-002 | low | Bullet 1, "A clause under a bounded profile SHALL read a lasso only when ...", makes the clause the subject of SHALL. The next sentence makes the evaluator the subject. Both obligations are the evaluator's, so the subject should be the evaluator in both. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:136-138 |
| FND-003 | low | Bullet 2 opens with a statement of fact with no SHALL ("The loop of a zero-counted lasso keeps the interval distance constant"). Its obligation, "An interval operator ... SHALL therefore not be met", is passive with an operator as subject. The bullet should say what the evaluator returns, for example "the evaluator SHALL evaluate `eventually[a, b] p` with `a >= 1` false at an activation from which only the loop remains". | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:144-150 |
| FND-004 | low | Bullet 3 combines three obligations: decide fairness by the clause's set, refuse an unfair lasso, and treat a fair unmet lasso as a refutation, the last stated without SHALL. Each needs its own statement so that AC-10 and AC-8 can each trace to one. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:152-159 |

## Verdict

Not clean, all low. The criteria AC-8 to AC-10 are direct, measurable assertions with named outcomes, and the new text has no changelog, status or review language. The four findings are actor and atomicity issues in the new Behavior prose. None of them changes what the evaluator must do.

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@9eb06cde9` (fix commit `9eb06cde9`). I checked each outcome against the spec at that head. `make check-spec-validation` exits 0 with no `ears:` warning on FR-329.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed 9eb06cde9 | The Description sentence now reads "Under a bounded profile, the evaluator SHALL read a lasso in one case only: a `Lasso::Model` over a protocol subject whose loop has no counted step (ADR-027 PB-3), ... and SHALL refuse every other lasso." |
| FND-002 | fixed 9eb06cde9 | Bullet 1 now reads "Under a bounded profile, the evaluator SHALL read a lasso only when ..." and the refusal is its own bullet with the evaluator as subject. |
| FND-003 | fixed 9eb06cde9 | The constant-distance sentence stays as the stated fact, and a separate bullet says "the evaluator SHALL evaluate the clause false, because no position lies at a distance in `[a, b]`". |
| FND-004 | fixed 9eb06cde9 | The fairness bullet is split: reading a scheduler constraint as taken and enabled (PA-2), refusing an unfair zero-counted lasso, and returning `Completed(false)` for a fair unmet lasso are three bullets, each with its own SHALL. |
| FND-005 | fixed f18e5c026 | FR-329 no longer states the obligation; it reads "FR-217 states the replay of a protocol lasso under a bounded profile whose loop has a counted step", and FR-217 states it with the executor as subject: "the executor SHALL settle `inconclusive` with cause `NoValue`". |

Disposition pass 2 at `agent-ix/quire-spec-language@f18e5c026` (fix commit `f18e5c026`). The FND-005 row above is pass 2's. `make check-spec-validation` exits 0 with no `ears:` warning on FR-329 or FR-217.

## New findings (disposition pass 1)

These were found at `9eb06cde9`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new bullet "Under a bounded profile, FR-217's replay of a protocol lasso whose loop has a counted step SHALL settle with no truth value" makes FR-217's replay the subject of a SHALL in FR-329, the evaluator's requirement. The obligation belongs to FR-217 and should be stated there, with FR-329 pointing to it, or rewritten as what the evaluator returns that makes the replay settle so. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:149-150 |
