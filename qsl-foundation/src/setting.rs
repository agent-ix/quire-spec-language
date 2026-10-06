// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255: the one stable setting name of every configurable resource limit.
//!
//! [`Setting`] is the closed set of names QSL accepts at every entry point:
//! a limits type's builder, a replay request's `stage_limits` entry and the
//! settings operation behind the driver's `--limit <name>=<value>`. Each
//! setting fixes its dotted name, the stage that owns it and the
//! [`LimitKind`] its outcome reports, so a producer of a limit outcome names
//! the setting and gets its kind from the same row.
//!
//! A limits type reports its fields through [`SettingLimits`]: each field has
//! exactly one setting, and [`SettingLimits::set_bound`] changes that field
//! and nothing else. Every bound is used as given, with no ceiling.

use crate::diagnostic::LimitKind;

/// Declares [`Setting`] and its table from one list, so a name, its stage and
/// its limit kind are written once.
macro_rules! settings {
    ($($variant:ident => $name:literal, $stage:literal, $kind:ident;)+) => {
        /// A configurable resource limit's stable setting name (FR-255).
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum Setting {
            $(
                #[doc = concat!("`", $name, "`.")]
                $variant,
            )+
        }

        impl Setting {
            /// Every setting, in FR-255's table order.
            pub const ALL: &'static [Setting] = &[$(Self::$variant),+];

            /// The setting's dotted name, such as `s3.nodes`.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }

            /// The stage that owns the limit, as FR-255's table spells it.
            #[must_use]
            pub const fn stage(self) -> &'static str {
                match self {
                    $(Self::$variant => $stage,)+
                }
            }

            /// The limit kind an outcome for this setting reports.
            #[must_use]
            pub const fn kind(self) -> LimitKind {
                match self {
                    $(Self::$variant => LimitKind::$kind,)+
                }
            }
        }
    };
}

settings! {
    S1InputBytes => "s1.input_bytes", "S1", InputBytes;
    S1Tokens => "s1.tokens", "S1", TokenCount;
    S1Nodes => "s1.nodes", "S1", NodeCount;
    S1WorkUnits => "s1.work_units", "S1", WorkBudget;
    S3Nodes => "s3.nodes", "S3", NodeCount;
    S3InputBytes => "s3.input_bytes", "S3", InputBytes;
    S3WorkUnits => "s3.work_units", "S3", WorkBudget;
    S3DecimalScale => "s3.decimal_scale", "S3 assembly", WorkBudget;
    EnvironmentAncestorSteps => "environment.ancestor_steps", "S3 type-environment admission", EdgeCount;
    EnvironmentWorkUnits => "environment.work_units", "S3 type-environment admission", WorkBudget;
    ModelDeclarationRecords => "model.declaration_records", "model normalization", NodeCount;
    ModelDerivationFacts => "model.derivation_facts", "model normalization", NodeCount;
    ModelEffectiveDeclarations => "model.effective_declarations", "model normalization", NodeCount;
    ModelDispatchCandidates => "model.dispatch_candidates", "model normalization", NodeCount;
    ModelHashedBytes => "model.hashed_bytes", "model normalization", InputBytes;
    ModelWorkUnits => "model.work_units", "model normalization", WorkBudget;
    ModelAncestorSteps => "model.ancestor_steps", "model normalization", EdgeCount;
    ModelFamilySteps => "model.family_steps", "model normalization", EdgeCount;
    AdmissionPopulationMembers => "admission.population_members", "population admission", NodeCount;
    AdmissionWorkUnits => "admission.work_units", "population admission", WorkBudget;
    AdmissionAncestorSteps => "admission.ancestor_steps", "population admission", EdgeCount;
    ObservationInputBytes => "observation.input_bytes", "observation admission", InputBytes;
    ObservationObjects => "observation.objects", "observation admission", NodeCount;
    ObservationValues => "observation.values", "observation admission", NodeCount;
    DependencyLibraries => "dependency.libraries", "S4 source resolution", NodeCount;
    DependencyImportEdges => "dependency.import_edges", "S4 source resolution", EdgeCount;
    DependencySourceBytes => "dependency.source_bytes", "S4 source resolution", InputBytes;
    LibraryDefinitions => "library.definitions", "library resolution", NodeCount;
    LibraryDependencyEdges => "library.dependency_edges", "library resolution", EdgeCount;
    LibraryArtifactBytes => "library.artifact_bytes", "library resolution", InputBytes;
    LibrarySingleArtifactBytes => "library.single_artifact_bytes", "library resolution", InputBytes;
    IdentityInputBytes => "identity.input_bytes", "identity encoding", InputBytes;
    ReplayInputBytes => "replay.input_bytes", "replay envelope readers", InputBytes;
    I2InputBytes => "i2.input_bytes", "I2 v2 reader", InputBytes;
    I2Nodes => "i2.nodes", "I2 v2 reader", NodeCount;
    I2Edges => "i2.edges", "I2 v2 reader", EdgeCount;
    I2Occurrences => "i2.occurrences", "I2 v2 reader", OccurrenceCount;
    I2Diagnostics => "i2.diagnostics", "I2 v2 reader", DiagnosticCount;
    I2WorkUnits => "i2.work_units", "I2 v2 reader", WorkBudget;
    ExploreStates => "explore.states", "exploration", NodeCount;
    ExploreTransitions => "explore.transitions", "exploration", EdgeCount;
}

