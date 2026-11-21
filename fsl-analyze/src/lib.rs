// SPDX-License-Identifier: AGPL-3.0-or-later
//! `fsl-analyze`: the ADR-011 §6.1 layer **A** crate (ADR-011 §7.3 X-12,
//! ADR-029 CB-3) -- the in-process analysis engines, above the qualified
//! core. Nothing in layers K to 6, R or tool depends on it.
//!
//! [`analyze`] is the `analyze` operation (FR-281): it routes items to the
//! engines below by their manifests and settles each result.
//!
//! [`zone_check`] holds the native zone engine EN-6's difference-bound
//! matrices (ADR-026 EZ-2, FR-238). The zone certificate checker in
//! `fsl-replay` keeps its own DBM code and never calls this one (ADR-026
//! CF-6).
//!
//! [`statistical`] holds EN-4, statistical model checking of probabilistic
//! claims (ADR-024 ST-1, FR-189 to FR-191).
//!
//! [`exact_probabilistic`] holds EN-5's probabilistic product (FR-196).
//!
//! [`schedulability`] holds EN-7, the closed-form schedulability engine
//! (ADR-026 §12, FR-247).

#![forbid(unsafe_code)]

/// FR-286: the typed refusal pairs produced by analysis.
pub const CATALOG_PAIRS: &[fsl_foundation::diagnostic::CatalogCode] = &[
    fsl_foundation::diagnostic::CatalogCode::new("cancelled", "caller-cancelled"),
    fsl_foundation::diagnostic::CatalogCode::new("cardinality_out_of_bound", "above-maximum"),
    fsl_foundation::diagnostic::CatalogCode::new("integer_out_of_domain", "outside-domain"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "duplicate-backend"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "inapplicable-kind"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "inconsistent-candidates"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "invalid-option-value"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "model-alias"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "not-statistical"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "unknown-claim"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "unknown-option"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_result", "invalid-value"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "invalid-value"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "strict-clock-constraint"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "wrong-value-kind"),
    fsl_foundation::diagnostic::CatalogCode::new("missing_import", "missing-selection"),
    fsl_foundation::diagnostic::CatalogCode::new("missing_declaration", "missing-name"),
    fsl_foundation::diagnostic::CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
    fsl_foundation::diagnostic::CatalogCode::new("runtime_invariant", "established-invariant-broken"),
    fsl_foundation::diagnostic::CatalogCode::new("unavailable_observation", "missing-required-artifact"),
    fsl_foundation::diagnostic::CatalogCode::new("unknown_required_feature", "unsupported-feature"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "missing-member"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "unknown-member"),
    fsl_foundation::diagnostic::CatalogCode::new("unsupported_construct", "expression-form"),
    fsl_foundation::diagnostic::CatalogCode::new("unsupported_construct", "not-yet-implemented"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "wrong-role-mapping"),
    fsl_foundation::diagnostic::CatalogCode::new("unsupported_projection", "unbounded-extent"),
];

pub mod analyze;
pub mod claim_family;
pub mod contract_check;
pub mod exact_probabilistic;
pub mod model_check;
pub mod monitor;
pub mod monitor_entries;
pub mod monitor_plan;
pub mod monitor_root;
pub mod monitor_session;
pub mod plan;
pub mod plan_search;
pub mod plan_source;
pub mod protocol_analysis;
pub mod provider;
pub mod schedulability;
pub mod statistical;
pub mod step_simulation;
pub mod tie_rule;
pub mod value_check;
pub mod zone_check;

#[cfg(test)]
pub(crate) mod test_ids;
