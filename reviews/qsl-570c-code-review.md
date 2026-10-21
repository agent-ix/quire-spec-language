---
id: SR-12267
title: "Code review (Rust lane) of QSL-570 part 2b: sampled-witness replay, symmetry orbits, quantile estimate, undecided successor (PR #1154)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bf9ea7519549139b2985fd03827c6d702aad6779; qsl-replay/src/{lib.rs,proof_result.rs,statistical.rs,statistical/settle.rs,statistical/witness.rs}, qsl-analyze/src/{statistical.rs,statistical/model.rs,statistical/tests.rs,model_check/check.rs,model_check/mod.rs,model_check/reduce.rs}, qsl-analyze/tests/it/{main.rs,prob_world.rs,reduction.rs,statistical_model.rs,statistical_witness_replay.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
---
# SR-12267: Code review (Rust lane) of QSL-570 part 2b: sampled-witness replay, symmetry orbits, quantile estimate, undecided successor (PR #1154)

## Summary

Ticket: QSL-570. PR quire-spec-language#1154, branch qsl-570c-impl, head bf9ea7519. The review covers `git diff origin/main...HEAD` only: 16 files, +1287/-112, one commit. The rust-review lane is folded into this file. Spec read at the PR head: FR-189 (Tests, Per sample, Decision and stops, AC-3, AC-5), FR-192 (estimate, the map), FR-193 (all), ADR-021 SYM-1 to SYM-8, ADR-024 ST-3 and RX-1, ADR-011 layering.

Checks run: `cargo test -p qsl-analyze --test it statistical` through `locked-build.sh` passed (23 passed, 0 failed). `make arch-lint-api-surface-qsl` passed, FR-100-AC-8 included. No other gate was run. The full gate belongs to the lead.

## Verdict

**MERGEABLE once the findings below are fixed in this PR. There are no majors.**

