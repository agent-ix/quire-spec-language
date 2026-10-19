---
id: SR-12211
title: "Code review of quire-spec-language PR #1066: each fairness on the annotated quotient, reduced liveness"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0a3c2a78aae79f00dfca2106c36fa09d6229d83a; git diff origin/main...HEAD: qsl-analyze/src/model_check/{quotient.rs, concretise.rs, liveness.rs, check.rs, graph.rs, mod.rs}, qsl-analyze/tests/it/{quotient.rs, toggle_world.rs, graph_core.rs, reduction.rs, reduction_world.rs, main.rs}"
review_set: subset
---
# Code review of quire-spec-language PR #1066

## Summary

Ticket: QSL-541. One review pass over the PR diff: `code-review` with its
`rust-review` lane folded in. Soundness was checked first, against ADR-021
AQ-1 to AQ-8, EI-6 and RV-1.

Examined and sound:

- **Retained edges carry the real `π`.** `retain_reduced` canonicalises every
  initial state and successor through the system's `canonical` hook and keeps
  the permutation on each `RetainedEdge`. `LassoSystem::canonical`
  canonicalises the model component and keeps the property component.
- **Spanning tree, tree words and generators.** The entry is the least
  discovery index. The BFS walks edges in canonical edge order with
  `u_c' = u_c ∘ π⁻¹`, and the generators are the non-identity words
  `u_c ∘ π⁻¹ ∘ u_c'⁻¹` without repeats. `Permutation::compose` is `self ∘
  other`, so the code agrees with AQ-1 and AQ-2.
- **Witness sets and closure.** `W_taken`, `W_enabled` and `W_disabled` are
  read from full enabled sets in the entry frame. Closure is breadth-first over
  the generators. The weak test is `O ⊆ close(W_taken ∪ W_disabled)` and the
  strong test is `close(W_enabled) \ close(W_taken)`.
- **Strong refinement.** `R = { c : B meets u_c(E(c)) }`. `R` is non-empty
  whenever `B` is, because `B` is `H_C`-invariant, so the recursion
  terminates. Sub-SCCs are re-classified, with their weak checks, under their
  own cycle group.
- **AQ-7 loop.** `Walker.closed` uses `tree ∘ π⁻¹ ∘ back` words, which generate
  the same `H_C`, as walkable closed walks. `solve` composes the closed walks
  in reverse path order, so `σ ∘ g_k ∘ … ∘ g_1 ∘ u_c (t) = e`. `wanted` is
  recomputed from the current `σ`. Strong obligations are filtered by the
  `H_C`-invariant owed set, so the filter does not depend on `σ`.
- **Concretisation.** `Lift::edge` checks every reduced concrete step against
  an expansion of the concrete source. The lasso loop is repeated until the
  concrete digest equals the concrete entry digest, at most `order(word)`
  passes, and faults otherwise. Every step counts against the remaining
  `max_transitions`. No reduced counterexample leaves without each step being
  re-executed on the subject.
- **Meter and cancel.** Tree edges, generator words, every permutation
  application and the identity closure in `quotient()` charge the meter, and
  `charge()` polls cancellation.

