// SPDX-License-Identifier: AGPL-3.0-or-later
//! Required selections and synthetic completion output for the process double.

pub(super) const CHECKS: [(&str, &str); 12] = [
    ("check::node_key::tests::conformance_fr322_application_keys_match_qspec_operation_vectors", "conformance: 1 of 1 QSpec operation vectors match (process double)"),
    ("check::node_key::tests::conformance_fr092_nominal_enum_keys_match_qspec_vectors", "conformance: 2 nominal enum vectors match (process double)"),
    ("quantities::tc_411_compound_unit_ids_match_qspec_vectors", "conformance: 1 compound-unit vectors"),
    ("checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures", "conformance: 1 source-map entries over 1 positive fixtures"),
    ("checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures", "conformance: 1 positive fixtures through QSL's full I2 read\nconformance: 1 adverse mutations refused"),
    ("checked_v2::tests::conformance_fr340_frame_mutations_match_qspec_vectors", "conformance: 1 frame-body mutation vectors matched"),
    ("checked_v2::tests::conformance_dependency_selection_vectors", "conformance: 1 dependency-selection entry mutations and 1 order vectors"),
    ("emit::tests::golden::conformance_emitted_application_nodes_match_qspec_positive_fixtures", "conformance: 1 emitted application nodes match QSpec's positive fixtures"),
    ("check::profile::tests::conformance_fr110_profile_causes_are_listed_by_qspec_native_diagnostics", "conformance: 1 profile (code, cause) pairs listed by QSpec"),
    ("library::bundle_tests::conformance_fr111_resolution_causes_are_listed_by_qspec_native_diagnostics", "conformance: 1 bundle (code, cause) pairs listed by QSpec"),
    ("complete_value_lock::conformance_catalog_matches_qspec_complete_value_lock", "conformance: 1 catalog rows match QSpec's lock"),
    ("complete_value_lock::conformance_admit_selection_matches_qspec_selection_vectors", "conformance: 1 accepted and 1 refused selection vectors"),
];
