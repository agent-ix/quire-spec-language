---
id: TC-1277
title: "Evaluate temporal obligations through checked-package admission"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: verifies
---
# TC-1277: Evaluate temporal obligations through checked-package admission

## Description

Checked-route successor controls for FR-043 and FR-044. Unlike TC-122 and
TC-123, these cases compile native source through S1-S4, select temporal
declarations and formulas from the resulting `CheckedPackage`, and evaluate
those checked nodes directly with independently constructed traces. TC-122 and
TC-123 remain the legacy emitted-artifact controls. The checked route does not
relabel their procedures or replace their evidence.

The executable controls are `tests/it/checked_temporal_successors.rs` and the
strict version-2 authentication control in
`tests/it/compiled_protocol_v2.rs`. Each checked formula test derives its
valuation handles from the checked graph and supplies its expected result
independently of the evaluator's output.

## Test Procedure

1. Run `checked_package_formula_uses_the_shared_bounded_evaluator` and
   `checked_profiles_keep_distinct_closed_boundary_results`. Require checked
   formula semantics to distinguish a `holds` atom from a constant and to
   retain each selected profile identity and its closed-boundary result.
2. Run `checked_binding_and_missing_fact_keep_distinct_outcomes`. Mutate the
   checked declaration's asserted profile and clock binding independently;
   require a typed binding refusal before position work. Also omit a required
   valuation and require unavailable/incomplete evidence instead of false.
3. Run `checked_temporal_zero_width_previous_and_duals_are_preserved`,
   `checked_until_and_release_share_the_inclusive_lower_bound_convention`, and
   `checked_formula_normalization_preserves_all_operator_families`. Require
   inclusive zero-width and nonzero lower bounds, strong `once[1,1]`, the
   `until[1,2]` witness at offset one, and release/triggered agreement with
   their Boolean duals on the same checked traces.
4. Run `checked_order_sensitive_formula_refuses_unordered_equal_coordinates`.
   Require refusal for equal coordinates with missing or foreign order
   authority, evaluation with one admitted order, and invariance under reversed
   input order.
5. Run `checked_atom_has_exact_decision_support_and_false_extension` and the
   pointwise-connective cases in
   `checked_formula_normalization_preserves_all_operator_families`. Require
   exact support and pointwise temporal Boolean connectives.
6. Run `checked_open_incomplete_and_empty_window_outcomes_stay_distinct`.
   Require closed, open decisive, open pending, unavailable, incomplete and
   finite-empty-window outcomes to remain distinct. Inspect the result premises
   and require decision closure, surrounding closure, assessment execution and
   input completeness to retain their independent supplied values.
7. Run `checked_past_operator_requires_authoritative_history`. Compare an
   authoritative origin with the identical history cutoff and omit an interior
   history interval; require the exact incomplete/refusal outcome.
8. Run `checked_watermark_deadline_and_silent_clock_semantics`. Require
   fixed-sample and timestamped watermarks to settle inclusive deadlines,
   deadline instants to participate, later instants to be excluded, and
   event-position silence not to advance the clock.
9. Run `checked_progress_is_keyed_by_checked_clause_identity` and
   `checked_repeated_results_retain_identity_inputs`. Require progress to be
   isolated by checked clause identity, changed decision-scope assertions to
   return a typed contradiction without replacing retained progress, identical
   inputs to reproduce an equal assessment, and changed profile or ceiling
   premises to remain distinguishable.
10. Run
    `native_v2_emission_reaches_the_strict_reader_and_authenticated_l5_adapter`
    from `tests/it/compiled_protocol_v2.rs`. Require the exact profile, clock
    and parameter binding to authenticate before evaluation and retain its
    package, declaration and definition identities; each mutated binding must
    refuse without truth or authenticated evidence.

## Expected Results

Every checked temporal evaluation derives truth, settlement, support, retained
premises and activation identity from the checked package and admitted trace.
Typed incomplete and refused outcomes remain distinct from Boolean truth.
Legacy TC-122 and TC-123 continue to describe only their emitted-artifact
procedures; this test case is the explicit checked-route successor.
