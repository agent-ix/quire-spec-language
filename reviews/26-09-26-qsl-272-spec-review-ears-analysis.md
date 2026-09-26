---
id: SR-642
title: "QSL-272 EARS review of FR-101"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@84a691bf8beb9df40aa945c48e1e05d7f5fb070b; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
---

## Summary

Ticket: QSL-272. EARS conformance of FR-101's Behavior and its eight ACs.

The ACs are atomic enough to test, each with one TC. They are event-driven
("A run the poll cancels returns …") or ubiquitous ("The state key is …").
FR-101 has no `SHALL` statement. That matches the sibling FR-097, which
also states behaviour declaratively and says it "adds no rule of its own".
It is recorded as low, not as a defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-7's clause "The same model under limits one larger returns `Exhaustive`" reads as universal, and it is false in general. Take TC-455 step 3's graph 0 → {1, 2}, 1 → 3. Under `max_depth` 1 the run is `Bounded` at `Depth`. Under `max_depth` 2 it is still `Bounded`, because state 3 is at depth 2 and a state at depth ≥ `max_depth` is not expanded (explore.rs:180). It needs `max_depth` 3. A test written to the AC's wording on any fixture other than TC-455 step 2's chain fails, and the AC can't be checked as a property. Fix: scope the clause to the chain fixture ("for TC-455's chain 0 → 1 → 2, …"), or state it as "a limit at least the model's own count returns `Exhaustive`". | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:137; qsl-eval/src/simulation/explore.rs:180 |
| FND-002 | low | Behavior and the ACs use no EARS keyword (`SHALL`, `WHEN`, `IF … THEN`). Unwanted-behaviour clauses read as facts, for example "It never returns `Exhaustive` or `Bounded`" and "`requires-bound` is never an `Outcome` variant". This matches FR-097's precedent, so it is not a defect. Optional: rewrite AC-6 and AC-8 as `IF … THEN the engine SHALL …`. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:88-120,136-138 |
