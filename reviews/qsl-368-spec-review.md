---
id: SR-1140
title: "Spec review of PR #568 (state-space reduction: ADR-021, FR-150 to FR-162, POR soundness)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@3146aa23b51ec82659afd2cade328a52e33e2ede; git diff 404a5a88...HEAD: spec/decisions/ADR-021-state-space-reduction.md, spec/functional/FR-150 to FR-162, spec/test-cases/TC-565 to TC-589 and TC-617, spec/usecase/US-019, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-368. Review of `git diff 404a5a88...HEAD` at 3146aa23. The
branch is stacked on a stale #562 head (404a5a88), so that is the diff
base. Checked against the wave-B and wave-C owner rulings and against the
QSpec counterpart FR-381 to FR-385 on QSpec branch
`spec/wave-b-q2-refine-reduce` at cdb221c.

Lenses applied: the POR soundness proofs, read line by line (POR-1, POR-4,
POR-7, POR-10, POR-11, the §7.5 corollary, the four §7.6 vectors);
consistency with QSpec; EARS phrasing; AC-to-TC coverage; caps and limits;
version tracking; compat paths; undefined-is-refuted.

Simulator re-runs, in the scratchpad `porsim/`. `fixed_q2.py`,
`fixed_q2_r2.py`, `recheck_r1.py` and `recheck_r2.py` all reproduce the
stated verdicts, with reduced equal to unreduced. The §7.6 state counts
match: 6, 4 and 4, plus 2 for the wrong resolution. `gate.py` reproduces
§7.5. Random search found no mismatch:

- 36,000 POR-plus-fairness systems (`search.py`, seeds 101 to 106);
- 19,000 combined runs of symmetry, POR, POR-10 and the BFS proviso, with
  weak and strong `whole` classes (`sr368/sympor.py`).

`sr368/newcx.py` gives three new false-`proved` vectors, FND-001 to FND-003
below. Each one comes from footprint derivation, not from the ample-set or
fairness logic.

What is right:

- POR-7's choice of `R(t)` as the model enabling footprint is right for
  every location a clause body reads.
- POR-4's membership meets are sound under FR-120's closed population
  states.
- The conclusion of POR-10/POR-11 holds for weak and strong fairness of both
  granularities.
- The §7.5 corollary holds: for a one-identity class, the closure already
  keeps that class's enabling-footprint writers together, so POR-10 matters
  there only for never-taken ample transitions.
- Symmetry combined with POR is sound: canonical-state C3 implies a fully
  expanded state on every lifted cycle.
- RV-7 and FR-160 settle an undefined claim evaluation `refuted`,
  `UndefinedEvaluation`.
- There are no caps, pins, ledgers or compat paths.
- `quire validate` on the 41 changed artifact files exits 0, and
  `tools/check-index-completeness.sh` passes.
