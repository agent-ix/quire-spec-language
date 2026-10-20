---
id: SR-13595
title: "Code review of quire-spec-language PR #1115: QSL-553 E14c1, HP-3 witness sets (FR-177, FR-184)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5198fd7ba; PR #1115 (git diff origin/main...5198fd7ba): qsl-eval/src/hyper/{joint,witness,product,seam,mod}.rs, qsl-eval/src/automaton/{monitor,translate}.rs, qsl-analyze/src/model_check/hyper.rs, qsl-replay/src/model_check/{limits,hyper_*,replay}.rs, qsl-semantics/src/check/hyper_clause.rs, qsl-analyze/tests/it/hyper_witness.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: reviews
---
# SR-13595: Code review of quire-spec-language PR #1115: QSL-553 E14c1, HP-3 witness sets (FR-177, FR-184)

## Summary

Ticket: QSL-553 (E14c1). This is the code review, with the rust-review lane folded in.

Gates run at 5198fd7ba in a fresh clone with its own `CARGO_TARGET_DIR`. I did not run `make ci`. Every gate passed, with 0 tests failed:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`
- root `cargo test --test it` (975 passed, 6 ignored)
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

I wrote a brute-force HP-3 oracle in a separate scratch copy, never pushed:
`qsl-analyze/tests/it/hyper_witness_probe.rs`, on the PR's vault world with a custom-contract variant.

- **What the oracle does.** It explores the vault's own `ModelSystem` graph. It builds the (universal state, set of consistent existential tuples) product from the prefix semantics of `P0 and always Q` bodies, with no automaton. It decides the verdict from reachable empty sets with a fair universal SCC reachable from them. It uses SCC-level weak fairness, and handles `μ_U`, `μ_E`, terminal stutter, and `n = 0` with two existential variables.
- **What it checks against the engine:**
  - the verdict, including `VacuousMatch`;
  - `position` equal to the shallowest such state's depth, and `trace_position == position`;
  - every product state's witness set, projected to existential tuples, against the oracle's sets;
  - the universal lasso: a valid `μ_U` path that closes at `loop_entry`, is fair, has its first empty set at `position`, and cannot be rotated further;
  - the `max_witness_set` boundary: largest set minus 1 stops with `count` equal to the largest set, and the largest set itself completes as the default does;
  - determinism.
- **Result: 1,200 of 1,200 random cases agree** (seeds 7, 11 and 23: 520 Holds, 674 Violated, 6 Vacuous), with 0 mismatches on every check above.

So the witness-set construction is sound against FR-177 on these models:

- the safety automaton with its empty-language states removed;
- `X_0` and `X'`;
- coalescing by sorted identity;
- the universal-scoped fairness;
- the first-empty-`X` choice, the lasso and its canonical rotation;
- the budget count.

Mutation testing of the PR's own tests: 24 hand-made mutants of witness.rs, hyper.rs, product.rs and monitor.rs, each run against `qsl-analyze --test it hyper` (plus qsl-eval `monitor` for the qsl-eval files).

- **17 killed:** M01 budget `>=`; M02 `μ_E` Fails ignored; M03 no automaton step in `carry`; M04 no automaton step at `X_0`; M05 last empty state, not first; M06 no rotation; M07 empty `X` wins ties; M08 refused after empty; M09 SCC phase over every state; M11 and M12 undefined letters dropped; M15 partner without its last step; M16 monitor without viability; M19 no `count`; M22 wrong locus; M23 refused `μ_E` ignored; M24 partner initial index 0.
- **7 survived:**
  - M10, `position` = stem length: the oracle kills it, so this is a real test gap (gap FND-007).
  - M13, member contract ignored, and M21, member contract at `depth + 1`: code FND-006.
  - M17, no refinement recursion in `fair_components`: no strong-fairness test.
  - M20, a later instance wins ties: no HP-3 test has two instances.
  - M14, the `n = 0` edge-label guard, and M18, key without the automaton index: the oracle does not kill either, so they are equivalent on the bodies tested.

