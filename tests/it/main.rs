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
//! Modules that call `qsl-semantics`' test-only fixtures
//! (`DeclarationKey::fixture`, `ReaderAuthority::fixture` and friends) reach
//! them through this crate's `qsl-semantics` dev-dependency, which enables
//! that crate's `test-support` feature in every test build (QSL-181), so no
//! `mod` line here is feature-gated. The integration tests that exercise
//! only layer 3 live in `qsl-semantics/tests/it/`.
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
mod clause_kind_canonical;
mod cli;
mod compile_command;
mod compiled_protocol_v2;
mod complete_cst;
mod complete_editor;
mod complete_package;
mod complete_v1_plan;
mod composed_admission_stages;
mod composed_binding;
mod composed_definition_source;
mod composed_definitions;
mod composed_domain_models;
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
mod config_version;
mod configversion_backends;
mod contract_model_architecture;
mod domain_protocol_emission;
mod exact_decimals;
mod extracted_command;
mod family_outcome_layering;
mod fixture_audit;
mod formal_source;
// QSL-251: the handoff writer is behind its feature; the all-features lane runs this.
#[cfg(feature = "handoff-writer")]
mod handoff_writer;
mod integer_lowering;
mod layer_crate_reexports;
mod linking;
mod located_json;
mod lower_command;
mod lowering_registry_isolation;
mod mapped;
mod model_source;
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
mod nesting_levels;
mod occurrence_key_schema;
mod package_construction;
mod package_reading;
mod package_runtime;
mod parser;
mod parser_differential;
mod protocol_artifact;
mod protocol_number;
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
mod verified_binding_witness;
