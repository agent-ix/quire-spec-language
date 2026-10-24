---
id: SR-16310
title: "Code review of QSL-1016b value-parity profile selections fixed empty (QSL PR #1390)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3aa38f6ce311416b06a3ce0d0622092515769987; qsl-replay/src/execute.rs, qsl-replay/src/execute/value_parity.rs, qsl-replay/src/outcome.rs, qsl-replay/src/admitted.rs, qsl-replay/src/execute/tests/value_parity_identity.rs, tests/it/value_parity_profile_selections.rs, tests/it/main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-484
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: references
---
# SR-16310: Code review of QSL-1016b value-parity profile selections fixed empty (QSL PR #1390)

## Summary

Ticket: QSL-1016 (PR #1390, branch `qsl-1016b-impl`, stacked on #1386). Code review with the
Rust lane (`rust-review`) folded in, scoped to `git diff origin/qsl-1016-impl...3aa38f6ce`. The
spec files in that diff come from the `main` merge (STD-443, #1387) and are not this PR's own
change. The PR's own change is 7 files: the new `ReplayRefusal::UnusedProfileSelections`
variant, the check in `value_parity::settle`, two exhaustive-match arms, and the AC-11/12/13
tests.

## Verdict

**PASS**: no findings.

Measured against FR-484 Behavior as amended by STD-443:

- Refusal order. `settle` runs `ReplayRequest::decode`, then `function_target` (the FR-357-AC-20
  replay-target check), then the new profile-selection check, then `request_limits`,
  `recompile`, `select`, `check_identity` and the argument join. So the refusal comes after the
  target check and before the limits, the recompile and any package or lock read. There is no
  call and no charge: `ValueParityResult::Refused` carries no charges.
- Settlement. The error goes through `replay_value_parity`'s `Err` arm. `input_refusal()` is
  `None` for the variant, so the result is `Refused`. `TerminalValue::from_replay_refusal` gives
  `Inconclusive(ReplayRefused(invalid_runtime_input))`. The claim is `ValueIdentity::sent`,
  computed before `settle`, so the claim identity is carried unchanged.
- Code `InvalidRuntimeInput`, cause `unknown-member`, shared with `UnusedObjectEnvironment`. The
  display text is byte-identical to FR-484 Behavior and AC-12, on one line. The field holds the
  request's selections in request order (`to_vec()` of the decoded list).
- Exhaustive matches. Grep for `ReplayRefusal::UnusedObjectEnvironment` / `ReplayRefusal::` in
  the workspace: `code()`, `cause()` (execute.rs), `replay_stage` (outcome.rs, stage S8 like
  `UnusedObjectEnvironment`) and `From<ReplayRefusal> for AdmittedRefusal` (admitted.rs, the
  unreachable-fault group) are the only exhaustive matches. All are wired. `pointer()` uses a
  wildcard and gives `None`; the spec names no pointer for this refusal.
- Downstream breakage. `ReplayRefusal` is not `#[non_exhaustive]`, so the PR body's BREAKING
  note is correct as an API statement. A grep of every local checkout under `~/dev` outside the
  QSL checkouts found no `ReplayRefusal` use at all. CG and the driver currently have no
  exhaustive match that breaks.
- Lock rows. No new code reads the lock's `profile_selections`. The rows bind only through
  `package_id`, which the recompile checks.
- No vendoring, no copied files, no new re-exports, no `unwrap`/panic on a production path, no
  ceremony.
- Gates (focused, through `locked-build.sh`): `cargo test -p qsl-replay --lib -- tc_1252 tc_904`
  45 passed, 0 failed. `cargo test --test it -- value_parity_profile_selections` 1 passed.
  fmt and clippy were not re-run; the PR body reports them clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