The author's judgement calls:

1. **`count` on `HyperOutcome::Stopped`.** Accepted. Making it `ReachedLimit`'s member would touch about 64 sites. `count = value + 1` matches FR-184-AC-2 (value 0, count 1), and the oracle's boundary check confirms it is the size of the first oversized set. FR-184-AC-3 states count = value for the tuple budget, so the spec's two counts differ in meaning. That is a spec follow-up.
2. **Tie order: undefined, then refused `match`, then empty `X`.** Accepted. AC-5 and AC-6 need exactly this order: at position 1, the Witnesses successor is discovered before the Undefined one, so discovery order alone would pick the empty `X`. FR-177's "the first in that order" should state the tie rule. That is a spec follow-up.
3. **`NoPartnerPath`.** Not accepted: code FND-001 (high) and FND-003 (medium).
4. **A member contract becomes `Undecided(UndecidedSuccessor)`.** Sound, since it never claims `Holds`. QSpec FR-400 lists a guard or state constraint of any component's subject among `undefined` counterexamples, so a member contract that is undefined arguably refutes. That needs a spec ruling (see also FND-006).
5. **`trace_position == position`, and the `ExistentialMatch{conjunct}` locus.** Confirmed against QSpec FR-400 in ~/dev/quire-specification: `trace_position` names the failing joint position, and a `match` locus is the conjunct's source expression, as HP-2's `Match{conjunct}` already is.
6. **HM-7 vacuity on HP-3.** Consistent with ADR-023 HM-7, and the oracle agrees on all 6 vacuous cases. FR-177 should say so.
7. **Canonical universal lasso rotation.** Sound: the oracle found no lasso that could be rotated further, and every lasso was valid. FR-177 says "canonical" without defining the rotation, which lives only in code. That is a spec follow-up.

Refactor check:

- `Joint` holds the letter cache, the seams, `stepping`, `joint_steps`, `order`, `record` and `poll`.
- The removed `HyperProduct` code was moved, not copied: `reading`, `matched`, `cartesian`, `Options` and the expansion block now exist once.
- `HyperProduct` keeps no shim. `cartesian` moved, with its re-export updated, and no compatibility alias was left.
- The only new duplication is in qsl-analyze (FND-005). There is no dead code beyond FND-008.
- Every new test in hyper_witness.rs carries `#[trace]`. The new monitor kernel test follows its file's untagged convention.

Spec follow-ups (spec questions, not code defects):

- FR-177 should state the tie rule between an undefined evaluation, a refused `match` and an empty `X` at one joint position.
- FR-177 should define the canonical universal lasso, including its rotation.
- FR-177 should give a verdict for an existential member whose contract evaluates undefined or refused.
- FR-177 should say what to return for an undefined `μ_U` or universal contract when `X` is empty.
- FR-177 should say how FR-126's binding ranking applies to HP-3.
- FR-184's `count` means size-reached + 1 in AC-2 but equals the value in AC-3; one meaning should be chosen.

Declared out of scope, and confirmed not built: FR-184-AC-3 (no HP-1 engine; `max_relation_tuples` has no consumer), FR-182's terminal-record marking, FR-183's witness-set recomputation, replay counting `max_witness_set`, and `Holds{Reduced}` under FR-174.

## Verdict

**Not mergeable: one high finding.**

- **FND-001 (high).** A `NoPartnerPath` refusal can replace a strictly earlier `Violated`, so a refutable HP-3 item becomes a refusal. Reproduced.
- **FND-002 (medium).** HP-3 drops FR-126's instance ranking, so a later instance's limit turns a found violation into `Stopped`. Reproduced.
- **FND-003 (medium).** The refusal itself deviates from FR-177's undefined-evaluation rule wherever `X` is non-empty. The empty-`X` case needs a spec ruling.
- **FND-004 to FND-009 (low).** Efficiency, duplication, finding order, an allow without a reason, dead API and a saturating cast.