- Every AC of FR-150 to FR-162 has a TC that tests behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `R(t)`, and so the enabling footprint, covers only clause-body reads and receiver or argument membership. FR-120's post-state construction reads more than that. It admits a candidate only when every reference names an object of the candidate, so a `deletes` of `k` is enabled only while no reference field names `k`. A `creates` that must create, or a reference-typed `modifies`, also depends on the target population's membership. Vector V1 (`sr368/newcx.py`): Jobs `j` and `k`, with `j.peer = k`; `lock(k)` clears `k.cancellable`; `unlink(j)` clears `j.peer`; `cancel(k)` deletes exactly its receiver; the claim is `always holds(k.x = 0)`, which is undefined once `k` is deleted. Unreduced: refuted by `unlink, cancel`. Reduced: `A = {lock(k)}`, because `unlink(j)` is not a writer of `cancel(k)`'s enabling footprint, and the run reports `proved` over 3 states. With the referrer field in the footprint, both runs refute. QSpec FR-383 has the same rule, so the fix lands on both sides. | spec/decisions/ADR-021-state-space-reduction.md:186, :192; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:92-96 |
| FND-002 | high | POR-1 and FR-155 "Locations read" map a navigation read (`self.parent.f`) to `AnyField{f}` alone. They never say the intermediate reference field (`Receiver`, `parent`) is read too, and no AC requires it: FR-155-AC-2's `copy()` lists only `AnyField{v}`. Vector V2: `disarm()` clears `g.armed`; `point()` sets `g.cell` to `b`; `boom()` has pre `self.armed and self.cell.v = 1` and writes `g.alarm`; `a.v = 0` and `b.v = 1`. Unreduced: refuted by `point, boom`. Reduced, with `R(boom) = {armed, AnyField{v}}`: `A = {disarm}`, and the run reports `proved` over 3 states. With `(g, cell)` in `R(boom)`, both runs refute. QSpec FR-383 uses the same wording. | spec/decisions/ADR-021-state-space-reduction.md:186; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:83-87, :137 |
| FND-003 | high | POR-7 delegates a protocol step's enabling footprint to ADR-027 FT-3, and states that FT-3 covers every model location the step's enabledness reads. FT-3 on `origin/spec/396-protocol-ts` reads only "the model locations of its precondition". It leaves out four things: the postcondition's pre-state reads and the receiver and argument membership (ADR-018 FA-2); the binders an `attempt`'s arguments read; and `queue(h)` for `duplicate` and `lose`, although FT-1 puts it in their `R`. An `attempt` built per FT-3 reproduces §7.6's first vector, the guard in a postcondition, as a false `proved`. The amendment line ("FT-1 and FT-3 are POR-1's protocol locations and POR-7's enabling footprints") does not say FT-3's set must become the application's `R(t)` plus `ctl`, binders and queue. | spec/decisions/ADR-021-state-space-reduction.md:192, :834-836 |
| FND-004 | medium | Nothing says how the deleted object "resolves" to the receiver or an argument, and nothing enforces it. FR-154 keeps `deletes T` type-wide. FR-120 lets a `deletes T` step delete any subset of conforming objects, and `check_frame` admits all of them. So `Membership{p, Receiver}` as `W(t)` is not the "checked fact" POR-2 promises, since a step can also delete other objects. No `deletes deref(self.target)` entry form exists, yet §7.6's fourth vector and FR-155-AC-5 use one. Either add a receiver- or argument-scoped `deletes` form that `check_frame` enforces, as POR-3 does for `modifies`, or always write `AnyMembership`. | spec/decisions/ADR-021-state-space-reduction.md:186, :722-726; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:97-104, :140 |
| FND-005 | medium | The POR-11 completeness induction does not hold as written. It picks `β`, "the first such extra", from those inserted after `p`, then claims "every earlier extra is outside `K`". Extras inserted before `p` can belong to `K`, and POR-10 exempts a member of `K` from `K`-visibility, so such an extra can write `β`'s enabling footprint. The conclusion still holds: when `σ` takes `K` finitely often, induct on the first `K`-extra anywhere in `σ'`. It stays enabled at every later `σ`-state, and the contradiction then uses `p`. So no extra belongs to `K` at all. The later step "the extra transitions write none of it" needs that stronger statement. | spec/decisions/ADR-021-state-space-reduction.md:196 |
| FND-006 | medium | QSL FR-155 and QSpec FR-383 disagree on footprint semantics that QSpec owns (FR-155 says so at :152). QSL says `Membership{p, o}` meets `AnyField{f}`, QSpec says it does not. QSL maps a quantified variable and a collection element to `AnyField`, and a quantification over a population to `AnyMembership`; QSpec has neither rule. FR-155-AC-2 tests the population-quantification rule. QSL's rules are the conservative ones. QSpec's missing rule for quantified-variable reads is a soundness gap on the QSpec side: `forall o in counters: o.v < 2` would get no `AnyField{v}`. The two specs must state one rule. Amend QSpec FR-383 to QSL's rules and have FR-155 cite it rather than restate it. | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:83-87, :108-116, :152 |
| FND-007 | low | POR-7 says the enabling footprint is "all of `R(t)`" but lists only three parts: the precondition's reads, the postcondition's pre-state reads, and membership. It drops POR-1's fourth part, the postcondition's post-state reads outside `W(t)`. FR-155's Enabling footprint bullet does the same. Those reads equal pre-state values and bear on enabledness, so an implementer working from the list would drop them. | spec/decisions/ADR-021-state-space-reduction.md:192; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:92-96 |
| FND-008 | low | FR-158 makes a state where the constraint evaluates `Undefined` or `Refused` a boundary state, but ADR-021 SC-2 does not state this. `ConstraintReached{boundary_states}` cannot tell a constraint that is false from one that is undefined, so an undefined constraint reports as a deliberate cut. State it in SC-2, or name the undefined boundary states in the cause. | spec/functional/FR-158-cut-the-search-with-a-state-constraint.md:67-74; spec/decisions/ADR-021-state-space-reduction.md:203 |

## Verdict

Not mergeable as it stands. FND-001 to FND-003 are false `proved` results
under POR. Each comes from an incomplete footprint rule, and each is
reproduced by simulation (FND-003 by reduction to §7.6's first vector).
FND-001 and FND-002 also need the same fix in QSpec FR-383, which was
reviewed as mergeable. The QSpec side should not merge alone. FND-004 to
FND-006 are a missing enforcement, a proof step that fails as written but
whose conclusion is right, and a QSL/QSpec divergence.

The fairness machinery is sound: POR-10, the conclusion of POR-11, the §7.5
corollary, symmetry combined with POR, and the annotated quotient.

## New findings (disposition pass 1)

Round 1 reviewed `git diff 3146aa23...d6eeaaa4` at d6eeaaa4, against QSpec
FR-383 and FR-013 on QSpec #169 at 39268af.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | high | The read rules POR-1 summarises, and QSpec FR-383 "Read footprint" now owns, give no rule for an operation's `result`. FR-120 ranges a result over the result type's domain in the post-state, so a postcondition such as `result.v = 1` with `result: Reference<Cell>` makes enabledness depend on `AnyField{Cell, v}` and `AnyMembership{cells}`. Neither is derived: `result` is not `self`, a parameter, a navigation, a quantified variable, a binder or a collection element. Vector V4 (`porsim/sr368/round1.py`): `pick()` frames `modifies self.alarm`, has pre `self.armed` and post `self.alarm and result.v = 1`; `disarm()` clears `armed`; `set()` sets `a.v` to 1. Unreduced: refuted by `set, pick` over 6 states. Reduced with the result read underived: `A = {disarm}`, and the run reports `proved` over 3 states. With the result read as `AnyField{Cell, v}` plus `AnyMembership{cells}`, both runs refute. The fix lands in QSpec FR-383, with a matching clause in POR-1's summary and an FR-155 AC. | spec/decisions/ADR-021-state-space-reduction.md:186; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:77-82 |
| FND-010 | medium | SC-2 and FR-158 now settle an undefined constraint `refuted`, `UndefinedEvaluation`, with "a counterexample ending there that replays". Three things in the record still contradict that. SC-3 says "a constraint never changes what a verdict means". RV-8 and FR-158-AC-4 keep the constraint out of the obligation identity, so two requests with one identity can settle `refuted` and `proved` for a claim that holds. EI-7 replays through ADR-018 CX-3, which re-evaluates the claim, not the constraint, so the prefix does not reproduce a violation. Either amend SC-3, EI-7 and RV-8 to say the constraint is part of what is refuted and replayed (QSpec FR-383 has the same rule), or settle an undefined constraint as SC-2 settles a refused one (V-7), naming `UndefinedEvaluation{where, cause}`. | spec/decisions/ADR-021-state-space-reduction.md:203-204, :352; spec/functional/FR-158-cut-the-search-with-a-state-constraint.md:67-78 |

## New findings (disposition pass 2)

Round 2 reviewed `git diff origin/spec/366-temporal-properties...e45da098` at
e45da098 (stacked on #562 at ce4f463c), against QSpec FR-383, FR-385 and
FR-013 on QSpec #169 at 725f3ce. The eight-step POR-11 proof and POR-8 were
checked step by step and hold under the stated footprints; the findings
below are a footprint rule gap and two citation or coverage gaps in the
proof text. Simulator: `porsim/sr368/round2.py` (V5); 9,000 more
symmetry-plus-POR and 12,000 POR-plus-fairness random systems found no
mismatch.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | high | The read rules give no rule for a call. QSpec FR-146 admits pure functions whose bodies `deref` a reference argument, and FR-146 adds FR-151 dispatch edges from a clause to every candidate's effective precondition, so a precondition such as `self.armed and hot(self.cell)` with `function hot(c: Reference<Cell>): Bool = deref(c).v = 1` reads `AnyField{Cell, v}`. FR-383 "A clause expression reads locations by these rules", which POR-1 and FR-155 cite, lists field reads, navigation, `reaches`, quantified variables, populations and `result`, and nothing for a call or a dispatched call. Vector V5 (`porsim/sr368/round2.py`): `disarm()` clears `g.armed`; `set()` sets `a.v` to 1; `boom()` has the precondition above and writes `g.alarm`. Unreduced: refuted by `set, boom` over 6 states. Reduced with the call body's reads underived: `A = {disarm}`, `proved` over 3 states. With `AnyField{Cell, v}` derived from the body, both refute. If the FR-155 read enforcement covers function bodies, the same model settles `failed` instead, so every model that calls a state-reading function in a clause cannot be checked under POR. Fix in QSpec FR-383: a call reads the locations of its arguments and the reads of the callee's body, transitively, with each reference parameter read as `AnyField` of its type (or the argument's location when it is `self` or an operation parameter); a dispatched call reads every candidate's effective-precondition reads. Add an FR-155 AC and a §7.6 vector. | spec/decisions/ADR-021-state-space-reduction.md:187; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:81-89 |
| FND-012 | low | POR-11 step (4) says what to do when `A(v)` has a step of `η` or an extra, but not when `E(v)` is empty. A fair accepting behaviour can end in a stutter-extended terminal state (ADR-018 SM-4), and then the construction must take no step. The argument holds: an extra taken at `v` is independent of all of the finite `η`, so it would stay enabled at `σ`'s terminal state, so no extra exists and `σ'` ends at the same terminal state. The proof should state this case. | spec/decisions/ADR-021-state-space-reduction.md:197 |
| FND-013 | low | POR-11 step 5(d) and §2 cite Peled and Wilke 1997 for "the infinite-trace forms PT-2 admits under POR are invariant under stuttering". Peled and Wilke prove the converse: every stutter-invariant LTL property is expressible without next. The direction step 5(d) needs (next-free LTL is stutter-invariant) is the older, easy one, and Peled and Wilke cover future operators only. The grammar also has `since` and `triggered`, whose stutter invariance is QS-8 and is tested by QSpec TC-328 (FR-161-AC-12). Cite FR-161 / TC-328 for the past operators and state the direction used. | spec/decisions/ADR-021-state-space-reduction.md:197, :265-270, :971-974 |

