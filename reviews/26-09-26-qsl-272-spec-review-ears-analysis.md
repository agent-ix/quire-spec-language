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

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@be1851894b7dbe9f47a606186f181900e5e6eb3b` (fix commit `be185189`, "QSL-272 spec: fix PR #457 review findings (SR-641 to SR-645)"). I re-checked each outcome against the spec at that head. I did not take any outcome from the commit message. Vectors re-run: the mixed-n vector (n=1 at steps 0 and 1, n=5 at step 2, seed 424242, trace 0) selects 4, and a counter that skips no-draw steps selects 0. The NaN digests for 7ff8000000000000 and 7ff8000000000001 are a3d5ecff68c7cfb60a687aa72b743a04e1dc8513e348b9c3f64393dd96f4bdec and 62c344cca9a4942b80644ca8527bc7ccced905f01bf67262a9bd5e824356a955, and they match TC-453 step 6.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed be185189 | AC-7 now says: "returns `Exhaustive` once `max_states` is at least its reachable state count, `max_transitions` at least its transition count, and `max_depth` greater than its deepest state's depth". This matches explore.rs:180. TC-455 step 2 states the chain's counts (3 states, 2 transitions, depth 2), and step 5 adds 5 states, 4 transitions and depth 2. I checked both against the engine. |
| FND-002 | accepted-no-change | Not a defect when raised. FR-101's Status section now records the declarative style and cites FR-097's precedent. |
