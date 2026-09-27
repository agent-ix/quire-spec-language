---
id: SR-745
title: "PR 492 code review (QSL-245 remainder)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@caa1520a4393c132583accda17aa9f8c01c14949; diff 2df75ab6...caa1520a; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/src/value/expression/mod.rs; qsl-eval/tests/it/model_reference_queries.rs; qsl-foundation/src/diagnostic/stage.rs; qsl-replay/src/bounds.rs; qsl-semantics/src/check/region.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#492. Code review with the Rust lane (rust-review) over the PR diff only.

- **CheckedInvariant to InternalFault.** `Machine::stopped` now returns `Err(InternalFault::new("S6a", "checked-program-invariant"))` for a kernel `CheckedInvariant`. `Machine::run` already returned `Result<Evaluation, InternalFault>`, and `stopped` is private, so no caller's signature changes. Every production caller was traced. `CheckedPackage::call` goes through `evaluate_declaration` and `ValueFunctionFamily::evaluate`; `CheckedPackage::evaluate` calls `Machine::run` directly. Both already map `Err` to `CallFailure::Fault`. The two production consumers are `qsl_replay::spine::run`, whose output is byte-identical (the same `RunRefusal::Fault` with `S6a`/`checked-program-invariant`, now reached through `convert_call_failure`), and `qsl_replay::replay`, whose behaviour changes (see the gap-analysis review).
- **"Defense-in-depth, never reached" is true.** `Machine::stopped` is the only place that builds `Evaluated(outcome_from_stop(Err(stop)))`. `Halt::Family` cannot carry a kernel refusal. The success arm builds `Completed`. `ValueFunctionFamily::evaluate` adds only `Kernel(Incomplete)`. So `qsl-replay/src/spine/call.rs:544` can be reached only from its own unit test.
- **Test input.** `not x` given `Value::Integer` reaches `pop_boolean` (evaluate.rs:522), whose non-Boolean arm is `invariant()`, which is `Halt::Stop(Refusal::CheckedInvariant)`. The mutation re-run confirms it: with the check disabled, the test panics with `Kernel(Refused(CheckedInvariant))`.
- **Rust idiom.** No new `unwrap` or `expect` in production, no panics, no integer-width changes. Removing `DeclarationRegions::limit_exceeded` also drops the now-unused `LimitExceeded`/`Locus` import. The tests follow repo conventions (`#[trace]`, whole-map asserts, `let ... else` with a panic message).

## Verdict

**Approve with changes.** Four low findings, all comments or test hygiene. None changes behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `stopped`'s doc cites FR-096-AC-9, which is the I2 reader's version refusal. The requirement is FR-096-AC-15, or the Behavior text "A kernel `CheckedInvariant` SHALL become an `InternalFault`". | qsl-eval/src/value/expression/evaluate.rs:492-494 |
| FND-002 | low | Two docs are stale now that `CheckedInvariant` is `Err`. `Machine::run`'s "`Err(InternalFault)` (FR-090-AC-10)" paragraph still names only `resolve_population` faults. `CheckedPackageEvaluation::call` still says the kernel outcome is returned "unchanged". | qsl-eval/src/value/expression/evaluate.rs:423-432; qsl-eval/src/value/expression/mod.rs:289-296 |
| FND-003 | low | The doc on `a_checking_limit_stop_is_located_at_its_node` says node count is located at "the node whose entry failed the charge". The node-count case asserts the whole declaration text. That case is the family's per-declaration `check_node_count` (actual 7 is the preimage count, family.rs:1470), not `Typer`'s package-wide count, whose actual would be bound+1 = 3. | qsl-semantics/src/check/region.rs:411-418, 440-446 |
| FND-004 | low | `a_lowering_stop_is_located_by_its_location` hard-codes `declared + 36` with no derivation. A cost-table change would silently move the stop to a different node, or past the end of lowering. Derive it, or say in a comment what the 36 units are. | qsl-semantics/src/check/region.rs:489 |
