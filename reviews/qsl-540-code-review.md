---
id: SR-12205
title: "Code review of quire-spec-language PR #1054: E12 symmetry reduction and state constraints on safety"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0c9ee6b172f48117ae13d7fdfaf30af911433463; PR #1054 (git diff origin/main...HEAD): qsl-analyze/src/model_check/reduce.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/src/model_check/mod.rs, qsl-eval/src/simulation/reduction.rs, qsl-eval/src/simulation/explore.rs, qsl-eval/src/simulation/model.rs, qsl-eval/src/simulation/model/symmetry.rs, qsl-eval/src/simulation/model/constraint.rs, qsl-eval/src/simulation/model/encode.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/model_check/outcome.rs, qsl-replay/src/model_check/settle.rs, qsl-replay/src/outcome.rs, qsl-semantics/src/model/state.rs, qsl-analyze/tests/it/reduction.rs, qsl-analyze/tests/it/reduction_world.rs and the 13 request constructions"
review_set: subset
---
# Code review of quire-spec-language PR #1054

## Summary

Ticket: QSL-540. Code review with the rust-review lane folded in. Soundness
came first: can a reduced run settle `Holds` or `Proved` where the unreduced
run refutes? No cargo build was run; this is a read of the diff at the head.

What is right:

- `preserves` has no `_` arm. Its rows match FR-159 "Preservation table":
  symmetry and the constraint read yes for every form. Partial-order reads yes
  for TP-1, for TP-3 and TP-4 without an interval, and for deadlock freedom,
  and no for the rest.
- The pre-check order is FR-159's: table, then symmetry form, then closure
  (`SymmetryBroken`), then V-8 (`unoffered`), then constraint resolution. All
  of it runs before any expansion.
- One run per orbit, bound to the least key, under the stabiliser
  (`instance_groups`, `SymmetryGroup::stabiliser`).
- The canonicaliser always returns `π(state)` for a `π` that is a bijection on
  each class. Equal canonical keys therefore imply one orbit, so coalescing is
  sound whether or not the form is fully canonical. Ties are kept in key
  order by a stable sort. References to fixed keys, the bound key included,
  are encoded concretely.
- Boundary states are stored and counted toward `max_states`. They are never
  expanded, stutter-extended or classified as deadlocks. An undefined
  constraint refutes at that state and does not make it a boundary state.
  `ConstraintReached` takes precedence over `BoundReached`.
- A constraint-only `Holds` with no boundary state explored exactly the
  unreduced graph. FR-160's map row gives `Proved{Exhaustive, Uncertified}`,
  so the proof is not vacuous.
- Concretisation follows FR-161 "Start" and "Steps". Each concrete step is
  checked against a real expansion of the concrete source, and a mismatch is a
  fault, never a wrong step. A nested reference argument that
  `permute_transition` leaves alone therefore faults, as intended.
- Every reduced proof settles `Certification::Uncertified`. No reduced path
  reaches the closure-certificate branch.
- The default hooks offer nothing, and `explore`, `sample` and `explore_model`
  never call `canonical`, `holds_constraint` or `offered_reductions`.

