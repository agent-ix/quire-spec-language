// SPDX-License-Identifier: AGPL-3.0-or-later
//! Single integration-test binary. Every module below was
//! formerly its own `tests/<name>.rs` cargo test target, statically linking
//! the whole `filament-spec-language` crate plus its dependencies on its own
//! (measured, #306: 426 test executables across 109 targets took 106.6 GiB
//! in one worktree's target dir, against 2.7 GiB for every compiled
//! library). Each is now a `mod` of this one `it` target instead, so
//! `cargo test` pays one link step, not ~105 of them.
//!
//! Shared helpers live in `tests/support/` and are reached as
//! `crate::support::<name>` from any module below (see `tests/support/
//! mod.rs`). `tests/fixtures/` is unchanged.
//!
//! Modules that call `fsl-semantics`' test-only fixtures
//! (`DeclarationKey::fixture`, `ReaderAuthority::fixture` and friends) reach
//! them through this crate's `fsl-semantics` dev-dependency, which enables
//! that crate's `test-support` feature in every test build, so no
//! `mod` line here is feature-gated. The integration tests that exercise
//! only layer 3 live in `fsl-semantics/tests/it/`.
//!
//! No test here needs its own process. There is no `trybuild` compile-fail
//! test, no process-wide `std::env::set_var`/`remove_var`, and no `harness =
//! false` target under `tests/` (verified by grep across every file this
//! module absorbs); every test that spawns a subprocess does so through
//! `std::process::Command` with its own `.current_dir(...)`/args, which does
//! not touch this process's own state and is unaffected by running
//! alongside every other module here.

const TEST_PACKAGE_NAME: &str = "root-integration-tests";

mod command_harness;
mod compile_command;
mod no_binary;
mod parse_operation;
#[cfg(feature = "checked-handoff")]
mod probability_source_routes;
#[cfg(feature = "checked-handoff")]
mod protocol_source_routes;
mod refinement_source_routes;
mod source_corpus;

#[cfg(feature = "checked-handoff")]
mod selected_object_closure;

pub(crate) mod negotiation;
#[path = "../support/mod.rs"]
pub(crate) mod support;

#[path = "../../fsl-semantics/tests/support/reward_evaluator.rs"]
mod reward_evaluator;

mod abstraction_relation_inert;
mod architecture_fixture_spine;
mod call_replay_context_items;
mod call_requirements_items;
mod check_imports;
mod checked_clause_kind_unique;
mod checked_package_state_core;
#[cfg(feature = "checked-handoff")]
mod checker_value_boundaries;
mod clock_derivation;
mod clock_reach;
mod clock_selection;
mod compile_package_output;
mod complete_cst;
mod complete_editor;
mod composition_planner;
mod config_version_spine;
mod control_node_lowered_type;
mod diagnostic_golden;
mod dimension_node_v2;
mod dispatch_emission;
mod division_law_editor;
mod doc_snippets;
mod effect_boundary;
mod emit_checked_v2;
mod emit_readback_agreement;
mod error_producer_inputs;
mod evaluate_reads_no_display_strings;
mod evidence_ledger_file;
mod exact_decimals;
mod export_rulings;
mod export_state_lowering;
mod export_surface;
mod export_temporal_lowering;
mod extension_forms;
mod extension_s1_parse;
mod family_outcome_layering;
mod first_rule_readme;
mod formal_source;
mod formatter_identity;
mod forms_package_contracts;
mod forms_refusal_cause;
mod function_claim_selection;
mod guarantee_planner;
mod half_open_watermark;
mod observation_model_header;
mod outcome_diagnostic_details;
mod package_run_route;
mod plan_export_surface;
mod planner_chain;
mod port_role_identity;
mod producer_unit_identity;
mod quantity_negation_v2;
mod quantity_power_v2;
mod record_sample_counterexample;
mod replay_request_file;
mod string_edge_marker;
mod value_parity_profile_selections;
mod workflow_population_aggregates;
mod workflow_relationship_navigation;
// The handoff writer is behind its feature; the all-features lane runs this.
#[cfg(feature = "checked-handoff")]
mod bare_equality;
#[cfg(feature = "checked-handoff")]
mod bare_package;
#[cfg(feature = "checked-handoff")]
mod bare_recovery_binder;
#[cfg(feature = "checked-handoff")]
mod checked_handoff;
#[cfg(feature = "checked-handoff")]
mod checked_handoff_healthy;
#[cfg(feature = "checked-handoff")]
mod checked_handoff_realizability;
#[cfg(feature = "checked-handoff")]
mod checked_views;
mod compound_quantity_types;
mod fairness_item_path;
mod hybrid_request_items;
mod layer_crate_reexports;
mod lifecycle_boundaries;
mod limit_name_patterns;
mod lowering_intake_boundaries;
#[cfg(feature = "checked-handoff")]
mod mapping_support;
mod mapping_support_occurrences;
mod mltl_differential;
#[cfg(feature = "checked-handoff")]
mod model_type_unit_roots;
mod modulo_claim_bounds;
mod name_resolution_confinement;
mod native_boundaries;
mod native_run_result_root;
mod replay_library_clocks;
mod replay_multi_source;
mod replay_refinement_internals;
mod request_builder;
mod request_surface_s1;
mod request_surface_s1_16;
mod requirement_item_proof_bounds;
mod retained_criteria_matrix;
mod retired_mapped_extraction;
mod retired_native_runtime;
mod run_items;
mod run_value_spellings;
mod run_values;
mod run_wire;
mod seam5_retired;
mod source_map;
#[cfg(feature = "checked-handoff")]
mod source_object_emission;
mod spine_deletion_boundaries;
mod spine_retained_criteria;
mod spine_run;
mod standard_pin;
mod synthesis_request;
mod synthesis_verify_check;
mod temporal_capture_run;
mod temporal_negotiation;
#[cfg(feature = "checked-handoff")]
mod temporal_package_route;
#[cfg(feature = "checked-handoff")]
mod temporal_package_route_formula;
#[cfg(feature = "checked-handoff")]
mod temporal_result_evidence;
mod temporal_selection_members;
mod temporal_uncertain_late;
mod temporal_v1_retired;
mod text_enum_identity;
mod text_size_v2;
mod timed_clock_unit;
mod typed_size_run;
mod unbounded_declarations;
mod union_nodes;
mod value_invariants_run;
mod verb_limits;
