// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public API tests for the unreduced graph core and the model checker.

mod closure_certificate;
mod counter;
mod deadlock_item;
mod example_units;
mod fill;
mod flag_world;
mod graph_core;
mod graph_world;
mod hyper;
mod hyper_replay;
mod hyper_witness;
mod hyper_world;
mod lasso_replay;
mod litmus;
mod liveness;
mod model_check;
mod model_check_safety;
mod model_instances;
mod model_settle;
mod model_world;
mod partial_order;
mod por_world;
mod protocol_constructs;
mod protocol_footprints;
mod protocol_system;
mod quotient;
mod reduction;
mod reduction_differential;
mod reduction_world;
#[path = "../../../tests/support/refinement_aux.rs"]
mod refinement_aux;
mod refinement_auxiliary;
mod refinement_budget;
mod refinement_certificate;
#[path = "../../../tests/support/refinement_cas.rs"]
mod refinement_fixture;
mod refinement_reads;
mod refinement_replay;
mod refinement_safety;
#[path = "../../../tests/support/ring_world.rs"]
mod ring_world;
mod schedulability;
mod state_graph;
mod state_graph_certificate;
mod state_graph_settle;
mod strong_fairness;
mod strong_hint;
mod units;
mod universe_obligation;
mod weak_memory;
