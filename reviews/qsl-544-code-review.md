---
id: SR-13362
title: "Code review of quire-spec-language PR #1071: QSL-544 part 2, check_initial / check_step, the refinement safety product, settlement and prefix replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1c3b7fae6; PR #1071 (git diff refs/remotes/origin/main...HEAD, base 891825c45): qsl-replay/src/refinement/{map,step,failure,outcome,subject}.rs, qsl-replay/src/model_check/{refinement_replay,refinement_settle,replay,counterexample}.rs, qsl-replay/src/{lib,outcome,proof_result,refinement}.rs, qsl-eval/src/simulation/model.rs, qsl-eval/src/simulation/model/{refine,contracts,frames}.rs, qsl-eval/src/simulation/model/frames/judge.rs, qsl-analyze/src/model_check/{refine,check,liveness,mod}.rs, qsl-analyze/tests/it/{refinement_safety,refinement_replay}.rs, qsl-replay/tests/it/{refinement_mapping,refinement_ring,refinement_step,refinement_unmapped}.rs, tests/support/{refinement_cas,ring_world,ring_queue}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: reviews
---
# SR-13362: Code review of quire-spec-language PR #1071

## Summary

Ticket: QSL-544 (E13), part 2 of the slice. Code review with the rust-review
lane folded in. The review is against the merged spec (FR-140, FR-141,
FR-142, FR-144, FR-145, ADR-020 section 8, TC-545/546/547/549/550/555/1086)
and QSpec FR-375, FR-377 and FR-379 at quire-specification origin/main.
I did not rely on the PR body; each claim below was checked against the code.

What the PR builds, as verified:

- `map_state` (FR-140) checks both systems before it reads any row. It reads
  every row in population, object-key and field-name-byte order. It returns
  `Fault` at once, and ranks the first `Undefined` above the first
  `Undetermined`. It lifts top-level references and whole-population
  reference sets, and applies the `T?` / `Option<T>` rule. The deleted
  `MappingFeature`, `EvalFailure::Stopped` and `EvalFailure::Unsupported`
  have no remaining reference in code. No shim was left behind.
- `check_initial` / `check_step` (FR-141, model to model). `judge_application`
  reuses the expansion's own `conjunction`, `decide`, `effect_of` and
  `result_order`, and `post_conjunction` was factored out of `complete`, not
  copied. So a step judged here and an expansion successor go through the
  same code.
- `check_refinement` (FR-142) explores the product breadth-first in canonical
  order: initial states by key bytes, then edges by (transition key, post key).
  It checks every edge, including edges into a product state it already knows.
  It ends at the first `Fails` or `Undefined`, and the counterexample is the
  BFS tree path plus the failing edge, with position = depth + 1. Both systems
  come from `open_systems`, the same builder replay uses, with the derived
  universes.
- Replay (FR-145) re-executes the prefix with FR-128's `reexecute`, reruns the
  same two functions, and requires full equality with the recorded failure.
  That is stricter than "kind and position", which is the safe direction.
- Settlement (FR-144, safety): engine `Fault` gives `failed`. A replay
  `ReplayRefusal::Fault` gives `failed` (`Row::refused`). A replay
  disagreement gives `ReplayParity`. `Undecided` gives `MappingUndetermined`
  or `UndecidedSuccessor`.

Gates I ran myself, in my own clone and target dir
(`CARGO_HOME=$HOME/.cargo-draco-lane2`, `CARGO_INCREMENTAL=0`, not
`make ci`):

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `make string-edge`: pass.
- `cargo test -p qsl-eval`: 681 passed.
- `cargo test -p qsl-replay`: 809 passed.
- `cargo test -p qsl-analyze`: 214 passed.
- `cargo test -p qsl-semantics`: 1163 passed, 1 ignored.
- Root `cargo test --test it`: 975 passed, 6 ignored.
- `make arch-lint-api-surface-qsl`: pass, FR-100-AC-8 "qsl_replay::spine
  names no qsl_eval path" included.
- `make arch-lint-canonical-encoder`: pass.
- `make arch-lint-qualified-core`: pass.

I wrote one probe in my own clone and reverted it afterwards; the PR branch
is untouched. It calls `check_step` on RingIsQueue `take` from `head = 1`,
`size = 2`, with an abstract system whose meter admits no charge. It returns
`StepVerdict::Fault(InternalFault { stage: "refinement-step", invariant:
"abstract-judgement" })` (FND-001).

The `#[allow(clippy::result_large_err)]` and fixture `dead_code` /
`unused_imports` allows each carry a reason and follow the existing pattern.
Comment density and idiom match the crates. I found no dead or duplicate code
beyond FND-010. `ModelSystem::initial_states()` sits beside the
`TransitionSystem::initial()` that clones; it is a borrowing accessor, not a
duplicate.

