// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-eval`'s integration tests: the ones that exercise only ADR-011 §6.1
//! layer 5 and the layers below it, moved from the root crate's
//! `tests/it/`. One binary, one link step, the same shape as the root
//! crate's `it` target.

pub(crate) mod support;

#[path = "../../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(dead_code)]
mod tc235_k2;

mod automaton_kernels;
mod automaton_key;
mod call_verdicts;
mod checked_package_call;
mod child_composite;
mod clause_read_paths;
mod collection_algebra;
mod collection_queries;
mod composite_values;
mod dispatch_calls;
mod equality_matrix;
mod evaluation_ignores_display_strings;
mod fairness_kernels;
mod finite_simulation;
mod float_rounding;
mod footprints;
mod hyper_tuple;
mod inherited_attributes;
mod kernel_refusal_payloads;
mod machine_contexts;
mod model_reference_queries;
mod model_simulation;
mod named_population;
mod native_literals;
mod native_scalar_equality;
mod property_automata;
mod protocol_reference;
mod refinement_populations;
mod relationship_navigation;
mod relationship_reachability;
mod source_call;
mod state_model_family;
mod supplied_phases;
mod tc235_business_meaning_runtime;
mod temporal_trace;
mod total_functions;
mod type_invariants;
mod unbounded_set_call;
mod variant_fields;
mod weighted_sampler;
mod window_aggregates;
