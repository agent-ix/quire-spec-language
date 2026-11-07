// SPDX-License-Identifier: AGPL-3.0-or-later
//! Single integration-test binary. Every module below was
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
//! that crate's `test-support` feature in every test build, so no
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

const TEST_PACKAGE_NAME: &str = "root-integration-tests";

#[cfg(feature = "checked-handoff")]
mod selected_object_closure;

#[path = "../support/mod.rs"]
pub(crate) mod support;

mod abstraction_relation_inert;
mod architecture_fixture_spine;
mod assessment_artifact_kinds;
mod call_replay_context_items;
mod call_requirements_items;
mod clause_kind_canonical;
mod cli;
mod compile_command;
mod compiled_protocol_v2;
mod complete_cst;
mod complete_editor;
mod config_version_spine;
mod deep_sources;
mod dimension_node_v2;
mod dispatch_emission;
mod emit_checked_v2;
mod evaluate_reads_no_display_strings;
mod exact_decimals;
mod extension_s1_parse;
mod family_outcome_layering;
mod formal_source;
mod formatter_identity;
mod producer_unit_identity;
mod quantity_power_v2;
mod retired_checked_views_paths;
mod string_edge_marker;
mod value_parity_profile_selections;
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
mod checked_views;
mod compound_quantity_types;
mod fairness_item_path;
mod hybrid_request_items;
mod layer_crate_reexports;
mod limit_name_patterns;
mod linking;
#[cfg(feature = "checked-handoff")]
mod mapping_support;
#[cfg(feature = "checked-handoff")]
mod model_type_unit_roots;
mod name_resolution_confinement;
mod native_boundaries;
mod nesting_levels;
mod parser;
mod protocol_artifact;
mod protocol_artifact_versions;
mod protocol_number_root_module;
mod replay_multi_source;
mod request_builder;
mod requirement_item_proof_bounds;
mod retained_criteria_matrix;
mod retired_mapped_extraction;
mod retired_native_runtime;
mod seam5_retired;
mod source_map;
mod spine_deletion_boundaries;
mod spine_retained_criteria;
mod spine_run;
mod temporal_negotiation;
#[cfg(feature = "checked-handoff")]
mod temporal_package_route;
#[cfg(feature = "checked-handoff")]
mod temporal_package_route_formula;
#[cfg(feature = "checked-handoff")]
mod temporal_result_evidence;
mod temporal_v1_retired;
mod text_enum_identity;
mod unbounded_declarations;
mod union_nodes;
mod v2_refusal_code_bridge;
