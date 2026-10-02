---
id: SR-1220
title: "QSL-606 gap analysis of PR #598 (FR-270, TC-745)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a49ee006adcdd0e5014dbf97f5b196df93d10aa2; PR #598 diff against origin/main; spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md; spec/test-cases/TC-745-the-checked-input-gate-rejects-pre-check-signatures-and-reconstruction.md; spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-270
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-745
    type: reviews
---
## Summary

Ticket: QSL-606. PR: quire-spec-language#598.

Trace, AC to test:
- FR-270-AC-1: `tc_745_planted_signatures_fail_the_gate` plants all five
  signatures over a copy of the scanned trees. It asserts the exact finding
  list (file, line, rule, entry, resolved type). This is a strong oracle.
- FR-270-AC-2: `tc_745_planted_reconstruction_fails_the_gate` asserts the
  exact `reconstruction` finding for the `qsl-cst` parse and
  `PackageDeclarations::check` plants, and `[]` for the plant under
  `qsl-replay/src/spine/`. This is a strong oracle.
- FR-270-AC-3: `tc_745_no_false_findings_and_every_finding_reported` covers
  the `StateClauseKind` entry, the `#[cfg(test)]` module, and two plants
  giving two findings. The `Limits` case is FND-001.
- FR-270-AC-4: `tc_745_the_workspace_passes_and_a_plant_fails_the_command`
  asserts `scan(workspace) == []`. It also asserts that `run` over a planted
  copy returns `Code::CheckedInput` with exit code 1 and the finding line. The
  make half is FND-002.
- `make ci` lists `checked-input`, and the recipe runs
  `cargo run --package xtask -- checked-input`.

Value test: the gate is a real check and it derives its sets from code. The
shared `shipped_files` and `S3_CONSTRUCTOR` take the place of a copy. No
ceremony was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test exercises the configuration exclusion. AC-3's `Limits` plant is a private `fn plant() -> qsl_cst::Limits` that builds `Limits` in its body. Neither rule looks there: a private fn is not an entry, return types are not checked, and the body path `qsl_cst::Limits::default` is not a stage function. No stage entry in the workspace takes `Limits` either (the one use is a body literal at `qsl-replay/src/execute.rs:578`). So deleting the `configuration` filter in `PreCheck::derive` leaves every test green. Add a plant `pub fn plant(l: qsl_cst::Limits) {}` in `qsl-eval` that expects `[]`, keep the construction case, and add the entry case to AC-3's text in the fix round's FR-270 edit. | xtask/src/checked_input.rs:995-999; xtask/src/checked_input.rs:320-332 |
| FND-002 | low | The make half of FR-270-AC-4 ("the `checked-input` make target that `make ci` runs exits non-zero") is checked only by hand (PR body), although AC-4's Verification says Test. An automated test that plants a file in the real tree and runs `make` is not worth it: it mutates the checkout and only re-tests `run`'s exit code, which the in-process test already asserts. `main` maps it with code shared by every subcommand. Change the make clause of AC-4 and TC-745 step 4 to Inspection: the `ci:` prerequisite list holds `checked-input` and the recipe runs `cargo xtask checked-input`. Do not add a test. | spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md (FR-270-AC-4); spec/test-cases/TC-745-the-checked-input-gate-rejects-pre-check-signatures-and-reconstruction.md (step 4); Makefile:35-39 |

## Verdict

Changes requested: FND-001 is medium and FND-002 is low. The SR-1219 FND-001
fix also needs TC-745 plants: a method-syntax `.check(..)` on a value from a
`pub` non-pre-check function, and a `<PackageDeclarations>::check` call.
