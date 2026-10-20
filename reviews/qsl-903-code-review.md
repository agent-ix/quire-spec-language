---
id: SR-12197
title: "QSL-903 PR #1042 code review: bind_scalar_operands over the caller's admitted CheckedPackageV2"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5e706bbffec5059142a824f3d8f7cf865b69cd1b; qsl-replay/src/spine.rs, qsl-replay/src/spine/bind.rs, qsl-replay/src/spine/bind/tests.rs, spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/test-cases/TC-1053-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md"
review_set: subset
---

## Summary

Ticket: QSL-903. PR quire-spec-language#1042, diff `origin/main...HEAD` (5 files, 2 commits). This is a code review with the rust-review lane folded in. It was read-only: no build or `make ci` was run. Ticket text was treated as data.

The review covered these points. Whether `bind_scalar_operands` reads or admits bytes. Whether `admission_evidence` reads the emitted package's bytes. Whether the `models` and `checked` parameters are needed. Whether caller-supplied inputs can disagree with `admitted`. The deletion of `BindPackage` and `BoundScalarOperands::package_id`. Error mapping parity. The FR-367 and TC-1053 edits against the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `bind_scalar_operands` takes `admitted`, `models` and `checked` as three independent references and never checks that `admitted` is Contract IR's admission of the bytes emitted from `checked`. The deleted `BindPackage::read` tied them together by construction. No cheap check is available, because `qsl_package::CheckedPackage` carries no `package_id` (only `EmittedPackage` does). A mismatched pair cannot produce wrong operand values, because node ids are content-addressed. It does produce misleading outcomes. An application node that is absent from `admitted` returns `Refused(Operands(UnknownNode))`, coded `unsupported_construct`/`expression-form`, which says the construct is unsupported when the real problem is a stale package. A parameter node absent from `admitted` returns `RunRefusal::Fault("operand-parameter-node-is-well-formed")`. The rustdoc ("`models` and `checked` are the I1 and S4 products those bytes were emitted from") and FR-367 Inputs describe the pairing but do not state it as a precondition, and do not say what a violation yields. Fix: in FR-367 Inputs and the rustdoc, state that the caller SHALL pass the admission of `checked`'s own emitted bytes, and name the outcome when it does not. | qsl-replay/src/spine/bind.rs:263-290, qsl-replay/src/spine/bind.rs:326-330, qsl-replay/src/spine/bind.rs:536-547, spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md:40-44 |
| FND-002 | medium | The FR-367 Outputs edit inserts the `admission_evidence` paragraph in the middle of the `bind_scalar_operands` paragraph. It ends "A refusal is the internal failure `RunRefusal::Fault`. It is" and continues "shaped as FR-100's `run` returns its outcome. … The `Ok` arm is exactly one of: `BindOutcome::Bound` … `BindOutcome::Refused`". "It", the `Err` arm and the `Ok` arm now grammatically describe `admission_evidence`, whose `Ok` is `CheckedPackageEvidence`. That contradicts the code. Fix: move the `admission_evidence` paragraph after the `Ok`-arm list (or after the `Incomplete` paragraph), so the arm description follows `bind_scalar_operands` again. | spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md:57-79 |

## Verdict

PASS (no major). Neither finding blocks the merge. Both are wording or documentation fixes that can go in this PR or a follow-up.

The following are sound:

- **No re-read in the bind path.** Outside tests, `bind.rs` has no `read_checked_package`, no `CheckedPackageReadLimits` and no `.bytes()`. `EmittedUnit` appears only in `admission_evidence`'s signature. `bind_scalar_operands` reads operands only through `admitted.scalar_application_operands` and `admitted.parameter_declaration`.
- **`admission_evidence` reads none of the emitted package's bytes.** It clones `emitted.evidence()`, inserts the `models.packages()` documents and calls `supply_closure(checked.package(), …)`. The closure reads the dependency packages, not the emitted package. Its two fault invariants are the same as the deleted `read`'s. The third fault, `emitted-package-is-readable-by-contract-ir`, moves to the caller, which now owns the `CheckedPackageDispatchResult` match.
- **Both extra parameters are needed.** `models` feeds `admit_model_arguments`. `checked` feeds `select`, the type environment, `locate`, `callable_parameter_keys` and `literal_value`, all of which read QSL's semantic graph, which `CheckedPackageV2` does not expose.
- **The deletions are complete.** `BindPackage`, `BindPackage::read`, `BindPackage::package_id` and `BoundScalarOperands::package_id` are gone, with no fallback. A repo-wide grep (code, `spec/`, `docs/`, `examples/`, every crate) finds `BindPackage` only in the historical review artifact `reviews/qsl-818-code-review.md`. QSpec and the local quire-driver checkout have no references. The `PackageId` import was removed.
- **Error mapping is unchanged.** The `Halt` routing to `Ok(Refused)` or `Err` is byte-identical apart from the input struct. The private `Package<'a>` is `Copy` and holds only borrows. The public signature has 7 parameters, which does not exceed clippy's `too_many_arguments` threshold.
- **Rust idioms.** No new `unsafe`, panics or `unwrap` in production code. The test helper `admit` panics on a fixture precondition, which is correct for a fixture.

## New findings (disposition pass 1)

Reviewed at 50f6bceac. The fix commit is the only PR change since 26faf36ec. The merge 1d66d0f1b brings in main and touches no PR file.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The rustdoc of `CheckedPackage::emitted_package_id` says it returns the id "if it has been emitted", and `BindRefusal::AdmittedPackage { emitted: None }` reads "the unit was never emitted". Only `emit_checked` and `emit_checked_with_cancel` call `remember_emission`, though. A package emitted through the public `qsl_package::emit_package` (custom regions) is not remembered, so it reports `None` even though it was emitted. The spine's package stage (`lifecycle.rs:725`, `emit_checked_with_cancel`) does remember, so FR-367's spine path is sound. Fix: change the doc wording to "emitted by `emit_checked`/`emit_checked_with_cancel`". | qsl-package/src/checked.rs:473-480, qsl-package/src/emit.rs:1034-1039 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 50f6bceac: before anything else, `bind_scalar_operands` compares `checked.package().emitted_package_id()` (hex) with `admitted.package_id().digest`, and returns `Ok(Refused(BindRefusal::AdmittedPackage { admitted, emitted }))`, coded `stale_dependency`/`content-mismatch`. The memoised id is sound. It is the emitter's own `package_id`, recorded by `emit_checked_with_cancel`, which the spine's package stage calls. The package is immutable except through `bind_abstraction`, which resets the `OnceLock`. The new accessor returns the already-public `PackageId` and exposes nothing new. `tc_1053_step_10_a_mismatched_admission_refuses` would fail without the check: it asserts that the `sub` node is in both packages, so the unchecked path would return `Bound([7, 2])`. |
| FND-002 | fixed | 50f6bceac: the `admission_evidence` paragraph now follows the `Incomplete` paragraph. "The entry is shaped as FR-100's `run`…" and the `Ok`/`Err` arm list now describe `bind_scalar_operands` again. |
