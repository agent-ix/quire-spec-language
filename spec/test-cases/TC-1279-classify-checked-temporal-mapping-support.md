---
id: TC-1279
title: "Classify temporal mapping support from checked declarations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: verifies
---
# TC-1279: Classify temporal mapping support from checked declarations

## Description

Checked-route successor controls for FR-045. Each case compiles native source
through S1-S4 and classifies the selected temporal declaration from the
`CheckedPackage` graph, profile and requested surrounding-execution closure.
TC-125 remains the legacy admitted-artifact procedure and is not relabeled by
this successor.

Executable controls are in `tests/it/checked_temporal_successors.rs`. A
classification is tested by exact disposition, target, outstanding premises
and retained checked selection identity.

## Test Procedure

1. Run `checked_mapping_exercises_each_supported_row_and_past_operator`.
   Exercise every checked support-table row: all four profiles, mixed
   directions, supported past and future forms, and both surrounding-execution
   closures. Require each exact target or unsupported dimension.
2. Run `checked_mapping_accumulates_finite_window_and_direction_limits` and
   `checked_mapping_exercises_each_supported_row_and_past_operator`. Require
   timestamped-window and mixed-direction dimensions to accumulate, and every
   checked past operator under both false-extension profiles to select the
   origin-complete-history target.
3. Run `checked_mapping_accumulates_finite_window_and_direction_limits` and
   `checked_mapping_retains_the_checked_selection`. Require unsupported results
   to retain the checked declaration identity, authored name, selected profile
   and activation; classification must not mutate the checked source.
4. Run `checked_mapping_accumulates_finite_window_and_direction_limits`.
   Require a timestamped mixed-direction request to name both unmatched
   dimensions, not a summary cause.
5. Run `checked_mapping_exercises_each_supported_row_and_past_operator` and
   `checked_mapping_retains_the_checked_selection`. Require supported results
   to retain the exact target and outstanding bridge premises without claiming
   correspondence; equal source bodies under distinct profiles remain
   differently classified.
6. Run `checked_mapping_retains_the_checked_selection` and
   `checked_mapping_refuses_an_unselected_clause_index`. Require the selected
   declaration's checked node identity, profile and activation to match the
   checked graph and require an out-of-package index to return a located typed
   refusal instead of a classification.

## Expected Results

The selected checked declaration is the sole authority for classification.
Supported and unsupported results retain the checked selection; invalid
selection is a typed refusal. TC-125 remains legacy emitted-artifact evidence.
