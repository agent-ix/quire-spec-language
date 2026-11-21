// SPDX-License-Identifier: AGPL-3.0-or-later
//! External consumers of the non-core request-file owner.

mod check_imports;
mod dependency_handoff;
mod check_source_identity;
mod cli_contract;
mod compile_limits;
#[path = "../../../examples/config-version/spine_corpus.rs"]
pub mod config_version_spine;
mod extraction_request_refusals;
mod inspect_handoff_locus;
mod integer_division_request;
mod model_unit_request;
mod native_source_projection;
mod native_workflow_family_cases;
mod negotiation;
mod package_run;
mod prepared_command;
mod probabilistic_package_locus;
mod protocol_refusal_names;
mod protocol_refusal_text;
mod reaches_optional_reference;
mod request_overrides;
mod revision_validation;
mod run_requirement_items;
mod short_circuit_integer_division;
mod wire_schema;