What is sound:
- **Layering (ADR-011).** `qsl-replay` still has no `qsl-analyze` dependency (Cargo.toml is not touched). The `WitnessReader` trait lives in `qsl-replay`. `qsl-analyze` implements it as `ModelWitnessReader`, which is the same seam as `ModelLetters` for FR-128. `SymmetryRefusal` moves to `qsl_replay::proof_result` and is re-exported, so `StatisticalRefusal::Symmetry` can carry it. The model checker now imports it from there. No downstream repo (quire-driver, quire-integration, quire-contract-codegen) names the old path. arch-lint FR-100-AC-8 passes.
- **Replay (FR-193 "Replay").** The draw checks run first in `qsl_eval::simulation::replay_sampled`: each value is in its support, each choice is recomputed from (seed, trace, step, choice index), and the path probability is recomputed exactly. A value outside the support is a refusal whose `catalog_code` is `invalid_runtime_input`/`invalid-value`. A changed seed or path probability gives `Inconclusive(SampledParity::Draw)`. The reader then draws the same (seed, trace) again and reads the claim with the same `violating` that `check_sampled` uses to keep witnesses. Its length and value must equal the record's. The draw check has already matched every recorded step, so equal length means equal steps. The verdict is never mutated (asserted).
- **Undefined witness (FR-193-AC-4).** This is a structural compare of `UndefinedEvaluation`, so a changed cause gives `Inconclusive(Value)`. A long-run walk is bounded by `steps + 1` iterations and replays to its undefined position.
- **Conversion (FR-193 "The qualitative claim").** The initial state and binding come from the provenance's test. The steps and post digests are `map_while` up to the first stutter. `Undefined` maps to `CounterexampleKind::UndefinedEvaluation`, and every other value maps to `Formula`.
- **Symmetry orbit soundness (the brief's question: can a WRONG merge happen?). No.** Two pairs merge only when (a) their initial states have byte-equal canonical state keys (`state_key` is the JCS bytes, not a digest), and (b) their bindings, carried by each state's canonicalising permutation `π`, are equal or are exchanged by a transposition that fixes the canonical state. `Permutation::image` and `permute` agree in direction (`from -> to`, and `canonicalise` returns `π` with `π(s) = c`). Both relations are true orbit relations under `G`, whose class groups are full symmetric groups, so they contain the transposition. "Exchangeable by a transposition fixing `c`" is an equivalence: `(a c) = (a b)(b c)(a b)`. So `block_key`'s first exchangeable member in key order is a well-defined block representative. A stabiliser element that is not a transposition (a 3-cycle, or `(a b)(c d)`) is missed. A miss only splits an orbit. More tests means a smaller `α' = α/m`, so Bonferroni stays valid. With exact orbits, every member's `Pr` equals the representative's (RX-1), so one test per orbit at `α/m_orbits` keeps family-wise error within `α`. Claim atoms are identity-transparent by SYM-8's authoring-time refusal, which is the premise RX-1 relies on.
- **Quantile censoring (FR-192 "estimate").** `q` is the reduced bound's threshold, which `reduce_quantile` sets to the level in both directions. The rank is `q·n` as a rational. The first value whose cumulative count reaches `seen >= q·n` is the `⌈q·n⌉`-th smallest, because `seen` is an integer. Censored samples count in `n` and sort above every value, so a rank past the finite values gives `Censored`. `n = 0` gives `None`. The logic is right. Its censored branch is untested (SR-12268 FND-002).
- **Panics, errors, wire.** No `unwrap`, `expect`, `unsafe` or indexing in production code. `u64::try_from(..).unwrap_or(u64::MAX)` is total. RefCell borrows are statement-scoped. The new types derive no serde. `TestResult.quantile` and `StatisticalVerdict.quantile` are in-memory fields only, so there is no wire stability concern. The new `UnsettledRefusal::Symmetry` is matched in `settle_statistical_refusal` with no `_` arm.
- **Declared blocked items, verified.** STD-250: `qsl-package/src/emit.rs:1247` refuses every probabilistic package (`ProbabilisticModelNotEmitted`, because FR-322 v2 has no node), and `replay_model_trace` takes a `ReplayRequestWire` that it recompiles, so the FR-098 recompile and the FR-128 "settles `refuted`" are genuinely blocked. STD-251: `qsl_semantics::check::Capability` is the ten-member FR-290 vocabulary, with no `probabilistic-satisfaction` and no evidence kind. That is genuinely blocked across repos. STD-252: the quantile field is unnamed in the spec, and the FR-189-AC-3 workload clause cannot be built under per-operation weights. Both are true. But the replacement STD-252 proposes cannot be built either (FND-004).
- **Tag honesty.** The doc comments state what each test covers ("FR-193-AC-3's conversion", "FR-189-AC-3 ... the symmetry clause"). The PR body lists the remainder. Partial-coverage tags are covered in SR-12268 FND-004.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `SampledWitnessRefusal::catalog_code` returns `None` for `UnknownTest` and `UnknownInitial`, and its doc says "a refusal whose code is its own", but those variants have no code of their own. FR-193 Outputs require a `ReplayRefusal` "with FR-128's catalog codes", so a witness naming a test or initial state the provenance or subject lacks is an invalid runtime input with no catalog mapping. A caller cannot settle it. Map both to `invalid_runtime_input`/`invalid-value`. `Read(WitnessReadStop)` should delegate, for example a `Sample(TotalAboveDrawRange)` to `invalid-value` as FR-193 states, with its own code otherwise. Add a replay test for an unknown test index. The only `UnknownTest` test today goes through `counterexample`. | qsl-replay/src/statistical/witness.rs:147-157 |
| FND-002 | low | The statistical path calls `admit` (SYM-2) but not `closure` (SYM-4). FR-189 says "under an admitted symmetry declaration (ADR-021 SYM-5)", and SYM-5's admission requires SYM-4. Orbit merging is still sound without closure, because only pairs that exist are merged. But a declaration that the model checker would settle `SymmetryBroken` is silently applied by EN-4. Either call `closure` and decide what EN-4 does with a broken declaration, or amend FR-189 (via STD-252) to say EN-4 needs only SYM-2. | qsl-analyze/src/statistical/model.rs:129-136 |
| FND-003 | low | `Tally::measured` fills the per-value `BTreeMap` for every counted `Sample::Measured` of every form, for example `probability >= θ [accumulate ... <= c]`, but only a quantile claim reads it. For a non-quantile measure claim that is an unbounded map that is never read. Gate `measured` on `q.is_some()`, or pass the form into the tally. | qsl-analyze/src/statistical.rs:309-318, 590-595 |
| FND-004 | low | `invariant()`'s reference branch cannot be reached. A support value must check as `NodeKind::Literal` or an integer `Coerce` (`probabilistic/model.rs` `literal`), and lowering refuses a `Value::Reference` literal (`UnbuiltLiteral`), so no support holds a reference and `invariant` always returns true. It also tests only a top-level `Value::Reference`, not a reference nested in an `Option` or `Composite`. STD-252's proposed AC replacement ("a random parameter of reference type with unequal weights") therefore cannot be built either. Correct STD-252 to say that RX-1's distribution condition cannot currently be distinguished, and keep or delete the branch by that ruling. | qsl-analyze/src/statistical/model.rs:211-226; qsl-semantics/src/check/probabilistic/model.rs:658-670; qsl-semantics/src/check/lowering.rs:4446-4453 |
| FND-005 | low | FR-192's map is stated as exhaustive by row, and it has no row for the new `StatisticalRefusal::Symmetry`, which settles no record (`UnsettledRefusal::Symmetry`). This is the same gap SR-12245 FND-008(a) closed for `Admission` and `RequiresBound`. Add the row to STD-252 (or to the FR-192 amendment), so the spec states that a malformed symmetry declaration is the caller's `invalid_runtime_input` refusal. | qsl-replay/src/statistical/settle.rs:133-139, 232 |


## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The total `catalog_code` that fixes FND-001 sends `SampleStop::Cancelled`, `GeneratorMismatch` and `NotMarkov` to `runtime_invariant`/`established-invariant-broken`, which is an internal-failure code. A caller who cancels a replay, or a witness whose provenance names another sampler, is reported as a tool bug. The repo has `cancelled`/`caller-cancelled` (`qsl-eval/src/simulation/explore.rs:237`). FR-192 settles a generator mismatch as `unsupported_projection`/`unsupported-requested-capability` and `NotMarkov` as unsupported. `AutomatonStates` maps to `resource_exhausted`/`insufficient-next-charge`, the meter's cause, where FR-126 names the `max_automaton_states` limit. Map each stop to its own code and keep `runtime_invariant` for true invariant breaks. Add one assertion (a cancelled replay). | qsl-replay/src/statistical/witness.rs:160-185 |

## Dispositions

Round 1, by the reviewer, at 99cab573f (fix commit f4025397a). This replaces the rows the coder added in 99cab573f: each row below was checked against the fix commit.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f4025397a. `catalog_code` is total. `UnknownTest` and `UnknownInitial` give `invalid_runtime_input`/`invalid-value`, and `a_witness_the_subject_does_not_place_is_refused_with_a_catalog_code` asserts both. The mapping of the other stops is FND-006. |
| FND-002 | fixed | f4025397a. FR-189 "Symmetry" now says EN-4 admits the declaration's SYM-2 form and does not check SYM-4's closure, because it merges only the pairs it was given. `StatisticalRequest.symmetry` docs match. |
| FND-003 | fixed | f4025397a. `tally.measured` now runs only for `ClaimForm::Quantile`. |
| FND-004 | fixed | f4025397a. `invariant()` was deleted, and the doc says invariance always holds today (per-operation weights, no reference literal in a support). The STD-252 correction comment is posted. |
| FND-005 | fixed | f4025397a. FR-192 has a `Symmetry` refusal row (the caller's `invalid_runtime_input`/`invalid-value`, no terminal result). FR-189 lists `SymmetryRefusal`. |