The declared departures were not reviewed as defects: no constraint keyword,
`Permutation` in qsl-eval, `ConstraintValue::Undefined` carrying
`UndefinedCause`, no POR offer, `ReducedLiveness` refused, and the 9-item
remainder.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Unsound canonicalisation with overlapping classes across declarations. `admit` resets `seen` for each declaration, so two declarations for one population with overlapping classes (`counters: [[a, b]]`, then `counters: [[a, c]]`) are both admitted. FR-151 "Form" refuses a key in two classes. `canonicalise` then calls `Permutation::insert` once per class on the same population map, and `extend` overwrites `a`'s image. Example at (a=1, b=0, c=0): `a→c, b→a, c→a`. That is not a bijection. `permute` collects two objects into key `a` and drops one, so the stored "canonical" state is outside the orbit. The reduced search can then explore unreachable states and coalesce states that are not symmetric, and a wrong `Proved{Reduced}` is possible. Fix: track `seen` per population across declarations (refuse `RepeatedKey` naming the shared key), or refuse a second declaration for the same population. Add a test for the overlapping pair. A `debug_assert!` in `Permutation::insert` that the moved keys are disjoint from the ones already there would catch any regression. | qsl-analyze/src/model_check/reduce.rs:235-276; qsl-eval/src/simulation/model/symmetry.rs:339-361; qsl-eval/src/simulation/reduction.rs:127-140 |
| FND-002 | medium | A constrained `UndefinedEvaluation` refutation cannot replay. The engine emits `UndefinedLocus::Constraint(name)`, but FR-128's `replay_model_trace` does not know the locus: `undefined()` looks only for contract and `terminal when` undefinedness, so a constraint-undefined counterexample disagrees. ADR-021 SC-2, FR-158 "Truth at a state" ("a counterexample ending there that replays") and FR-160 "Undefined evaluation" need it to settle `refuted` through replay. As written, a real refutation never settles `refuted`. No wrong `proved` follows. | qsl-replay/src/model_check/replay.rs:455-500; qsl-analyze/src/model_check/check.rs:983-1004 |
| FND-003 | low | Reduction work is not metered. FR-152 "Cost" charges each sort-key comparison and the re-encoding to the run's meter. FR-161 "Length and limits" counts each concretisation step against `max_transitions` and the meter. `canonicalise` charges nothing, and `concretise` calls `successors` once per path step without counting edges. Both are bounded by work already admitted, so nothing runs away, but neither limit is honoured as stated. | qsl-eval/src/simulation/model/symmetry.rs:308-365; qsl-analyze/src/model_check/check.rs:1300-1340 |
| FND-004 | low | The outcome wire drops the payloads of the new inconclusive causes. `ItemCause::inconclusive` matches `ReductionNotPreserving`, `SymmetryBroken` and `ConstraintReached` in the no-detail arm. The wire item therefore carries only the kind, with no `boundary_states`, reduction and form, or population and generator, although `BoundReached` writes its `depth`. FR-158's use case has the operator read "`ConstraintReached` with the number of boundary states". | qsl-replay/src/outcome.rs:276-285 |
| FND-005 | low | With more than one orbit (`[[a], [b, c]]`), `check_reduced` pushes one `AppliedReduction::Symmetry` per orbit into the single `HoldsReduced.reductions`, so one record's `Reduced` basis lists symmetry twice. FR-160 "`Reduced` contents" lists each applied reduction once per instance, and "Instances" gives each orbit key its own record. This is an interim shape that the remainder's per-instance records must replace. | qsl-analyze/src/model_check/check.rs:572-585 |
| FND-006 | low | `Renaming::value` sorts a renamed set or bag with `compare_keys(..).unwrap_or(Ordering::Equal)`. An error is treated as equality, which can leave the elements out of canonical order. The result is a non-canonical value and duplicate stored states rather than a wrong verdict, but the error is lost. The state-key encoder already sorts by encoded bytes, and the same order would serve here. | qsl-eval/src/simulation/model/symmetry.rs:73-80 |
| FND-007 | low | The undefined-evaluation guard site is renamed with `permute_transition` and not checked against the subject, unlike each prefix step. A guard identity with a reference nested in an argument keeps its canonical key, so the site names a transition the concrete state does not have, and replay would disagree. This is not a wrong verdict. Check it against the expansion of the last concrete state, as the steps are. | qsl-analyze/src/model_check/check.rs:1340-1350 |

## Verdict

Changes requested. FND-001 is a soundness defect: an admitted request can
canonicalise a state outside its orbit, and a reduced run could then settle
`proved` where the unreduced run refutes. It blocks. FND-002 is a real
behaviour gap that the whole-AC tag on FR-158-AC-3 hides (SR-12206 FND-001).
FND-003 to FND-007 are recorded and do not block.

Rust lane: no `unsafe`, no `unwrap`/`expect` in non-test code, and the new
public enums are matched exhaustively. Every `ModelCheckOutcome`,
`InconclusiveCause`, `ProofBasis` and `UnavailabilityCause` match was updated.
The `u64::try_from(..).unwrap_or(u64::MAX)` widening is consistent with the
crate. `check_reduced` is long (about 190 lines) but follows `check_model`'s
shape. The `TransitionSystem` hooks are additive, with defaults that offer
nothing.
