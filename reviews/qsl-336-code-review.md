---
id: SR-906
title: "QSL-336 code review of PR 541 (state-clause replay entry, FR-106 PreCall admission)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ec9476fb2a8970754aaf6974647841a995991f32; qsl-replay/src/execute.rs; qsl-replay/src/execute/state_clause.rs; qsl-replay/src/execute/frame.rs; qsl-replay/src/lib.rs; qsl-replay/src/spine.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/witness.rs; qsl-replay/src/witness/state_clause.rs; qsl-replay/src/spine/clause/tests.rs; qsl-replay/src/spine/clause/tests/frame_replay.rs; qsl-replay/src/spine/clause/tests/state_clause_replay.rs; qsl-semantics/src/model/observation.rs; qsl-semantics/src/model/observation/document.rs; qsl-semantics/src/model/observation/frame.rs; qsl-semantics/tests/it/state_clauses.rs; qsl-eval/src/value/expression/mod.rs; examples/config-version/spine.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-336 (code half). PR: quire-spec-language#541 at ec9476fb, diff
`origin/main...HEAD` (merge base 7d0497be). This file covers code review
with the rust-review lane folded in.

What was checked:

- **Facade surface.** `make arch-lint-api-surface
  CG_CLONE=/home/peter/dev/quire-contract-codegen` passes T12-A..E and
  FR-100-AC-8 (exit 0, run by the reviewer). Re-exporting
  `ClauseSelectionInput`, `ObservationForm`, `SnapshotValue`,
  `SelectedAnchor`, `AnchorKind` and `StateClauseKind` through `qsl-replay`
  follows the existing facade precedent (`AdmissionFailure`, `DocumentRef`,
  `SelectedObject`, `FrameWitness` are already re-exported from the same
  module). CG needs these to build a `StateClauseCounterexample`, and it
  names them only through `qsl-replay` (ADR-011 FB-05). No `spine` type
  reaches the signature. This is a valid facade, not a stage leak.
- **Replay order.** decode, recompile, stale `package_id`, name lookup,
  clause node, occurrence key, form, then admission and evaluation. This
  matches FR-122 Behavior. Mutants that drop the form check, the node check
  or the occurrence check each fail a TC-517 test (reviewer's
  `sr-541-mutants.log`, exit 101 each).
- **`check_clause` extraction.** `run_clause`'s `Clause` arm now calls the
  same `check_clause` replay uses. The reported documents are unchanged for
  `Current` and `Invocation`, and `admitted_documents` gives the same read
  order. Renaming `FrameRun` to `CompiledRun` is justified because the
  struct now serves clause runs too (pub(crate), three call sites). It is
  not churn.
- **Evaluator fallback.** Moving `pre` ahead of `post` breaks four
  postcondition tests (mutant M4, exit 101), so the main path is pinned.
  The remaining risk is FND-002.
- **Check 7 seeding.** The seeding for `PreCall` is pinned by the archive
  test (mutant M5, exit 101). The seeding for invocations is not pinned
  (FND-003). No existing `run_clause` test regressed: the whole workspace
  passes with and without it.
- **Rust lane.** The new production code has no `unwrap`, `expect` or
  wildcard arm. The admission form match lists all nine pairs. The large
  refusal is boxed (`ClauseIdentity(Box<..>)`). Errors use thiserror and
  carry stable codes (`Code::WrongSnapshot`, `MissingDeclaration`,
  `StaleDependency`). The `RawValue` to `SnapshotValue` rename comes from
  making the type public, so it is not churn.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `parameter_populations` collects every reference in every supplied parameter value, including undeclared parameters and parameters whose declared type is not a reference. It then makes those populations required at check 7, which runs before check 10. Reproduced: a `PreCall` for `ReachesTarget` with a valid `target` plus an undeclared `zextra` naming `archive`/`a1` (archive `complete: false`) returns `Incomplete` `incomplete_population`/`incomplete-scope` naming `archive`. It should refuse `invalid_runtime_input`/`unknown-member`. FR-106 makes required only the populations "a reference-valued parameter names", meaning a declared parameter. Calling malformed input incomplete tells the caller that more evidence would fix it. The invocation path has the same defect. Fix: derive the populations from the operation's declared reference-typed parameters only. | qsl-semantics/src/model/observation/document.rs:1242-1256; qsl-semantics/src/model/observation.rs:1022,1150 |
| FND-002 | medium | `evaluate_clause` now falls back to `pre` when there is no `current` or `post`, whatever the clause kind. A postcondition or invariant given an `AdmittedObservations` with only `pre` used to fault S6a. It now evaluates silently, reading the pre state as the clause's own observation, so post reads see pre values and can give a wrong verdict. Admission never builds that shape today, but `AdmittedObservations` has all-pub fields and `CheckedPackageEvaluation::evaluate_clause` is public. Tests already build it by hand (tests.rs:331). Fix: select by `declaration.kind()`: invariant reads `current`, postcondition reads `post`, precondition reads `post.or(pre)`. Keep the fault for anything else. | qsl-eval/src/value/expression/mod.rs:492-502 |
| FND-003 | medium | No test covers the new check-7 seeding on the invocation path. Replacing `&document::parameter_populations(views, &invocation.parameters)` with `&[]` passes `cargo test --workspace` (reviewer mutant M6, exit 0). This is a behaviour change to existing invocation admission, used by `run_clause` and by frame replay, so nothing guards it. Add an invocation twin of the TC-464 step 5 archive test. | qsl-semantics/src/model/observation.rs:1022; qsl-semantics/tests/it/state_clauses.rs |
| FND-004 | low | The disclosure comment's reasoning is now stale. It says the evaluator's missing-observation fault is unreachable because `admit_operation` always sets `pre` and `post` and never leaves both `current` and `post` `None`. A `PreCall` admission now does leave both `None`, and only the new `pre` fallback keeps that path from faulting. The comment was edited for the renamed fault text but its argument was not updated. | qsl-replay/src/spine/clause/tests.rs:2276-2290 |

## Verdict

The replay entry is correct and well tested. Identity, form and admission
refusals all have strong oracles, and the facade surface passes arch-lint.
FND-001 and FND-002 are real edge defects in shared admission and evaluation
code. Both should be fixed in this PR along with FND-003, which is a small
test. FND-004 is a comment.