## New findings (disposition pass 3)

Reviewed at 31d5eb9ae92691b81b6b42a0d7563268b6f7274a, limited to the plan lead's four items and to lines the PR changed.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | low | TC-886 is the reduced-versus-unreduced differential test, and it carries the trace tag `#[trace("TC-886", "FR-160-AC-6")]`. `make ci` runs `cargo test --locked --workspace`, so it gates TC-886 once implemented, but only if the test is not `#[ignore]`. The repo already splits one corpus-wide test out of `make ci` this way (`make test-differential` runs the parser differential with `--include-ignored`). TC-886 does not say it runs in the gate. State that TC-886 runs under `make ci` and is not ignored, or name the part of the corpus that `make ci` runs if a full run is too slow. | spec/test-cases/TC-886-reduced-and-unreduced-runs-agree-over-the-corpus.md:36 |

## New findings (disposition pass 4)

Reviewed at 450d70ede85affad5413ea9f9486e5f0f44384bc (rebased onto main 4d7ba58e), limited to the delta 31d5eb9a..450d70ed.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-015 | low | Amended FT-3's enabling footprint holds the pre-state locations a step's postconditions read and the membership of its receiver and reference arguments. Its closing list, "A disabled step can be enabled only by a step that writes one of these", names a writer of the precondition's model locations but no writer of those postcondition pre-state locations or of that membership. POR-7's closure uses the footprint meet, so a closure built per POR-7 is still sound. A closure built from FT-3's list would miss those enablers, which is the §7.6 false-`proved` pattern. Add "a writer of a location its postconditions read in the pre-state, a creator or deleter of its receiver or a reference argument" to the list, or replace the list with "a step whose write footprint meets this enabling footprint". | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:305 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2ae3a808 |
| FND-002 | fixed | 2ae3a808 |
| FND-003 | fixed | 2ae3a808 |
| FND-004 | fixed | 2ae3a808 |
| FND-005 | fixed | 2ae3a808 |
| FND-006 | fixed | 2ae3a808 |
| FND-007 | fixed | 2ae3a808 |
| FND-008 | fixed | 2ae3a808 |
| FND-009 | fixed | b7960b89 |
| FND-010 | fixed | b7960b89 |

