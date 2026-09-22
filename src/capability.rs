// SPDX-License-Identifier: AGPL-3.0-or-later
//! The canonical capability-kind value type (ADR-013 O-19).
//!
//! [`Capability`] is the one QSL Rust type that carries an
//! `ix://agent-ix/quire-specification/FR-290` capability-kind label.
//! `ix://agent-ix/quire-specification#134` owns the ten-label vocabulary
//! `quire.capability-kind/v1` and its wire spelling; QSL's copy here defines
//! no local alias, abbreviation or extra member ([FR-057], "Admitted
//! vocabulary").
//!
//! Recording a `Capability` is language admission only (ADR-013 O-19;
//! quire-specification AD-016 arrow 1): it names which claim a declaration
//! requests, grants no checking, lowering, proof or execution, and
//! negotiates nothing.
//!
//! This type lives here, in foundation, rather than in `check`, `package`
//! or `route`, because O-19 has it cross every one of those layers: it is
//! recorded during S3 admission (layer 3 `check`), carried into the v2
//! `capability_report` at S4 (layer 4 `package`), and read by the layer-R
//! registry (`route`, [#185]). A shared value with consumers on more than
//! one side of a layer boundary lives in F, the same reasoning
//! [`crate::absence`] already states for `AbsenceMode`.
//!
//! `Capability` converts to and from its FR-290 wire spelling totally
//! (ADR-013 C-24, [`Capability::to_wire`] and [`Capability::from_wire`]):
//! every value has exactly one wire string, and every wire string either
//! names exactly one value or is refused. There is no default for an
//! unrecognized string.
//!
//! Registration, negotiation and routing over this type, and the composed
//! linker's admission rules that produce a `Capability` from a requested
//! clause/capability pair, are implemented separately ([FR-057], [FR-075]);
//! this module specifies and implements only the value type and its wire
//! conversion.
//!
//! [FR-057]: https://github.com/agent-ix/quire-spec-language/blob/main/spec/functional/FR-057-admit-shared-capability-kinds.md
//! [FR-075]: https://github.com/agent-ix/quire-spec-language/blob/main/spec/functional/FR-075-compute-candidates-from-registered-backends.md
//! [#185]: https://github.com/agent-ix/quire-spec-language/issues/185

use std::fmt;

/// The canonical FR-290 capability-kind value (ADR-013 O-19).
///
/// Two values are equal exactly when their FR-290 labels are byte-equal
/// (FR-057-AC-4). Declaration order here, in the FR-290 vocabulary table,
/// and in any serialization confers no strength, precedence or dispatch
/// priority (FR-057, "Spelling, identity and order").
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Capability {
    /// `value-validity` (FR-290 family `value`).
    ValueValidity,
    /// `operation-contract` (FR-290 family `state-model`).
    OperationContract,
    /// `finite-replay` (FR-290 family `finite-replay`).
    FiniteReplay,
    /// `temporal-satisfaction` (FR-290 family `temporal-trace`).
    TemporalSatisfaction,
    /// `global-conformance` (FR-290 family `protocol`).
    GlobalConformance,
    /// `monitorability` (FR-290 family `protocol`).
    Monitorability,
    /// `local-projection` (FR-290 family `protocol`).
    LocalProjection,
    /// `refinement` (FR-290 family `protocol`).
    Refinement,
    /// `realizability` (FR-290 family `protocol`).
    Realizability,
    /// `composition` (FR-290 family `protocol`).
    Composition,
}

impl Capability {
    /// Every admitted [`Capability`] value, in the FR-290 vocabulary
    /// table's declaration order.
    ///
    /// The order carries no meaning (see the type's own doc); this array
    /// exists only so callers, including this module's own tests, can walk
    /// the whole vocabulary without repeating its ten members.
    pub const ALL: [Capability; 10] = [
        Capability::ValueValidity,
        Capability::OperationContract,
        Capability::FiniteReplay,
        Capability::TemporalSatisfaction,
        Capability::GlobalConformance,
        Capability::Monitorability,
        Capability::LocalProjection,
        Capability::Refinement,
        Capability::Realizability,
        Capability::Composition,
    ];

