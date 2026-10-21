---
id: SR-12255
title: "Code review (Rust lane) of QSL-566 E16b ProtocolSystem<Ra>, release-acquire and RC11 SC order (PR #1141)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@233c0191d0a3df6a21589ebadeb17c7aa0a6698e; git diff origin/main...HEAD: qsl-eval/src/simulation/protocol/{ra.rs,memory.rs,steps.rs,footprint.rs,identity.rs,channel.rs,compensation.rs,selection.rs,tso.rs}, qsl-eval/src/simulation/protocol.rs, qsl-analyze/src/model_check/check.rs, qsl-replay/src/{certificate/closure.rs,model_check/replay.rs,model_check/settle.rs}, qsl-analyze/tests/it/{release_acquire.rs,weak_memory.rs,protocol_system.rs,main.rs}; ticket QSL-566"
review_set: subset
date: '2026-10-21'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: reviews
---

## Summary

Ticket: QSL-566. PR: quire-spec-language#1141, head 233c0191d. Method: code-review with the rust-review lane folded in. Checked against ADR-025 RA-1 to RA-8, PSC-1 to PSC-6, MB-2, MB-3 and MR-1, and against FR-222, FR-223, FR-227-AC-2 and FR-229. The coder's three disclosed spec-gap choices (STD-239, STD-240, STD-241) were read too.

## Verdict

**Changes requested.** There are two high (blocking) soundness findings. Both are in how SC events leave the graph `P`. In both, `Ra` admits an execution that RC11's partial SC order forbids, so a `refuted` verdict can carry a counterexample that is not real. MB-4 says a counterexample found is real.

- FND-001: when RA-7 removes an SC event, its live predecessors are kept only in `F(ℓ)`, and only SC writes to `ℓ` read `F(ℓ)`. A later event of the same thread on another location loses them.
- FND-002: the engine supersedes an older SC fence when the same thread runs a newer one. That drops the older fence's happens-before edges to fences of threads that synchronised with it before the newer fence ran. PSC-4 has no such rule, and none of STD-239, STD-240 or STD-241 discloses it.

Both were traced by hand through `sc_event`, `collect` and `drop_events` on a concrete litmus program (below). They were not run: the brief keeps this review read-only on code. Each fix should land with its litmus program as a regression test in `release_acquire.rs`, and with that program added as a fixture of `every_independent_pair_of_ra_steps_commutes` (FND-005).

Clean, examined: the RA-2 load window, the RA-3 slot rule with attached successors, RMW insertion (RA-4), and the release and relaxed message views. Also clean: the order of the fence rules (RA-5, `S` joined after the release part) and the fork, join and channel view carry (RA-6). The channel's carried view is re-encoded against the current component at key time. Other clean units:

- Depth naming. It survives front removal, so GC never renames a choice.
- `kept_from` and `collect` (STD-240). A pointer below every running thread's `cur` joins as nothing, so the reduced pin set is sound.
- The canonical key (STD-241). The id order among one thread's live events can be recovered from the `po` edges in `P`, so the frontier's "covered count" is not ambiguous.
- The transitive-closure update and the cycle test in `sc_event`.
- Supersession of SC reads. A read has no in-edges after it is created, and its replacement reads the same message, so nothing is lost.
- The `Sc` and `Tso` paths. They are mechanically adapted to `Vec<Chosen>` and behave the same.
- Replay and closure integration through `SubjectMemory::Ra`.

There are no `unwrap`/`expect`/`panic!` in production code, and integer conversions saturate or use `try_from`.

### FND-001 litmus (forbidden by RC11, admitted by `Ra`)

Locations `X` (here `ℓ4`) and `L`, all at 0. Thread `u`: `y: X := 1 sc`; `r: L := 1 sc`; `a := L relaxed`; `b := L relaxed`. Thread `t`: `e: L := 2 sc`; `L := 3 relaxed`; `x: X := 2 sc`. The outcome is `a = 2`, `b = 3`, final `X = 1`. `a = 2` forces `r mo e`, and final `X = 1` forces `x mo y`, which gives the psc cycle `y →po r →mo e →po x →mo y`. The schedule is `y`, `r`, `e`, `L := 3`, `a`, `b`, `x`. After `b`, every running `cur(L)` is at `L3`, so `L0`, `r`'s and `e`'s messages are removed. `r` and `e` leave `P`, and `y` goes into `F(L)` only. `x` writes `X`, so its `before` is empty, while its `after` is `{y}`. No cycle is found, and slot depth 1 (before `y`) is enabled. Without the GC between `b` and `x`, `e` is live, and `y → e → x` closes the cycle. That also shows that the non-SC load `b` and the SC store `x` are footprint-independent but do not commute (FND-005).

### FND-002 litmus (forbidden by RC11, admitted by `Ra`)

