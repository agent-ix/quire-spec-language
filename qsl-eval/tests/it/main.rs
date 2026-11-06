// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-eval`'s integration tests: the ones that exercise only ADR-011 §6.1
//! layer 5 and the layers below it, moved from the root crate's
//! `tests/it/`. One binary, one link step, the same shape as the root
//! crate's `it` target.

pub(crate) mod support;

/// The package name test fixtures assemble under.
const TEST_PACKAGE_NAME: &str = "qsl-eval-integration-tests";

#[path = "../../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(dead_code)]
mod tc235_k2;

mod absent_literal;
mod accounting_boundaries;
mod aggregate_samples;
mod application_observation;
mod automaton_kernels;
mod automaton_key;
mod automaton_key_bounded;
mod call_verdicts;
mod canonical_node_probe;
mod checked_package_call;
mod checked_route_owner;
mod checked_route_trigger;
mod child_composite;
mod clause_read_paths;
mod collection_algebra;
mod collection_queries;
mod composite_values;
mod compound_conversion_frame;
mod decimal_source_forms;
mod dispatch_calls;
mod duplicate_preserving_queries;
mod equality_matrix;
mod evaluation_ignores_display_strings;
mod fairness_kernels;
mod family_sums;
mod finite_simulation;
mod finite_window;
mod float_callables;
mod float_rounding;
mod footprints;
mod hyper_tuple;
mod ieee_node_charges;
mod inherited_attributes;
mod int_arithmetic_expected_type;
mod kernel_refusal_payloads;
mod machine_contexts;
mod model_reference_queries;
mod model_simulation;
mod named_population;
mod native_literals;
mod native_scalar_equality;
mod one_occurrence;
mod package_aggregate_route;
mod property_automata;
mod protocol_delivery_interval;
mod protocol_reference;
mod protocol_retry_bound;
mod protocol_slots;
mod quantity_literals;
mod quantity_power;
mod reference_adapter;
mod refinement_populations;
mod relationship_navigation;
mod relationship_reachability;
mod removal_filter_decrease;
mod smt_query;
mod source_call;
mod state_model_family;
mod supplied_phases;
mod tc235_business_meaning_runtime;
mod temporal_activation;
mod temporal_aggregate_precedence;
mod temporal_causes;
mod temporal_evidence;
mod temporal_identity;
mod temporal_relation;
mod temporal_rulings;
mod temporal_trace;
mod tick_monitor;
mod timed_subject;
mod timed_trace;
mod total_functions;
mod type_invariants;
mod unbounded_set_call;
mod union_value_profile;
mod variant_fields;
mod weighted_sampler;
mod window_aggregates;
mod witnessed_environment;