## Scope examined

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-140 Behavior (read order, precedence, Fault) | examined | "`map_state` SHALL read the rows in a fixed order ... the populations in the checked node's order, within each population its objects in ascending key order, and per object its type's object row and then that type's field rows in ascending field name as UTF-8 bytes" |
| FR-140 Inputs (WrongSystem) | examined | "`map_state` SHALL check that `concrete_system` is over the refinement's concrete model and `abstract_system` over its abstract side, and SHALL return `MappingFailure::WrongSystem`" |
| FR-141 Inputs and Outputs | examined | "`check_initial(refinement, concrete_system, abstract_system, concrete_initial) -> StepVerdict`; `check_step(refinement, concrete_system, abstract_system, pre, step) -> StepVerdict`" |
| FR-141 Selecting the row | examined | "An argument expression of the row that evaluates `Undefined` at the pre-state SHALL return `Undefined`; one that is refused or incomplete SHALL return `Undetermined(MappingUndetermined)`." |
| FR-141 Deciding the step rules | examined | "A `MappingFailure::Fault` or `MappingFailure::WrongSystem` from FR-140 SHALL return `StepVerdict::Fault`." |
| FR-142 Product and Verdicts | examined | "The first edge, in canonical breadth-first order, whose `check_step` returns `Fails` SHALL end the phase with `Violated` ... `Stopped` naming the limit and its value for `max_states`, `max_transitions`, `max_candidates` or the evaluation meter" |
| FR-144 Settlement and Certificate | examined | "A safety half that the product proves SHALL settle `proved` only after FR-149's checker accepts its certificate" |
| FR-145 Prefix and Undefined mapping | examined | "For a safety failure it SHALL run `check_step` (or `check_initial` for `InitialNotAbstract`) at each position and stop at the first failure." |
| QSpec FR-377 Failure precedence | context_only | "an undefined row outranks every undetermined row, so the claim is refuted at that state even when an earlier row is refused, incomplete or unmapped" |
| QSpec FR-375 Checked-package node | context_only | "Every other array of the body (`populations`, ...) is sorted ascending by the node-key digest of its first named declaration, then by member name as UTF-8 bytes" |

## Verdict

**Not mergeable as is: one medium code defect (FND-001) to fix in this PR,
plus one wrong trace tag (FND-005).** The product exploration, prefix replay,
row read order inside a population, Fault and WrongSystem routing to
`failed`, and the deletions are sound. The other medium findings (FND-002,
FND-003, FND-004) are spec or owner decisions, not code to change blindly.

On the coder's three additions that no spec names:

- `check_initial`'s `index` and `ConcreteStep::position` are needed. The spec
  signatures carry no way to write `InitialNotAbstract{initial}` or a failure
  `position`. These conform to the spec's intent, but FR-141 Inputs needs a
  small amendment.
- `abstract_successors: &mut u64` meets FR-141's "SHALL count toward
  `max_transitions`" but is an out-channel the signature and the Purity bullet
  do not describe, and replay discards it (`&mut 0`). Either amend FR-141 to
  name the count, or carry it in the verdict (FND-007).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An abstract-side resource stop becomes an internal fault and settles the item `failed`. `check_step` turns every `ExpansionStop` from the abstract system into `StepVerdict::Fault`: `expand_state` gives `fault("abstract-expansion")`, `judge_application` gives `fault("abstract-judgement")`, and `resolve_step` gives `fault("concrete-step-resolves")`. `conjunction` returns `Err(exhausted(EvaluationMeter))` for an incomplete contract. A result domain over `RESULT_ORDER_LIMIT` stops with `EXHAUSTED`. So when an abstract precondition or postcondition exhausts the per-evaluation meter, FR-144 settles `failed`, which says QSL has a bug. FR-142 requires `Stopped{ResourceExhausted, EvaluationMeter}` (or `MaxCandidates`) there, and FR-141 allows `Fault` only for FR-140's `Fault`/`WrongSystem`. Reproduced: RingIsQueue `take` with a zero-meter abstract system returns `Fault(refinement-step/abstract-judgement)`. Production shares one meter between the two systems (`open_systems` copies `concrete.meter`), so a real budget hits this. `StepVerdict` also has no arm for a budget stop, so the fix needs one (spec plus code); the engine's `expansion_stop` already maps the concrete side correctly. | qsl-replay/src/refinement/step.rs:199-201,330-332,428-430; qsl-eval/src/simulation/model/frames/judge.rs:249-251; qsl-eval/src/simulation/model/contracts.rs:164 |
