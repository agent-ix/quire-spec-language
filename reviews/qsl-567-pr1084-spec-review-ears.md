---
id: SR-13402
title: 'Spec review of PR #1084: visibility fairness as an edge Streett condition, EARS'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@0455abebf; git diff refs/remotes/origin/main...0455abebf (PR #1084, base main 47047b1df): spec/decisions/ADR-025-weak-memory-models-for-parallel.md, spec/functional/FR-228-derive-memory-fairness-constraints.md, spec/test-cases/TC-673-memory-fairness.md; read against ADR-018, ADR-019, FR-126, FR-339 on the head, QSpec main FR-437, QSpec PR #294 head cb8f1d8, and code PR #1080 head 18c9bafd2; ticket QSL-567'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-228
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-673
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-025
  type: reviews
---

## Summary

Ticket: QSL-567. PR: quire-spec-language#1084, head 0455abebf, scoped to `git diff refs/remotes/origin/main...0455abebf` (one commit, three files, +18/-7). Spec-only. It replaces the whole-SCC visibility-fairness rule of ADR-025 MF-2 and FR-228, under which a load-free sub-cycle of an SCC with a stale load was rejected (a false `proved`, SR-12218 FND-001 on code PR #1080), with an edge Streett condition decided by recursive SCC decomposition. It also restates MF-5, changes FR-228's Description, Behavior and AC-3, and adds FR-228-AC-5 and TC-673 step 5. EARS review of the changed FR-228 Description, the Behavior bullets and AC-3, AC-5. Independent checks. (1) Exactness of the decomposition, by brute force: sr-1084-tools/streett_oracle.py builds random graphs (1 to 5 states, up to 9 edges, self-loops and parallel edges), 1 to 3 visibility pairs with random R and G ⊆ R, a random accepting set and, in half the cases, one weak constraint. The oracle enumerates every edge set, keeps the nonempty strongly connected ones (exactly the edge sets of closed walks), and applies the per-loop (lasso) check. The decomposition is MF-2 with FA-4's acceptance and weak tests. Over 21,000 cases (seeds 1 to 5) the two agree on every case: 0 mismatches, about 40% of cases fair. Removing every failing pair's R at once (FR-228's wording) also gives 0 mismatches in 6,000 cases. The old whole-SCC rule rejects a graph that has a fair cycle in 461 of 6,000 cases (a false `proved`), and it never accepts a graph with no fair cycle. For every passing SCC, the closed walk that covers all of its edges passes the per-loop check, so the SCC form and the lasso form agree. The proof by hand: a closed walk lies in one SCC. A passing SCC holds a covering walk, which takes a G edge for every pair whose R it touches. A fair walk in an SCC with R_i but no G_i takes no R_i edge, so removing R_i loses no fair walk. A failing acceptance or weak test is inherited by every sub-SCC, so rejecting there is exact. Each removal drops at least one edge and empties R_i below it, so the recursion ends, with depth at most the number of pairs plus one. (2) Machine closure: RA-2 lets a load read any message at or after cur_b(ℓ), and the mo-last message always qualifies. An RMW may read it (it has no attached successor). An SC read of it adds no outgoing fr edge, so P stays acyclic and the choice is never pruned. So wherever an R edge is enabled, a G edge is too, and MF-3 holds. (3) AC-5 checked by hand and by the script: R = {q→r}, G = ∅. Removing q→r leaves p→q, q→p and r→q, with SCCs {p,q} and {r} (trivial). {p,q} is accepting and holds no R edge, so it passes. The whole-SCC rule rejects. (4) Repository grep: MF-2, MF-5, 'weak rows', FA-4, CX-5, FS-2, FS-6, FS-7, PC-4, FR-339, FR-126 certificates, TC-673. (5) QSpec PR #294 states the same rule and the same p q r example (AC-7, LT-12). QSpec main FR-437 still states the whole-SCC rule. (6) Code PR #1080 (qsl-567-impl, 18c9bafd2) already has `ComponentWitness::UnfairVisibility{constraints, sub}`, under which an edge a visibility refinement removed is in no rule below it. No spec defines that witness. Methods: spec-review/base (SR-13401), spec-review/ears (SR-13402), spec-review/integrity (SR-13403). code-review and gap-analysis were not run, because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same as main 47047b1df, none in the changed files. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2), newer than the rollout note's 0.36.1/0.50.1.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-228-description | spec/functional/FR-228-derive-memory-fairness-constraints.md | examined |
| FR-228-filter-bullet | spec/functional/FR-228-derive-memory-fairness-constraints.md | examined |
| FR-228-cx5-bullet | spec/functional/FR-228-derive-memory-fairness-constraints.md | examined |
| FR-228-AC-3 | spec/functional/FR-228-derive-memory-fairness-constraints.md | examined |
| FR-228-AC-5 | spec/functional/FR-228-derive-memory-fairness-constraints.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The filter bullet holds four SHALLs (accept only when..., decompose, remove..., SHALL NOT reject) and uses 'fresh load', a term that FR-228, ADR-025 and QSpec FR-437 never define (they say 'a load ... from its mo-last message'). 'remove the loads of every pair' means removing the R edges, not the loads. Split the bullet into one statement for the decomposition and one for acceptance. Say 'a load of ℓ by b from ℓ's mo-last message' and 'remove the edges on which b loads ℓ'. Drop the SHALL NOT sentence, which the decomposition already implies, or make it a note. | spec/functional/FR-228-derive-memory-fairness-constraints.md:66-72 |
| FND-002 | low | 'and that has no other cycle' does not say other than what. It is also only grammatical if 'that' refers to the SCC, not to the message. The vector it means is an SCC each of whose closed walks takes b's stale load of ℓ, so that removing that load leaves no non-trivial SCC. Say so, for example 'an SCC every cycle of which takes a load by b of ℓ, and none of whose edges loads ℓ's mo-last message by b'. | spec/functional/FR-228-derive-memory-fairness-constraints.md:87 |
| FND-003 | low | 'ADR-018 CX-5's walk SHALL take each as a weak obligation' still covers every memory constraint. The changed Description and MF-5 now say visibility fairness is not weak. For visibility, MF-2 makes the walk's obligation one per pair that the passing component still loads, discharged only by a G edge. Restate the bullet so that it matches: flush constraints as weak obligations, visibility pairs as MF-2 states. | spec/functional/FR-228-derive-memory-fairness-constraints.md:73-75 |

## Verdict

Three low wording findings, which can be fixed in the same round as the base findings. The changed Description is one sentence with two SHALLs, as it was on main, and the grammar lint raises no new warning. AC-3 and AC-5 are direct assertions of the outcome. Merge order: FR-228 reads visibility fairness 'as QSpec FR-436 and FR-437 define it', and QSpec main still states the whole-SCC rule, so merge this PR with or after QSpec #294.

## Dispositions

Round 1, head be9f7297d (fix commit be9f7297d over merge 4c1eb98e4, which brings in main 70c7092b9). The fix commit also commits this review's round-0 SR files under reviews/ unchanged; this file supersedes them. Gates: make check-spec-validation exits 0 with 18 warnings against 17 on main 70c7092b9; the one new warning is FR-228 line 73 [ears:non-singular] (SR-13402 FND-001). tools/check-index-completeness.sh passes. Re-run checks (sr-1084-tools): walk_check.py runs a CX-5-style greedy walk (nearest obligation by BFS, canonical edge order, closing path, shortest cycle through the entry when the loop is empty) on the first passing component of 20,000 random cases (seeds 1 to 5, 8,130 passing). Over the kept edges every loop is fair: 0 unfair. Over the induced edges, which is what the round-0 text allowed, 1,197 loops are unfair or cannot be built. FR-228-AC-6: over the kept edges the loop is `p q`; over the induced edges the pair is still loaded and no G edge exists. FR-228-AC-7 gives `{p, q}` under both pair orders. order_check.py: refining one failing pair at a time and refining all failing pairs in one step pick a different first passing component in 4 of 8,078 passing cases. Certificate contract (ADR-018 PC-4, FR-339): checked by hand. A fair loop of the component takes no R edge of a listed pair, since the component has no G edge for it. So the loop lies in the subgraph without those edges, and by (c) over that subgraph it lies in one sub-component, whose witness, checked over the same subgraph with the removed edges exempt, excludes it. Exempting the removed edges from `BackwardEdge`, `Trivial` and `UnfairWeak` below is sound for the same reason. Strong-kind routing (MF-5): consistent with QSpec #294 at ad420fd. FR-437 makes the member `{kind: strong, target: visibility/ra, granularity: whole, origin: memory}`, FR-430 lists it, and FR-368 routes by kind. ADR-019 FS-2's amendment keeps visibility out of rule (c)'s state removal.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The bullet was split and now says 'mo-last message' and 'remove the edges', and the SHALL NOT sentence is gone. But the decomposition bullet (FR-228 lines 70-74) still holds three SHALLs ('SHALL remove ..., SHALL decompose ..., and SHALL repeat'), and the grammar lint now flags it as [ears:non-singular] at line 73. That is the only new warning on the head (18 against 17 on main). Make it one SHALL, for example 'the filter SHALL refine the SCC by removing ... and decomposing what remains, repeating on each SCC'. |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

Round 1 verdict: Not mergeable yet. Every round-0 high and medium finding is fixed: the certificate witness, the kept-edge component and walk, AC-5, and the stale weak-row text. Two new medium findings remain: SR-13401 FND-006 (FR-339-AC-3's vector cannot produce `UnfairVisibility`) and FND-007 (the refinement step is not pinned to QSpec FR-365's single step, so the canonical counterexample can differ). Lows still open: SR-13401 FND-008, SR-13402 FND-001 (the one new lint warning), SR-13403 FND-003 and FND-004. Merge order: with or after QSpec #294.
