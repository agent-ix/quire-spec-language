---
id: SR-14070
title: "integrity review of STD-246 PSC-4 collection, owed sets and LT-15/LT-16 (PR #1193, QSpec PR #332)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@851e3a6a155b7b7c17ee088688e83d49139b68ad; spec/decisions/ADR-025-weak-memory-models-for-parallel.md, spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md, spec/test-cases/TC-668-rc11-partial-sc-order.md; agent-ix/quire-specification@eba4eabe67c00ba55c36230429451fb06719183a; spec/functional/memory/FR-437-explore-release-acquire-memory.md, spec/test-cases/TC-391-litmus-conformance-vectors.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-668
    type: reviews
---
# SR-14070: integrity review of STD-246 PSC-4 collection, owed sets and LT-15/LT-16

## Summary

Ticket: STD-246. The PRs are stacked. QSL #1193 is reviewed as `git diff origin/std-ra-239-qsl...851e3a6a1` (3 files, +22/-8). QSpec #332 is reviewed as `git diff origin/std-ra-239-qspec...eba4eab` (2 files, +22/-6). The STD-239 base (#1192, #331) is out of scope and is cited only where a new rule depends on it.

I checked the new PSC-4 parts (a) to (d) against RC11's `psc` and against the engine (qsl-eval/src/simulation/protocol/ra.rs: `sc_event`, `drop_events`, `kept_from`, `publish`, `fenced`). Part (b) `owed(b)` matches the code's `owed` map. Part (c), the view-carried pairs, is new and has no code yet. I traced LT-15 and LT-16 by hand through the engine's rules. I recomputed both vectors with a script that enumerates SC interleavings, ADR-025 TSO-1..8 store-buffer states, and RC11 axiomatic executions (coherence, no-thin-air, `psc` acyclicity). It also computes the `psc` subset that PSC-3 builds (litmus.py next to this file). The script reproduces LT-12 and LT-13 as `ra`-allowed and RC11-forbidden. It gives LT-15 and LT-16 as forbidden under `sc`, `tso`, RC11 and the PSC-3 edge set, which agrees with both tables.

STD-239 item 3 (PSC-4(a)) holds. PSC-3 gives a new event an incoming edge only into a live SC write, so no edge into a removed read can arise later. Any later cycle through the read enters it through a predecessor it had when it was removed, and `F(ℓ)` now places that predecessor before the later write.

Gates: `make check-spec-validation` exit 0, with no diagnostic on a touched file. `tools/check-index-completeness.sh` passes. No `docs/` file and no test-parsed vector table is touched (no test reads ADR-025 or TC-668 rows). QSpec: `quire validate --scope . spec/**/*.md` reports 134 failing documents, as on main, and none is touched. `cargo test --test checked_package_v2 --test backend_provider_v1` passes (14 + 117). The new ids (FR-223-AC-7, AC-8, LT-15, LT-16, FR-437-AC-10, AC-11) appear in no other open PR except PRs stacked on this one.

Units examined (role, id, path):

