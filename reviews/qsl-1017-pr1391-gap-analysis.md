---
id: SR-16309
title: "Gap analysis of quire-spec-language PR #1391: hybrid forms and checked hybrid model (QSL-1017 PR 1 of 4)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f6e8991c2f77c5e63556f0ff568de37ff9270d95; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md (AC-1, AC-2, AC-6, AC-8; AC-7 expression-form half); qsl-forms/src/hybrid_declaration.rs; qsl-semantics/src/check/hybrid/; qsl-forms/tests/it/hybrid_forms.rs; qsl-semantics/src/check/hybrid/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
---
# Gap analysis of quire-spec-language PR #1391

## Summary

Ticket: QSL-1017 (PR 1 of 4). Plan completion: not assessed. The AC-to-test
map was built by reading the `#[trace]` tags in the diff (the repo-wide
`quire matrix` did not finish in the time allowed here). FR-447-AC-3 to
AC-5 and AC-7 (calls) to AC-17 belong to the later PRs of the stack, as the
PR body says, and are not assessed.

- FR-447-AC-1: S2 half
  (`braking_builds_a_hybrid_form_with_its_modes_equations_and_transition`)
  and S3 half
  (`braking_checks_to_a_classified_model_with_its_rates_initial_mode_and_reset`)
  assert the modes, equations and spans, the variable order and classes, the
  `Moving` rates, the default initial entry and the reset. Each can fail.
- FR-447-AC-2: all 14 mutations asserted for code, tag and the source text
  at the refusal's span, each as the sole refusal.
- FR-447-AC-6: S2 entries and spans, S3 order, the written default equal to
  the unwritten one, `Halted` and the non-Boolean condition, `initial` as an
  identifier. The checked-package-identity clauses are tested in PR 3
  (#1393, `qsl-package/src/emit/tests/hybrid.rs`).
- FR-447-AC-8: every listed reset, including `pre(v)`, the bare pre-jump
  read, the `let` and both refusals.
- AC-7 expression-form half: tested and deliberately left untagged until the
  calls land.

No stub, placeholder return or unowned code. `CaptureIndex` and
`TransitionIndex` have no caller in this PR; the stacked PRs use them.

## Verdict

**CONDITIONAL**: one medium coverage gap. The "naming both" and "naming
`Stopped` and `t`" clauses of AC-2 and AC-8 have no assertion.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | FR-447-AC-2 and AC-8 require the duplicate refusals to name both declarations and the missing-equation refusal to name `Stopped` and `t`. `sole_refusal` returns only code, tag and the region text, so no test reads `DuplicateMode::first`, `DuplicateEquation::first`, `DuplicateReset::first` or `MissingEquation::{mode, variable}`. If one of these fields is wrong, for example `first` set to the second span or `variable` to the wrong name, every test still passes. Assert the cause's fields for those four cases. | qsl-semantics/src/check/hybrid/tests.rs:98-111, 176-253, 523-530 |
