// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-175 (#306): single integration-test binary. Every module below was
//! formerly its own `tests/<name>.rs` cargo test target, statically linking
//! the whole `quire-spec-language` crate plus its dependencies on its own
//! (measured, #306: 426 test executables across 109 targets took 106.6 GiB
//! in one worktree's target dir, against 2.7 GiB for every compiled
//! library). Each is now a `mod` of this one `it` target instead, so
//! `cargo test` pays one link step, not ~105 of them.
//!
//! Shared helpers live in `tests/support/` and are reached as
//! `crate::support::<name>` from any module below (see `tests/support/
//! mod.rs`). `tests/fixtures/` is unchanged.
//!
//! The seven modules gated on `feature = "test-support"` call fixture
//! constructors (`DeclarationKey::fixture` and friends, see the crate's own
//! `Cargo.toml`) that only exist under that feature. Gating the `mod` line
//! itself -- not just the calls inside -- keeps a default-feature `cargo
//! test` / `cargo clippy --all-targets` from trying to compile them at all,
//! matching the seven previous `required-features = ["test-support"]`
//! per-file targets they replace (Cargo.toml's own `[[test]] name = "it"`
//! carries no `required-features`).
//!
//! No test here needs its own process. There is no `trybuild` compile-fail
//! test, no process-wide `std::env::set_var`/`remove_var`, and no `harness =
//! false` target under `tests/` (verified by grep across every file this
//! module absorbs); every test that spawns a subprocess does so through
//! `std::process::Command` with its own `.current_dir(...)`/args, which does
//! not touch this process's own state and is unaffected by running
//! alongside every other module here.

#[path = "../support/mod.rs"]
pub(crate) mod support;

mod assessment_artifact_kinds;
mod checked_package_call;
mod clause_kind_canonical;
mod cli;
mod collection_algebra;
mod collection_queries;
mod compile_command;
mod compiled_protocol_v2;
mod complete_cst;
mod complete_editor;
mod complete_v1_plan;
mod complete_value_lock;
mod composed_admission_stages;
mod composed_binding;
mod composed_definition_source;
mod composed_definitions;
mod composed_linking;
mod composed_models;
mod composed_namespace;
mod composed_proofs;
mod composed_query_proofs;
mod composed_scopes;
mod composed_state_evaluation;
mod composed_syntax;
mod composed_temporal_activation;
mod composed_temporal_evaluation;
mod composed_temporal_limits;
mod composed_temporal_mapping;
mod composed_temporal_mapping_v2;
mod composed_type_pipeline;
mod composed_types;
mod composite_values;
mod config_version;
mod configversion_backends;
mod contract_model_architecture;
#[cfg(feature = "test-support")]
mod dispatch_calls;
mod equality_matrix;
mod exact_decimals;
mod extracted_command;
mod finite_simulation;
mod fixture_audit;
mod formal_source;
mod ieee_profiles;
mod import_view_names;
mod integer_division;
mod integer_lowering;
mod library_resolution;
mod linking;
mod located_json;
mod lower_command;
mod lowering_registry_isolation;
mod mapped;
#[cfg(feature = "test-support")]
mod model_conformance;
#[cfg(feature = "test-support")]
mod model_dispatch;
mod model_intake;
#[cfg(feature = "test-support")]
mod model_normalization;
#[cfg(feature = "test-support")]
mod model_population;
#[cfg(feature = "test-support")]
mod model_reference_queries;
mod model_source;
#[cfg(feature = "test-support")]
mod model_systems;
mod name_resolution_confinement;
mod native_backend;
mod native_boundaries;
mod native_checking;
mod native_choice_emission;
mod native_compensation_emission;
mod native_composite_visible_choices;
mod native_composite_visible_truth_tables;
mod native_domain_event_boundaries;
mod native_domain_event_choices;
mod native_linking;
mod native_lowering;
mod native_mixed_observation_choices;
mod native_model;
mod native_model_profiles;
mod native_model_qualification;
mod native_numeric_domain_emission;
mod native_owned_attempt_progress;
mod native_payment_retry_emission;
mod native_population_emission;
mod native_protocol_emission;
mod native_query_emission;
mod native_refund_emission;
mod native_split_shipment_emission;
mod native_temporal_owner;
mod occurrence_key_schema;
mod package_construction;
mod package_reading;
mod package_runtime;
mod parser;
mod protocol_artifact;
mod protocol_number;
mod quantities;
mod route_registry;
mod runtime_evaluation;
mod runtime_execution;
mod runtime_inputs;
mod runtime_reading;
mod runtime_validation;
mod runtime_workflow;
mod seam5_retired;
mod source_map;
mod standalone;
mod state_scalar_lowering;
mod text_enum_identity;
mod total_functions;
mod verified_binding_witness;
