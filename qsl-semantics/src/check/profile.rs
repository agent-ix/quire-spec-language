// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-110: E3 resolves a unit's header profile selections against the
//! closed typed definition roles.
//!
//! A header selects one of the closed `header_selectable_layers` roles (the
//! six layers of QSpec AD-003, including the `root` row
//! `quire.value.complete/v1`); qualification applicability is determined by
//! the typed semantic roles. A clause
//! profile resolves against its clause family's closed semantic profile
//! definition (ADR-012 §2): the
//! `ProtocolClause` family's is [`protocol_profile`] and the `TemporalTrace`
//! family's is [`TemporalProfile`]. Every other header profile reaches the
//! `Value` table.

use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::{Code, Span};

use crate::value::definition::{DefinitionReference, DefinitionRole};
use crate::value::layer::HeaderLayer;

/// The publishing authority of the clause families' profiles.
pub(super) const CLAUSE_PROFILE_AUTHORITY: &str = "agent-ix";

/// The protocol clause family's one profile: bounded finite global
/// choreography (QSpec FR-322 `protocol_profile`).
const PROTOCOL_PROFILE: &str = "quire.protocol.finite-global/v1";

/// The `protocol_profile` definition a header profile `identity` selects, if
/// it is admitted for the `ProtocolClause` family (FR-110). The one
/// place a header profile's spelling is read for that family.
#[ix_trace_rs::string_edge]
pub(crate) fn protocol_profile(identity: &str) -> Option<DefinitionReference> {
    (identity == PROTOCOL_PROFILE).then(|| DefinitionReference {
        authority: CLAUSE_PROFILE_AUTHORITY.to_owned(),
        identity: PROTOCOL_PROFILE.to_owned(),
    })
}

/// The `TemporalTrace` family's closed temporal profile set (QSpec FR-250 as
/// FR-326 admits it, with the timed profile FR-234 admits): the five profiles
/// a `temporal` clause is checked under. A clause is checked under exactly
/// one, selected by identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemporalProfile {
    /// `quire.temporal.event-position.false-extension/v1`.
    EventPositionFalseExtension,
    /// `quire.temporal.fixed-sample.false-extension/v1`.
    FixedSampleFalseExtension,
    /// `quire.temporal.timestamped-event.finite-window/v1`.
    TimestampedEventFiniteWindow,
    /// `quire.temporal.infinite-trace/v1`.
    InfiniteTrace,
    /// `quire.temporal.timed/v1` (QSpec FR-250, FR-416; QSL FR-234): intervals
    /// measure exact rational time between time-stamped positions.
    Timed,
}

impl TemporalProfile {
    /// Every profile, in enum order.
    pub const ALL: [Self; 5] = [
        Self::EventPositionFalseExtension,
        Self::FixedSampleFalseExtension,
        Self::TimestampedEventFiniteWindow,
        Self::InfiniteTrace,
        Self::Timed,
    ];

    /// The profile's definition identity.
    pub const fn identity(self) -> &'static str {
        match self {
            Self::EventPositionFalseExtension => "quire.temporal.event-position.false-extension/v1",
            Self::FixedSampleFalseExtension => "quire.temporal.fixed-sample.false-extension/v1",
            Self::TimestampedEventFiniteWindow => {
                "quire.temporal.timestamped-event.finite-window/v1"
            }
            Self::InfiniteTrace => "quire.temporal.infinite-trace/v1",
            Self::Timed => "quire.temporal.timed/v1",
        }
    }

    /// The profile a header identity selects, if it is admitted for the
    /// `TemporalTrace` family (FR-110). The one place a header profile's
    /// spelling is read for that family.
    #[ix_trace_rs::string_edge]
    pub fn from_identity(identity: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|profile| profile.identity() == identity)
    }

    /// Whether every operator needs a closed interval and the clause has a
    /// finite horizon (FR-326 "Bounded profiles"); `false` for
    /// infinite-trace and for the timed profile, which admits every
    /// unbounded operator infinite-trace admits (FR-234).
    pub const fn is_bounded(self) -> bool {
        match self {
            Self::EventPositionFalseExtension
            | Self::FixedSampleFalseExtension
            | Self::TimestampedEventFiniteWindow => true,
            Self::InfiniteTrace | Self::Timed => false,
        }
    }

    /// The profile's definition as a [`DefinitionReference`].
    pub fn definition(self) -> DefinitionReference {
        DefinitionReference {
            authority: CLAUSE_PROFILE_AUTHORITY.to_owned(),
            identity: self.identity().to_owned(),
        }
    }
}

/// Why a header profile does not resolve (FR-110's table).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileCause {
    /// The identity is not a closed semantic role: `unknown_profile`/`unsupported-selection`.
    UnsupportedSelection,
    /// The identity is a closed semantic role other than `root`:
    /// `unknown_profile`/`wrong-selection-role`.
    WrongSelectionRole,
}

/// One header profile E3 refused, retaining the supplied selection and the
/// row it was compared with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileRefusal {
    /// The profile's local alias.
    pub alias: String,
    /// The span of the selection's identity literal.
    pub identity_span: Span,
    /// The definition identity the header selected.
    pub selected: String,
    /// The role a header profile must select.
    pub required_role: DefinitionRole,
    /// The `root` row the selection was compared with.
    pub root: DefinitionReference,
    /// Why it refused.
    pub cause: ProfileCause,
}

