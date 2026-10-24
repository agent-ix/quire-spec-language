---
id: SR-16625
title: "Code review of PR #1430: Projected (HP-6) arm of replay_hyper_trace_tuple, code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1ee181562530b8029b14bc6296071d9b670ee9c6; git diff origin/main...HEAD (PR #1430, branch qsl989-hyper-projected): qsl-replay/src/model_check/hyper_replay.rs, qsl-replay/src/model_check/replay.rs, qsl-analyze/tests/it/hyper_replay.rs, qsl-analyze/tests/it/hyper_projected.rs; ticket QSL-989"
review_set: subset
date: '2026-10-24'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: reviews
---
# SR-16625: Code review of PR #1430, Projected (HP-6) replay

## Summary

Ticket: QSL-989. PR: quire-spec-language#1430, head 1ee181562. Method: code-review with the rust-review lane. The PR adds the `Projected` arm (`projected`, `visible_steps`, `ProjectedLetters`) to `replay_hyper_trace_tuple`, factors the clause/instance shape checks into `about_clause` and `μ_U` into `universal_reading`, narrows `HyperProjected` to the `Undefined` counterexample of an `align skip` clause, and adds five tests (four TC-608 tests and the FR-183-AC-3 aggregate).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-183 "Projected (HP-6)" | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-183-AC-3 | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-183-AC-2 | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-183-AC-7 | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| ADR-023 PA-6, PA-7 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | context_only |
| FR-178 "Projection", "Three-valued prefix evaluation" | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | context_only |
| projected / visible_steps / ProjectedLetters | qsl-replay/src/model_check/hyper_replay.rs:938-1101 | examined |
| replay_hyper_trace_tuple dispatch | qsl-replay/src/model_check/hyper_replay.rs:129-167 | examined |
| HyperProjected / HyperShape | qsl-replay/src/model_check/replay.rs:210-222 | examined |
| new and changed tests | qsl-analyze/tests/it/hyper_replay.rs:980-1121, qsl-analyze/tests/it/hyper_projected.rs:416-434 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test fails without the mark-agreement rules. With both `visible_steps` mark arms neutralised (a skipped mark on an unlisted operation read as skipped, a visible mark on a listed one read as visible), all 139 `hyper_` tests pass. AC-3's Projected case, a `step` marked skipped, is refused by the visible-count and position rules instead. The tests match `HyperShape { .. }` without its reason, so they cannot tell which rule refused. FR-183's first Projected rule therefore has no test, and dropping it would let a payload hide a visible step by marking it skipped. | qsl-replay/src/model_check/hyper_replay.rs:1048-1061; qsl-analyze/tests/it/hyper_replay.rs:1039-1063 |
| FND-002 | low | The visible-count equality rule is masked by the `trace_position` rule. With the equality check removed, all `hyper_replay` tests still pass, because the shortened trace is `traces[0]`. That check is also what keeps `visible[component][joint - 1]` in bounds when `traces[0]` has more visible steps than another trace. | qsl-replay/src/model_check/hyper_replay.rs:954-956, 968; qsl-analyze/tests/it/hyper_replay.rs:1065-1068 |
| FND-003 | medium | A `Lockstep` or `WitnessExhausted` counterexample handed in with an `align skip` clause is still refused as `HyperProjected` (`unsupported_construct`/`not-yet-implemented`). The narrowed docs say that refusal is only for an `Undefined` counterexample of such a clause. A counterexample of the wrong kind for the clause is a shape error (`invalid_runtime_input`/`invalid-value`), which is how the new test treats the reverse case, a `Projected` counterexample of a lockstep clause. No test covers it. | qsl-replay/src/model_check/hyper_replay.rs:142-147; qsl-replay/src/model_check/replay.rs:210-214 |
| FND-004 | low | The narrowing is incomplete in two places. `HyperProjected`'s `#[error]` text still reads "replay of a counterexample of an `align skip` clause", which a caller now reads as covering the `Projected` ones that do replay. `HyperShape`'s doc still lists only the lockstep shape reasons, not the new Projected ones (marks, visible counts, stutter, position). | qsl-replay/src/model_check/replay.rs:213-217 |
| FND-005 | low | FR-183-AC-3 changes `b`'s first input to 1. The test the AC-3 aggregate now claims changes both of `b`'s inputs to 1, and its doc says "inputs", so the AC's exact case is never run. | qsl-analyze/tests/it/hyper_replay.rs:97-99, 113-124 |
| FND-006 | low | A reproduced Projected replay reports the payload's `trace_position` (L), not a position the replay derived. A hand-built payload whose projected prefix is already bad before L, with extra visible steps after it, reproduces at L. FR-183 only requires `false` to reproduce, but Lockstep reports the evaluator's `at`. | qsl-replay/src/model_check/hyper_replay.rs:1016-1020 |

