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
//! This type lives in `check` core, not in a foundation module, because its
//! three consumers form a downward chain rather than AbsenceMode's
//! two-sided boundary. It is recorded during S3 admission at E3 (ADR-011
//! §2.1, "capability_report ... holding the item's one capability kind";
//! `check` is E3's producer, ADR-011 §2.1's own E3 row), carried into the
//! v2 `capability_report` at S4 (layer 4 `package`), and read by the
//! layer-R registry (`route`, [#185]). ADR-011 §6.1's layer table lists
//! layer 4's and layer R's "Depends on" columns as each including "3"
//! (`package | ... | 3, F, K`; `route | ... | 4, 3, F, K`), so both may
//! import layer 3 `check` directly; nothing needs to import upward. §6.1's
//! layer-3 row also names the right home directly: `check` core holds
//! "`CheckContext`, family checker trait, shared checked types,
//! `FamilyOutcome`, `FamilyResult` and `EvalOutcome`" -- `Capability` is exactly a shared
//! checked type admission produces. (`AbsenceMode`, in
//! [`qsl_foundation::absence`], sits in foundation for the opposite reason: its two
//! consumers, layer-2 `forms` and layer-3 `model`, cannot depend on each
//! other, so neither layer may own it.)
//!
//! `Capability` converts to and from its FR-290 wire spelling totally
//! (ADR-013 C-24, [`Capability::to_wire`] and [`Capability::from_wire`]):
//! every value has exactly one wire string, and every wire string either
//! names exactly one value or is refused. There is no default for an
//! unrecognized string. The enum, [`Capability::ALL`] and
//! [`Capability::to_wire`] are generated together from one `capability_kinds!`
//! macro table below -- the same technique `ProjectionTarget`'s `targets!`
//! macro (`src/lowering/target.rs`) uses -- so there is exactly one
//! hand-written list of the ten labels; the variant set, `ALL` and the wire
//! spelling all expand from it in one place and cannot drift apart the way
//! three separately maintained lists could. [`Capability::from_wire`]
//! derives from `ALL`/`to_wire` rather than a fourth copy of the label list.
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