The soundness hole is in routing, not in the quotient: see FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | False refutation under symmetry with strong fairness when the binding's stabiliser is trivial. `check` lifts the `StrongFairness` refusal whenever symmetry is requested (`symmetric`, liveness.rs:137). But `second_phase` sends a product to the annotated quotient only when `binding.group` is non-trivial (liveness.rs:1548). Every other product goes to `FairModel::build`, which builds `Constraint::weak` for every constraint whatever its kind (liveness.rs:1964-1968). So strong constraints are decided as weak. The stabiliser is trivial in the common case: an `over` parameter bound to a two-key class, or to a key whose class leaves one key. Reproduced at the reviewed sha with a scratch test, not committed: the `Toggle` subject with class `[a, b]`, clause over `x: M::Toggle` under `fair weak each flip; fair strong each fire`, `eventually` every toggle fired. It returns `Violated` with the loop `flip(a), flip(b), flip(a), flip(b)`. Along that loop `fire(a)` and `fire(b)` are enabled infinitely often and never taken, so the loop is not strongly fair. The claim holds, and the PR's own `strong_each_holds_where_weak_each_admits_a_loop_that_never_fires` asserts `HoldsReduced` for it over the watcher binding. RV-7 requires a refutation to be a fair counterexample, and replay re-checks steps, not strong fairness. Fix: route a product to the quotient whenever the clause holds a strong constraint (the quotient handles the trivial group: no generators, identity words), or keep the `StrongFairness` refusal for trivial-group products. Add the trivial-stabiliser strong case as a test. | qsl-analyze/src/model_check/liveness.rs:137-145, qsl-analyze/src/model_check/liveness.rs:1548-1550, qsl-analyze/src/model_check/liveness.rs:1948-1972 |
| FND-002 | medium | `order(&word)` is neither metered nor cancellable. It composes powers until the identity, and the order of a permutation of `n` symmetric keys is the lcm of its cycle lengths. Landau's function gives about 2.3×10⁸ at `n = 100`. Every composition is `O(n)`, and no `cancel.poll()` or meter charge runs, so a large class can hang the lasso build past the run's time budget. FR-153 "Limits" and AQ-8 require every such cost to run under the meter and time budget. The bound is not needed up front: stop the pass loop on the digest match, and let `max_transitions` (already charged per step) or an explicit pass counter checked against the meter end it. | qsl-analyze/src/model_check/concretise.rs:233-241, qsl-analyze/src/model_check/liveness.rs:1802-1814 |
| FND-003 | medium | The strong refinement recurses without a bound on depth. `classify` calls `analyze(&rest)` on the refined set (quotient.rs:391), and `Analysis::first_fair_in` recurses over `sub` (quotient.rs:142-150). Each round removes at least one state, so the depth can reach the SCC's size. A long chain whose refinement peels one state per round can overflow the stack on a large quotient, which aborts the process rather than settling V-7. Use an explicit worklist, or charge the meter and enforce a depth bound that settles `Stopped`. | qsl-analyze/src/model_check/quotient.rs:383-396, qsl-analyze/src/model_check/quotient.rs:142-150 |
| FND-004 | low | The loop-building walk is partly unmetered. `Walker::step`, `tree`, `back` and `cycle`'s inner length walk compose `σ` and follow `toward_entry` with no `charge()`. They are bounded by states times obligations, so this is not a hang, but FR-153 "Limits" asks every word computation to charge the meter. | qsl-analyze/src/model_check/quotient.rs:852-955 |
| FND-005 | low | The quotient analysis gets a fresh `Meter::new(self.request.meter)` per product (liveness.rs:1585), not the run's remaining meter. FR-153 "Limits" says "the run's meter". This matches the engine's per-evaluation meter convention, so it is a wording mismatch, but N bindings get N full budgets. | qsl-analyze/src/model_check/liveness.rs:1585 |
| FND-006 | low | Untagged new tests in a traced repo. `the_analysis_charges_the_meter` (unit), `the_annotated_quotient_charges_the_run_meter` and `concretising_the_loop_counts_against_max_transitions` carry no `#[trace]`. The first two back FR-153-AC-4's meter clause, and the third backs FR-161 "Length and limits". | qsl-analyze/src/model_check/quotient.rs:1425-1443, qsl-analyze/tests/it/quotient.rs:264-317 |

## Verdict

Not mergeable: one major. FND-001 is a reproduced false `Violated`, a
soundness defect in the refutation path. It is reached whenever symmetry is
requested, a strong constraint is present, and the bound key's stabiliser is
trivial. The annotated-quotient core is correct on every point the brief
named: real `π` on retained edges, the spanning tree and tree words, the
closure of witness sets under the cycle group, the strong refinement, the
concretisation of stems and lasso loops with per-step subject checks, the
closure of loops within their order, and the meter and cancel polling in the
analyzer. FND-002 and FND-003 are resource-bound defects (mediums), and
FND-004 to FND-006 are lows. Only FND-001 blocks.

Rust lane: no `unwrap` or `expect` or indexing on library paths. Errors are
typed (`QuotientStop`, `LiftStop`, `LoopStop`) and use the repo's
`InternalFault` invariant-string convention. No new `allow`, no `unsafe`, and
no CI or workflow change. Narrow test run at the reviewed sha:
`cargo test -p qsl-analyze --test it -- quotient::` passed its 5 tests,
plus the scratch probe for FND-001, which failed as described.

## Dispositions
