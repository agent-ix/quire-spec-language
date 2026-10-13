---
id: SR-9084
title: "Code review of quire-spec-language PR #832: model-graph expression row tags and two vectors"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@77b16803f4ba9a2b7c9372fa5a33b42c0529cab9; PR #832 diff vs impl/business-model-meanings: qsl-eval/tests/it/model_reference_queries.rs, qsl-replay/src/spine/clause/tests/frame.rs, qsl-replay/src/spine/clause/tests/pre_call_invocation.rs, qsl-replay/src/spine/clause/tests/state_clause_replay.rs, qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
---
# Code review of quire-spec-language PR #832

## Summary

Ticket: QSL-716. Code review with the Rust review lane folded in. The PR changes tests only and touches no production code.

Checked:

- The diff has 5 files. Every change is a `#[trace(...)]` attribute or one of the two new tests in `qsl-semantics/tests/it/state_clauses.rs`.
- `pre_in_an_invariant_or_a_precondition_refuses_forbidden_pre_read_at_the_pre` asserts typed values. It checks the refusal count (1), the `CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)` cause and the location path `[0]` for each of 2 fixed cases. A postcondition control over the same body checks, so the test would fail if the clause-kind rule were widened.
- `an_invocation_naming_the_wrong_observation_roles_refuses_wrong_observation` asserts `wrong_snapshot`/`wrong-observation` at admission through `assert_frame_refused` for the swapped references and for post naming the pre snapshot. Removing the role check would turn both cases red. The `expect` calls are on fixture JSON that the test builds itself, which is acceptable in a test.
- `#[trace]` comes from `ix_trace_rs`, which takes the strings as given. The `QSpec-` prefix matches the existing `QSpec-V1-TYPE-022` and `QSpec-FR-042-AC-*` tags.
- Gate: `qsl716-gate-clippy.log` (`--workspace --all-targets --all-features -D warnings`) EXIT 0. `qsl716-gate-tests.log` sums to 3613 passed and 0 failed, the two new tests are `ok`, and the xtask lib tests pass. `qsl716-gate-spec.log` EXIT 0. I ran `cargo fmt --all -- --check` read-only on the worktree and it was clean.

## Verdict

No code defect. Trace correctness belongs to the gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
