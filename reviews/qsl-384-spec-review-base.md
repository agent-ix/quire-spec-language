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

## New findings (disposition pass 1)

Reviewed at 2120f55561c50e370f540db8a983acc1f0c4fde3 (`git diff 0ecfb9e9...2120f555`).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | TC-837 was not updated for the new FR-327-AC-4. Step 4 still runs "a meter of 2 work units, then unlimited", and its expected result still reads "then the work charged equals the visit count". AC-4 now expects 4 visits: a meter of 3 gives `Incomplete`, and a meter of 4 gives `Completed(true)` charging 4. A test written from TC-837 would check the old criterion. | spec/test-cases/TC-837-s6a-evaluates-a-temporal-clause-over-a-finite-trace.md:27-28,38-39 |
| FND-013 | low | Some new text states what is absent instead of what is. FR-330 Outputs: "with no provenance record of the source, the selection or the observations read". ADR-014 §10: "neither keeps a provenance record of its inputs". FR-331 Description: "It is not the QSpec FR-364 counterexample wire". The positive statements beside each one already say what the report and the result hold. Delete the negations. | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:60-62; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:482-483; spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md:46-49 |
| FND-014 | low | Follow-up work is recorded as prose in the spec instead of as a ticket. FR-331's Description and References, and ADR-014 §10, say "a QSpec wire form ... is a follow-up against FR-364", with no ticket. That puts roadmap text in normative sections. FR-331's Status records a code deletion that belongs to FR-109, and FR-109's Status points to it. File Linear tickets (the QSpec wire form; deleting `ClauseRunProvenance` and `source_digest`), cite them in References, and remove the follow-up sentences from the Description and the ADR. | spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md:46-49,137-143,148-150; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:466-469; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:173-177 |
| FND-015 | low | TC-468 step 6 still expects a `compile`, `missing_import`/`missing-selection` result for the extracted missing-model run. The rewritten FR-109-AC-6 has no clause for that case. Add the clause to AC-6, or drop the expectation. | spec/test-cases/TC-468-spine-clause-run-reports-typed-dispositions.md:69-70; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:159 |

## New findings (disposition pass 2)

Reviewed at 92070da9804a6951c5b353c942042961d185cf3c (`git diff origin/spec/366-temporal-properties...92070da9`). First drafted at 5f8d712e, before the rebase onto #562's ed7bcc8b; each finding was re-checked at 92070da9 and is unchanged there.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-016 | medium | FR-331 defines the payload as `TemporalCounterexample{steps, fairness, interval, kind}` with `CounterexampleSteps::Observed{prefix, loop}`, says FR-128 and FR-331 "share the payload type", and calls `CounterexampleSteps::Model` "FR-128's step content". FR-128 and ADR-018 CX-2 now make `TemporalCounterexample` exactly QSpec FR-364's wire (`kind`, `initial_state`, `prefix`, `loop`, `over_binding`, `fairness`, `trace_position`), with no member list of its own and no `steps` or `interval` member. ADR-014 §10 gives a third shape, `TemporalCounterexample{prefix, loop, fairness, interval}`. One type cannot be all three. QSpec FR-364 at main aa031866 now has an observed arm (`trace: "observed"`, `{document}` steps, `interval` as `{lower, upper}` or `{lower, upper: null}` under every profile), so FR-331's "QSpec FR-364's wire carries model steps" and its `interval: Option<IntervalKey>`, `None` under infinite-trace, are both out of line with it. Read the observed payload as FR-364's observed arm, drop the shared-type sentences, and make ADR-014 §10 agree. | spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md:41-48,61-71; spec/functional/FR-128-replay-a-model-counterexample.md:68-73; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:455-467 |
| FND-017 | medium | FR-329's past-reach rule gives `R + b` for a past interval operator with upper bound `b`, and operand reach plus `L` for an unbounded past operator. It has no case for a past operator with an `[a,*]` interval (`once[a,*]`, `historically[a,*]`, `since[a,*]`, `triggered[a,*]`). ADR-018 IV-1 and FR-326 admit these under infinite-trace. For `once[2,*] p` the unroll count `m` is undefined, so the exact lasso evaluation that SM-1 makes the semantics has no rule. Add the case: operand reach plus `a + L`. | spec/functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md:107-112 |
| FND-018 | low | FR-328's safety fragment admits "interval operators" with no qualification. ADR-018 IV-4 admits every closed interval operator and `always[a,*]`, `release[a,*]` and the past `[a,*]` forms, and places `eventually[a,*]` and `until[a,*]` outside the fragment. Cite IV-4's fragment instead of restating it. | spec/functional/FR-328-evaluate-an-infinite-trace-clause-over-a-finite-prefix.md:69-73 |


## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 539b8e9e |
| FND-002 | fixed | 539b8e9e |
| FND-003 | fixed | 539b8e9e |
| FND-004 | fixed | 539b8e9e |
| FND-005 | fixed | 539b8e9e |
| FND-006 | fixed | 539b8e9e |
| FND-007 | fixed | 539b8e9e |
| FND-008 | fixed | 539b8e9e |
| FND-009 | fixed | 539b8e9e |
| FND-010 | fixed | 539b8e9e |
| FND-011 | fixed | 539b8e9e |

Round 2, reviewed at 92070da9804a6951c5b353c942042961d185cf3c.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-012 | fixed | 479b8376 |
| FND-013 | fixed | 2afd7b93 |
| FND-014 | fixed | 2afd7b93 |
| FND-015 | fixed | 2afd7b93 |