- examined: ADR-025 PSC-1 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 PSC-4(a) (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 PSC-4(b) (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 PSC-4(c) (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 PSC-4(d) (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 PSC-5 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- context_only: ADR-025 RA-7 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 LT-15 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: ADR-025 LT-16 (spec/decisions/ADR-025-weak-memory-models-for-parallel.md)
- examined: FR-223 Description (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223 collection bullet (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223 key bullet (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223-AC-7 (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223-AC-8 (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: TC-668 step 7 (spec/test-cases/TC-668-rc11-partial-sc-order.md)
- examined: FR-437 State (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- context_only: FR-437 Garbage collection (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: FR-437 Collection (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: FR-437-AC-10 (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: FR-437-AC-11 (spec/functional/memory/FR-437-explore-release-acquire-memory.md)
- examined: TC-391 LT-15 (spec/test-cases/TC-391-litmus-conformance-vectors.md)
- examined: TC-391 LT-16 (spec/test-cases/TC-391-litmus-conformance-vectors.md)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-223-AC-7, FR-437-AC-10, LT-15's rationale and TC-668 step 7 all say that once `b` reads `L := 3`, the SC events on `L := 1` and `L := 2` are collected. The RA-7 and FR-437 garbage-collection rule on this stack still pins a message from every thread's three views, every message view and `S`. `rel_u(L)` (set by the `seq_cst` store `L := 1`) points at `L := 1`, and `rel_t(L)` (set by `L := 2`) points at `L := 2`, so neither message is removed and `owed(t)` is never exercised. The outcome is still forbidden, by `L := 2`'s `po` edge to `X := 2`, but the stated collection never happens. The claim holds only under the current-view pin set of STD-240 (#1195 / #334), which matches the code's `kept_from` (ra.rs:769). Neither PR names that PR or a merge order. Either name the dependency and merge #1195/#334 first, or carry the RA-7 / FR-437 pin-set change here. | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:88 |
| FND-002 | medium | PSC-4(c) adds the pair `(ℓ, p)` to 'every view (RA-1) ... whose frontier covers `e`'. That does not name a view a `send` carries that no `receive` has yet joined. STD-247's PSC-4(e) lists such views separately, so a reader takes (c) to exclude them. Counterexample: `u: X := 1 sc; L := 1 sc; a := L relaxed; b := L relaxed`; `t: L := 2 sc; send; L := 3 relaxed`; `w: L := 4 relaxed; receive; d := L sc; X := 2 sc`; outcome a = 2, b = 3, d = 4, final X = 1. With the channel modelled as a release/acquire flag (RA-6), the script gives forbidden under `sc`, `tso` and RC11. Under the current-view pin set (#1195), `L := 2` is collected once `b` has read and `w` has stored `L := 4`, while the carried view is still in flight. `w` then receives a view that covers `L := 2` but holds no pair, so `d` does not take `X := 1` and `X := 2` can take the slot before `X := 1`. RC11 has `L := 2` hb `d` through the channel (`hb\|loc`). Name in-flight carried views in (c) and in FR-437's pair bullet. A simpler alternative is a table of removed events keyed by thread and position, applied at join time to any view whose frontier covers the event, which also reaches views held outside the component (the engine's `CarriedView`). | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:591 |
| FND-003 | low | PSC-4(d) and FR-437's last Collection sub-bullet replace a removed member of `owed(b)` or a pair by its live predecessors, but say nothing about a member of `F(ℓ)` that is itself removed. `F(ℓ)` holds live events by PSC-1, and members can be removed (an access on another location, or a fence under STD-247's (e)). The code already replaces `F(ℓ)` members (ra.rs:737, `drop_events`, which iterates `fronts` and `owed`). Add `F(ℓ)` to (d) in both halves. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:591 |
| FND-004 | low | Several statements still list the SC graph members without the new `owed` sets. FR-223's Description says '`P`, `F` and the frontiers SHALL be part of the state key'. FR-223's first two Behavior bullets ('keep `P`, `F(ℓ)` and the frontiers as members', 'update `P`, `F(ℓ)` and the frontiers', lines 53 and 55) do the same. So does QSpec FR-437's State bullet (line 95). The new Outputs and key bullets, PSC-1 and PSC-5 do include them. Add the `owed` sets, the thread sets and the per-view pairs, to each statement. | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:27 |
| FND-005 | low | LT-15's ADR rationale reads 'the removed `L := 1` and `L := 2` leave their predecessors in `owed(t)`'. `L := 1` is `u`'s event, and by PSC-4(b) its predecessors go to `owed(u)`. `t`'s `X := 2` is held back by `L := 2`'s predecessors (`X := 1`, through `L := 1 → L := 2` in mo). Reword to 'the removed `L := 2` leaves `X := 1` in `owed(t)`'. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:605 |

## Verdict

Not mergeable as is: two medium and three low findings. PSC-4(a), (b) and (d) are sound for owed and pairs. Part (b) matches the engine's `owed` map. LT-15 and LT-16 recompute as stated in every column. The QSL and QSpec halves agree with each other. PR bodies list lead defaults and contain no review-finding text. The lead default (pairs rather than a new PSC-7 omission) leaves every AC buildable once FND-002 names in-flight views.

## New findings (disposition pass 1)

Reviewed at QSL f12e88a3497a28fb244da5be4ba41fbb3676c8ad (#1193, own diff against origin/std-ra-239-qsl 985acb35d; fix commit f12e88a34) and QSpec 0943c2cc67912ebd3039134e8fb85ddad23241c7 (#332, own diff against origin/std-ra-239-qspec 7e7a8e0; fix commit 0943c2c).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | FR-437-AC-10 says garbage collection removes "the SC events on the earlier messages of `L` when the thread that wrote them stores `X := 2`" and that "their predecessors are kept for that thread". In LT-15 the earlier messages have two writers: `u` wrote `L := 1` and `t` wrote `L := 2`, and only `t` stores `X := 2`. By the collection rule `L := 1`'s predecessors go to `owed(u)`; `t` is held back by `L := 2`'s live predecessors (`X := 1`), which is how this round reworded LT-15's ADR rationale (FND-005). Reword to name `L := 2`: "whether or not garbage collection has removed the SC event on `L := 2`, the earlier message the storing thread wrote: when it has, its live predecessors, among them `X := 1`, are kept for that thread". FR-223-AC-7's "their predecessors are kept in `owed(t)`" reads the same way and can take the same wording. | spec/functional/memory/FR-437-explore-release-acquire-memory.md:226 |

## Dispositions

Round 1, reviewed at QSL f12e88a3497a28fb244da5be4ba41fbb3676c8ad (#1193, own diff against origin/std-ra-239-qsl 985acb35d; fix commit f12e88a34) and QSpec 0943c2cc67912ebd3039134e8fb85ddad23241c7 (#332, own diff against origin/std-ra-239-qspec 7e7a8e0; fix commit 0943c2c).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | FR-223-AC-7, FR-437-AC-10 and the LT-15 rationale no longer claim that the events are collected: each holds whether or not garbage collection has removed them, and TC-668 step 7 inspects only states in which they were collected. The #1193 and #332 bodies name STD-240 (#1195, #334) and the merge order for the vectors to exercise `owed`. After: \| FR-223-AC-7 \| ADR-025 §15's LT-15 does not reach a = 2, b = 3 and final `X = 1` under `ra`, whether or not garbage collection has removed the SC events on `L := 1` and `L := 2` when `t` stores `X := 2`: when it has, their predecessors are kept in `owed(t)`, `u`'s `X := 1` still precedes `t`'s `X := 2` in the SC event graph, and the slot for `X := 2` before `X := 1` in modification order is not offered. \| |
| FND-002 | fixed | PSC-4(c) and the FR-437 pair bullet name a view a `send` carries that no `receive` has joined, and add pairs to every view wherever it is held, in flight on a channel included. After: (c) *Later events of other threads.* Every view (RA-1), including a view that a `send` carries and no `receive` has yet joined, also holds `owed(V)`, pairs `(ℓ, p)` with `p` a live SC event: `p` precedes every later SC event on `ℓ` of a thread whose current view includes `V`. A removed event `e` on `ℓ` adds `(ℓ, p)`, for each live predecessor `p` of `e`, to every view whose frontier covers `e`, wherever that view is held (by a thread, in a message, as `S`, or in flight on a channel) |
| FND-003 | fixed | PSC-4(d) and FR-437's last Collection sub-bullet both cover a removed member of `F(ℓ)`. After: (d) A member of `F(ℓ)` or of `owed(b)`, or the `p` of a pair, that is itself removed is replaced by its live predecessors. - a member of `F(ℓ)` or of `owed(b)`, or the `p` of a pair, that is itself removed is replaced by its live predecessors. |
| FND-004 | fixed | FR-223 Description and its first two Behavior bullets, FR-437's State bullet and Key bullet all list the `owed` sets; FR-437 names both the per-thread sets and the per-view pairs. After: `P`, `F`, the frontiers and the `owed` sets SHALL be part of the state key. come; and the `owed` sets of the collection rule below, one per thread and the pairs of every view. |
| FND-005 | fixed | LT-15's rationale now names `L := 2` and `owed(t)`. After: forbidden: `u`'s `X := 1` precedes `t`'s `X := 2` through `L := 1` and `L := 2`, and when `L := 2` is collected it leaves `X := 1` in `owed(t)` \| |

## Dispositions (pass 2)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-006 | fixed | QSpec FR-437-AC-10 and QSL FR-223-AC-7 now name `L := 2`, the earlier message the storing thread wrote: when its SC event has been collected, its live predecessors, among them `X := 1`, are kept for that thread (in `owed(t)` in QSL). |