Thread `b`: `a: X := 1 sc`; `f1: fence sc`; `flag := 1 release`; `f2: fence sc`. Thread `c`: `flag acquire` (reads 1); `x: fence sc`; `z: Y := 1 sc`. Thread `d`: `w: Y := 2 sc`; `v := X sc` (reads 0). The outcome is `c` reads `flag = 1`, `v = 0`, final `Y = 2`. That gives the psc cycle `a →po f1 →hb x →po z →mo w →po v →rb a`, which is forbidden. The schedule is `a`, `f1`, `flag := 1`, `f2`, `w`, `c`'s acquire, `x`, `z` (slot before `w`), `v`. `f2` supersedes `f1`. `c`'s frontier names `f1`'s id, which is below `f2`'s, so `x` gets no edge from `b`, and `v`'s cycle test does not reach `a`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | PSC-4 collection keeps a removed SC event's live predecessors only in `F(ℓ)`, which only SC writes to `ℓ` read. A later SC event of the same thread on another location loses its `po`-transitive predecessors, so `Ra` admits a psc cycle RC11 forbids (litmus in Verdict). Keep the predecessors of a removed event for every later event of its thread, and file the PSC-4 rule gap | qsl-eval/src/simulation/protocol/ra.rs:699-730, ra.rs:773-799, ra.rs:594-628 |
| FND-002 | high | `sc_event` drops every earlier SC fence of the same thread when a new one runs, with no summary. The earlier fence's hb edges to fences of threads whose frontier names it but not the newer fence are lost, so `Ra` admits a psc cycle RC11 forbids (litmus in Verdict). The rule is in neither PSC-4 nor STD-239/240/241 | qsl-eval/src/simulation/protocol/ra.rs:663-691, ra.rs:596-605 |
| FND-003 | medium | `max_messages` is checked against the live count before insertion and before GC, and before the step is known to be enabled. That contradicts MB-2 ("live messages per location after garbage collection"). A store that GC brings back under the bound is cut, and so is an RMW disabled by an attached successor, or a store or RMW whose every SC choice is pruned. Each such cut marks the state bound-limited, and the run settles V-6 spuriously | qsl-eval/src/simulation/protocol/ra.rs:1070-1073, ra.rs:1133-1146 |
| FND-004 | medium | FR-222-AC-1 says the claim settles `refuted` with §10's counterexample, `Sx`, `Ly` at depth 0, `Sy`, `Lx` at depth 1. The test asserts that the found counterexample reads `Ly` at depth 1, and checks §10's order only as some path of the system. The AC is not met as written, and no spec ticket records the divergence | qsl-analyze/tests/it/release_acquire.rs:211-284 |
| FND-005 | medium | `every_independent_pair_of_ra_steps_commutes` has no fixture in which a non-SC step's GC removes SC events. MR-1's claim that "garbage collection adds no location" is untested exactly where GC writes the SC graph, and under FND-001 such a pair (`b` and `x` in the litmus) is independent by footprint but does not commute. Add the FND-001 and FND-002 programs as fixtures | qsl-analyze/tests/it/release_acquire.rs:900-952, qsl-eval/src/simulation/protocol/ra.rs:1434-1435 |
| FND-006 | low | `sorted` maps a canonical-encoding error to empty bytes (`unwrap_or_default`), so a failed encode silently changes the key order instead of surfacing | qsl-eval/src/simulation/protocol/ra.rs:198-211 |
| FND-007 | low | FR-227-AC-2's "changing a carried message view" is tested by replacing step 2's whole component with step 0's. Changing only one message view in the carried JSON would pin the AC as written | qsl-analyze/tests/it/release_acquire.rs:1030-1044 |

## New findings (disposition pass 1)

Disposition pass 1 at head 053b1655e (fix commit 053b1655e). One focused run: `cargo test -p qsl-analyze --test it -- release_acquire`, 17 passed.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The two regression tests are tagged with ACs they do not test: `a_removed_sc_event_still_precedes_its_threads_later_events` is tagged `FR-223-AC-3`, the graph-size bound, and `a_fence_stays_while_a_view_covers_it_and_not_its_successor` is tagged `FR-223-AC-2`, IRIW and fenced SB. The second test's name also describes a retention rule that does not exist; its own doc says it passes without one. Rename it, for example "a superseded fence's edges follow through `S` and `po`", and tag both tests `TC-668` alone or with the STD-246 / STD-247 vectors | qsl-analyze/tests/it/release_acquire.rs:1053-1075 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 053b1655e: `RaComponent.owed`. A collected SC event's live predecessors join its thread's `owed` set (`drop_events`), and a collected member of `owed` is replaced by its own predecessors. Every new SC event of the thread takes `owed` into `before` (`sc_event`). `owed` is in the key and is dropped for joined branches. The FND-001 litmus is pinned by `a_removed_sc_event_still_precedes_its_threads_later_events` and is a commute fixture. STD-246 names the residual cross-thread `hb\|loc` case, where only a view carries the happens-before. That case is narrower and only admits extra executions, so a `proved` verdict stays sound. It is the same class as the clauses STD-239 already discloses. Not blocking |
| FND-002 | rejected | My trace was wrong. `fenced` joins `S` into `cur` before `sc_event` (ra.rs:818-823), and `S` carries the frontier of each thread's newest earlier fence. So `c`'s fence `x` takes an hb edge from `b`'s `f2`, `a →po f2` holds, and the cycle closes without `f1`. The fixture `superseded_fence_fixture` pins the forbidden outcome. STD-247 records why supersession is sound only while fences take fence-to-fence edges alone |
| FND-003 | fixed | 053b1655e: the bound is checked after `collect` on each choice (`Ra::exceeds`, `live > max_messages`). A choice pruned by an attached successor or an SC cycle returns before the check, so it is never cut. `a_store_that_collection_brings_under_the_bound_is_not_cut` gives `bound_limited == 0` and `proved` where the old check cut |
| FND-004 | deferred | STD-248: the spec should name the canonical counterexample (`Sx`, `Sy`, `Lx` at depth 1, `Ly` at depth 1), or state that §10's table is one violating path. The test doc now names both |
| FND-005 | fixed | 053b1655e: while any SC event is live, every view-moving step (attempt, receive, fence, join) reads and writes the SC graph. Both review litmus programs are fixtures of `every_independent_pair_of_ra_steps_commutes` |
| FND-006 | fixed | 053b1655e: `sorted` returns `Enc::Unencodable` on an encode error, so the key refuses |
| FND-007 | fixed | 053b1655e: the forgery flips one message view's depth (`elements[1].view.at`) in step 2's carried JSON, and replay refuses `stale_dependency`/`content-mismatch` |
