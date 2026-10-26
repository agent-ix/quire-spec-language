---
id: SR-1639001
title: "code-review and rust-review of PR #1639 (QSL-583a, FR-240 time-lock freedom on zones)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1639 git diff origin/qsl-582-cert-checker...390f7abb0: qsl-analyze/src/zone_check/{time_lock.rs,search.rs,dbm.rs}, qsl-analyze/tests/it/{zone_search.rs,zone_replay.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: reviews
---
# SR-1639001: code-review and rust-review of PR #1639 (QSL-583a, FR-240 time-lock freedom on zones)

## Summary

Ticket: QSL-583. I reviewed 9412e295a, then the final head 390f7abb0, which adds one test-only commit (`a_covered_successor_hides_no_deadlock`). The review was led by soundness.

**The departure from EZ-7 is right.** EZ-7 marks whole symbolic states that reach no divergent target. Reachability over a zone is existential, so a zone where some valuations escape and others are trapped is never marked. My `Hold` probes confirm this:

| `go` guard | `ping` guard | Result | Replay |
| --- | --- | --- | --- |
| `<= 1` | `> 1` | Violated, final delay 3/2 | reproduces |
| `< 1` | `> 1` | Violated, final delay 1 | reproduces |
| `<= 1` | `>= 1` | Violated, final delay 3/2 | reproduces |
| `<= 2` | `> 1` | Holds | (none) |

A per-valuation definition is needed. Using aM for this item is also right. With `L = U = M`, the LU relation is "equal, or both above `M`", which is symmetric. So it is a bisimulation, and it preserves the absence of a divergent continuation, which one-directional aLU does not.

**The fixpoint is sound and terminates.** It computes `νY. μX. Q ∪ Pre(X) ∪ Pre_z(Y)` (time_lock.rs:155-277), which is the Büchi characterisation of "reaches a quiescent state, or resets `z` after a unit infinitely often" (Tripakis). I checked each operator:

- **`Q`:** a quiescent state's valuations over the whole space. Such a state has no invariant and is not urgent.
- **`Pre_z`:** `Y ∧ z = 0`, then `free(z)`, then `z >= 1`, then the time predecessors.
- **`Pre` over a step:** target invariants, then each reset inverted (`x = v`, `free`) in reverse order, then the guard part, then the time predecessors.
- **Time predecessors:** `down` inside the source invariants, applied only when time elapses there. The invariants are convex, so this is exact.
- **Iteration:** the outer loop stops when `Y_{k+1}` covers `Y_k` (a decreasing sequence), and each `μ` restarts from `Q ∪ Pre_z(Y_k)`.
- **Domain:** only model states in the graph enter. A step to a state outside the graph is taken by no reachable valuation.
- **Termination:** the operators are applied without extrapolation, over the model's constants and `z`'s single unit. Every set they produce is a union of M-regions, and there are finitely many of those (the backward fixpoint of Henzinger, Nicollin, Sifakis and Yovine).

**A reported time-lock replays.** Local locks are the node zone minus the instant-enabled zones and minus the strict-invariant delaying zone. Trapped pieces are the node zone minus the divergent set, and each is concretised and then confirmed by `point_diverges` from the point zone. Stored zones are never extrapolated, so the last valuation is a real, trapped one. FR-237's replay, local arm or fresh exploration, then reproduces it unless a limit stops it. My probes reproduced every case:

- the `Hold` variants above;
- a Zeno reset loop under `x <= 0`;
- a two-clock diagonal trap (a local lock at `y = 3`, `x = 1/2` after `ping` at 5/2);
- a two-phase trap reached by a step;
- a tick-source trap (`go`, then a `ping` loop with `tick` disabled; final delay 0).

The matching Holds cases all returned `Holds`: the diagonal variant without the `x` guard, a resetting `ping` loop, and a tick source with `tick` always enabled. The probes ran in the review worktree and were reverted.

**Can a Holds still be wrong?** Not from the fixpoint. It is exact, and aM coverage is a bisimulation. The only path is internal: a trapped piece that fails to concretise, or one that `point_diverges` calls divergent, is skipped silently, and the run can end `Holds` (FND-001).