## Verdict

Not mergeable as is. FND-001 and FND-003 are medium. The others are low and can be fixed in the same round. Clause-by-clause check against FR-183 "Projected (HP-6)" and ADR-023 PA-6 and PA-7:

- **Re-execution.** It runs through `execute` with FR-128's refusals. `reexecute` compares each payload transition to an enabled successor, so the labels the marks and `μ_U` read are verified.
- **Marks.** The skipped set is built per trace from that trace's subject (`operation_text_of`), the same way the engine's `ProjectedProduct` builds it (qsl-eval/src/hyper/projected.rs:103-112). Both mismatch directions are refused.
- **Visible counts.** They are checked equal.
- **`trace_position`.** It must equal L, which follows PA-6.
- **Stutter.** A stutter step is refused, which follows PA-4: the projected product has only skip and visible moves.
- **`μ_U`.** It is checked at every joint visible step, and a false reading is refused `HyperUnmatched`. A refused or incomplete reading completes no value.
- **Body.** `evaluate_projected_prefix` (FR-178) runs over L+1 projected positions. `ProjectedLetters` maps projected position p to the state the p-th visible step entered. `false` reproduces. `Incomplete` and `WindowLimitReached` complete no value. Anything else settles `Inconclusive(Verdicts)`, which follows FR-183. ADR-023 PA-7 says `ReplayParity` here; see SR-16626.
- **Panics.** None in production. Indexing is guarded by `about_clause`, `marks_match_kind` and the equality check. The `evaluate_projected_prefix` error arm maps to a fault and cannot be reached: the form and `len >= 1` are checked first.

Refusing a mark mismatch as `HyperShape { reason }` is acceptable and needs no spec ruling. FR-183 requires only the code `invalid_runtime_input`/`invalid-value`, and `HyperShape` carries exactly that code.

Evidence, all run by the reviewer at 1ee181562:

- `cargo test -p qsl-replay`: 18 test binaries, 1047 passed, 0 failed (lib 840).
- `cargo test -p qsl-analyze --test it hyper_`: 139 passed, 0 failed.
- `cargo clippy -p qsl-replay -p qsl-analyze --all-targets -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.

Mutation (red) checks, each in the review worktree and then reverted:

- Removing the Projected arm fails 5 tests.
- Removing the `trace_position` check fails 2 tests.
- Removing the visible-count equality check fails none (FND-002).
- Removing both mark rules fails none of the 139 `hyper_` tests (FND-001).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At a152a0f10 each Projected refusal case isolates one rule and asserts the reason through `shape_reason`: every step marked skipped with counts 0/0 and position 0, and both runs mid-operation with `mix` marked visible, counts 2/2 and position 2. Reviewer red check: neutralising either mark arm alone fails 2 tests. |
| FND-002 | fixed | The `fewer` case now sets `trace_position` to the first trace's visible count, so only the equality rule can refuse it. Reviewer red check: removing the equality check fails 2 tests. |
| FND-003 | fixed | Any kind other than `Undefined` of an `align skip` clause is now `HyperShape` (`WRONG_KIND_FOR_ALIGN_SKIP`); `HyperProjected` is kept for `Undefined` only. A new test covers Lockstep and WitnessExhausted, and reverting the change fails it. |
| FND-004 | fixed | The `HyperProjected` `#[error]` text now names the `Undefined` counterexample, and the `HyperShape` doc lists the Projected reasons and the wrong-kind case. |
| FND-005 | accepted-no-change | Changing only `b`'s first input leaves `b`'s loop, re-entered at 1, open: `(1,0) -step(1)-> (1,0) -step(0)-> (1,1)`. FR-128's loop check precedes `μ_U`, so the literal case is refused `LoopOpen`, not `μ_U` false. The test changes both steps and says why in a comment, which keeps AC-3's intent (`μ_U` false) and the tag. Spec ruling to request: amend FR-183-AC-3's clause to "with `b`'s inputs changed to 1 at both steps (`μ_U` false at joint step 1)". |
| FND-006 | fixed | The replay first evaluates the whole prefix, then bisects for the shortest bad prefix and reports its last position. That is sound because a bad prefix stays bad as it grows, and an `Incomplete` or `WindowLimitReached` probe settles no value. The engine's canonical counterexample still reproduces at L (its BFS-first rejecting state is minimal), and `hyper_settle` does not compare positions. The new test (payload names 2, reproduces at 1) fails when the payload position is echoed. |