macro_rules! capability_kinds {
    ($($(#[$doc:meta])* $variant:ident => $wire:literal),+ $(,)?) => {
        /// The canonical FR-290 capability-kind value (ADR-013 O-19).
        ///
        /// Two values are equal exactly when their FR-290 labels are byte-equal
        /// (FR-057-AC-4). Declaration order here, in the FR-290 vocabulary table,
        /// and in any serialization confers no strength, precedence or dispatch
        /// priority (FR-057, "Spelling, identity and order").
        ///
        /// `#[cfg(seam_probe)]` adds one probe-only variant (ADR-012 §5.1 S7,
        /// FR-063): under `--cfg seam_probe`, every closed `match` over this
        /// type below this module's own [`Capability::to_wire`] becomes
        /// non-exhaustive (`E0004`) unless it has its own probe arm.
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        pub enum Capability {
            $($(#[$doc])* $variant),+,
            /// FR-063/S7: exists only so `--cfg seam_probe` makes every
            /// match over `Capability` outside this module non-exhaustive.
            /// Never constructed outside the probe build.
            #[cfg(seam_probe)]
            __SeamProbe,
        }

        impl Capability {
            /// Every admitted [`Capability`] value, in the FR-290 vocabulary
            /// table's declaration order.
            ///
            /// The order carries no meaning (see the type's own doc); this
            /// slice exists so [`Capability::from_wire`] and this module's
            /// tests can walk the whole vocabulary without repeating its ten
            /// members a third time. Generated by this same
            /// `capability_kinds!` expansion as the enum and
            /// [`Capability::to_wire`], so it cannot drift out of sync with
            /// either.
            pub const ALL: &'static [Capability] = &[$(Capability::$variant),+];

            /// The FR-290 wire spelling of this value.
            ///
            /// Total (ADR-013 C-24): every [`Capability`] has exactly one wire
            /// string, and this function never fails.
            ///
            /// Not an S7 seam-probe location: this function owns the probe
            /// variant's own arm (below), since it is generated in the same
            /// macro expansion as the variant itself. The seam probe
            /// exercises match sites *outside* this module instead.
            pub const fn to_wire(self) -> &'static str {
                match self {
                    $(Capability::$variant => $wire),+,
                    #[cfg(seam_probe)]
                    Capability::__SeamProbe => "__seam_probe__",
                }
            }

            /// Reads a [`Capability`] from its exact FR-290 wire spelling.
            ///
            /// Matches by exact equality of the decoded bytes against one admitted
            /// label (FR-057, "Spelling, identity and order"); this function
            /// applies no normalization, alias or default. Derives its answer from
            /// [`Capability::ALL`] and [`Capability::to_wire`] rather than
            /// restating the label list, so the two conversions cannot disagree. A
            /// label that is not byte-equal to one of the ten admitted labels --
            /// including the empty string, a case variant, a separator variant, a
            /// display form, a padded label, or a Rust variant name -- refuses with
            /// [`UnknownCapabilityLabel`], carrying the exact received bytes
            /// (ADR-013 C-24; FR-057-AC-2).
            pub fn from_wire(label: &str) -> Result<Capability, UnknownCapabilityLabel> {
                Capability::ALL
                    .iter()
                    .copied()
                    .find(|value| value.to_wire() == label)
                    .ok_or_else(|| UnknownCapabilityLabel(label.to_owned()))
            }
        }
    };
}

capability_kinds! {
    /// `value-validity` (FR-290 family `value`).
    ValueValidity => "value-validity",
    /// `operation-contract` (FR-290 family `state-model`).
    OperationContract => "operation-contract",
    /// `finite-replay` (FR-290 family `finite-replay`).
    FiniteReplay => "finite-replay",
    /// `temporal-satisfaction` (FR-290 family `temporal-trace`).
    TemporalSatisfaction => "temporal-satisfaction",
    /// `global-conformance` (FR-290 family `protocol`).
    GlobalConformance => "global-conformance",
    /// `monitorability` (FR-290 family `protocol`).
    Monitorability => "monitorability",
    /// `local-projection` (FR-290 family `protocol`).
    LocalProjection => "local-projection",
    /// `refinement` (FR-290 family `protocol`).
    Refinement => "refinement",
    /// `realizability` (FR-290 family `protocol`).
    Realizability => "realizability",
    /// `composition` (FR-290 family `protocol`).
    Composition => "composition",
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_wire())
    }
}

impl std::str::FromStr for Capability {
    type Err = UnknownCapabilityLabel;

    /// Delegates to [`Capability::from_wire`]: the same exact-match rule,
    /// the same refusal type.
    fn from_str(label: &str) -> Result<Self, Self::Err> {
        Self::from_wire(label)
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

    /// The FR-290 vocabulary `quire.capability-kind/v1`
    /// (`ix://agent-ix/quire-specification#134`), exactly as FR-057's own
    /// "Admitted vocabulary" table restates it, paired with the specific
    /// [`Capability`] variant each label names. Listed here, independently
    /// of [`Capability::ALL`]/[`Capability::to_wire`], so the test below
    /// checks the implementation against the vocabulary rather than
    /// against itself.
    ///
    /// The pairing matters, not just the label set: [`Capability::from_wire`]
    /// derives from [`Capability::to_wire`] (both directions move together),
    /// so a bug that swaps two labels between two variants in `to_wire`
    /// still round-trips every label to itself -- each label just now names
    /// the other variant. Asserting `variant.to_wire() == label` per pair,
    /// against this independently-typed table, is what actually catches
    /// that swap.
    const FR_290_VOCABULARY: [(Capability, &str); 10] = [
        (Capability::ValueValidity, "value-validity"),
        (Capability::OperationContract, "operation-contract"),
        (Capability::FiniteReplay, "finite-replay"),
        (Capability::TemporalSatisfaction, "temporal-satisfaction"),
        (Capability::GlobalConformance, "global-conformance"),
        (Capability::Monitorability, "monitorability"),
        (Capability::LocalProjection, "local-projection"),
        (Capability::Refinement, "refinement"),
        (Capability::Realizability, "realizability"),
        (Capability::Composition, "composition"),
    ];

    #[test]
    #[trace("TC-153", "FR-057-AC-1")]
    fn every_fr_290_label_names_its_specific_variant_and_round_trips() {
        assert_eq!(FR_290_VOCABULARY.len(), Capability::ALL.len());
        for (variant, label) in FR_290_VOCABULARY {
            assert_eq!(
                variant.to_wire(),
                label,
                "{variant:?} no longer converts to its FR-290 label {label:?}"
            );
            assert_eq!(
                Capability::from_wire(label),
                Ok(variant),
                "{label:?} no longer reads back as {variant:?}"
            );
        }
        // The admitted set equals the FR-290 table exactly: no extra and no
        // missing member (FR-057-AC-1).
        for value in Capability::ALL.iter().copied() {
            assert!(
                FR_290_VOCABULARY
                    .iter()
                    .any(|(variant, _)| *variant == value),
                "{value:?} is not one of the ten FR-290 vocabulary entries"
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

    #[test]
    fn from_str_agrees_with_from_wire() {
        for (variant, label) in FR_290_VOCABULARY {
            assert_eq!(label.parse::<Capability>(), Ok(variant));
        }
        assert_eq!(
            "not-a-capability".parse::<Capability>(),
            Capability::from_wire("not-a-capability"),
        );
    }
}