**The author's aLU argument for the deadlock item (STD-493 comment 10cc4a44) holds.** A quiescent node's zone is closed under delay. Guards and target invariants bound one clock each (FR-230 has diagonal-free guards), so after `down` each enabling zone is a box of upper bounds. Such boxes cover a delay-closed non-empty zone only when one of them is the universe, and the universe enables every valuation of that model state. So aLU coverage can make the item `Undecided(DeadlockAfterDelay)`, but never a wrong `Holds`. That is an incompleteness, already a known gap.

**Gates at 390f7abb0:**
- `zone_` tests: 55/55 passed at 9412e295a, plus the added probe test.
- clippy `-p qsl-analyze --all-targets --all-features -D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `first_time_lock` silently skips a trapped piece in two cases: when `lock_path` returns `None` (it maps every `concretise` error to `None` through `.ok()`), and when `point_diverges` says the chosen point diverges. The run then goes on to `Holds`. With an exact fixpoint and unextrapolated stored zones, either case is an internal disagreement (a concretiser or fixpoint bug), not a property of the model. The local-lock branch already treats the same failure as `Fault("time-lock-concretisation")`. Fix: fault on both cases for trapped pieces, so an internal error can never surface as a proof of time-lock freedom. | qsl-analyze/src/zone_check/time_lock.rs:111-119,333-353 |
| FND-002 | medium | The ν/μ fixpoint is charged to no `ZoneCheckLimits` budget. It is bounded only by `poll()` (cancellation). Its federations can be much larger than the search graph, because they are unions of zones over all valuations with the extra `z` clock. A model that the search finishes under `max_symbolic_states` and `max_zone_bytes` can therefore run without bound here and never settle V-7. Also, each ν round recomputes the inner μ from scratch. Fix: charge federation zones to `max_zone_bytes` (or a named fixpoint limit) and stop with `ZoneStop::LimitReached`. Optionally, warm-start each μ round. | qsl-analyze/src/zone_check/time_lock.rs:208-262 |
| FND-003 | low | `divergent_valuations` returns an anonymous `(BTreeMap<Vec<u8>, usize>, Vec<Vec<Dbm>>)` behind `#[allow(clippy::type_complexity)]`. A small named struct (key index plus per-state divergent zones) would remove the allow and document the pairing. | qsl-analyze/src/zone_check/time_lock.rs:154-155 |

## Verdict

**Mergeable. There are no major bugs.** The departure from EZ-7 is correct and well founded. The fixpoint is sound and terminates. Every reported time-lock I produced replays. A wrong `Holds` is possible only on an internal failure, which FND-001 would make loud. FND-001 and FND-002 are medium and cheap to fix in this PR. FND-003 is a nit.

Rust lane: no `unwrap`, `expect` or panicking index in the new production code. `Federation::add` keeps the union free of subsumed zones. `without_last_clock` projects a closed matrix correctly and keeps emptiness. The trap loop calls `poll()` for cancellation.

## Coverage

Examined:
- FR-240 "Time-lock search";
- FR-236-AC-2;
- FR-237 non-local arm (context);
- ADR-026 EZ-5 and EZ-7 (context);
- STD-493 comments f4765723 and 10cc4a44.

Gap analysis is SR-1639002.

## Dispositions

Round 1. I checked each fix against the code in a fresh detached worktree and target. Head af99b38b4 is restacked on #1637's 982b0235a, and the fix commit is af99b38b4. `git range-diff` shows the two original commits unchanged except for the base's new aM coverage of the deadlock item.

At af99b38b4:
- `zone_` tests: 63/63 passed.
- clippy `-p qsl-analyze --all-targets --all-features -D warnings`: exit 0.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `first_time_lock` takes the node's first trapped piece. If it does not concretise, the run faults with `Fault("time-lock-concretisation")`. If `point_diverges` calls its point divergent, it faults with `Fault("time-lock-confirmation")`. Neither case can end in `Holds` any more. |
| FND-002 | fixed | Every zone a federation gains goes through `hold`, which calls `charge_bytes` against `max_zone_bytes` and stops with V-7. `the_trap_search_is_charged_to_the_zone_budget` shows that a budget one byte short stops the run. The case against a warm start is right. The ν sets decrease, so each round's μ set is a subset of the previous one: an upper bound, not a start from below. A least fixpoint must be iterated from below. |
| FND-003 | fixed | `Divergence { keys, held }` and `LockPath { initial, concrete }` replace the tuples, and the `type_complexity` allow is gone. |
