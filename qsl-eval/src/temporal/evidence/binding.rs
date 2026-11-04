// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-522: the profile, clock binding name and clock parameters a
//! clock-bearing request asserts, compared with the checked clause's own
//! selection before any position is visited.
//!
//! The checked package is the authentication: the profile is the lock's
//! `profile_selections` row of the clause and the clock is the
//! `temporal_bindings` row of its declaration (FR-515, FR-050). The
//! comparison substitutes, widens, defaults and nearest-matches nothing.

use qsl_package::{
    CheckedArtifactRef, CheckedNodeId, CheckedSelection, ClockConfiguration, ClockExactNumber,
    TemporalBinding,
};
use quire_semantic_value::catalog::CatalogCode;

/// The stage a refused binding is reported at.
pub const ADMIT_STAGE: &str = "admit";

/// The profile, clock name and clock parameters a request asserts (FR-522
/// "Inputs"). It is a required member of every request whose quantities are
/// ticks of a clock; position-index traces carry none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceBinding {
    /// The profile identity the trace asserts.
    pub profile: Box<str>,
    /// The clock binding name the trace asserts.
    pub clock_name: Box<str>,
    /// The clock configuration the trace asserts.
    pub parameters: ClockConfiguration,
}

/// The first member, in FR-522's order, that a binding asserts differently
/// from the clause's selection.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MismatchedDimension {
    /// The profile identity.
    Profile,
    /// The clock binding name, or a clause with no clock row.
    Clock,
    /// The clock configuration is another alternative than the row's.
    ClockParameters,
    /// A fixed-sample epoch.
    Epoch,
    /// A fixed-sample period.
    SamplePeriod,
    /// A fixed-sample unit.
    ClockUnit,
    /// An event-position sequence authority.
    SequenceAuthority,
    /// A timestamped-event timestamp unit.
    TimestampUnit,
}

impl MismatchedDimension {
    /// The dimension's name as the refusal's `mismatched_dimension` field
    /// spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Profile => "Profile",
            Self::Clock => "Clock",
            Self::ClockParameters => "ClockParameters",
            Self::Epoch => "Epoch",
            Self::SamplePeriod => "SamplePeriod",
            Self::ClockUnit => "ClockUnit",
            Self::SequenceAuthority => "SequenceAuthority",
            Self::TimestampUnit => "TimestampUnit",
        }
    }
}

/// A refused binding (FR-522 "Outputs"): stage `admit`,
/// `invalid_runtime_input`/`invalid-value`, with the differing dimension and
/// the asserted and expected values. It holds no outcome, no truth and no
/// authenticated binding.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("the trace binding differs from the clause's in {dimension:?}: asserted {asserted}")]
pub struct BindingMismatch {
    /// The first differing dimension.
    pub dimension: MismatchedDimension,
    /// The value the request asserted.
    pub asserted: String,
    /// The value the clause's row holds; `None` when the clause has no
    /// `temporal_bindings` row.
    pub expected: Option<String>,
}

impl BindingMismatch {
    /// The stage the refusal is reported at: `admit`.
    pub const fn stage(&self) -> &'static str {
        ADMIT_STAGE
    }

    /// The catalog code, `invalid_runtime_input`/`invalid-value`.
    pub fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("invalid_runtime_input", "invalid-value")
    }
}

/// The checked package's facts about one temporal clause that authenticate a
/// request: what FR-515's `temporal_subject` view returns from `profile()`
/// and `clock()`, and the clause's declaration.
#[derive(Clone, Copy, Debug)]
pub struct ClauseSelection<'p> {
    /// The clause's declaration node.
    pub declaration: &'p CheckedNodeId,
    /// The lock's `profile_selections` row whose definition equals the
    /// clause's profile law.
    pub profile: &'p CheckedSelection,
    /// The `temporal_bindings` row of the declaration, or `None` when the
    /// lock selects no clock configuration.
    pub clock: Option<&'p TemporalBinding>,
}

/// The binding a request agreed with, retained in the report (FR-522
/// "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedBinding {
    /// The checked package's identity: the package of the declaration's node
    /// identity.
    pub package: Box<str>,
    /// The declaration's node identity.
    pub declaration: CheckedNodeId,
    /// The definition identity of the profile selection (authority and
    /// identity).
    pub profile: CheckedArtifactRef,
    /// The clock binding name of the row.
    pub clock_name: Box<str>,
    /// The clock configuration of the row.
    pub clock: ClockConfiguration,
}

