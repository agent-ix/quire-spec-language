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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | `place_images` sends the place at slots `s` to the image object's place at the same `s`. That holds for a sequence, whose positions FR-152's renaming keeps. It does not hold for a set or bag whose elements hold a reference to the permuted population: `ModelSystem::permute` renames those references and re-sorts the collection by canonical key, so an element can change slot. Example: `t1.ts = t2.ts = {R(peer t1), R(peer t2)}` under the swap. Old `t2`'s `R(peer t1)` becomes new `t1`'s `R(peer t2)` at slot 1, while its clock goes to slot 0. The mapping is still a bijection, so `NotAPermutation` does not catch it, and the canonical zone pairs clocks with the wrong elements. This cannot be reached at this head: EN-6 refuses a granted `Sequence`/`Set` of clock records with `RequiresBound`, and an ungranted collection's slot clocks are never reset. It becomes reachable when 580o (#1688) makes these collections finite. Fix: refuse symmetry (V-8) when a clock-bearing set or bag element holds a reference into a symmetric population, or map slots by element after `permute`. Record it on QSL-1005 or QSL-580 so the #1688 merge carries it. | qsl-eval/src/simulation/model/timed.rs:730-749; qsl-eval/src/simulation/model/symmetry.rs:203-260 |

## Dispositions

Round 1, fix commit 8ad9b7408, a merge of `qsl-583e-mightyl-general`. `cargo test -p qsl-analyze --all-features` passed at 8ad9b7408: 974 tests in `it` and 90 lib tests, with no failures apart from my local probe. It ran through locked-build with jobs 4 and QSPEC_DIR at QSpec origin/main 60383a35.

I re-ran the differential probe with slot-bearing objects under symmetry: two symmetric `Call`s, each with `ts: Sequence<Timer>[0, 2]`, quantified guards and resets by selection. EN-6 refuses that model with `RequiresBound` at this head, because the selection reset needs `ts` granted. Without the grant, every slot clock equals elapsed time, so a differential run shows nothing. Slot mapping under symmetry can therefore be checked only at the unit level at this head (`a_permutation_maps_each_slot_clock_to_the_same_slot`, 14 places, a bijection, same clock and slots, different object). The probe should be re-run once 580o lands.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `Dbm::renamed` checks that the image is in range and injective over `0..dim` before writing any row, so it is a bijection, and otherwise returns `NotAPermutation { first, second }` (`a_renaming_must_be_a_permutation`: a swap round-trips; `[0, 0]` and an out-of-range image are refused). `place_images` matches `clock`, `slots`, `population` and the image key. `a_permutation_maps_each_slot_clock_to_the_same_slot` checks 14 places over `Sequence<Timer>[0, 2]` on two objects. This is correct for sequences. Sets and bags whose elements reference the permuted population remain open as new FND-005. |
| FND-002 | fixed | `reads_unreduced_graph` runs in the pre-check, after the partial-order/constraint V-8 and before `admit` and `closure`. `symmetry_admission_is_judged_before_the_search` shows the time-lock item over a non-closed initial state settling V-8 rather than `SymmetryBroken`. Note: the pre-check sizes `flat_window` by the full instance count, whereas a reduced plan binds one representative. A clause whose window fits one instance but not all of them now settles V-8 where it could have run reduced. That is conservative and never unsound. |
| FND-003 | fixed | When the `over` universe is empty, `symmetric` is cleared and the run is one unreduced plan with the trivial group, so a proof is `Holds` with a certificate (`symmetry_over_no_instance_runs_unreduced`). |
| FND-004 | fixed | The `ZoneReduction::Unsupported` doc now states both paths: partial-order then constraint over any item, then symmetry over a liveness-product or time-lock item. The `Fault("reduced-certificate")` arm carries a comment that it is unreachable by design. |
