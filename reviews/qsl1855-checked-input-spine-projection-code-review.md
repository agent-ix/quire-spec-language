---
id: SR-20261110063718000001
title: "QSL-1855 checked-input spine projection code-review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1855; branch qsl-12-drift-evidence commit 1630cac58 against origin/main bcb64e734; qsl-replay/src/outcome.rs, qsl-replay/src/spine.rs, callers qsl-command/src/command.rs and qsl-replay/tests/it/source_diagnostic_outcome.rs, xtask/src/checked_input.rs, ADR-032, FR-270"
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-270, type: references}
  - {target: ix://agent-ix/quire-spec-language/ADR-032, type: references}
---
## Summary

Review of 1630cac58, which renames the `pub` `OutcomeDiagnostic::from_source_diagnostic` in `qsl-replay/src/outcome.rs` to the crate-private `project_source_diagnostic` and re-exposes the `pub` name through an inherent `impl OutcomeDiagnostic` block in `qsl-replay/src/spine.rs`, so `cargo xtask checked-input` (FR-270, ADR-032 CK-1/CK-3) passes. Behavior and callers are unchanged and the gate now passes, but the new impl block sits after the `#[cfg(test)] mod tests` block and fails workspace Clippy under `-D warnings`, which `make ci` runs. Code-review includes the Rust review lane.

## Verdict

**FAIL**: FND-001 is `high`. The change turns one `make ci` prerequisite green and another red. Moving the impl block above the `#[cfg(test)]` modules (before `spine.rs:2488`) fixes it. The rest of the change is sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new `impl crate::outcome::OutcomeDiagnostic` block is declared after `#[cfg(test)] mod tests`, so `clippy::items_after_test_module` fails `cargo clippy -p qsl-replay --all-targets -- -D warnings`; `make ci` runs workspace Clippy with `-D warnings` (Makefile:43, :234), so this commit makes CI red. | qsl-replay/src/spine.rs:3508 |
| FND-002 | low | The wrapper's doc comment cites "FR-270 CK-1". CK ids belong to ADR-032 (cited as `ADR-032 CK-n` per its Context), and the rule that keeps a pre-check type out of a stage entry's parameters is CK-3; CK-1 only defines the stage-entry set and the `spine` exclusion. | qsl-replay/src/spine.rs:3513 |
| FND-003 | low | The contract text "Retains the typed cause, optional region and human message. A blank label retains the source owner's exact label; other causes omit it." is now stated twice, on the crate-private projection and on the public wrapper. A future change to the projection has to edit both. Keep the contract on one of them and point the other at it. | qsl-replay/src/outcome.rs:1403, qsl-replay/src/spine.rs:3510 |

## Analysis

**Behavior.** `from_source_diagnostic(&CompleteDiagnostic)` in `spine` forwards to `project_source_diagnostic`, whose body is unchanged. `qsl_cst::CompleteDiagnostic` (spine's import) and `qsl_cst::diagnostic::CompleteDiagnostic` (outcome's path) are the same type, so the signature is unchanged. The two internal call sites (`From<&FormatRefusal>` and `from_compile_refusal`) now call the crate-private name directly. Behavior is identical.

**Callers.** `qsl-command/src/command.rs:411` and `:428` and `qsl-replay/tests/it/source_diagnostic_outcome.rs:30` still call `OutcomeDiagnostic::from_source_diagnostic` by the same path. Inherent methods resolve across modules of the defining crate, so no caller changes. The wrapper must stay `pub` because `qsl-command` is another crate. Verified by compiling and by the source-diagnostic integration tests (3 passed).

**Gate intent (ADR-032 CK-1 to CK-4).** `CompleteDiagnostic` is a qsl-cst struct with fields that a `pub` qsl-cst function returns (`qsl_cst::diagnostic::source_refusal`), so it is a CK-2(a) pre-check representation, and the gate correctly flagged a `pub` method taking it in `qsl-replay` outside `spine`. The function projects a refusal record into a wire diagnostic. It executes, lowers and rebuilds nothing, and it names no pre-check stage function (CK-4). ADR-032 places the S1 to S4 orchestrator in `spine`, and that orchestrator produces this diagnostic and already carries it (`CompileRefusal::Source(Box<CompleteDiagnostic>)`, spine.rs:259). So projecting S1's refusal there fits CK-1's split: orchestrators handle pre-check outputs, and stage entries take checked values. The move does not hide a stage entry, and no execution path gains a pre-check input. The gate is not narrowed: `xtask/src/checked_input.rs` and the Makefile are untouched.

## Coverage

Plan completion: not assessed

Scope is the diff only. Pre-existing tree defects are out of scope. No spec or plan diff, so spec-review and its sub-analyses do not apply. No applicable AssuranceProfile was found under `spec/`. The diff adds no new type, dependency, unsafe code, async lifecycle, wire change or test, and duplicates no production rule. FND-003 concerns doc text only.

## Verification and Limits

Run in `/Users/peter/dev/qsl-1825-stack` at 1630cac58:

- `cargo run --package xtask -- checked-input`: exit 0 ("no stage entry takes, and no stage crate rebuilds, a pre-check representation").
- `cargo fmt --check -p qsl-replay`: exit 0.
- `cargo clippy -p qsl-replay -p qsl-command --all-targets --all-features -- -D warnings`: exit 101, `items_after_test_module` at spine.rs:3508 (FND-001).
- `cargo test -p qsl-replay --test it source_diagnostic`: 3 passed.
- `cargo test -p qsl-command`: all pass (14 and 10 tests across its targets).
- `cargo test -p xtask checked_input`: 5 passed (TC-745 checked-input tests).

The full `make ci` aggregate was not run.

## Dispositions

| FND | Disposition |
| --- | --- |
| FND-001 | Fixed: the impl block moved above the test modules; clippy -D warnings is clean. |
| FND-002 | Fixed: the doc cites ADR-032 CK-3. |
| FND-003 | Fixed: the wrapper doc points at the projection instead of repeating it. |
