---
id: SR-13374
title: "Code review of quire-spec-language PR #1073: strong fairness by SCC refinement in EN-1 and strong-constraint replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0f8a69083; git diff refs/remotes/origin/main...HEAD: qsl-analyze/src/model_check/liveness.rs, qsl-replay/src/model_check/replay.rs, qsl-replay/src/model_check/settle.rs, qsl-analyze/tests/it/{strong_fairness,strong_hint,lasso_replay,liveness,quotient,main}.rs, spec/functional/FR-126, spec/test-cases/TC-1085; read against FR-126, FR-127 (V-6), FR-128, FR-130, FR-131, FR-133, ADR-019 FS-2 to FS-8 and SV-4, TC-531, TC-532, TC-534, TC-1085; kernel read: qsl-eval/src/fairness/{refine,lasso,constraint}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-130
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-131
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1085
    type: reviews
---
# SR-13374: Code review of quire-spec-language PR #1073

## Summary

Ticket: QSL-532 (E10a). PR #1073, branch `qsl-532-engine`, head 0f8a69083.
Scope is `git diff refs/remotes/origin/main...HEAD`, 11 files. The
`rust-review` lane is folded into this file.

What the PR does. EN-1 now builds weak and strong kernel constraints. The
existing `qsl_eval::fairness::analyze` refinement (from #953) therefore runs
on strong constraints. The CX-5 greedy walk owes a strong constraint an edge
only when the passing component enables it somewhere. No state ever
discharges a strong constraint. Replay's `unfair_constraint` checks strong
constraints by SF-4, over the enabled sets it recomputes from `ModelSystem`.
The settlement map turns `Err(UnfairLasso)` into
`Inconclusive(ReplayParity(NoValue))`. The PR deletes
`LivenessRefusal::StrongFairness` and `ModelTraceRefusal::StrongFairness`.
It also removes the refusal from FR-126, FR-126-AC-13 and TC-1085.

Soundness, checked by hand. If a component meets (a) and (b) but fails
(c), it is refined. The refinement removes every state where a failing
strong constraint is enabled. The constraint is taken by no edge of `S`.
So any cycle in `S` that visits such a state infinitely often is strongly
unfair, and every fair cycle lies inside `S \ R`. Refinement therefore
loses no fair cycle. `UnfairWeak` is not refined. That is correct: a weak
constraint enabled everywhere in `S` and taken nowhere in it is enabled
everywhere in every sub-component, and taken nowhere there either. A `Fair`
component holds a cycle through all its vertices and edges, and that cycle
meets every obligation. In the greedy walk, each strong constraint enabled
somewhere in `P` is taken by some edge of `P`, so the walk can always
discharge it. A strong constraint enabled nowhere in `P` is enabled at no
loop state. So every loop the walk returns is fair. I found no unsound
shortcut, neither a false `Holds` nor a false lasso.

Measured, with scratch tests in a separate clone (not committed):

- **Kernel and greedy-walk oracle.** The test is
  `scratch_oracle` in `qsl-analyze/src/model_check/liveness.rs` of
  /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1073/oracle.
  It ran 60,000 random graphs: 1 to 6 vertices, 1 to 3 identities, random
  enabled sets, edges labelled only by identities enabled at their source,
  terminal stutters, 0 to 2 acceptance sets, and 1 to 3 constraints mixing
  weak and strong, whole and each. A brute-force oracle enumerates every
  vertex subset. It accepts a subset that is strongly connected over its
  induced edges, meets every acceptance set, and meets every constraint
  (weak: taken or disabled somewhere; strong: taken or enabled nowhere).
  The oracle and `first_fair_in(analyze(..))` agreed on all 60,000 graphs:
  33,474 fair, 188 of them found only by refinement, 26,526 not fair. Two
  runs gave equal analyses. On every fair graph, `FairModel::greedy_loop`
  returned a closed walk from the entry. The walk stayed inside the
  component, used only graph edges, and visited every acceptance set.
  `check_loop` judged it `Fair`. Mutation check: restoring the old
  `disabled_at`, which lets a disabling state discharge a strong
  constraint, fails the oracle at trial 155. So the test has teeth.
  Log: /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1073/oracle-kernel.log.
- **Engine oracle on mutex worlds.** The test is
  `qsl-analyze/tests/it/scratch_oracle.rs` in the same clone. It covers
  2-process and 3-process mutexes and the 3-process `Handoff`. Each of
  `setTo`, `inc` and `pass` gets one of: none, weak whole, weak each,
  strong whole, strong each, in every combination. Two mixed sets are
  added (weak each plus strong each on `setTo`; strong whole `setTo` plus
  strong each `inc` plus weak whole `setTo`). The claims are `always
  eventually holds(value in K)` for six target sets. The oracle enumerates
  every strongly connected set of model states outside `K`, using the
  model's own enabled sets. Over 1,032 engine runs, `Violated` versus
  `ComponentCertificateNotBuilt` matched the oracle every time. All 644
  refutations replayed through `replay_with`, which means strong replay
  found every one fair. A second decide gave the same `Debug` output.
  Log: /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1073/oracle-engine.log.

Gates, run at 0f8a69083 with their own target dir,
`CARGO_HOME=$HOME/.cargo-draco-lane2`, `CARGO_INCREMENTAL=0`. All exited 0:

- `cargo fmt --all -- --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `make string-edge`
- `cargo test`: qsl-eval 681 passed, qsl-replay 783, qsl-analyze 222,
  qsl-semantics 1163, root `--test it` 975; 0 failed
- `make arch-lint-canonical-encoder`, `make arch-lint-api-surface-qsl`
  (this includes FR-100-AC-8 "qsl_replay::spine names no qsl_eval path":
  PASS), and `make arch-lint-qualified-core`

`make ci` was not run.

Settlement mapping. FR-131 says an EN-1 `Violated` refused with
`UnfairLasso` settles `inconclusive`, `ReplayParity` (SV-4). FR-127's V-6
row says the same. The arm in `settle.rs` does this, and FR-131-AC-3's test
asserts it. Neither spec names the `DisagreementCause` inside
`ReplayParity`. The PR picks `NoValue{proved: Violation, replayed:
Incomplete}`. Nothing in the spec rules that out, so it is allowed. The arm
covers weak unfair lassos too, as FR-127-AC-3 asks; see the gap analysis
for that clause's missing test.

Spec deletions. FR-130 decides strong constraints, so removing the
`StrongFairness` refusal from FR-126's "Unsupported liveness clauses",
FR-126-AC-13 and TC-1085's steps and results is legitimate. One leftover
sentence remains (FND-002).

Compatibility and dead code. Both refusal variants are deleted with their
`code()`/`cause()` arms, and so is the interim test. No shim or re-export
is left, and no file outside the diff names either variant. The only
remaining mentions are in committed `reviews/` history.

## Verdict

Mergeable once FND-001 and FND-002 are fixed. Both are small:
one tag and one sentence. No high finding.

The engine and replay changes are correct and minimal. They reuse the
existing kernels and add no new algorithm. Both brute-force oracles agree
with the engine, and determinism holds. Cancellation goes through
`analyze`'s charges to `Stopped{Cancelled, None}`. The deleted refusals
leave no trace.

The new test names and doc comments follow repo idiom. `#[trace]` tags
use the `("TC-nnn", "FR-x-AC-n")` form that TC-531, TC-532 and TC-534 ask
for. Bindings checked:

- correct: FR-130-AC-1, AC-2, AC-3, FR-131-AC-1, AC-2, AC-3
- partial: FR-133-AC-1 (FND-007)
- wrong: FR-130-AC-5 (FND-001)

FR-130-AC-1 and AC-3 assert `Holds` through the
`ComponentCertificateNotBuilt` statistics, which FR-126 makes the
certificate-less form of a proof. I accept that and record it in the gap
analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `two_strong_constraints_failing_on_one_component_refine_together` is tagged `FR-130-AC-5`, but it asserts nothing AC-5 states. It checks only that the three-process `strong each` claim is a proof (`ComponentCertificateNotBuilt`, `product_states > 0`) and that the `weak each` claim is refuted. AC-5's content is unbuilt: one `UnfairStrong` entry listing both `acq(1)` and `acq(2)`, `sub` `{(3, q1)}` `Trivial`, acceptance by `check_components`, and `WitnessFails` for the entry listing `acq(1)` and `acq(3)`. The test also cannot fail if the engine refined the two constraints one at a time, so even the title's "refine together" goes unchecked. The matrix will show AC-5 covered while all of it is open. The PR body calls this the "engine half", but a tag cannot say "half". Fix: keep only `TC-531` on this test (or tag it to FR-130 "The filter" in its doc comment) until the certificate lands. | qsl-analyze/tests/it/strong_fairness.rs:247-265 |
| FND-002 | medium | TC-1085's Description still says the TC verifies "that the strong-fairness, guarded-activation and unbuilt-certificate cases refuse". The PR removed the strong-fairness refusal from TC-1085's steps and results, and from FR-126 and FR-126-AC-13, because FR-130 decides strong constraints. So TC-1085 now contradicts itself and FR-130. This is the one remaining spec text that says a strong constraint is refused. Fix: drop "strong-fairness," from the sentence. | spec/test-cases/TC-1085-liveness-model-only-pass-and-unsupported-refusals.md:13-16 |
| FND-003 | low | `second_phase`'s comment says a reduced run goes to the annotated quotient even under a trivial stabiliser because "only the quotient decides strong fairness, which the request admits under symmetry". After this PR the unreduced `FairModel` decides strong fairness too, so the stated reason is false. The routing itself is still fine. Fix: give the reason that is still true (a reduced run is read as its quotient, which `HoldsReduced` and concretisation need), or delete the comment. | qsl-analyze/src/model_check/liveness.rs:1679-1681 |
| FND-004 | low | `cancellation_mixed_kinds_and_determinism` departs from TC-531 step 3 in two ways. (1) It cancels at the run's last charge, not "when the second phase first checks it". The last charge falls inside `analyze`, so the test still proves that the second phase stops on `Cancel`. It does not exercise the check before the first SCC decomposition. (2) The determinism loop repeats only the `weak each` mutex and `Handoff`, while step 3 says "the requests of steps 1 and 2 twice each". The `strong` (whole) refutation and the `strong each` proof are never repeated. Fix: add `STRONG_WHOLE`, and the `strong each` proof statistics, to the loop. Either cancel at the first second-phase charge or say why the last one is used. | qsl-analyze/tests/it/strong_fairness.rs:173-245 |
| FND-005 | low | FR-133's suppression rule ("an operation the clause already constrains that way gets none"), coded as `if !clause.fairness().contains(&suggested)`, can no longer be reached. A `Whole` hint is computed only when the operation is enabled at a loop state and taken by no loop step. An `Each` hint is computed only when an identity is enabled at a loop state and taken by no loop step. If the clause held that same strong constraint, the strong check in `unfair_constraint`, which this PR adds, would refuse the lasso `UnfairLasso` before `strong_hints` runs. That holds for any lasso, not only EN-1's. The new `strong_hint.rs` header gives the wrong reason the case is uncovered ("EN-1 builds only lassos that take it"). Fix: correct the header, and raise with the spec owner whether FR-133 should drop the suppression bullet, or keep it with a note that the strong replay check makes it unreachable. | qsl-replay/src/model_check/replay.rs:934, qsl-analyze/tests/it/strong_hint.rs:8-11 |
| FND-006 | low | Small comment and idiom issues. (1) The new `quotient.rs` module doc line is about 150 columns wide and reads as history ("decided unreduced since FR-130, but AC-4 stays untagged"), with no reason given for leaving AC-4 untagged. (2) `decided`'s doc says "with a refinement-free outcome", but it is also used for the `Handoff` run, whose outcome comes from a refinement. (3) The `FairnessKind` to `Strength` mapping is now written twice, as `strength_of` in liveness.rs and as an inline `match` in replay.rs. Both crates and qsl-eval already depend on `quire-semantic-value`, so one `impl From<FairnessKind> for Strength` in `qsl_eval::fairness` would serve both. | qsl-analyze/tests/it/quotient.rs:14-18, qsl-analyze/tests/it/strong_fairness.rs:39-42, qsl-analyze/src/model_check/liveness.rs:267-273, qsl-replay/src/model_check/replay.rs:848-851 |
| FND-007 | low | `FR-133-AC-1` is now tagged on just one test, `the_strong_whole_mutex_refutation_carries_the_same_hint`, the `strong` (whole) half, so the matrix shows AC-1 as covered. But the test for its `weak each` half, `the_weak_each_mutex_refutation_names_acq_one_at_loop_position_zero`, still carries only `TC-534`. AC-1's last sentence ("Both records still settle `refuted`, with the same counterexample and obligation identity as when the hint is not computed") is asserted by neither test. Neither one settles the mutex refutations; both stop at `hints_of`. Fix: tag the weak test `FR-133-AC-1` too, and settle both refutations through `settled` (as the AC-2 test does), asserting `refuted` and the unchanged counterexample and obligation identity. | qsl-analyze/tests/it/strong_hint.rs:248-310 |
