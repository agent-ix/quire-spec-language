---
id: SR-13609
title: "integrity review of the probabilistic spec follow-ups (PR #1124)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@8a1f1ba219d7c8344c06301b2758cfad1a03fa23; spec/decisions/ADR-028-exact-probabilistic-engine.md, spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md, spec/functional/FR-196-build-the-probabilistic-product.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-199
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-200
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: reviews
---
# SR-13609: integrity review of PR #1124 (QSL-573, QSL-568, QSL-571)

## Summary

Ticket: QSL-573 (also QSL-568, QSL-571). This review checks the diff's
consistency against the ADRs it cites (ADR-024 PF-3 and PF-4, ADR-028
SP-2, PR-4, PR-6, XF-2, XF-4, XF-5, XF-7, WS-1 and WS-2, ADR-013 R-04) and
against the FRs that consume FR-196 (FR-199, FR-200, FR-202 and FR-203).
ADR-028 PR-4's amended key list matches FR-196's key array, which now has
five components. XF-2's `[activated]` factor agrees with the new
unactivated rule. Under R-04 the key is still one complete-octet identity
with no new digest domain. FR-186-AC-3 agrees with FR-195-AC-1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-186's new exception defines the Reach shape as "exactly one `eventually`, `until` or `always` written with no interval and with `holds` operands" and says "Every other event under that profile ... is still refused `unbounded-event`". By that text `eventually eventually holds(P)` and `eventually true` (operands not `holds`) are refused `unbounded-event`. FR-195 (lines 81-88) and FR-195-AC-1 refuse them `not-state-predicate` at the operand, and so does the #1122 code: reach_shape() matches on the root operator, interval and arity only, and Pass::reach refuses each non-`holds` operand NotStatePredicate. Fix: in FR-186 define the shape without the operand condition ("one `eventually`, `until` or `always` at the root, with no interval") and say its operands are judged by FR-195. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:158-168 |
| FND-002 | medium | ADR-028 XF-5 still says the expected-reward threshold is "a threshold `c` of its type" (the reward's). This PR's FR-195 rules that `c` is "not of `R`'s type but of the expectation's": a rational for an integer reward, admitting `rational(6, 5)`, `0` and `rational(-1, 2)`. The FR now contradicts the ADR it implements, although the PR amends ADR-028 PR-4 in the same diff. Fix: amend XF-5 to "a threshold `c` of the expectation's type: an exact rational for an integer reward, a quantity of `R`'s dimension for a quantity reward". | spec/decisions/ADR-028-exact-probabilistic-engine.md:153 |
| FND-003 | medium | The edited FR-192 bullet sends `ExactOnlyForm` only for "a mean-time or expected-reward form" and lists the three causes with no order. FR-195 (lines 123-130) and the #1122 require_statistical also refuse an unbounded `Reach` form with `ExactOnlyForm`, and fix the order EveryScheduler, then ExactOnlyForm, then MissingConfidence. A Reach claim under a workload with confidence parameters has no FR-192 cause, and an every-scheduler claim with no confidence gets either cause by FR-192. Fix: make the FR-192 bullet defer to FR-195 ("When EN-4's negotiation gate (FR-195) refuses a claim, the map SHALL settle the refusal `unsupported` with its cause") and drop the form list. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:84-88 |
| FND-004 | low | FR-196 now gives `ProductAction::PostState` two payload types: the new prose says "`PostState` holds the builder's complete `ProbKey` of the target", and the enum just below still reads `PostState(ProductKey)`. A reader cannot tell whether a certificate's choice holds the in-memory key or its canonical encoding. Fix: say the builder's `PostState` holds a `ProbKey` and the qsl-replay `ProductAction` holds its canonical `ProductKey`, or show both types. | spec/functional/FR-196-build-the-probabilistic-product.md:148-160 |
| FND-005 | low | Stale cross-reference: "The `monitor: Option<u32>` above describes graph-local storage only." The struct above no longer has that field; this PR replaced `ProductKey{monitor: Option<u32>}` by `ProbKey{monitor: Option<MonitorState>}`. Fix: point the sentence at `MonitorState::Formula(Running(local index))`, which is the graph-local index now. | spec/functional/FR-196-build-the-probabilistic-product.md:193 |
| FND-006 | low | FR-196 renames `ProductStateId` to `ProbStateId` and types `initial`'s binding as `Option<ModelInstance>`. The FRs that consume this product still use the old names: FR-199 (`ComponentValue.states: Vec<ProductStateId>`, `GainBias.bias`), FR-200 (`FairEndComponent`), FR-202 and FR-203 (`binding: Option<Binding>`). Fix: rename in those FRs, or state in FR-196 that `ProbStateId` is the `ProductStateId` they cite and `ModelInstance` the `Binding`. | spec/functional/FR-196-build-the-probabilistic-product.md:95-110 |
| FND-007 | low | The PR amends PR-4 so the key carries the `over` binding, but PR-6 still says an unbounded form's "product is the MDP or DTMC itself". With a binding in every key, an unbounded claim `over` two objects has two copies of each model state with different labels; FR-196-AC-10 and the #1122 test each_over_binding_has_its_own_states rely on that. Fix: amend PR-6 to "the MDP or DTMC itself, once per `over` binding". | spec/decisions/ADR-028-exact-probabilistic-engine.md:109 |
| FND-008 | low | The `labels` comment lists the predicates of unbounded forms as "(P; A then B; B)", which leaves out a long-run fraction's P. That is also a product with labels (ADR-028 PR-6, XF-6), and the #1122 ProbProduct.labels docs and the test a_long_run_fraction_product_is_the_chain_with_labels include it. Fix: add "P of a long-run fraction". | spec/functional/FR-196-build-the-probabilistic-product.md:100 |

## Verdict

Not mergeable as is. FND-001 to FND-003 are medium contradictions, each
between an edited statement and another statement in the same spec set.
Each one is a wording fix. FND-004 to FND-008 are low.

## Dispositions

Round 1, at d1bc9d71a.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | d1bc9d71a: the shape is defined by root operator and interval only, and FR-195 judges the operands. This matches reach_shape() and Pass::reach on #1122. |
| FND-002 | fixed | d1bc9d71a: XF-5 now types `c` by the expectation. |
| FND-003 | fixed | d1bc9d71a: the FR-192 bullet defers to FR-195 for the claims and the cause order. |
| FND-004 | fixed | d1bc9d71a: the qsl-replay enum holds ProductKey and the builder's holds ProbKey, stated in one place. |
| FND-005 | fixed | d1bc9d71a: the reference now points at MonitorState::Formula(Running(index)). |
| FND-006 | fixed | d1bc9d71a: FR-196 states that the cited names are aliases (the second option offered). |
| FND-007 | fixed | d1bc9d71a: PR-6 now says the product is built once per `over` binding. |
| FND-008 | fixed | d1bc9d71a: the labels comment now includes the long-run fraction's P. |