    /// The FR-290 wire spelling of this value.
    ///
    /// Total (ADR-013 C-24): every [`Capability`] has exactly one wire
    /// string, and this function never fails.
    pub const fn to_wire(self) -> &'static str {
        match self {
            Capability::ValueValidity => "value-validity",
            Capability::OperationContract => "operation-contract",
            Capability::FiniteReplay => "finite-replay",
            Capability::TemporalSatisfaction => "temporal-satisfaction",
            Capability::GlobalConformance => "global-conformance",
            Capability::Monitorability => "monitorability",
            Capability::LocalProjection => "local-projection",
            Capability::Refinement => "refinement",
            Capability::Realizability => "realizability",
            Capability::Composition => "composition",
        }
    }

    /// Reads a [`Capability`] from its exact FR-290 wire spelling.
    ///
    /// Matches by exact equality of the decoded bytes against one admitted
    /// label (FR-057, "Spelling, identity and order"); this function
    /// applies no normalization, alias or default. A label that is not
    /// byte-equal to one of the ten admitted labels -- including the empty
    /// string, a case variant, a separator variant, a display form, a
    /// padded label, or a Rust variant name -- refuses with
    /// [`UnknownCapabilityLabel`], carrying the exact received bytes
    /// (ADR-013 C-24; FR-057-AC-2).
    pub fn from_wire(label: &str) -> Result<Capability, UnknownCapabilityLabel> {
        match label {
            "value-validity" => Ok(Capability::ValueValidity),
            "operation-contract" => Ok(Capability::OperationContract),
            "finite-replay" => Ok(Capability::FiniteReplay),
            "temporal-satisfaction" => Ok(Capability::TemporalSatisfaction),
            "global-conformance" => Ok(Capability::GlobalConformance),
            "monitorability" => Ok(Capability::Monitorability),
            "local-projection" => Ok(Capability::LocalProjection),
            "refinement" => Ok(Capability::Refinement),
            "realizability" => Ok(Capability::Realizability),
            "composition" => Ok(Capability::Composition),
            other => Err(UnknownCapabilityLabel(other.to_owned())),
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_wire())
    }
}

/// A wire string that is not byte-equal to any FR-290 capability-kind
/// label.
///
/// Carries the exact received bytes (FR-057-AC-2). Refusal never supplies a
/// default or a suggested replacement kind ([FR-057], "Refusal of labels
/// outside the vocabulary").
///
/// [FR-057]: https://github.com/agent-ix/quire-spec-language/blob/main/spec/functional/FR-057-admit-shared-capability-kinds.md
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownCapabilityLabel(String);

impl UnknownCapabilityLabel {
    /// The exact bytes [`Capability::from_wire`] received and refused
    /// (FR-057-AC-2).
    pub fn received(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnknownCapabilityLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not an FR-290 capability-kind label", self.0)
    }
}

impl std::error::Error for UnknownCapabilityLabel {}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// The FR-290 vocabulary `quire.capability-kind/v1` at
    /// quire-specification revision `55d2fcc`
    /// (`ix://agent-ix/quire-specification#134`), exactly as FR-057's own
    /// "Admitted vocabulary" table restates it. Listed here, independently
    /// of [`Capability::ALL`]/[`Capability::to_wire`], so the round-trip
    /// test below checks the implementation against the vocabulary rather
    /// than against itself.
    const FR_290_LABELS: [&str; 10] = [
        "value-validity",
        "operation-contract",
        "finite-replay",
        "temporal-satisfaction",
        "global-conformance",
        "monitorability",
        "local-projection",
        "refinement",
        "realizability",
        "composition",
    ];

    #[test]
    #[trace("TC-153", "FR-057-AC-1")]
    fn every_fr_290_label_round_trips_to_a_byte_identical_string() {
        assert_eq!(FR_290_LABELS.len(), Capability::ALL.len());
        for label in FR_290_LABELS {
            let value = Capability::from_wire(label)
                .unwrap_or_else(|e| panic!("admitted label {label:?} refused: {e}"));
            assert_eq!(
                value.to_wire(),
                label,
                "round trip changed the wire spelling of {label:?}"
            );
        }
        // The admitted set equals the FR-290 table exactly: no extra and no
        // missing member (FR-057-AC-1).
        for value in Capability::ALL {
            assert!(
                FR_290_LABELS.contains(&value.to_wire()),
                "{value:?} is not one of the ten FR-290 labels"
            );
        }
    }

    #[test]
    #[trace("TC-153", "FR-057-AC-2")]
    fn a_label_outside_the_vocabulary_refuses_and_names_the_received_bytes() {
        let cases = [
            "",
            "not-a-capability",
            "Global-Conformance", // case variant
            "value_validity",     // separator variant
            "Operation contract", // display form
            " composition",       // padded label
            "ValueValidity",      // Rust variant name
            "FamilyCheck",        // the OBS-003 four-member request vocabulary
            "StateOperation",
            "FiniteReplay", // note: not "finite-replay"
            "TemporalProjection",
        ];
        for label in cases {
            match Capability::from_wire(label) {
                Ok(value) => panic!("{label:?} should have refused, admitted as {value:?}"),
                Err(refusal) => {
                    assert_eq!(
                        refusal.received(),
                        label,
                        "refusal did not name the exact received bytes"
                    );
                }
            }
        }
    }

    #[test]
    #[trace("TC-153", "FR-057-AC-4")]
    fn equality_is_label_equality_and_the_ten_values_are_pairwise_distinct() {
        for (i, a) in Capability::ALL.iter().enumerate() {
            for (j, b) in Capability::ALL.iter().enumerate() {
                assert_eq!(
                    a == b,
                    i == j,
                    "{a:?} and {b:?} equality disagreed with their vocabulary positions"
                );
            }
            // A value converted through its own wire spelling is equal to
            // the original (same as the round-trip test, stated as an
            // equality property here).
            assert_eq!(*a, Capability::from_wire(a.to_wire()).unwrap());
        }
    }
}
