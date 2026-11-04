---
id: SR-20261104163646
title: "QSL-1638 gap analysis of commit ee0f6c400"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; commit ee0f6c400 (PR #2301), QSL-1638 qsl-semantics files only; FR-480 Selections and imports; qsl-semantics/src/check/{assemble.rs,check.rs,model_spelling.rs,timed_model/reading.rs,lowering/model.rs}; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/tests/it/timed_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
---

## Summary

Ticket: QSL-1638. Plan completion: not assessed (planless). The ticket lists four lookups. These tests, each tagged `#[trace("FR-480")]`, back them:

- Whole-model role: `a_whole_model_role_on_a_shared_alias_selects_its_own_units_package` assembles and checks two units, each with a protocol `role w on N`, and asserts each role's `model_selection`.
- Port role: `a_shared_model_alias_names_each_units_own_selection_and_port` is a unit test over a hand-built `Scope`.
- Terminal join: `a_terminal_on_a_shared_alias_is_on_its_own_units_package` asserts `package` and the `self` object of the later unit's package.
- Population-bound join: `a_population_bound_on_a_shared_alias_bounds_its_own_units_package` asserts the bound lands on the later unit's package only.
- Timed-model reader: `a_clock_reading_parameter_names_its_own_units_package`. Unit `a`'s package of the same alias has `x` as a Boolean, so the wrong package refuses.
- `ModelSpellings::selection`: `each_unit_reads_a_model_alias_as_its_own_package`.

Two spot mutations each fail their test: the terminal join and the reader's package filter. The tests are not tautological. The common layout (both units `model M = X;`) is covered downstream by qsl-replay `aggregate_dispatch`/`static_subject`, 17 passed. The full evidence is in code-review SR-20261104163645.

## Verdict

**CONDITIONAL.** The Port-role keying change (`AdmittedModel::port_roles` keyed by package identity) has no end-to-end test. Its only test injects ports into a hand-built scope, so a defect in how assembly builds the keys would pass (FND-001). Nothing tests the remaining alias-only joins in the shared-alias layout (FND-002, the test side of code-review FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No assembly-level test covers a Port role `on N::artifact` when two units bind `N` to two packages that both declare the Port. `source_scope_tests` builds the `(identity, artifact)` port map by hand, so `AdmittedModel::port_roles` and the `with_model_ports` wiring in mod.rs are never exercised in the layout. Fix: add an assembler test in which each unit's protocol has `role r on N::<port>` and both check, asserting each role's `model_member` package and `model_selection`. | qsl-semantics/src/check/lowering/model.rs:413-432; qsl-semantics/src/check/mod.rs:2183-2188; qsl-semantics/src/check/check.rs:4981-5021 |
| FND-002 | medium | No test covers a time source (`time N`) or the other remaining alias-only joins (code-review SR-20261104163645 FND-001) in the one-alias-two-packages layout. `a_clock_reading_parameter_names_its_own_units_package` writes `time Config` in the later unit, but it asserts only that the check succeeds, not which package's timed model was built. Fix: assert that the timed model built for unit `b`'s `time Config` is on `b`'s package, plus a test per remaining join once it is fixed. | qsl-semantics/tests/it/timed_model.rs:1075-1147; qsl-semantics/src/check/timed_model.rs:4351-4355 |