The witness-set construction itself agrees with the brute-force oracle on all 1,200 cases. The coverage and trace findings are in SR-13596, the gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | An HP-3 `NoPartnerPath` refusal pre-empts a strictly earlier verdict. `decide_witness` calls `self.first_phase(&run)?`, and `first_phase` builds the counterexample of its own best candidate eagerly. For an undefined `μ_U`, or a universal contract with empty `X`, `undefined_counterexample` returns `Err(HyperRefusal::NoPartnerPath)`. The `?` turns the whole item into a refusal before `exhaustion` runs and before the ranking compares positions. Measured with a probe. ADR-023 §8.2's leaky `Opaque` with the added universal conjunct `a.step.op != stutter` returns `Violated(WitnessExhausted{loop_entry: 0, position: 1})`. With a `StepMatch` that makes that conjunct undefined only on `step(1)` from BFS layer 1 on (joint position 2), the item returns `Err(Hyper(NoPartnerPath))`. FR-177 'Undefined and undetermined evaluation' says the first finding in canonical order wins, which is the empty `X` at position 1. Fix: rank the candidates first and build a counterexample only for the one that wins, or remove the refusal (FND-003). The probe is a scratch copy at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/rv1115-probe, never pushed: qsl-analyze/tests/it/hyper_witness_probe.rs plus a `Vault::Custom` contract variant in hyper_world.rs. | qsl-analyze/src/model_check/hyper.rs:592-605, 1443-1446, 1496-1499 |
| FND-002 | medium | HP-3 instances drop FR-126's binding ranking. `decide_witness` calls `explore(instance, None, ..)` for every instance with no depth cap, and returns any later instance's `Err(outcome)` at once. So a later instance that reaches a limit turns an earlier instance's violation into `Stopped`. The HP-2 path in `decide` (hyper.rs:513-533) breaks after a depth-0 finding and caps later instances at depth `d`. Measured with a probe. `World::with_vaults(Leaky, [v (0,0), w (1,0)], Opaque)`, default limits: `Violated(WitnessExhausted{position: 0})` on instance `v`. With `max_states` 5, 6 or 7: `Stopped(ResourceExhausted, MaxStates)`, because instance `v` completed and instance `w` ran out. From 8 up the result is `Violated` again. A depth cap does not port directly to the second phase, which needs the fair cycle beyond the empty-`X` state. At least a position-0 finding (and a first-phase hit at depth `d`) should bound the later instances as HP-2 does, and FR-177 should say how FR-126's rule reads for HP-3. Mutant M20 (`<` to `<=` in the ranking) survives the suite: no HP-3 test has two instances. | qsl-analyze/src/model_check/hyper.rs:573-623 |
| FND-003 | medium | `HyperRefusal::NoPartnerPath` refuses where FR-177 says the engine SHALL return `Violated` with an `Undefined` counterexample. The two arms that need an existential path also disagree. The universal-contract arm takes the first member of the state's `X` as the path, and refuses only when `X` is empty. The `μ_U` arm refuses always, even when `X` has members whose partner path reaches the pre-state. QSpec FR-400 (checked at ~/dev/quire-specification) lets an `undefined` counterexample's traces be finite paths of unequal length, and imposes no equal-step rule on that kind. So a member's partner path to the pre-state of an undefined `μ_U` step is a valid existential trace whenever `X` is non-empty. Only an empty `X` leaves no existential path. FR-177 does not cover that case, and it needs a spec ruling: flag for /specify, not a guess in code. The test `an_undefined_universal_match_of_an_hp3_clause_is_refused_not_guessed` (hyper_witness.rs:365-385) enshrines the refusal under an `FR-177-AC-5` tag. | qsl-analyze/src/model_check/hyper.rs:1438-1446, 1484-1499 |
| FND-004 | low | `carry` re-expands every member's existential components for every universal joint step. `self.joint.stepping(self.universal + offset, model)` is inside `for member in members`, which `successors` calls once per universal step. A product state with `k` universal steps and `\|X\|` members costs `k * \|X\| * E` `ModelSystem::successors` calls, each of which evaluates contracts, instead of `\|X\| * E`. `max_witness_set` defaults to 65,536 members, so this is the hot loop of an HP-3 run. Expand each member once per `successors` call (keyed by its digests) and reuse the options across universal steps. The duplicated `MemberContract` finding, pushed once per universal step per member, goes away with it. | qsl-eval/src/hyper/witness.rs:395-415 |
| FND-005 | low | One fact is in two places. `fair_components` re-implements the depth-first refinement walk of `Analysis::first_fair`/`first_fair_in` (qsl-eval/src/fairness/refine.rs:96-112), but collects every fair component where those return the first. `exhaustion` also copies `second_phase`'s analyze-error mapping and its `greedy_loop` -> `loop_edges` -> `outgoing().nth()` loop-path block (hyper.rs:1741-1790). A change to the refinement order (ADR-019 FS-6), or to the visibility-removal rule, now has two walks to keep equal, and only one of them is in qsl-eval beside the filter. Put an `Analysis::fair_in` iterator in qsl-eval and share a `loop_path(model, passing, entry)` helper between the two phases. Mutant M17 (drop the refinement recursion in `fair_components`) survives the suite. | qsl-analyze/src/model_check/hyper.rs:1864-1876, 652-760 |
| FND-006 | low | `first_phase` keeps the first refused finding in a state's list (`refused.get_or_insert`). In a witness state, a `MatchRefused` at position `d+1` (a refused `μ_U` or `μ_E` on an earlier universal step) can come before a `MemberContract` at position `d`. The state's refused candidate is then offered at `d+1` as `MatchUndetermined`, not at `d` as `UndecidedSuccessor`. That changes both the cause and how it ranks against an undefined evaluation or an empty `X` at `d+1`. In HP-2, contract findings always precede step findings, so first-in-list was the minimum. `WitnessProduct::successors` interleaves them per universal step. Take the minimum `(position, cause)` instead of the first. Mutants M13 (ignore member contracts) and M21 (`MemberContract` at `depth + 1`) both survive, so no test runs this path. | qsl-analyze/src/model_check/hyper.rs:1236-1252 |
| FND-007 | low | New `#[allow(clippy::too_many_arguments)]` on `Joint::new`, with no reason comment. The repo's stated idiom is to bundle the inputs rather than allow the lint (qsl-semantics/src/check/family.rs:1184-1185). Both callers already hold a bundle (`HyperInputs`, `WitnessInputs`). Rust-review §5: no new allow without a stated reason. Pass a borrowed inputs struct, or give the allow its reason. | qsl-eval/src/hyper/joint.rs:80-90 |
| FND-008 | low | This adds public API with no production caller. `ModelCheckLimits::hyper_used()` is called only by its own test (hyper_witness.rs:599). `WitnessProduct::systems()` (witness.rs:337) has no caller at all. `hyper_used` stands in for FR-182's terminal record, which is not built, so the test that asserts it binds FR-184-AC-1 to a list no record emits (see the gap analysis). Wire both into a consumer, or leave them out until FR-182 lands. | qsl-replay/src/model_check/limits.rs:84-92 |
| FND-009 | low | `WitnessProduct::start` numbers the existential initial states with `u32::try_from(index).unwrap_or(u32::MAX)`. An index past `u32` would silently become `u32::MAX` and come back as a wrong `initial` in a counterexample's existential trace. The engine's own `traces` refuses the same conversion with a fault (`a-root-component-is-an-initial-state`, hyper.rs:1346). Rust-review §7 (boundary conversions that persist into the wire). Record `Halt::Fault` instead of saturating. | qsl-eval/src/hyper/witness.rs:262-271 |
