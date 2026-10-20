---
id: SR-13601
title: "Code review of quire-spec-language PR #1121: QSL-554 E14c2, HP-6 projection with three-valued prefix (FR-178)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b45e62a97; PR #1121 (git diff origin/main...b45e62a97): qsl-eval/src/hyper/{projected,product,joint,mod,witness}.rs, qsl-eval/src/temporal/{tuple,outcome,mod}.rs, qsl-eval/src/simulation/model.rs, qsl-analyze/src/model_check/hyper.rs, qsl-analyze/src/model_check/hyper/projected.rs, qsl-replay/src/model_check/{hyper_counterexample,hyper_outcome,hyper_replay,replay}.rs, qsl-replay/src/spine/clause.rs, qsl-analyze/tests/it/{hyper_projected,hyper_world,hyper}.rs, qsl-eval/tests/it/hyper_tuple.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: reviews
---
# SR-13601: Code review of quire-spec-language PR #1121: QSL-554 E14c2, HP-6 projection with three-valued prefix (FR-178)

## Summary

Ticket: QSL-554 (E14c2). This is the code review, with the rust-review lane folded in.

Gates run at b45e62a97 in a fresh clone with its own `CARGO_TARGET_DIR`. I did not run `make ci`. Every gate passed, with 0 tests failed (4,849 passed, 7 ignored in all):

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`
- root `cargo test --test it` (975 passed, 6 ignored)
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

I wrote a brute-force HP-6 oracle in a separate scratch copy that was never pushed: `qsl-analyze/tests/it/hyper_projected_probe.rs` at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/rv1121-probe, plus a `Vault::CustomBusy` variant in hyper_world.rs for random busy-vault contracts.

- **The models.** Random `step` and `mix` contracts over `h`, `l` and `busy`. Random starts, from 1 to 3 of the 8 `(h, l, busy)` triples. Random `align skip` lists: `{mix}`, `{step}` or both. Random `μ_U` from the conjuncts the fixture's seams read: the AC-1 conditional, op equality, input equality and the AC-5 division, in random order. Random bodies `P0 and always Q`, with `Q` an equality, a constant, or a disjunction of two atoms.
- **What the oracle does.** It reads ADR-023 PA-3 directly, with no monitor. It explores tuples of the vault's own `ModelSystem` states with breadth-first distances. Skip moves move one component. Visible moves need `μ_U`, read in conjunct order: the first that fails or is undefined decides. The body's bad prefix is read at each projected position.
- **What it checks against the engine:**
  - the verdict: `Holds`, `Violated` with kind `Projected` or `Undefined`, or `BoundReached`;
  - the product state count of every exhaustive run;
  - each counterexample's traces, re-executed: every step an edge of the model, the marks matching the skip list, equal visible counts, `trace_position` equal to the visible count, `μ_U` holding at every earlier joint visible step, every earlier projected position good, and the last bad (`Projected`) or undefined at the named conjunct with cause `division-by-zero` (`Undefined`, unmarked);
  - the total moves equal to the shallowest bad state's distance;
  - with a random horizon `k` from 0 to depth + 1: a counterexample at exactly `k` moves is found, one at `k + 1` gives `BoundReached{depth: k}` with `stats.depth <= k`, and a run whose graph ends before `k` holds;
  - determinism.
- **Result: 1,540 of 1,540 random cases agree** (seeds 7, 11, 23 and 41: 240 Holds, 1,216 `Projected`, 59 `Undefined` and 25 ties between the two), with 0 mismatches. The 1,540 horizon runs also agree: 182 `BoundReached`, 130 `Holds`, 1,228 `Violated`. 57 projected counterexamples hold skipped steps, 37 of them with unequal lengths. Every exhaustive run's product state count matches the oracle's.
- **Limit.** On a tie between a rejecting state and an undefined `μ_U` at one depth, the oracle accepts either kind (gap FND-004).

So the projected product is sound against PA-3 and PA-4 on these models: the skip and visible moves, the monitor reading only joint visible positions, the rejecting, undefined-letter and undefined-`μ_U` states as product states, the move-counting horizon, `trace_position`, and the marks.

Replay: every `Projected` counterexample (1,216) is refused `unsupported_construct/not-yet-implemented`, as the PR intends. Every skip-bearing `Undefined` counterexample (7 of 7) is refused as malformed (FND-001).

Mutation testing of the PR's own tests: 27 hand-made mutants of the two projected.rs files, hyper.rs, tuple.rs, hyper_replay.rs and model.rs. Each was run against `qsl-analyze --test it hyper`, plus `qsl-eval --test it hyper` for the qsl-eval files. 17 were killed and 10 survived.

- **17 killed:** P01, position counts every move; P02, no skip moves; P03, skip moves for the first component only; P04, visible moves include skipped steps; P06, horizon ignored; P07, `BoundReached` depth + 1; P08, never bounded; P10, no `HorizonNotDecided`; P11, every mark visible; P12, no marks; P15, `Undefined` stem counts every move; P18, the monitor reads pre-states; P19, no matcher; P20, a refused `match` dropped; P22, the prefix read as a lasso; P23, projected components stutter; P26, replay of a `Projected` counterexample not refused.
- **10 survived:**
  - P09, P16 and P21: the rejecting-state confirmation (removed, reading skip-move states, meter exhaustion ignored). Gap FND-002.
  - P13 and P14: an undefined `μ_U` or a rejecting state ranked one depth later; no test has the HV-8 tie. Gap FND-004.
  - P27, the operation text matched by name only; P28, no break after a depth-0 finding; P30, a rejecting initial state read as accepting. These are untested paths: two aliases, two instances, a bad projected position 0. Gap FND-004.
  - P24 (no `len == 0` check: the engine refuses `EmptyTrace` itself) and P25 (the terminal-stutter filter: the skip and visible filters already take model steps only) are equivalent.

The author's judgement calls:

1. **The AC-2 fixture.** The spec is contradictory, and the author's fixture is right. From idle starts the breadth-first first rejecting state is one visible move of each run, so AC-2's unequal lengths and AC-4's `BoundReached` at depth 1 cannot both hold on §13's vault. The mid-operation start is the smallest fixture that meets both ACs. The spec fix is gap FND-001: name the start in AC-2, AC-4 and TC-603, and write 'total lengths free' in ADR-023 PA-6 and QSpec FR-398, as QSpec FR-400 already does.
2. **No HM-7 vacuity on HP-6.** Accepted. PA-3 quantifies over matched projected prefixes, and the `L = 0` prefix of every initial tuple is always matched, so the matched domain is never empty and HM-7 ('no tuple of fair universal behaviours on which `μ_U` holds at every joint step') has no HP-6 case. FR-178 states `Holds{Exhaustive}`. A spec follow-up: FR-178 should say in one sentence that `VacuousMatch` does not apply to HP-6, or define a projected vacuity (no joint visible step ever matched) if the owners want one. No VacuousMatch rule should be coded against the spec as it stands.
3. **`Undefined` counterexamples of unequal length, `position` = visible steps reached.** Accepted for the engine. HV-8 names the joint position, which for HP-6 is the projected position. QSpec FR-400's `undefined` kind takes finite paths with free lengths, and FR-183 sets equal step counts only for `Lockstep` and `WitnessExhausted`. The oracle confirms both on all 59 cases. But replay refuses these as malformed: code FND-001.
4. **Marks as a parallel `HyperTrace.marks` field.** It works, and every engine site fills it correctly. It leaves the length rule to the reader instead of the type: FND-004 (low, low confidence).
5. **Structural and `Debug` equality for 'byte-equal' (AC-4).** Accepted. This is the precedent merged with #1092 (FR-176-AC-4): a `HyperCounterexample` has no wire encoding until QSpec FR-400's writer exists.
6. **Seams that read `holds` and `match` from source text.** Accepted as a test fixture, the same as #1092 and #1115. The probe uses the same seams, so it checks the product and the semantics independently, not atom evaluation.

Other checks:

- **ACs and failing tests.** Every AC from 1 to 6 has a tagged test that can fail. Mutants kill the position counting (P01, P15), the skip-move generation (P02, P03, P04), the horizon (P06), the `BoundReached` depth (P07, P08), `HorizonNotDecided` (P10), the marks (P11, P12) and the monitor's post-state reading (P18). The rejecting-state confirmation survives (P09, P16, P21): gap FND-002.
- **The two TC-603-only tests.** `from_idle_starts_the_leak_is_one_visible_step_away` and `a_projected_counterexample_is_refused_by_replay_until_fr_183` are fine as TC-only tags, the convention #1115's fix round settled on. But the first one's doc claims a confirmation it does not assert (gap FND-002).
- **Dead code and compat layers.** There are none. `Built::skipped` is used. The `FirstHit::Unstated` arm in `decide_projected` and `FirstHit::Stopped` in `decide` are exhaustiveness arms. `HyperTransition::labels` now returns `Option`, and both callers were updated with a fault on `None`. No alias or shim remains.
- **The `pub(crate)` widenings.** The test helpers `Refusing`, `Exhausted` and `input` became `pub(crate)`; that is fine. No production item was widened. The new `pub` API is all used, except `ModelSystem::operation_text_of`, which has only an in-crate caller (FND-006).
- **The reworded `AlignSkipNotImplemented` refusal.** It now gives the wrong reason: FND-005.
- **Duplication.** `decide_projected` copies `decide`'s ranking loop (FND-007).

Spec follow-ups (spec questions, not code defects):

- The FR-178-AC-2, AC-4 and TC-603 fixture, and 'total lengths free' in ADR-023 PA-6 and QSpec FR-398 (gap FND-001).
- FR-178 should say whether HM-7 `VacuousMatch` applies to HP-6 (judgement call 2).
- FR-183 should say how an HP-6 `Undefined` counterexample is replayed: projected positions and skipped steps (code FND-001).
- FR-178 should state the tie rule between a rejecting state, an undefined letter and an undefined `μ_U` at one depth. HV-8's 'first product state in breadth-first order' is what the code does.

Declared out of scope, and confirmed not built: FR-183's HP-6 replay (PA-7; refused `HyperProjected`), FR-182's settlement, and `Holds{Reduced}` under FR-174.

## Verdict

**Mergeable after FND-001 is fixed.** FND-001 is the only finding above low, and the engine's result itself is sound.

- **FND-001 (medium).** Replay refuses an HP-6 `Undefined` counterexample with a skipped step as malformed input, where it should refuse it as not yet implemented, as it does `Projected`. Reproduced on the PR's own AC-5 fixture.
- **FND-002 to FND-007 (low).** A missing profile guard on the projected evaluator, a doc overclaim, the parallel marks field, the reworded refusal, a `pub` method with no external caller, and the copied ranking loop.

The projected product, the horizon, positions, marks and counterexamples agree with the brute-force oracle on all 1,540 cases. The coverage and trace findings, and the spec contradiction behind the AC-2 fixture, are in SR-13602, the gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Replay refuses an HP-6 `Undefined` counterexample whose paths hold a skipped step as malformed input. `replay_hyper_trace_tuple` refuses `Projected` as not yet implemented (`HyperProjected`), but lets every `Undefined` counterexample through to the lockstep `shape` check, which requires equal step counts and `position == steps`. The PR's engine ends an HP-6 `Undefined` counterexample's paths at the projected position (the visible steps), as FR-178 and ADR-023 HV-8 ask, so any skipped step makes `shape` refuse it `invalid_runtime_input`/`invalid-value`. Reproduced: the counterexample of the PR's own AC-5 fixture `an_undefined_match_after_skip_moves_is_at_the_projected_position` (both runs start busy, paths of 2 steps, position 1) replays to `invalid_runtime_input/invalid-value: the hyper counterexample does not end its paths at the joint position it names`. In the probe, 7 of 7 skip-bearing HP-6 `Undefined` counterexamples are refused this way, and the 77 skip-free ones are replayed under lockstep, which coincides with the projection only because nothing was skipped. FR-183 refuses unequal step counts only for `Lockstep` and `WitnessExhausted`, and QSpec FR-400's `undefined` kind sets no equal-step rule. HV-2 settles `refuted` only after replay, so such an item reports a malformed counterexample instead of the known replay gap. Fix: until FR-183's HP-6 replay is built, refuse an `Undefined` counterexample of a clause with a non-empty `align_skip` as `HyperProjected` (`unsupported_construct`/`not-yet-implemented`), with a test, or make the `Undefined` shape and evaluation read projected positions. | qsl-replay/src/model_check/hyper_replay.rs:97-105, 151-158, 175-183 |
| FND-002 | low | `evaluate_projected_prefix` checks neither the clause's profile nor its form. It runs `TemporalTrace::Finite { len }` under `body.profile()`, and the engine reads a finite trace three-valued (`Mode::Prefix`) only under `quire.temporal.infinite-trace/v1`. Under a bounded profile it reads it closed-world (`Mode::Closed`), two-valued, so the function silently returns `false` for prefixes that are not bad prefixes, against its doc ('three-valued'). `evaluate_tuple` on the same clause is refused (`LassoUnderBoundedProfile`), so the lockstep entry cannot make this mistake and the projected one can. A hyper clause under another profile has no form (hyper_clause.rs:291-292), so refuse unless `body.form()` is `ProjectionAligned` (or at least the profile is infinite-trace). No caller passes such a clause today. | qsl-eval/src/temporal/tuple.rs:388-425 |
| FND-003 | low | The `HyperCounterexampleKind::Projected` doc says 'the traces are of unequal total length' as an invariant. The engine does not keep it, and should not: from idle starts the canonical counterexample is one visible step of each trace, equal lengths, which the PR's own `from_idle_starts_the_leak_is_one_visible_step_away` asserts. The probe saw 37 projected counterexamples with unequal lengths among 1,216, the rest equal. QSpec FR-400 says 'total lengths free'. Reword to 'total lengths free; they differ when a trace takes skipped steps'. The same overclaim is in ADR-023 PA-6 and QSpec FR-398 (gap analysis SR-13602 FND-001). | qsl-replay/src/model_check/hyper_counterexample.rs:118-123 |
| FND-004 | low | Marks are a parallel `HyperTrace.marks: Vec<StepMark>` beside `steps`. The type can hold a marks list of the wrong length, or marks on a `Lockstep`, `WitnessExhausted` or `Undefined` trace, which QSpec FR-400 refuses ('a `mark` on a step of any other kind'). Every construction site must remember `marks: Vec::new()`. The author's judgement call. QSpec FR-400 puts `mark` on each step, and FR-183's struct comment says the steps carry it. A step-level `Option<StepMark>` (or a `Projected` arm that owns marked traces) makes the length rule a type fact, and FR-183's reader will otherwise have to check it by hand. Low confidence: the parallel field works today, and every engine site fills it correctly. | qsl-replay/src/model_check/hyper_counterexample.rs:49-54 |
| FND-005 | low | The reworded `AlignSkipNotImplemented` refusal now names the wrong reason. Refusing an `align skip` clause in `evaluate_tuple` is permanent, because such a clause is read by `evaluate_projected_prefix` and never in lockstep. The variant keeps the `NotImplemented` name and the `unsupported_construct`/`not-yet-implemented` code, and the text points at 'its replay', which the layer-5 evaluator does not know about. Replay already refuses a `Projected` counterexample before it evaluates anything (`HyperProjected`), so this refusal is reached only when a caller hands an HP-6 clause to the lockstep reader. The mirror `TemporalTraceRefusal::AlignSkipNotImplemented` (qsl-replay/src/spine/clause.rs:471-478) and the test name `a_clause_with_align_skip_is_refused_as_not_yet_implemented` (hyper_tuple.rs:473) are stale in the same way. Name the condition, for example `AlignSkipNotLockstep`, 'a clause with `align skip` is read over projected prefixes, not in lockstep', under an input-shape code. Pre-release, so no alias is needed. | qsl-eval/src/temporal/outcome.rs:88-92, 137-139 |
| FND-006 | low | `ModelSystem::operation_text_of` is new `pub` API whose only caller is `ProjectedProduct::new` in the same crate, and it has no test of its own. Its `None` branch (an operation of another alias's model) decides which steps of a multi-alias clause are skipped, and no test has two aliases. Rust-review: `pub` only for what consumers use. Make it `pub(crate)`, or add a test of the other-alias `None` case (mutant P27, which drops the declaring-type check, survives). | qsl-eval/src/simulation/model.rs:663-677 |
| FND-007 | low | One rule is in two places. `decide_projected` copies `decide`'s FR-126 instance-ranking loop and its `FirstHit` to `HyperOutcome` mapping (hyper.rs:596-634) line for line. It differs only in the horizon cap and in the missing second phase. The rule for ranking instances (cap at the best depth, stop after depth 0, strictly shallower wins) now has two copies to keep equal, and HP-3's `decide_witness` has a third. Factor the loop into one `first_phase_over(instances, horizon)` that returns the best hit and `bounded`. `decide` then adds its second phase and `decide_projected` its `BoundReached`. | qsl-analyze/src/model_check/hyper/projected.rs:26-78 |
