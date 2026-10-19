---
id: SR-12189
title: "Code review of quire-spec-language PR #1029: QSL-528 EN-1 decides TP-2 and TP-3 on the product with a lazy bad-prefix monitor"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@64ba23327be808d3dc22067405cd79e417cebf8a; PR #1029 diff against origin/main (12 files, +1579/-338): qsl-eval/src/automaton/{gba,letter,mod,monitor,translate}.rs; qsl-eval/tests/it/{automaton_kernels,property_automata}.rs; qsl-analyze/src/model_check/check.rs; qsl-analyze/tests/it/{main,model_check,model_check_safety,model_world}.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1029

## Summary

Ticket: QSL-528. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` at the reviewed sha. Read-only: nothing
was built or run, at the dispatcher's instruction. Context read: ADR-018
UE-1 to UE-5, SM-6, PC-3; FR-125 "Undefined evaluation"; FR-126 "Property
automaton" and "First phase"; `qsl-replay` `check_closure`,
`ModelCheckItem::instances` and `settle`.

Checked and clean:

- `Monitor<A: SafetyAutomaton>` viability. `viable()` is a depth-first
  search over satisfiable transitions. It marks the whole path viable when
  the top state has an edge to a state on the path (a cycle) or to a state
  already known viable. It marks a state dead only when every target is
  known dead, because a target that is on the path, known viable or still
  undecided either closes the search or is explored first. So "known dead"
  is exact by induction, and "viable" is exact because every path member
  reaches the cycle. With no acceptance set, every infinite run accepts, so
  this is the eager trim's greatest fixpoint, computed per state on demand.
  A `StateLimit` part-way through leaves only cached rows, no wrong
  verdicts.
- First-bad-prefix rejection: `step` keeps only admitted targets that are
  viable, so the empty subset is reached at the first position with no
  continuation. The new `eventually[0,2] false` row in
  `a_safety_monitor_rejects_exactly_the_bad_prefixes` checks the lazy and
  explicit monitors against each other and against the evaluator on every
  prefix up to length 4. I found no path that misses a bad prefix or
  reports a false one *in the monitor itself*.
- Laziness: `bad_prefix` reads nothing. States are read as `initial`,
  `step`, `reads` and the viability search reach them. The search can read
  ahead of the product, but only until the first cycle or known-viable
  state, as the module doc says.
- The deleted eager `trim` and `Gba::rows`: no caller is left in the
  workspace. The `mod.rs` and `translate.rs` docs are updated. FR-126 and
  ADR-018 SM-6 still say "trimmed ... then determinized". That is the same
  automaton, so the spec text is not stale.
- `Guard::literals` and `Monitor::reads` are correct *for the monitor*: a
  guard's admission depends only on its required and forbidden bits. How
  `check.rs` uses `reads` is FND-001.
- One path for TP-1, TP-2, TP-3 and deadlock-freedom. The product key is
  (FR-101 key, monitor state per instance). The letter is read when the
  product state is created (UE-4, FR-126 "First phase"). A bad state is not
  inserted into `known`, so it never coalesces. After a bad state is
  created, no further state is created, but the earlier frontier states are
  still expanded, so an earlier contract-undefined or TP-2 closure
  violation wins in BFS order.
- Terminal handling. TP-2 runs `close()`: all-false letters, `live` false
  too, until the monitor rejects or repeats. It is deterministic, so this
  terminates and is exact. TP-1 and TP-3 push one stutter step per terminal
  product state. A stutter that changes the monitor state creates a new
  terminal node, which stutters again until the monitor repeats, so
  "stuttered forever" is explored to its fixpoint. The counterexample drops
  the `None` stutter transition from the prefix.
- Refusals: `GuardedActivation` (an `on each ... when`), `Translation`
  (no property automaton), and TP-4 or no form gives `FormNotDecided`, all
  before any state is explored. Removing `TooManyBindings` is correct now
  that each instance has its own monitor state rather than one letter
  atom. `ItemRefusal::TooManyBindings` remains on the TP-1 certificate path
  in `qsl-replay`.
- The empty closure certificate for TP-2 and TP-3 (focus 3). `certified`
  is true only for `ReachableInvariant` under infinite-trace. Otherwise
  `instances` is not resolved and the certificate is empty. In
  `check_closure`, `request.item.instances()` calls
  `SafetyTranslation::reachable_invariant`. That refuses any clause that is
  not `always holds(A)` under infinite-trace, which gives
  `ClosureRefusal::Item`, and `settle` maps that to `failed()`. Even if
  `instances()` admitted the clause, an empty certificate would reject at
  the first initial product state (`InitialMissing`). So no path turns an
  empty certificate into `Accepted` or `Proved`. Until QSL-878, settling
  `Failed` (never `Proved`) is the honest state. A test that pins it is
  SR-12190 FND-003.
- Interplay with #1017: TP-1 still resolves `ClauseIdentity` and each
  instance's carried key, as before. TP-2 and TP-3 skip clause-root
  resolution, so a TP-2/TP-3 clause with no root explores instead of
  refusing `Item`. That has no effect while their certificates are empty,
  and it goes away with QSL-878. The expected #1019 conflict in `check.rs`
  is not a finding.
- `#[trace]` bindings are judged in SR-12190.

## Verdict

