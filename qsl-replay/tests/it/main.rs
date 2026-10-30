// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests that reach `qsl-replay` through its public facade only.

mod category_facade;
mod checked_body_failure;
mod compile_ieee_profile;
mod compile_protocol_profile;
mod format_outcome_facade;
mod hyper_identity;
mod hyper_query;
mod hyper_smt_settle;
mod outcome_facade;
mod parse_outcome_facade;
mod quantity_namespace_source;
mod quantity_type_source;
#[path = "../../../tests/support/refinement_aux.rs"]
mod refinement_aux;
#[path = "../../../tests/support/refinement_cas.rs"]
mod refinement_fixture;
mod refinement_hidden;
mod refinement_history;
mod refinement_mapping;
#[path = "../../../tests/support/refinement_smt_proofs.rs"]
mod refinement_proofs;
mod refinement_query_facade;
mod refinement_ring;
mod refinement_smt;
mod refinement_step;
mod refinement_unmapped;
#[path = "../../../tests/support/ring_world.rs"]
mod ring_world;
mod script_search;
mod vault_world;