impl Setting {
    /// The setting named `name`, or `None` when no setting has that name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|setting| setting.name() == name)
    }
}

impl std::fmt::Display for Setting {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// A limits type: the settings its fields carry, and the one place that maps
/// each field to its setting (FR-255 Behavior 2).
pub trait SettingLimits {
    /// Every field of this type with its setting and its current bound. Each
    /// setting appears at most once, and no two types report the same one.
    fn bounds(&self) -> Vec<(Setting, u64)>;

    /// Set the bound of the field `setting` names and change nothing else.
    /// Returns `false`, changing nothing, when this type has no such field.
    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool;
}

/// Why a `<name>=<value>` operand of the settings operation was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UsageCause {
    /// The operand has no `=`.
    NotAnAssignment,
    /// The name is not a setting of FR-255's table.
    UnknownSetting,
    /// The value is not a non-negative decimal integer that fits a bound.
    NotAnInteger,
    /// The setting was given more than once.
    Repeated,
}

/// The settings operation's usage refusal: it names the offending operand and
/// no stage runs (FR-255 Behavior 6).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("usage: {cause:?} in `--limit {operand}`")]
pub struct UsageRefusal {
    /// The operand as the caller wrote it.
    pub operand: String,
    /// Why it was refused.
    pub cause: UsageCause,
}

/// Parse the `<name>=<value>` operands of the settings operation into
/// settings and bounds, in operand order.
///
/// # Errors
///
/// [`UsageRefusal`] naming the first operand that is not `<name>=<value>`,
/// names no setting, holds a value that is not a non-negative decimal integer
/// of at most `u64::MAX`, or names a setting an earlier operand named.
pub fn parse_operands<'a>(
    operands: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<(Setting, u64)>, UsageRefusal> {
    let mut parsed: Vec<(Setting, u64)> = Vec::new();
    for operand in operands {
        let refuse = |cause| UsageRefusal {
            operand: operand.to_owned(),
            cause,
        };
        let (name, value) = operand
            .split_once('=')
            .ok_or_else(|| refuse(UsageCause::NotAnAssignment))?;
        let setting = Setting::from_name(name).ok_or_else(|| refuse(UsageCause::UnknownSetting))?;
        let bound = if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
            value
                .parse::<u64>()
                .map_err(|_| refuse(UsageCause::NotAnInteger))?
        } else {
            return Err(refuse(UsageCause::NotAnInteger));
        };
        if parsed.iter().any(|(earlier, _)| *earlier == setting) {
            return Err(refuse(UsageCause::Repeated));
        }
        parsed.push((setting, bound));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::{parse_operands, Setting, UsageCause};

    #[test]
    fn names_are_distinct_and_round_trip() {
        for (index, setting) in Setting::ALL.iter().enumerate() {
            assert_eq!(Setting::from_name(setting.name()), Some(*setting));
            assert!(
                Setting::ALL[index + 1..]
                    .iter()
                    .all(|other| other.name() != setting.name()),
                "{setting}"
            );
        }
    }

    #[test]
    fn operands_parse_in_order_and_refuse_the_first_bad_one() {
        assert_eq!(
            parse_operands(["s3.nodes=5", "s1.tokens=0"]),
            Ok(vec![(Setting::S3Nodes, 5), (Setting::S1Tokens, 0)])
        );
        for (operand, cause) in [
            ("s9.nodes=1", UsageCause::UnknownSetting),
            ("s3.nodes=ten", UsageCause::NotAnInteger),
            ("s3.nodes=-1", UsageCause::NotAnInteger),
            ("s3.nodes=+1", UsageCause::NotAnInteger),
            ("s3.nodes=", UsageCause::NotAnInteger),
            ("s3.nodes=18446744073709551616", UsageCause::NotAnInteger),
            ("s3.nodes", UsageCause::NotAnAssignment),
        ] {
            let refusal = parse_operands([operand]).unwrap_err();
            assert_eq!((refusal.operand.as_str(), refusal.cause), (operand, cause));
        }
        let repeated = parse_operands(["s3.nodes=5", "s3.nodes=6"]).unwrap_err();
        assert_eq!(repeated.operand, "s3.nodes=6");
        assert_eq!(repeated.cause, UsageCause::Repeated);
    }
}
