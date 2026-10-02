---
id: SR-1020
title: "QSL-372 integrity review of ADR-025, FR-219 to FR-229 and their amendments"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@31d4826351932a91059e7e89a5288eb2d7899c7e; diff 404a5a88..31d48263 (the QSL-372 commits on top of the #366 stack); spec/decisions/ADR-025-weak-memory-models-for-parallel.md; ADR-011, ADR-013, ADR-014, ADR-017, ADR-018 amendments; spec/functional/FR-219..FR-229; spec/test-cases/TC-664..TC-674; spec/usecase/US-025; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: reviews
---

## Summary

Ticket: QSL-372 (PR #574). This review checks that ADR-025, FR-219 to FR-229, their TCs and US-025 agree with each other, with the owner rulings RU-1 to RU-6, with the Wave B and Wave C rules, and with the QSpec counterpart FR-434 to FR-439 (QSpec branch spec/wave-b-q5-memory-protocol).

**Consistent:**
- RU-1 to RU-6 are each applied. The ADR depends on ADR-027 (RU-1). MM-2 adds request selection (RU-2). §15 PSC-1 to PSC-6 and FR-223 add RC11 `seq_cst` (RU-3). §14 and FR-224 admit non-atomic accesses with a derived race-freedom item (RU-4). MB-2 and FR-225 set the bounds to 4 as request budgets with no ceiling (RU-5). Relaxed accesses are kept (RU-6).
- An undefined evaluation settles refuted with `UndefinedEvaluation` (MV-1 and FR-227). No new result kind is added.
- No fixed cap is added and depth is never a limit kind. "Depth" names a message position only.
- No pins, ledgers, provenance records or compatibility paths are added. Ticket ids appear only in References.
- The §10 state counts (16, 38, 18) and both counterexamples were rechecked by hand and agree.
- Every AC has a TC row, and each TC tests behaviour.
- `quire validate` on the 31 changed files exits 0. `tools/check-index-completeness.sh` passes.

The FR-205 to FR-218 and ADR-027 references resolve only after #575 lands. That is a merge-order item, not a finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The MP shape `r1 := flag; r2 := data` loads `data` whether or not `r1` read 1. When `r1` reads the initial flag message, nothing synchronises, so `r2 := data` races with `data := 1` even with a `release`/`acquire` flag (RC11 race definition, DR-2). The race-freedom item therefore settles `refuted`, not `proved`. To make the AC hold, guard the data load on `r1`'s binder (a choice on `r1 = 1`, which MA-6 permits). The same error appears in ADR-025 §10 "Message passing", in FR-227-AC-3's "forged race" case, and in QSpec FR-438-AC-1. | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md:108; spec/decisions/ADR-025-weak-memory-models-for-parallel.md:477-483; spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md:94 |
| FND-002 | high | Under MA-1, a location is shared only when attempts in two or more branches touch it. In this AC, `y` and `z` are written by one branch only, so they are branch-local and apply in place (MA-5). Only `x` is ever buffered, so `max_store_buffer` 2 is never reached, and both runs settle V-1. The AC cannot pass as written. Make `y` and `z` shared, or store repeatedly to `x`, as QSpec FR-439-AC-2 does. | spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md:89 |
| FND-003 | high | The new branch `w` stores to `x`, but no other branch touches `x`, and `x` is not in `Chatter`'s universe (`{k}` with field `flag`). Under MA-1 `x` is therefore branch-local, so no buffer `[x := 1]` ever exists. The `duplicate`/`lose` lasso is also already rejected by `Chatter`'s scheduler fairness: FR-212-AC-2 admits it only under `scheduling adversarial`. The AC never exercises flush fairness. | spec/functional/FR-228-derive-memory-fairness-constraints.md:83 |
| FND-004 | medium | This AC reuses FR-228-AC-2's lasso but names `s`'s buffer and `flush(s)`. In FR-228-AC-2 the storing branch is `w`, and `s` performs a `send`, which `tso` gates on an empty buffer. The AC also inherits FND-003. | spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md:95 |
| FND-005 | medium | "After both stores its buffer is `[x := 1, y := 1]`" needs `y` to be shared. The AC names another thread's load of `x` only. As written `y` is branch-local and never enters the buffer. State that another branch accesses `y`. | spec/functional/FR-221-explore-the-x86-tso-memory-model.md:76 |
| FND-006 | medium | The MX-4 `tso` precondition requires "every shared load at least `acquire` and every shared store at least `release`". This contradicts MX-2, MX-3 and DR-1, which admit non-atomic shared accesses under `tso` and let a non-atomic model access bind to a plain access. FR-226 repeats the clause. Limit it to atomic accesses, and rely on the race-freedom item for non-atomic ones. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:328; spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:88-89 |
| FND-007 | medium | Request selection does not refuse two entries that name the same `parallel`. QSpec FR-434 refuses that case, `invalid_runtime_input`/`invalid-value`. With two entries for one `parallel`, the resolved model is undefined in QSL. Add the refusal to MM-2, FR-219 and FR-219-AC-2. | spec/functional/FR-219-declare-and-select-the-memory-model-of-a-parallel.md:83; spec/decisions/ADR-025-weak-memory-models-for-parallel.md:168 |
| FND-008 | medium | `WeakAccessShape.shape` is `MultiLocation` or `JoinPolicy` in QSL (MV-1, FR-220). QSpec FR-439 also has a `shared-read` shape. The two repos disagree on the cause's variants. Pick one set; MA-6 suggests QSpec's extra variant is the one to drop. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:59,97; spec/decisions/ADR-025-weak-memory-models-for-parallel.md:350 |
| FND-009 | medium | The branch is stacked on a stale copy of #366 (404a5a88). It lacks #366's head commit e6a5fb56, which drops the Kani tool pin wording from ADR-014 B-5, adds `max_store_buffer` and `max_messages` to FR-126, and rewrites the ADR-018 rows this PR amends. This PR's B-5 amendment still carries "the AD-016 Kani tool pin". Rebase onto the current #366, and keep #366's B-5 text when resolving the conflict. | spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:136 |
| FND-010 | low | FR-225 orders `MemoryBoundReached` before `InstanceBoundReached` (ADR-027). ADR-025 MB-4 states only `ConstraintReached` > `MemoryBoundReached` > `BoundReached`. Amend MB-4 so the ADR and the FR agree. | spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md:80-82; spec/decisions/ADR-025-weak-memory-models-for-parallel.md:284 |
| FND-011 | low | `MemoryBoundReached{bound, states}` names the budget and its value but does not say how to raise it. The Wave B rule requires a reached limit to name how it is raised, and QSpec FR-439 names "the request member that raises it". State that `bound` names the request member. | spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md:57 |
| FND-012 | low | The Description says S3 warns of load-buffering shapes "under `ra`". S3 does not know the resolved model, which the request picks. The Behavior and ADR-025 MX-4 report every shape regardless of model. Drop "under `ra`". | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md:34 |
| FND-013 | low | The Inputs list `kind` as `Formula`, `Deadlock` or `Race`, but the Behavior also replays `kind: UndefinedEvaluation`. | spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md:54-56 |
| FND-014 | low | FR-222-AC-2 and TC-667 step 2 run GenMC inside a QSL test, which makes the QSL gate depend on an external checker. The litmus vectors are QSpec's. Record GenMC's outcomes once as QSpec conformance vectors and assert against those. | spec/functional/FR-222-explore-the-release-acquire-memory-model.md:91 |

## Verdict

The design follows every owner ruling. It is internally coherent apart from the AC fixtures. Three high findings are ACs that cannot pass as written. FND-001 (the MP race) is also wrong in ADR-025 §10 and in QSpec FR-438-AC-1. FND-002 and FND-003 are fixtures that forget MA-1's sharing rule. The branch also needs a rebase onto the current #366 (FND-009). The PR is not mergeable as it stands.
