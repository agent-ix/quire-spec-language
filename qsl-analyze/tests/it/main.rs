// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public API tests for the unreduced graph core and the model checker.

mod closure_certificate;
mod counter;
mod deadlock_item;
mod example_units;
mod flag_world;
mod graph_core;
mod graph_world;
mod lasso_replay;
mod liveness;
mod model_check;
mod model_check_safety;
mod model_instances;
mod model_settle;
mod model_world;
mod quotient;
mod reduction;
mod reduction_world;
mod refinement_budget;
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
mod units;
mod universe_obligation;
