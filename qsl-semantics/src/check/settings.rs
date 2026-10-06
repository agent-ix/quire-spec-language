// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255: the settings of the checking stage's own limits (`s3.*`) and of
//! type-environment admission (`types.*`). Both limits types live in
//! `quire-semantic-value`, so their field-to-setting mappings are these
//! functions, one pair per type, rather than a trait impl.

use qsl_foundation::Setting;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::TypeEnvironmentLimits;

/// Every field of `limits` with its setting and bound.
pub fn checking_bounds(limits: &CheckingLimits) -> Vec<(Setting, u64)> {
    vec![
        (Setting::S3Nodes, limits.nodes()),
        (Setting::S3InputBytes, limits.input_bytes()),
        (Setting::S3WorkUnits, limits.work_budget()),
    ]
}

/// Set the field of `limits` that `setting` names and change nothing else.
/// Returns `false`, changing nothing, for a setting this stage has no field
/// for.
pub fn set_checking_bound(limits: &mut CheckingLimits, setting: Setting, bound: u64) -> bool {
    match setting {
        Setting::S3Nodes => *limits = limits.with_nodes(bound),
        Setting::S3InputBytes => *limits = limits.with_input_bytes(bound),
        Setting::S3WorkUnits => *limits = limits.with_work_budget(bound),
        _ => return false,
    }
    true
}

/// Every field of `limits` with its setting and bound.
pub fn environment_bounds(limits: &TypeEnvironmentLimits) -> Vec<(Setting, u64)> {
    vec![
        (Setting::EnvironmentAncestorSteps, limits.ancestor_steps),
        (Setting::EnvironmentWorkUnits, limits.work_units),
    ]
}

/// Set the field of `limits` that `setting` names and change nothing else.
/// Returns `false`, changing nothing, for a setting this stage has no field
/// for.
pub fn set_environment_bound(
    limits: &mut TypeEnvironmentLimits,
    setting: Setting,
    bound: u64,
) -> bool {
    match setting {
        Setting::EnvironmentAncestorSteps => limits.ancestor_steps = bound,
        Setting::EnvironmentWorkUnits => limits.work_units = bound,
        _ => return false,
    }
    true
}
