---
id: SR-16232
title: "spec-review of memory and instance bounds on the certificate request and replay subject (quire-spec-language PR 1343)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f603deea; spec/functional/FR-338-check-an-en-1-closure-certificate.md; spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md; spec/functional/FR-217-replay-a-protocol-counterexample.md; spec/test-cases/TC-1200-certificate-check-and-replay-run-under-the-run-s-memory-and-instance-bounds.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: references
---

# Spec review of memory and instance bounds on the certificate request and replay subject (quire-spec-language PR 1343)

## Summary

Ticket: STD-382 (QSL-934). This is one spec-review pass over
`git diff origin/main...f603deea`. It covers FR-338's request members
`max_store_buffer`, `max_messages` and `max_live_instances`, the "Memory
bounds" and "Instance bound" sections with the cut-state `SuccessorMissing`
rule, FR-227's and FR-217's replay-subject bounds and refusals, FR-338-AC-17
and AC-18, FR-227-AC-5, FR-217-AC-8 and TC-1200. The oracles were FR-225
(bound-limited states, V-1 only when none), FR-209 (instance-limited states,
FA-2 enabledness), ADR-025 MB-3, FR-225-AC-2 and AC-3, and FR-209-AC-1, AC-2
and AC-6.

Gates: `make check-spec-validation` at f603deea exits 0, with no finding on a
touched file. The PR merges cleanly with origin/main, but origin/main is not
an ancestor of the head. FR-338-AC-17 and AC-18, FR-227-AC-5, FR-217-AC-8 and
TC-1200 are unique across open PRs. #1311 holds FR-338-AC-12 to AC-14 and
#1310 holds AC-16. A merge with #1311 conflicts textually in FR-338's AC
table, and both sides' rows should be kept.

Examined, clean:

- **Soundness of the cut-state rejection.** If no reached certificate state
  is bound-limited (or instance-limited) under the request's bound, then each
  state's bounded expansion equals its unbounded expansion. A closure accepted
  under the bound is then closed for the unbounded subject, which is the claim
  V-1 asserts. A state the bound cuts has a successor that the checker cannot
  see, so rejecting `SuccessorMissing` there is conservative. Not comparing
  the request's bound with the run's is also sound, because acceptance never
  depends on that bound being the run's. A request with a larger bound
  accepts a closure that the run proved. A request with a smaller bound
  rejects it. Neither result can accept an unclosed set.
- **Vectors.** FR-225-AC-2 settles V-1 at `max_store_buffer` 3 and V-6 at 2,
  so AC-17's accept-at-3 and reject-at-2 follow. FR-225-AC-3's "no store can
  add a second message" under `max_messages` 1 backs FR-227-AC-5's refusal at
  the first store. FR-209-AC-2's guard `{false}` has no `activate`, so
  AC-18's two acceptances follow.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Replay calls the cut step "not enabled", which contradicts FR-209 and FR-225, and the difference reaches the fairness check. FR-217 says an `activate` at a state with `max_live_instances` live instances "is not enabled there". FR-227 says the same of a store the bound cuts. FR-209 states "ADR-018 FA-2 enabledness SHALL read `activate` as enabled there". FR-225 treats cut stores as enabled steps left unexpanded (ADR-025 MB-3). Replay reads enabledness for the lasso fairness check (FR-128 `UnfairLasso`, FR-217-AC-7). A replay that drops the cut step from the enabled set therefore judges a lasso through a limited state as the engine does not. FR-209-AC-6's lasso passes through an instance-limited state where `activate` is enabled and not taken. Fix: say the step cannot be taken under the bound and refuse it with the stated code, and keep enabledness as FR-209 and FR-225 state. Add a vector in which a lasso through a limited state under a weak constraint on `activate` (or `flush`) is judged as the engine judges it. | spec/functional/FR-217-replay-a-protocol-counterexample.md:80-85; spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md:72-78; spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:93-97 | wrong-requirement |
| FND-002 | medium | The new rejections have no defined place in FR-338's rejection order, and the ACs do not name a locus. The cut-state `SuccessorMissing` rule is added as a separate section. It does not say where it ranks against FR-338's other rejections (`BadState`, `ShapeMismatch` and so on, AC-8's least-refused-ref order) when one certificate has both. AC-17 and AC-18 and TC-1200 expect rejection "at a product state where the third store is cut" and "at an instance-limited product state", and several such states exist. A checker that rejects at the wrong state passes. Fix: place the rule in FR-338's rejection order, and name the expected product state (its key or model state) in AC-17, AC-18 and TC-1200 steps 1, 2 and 5. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:737-769,786-787; spec/test-cases/TC-1200-certificate-check-and-replay-run-under-the-run-s-memory-and-instance-bounds.md:44-53 | wrong-requirement |

## Verdict

Not mergeable as written. The cut-state rejection rule is sound, and so is
not comparing bounds, but FND-001 must be fixed first. The replay calls a
cut step "not enabled", which contradicts the engine's enabledness ruling
that the fairness check reads. FND-002 leaves the new rejections unordered
and their locus unchecked. A main merge is needed (clean), and the FR-338 AC
table conflicts textually with #1311.