| FND-002 | medium | Neither FR-141 nor QSpec FR-377/FR-379 orders the outcomes of the step-row arguments, and the code picks an order that inverts the undefined-outranks rule. `check_step` evaluates the receiver, then each parameter, and returns on the first non-value. It does this before it maps the post-state. So an undetermined receiver hides an undefined parameter, and an undetermined argument hides an undefined post-state row: the step settles `inconclusive` where FR-377's precedence would refute it. The order is deterministic, so verdict and replay agree in QSL, but another conforming engine can report a different row or verdict. The author disclosed this ("the spec does not order them"). Needs a spec amendment that fixes the precedence across arguments and between arguments and the post-state map, with the code following it (FR-141-AC-7's zero-meter case pins only one point). | qsl-replay/src/refinement/step.rs:140-157,390-400 |
| FND-003 | medium | Populations are read in `DeclarationKey` byte order, not in the node's order. FR-140 says "the populations in the checked node's order", and QSpec FR-375 fixes that order as ascending node-key digest of the first named declaration. The two orders differ whenever a refinement has two populations, so with failures in two populations QSL reports a different row and object from a QSpec-conforming engine. The `RefinementFailure::Undefined{row}` and `MappingSite` then differ, and cross-engine replay settles `ReplayParity`. The author disclosed that the digest order "is not available". Either compute the digest order (the node key is in the checked graph) or amend FR-140 to name declaration-key order. | qsl-replay/src/refinement/map.rs:204-205 |
| FND-004 | medium | A safety half that holds settles `Proved{basis: Exhaustive, certification: Uncertified}`. FR-144 says "A safety half that the product proves SHALL settle `proved` only after FR-149's checker accepts its certificate". No spec states the interim value without FR-149. The PR body and the QSL-544 comment disclose the choice and defer it to QSL-549, but until QSL-549 lands the code emits a `proved` record that FR-144 forbids. Needs an owner ruling or FR-144 amendment: either allow `Proved{Exhaustive, Uncertified}` until FR-149, or settle another value. | qsl-replay/src/model_check/refinement_settle.rs:40-44 |
| FND-005 | medium | The test tagged `FR-144-AC-2` asserts the opposite of that AC. FR-144-AC-2 requires the safety half `Proved{basis: Exhaustive, certification: Certified}` and the item `Uncertified` with a liveness half. `a_safety_half_that_holds_settles_proved_uncertified` asserts the record is `Uncertified` with no liveness half and no item combination. The matrix shows FR-144-AC-2 as `tagged`, so it overstates coverage, and the test will have to flip when FR-149 lands. Drop the AC from the tag (keep `TC-549`) until QSL-549/QSL-547. | qsl-analyze/tests/it/refinement_replay.rs:255-273 |
| FND-006 | low | Several AC tags claim a whole AC where the test covers part of it, so the matrix reads them as covered. FR-140-AC-11: a unit test of `unwrap_optional` on values, not the AC's row. FR-145-AC-4: the test checks FR-128's initial-index, not-enabled and post-state refusals, none of which AC-4 lists; only its determinism clause matches. AC-4's node-identity, history-lasso and stale `package_id` cases are unbuilt. FR-144-AC-3: only the `only` variant; the `writes` and hidden-`ensure` cases are unbuilt. FR-145-AC-1: no history or Coin case. FR-145-AC-3: no lasso cases. FR-140-AC-14: no "even when an earlier row was undefined" case. The PR body states each gap, but the tags do not. Use TC-only tags, or split these ACs. | qsl-replay/src/refinement/map.rs:572; qsl-analyze/tests/it/refinement_replay.rs:117,161,194,221,277; qsl-replay/tests/it/refinement_mapping.rs:241 |
| FND-007 | low | The public shapes differ from the spec's Outputs without a spec amendment. `check_initial` takes `index`. `ConcreteStep` carries `position`. `check_step` takes `abstract_successors: &mut u64`, an out-channel the Purity bullet does not describe and replay passes as `&mut 0`. `MappingFailure::WrongSystem` holds a `CatalogCode`, not a `RefusalRecord`. `EvalFailure` is `Evaluation(Box<ClauseOutcome>)`, not `Refused`/`Incomplete`/`OutOfType`. `RefinementOutcome.safety` is a new `SafetyOutcome`, not FR-126's `ModelCheckOutcome`. The first two are needed and conform in intent. The rest need FR-140/141/142 amended to match, or the count carried in the verdict. | qsl-replay/src/refinement/step.rs:73-79,122-129; qsl-replay/src/refinement/map.rs:86-92,125; qsl-replay/src/refinement/outcome.rs:42-97 |
| FND-008 | low | `object_of` finds a step receiver's population by scanning every population for its key string. Keys are scoped to a population, so with two populations that share a key, `MappingSite.object` / `MappingUndetermined.object` can name the wrong population. The receiver reference's type, through `concrete_system`, identifies the population. | qsl-replay/src/refinement/step.rs:482-491 |
| FND-009 | low | `replay_refinement_trace` returns an internal fault, which settles `failed`, for a payload whose `refinement` member is `None`. That is malformed input to the replay facade, not a broken QSL invariant. It should refuse `invalid_runtime_input`/`invalid-value`, the way FR-145 treats other bad envelopes. | qsl-replay/src/model_check/refinement_replay.rs:81-83 |
| FND-010 | low | `subject_models` is re-exported at the `qsl_replay` crate root, but only `map.rs` and `subject.rs` inside the crate use it. That widens the public surface for no caller; make it `pub(crate)`. | qsl-replay/src/lib.rs:138; qsl-replay/src/refinement.rs:17 |