impl ProfileRefusal {
    /// The `quire.native.diagnostics/v1` code.
    pub fn code(&self) -> Code {
        match self.cause {
            ProfileCause::UnsupportedSelection | ProfileCause::WrongSelectionRole => {
                Code::UnknownProfile
            }
        }
    }

    /// The `quire.native.diagnostics/v1` cause tag.
    pub fn cause(&self) -> &'static str {
        match self.cause {
            ProfileCause::UnsupportedSelection => "unsupported-selection",
            ProfileCause::WrongSelectionRole => "wrong-selection-role",
        }
    }
}

/// Resolve each header profile by identity against the closed typed profile
/// roles of the value, model, temporal and protocol families, in source order
/// (FR-110). Every refusing
/// profile is returned, in source order.
pub fn resolve_profiles(profiles: &[ProfileSelection]) -> Result<(), Vec<ProfileRefusal>> {
    let root = DefinitionRole::Root.reference();
    let refusals: Vec<ProfileRefusal> = profiles
        .iter()
        .filter_map(|profile| {
            let selected = profile.identity.as_str();
            let cause = if HeaderLayer::from_identity(selected).is_some()
                || protocol_profile(selected).is_some()
                || TemporalProfile::from_identity(selected).is_some()
            {
                return None;
            } else if DefinitionRole::ALL
                .into_iter()
                .any(|role| role.identity() == selected)
                || crate::value::definition::ModelRole::ALL
                    .into_iter()
                    .any(|role| role.identity() == selected)
                || crate::value::definition::TemporalRole::ALL
                    .into_iter()
                    .any(|role| role.identity() == selected)
            {
                ProfileCause::WrongSelectionRole
            } else {
                ProfileCause::UnsupportedSelection
            };
            Some(ProfileRefusal {
                alias: profile.alias.clone(),
                identity_span: profile.identity_span,
                selected: selected.to_owned(),
                required_role: DefinitionRole::Root,
                root: root.clone(),
                cause,
            })
        })
        .collect();
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// FR-110: a header profile naming the protocol clause family's profile
    /// resolves against that family's closed profile set, and selects it as the package's
    /// `protocol_profile`; a profile outside its closed set still refuses.
    #[trace("FR-092-AC-18", "TC-1021")]
    #[test]
    fn the_protocol_profile_resolves_in_the_protocol_profile_set() {
        let selection = |identity: &str| ProfileSelection {
            alias: "p".to_owned(),
            identity: identity.to_owned(),
            span: Span { start: 0, end: 0 },
            identity_span: Span { start: 0, end: 0 },
        };
        let protocol = selection("quire.protocol.finite-global/v1");
        assert_eq!(resolve_profiles(std::slice::from_ref(&protocol)), Ok(()));
        assert_eq!(
            protocol_profile(&protocol.identity),
            Some(DefinitionReference {
                authority: "agent-ix".to_owned(),
                identity: "quire.protocol.finite-global/v1".to_owned(),
            })
        );
        let unknown = selection("quire.protocol.unknown/v1");
        assert_eq!(protocol_profile(&unknown.identity), None);
        let refused = resolve_profiles(&[unknown]).expect_err("an unknown profile refuses");
        assert_eq!(refused[0].cause, ProfileCause::UnsupportedSelection);
    }

    /// FR-326, FR-234, FR-110: the five temporal profiles, the timed
    /// profile `quire.temporal.timed/v1` among them, resolve in the
    /// `TemporalTrace` family's closed profile set by identity, each as bounded or not;
    /// a near-miss identity refuses.
    #[trace("FR-110")]
    #[test]
    fn the_temporal_profiles_resolve_in_the_temporal_profile_set() {
        let selection = |identity: &str| ProfileSelection {
            alias: "t".to_owned(),
            identity: identity.to_owned(),
            span: Span { start: 0, end: 0 },
            identity_span: Span { start: 0, end: 0 },
        };
        for profile in TemporalProfile::ALL {
            assert_eq!(
                TemporalProfile::from_identity(profile.identity()),
                Some(profile)
            );
            assert_eq!(resolve_profiles(&[selection(profile.identity())]), Ok(()));
            assert_eq!(
                profile.is_bounded(),
                !matches!(
                    profile,
                    TemporalProfile::InfiniteTrace | TemporalProfile::Timed
                )
            );
        }
        assert_eq!(
            TemporalProfile::from_identity("quire.temporal.timed/v1"),
            Some(TemporalProfile::Timed)
        );
        for unknown in [
            "quire.temporal.timed/v2",
            "quire.temporal.Timed/v1",
            "quire.temporal.infinite-trace/v2",
            "quire.temporal.Infinite-trace/v1",
        ] {
            assert_eq!(TemporalProfile::from_identity(unknown), None);
            let refused = resolve_profiles(&[selection(unknown)]).expect_err("refuses");
            assert_eq!(refused[0].cause, ProfileCause::UnsupportedSelection);
        }
    }
}
