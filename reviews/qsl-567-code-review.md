---
id: SR-12217
title: "Code review (Rust lane) of QSL-567 E16c part: visibility fairness and routing by memory model (PR #1080)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c90b688f3a78524650762e8d052cdd585f14220e; qsl-eval/src/fairness/constraint.rs, qsl-eval/src/fairness/lasso.rs, qsl-eval/src/fairness/mod.rs, qsl-eval/src/fairness/refine.rs, qsl-eval/tests/it/fairness_kernels.rs, qsl-replay/src/certificate/scc.rs, qsl-route/src/lib.rs, qsl-route/tests/it/main.rs, qsl-route/tests/it/memory_models.rs, qsl-foundation/src/lib.rs, qsl-foundation/src/memory.rs, qsl-forms/src/lib.rs, qsl-forms/src/protocol_control.rs, qsl-forms/tests/it/protocol_control_forms.rs, qsl-semantics/src/check/protocol_memory.rs, qsl-semantics/src/check/protocol_memory/tests.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/tests/it/temporal_fairness.rs, tests/it/family_outcome_layering.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-226
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
---
# SR-12217: Code review (Rust lane) of QSL-567 E16c part: visibility fairness and routing by memory model (PR #1080)

## Summary

Ticket: QSL-567 (E16c), partial. PR quire-spec-language#1080 at head c90b688f3, diff `origin/main...HEAD` only (19 files). Reviewed as code-review with the rust-review lane folded in. No gates were run. Narrow tests run at the head: `cargo test -p qsl-eval --test it visibility` (2 passed), `cargo test -p qsl-replay --lib visibility_witness` (1 passed), `cargo test -p qsl-route --test it memory_models` (1 passed), `cargo test -p qsl-semantics --features test-support --test it an_authored_constraint_cannot_name_the_flush` (1 passed).

What was checked:

- **Visibility fairness in the three places.** The SCC filter (`refine.rs` `weak_owed`), the lasso check (`lasso.rs` `check_loop`) and the certificate checker (`scc.rs` `UnfairWeak`) all read `Obligation::Visibility`. Each one is discharged only by an edge that takes `fresh`, the loads of the mo-last message. The lasso check owes the constraint per loop: it is owed when the loop loads the pair. The SCC filter and the certificate checker owe it per component: it is owed when *any* edge inside the SCC loads the pair. These are different rules, and they disagree. See FND-001.
- **Discharge.** In all three places only `is_taken_by`, membership in `fresh`, discharges the constraint. A stale load never discharges it. `Constraint::visibility` adds `fresh` to `loads`, so a fresh load also makes the constraint owed. A stutter edge neither makes it owed nor discharges it. Correct.
- **Routing (FR-226).** `decides_models` is `!any(is_weak) || resolved ⊆ advertised`. A weak model is never replaced by a weaker or stronger one. When no backend advertises the models, the candidate set is empty and is returned through the ordinary `Candidates` path. `candidates_for_fairness` delegates with an empty `resolved`, so existing callers behave as before. Correct.
- **Type move.** `MemoryModelName` is now defined only in `qsl-foundation::memory` and re-exported from the foundation crate root, which is its own home. `qsl-forms` no longer names it in `pub use`, and no `qsl_forms::MemoryModelName` path remains in the workspace. There is no compatibility re-export or shim. Correct.
- **Rust idioms.** There is no new `unwrap`/`expect`/`panic` in production code. `with_memory_models` is `#[must_use]`. The new `Obligation` match arms are exhaustive. The tests carry `#[trace]` tags.

FND-001 is a major soundness defect. I reproduced it with a scratch test that I reverted afterwards, so it is not on the branch. The rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | MAJOR. The SCC filter and the certificate checker reject a fair accepting cycle under visibility fairness, and the lasso check disagrees with them. `weak_owed` makes the constraint owed on the whole SCC when any edge inside it loads the pair. Then `UnfairWeak` declares that no fair accepting cycle exists in the SCC. But the obligation belongs to a cycle: a cycle in the same SCC that never takes a load edge owes nothing and is fair. Reproduction: vertices a and b, edges a→b `STALE`, b→a `OTHER`, and a→a `OTHER`, with `fresh = {FRESH}`. `analyze` returns `[UnfairWeak { constraint: 0 }]`, but `check_loop` on the self-loop a→a returns `Ok(Fair)`. If a is accepting, the filter (and an accepted certificate) settles `proved` even though a fair counterexample exists. This is a wrong verdict. Enablement weak fairness is sound in SCC form, because every cycle in the SCC passes only through enabled vertices. "Loads infinitely often implies a fresh load infinitely often" is an edge Streett pair, so when no fresh edge is inside the SCC, the load edges have to be removed and the sub-graph decomposed again (in the same way as ADR-019 strong refinement), not the SCC rejected. The certificate witness then also needs a refinement form. The rule follows the SCC wording of ADR-025 MF-2, FR-228-AC-3 and QSpec FR-437 literally, so the spec has to be amended together with the code. | qsl-eval/src/fairness/refine.rs:185-195,136-142; qsl-replay/src/certificate/scc.rs:345-356; qsl-eval/src/fairness/lasso.rs:68-72 |
| FND-002 | low | `is_enabled_in` returns `true` for every `Visibility` constraint ("reads as enabled everywhere"). It does this so that the enablement-based paths do nothing for it. A real meaning is overloaded to hide a code path. `(Strength::Strong, Obligation::Visibility)` in `check_loop` is reachable only by construction, since `visibility()` always builds a weak constraint, and it would then demand the constraint be taken on every loop. Match on the obligation in place of the sentinel, or make the strong-visibility combination unrepresentable. | qsl-eval/src/fairness/constraint.rs:134-139; qsl-eval/src/fairness/lasso.rs:73-75 |
| FND-003 | low | The docs of the certificate module and of `ComponentWitness::UnfairWeak` still say that the witness names a weak constraint "enabled at every state". For a visibility constraint the checker now applies the load-owed rule. The FR-339 rule list and the docs no longer describe what the checker accepts. | qsl-replay/src/certificate/scc.rs:17-18,84-85 |