FIX. One blocking defect (FND-001). `check.rs` evaluates only the atoms
the monitor's next step constrains (`Monitor::reads`). ADR-018 UE-1 and
FR-125 require the value of *every* atom of the claim at every position
the claim reads. A TP-2 or TP-3 clause can therefore return `Holds` where
the claim is undefined on an admitted behaviour, which is a missed
refutation. A `Formula` counterexample can also fail UE-5 replay parity.
FND-002 to FND-004 are recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `Run::read` evaluates an atom only when `Monitor::reads(from)` has its bit, that is, only the atoms the guards out of the current monitor subset constrain. The module doc states this as the rule: "a position the claim no longer reads evaluates nothing (FR-125)". The spec says otherwise. ADR-018 UE-1 and FR-125 "Undefined evaluation" say that at each position a claim reads, it reads "the value of every atom of the claim there". A TP-2 `on origin` claim reads positions 0 to `h`, and every other form reads every position. Only closed-boundary positions past a terminal state evaluate no atom. Once the monitor's next step stops depending on an atom (an always-true sink after `release` is discharged, or after an `implies` antecedent is false), that atom is not evaluated, so an undefined value there is missed. Example over `Counter`, infinite-trace: `holds(c.value >= 0) release holds(6 mod (2 - c.value) >= 0)` is discharged at position 0. `reads` is then 0, position 2 is never evaluated, and the run returns `Holds`. By UE-1 it is `Violated`, `UndefinedEvaluation` at position 2. The same happens inside a TP-2 horizon: `holds(p) implies eventually[0,3] holds(q)` with `p` false at 0 never reads `q` at positions 1 to 3. A second effect: a `Formula` counterexample whose prefix has an unevaluated undefined atom before `where` disagrees with UE-5 replay, which evaluates whole letters, so it settles `inconclusive` `ReplayParity`. TP-1 and deadlock-freedom are unaffected, because their one guard always reads its atom. Fix: evaluate every clause atom at every product state the claim reads, bounding TP-2 `on origin` by `h` rather than by `reads`, and delete the `reads` filter or keep it only for closure positions. Add the release row above as a test. | qsl-analyze/src/model_check/check.rs:20-23; qsl-analyze/src/model_check/check.rs:668-674; qsl-eval/src/automaton/monitor.rs:213-232 |
| FND-002 | low | The horizon check sets `horizon_cut` whenever `steps` is non-empty, and `steps` now includes the terminal stutter step. A TP-1 or TP-3 run whose deepest state is terminal at depth `k = max_depth` returns `BoundReached`, even when that stutter would coalesce with the terminal product state itself (same key, same monitor state), so no product state is left unexplored. For TP-1 this regresses from `Holds` before this PR. The answer is weaker, not wrong. Fix: count a stutter step toward the cut only when it would create a new product state. | qsl-analyze/src/model_check/check.rs:548-556 |
| FND-003 | low | When an atom is undefined, `create` pushes the undefined instance's `from` state and breaks, so `node.monitor` is shorter than the instance count. This is safe only because a bad node is never expanded (`first_bad` returns before `successors`). It is a fragile, unstated invariant: `close()` and the stutter path would index a short vector if that order ever changed. Fix: say so on `Node::monitor`, or fill the remaining instances with `from`. | qsl-analyze/src/model_check/check.rs:603-618 |
| FND-004 | low | Every `Monitor::row` call clones the cached `Vec<Transition>`, and `step`, `reads` and each `viable` iteration all call it. `viable` also rescans the whole row for `closes` on every loop turn. Each product-state creation runs `reads` and then `step` over the same member rows, so each row is cloned at least twice per instance per state. Fix: return `&[Transition]` from the cache (for example, read and cache first, then borrow), and test `closes` once per pushed frame. | qsl-eval/src/automaton/monitor.rs:200-232; qsl-eval/src/automaton/monitor.rs:271-327 |

## Dispositions

Round 1, reviewed at d883b9c85b6ebab8735218f76c6fca416bca2adc: fix commit
4e9c2ff03, merge e5c575817 of origin/main (it does not touch the PR's
files), and the clippy type alias d883b9c85.

- `Run::read` now evaluates every clause atom when the claim reads the
  position. `Item::Clause::last_read` is `clause.horizon()` for a bounded
  `on origin` clause and `None` otherwise, which is UE-1's read set.
  `Monitor::reads` and `Guard::literals` are deleted, and no caller is left.
  Past `h`, a TP-2 `on origin` monitor steps on an all-false letter. That
  is sound: a bounded formula's truth is fixed by positions 0 to `h`, so the
  deterministic monitor has already rejected or can no longer reject.
- Both new UE-1 tests fail without the fix. In
  `a_discharged_infinite_trace_claim_still_reads_every_atom`, `reads` was 0
  after the release is discharged, so the old code returned `Holds` and
  `violated()` panics. In
  `a_decided_bounded_claim_still_reads_every_atom_up_to_its_horizon`, the
  antecedent is false at 0, so the old code read nothing at positions 1 to 3.
  With the fix, both expect `UndefinedEvaluation` at position 2 with the
  prefix `0, 1, 2`.
- UE-5 parity: every product state on a counterexample's path is now
  created with its whole UE-1 letter read, so no position before `where`
  can hold an unevaluated undefined atom. Parity holds by construction. It
  cannot be exercised end to end yet: `replay_model_trace` refuses an
  authored clause (`content-mismatch`, FR-337, no temporal clause node in a
  recompiled package), so only deadlock-freedom replays today.
- The `viable` change (test for a closing edge only at `cursor == 0`) is
  sound. On a later visit the path prefix is the same, and no target can
  have become viable, because a closing child returns at once after marking
  the whole path.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4e9c2ff03 |
| FND-002 | fixed | 4e9c2ff03 |
| FND-003 | fixed | 4e9c2ff03 |
| FND-004 | fixed | 4e9c2ff03 |
