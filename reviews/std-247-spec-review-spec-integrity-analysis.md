---
id: SR-14073
title: "integrity review of STD-247 SC fence lifetime and LT-17 (PR #1194, QSpec PR #333)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@a3c2673b34bb35ab546454473df4188732a87c3d; spec/decisions/ADR-025-weak-memory-models-for-parallel.md, spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md, spec/test-cases/TC-668-rc11-partial-sc-order.md; agent-ix/quire-specification@e69cade8882527e8f53844c9fc14e5b70116ff90; spec/functional/memory/FR-437-explore-release-acquire-memory.md, spec/test-cases/TC-391-litmus-conformance-vectors.md, spec/test-cases/TC-393-memory-budgets-and-verdicts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-668
    type: reviews
---
# SR-14073: integrity review of STD-247 SC fence lifetime and LT-17

## Summary

Ticket: STD-247. The PRs are stacked. QSL #1194 is reviewed as `git diff origin/std-ra-246-qsl...a3c2673b3` (3 files, +14/-4). QSpec #333 is reviewed as `git diff origin/std-ra-246-qspec...e69cade` (3 files, +12). The stack bases (#1193/#1192, #332/#331) are out of scope and are cited only where the new rule depends on them.

I searched for a counterexample to the lifetime rule. PSC-4(e) is sound with respect to the frontier. Every view that covers `f` but not as its latest live fence covers a later live fence `f'` of `t`, and `f → f'` is a `po` edge in `P`. A new view gets coverage only by copying or joining views, and a join takes the later entry, so `f` never again becomes the latest covered fence of any view. Fences get no new incoming edge (PSC-3 adds `x → e` only into SC writes), so a later cycle through `f` enters through an existing predecessor and `f'` carries every outgoing hb edge. What breaks is the input to the rule: RA-5 runs a `seq_cst` fence's release part before the fence becomes an SC event, and the code does the same (ra.rs:832 copies `cur` into every `rel` view before `sc_event`). So no release view ever records the fence that wrote it (FND-001, FND-002).

LT-17 recomputes by script (litmus.py next to this file) as forbidden under `sc`, `tso`, RC11 and the PSC-3 edge set. A hand trace through the engine with (e) in place of the superseded-fence removal keeps the first fence live through `Y`'s message view, and `v`'s `P := 1` takes the edge from it.

Gates: QSL `make check-spec-validation` exit 0, with no diagnostic on a touched file. `tools/check-index-completeness.sh` passes. No `docs/` file and no test-parsed vector table is touched. QSpec: `quire validate --scope . spec/**/*.md` reports 134 failing documents, as on main, and none is touched. `cargo test --test checked_package_v2 --test backend_provider_v1` passes (14 + 117). The new ids (FR-223-AC-9, AC-10, LT-17, FR-437-AC-12, AC-13, MB-12) appear in no other open PR. #336 (STD-241) is stacked on #333 and carries them unchanged.

Units examined (role, id, path):

- examined: ADR-025 PSC-4(e) (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 PSC-6 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 MS-5 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- context_only: ADR-025 RA-5 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- context_only: ADR-025 PSC-3 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 LT-17 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: FR-223 fence bullet (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223-AC-9 (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223-AC-10 (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-437 Fence lifetime (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: FR-437-AC-12 (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: FR-437-AC-13 (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: TC-391 LT-17 (spec/test-cases/TC-391-litmus-conformance-vectors.md)
- examined: TC-393 MB-12 (spec/test-cases/TC-393-memory-budgets-and-verdicts.md)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-223-AC-10, FR-437-AC-13 and MB-12 say a thread that fences twice with no store in between has no first fence in `P` after the second. Under RA-5 as written ('does both, then sets `cur_b` and `S` ... and is an SC event'), the second fence's release part sets every `rel_b(ℓ)` to `cur_b` while `cur_b`'s frontier entry for `b` is still the first fence. The code does the same (ra.rs:832 runs before `sc_event`). So every release view has the first fence as the latest fence it covers, and PSC-4(e) keeps that fence live until the next release. No implementation of the stated rules can pass the first half of these ACs. Fix RA-5 and PSC-2 so that a `seq_cst` fence's release part copies `cur_b` after the fence enters its frontier (see FND-002), or rewrite the ACs. | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:94 |
| FND-002 | medium | The brief asked for a counterexample: can a later acquire need a fence the rule drops? (e) drops none, but RA-5's order means the fence a later acquire needs is never recorded. Take LT-14 with the flag store made `relaxed`: `u: X := 1 sc; fence sc; Y := 1 relaxed`, `v: a := Y acquire; P := 1 sc`, `w: P := 2 sc; b := X sc`, outcome a = 1, b = 0, final P = 2. RC11 forbids it (script: forbidden under `sc`, `tso`, RC11). `Y`'s message view is `rel_u(Y)`, copied before the fence entered `u`'s frontier, so `v`'s view does not cover the fence. The PSC-3 edge `fence → P := 1` is not built and `ra` admits the outcome. This shape is not one of PSC-7's omitted shapes. The same gap makes `u: fence; X := 1 sc; fence; Y := 1 relaxed` lose the second fence's edge while (e) keeps only the first. The root is in the stack base (RA-5, PSC-3 from #1192, STD-239). Fix it there, or here with (e), by stating that a `seq_cst` fence is in its thread's frontier before its release part runs, and add the relaxed-flag vector. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:219 |
| FND-003 | low | (e) says when a fence is live and that it 'leaves `P` by the restriction above', but not when the check runs. RA-7 runs message removal 'after every step'. (e)'s removal depends on views changing at loads, stores, fences and receives, and the canonical key (PSC-5) needs one answer. State that fence removal runs with RA-7 after every step, in both halves. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:591 |

## Verdict

Not mergeable as is: one high, one medium and one low finding. The lifetime rule itself is sound with respect to the frontier, and the alternative (keep every fence until its thread finishes) is correctly rejected as unbounded for loops. PSC-6 and MS-5 agree with (e). The QSL and QSpec halves agree with each other. PR bodies list lead defaults and contain no review-finding text. FND-001 leaves FR-223-AC-10 and FR-437-AC-13 unbuildable until RA-5's order is fixed.
