---
id: SR-1671001
title: "Code review of quire-spec-language PR #1671: symmetry on zones (QSL-1005a)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1dba086ab; PR #1671 own diff 21aa9813d..1dba086ab: qsl-analyze/src/zone_check{.rs,/dbm.rs,/liveness.rs,/search.rs,/settle.rs}; qsl-eval/src/simulation/model/timed.rs; qsl-analyze/tests/it/{main,timed_world,zone_certificate,zone_dense,zone_search}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-151
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: reviews
---
# Code review of quire-spec-language PR #1671

## Summary

Ticket: QSL-1005, slice 1005a. The Rust review lane is folded into this
review. I reviewed only `git diff 21aa9813d..origin/qsl-1005a-zone-symmetry`
(11 files).

**Clocks move with the permutation (FR-430 "Reductions over a dense-time
subject").** `TimedSystem::place_images` maps the place of clock `c` on the
object under key `k` to the place of `c` on the object under `π(k)`, in the
same population. `Dbm::renamed` moves rows and columns `i → image[i-1]+1`
and keeps the reference clock. `canonical_symbolic_state` applies FR-152's
model-state permutation and the same permutation to the zone.

**Coverage on canonical states.** `add_node` canonicalises before it
computes the key and before the coverage test, and it stores the
permutation on the node. Ties between objects with equal discrete state are
broken in key order without looking at the zone, which FR-430 explicitly
allows ("they may stay distinct, which is sound and only reduces less").

**One instance per orbit (FR-151).** A clause with an `over` binding gets
one `Plan` per orbit from `instance_groups`, bound to the orbit's least key
under its stabiliser. The deadlock-freedom item runs under the full group.

**Lifting.** σ starts as π0⁻¹. Each canonical step `t` becomes the concrete
step whose transition equals `permute_transition(t, σ)`, and σ becomes
σ∘π⁻¹. `Permutation::compose(a, b)` applies `b` first, so the concrete
successor is σ(π⁻¹(canonical target)), which is correct. The deadlock zone
and the divergence targets are renamed by σ, and the instance is mapped by
σ. Guard-part indices carry over, because a part is indexed within the same
clause.

**Settlement.** A reduced `Holds` becomes `HoldsReduced`, which settles
`Proved{Reduced{symmetry}, Uncertified}`, `closed-scope`.
`SymmetryBroken` settles as FR-160's V-6 inconclusive. Partial-order
reduction or a state constraint settles V-8 `UnsupportedReduction` right
after the obligation check, before bounds, admission or any state.
`Reductions::selected` lists partial-order before constraint, so
partial-order is named first.

**Differential probe (local test, never committed).** I compared each
reduced verdict with the unreduced one, using the PR's own `Timer` fixture
with non-uniform guard bounds and with a time invariant:

| World | Deadlock item | Bound claim | Stored states | Lifted counterexamples |
|---|---|---|---|---|
| `arm x>=1`, `disarm x>=2` | Holds = Holds | Violated = Violated | 14 → 8 | replay |
| Latching (no `disarm`) | Violated = Violated | Violated = Violated | 4 → 3 | all replay |
| Three timers, group `[[t1,t2]]` only (`t3` fixed) | Holds = Holds | Violated = Violated | 95 → 51 | replay |
| Three timers, latching | Violated = Violated | Violated = Violated | 11 → 7 | all replay |
| `time invariant when inflight { x <= 3 ms }` | Holds = Holds | Violated = Violated | 17 → 10 | replay |
| Invariant, latching | Holds = Holds | Holds = Holds | | |

The reduced search never returned Holds where the unreduced one returned
Violated. QSL cannot write a guard that is asymmetric between objects:
guards are relative to the receiver, there is no key literal (STD-493,
ad5c9496 item 1), and asymmetric initial data is caught by `closure` as
`SymmetryBroken`. So the remaining risk to soundness is the clock renaming
itself (FND-001).

Gates: `cargo test -p qsl-analyze -p qsl-eval --all-features` ran at
1dba086ab through `locked-build.sh` (jobs 4, `QSPEC_DIR` at QSpec bd73a062)
and passed (exit 0). qsl-analyze ran 969 tests.

## Verdict

**CONDITIONAL.** FND-001 is medium: it guards soundness against the open
580j stack. The other findings are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Clock renaming assumes one place per (clock, object) and never checks that its image is a permutation. `place_images` matches only `clock`, `population` and `key`. `Dbm::renamed` writes `renamed[to(i)][to(j)]` for every `i, j`, so a duplicate image silently overwrites rows and leaves others stale. The open 580j stack (#1655, approved) adds `ClockPlace.slots`, so a clock in a collection has one place per slot on the same object. Whichever of #1655 and #1671 lands second, the code still compiles, but `place_images` maps every slot place of `k` to the first slot place of `π(k)`. The canonical zone would then be wrong, and a reduced `Holds` could be unsound with no fault raised. Fix: (a) `Dbm::renamed` rejects an `image` that is not a bijection onto `1..dim` (a `DbmError`), so any mismatch is a fault rather than a verdict; (b) the PR that lands second makes `place_images` match `slots` too. Record (b) on QSL-580/QSL-1005 so the merge does not drop it. | qsl-eval/src/simulation/model/timed.rs:463-481; qsl-analyze/src/zone_check/dbm.rs:242-266 |
| FND-002 | low | Ordering of the symmetry refusal. For the time-lock item and a clause decided on the liveness product, symmetry can never apply on EN-6. Yet `admit` and `closure` run first, and the V-8 check runs only inside the plan loop after `judge_plan`. So an inadmissible declaration refuses `Symmetry`, and initial states that are not closed settle `SymmetryBroken` (V-6), where the item should settle V-8 like partial-order reduction does. Move this check into the pre-check beside partial-order: the item, and whether a clause takes the liveness product, are known before admission. | qsl-analyze/src/zone_check/search.rs:445-510 |
| FND-003 | low | When symmetry is selected and the clause's `over` universe is empty, `instance_groups` yields no plans. The outcome is then `HoldsReduced{reductions: []}`, settling `Proved{Reduced{[]}, Uncertified}` though no reduction was applied, while the unreduced path gives a certified `Holds`. Either run the unreduced plan or settle certified when no orbit exists. | qsl-analyze/src/zone_check/search.rs:470-480, 565-578 |
| FND-004 | low | Rust and docs. The doc of `ZoneReduction::Unsupported` says the first reduction is named "in the order symmetry, partial-order, constraint", but the pre-check skips symmetry and the plan-loop path names only symmetry. State the two paths separately. The `Some(_) => Fault("reduced-certificate")` arm cannot be reached by design (`run.reduced` forces `HoldsReduced`); a comment saying so would help. `lifted_to` re-expands the concrete steps at each stage (`steps_for`), which is fine for counterexample lengths. | qsl-analyze/src/zone_check/search.rs:128-150, 571, 1826-1900 |