Round 2, reviewed at 38add39a2862c34c28f7a0ae8aa38363fe6ba56a.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-009 | fixed | 2b7d4c7e: POR-1 cites QSpec FR-383's read footprint, which reads `AnyField` and `AnyMembership` through `result`. |
| FND-010 | fixed | 2b7d4c7e: SC-3 carves out the undefined-constraint refutation; EI-7 re-evaluates the constraint on replay. |
| FND-011 | fixed | 2b7d4c7e: Resolved by citation: POR-1 cites QSpec FR-383, whose call rule QSpec #169 adds at da0f8347; merge #169 first. |
| FND-012 | still-open | POR-11 step (4) still says nothing about a reduced state with `E(v)` empty, where the construction must take no step and `σ'` ends at the same terminal state. |
| FND-013 | still-open | POR-11 5(d) still cites Peled and Wilke for "the infinite-trace forms PT-2 admits under POR are invariant under stuttering"; that needs the easy converse direction, and `since` and `triggered` need QS-8. |

Round 3, reviewed at 31d5eb9ae92691b81b6b42a0d7563268b6f7274a.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-012 | fixed | 31d5eb9a: POR-11 step (4) covers a terminal `v` with empty `E(v)`. |
| FND-013 | fixed | 31d5eb9a: 5(d) and §2 cite Lamport 1983 for next-free implies stutter-invariant, QSpec FR-161/TC-328 for `since` and `triggered`, and Peled and Wilke as the converse. |

Round 4, reviewed at 450d70ede85affad5413ea9f9486e5f0f44384bc.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-014 | fixed | 450d70ed: TC-886 runs under `make ci` over the whole corpus and is not ignored. |