## Verdict

**FAIL: not mergeable, one major (FND-001).** Discharge (fresh load only) is correct in all three places. The SCC filter and the certificate checker agree with each other, and the lasso check is correct for each loop. But the SCC-level obligation ("owed if any edge in the SCC loads the pair") is unsound, because a sub-cycle that avoids every load is fair, the lasso check confirms this, and yet the filter rejects it. That can produce a false `proved`. The kernel has no production caller yet, because the FR-228 derivation is not built, so nothing ships wrong today. But the PR delivers FR-228-AC-3's rule, and that rule is wrong. Routing by memory model is correct: it never weakens the model, and it returns an empty set when nothing matches. The type move has no shim. FND-002 and FND-003 are low.

## New findings (disposition pass 1)

Round 1 reviewed at 220ee2b43e1d460dc23c9279cdd0c69e55b3d37c against the merged spec (ADR-025 MF-2 and MF-5, FR-228 Behavior and AC-3 to AC-9, FR-339, ADR-019 FS-2), merged on main by bf027b4fb (#1084) and fbfa6e303 (#1083). Narrow tests at this head: `qsl-eval --test it fairness_kernels` (30 passed), `qsl-replay --lib certificate::scc` (13 passed), `qsl-analyze --lib liveness` (5 passed), `qsl-route --test it memory_models` (1 passed).

The original repro (a→b `STALE`, b→a `OTHER`, a→a `OTHER`, all states accepting) now gives `UnfairVisibility{[0], sub: [{a} Fair, {b} Trivial]}` with `removed = {10, 11}`, and `first_fair_in` returns `{a}`. The load-free self-loop is found as a fair accepting cycle, which agrees with the lasso check. The certificate checker is sound for the nesting:
- `UnfairVisibility` adds its pairs' loads to the level's removed set.
- `UnfairStrong` keeps its parent's removed set.
- A removed edge is exempt from `BackwardEdge` and from every rule below its witness, and nowhere above it.

A cycle that crosses sub-components on a removed edge loads a pair that has no fresh load in the enclosing component, so that cycle is unfair. A probe certificate shaped as the spec's one combined refinement step (`UnfairStrong` over `UnfairVisibility` over trivial leaves) is accepted.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The combined strong-plus-visibility step builds a witness tree that the checker rejects. The step removes the loads of `visibility`: the pairs that fail on the whole component C. But the wrapper `UnfairVisibility` over the remaining states lists `still = failing_visibility(&remaining, removed)`. That is recomputed on the remaining states, and it also contains a pair whose only fresh-load edge touched a removed vertex. The step did not remove that pair's loads, so `refined` decomposes with those edges present and classifies the pair again one level down. The tree then has `UnfairVisibility{P}` directly inside `UnfairVisibility{P}`, and the checker rejects the inner one with `WitnessFails`, because its R edges are already removed at that level. Probe: states 0, 1, 2; strong `s` enabled only at 0 and never taken; edges 0→1 `FRESH`, 1→0, 1→2 `STALE`, 2→1; states 1 and 2 accepting. The filter's verdict is still correct (no fair component). The same tree written as a certificate is `Rejected(WitnessFails)`, and the spec-shaped tree is `Accepted`. QSL builds no certificate from `Analysis` today, so nothing ships wrong. But the module claims to keep "every witness the certificate checker reads", and the tree does not match FR-228's "every failing visibility pair in one refinement step". Fix: set `still` to `visibility` filtered to the pairs that a kept edge inside `remaining` still loads. | qsl-eval/src/fairness/refine.rs:213,247-259 |

## Dispositions

Round 1, reviewed at 220ee2b43e1d460dc23c9279cdd0c69e55b3d37c.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 18c9bafd2 / 220ee2b43: the SCC filter and the certificate checker refine by removing a failing pair's load edges and decomposing again (`UnfairVisibility{constraints, sub}`). The CX-5 walk uses only kept edges. The repro's load-free self-loop is now the first fair component. |
| FND-002 | fixed | 18c9bafd2 / 220ee2b43: `is_enabled_in` returns `Option<bool>` (`None` for visibility), and `Obligation::Enablement(Strength)` makes a strong-enablement visibility constraint unrepresentable. Visibility reports kind strong (MF-5). |
| FND-003 | fixed | 18c9bafd2: the module doc and the `UnfairWeak` and `UnfairVisibility` docs state the enablement and load-edge rules separately. |
