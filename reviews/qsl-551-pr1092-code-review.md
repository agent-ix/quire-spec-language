---
id: SR-13450
title: "Code review of quire-spec-language PR #1092: QSL-551 E14, HP-2 self-composition product and EN-1 hyper run (FR-176, FR-183)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@41db278b3; PR #1092 (git diff origin/main...41db278b3): qsl-eval/src/hyper/, qsl-eval/src/automaton/translate.rs, qsl-analyze/src/model_check/{hyper,fair_walk,liveness,mod,check}.rs, qsl-replay/src/model_check/hyper_*.rs, qsl-replay settle/outcome/proof_result, qsl-semantics/src/check/state_clause.rs, qsl-analyze/tests/it/hyper*.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: reviews
---
# SR-13450: Code review of quire-spec-language PR #1092

## Summary

Ticket: QSL-551 (E14, part 2). This is the code review, with the rust-review
lane folded in. The PR adds the HP-2 self-composition product `HyperProduct` (qsl-eval),
`check_hyper` (qsl-analyze), the shared lasso walk `fair_walk`, and
`HyperCounterexample` with `replay_hyper_trace_tuple` (qsl-replay).

Gates run at 41db278b3 in a fresh clone with my own `CARGO_TARGET_DIR`. I did not run
`make ci`. Every gate passed. In total, 4,539 tests passed and 0 failed.

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`
- root `cargo test --test it`
- `make string-edge`
- `make arch-lint-api-surface-qsl`
- `make arch-lint-canonical-encoder`
- `make arch-lint-qualified-core`

I wrote a brute-force oracle in a separate scratch copy, never pushed. It is
`qsl-analyze/tests/it/hyper_oracle_probe.rs`, built on the PR's vault world
with a custom-contract variant.

- **What the oracle does.** It explores the vault's own `ModelSystem` graph,
  builds the matched pair graph by hand, and decides G, F, GF and FG bodies
  under per-variable weak fairness. It uses SCC fairness on the maximal
  components, with no automaton. It also checks vacuity, replays every
  `Violated`, and runs each case twice.
- **Without the fix: 588 of 700 random cases agree.** The other 112 are FND-001.
- **With a one-line probe fix for FND-001: 700 of 700 agree** on Holds, Violated and VacuousMatch.
  Every counterexample replays `Reproduced`. The runs are deterministic.
- **What this shows.** The product, the `μ_U` filter, terminal stutter, the
  per-component fairness scoping (HC-4), the lasso split, vacuity (HM-7) and
  determinism are sound on these models. ADR-023 §8.1 verdicts reproduce:
  leaky is refuted with the §8.1 counterexample, and secure holds over 8
  product states.

What I checked by reading or diffing:

- **Moved code is unchanged.** fair_walk.rs against the removed liveness.rs lines: `tree_path` gained graph generics, the moved methods sit in an `impl FairModel` block, and one signature was reflowed. No logic changed.
- **The seams are not a compatibility layer.** `HyperLetters` and `StepMatch` follow `ModelLetters`, whose implementors are also all in tests. They exist because S3 does not lower hyper atoms. Production has no implementor, so FR-176's "through the one clause evaluator (FR-107)" is not met yet. The PR says so.
- **No dead code**, and no added `unwrap`, `expect`, `panic` or `unsafe` outside tests. Indexing in hyper_replay.rs is guarded by `shape()`.
- **Comments follow the house idiom**: requirement ids and rationale, no build log.

Spec follow-ups (re-measured; spec questions, not code bugs):

- Leaky product 10 vs 14 states (FR-176-AC-1, ADR-023 §8.1). The test asserts 10 and 2 accepting SCCs. By hand with the shipped tableau, which guards the `q0` stay on the letter: 2 x 2 equal-secret states, plus 3 per unequal pair ((0,0) q0, (0,1) q1, (1,0) q1), gives 10. The ADR's 14 assumes a `q0` that also stays on a differing letter. The two automata accept the same language, and ADR-023 says its §8 counts are hand enumeration. The AC needs restating; the PR rightly leaves it unclaimed.
- FR-176-AC-2's spelling `a.step.op = V::Vault::step`. Reproduced: S3 refuses it as `Hyper(MatchReadsState)`, from qsl-semantics/src/check/hyper_clause.rs:1470 on main. FR-172 types `t.step.op` but names no operation-constant operand. Either FR-172 adds one or the AC is respelled. This is not this PR's code.
- Vacuity across instances. HM-7 is per clause, and FR-176 does not say how vacuity reads when one instance is vacuous and another is not. The PR rules vacuity item-wide: VacuousMatch only when no instance has a matched fair cycle. That is the permissive reading, since a vacuous instance can sit under `Holds`. A spec ruling is needed.
- `max_transitions` counts joint steps (FR-176, FR-184). The engine counts retained product edges, so one joint step with two automaton successors counts twice. FR-126 charges each followed product edge the same way, so the engine is consistent with the liveness phase. FR-184's wording should say which it means.

## Verdict

**Not mergeable: one high finding.**

- **FND-001 (high).** Bodies whose negation has no acceptance set end in an
  internal fault, not a verdict. The fix is small: treat `Dead` as
  non-accepting when the automaton has no acceptance set. The fault refuses
  rather than misleads, because the evaluator cross-check catches it. Still,
  FR-176 admits these bodies.
- **FND-002 (medium).** Instance ranking does not apply FR-126's
  depth-`d - 1` cap.
- **FND-003 and FND-004 (low).** Small ordering and spec-shape divergences.

Everything else holds under the oracle. The coverage and trace findings are in
SR-13451, the gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | When the negated body's automaton has no acceptance set (a body whose negation is a safety formula, e.g. `eventually p` or a bare `holds(p)`), the second phase accepts SCCs of `Property::Dead` states (automaton has no run), so `false_position` faults `the-evaluator-confirms-the-automaton` and `check_hyper` returns `Err(Fault)` instead of a verdict. Measured with a reviewer probe in a scratch copy (never pushed). Minimal repro: the secure vault from (0, 0), `forall trace a of V { eventually holds(v.l @ a = 0) }` (true at position 0) returns `Err(Fault(InternalFault{model-check, the-evaluator-confirms-the-automaton}))`; so does the body `holds(v.l @ a = 0)`. A brute-force self-composition oracle over 700 random vault models (6 post contracts, 3 preconditions, optional reset, random initial sets, 1 or 2 variables, 5 matches, per-variable weak fairness, bodies G/F/GF/FG over 3 atom kinds) hit this fault on 112 runs, every one an `eventually p` body; 69 of the 112 the oracle says Holds, the rest Violated. Cause: `accepting` adds only the automaton's own acceptance sets; with none, `analyze` passes any fair SCC, including the absorbing `Dead` part the product keeps for UE-1. With a probe patch that adds the non-Dead states as one acceptance set when the automaton has none, all 700 runs agree with the oracle, every Violated replays `Reproduced`, and each run repeats exactly. FR-176 says HP-2 admits any infinite-trace body (ADR-023 HP-2, HC-4). | qsl-analyze/src/model_check/hyper.rs:1077-1100, 1236; qsl-eval/src/hyper/product.rs:449-469 |
| FND-002 | medium | Instances are not ranked by FR-126's rule: after an instance's first phase finds a hit at depth d, `decide` still explores every later instance in full (no d-1 cap), so a later instance that hits `max_states`/`max_transitions`/meter beyond depth d-1 returns `Stopped` instead of the finding, and spends the shared budget on states FR-126 never explores. The module doc (hyper.rs:19-21) says instances are ranked as FR-126 ranks bindings, and FR-176 reuses FR-126's first phase. FR-126 'Bindings and ranking' caps each later binding at depth d-1 and returns the Stopped of a later binding only when it stops before completing depth d-1. `explore` runs `retain` with no horizon and `decide` returns any `Err(outcome)` from a later instance immediately. Traced by reading; not measured, because the vault fixture has one key, and no test has more than one instance. | qsl-analyze/src/model_check/hyper.rs:401-426, 589-682 |
| FND-003 | low | Within one instance the first phase takes the first state in discovery order, so an undefined `μ_U` on a joint step out of a state at depth d (reported at joint position d+1) wins over an undefined atom at a later-discovered state at depth d (position d); across instances the same pair is ranked by position, the other way. `first_phase` returns at the first state whose own letter or findings carry a hit, and records `depth + 1` for a `Match` hit; `decide` compares those depths across instances. ADR-023 HV-8 asks for the first refuting evidence in canonical order. The two orders disagree when one instance holds both kinds at the same BFS depth. Judgement call on HV-8's reading; confidence low. | qsl-analyze/src/model_check/hyper.rs:703-769 |
| FND-004 | low | The engine returns a new `HyperOutcome` enum and replays through a new public `replay_hyper_trace_tuple`; FR-176 Outputs specifies FR-126's `ModelCheckOutcome` with `Violated` carrying a `HyperCounterexample`, and FR-183 names only the facade `replay_model_trace_tuple`. The parallel outcome type duplicates `ModelCheckOutcome`'s four arms and will need its own settlement map (FR-182 is not built here), and the core replay is a published `qsl-replay` entry point the spec does not name. Either the spec gets amended to name these or the types fold into the specified ones when FR-182 and the facade land; record which in the ticket so the interim surface is not kept by accident. | qsl-replay/src/model_check/hyper_outcome.rs:16-47; qsl-replay/src/lib.rs:113-117; qsl-replay/src/model_check/hyper_replay.rs:92 |
