---
id: SR-202611010117
title: "Code and Rust review of QSL-1384 PR #2049"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@40bc5a8b3d08193de3e893e64a65fb662b19072d; PR #2049 diff against peter/qsl-1443-ir1230-relock; FR-050-AC-6"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
---

## Summary

Reviewed PR #2049 at `40bc5a8b3d08193de3e893e64a65fb662b19072d` against its actual base `peter/qsl-1443-ir1230-relock`. Producer-side explicit clock selections and the focused emission test pass, but two replay identity paths discard those selections, so a configured temporal package containing state-graph or hyper claims can fail during readback.

## Verdict

**FAIL** — `StateGraphIdentity::resolve` and `HyperIdentity::resolve` pass `&[]` to `read_back`, which emits the entire checked package through the selection-aware writer. For packages containing a clock-configured temporal declaration and a state-graph or hyper claim, readback fails with `MissingTemporalClockSelection` despite a supplied selection; thread selections through these identity/replay paths and cover this combination.

## Assurance Context

No applicable AssuranceProfile was found under `spec/` for the changed code.

## Scope

- `Cargo.lock`: examined.
- `qsl-analyze/src/model_check/check.rs`: examined.
- `qsl-analyze/src/model_check/liveness.rs`: examined.
- `qsl-analyze/tests/it/bounded_closure.rs`: examined.
- `qsl-analyze/tests/it/closure_certificate.rs`: examined.
- `qsl-analyze/tests/it/component_certificate.rs`: examined.
- `qsl-analyze/tests/it/example_units.rs`: examined.
- `qsl-analyze/tests/it/fill.rs`: examined.
- `qsl-analyze/tests/it/flag_world.rs`: examined.
- `qsl-analyze/tests/it/lasso_replay.rs`: examined.
- `qsl-analyze/tests/it/liveness.rs`: examined.
- `qsl-analyze/tests/it/model_check.rs`: examined.
- `qsl-analyze/tests/it/model_check_captures.rs`: examined.
- `qsl-analyze/tests/it/model_check_safety.rs`: examined.
- `qsl-analyze/tests/it/model_instances.rs`: examined.
- `qsl-analyze/tests/it/model_settle.rs`: examined.
- `qsl-analyze/tests/it/partial_order.rs`: examined.
- `qsl-analyze/tests/it/por_world.rs`: examined.
- `qsl-analyze/tests/it/protocol_compensation.rs`: examined.
- `qsl-analyze/tests/it/protocol_counted_steps.rs`: examined.
- `qsl-analyze/tests/it/protocol_roles.rs`: examined.
- `qsl-analyze/tests/it/protocol_system.rs`: examined.
- `qsl-analyze/tests/it/race_freedom.rs`: examined.
- `qsl-analyze/tests/it/reduction.rs`: examined.
- `qsl-analyze/tests/it/reduction_world.rs`: examined.
- `qsl-analyze/tests/it/repeat_guard.rs`: examined.
- `qsl-analyze/tests/it/scheduler_fairness.rs`: examined.
- `qsl-analyze/tests/it/strong_fairness.rs`: examined.
- `qsl-analyze/tests/it/strong_hint.rs`: examined.
- `qsl-analyze/tests/it/tso_fairness.rs`: examined.
- `qsl-analyze/tests/it/weak_memory.rs`: examined.
- `qsl-package/src/checked_v2/tests.rs`: examined.
- `qsl-package/src/emit.rs`: examined.
- `qsl-package/src/emit/tests.rs`: examined.
- `qsl-package/src/emit/tests/hybrid.rs`: examined.
- `qsl-package/src/emit/tests/hyper_clause.rs`: examined.
- `qsl-package/src/emit/tests/temporal_clause.rs`: examined.
- `qsl-package/src/lib.rs`: examined.
- `qsl-replay/src/admitted.rs`: examined.
- `qsl-replay/src/certificate/closure.rs`: examined.
- `qsl-replay/src/compile.rs`: examined.
- `qsl-replay/src/execute.rs`: examined.
- `qsl-replay/src/lib.rs`: examined.
- `qsl-replay/src/model_check/identity.rs`: examined.
- `qsl-replay/src/model_check/item.rs`: examined.
- `qsl-replay/src/model_check/replay.rs`: examined.
- `qsl-replay/src/spine.rs`: examined.
- `qsl-replay/src/spine/lifecycle.rs`: examined.
- `qsl-replay/src/spine/lifecycle/tests.rs`: examined.
- `qsl-replay/tests/it/vault_world.rs`: examined.
- `spec/reviews/qsl-1443-code-review.md`: examined.
- `spec/reviews/qsl-1443-gap-analysis.md`: examined.
- `tests/it/emit_checked_v2.rs`: examined.
- `FR-050-AC-6`: examined; checked-route counterparts must join the exact source locus, declaration, profile and clock before evaluation.
- `FR-050`: context only; owns checked temporal selection and typed clock configuration behavior.

## Checks

- `git diff --check origin/peter/qsl-1443-ir1230-relock...HEAD`: passed.
- `cargo test -p qsl-package --lib temporal_clause --locked`: passed, 21/21.
- QSL-1384-reported qsl-analyze integration: 1,160/1,160; qsl-replay lifecycle: 33/33; no-default qsl-package/qsl-semantics, formatting and workspace Clippy passed. These were not independently rerun in this review.
- No aggregate CI command was run by this reviewer.
- Matrix computation was interrupted after producing no output; matrix evidence is unavailable from this review run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `StateGraphIdentity::resolve` and `HyperIdentity::resolve` pass `&[]` to `read_back`, which emits the entire checked package through the selection-aware writer. For packages containing a clock-configured temporal declaration and a state-graph or hyper claim, readback fails with `MissingTemporalClockSelection` despite a supplied selection; thread selections through these identity/replay paths and cover this combination. | qsl-replay/src/model_check/identity.rs:164,219 |
