// SPDX-License-Identifier: AGPL-3.0-or-later
//! Required selections and synthetic completion output for the process double.

pub(super) const CHECKS: [(&str, &str); 6] = [
    ("checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures", "conformance: 1 source-map entries over 1 positive fixtures"),
    ("checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures", "conformance: 1 positive fixtures through QSL's full I2 read\nconformance: 1 adverse mutations refused"),
    ("checked_v2::tests::conformance_fr340_frame_mutations_match_qspec_vectors", "conformance: 1 frame-body mutation vectors matched"),
    ("checked_v2::tests::conformance_dependency_selection_vectors", "conformance: 1 dependency-selection entry mutations and 1 order vectors"),
    ("emit::tests::golden::conformance_emitted_application_nodes_match_qspec_positive_fixtures", "conformance: 1 emitted application nodes match QSpec's positive fixtures"),
    ("complete_value_lock::conformance_admit_selection_matches_qspec_vectors", "conformance: 1 accepted and 1 refused selection vectors"),
];
