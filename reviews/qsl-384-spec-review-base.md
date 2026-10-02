---
id: SR-1130
title: "QSL-384/QSL-385 spec review: infinite-trace profile and unbounded declarations (FR-325 to FR-336)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@253ddd2f2075039ffb18d46d23b78a73478a6542; diff origin/spec/366-temporal-properties...HEAD; spec/functional/FR-325..FR-336; spec/test-cases/TC-835..TC-848; spec/usecase/US-033; ADR-014 A-4 and §10; ADR-018 §3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-334
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-335
    type: reviews
---

## Summary

Ticket: QSL-384 (with QSL-385). PR agent-ix/quire-spec-language#585. QSpec
counterparts checked: FR-360 to FR-368, FR-161 and FR-301 on
origin/spec/wave-b-q1-temporal.

Examined: FR-325 to FR-336 and every AC in them, TC-835 to TC-848, US-033,
the spec.md and tests.md index rows, and the two in-place ADR amendments
(ADR-014 A-4 and §10, ADR-018 §3 EN-1 pre-check).

Checked against the rulings:

- An undefined letter is a violation, exit 10, cause `UndefinedEvaluation`.
  FR-327, FR-328, FR-329 and FR-330 hold to this.
- A missing fairness premise is `unsupported`, as QSpec FR-362 states.
  FR-328, FR-329 and FR-330 hold to this. Replay (FR-331) does not
  (FND-003).
- No exact-revision refusal: FR-334 names a definition revision (FND-002).

`quire validate` on the changed files exits 0, and
`tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-335-AC-5 asks S6a to evaluate `f` on "a concrete set of 1,000 distinct members". `s` is declared `Set<Int[0, 9]>`, which holds at most 10 distinct members, so no test can construct that input. TC-845 step 5 repeats it. Use an element type with enough members, or a size of at most 10. | spec/functional/FR-335-settle-a-claim-over-an-unbounded-declaration.md:100; spec/test-cases/TC-845-settle-a-claim-over-an-unbounded-declaration.md:32 |
| FND-002 | medium | FR-334 keys collection nodes by the preimage the root definitions "state at `1-draft.2`". That is a revision pin in normative text: the preimage is whatever the linked bundle supplies (FR-111), as the next sentence already says. Delete "at `1-draft.2`". | spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md:31 |
| FND-003 | medium | FR-331 replays an observed-trace packet through FR-329, including its fairness check. For a clause with a non-empty fairness set, FR-329 returns `MissingFairnessPremise`. FR-331 then falls into "anything else", which settles `inconclusive`/`ReplayParity`. QSpec FR-362 says such a clause over a supplied trace settles `unsupported`, cause `missing-fairness-premise`. Add that mapping and an AC for it. | spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md:86-101 |
| FND-004 | medium | FR-331 defines a counterexample with observed steps: payload `{steps, fairness, interval, kind}`, with `CounterexampleSteps::Observed{prefix, loop}` and only `kind: Formula`. QSpec FR-364 owns the counterexample wire. It allows exactly `kind`, `initial_state`, `prefix`, `loop`, `over_binding`, `fairness` and `trace_position`, with model steps only. It spells the undefined kind as `{"undefined_evaluation": {where, cause}}`. The QSL shape has no QSpec counterpart. Either get a QSpec FR for observed-trace counterexamples and cite it, or keep the observed payload internal to `qsl-replay` and say that it is not the FR-364 wire. | spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md:57-66; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:459-462 |
| FND-005 | medium | FR-329 restates the weak lasso fairness rule ("fair for a weak constraint when the loop takes it or disables it somewhere"), which QSpec FR-362 owns. It says nothing about `strong` constraints, which FR-362 admits with its own lasso rule. That leaves a `fair strong` clause over a `Lasso::Model` with no defined result. Cite FR-362's lasso table for both kinds instead of restating one. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:87-94 |
| FND-006 | medium | FR-330 adds "the identity and digest of every observation document read, in position order, and the loop start" to the report's provenance, and FR-330-AC-1 tests that the record is there. FR-331's result also retains each document's identity and digest. This is a provenance record. The caller supplied those documents, so nothing breaks without it. Drop the extension and the AC clause. FR-109's base provenance is outside this diff. | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:97-99,105; spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md:70-73 |
| FND-007 | low | FR-327-AC-4 expects `eventually[0,2] holds(c.value = 2)` to exhaust a 2-unit meter. Whether 2 units suffice depends on what counts as a "temporal node" visit, and the FR does not say whether `holds` atoms count. Two implementations could disagree and both claim the AC. State the visit count the AC expects, or pick a meter of 0. | spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md:133 |
| FND-008 | low | FR-332-AC-3 says "as the Behavior table states", but the Behavior section has a bullet list, not a table. | spec/functional/FR-332-settle-an-infinite-trace-item-through-negotiation.md:101 |
| FND-009 | low | Ticket ids appear outside References: "(QSL-43 interfaces)" and "(QSL-42 interfaces)" in Dependencies. | spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md:82; spec/functional/FR-333-read-an-optional-collection-bound-and-population-maximum.md:79; spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md:84 |
| FND-010 | low | FR-334-AC-2 ends "it no longer refuses with `UnrepresentableBound`". That describes a change rather than stating what is. "It lowers with no refusal" says the same thing. | spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md:79 |
| FND-011 | low | In FR-329 the AC table lists AC-7 between AC-4 and AC-5. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:132 |

## Verdict

These parts are sound:

- The trace semantics. The worked ACs of FR-326 to FR-331 all check out by
  hand, including the horizon, the bad prefixes, past reads before
  position 0 and the lasso unrolling.
- The ADR-018 §3 amendment and FR-336: a `ProofBound` never makes an EN-1
  root finite.
- The ADR-014 A-4 amendment and FR-328 to FR-330: a missing fairness
  premise is `unsupported`, exit 21.
- Undefined evaluation: it is a violation everywhere, exit 10, never
  FR-109's `undefined` category.
- FR-331's `revision-mismatch` refusals compare the clause's profile,
  fairness set and interval key with the packet's, as ADR-014 §10 states.
  They are not a revision check.
- Every AC has a behaviour TC.

The PR is not mergeable as it stands. FND-001 makes an AC impossible to
test. FND-002 to FND-006 should be fixed in the same round.
