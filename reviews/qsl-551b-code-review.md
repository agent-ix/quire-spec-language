---
id: SR-13624
title: "Code review of quire-spec-language PR #1128: QSL-551 FR-182 hyper settlement, WitnessExhausted replay, max_depth for HP-2/HP-3"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6d17c8704; PR #1128 (git diff origin/main...6d17c8704): qsl-replay/src/model_check/{hyper_settle,hyper_replay,hyper_outcome,outcome,replay,settle}.rs, qsl-replay/src/{lib,model_check}.rs, qsl-analyze/src/model_check/hyper.rs, qsl-analyze/src/model_check/hyper/projected.rs, qsl-analyze/tests/it/{hyper_settle,hyper_replay,hyper_witness,hyper_world,hyper,hyper_projected,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: reviews
---
# SR-13624: Code review of quire-spec-language PR #1128: QSL-551 FR-182 hyper settlement, WitnessExhausted replay, max_depth for HP-2/HP-3

## Summary

Ticket: QSL-551 (E14 remainder). This is the code review, with the rust-review lane folded in.

Gates run at 6d17c8704 in a fresh clone with its own `CARGO_TARGET_DIR`. I did not run `make ci`. Every gate passed, with 0 tests failed (4,960 passed, 7 ignored in all):

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`
- root `cargo test --test it` (975 passed, 6 ignored)
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

The branch does not merge onto main: main moved on with #1123 (QSL-555, copy-swap), and `git merge-tree` reports conflicts in `qsl-analyze/src/model_check/hyper.rs`, `qsl-analyze/tests/it/main.rs` and `qsl-replay/src/lib.rs` (FND-005).

**Oracle.** In a scratch copy that was never pushed (`qsl-analyze/tests/it/hyper_depth_probe.rs` at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/rv1128/probe), I compared each bounded run with the unbounded run of the same item.

- **The models.** Random vault contracts: 9 `step` contracts (set, leaky, flip, invert, enabled at `l = 0` only, enabled at `l = 1` only, constant, enabled at `i = h` only, `l` follows `h`) and, half the time, 4 `reset` contracts. Random starts, 1 to 4 of the `(h, l)` pairs.
- **The clauses.** Half HP-2: two universal copies with random fairness (also asymmetric), 3 `match` blocks and 12 safety and liveness bodies. Half HP-3: `forall a exists b` with random fairness on `a`, 4 `match` blocks (`μ_E` input or op equality, a `μ_U` conjunct) and 6 safety bodies.
- **What it checks.** For every horizon `k` from 0 to the product size + 2: a bounded `Holds` only when the unbounded run holds; a bounded `Violated` only when the unbounded run is violated, and its counterexample replays `Reproduced`; no `BoundReached` after a smaller `k` was `Violated`; `BoundReached` carries `k` and `JointSteps`; and a horizon past the product gives the unbounded outcome. Every unbounded counterexample, `WitnessExhausted` included, must replay `Reproduced`.
- **Result.** 600 cases over seeds 7, 11, 23 and 41, none skipped: 200 HP-2 refutations, 168 HP-3 refutations and 232 holding items. All 368 unbounded counterexamples replay `Reproduced`, every bounded counterexample replays, and no bounded run holds, refutes or bounds where it should not. A violation reachable only beyond `k` gives `BoundReached`, never `Holds`, and a lasso longer than `k` found inside the retained graph is reported `Violated` (FR-126-AC-5's reading; 58 cases). 40 bounded refutations have a different, still replaying, lasso from the unbounded one.
- **HV-3 for HP-3.** In 15 of the 168 HP-3 refutations, and in none of the HP-2 ones, the run returns `BoundReached{k}` while the unbounded counterexample's universal lasso has at most `k` joint steps (FND-003).

**Targeted probes** (same scratch file):

- `reviewer_mu_e_incomplete_replay`: ADR-023 section 8.2's leaky `Opaque` counterexample replayed over the secure vault, with a `μ_E` whose evaluation is incomplete. The engine on that model and seam returns `Undecided(MatchUndetermined)`. The replay returns `Reproduced{Truth(false), Some(1)}`, and `settle_hyper_check` settles `Refuted`, `decisive-counterexample` (FND-001). The same probe shows the replay-stopped record with `count: None`, beside the engine-stopped record's `Some(1)` (FND-004).
- `reviewer_gradual_exhaustion_position_past_lasso`: the engine's own `WitnessExhausted` counterexample, `position: 2` on a one-step universal lasso, is refused by the new replay shape rule, and the item settles `inconclusive`, `ReplayRefused(invalid_runtime_input)` instead of `refuted` (FND-002).

Mutation testing of the PR's own tests: 33 hand-made mutants of hyper_settle.rs, hyper_replay.rs, outcome.rs, settle.rs, hyper.rs and projected.rs, each run against `qsl-analyze --test it hyper` plus `cargo test -p qsl-replay` for qsl-replay files. 25 were killed, 8 survived, and 2 of the 8 are equivalent.

- **25 killed:**
  - settlement: S1, a reproduced refutation carries no counterexample; S2, a disagreeing replay refutes; S3, a stopped replay settles inconclusive; S4, every limit marked published default; S5, the engine stop's count dropped; S6, the depth dropped; S7, the coverage dropped; S8, `max_witness_set` reports `max_relation_tuples`' value; S9, `VacuousMatch` settles `MatchUndetermined`;
  - engine: E1, the HP-2 horizon ignored; E2, the HP-3 horizon ignored; E3 and E4, a bounded HP-2 or HP-3 run reads `Holds`; E8, HP-6 records `JointSteps`; E9, the HP-2 horizon one deeper;
  - replay: R1, the universal lasso unchecked; R2, any empty set reproduces; R3, a witness-set halt completes no value; R4, a position past the lasso accepted (killed by the test that encodes FND-002's refusal); R6, `μ_E` not read by the `Undefined` replay; R9, the sets recomputed one position short; R10, a non-empty `X_position` reproduces; R12, a trace demanded for the existential variable; R13, the loop entry unchecked; R14, the trace position unchecked.
- **8 survived** (gaps in SR-13625):
  - S10, the new `Stopped` arm of FR-127's liveness `settle` settles anything at all: no liveness replay returns `Stopped`;
  - E5, the HP-3 ranking cap takes the larger of ranking and horizon: no test has two instances and a horizon;
  - R5, the 'not a forall-exists clause' refusal removed: the test meant for it is refused by the variable-name rule first;
  - R7, the `Undefined` replay's `μ_U` reads every trace, and R8, a `μ_E` that `Fails` on an `Undefined` path is ignored: neither the universal-only reading nor the new `Fails` arm is tested;
  - R11, the replay steps to any successor with the same transition without checking its post-states: no fixture has a nondeterministic step;
  - E6 and E7, `bounded` set even after a finding, are equivalent: `bounded` is read only when no finding was made.

The author's judgement calls:

1. **`max_depth` for HP-2 and HP-3 by analogy with FR-126-AC-5.** Accepted for HP-2: the oracle agrees on every case, and a lasso longer than `k` inside the retained graph is a real counterexample (the retained graph is a subgraph of the full product, and only expanded states have edges). For HP-3 the horizon counts product depth, while HV-3 and the record's `BoundCovers::JointSteps` speak of the universal lasso, which FR-177's rotation can make shorter (FND-003).
2. **`requested: &[ModelCheckLimit]` for the published-defaults marking.** Accepted as a stopgap: `ModelCheckLimits` holds plain values and no request type says which members were set, so the caller must say so. The spec gap (FR-184-AC-1 needs that request input) is real and is listed in the gap analysis.
3. **Widened test seams.** `IDLE`, `MID_OPERATION`, `aligned` and `ExhaustedExistential` are `pub(crate)` in the integration-test crate only, each with a new caller in `hyper_settle.rs`; `opaque` moved to `hyper_world.rs` with two callers. No production visibility changed. Accepted.
4. **Spec gaps in the PR body.** Gap 1 (no `max_depth` AC in FR-176/177) is real, and FND-003 adds to it. Gap 2 (the traceless HP-3 counterexample with no universal variable) is real and correctly refused. Gap 3 (FR-183 'equal step counts' against FR-177-AC-10's short paths) is real. Gap 4 is judgement call 2.
5. **`Holds` refused with `ProductClosureCheckerNotBuilt`.** Accepted: it settles no `proved` that no checker stands behind, as `LivenessRefusal::ComponentCertificateNotBuilt` does for TP-4. No verdict is replaced by a refusal: every other `HyperOutcome` arm settles a record.
6. **Deleted stand-ins.** `HyperRefusal::HorizonNotDecided` and `ModelTraceRefusal::HyperWitnessExhausted` are gone from code and tests (only historical `reviews/` mention them). No shim or alias remains; `UndecidedCause::inconclusive` replaces the duplicated V-6 mapping in `settle.rs`.

## Verdict

**Not mergeable.** Two high findings change a verdict, and the branch conflicts with main.

- **FND-001 (high):** the `WitnessExhausted` replay ignores the witness product's findings, so an empty `X` that comes from a refused or incomplete `μ_E`, or from a member whose existential contracts are undecided, reproduces and settles `refuted`. That is a refutation of a property that holds, settled by the very check that is meant to confirm the engine.
- **FND-002 (high):** the new rule `position <= steps` refuses counterexamples the engine itself emits under FR-177's canonical rotation, so a real violation settles `inconclusive`.
- **FND-003 (medium):** an HP-3 `BoundReached{k}` record states a coverage the run did not reach.
- **FND-004 (medium):** a replay that `max_witness_set` stopped settles V-7 with no count.
- **FND-005 (medium):** the branch conflicts with main; the rebase must keep #1123's copy-swap and drop the stale module doc saying it is not built.
- **FND-006, FND-007 (low):** a stale fixture doc and two doc comments that open with an AC id.

What holds up: the settlement rows match FR-127/FR-182 (V-4 only through replay; `ReplayParity`, `ReplayRefused(code)`, `failed` for a fault, V-7 for a stopped replay, V-5 with depth and coverage, V-6 causes, V-7 limit/value/count for an engine stop, cancellation), every record states the five limits, the count is one past the budget for an engine stop, HP-6 records `Moves`, the HP-2 and HP-3 horizons are sound in the oracle, the `Undefined` replay's `μ_E` and universal-only `μ_U` reading are right, and the deleted stand-ins are really gone.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The `WitnessExhausted` replay can reproduce a refutation the model does not have. `witness_exhausted` walks `product.successors(&current)` and reads only `expansion.successors`; it never reads `expansion.findings` (nor the initial state's). `WitnessProduct::carry` drops a member whose `μ_E` is refused or incomplete (`HyperFinding::MatchRefused`), a member whose existential expansion has contract findings (`MemberContract`), and a partner whose letter is undefined (an `Undefined` sibling state). The engine ranks those findings before an empty `X` and settles `MatchUndetermined`, `UndecidedSuccessor` or `Undefined`. The replay treats the shrunken set as evidence. Reproduction: ADR-023 section 8.2's leaky `Opaque` counterexample replayed over the secure vault, whose `Opaque` holds, with a `μ_E` whose meter is exhausted (`ExhaustedExistential`): the engine on that model says `Undecided(MatchUndetermined)`; `replay_hyper_trace_tuple` returns `Reproduced{Truth(false), trace_position: Some(1)}`, and `settle_hyper_check` settles `Refuted`, `decisive-counterexample`. The same happens whenever the replay's meter is smaller than the engine's. Fix: read the findings of every expansion on the path up to `position` (and of `start`): a `MatchRefused`, `MemberContract` or refused contract settles no value, and an undefined `μ_U`/`μ_E`/letter (or an `Undefined` sibling of the stepped state) settles `inconclusive`, `Verdicts`, since the engine would have reported it first. Add the reproduction as a test. | qsl-replay/src/model_check/hyper_replay.rs:585-600 |
| FND-002 | high | The new shape rule refuses the engine's own counterexamples. `shape` refuses a `WitnessExhausted` whose `position` exceeds the lasso's step count, and the PR body says the engine's counterexamples satisfy that. They do not: FR-177 requires the loop entry to rotate back while the lasso's last step repeats the step before the entry, and `exhaustion` shortens the trace by one step each time, so a set that empties gradually under a self-looping universal trace gives `position` past the shortened lasso. Reproduction: `Vault::Custom("post Step using v on V::Vault::step { self.l = i }")`, starts `(0, 0)` and `(1, 1)`, `forall trace a of V exists trace b of V match { a.step.Vault::step.i = b.step.Vault::step.i } { holds(v.h @ a != v.h @ b) and always (once[2,2] holds(v.l @ b = 1) implies holds(v.l @ b = 1)) }`. The engine returns `WitnessExhausted{loop_entry: 0, position: 2}` with a one-step trace (`step(0)` self-loop at `(0, 0)`; `X_0` and `X_1` hold `b` from `(1, 1)`, `X_2` is empty). The replay refuses it `HyperShape("names a joint position other than its trace position, or past its lasso")`, and the item settles `inconclusive`, `ReplayRefused(invalid_runtime_input)` instead of `refuted`. The same with `once[1,1] once[1,1]`. Fix: replay positions past the steps around the loop (joint `p >= steps` reads step `entry + (p - entry) mod (steps - entry)`), drop the 'past its lasso' refusal, and add the reproduction as a test; `a_witness_exhausted_counterexample_that_breaks_a_replay_rule_is_refused` asserts the wrong refusal today. | qsl-replay/src/model_check/hyper_replay.rs:216-224, 576-580; qsl-analyze/src/model_check/hyper.rs:949-967 |
| FND-003 | medium | An HP-3 `BoundReached{k}` states a coverage the run did not reach. The record says `BoundCovers::JointSteps`, 'no counterexample whose universal lasso has at most `depth` joint steps' (ADR-023 HV-3, FR-182's V-5 bullet). The HP-3 horizon bounds product depth, and the product path to an empty `X` and around its fair loop is longer than the rotated universal lasso the engine reports. The PR's own test shows it: over the leaky vault, `max_depth` 1 returns `BoundReached{1, JointSteps}`, while the counterexample found at `max_depth` 2 and unbounded is a one-step lasso (`a_horizon_cuts_the_witness_set_search_before_its_lasso`). The oracle finds 15 such HP-3 cases in 168, and none for HP-2. The verdict stays `inconclusive`, but the stated coverage is false. Fix: either state an HP-3 coverage that the horizon guarantees (counterexamples whose product path, stem to the empty set plus loop, has at most `k` joint steps) and say so in `BoundCovers`, or explore until every universal lasso of at most `k` steps has been read. HV-3 needs a spec decision either way (gap analysis). | qsl-analyze/src/model_check/hyper.rs:707-775; qsl-replay/src/model_check/hyper_outcome.rs:17-26 |
| FND-004 | medium | A replay that `max_witness_set` stopped settles V-7 with no count. FR-184: 'A stopped record for `max_witness_set` or `max_relation_tuples` SHALL carry the count reached, one past the budget's value', and FR-184 says the budget is counted during replay too. `halted` discards `Halt::WitnessSet(count)` and returns `ModelTraceSettlement::Stopped(ReachedLimit)`, which has no count, and `settle_hyper_check` leaves `count` `None` on that arm. The probe: the leaky `Opaque` with `max_witness_set` 0 settles `count: Some(1)` from the engine and `count: None` from the replay. `a_refutation_that_does_not_replay_settles_with_no_counterexample` does not assert the count. Fix: carry the count in the stopped replay result and set it on the record. | qsl-replay/src/model_check/hyper_replay.rs:624-633; qsl-replay/src/model_check/hyper_settle.rs:121-123 |
| FND-005 | medium | The branch does not merge onto main (97654ffd4): conflicts in `qsl-analyze/src/model_check/hyper.rs`, `qsl-analyze/tests/it/main.rs` and `qsl-replay/src/lib.rs`, from #1123 (copy-swap). The module doc this PR rewrites still lists 'the copy-swap reduction (FR-174)' as not built, which is false on main. Rebase or merge main, keep both features (`HoldsReduced`, `reductions`, `copy_swap` beside `bounded`/`covers`), fix the doc, and re-run the gates and the oracle on the merge. | qsl-analyze/src/model_check/hyper.rs:75-80 |
| FND-006 | low | Stale doc: `World::max_depth` is documented 'The search horizon of an HP-6 run (`max_depth`)', but HP-2 and HP-3 runs now read it too. | qsl-analyze/tests/it/hyper_world.rs:406 |
| FND-007 | low | Two new doc comments open with an AC id: `/// FR-177-AC-5's outcome settles ...` above the `FR-182-AC-8` test, and `/// FR-177-AC-5's Undefined counterexample ...` above the `FR-183-AC-6` test. The trace tag carries the id; the doc should say what the test checks. | qsl-analyze/tests/it/hyper_settle.rs:199; qsl-analyze/tests/it/hyper_witness.rs:1243 |