impl TraceBinding {
    /// Compare this binding with the clause's selection, before any position,
    /// stamp or meter is touched.
    ///
    /// The dimensions are tested in the order `Profile`, `Clock`,
    /// `ClockParameters`, then the member dimensions, and the first that
    /// differs is reported; a `FixedSample` reports `Epoch`, `SamplePeriod`,
    /// `ClockUnit` in that order. A clause with no clock row refuses any
    /// binding with `Clock`.
    ///
    /// # Errors
    ///
    /// [`BindingMismatch`] naming the first differing dimension.
    pub fn authenticate(
        &self,
        selection: &ClauseSelection<'_>,
    ) -> Result<AuthenticatedBinding, BindingMismatch> {
        let expected_profile = &selection.profile.definition.identity;
        if self.profile != *expected_profile {
            return Err(BindingMismatch {
                dimension: MismatchedDimension::Profile,
                asserted: self.profile.to_string(),
                expected: Some(expected_profile.to_string()),
            });
        }
        let row = clock_mismatch(&self.clock_name, &self.parameters, selection.clock)?;
        Ok(AuthenticatedBinding {
            package: selection.declaration.package.clone(),
            declaration: selection.declaration.clone(),
            profile: selection.profile.definition.clone(),
            clock_name: row.clock_name.clone(),
            clock: row.clock.clone(),
        })
    }
}

/// FR-522 "Authentication" after the profile: compare an asserted clock
/// binding name and clock configuration with the declaration's
/// `temporal_bindings` row, in the order `Clock`, `ClockParameters`, then the
/// member dimensions, and return the row they agree with.
///
/// Both the evidence call ([`TraceBinding::authenticate`]) and the spine's
/// `Progressed` and `Timestamped` traces authenticate through this one
/// comparison.
///
/// # Errors
///
/// [`BindingMismatch`] naming the first differing dimension: `Clock` when
/// `row` is `None` (the lock selects no clock for the declaration) or names
/// another clock, `ClockParameters` for another configuration alternative,
/// else the first differing member.
pub fn clock_mismatch<'r>(
    clock_name: &str,
    parameters: &ClockConfiguration,
    row: Option<&'r TemporalBinding>,
) -> Result<&'r TemporalBinding, BindingMismatch> {
    let Some(row) = row else {
        return Err(BindingMismatch {
            dimension: MismatchedDimension::Clock,
            asserted: clock_name.to_owned(),
            expected: None,
        });
    };
    if clock_name != row.clock_name.as_ref() {
        return Err(differs(
            MismatchedDimension::Clock,
            clock_name.to_owned(),
            row.clock_name.to_string(),
        ));
    }
    match parameters_mismatch(parameters, &row.clock) {
        Some(mismatch) => Err(mismatch),
        None => Ok(row),
    }
}

fn alternative(clock: &ClockConfiguration) -> &'static str {
    match clock {
        ClockConfiguration::EventPosition { .. } => "event_position",
        ClockConfiguration::FixedSample { .. } => "fixed_sample",
        ClockConfiguration::TimestampedEvent { .. } => "timestamped_event",
    }
}

fn number(value: &ClockExactNumber) -> String {
    match value {
        ClockExactNumber::Integer { decimal } => decimal.to_string(),
        ClockExactNumber::Rational {
            numerator,
            denominator,
        } => format!("{numerator}/{denominator}"),
    }
}

fn differs(dimension: MismatchedDimension, asserted: String, expected: String) -> BindingMismatch {
    BindingMismatch {
        dimension,
        asserted,
        expected: Some(expected),
    }
}

/// The first parameter dimension in which `asserted` is not `row`.
fn parameters_mismatch(
    asserted: &ClockConfiguration,
    row: &ClockConfiguration,
) -> Option<BindingMismatch> {
    match (asserted, row) {
        (
            ClockConfiguration::EventPosition {
                sequence_authority: asserted,
            },
            ClockConfiguration::EventPosition {
                sequence_authority: expected,
            },
        ) => (asserted != expected).then(|| {
            differs(
                MismatchedDimension::SequenceAuthority,
                asserted.to_string(),
                expected.to_string(),
            )
        }),
        (
            ClockConfiguration::TimestampedEvent {
                timestamp_unit: asserted,
            },
            ClockConfiguration::TimestampedEvent {
                timestamp_unit: expected,
            },
        ) => (asserted != expected).then(|| {
            differs(
                MismatchedDimension::TimestampUnit,
                asserted.to_string(),
                expected.to_string(),
            )
        }),
        (
            ClockConfiguration::FixedSample {
                epoch,
                period,
                unit,
            },
            ClockConfiguration::FixedSample {
                epoch: row_epoch,
                period: row_period,
                unit: row_unit,
            },
        ) => {
            if epoch != row_epoch {
                Some(differs(
                    MismatchedDimension::Epoch,
                    number(epoch),
                    number(row_epoch),
                ))
            } else if period != row_period {
                Some(differs(
                    MismatchedDimension::SamplePeriod,
                    number(period),
                    number(row_period),
                ))
            } else if unit != row_unit {
                Some(differs(
                    MismatchedDimension::ClockUnit,
                    unit.to_string(),
                    row_unit.to_string(),
                ))
            } else {
                None
            }
        }
        (asserted, row) => Some(differs(
            MismatchedDimension::ClockParameters,
            alternative(asserted).to_owned(),
            alternative(row).to_owned(),
        )),
    }
}
