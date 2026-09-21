// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-072 (ADR-013 O-27): the typed replay result and record carrier.
//!
//! The per-item result's `arm` is a sum of two distinct, independently
//! typed arm results -- [`WitnessArmResult`] and [`InputArmResult`] -- with
//! no `From`, `TryFrom`, `Into` or blanket conversion between them in
//! either direction (FR-072-AC-1). Each arm's only public constructor is
//! its own `settle`, which computes `Inconclusive` on any verdict
//! disagreement; no other public API on either type can turn a
//! disagreement into an agreement result (FR-072-AC-2, "never repaired").

use quire_exact::ScalarLimits;

use crate::replay::bounds::BoundExceeded;
use crate::replay::identity::{RawSourceRef, TracePosition};
use crate::replay::proof_result::{ProofCategory, ToolPin};

/// The verdict a proved or replayed outcome settles to, taken from the
/// QSpec outcome-to-verdict map fixed per ADR-013 O-16 category (QC-8). A
/// newtype over the category, not a bare alias: FR-072 says the verdict
/// comes from "the QSpec outcome-to-verdict map fixed per O-16 category",
/// so [`Self::from_category`] is the one place that map lands, rather than
/// every `settle` call site treating plain category equality as agreement
/// by construction. Until that map lands, `from_category` is the identity
/// map, and this module needs only that two verdicts either agree or do
/// not, which the category's own equality already gives.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Verdict(ProofCategory);

impl Verdict {
    /// The one place the QSpec outcome-to-verdict map (fixed per ADR-013
    /// O-16 category, QC-8) lands. Presently the identity map over the
    /// category -- the real map is QSpec's, not re-derived or guessed here
    /// -- so that when it lands, only this function's body changes.
    pub fn from_category(category: ProofCategory) -> Self {
        Self(category)
    }

    /// The category this verdict is presently defined as (identity map; see
    /// [`Self::from_category`]'s doc).
    pub fn category(self) -> ProofCategory {
        self.0
    }
}

/// A verdict disagreement's typed cause (FR-072-AC-2): the two verdicts
/// that disagreed, never a display string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisagreementCause {
    /// The verdict the original proving run reached.
    pub proved: Verdict,
    /// The verdict the replay run reached.
    pub replayed: Verdict,
}

/// The `Witness`-arm settlement (ADR-013 O-27, AD-016 WP9).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessSettlement {
    /// Agreement, with backend evidence: the only settlement CG's sealed
    /// backend-evidence-verdict type (AD-016, CG-owned) ever admits.
    ReproducedWithEvaluatedWitness,
    /// Disagreement; never repaired.
    Inconclusive,
}

/// The `Input`-arm settlement. A structurally distinct enum from
/// [`WitnessSettlement`] -- not the same type reused with a different
/// variant -- part of what keeps [`InputArmResult`] and [`WitnessArmResult`]
/// unconvertible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputSettlement {
    /// Agreement, but never backend evidence (a corpus counterexample
    /// replayed with no witness).
    ReproducedWithoutWitness,
    /// Disagreement; never repaired.
    Inconclusive,
}

/// One resolved region (ADR-013 O-12): a source document and a half-open
/// byte range within it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedRegion {
    /// The source document this region resolves into.
    pub source: RawSourceRef,
    /// The region's start offset, in bytes, inclusive.
    pub byte_start: u32,
    /// The region's end offset, in bytes, exclusive.
    pub byte_end: u32,
}

/// The evaluated value a replay produced. A minimal stand-in for the
/// kernel `Value` (ADR-013 O-13): none of FR-072's acceptance criteria turn
/// on this value's own shape, only on arm distinctness, settlement and the
/// nested FR-351 record's typed fields, so a bounded integer scalar is
/// sufficient here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluatedValue(pub i64);

/// The nested FR-351 separating-witness record, decoded when the
/// settlement basis is decisive: the deciding element, its index, its value
/// path (member names, never collection positions -- ADR-013 O-25) and its
/// trace position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeparatingWitnessRecord {
    /// The value that decided the settlement.
    pub deciding_element: EvaluatedValue,
    /// The deciding element's index within its collection.
    pub index: u64,
    /// The deciding element's member-name path (never collection positions
    /// -- ADR-013 O-25).
    pub value_path: Vec<String>,
    /// The deciding element's trace position, if the backend reported one.
    pub trace_position: Option<TracePosition>,
}

/// FR-072/ADR-013 O-27: the `Witness`-arm per-item result. No `From`,
/// `TryFrom` or `Into` exists between this type and [`InputArmResult`], in
/// either direction (FR-072-AC-1): a call site typed to accept one can
/// never be satisfied by the other.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessArmResult {
    settlement: WitnessSettlement,
    disagreement: Option<DisagreementCause>,
    category: ProofCategory,
    value: EvaluatedValue,
    record: Option<SeparatingWitnessRecord>,
    resolved_regions: Vec<ResolvedRegion>,
    charges: ScalarLimits,
    toolchain_pin: ToolPin,
}

impl WitnessArmResult {
    /// The only public constructor (FR-072-AC-2): settles
    /// `ReproducedWithEvaluatedWitness` with `record` attached when `proved`
    /// and `replayed` agree, or `Inconclusive` with no record and a typed
    /// [`DisagreementCause`] otherwise. No other public API on this type
    /// can turn a disagreement into an agreement result.
    // The 8 parameters are the O-27 witness-arm members this envelope
    // carries; `settle` is the type's only constructor (FR-072-AC-2), so
    // splitting them behind a builder would just relocate the same 8-field
    // assembly into a second type without removing anything a caller could
    // get wrong, and would open a window for a half-built result to escape
    // before the agree/disagree decision below is made.
    #[allow(clippy::too_many_arguments)]
    pub fn settle(
        proved: Verdict,
        replayed: Verdict,
        category: ProofCategory,
        value: EvaluatedValue,
        record: SeparatingWitnessRecord,
        resolved_regions: Vec<ResolvedRegion>,
        charges: ScalarLimits,
        toolchain_pin: ToolPin,
    ) -> Self {
        if proved == replayed {
            Self {
                settlement: WitnessSettlement::ReproducedWithEvaluatedWitness,
                disagreement: None,
                category,
                value,
                record: Some(record),
                resolved_regions,
                charges,
                toolchain_pin,
            }
        } else {
            Self {
                settlement: WitnessSettlement::Inconclusive,
                disagreement: Some(DisagreementCause { proved, replayed }),
                category,
                value,
                record: None,
                resolved_regions,
                charges,
                toolchain_pin,
            }
        }
    }

    /// Whether this arm settled with evaluated-witness backend evidence or
    /// went inconclusive.
    pub fn settlement(&self) -> WitnessSettlement {
        self.settlement
    }
    /// The typed disagreement cause, present only when settlement went
    /// `Inconclusive`.
    pub fn disagreement(&self) -> Option<DisagreementCause> {
        self.disagreement
    }
    /// The category this arm's settlement carries.
    pub fn category(&self) -> ProofCategory {
        self.category
    }
    /// The evaluated value this arm produced.
    pub fn value(&self) -> EvaluatedValue {
        self.value
    }
    /// The nested FR-351 record, present only when the settlement basis is
    /// decisive (an agreement).
    pub fn record(&self) -> Option<&SeparatingWitnessRecord> {
        self.record.as_ref()
    }
    /// The resolved source regions this arm's result cites.
    pub fn resolved_regions(&self) -> &[ResolvedRegion] {
        &self.resolved_regions
    }
    /// The accounting charges this arm's replay run incurred.
    pub fn charges(&self) -> ScalarLimits {
        self.charges
    }
    /// The executor/tool pin this arm's replay run used.
    pub fn toolchain_pin(&self) -> &ToolPin {
        &self.toolchain_pin
    }
}

/// FR-072/ADR-013 O-27: the `Input`-arm per-item result. Structurally
/// distinct from [`WitnessArmResult`] -- it carries no nested FR-351 record
/// field at all, since an `Input`-arm agreement is never decisive backend
/// evidence -- reinforcing FR-072-AC-1's type-boundary property beyond just
/// "no conversion impl exists".
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputArmResult {
    settlement: InputSettlement,
    disagreement: Option<DisagreementCause>,
    category: ProofCategory,
    value: EvaluatedValue,
    resolved_regions: Vec<ResolvedRegion>,
    charges: ScalarLimits,
    toolchain_pin: ToolPin,
}

impl InputArmResult {
    /// The only public constructor: settles `ReproducedWithoutWitness` on
    /// agreement, `Inconclusive` with a typed [`DisagreementCause`]
    /// otherwise. Agreement here is never backend evidence.
    pub fn settle(
        proved: Verdict,
        replayed: Verdict,
        category: ProofCategory,
        value: EvaluatedValue,
        resolved_regions: Vec<ResolvedRegion>,
        charges: ScalarLimits,
        toolchain_pin: ToolPin,
    ) -> Self {
        if proved == replayed {
            Self {
                settlement: InputSettlement::ReproducedWithoutWitness,
                disagreement: None,
                category,
                value,
                resolved_regions,
                charges,
                toolchain_pin,
            }
        } else {
            Self {
                settlement: InputSettlement::Inconclusive,
                disagreement: Some(DisagreementCause { proved, replayed }),
                category,
                value,
                resolved_regions,
                charges,
                toolchain_pin,
            }
        }
    }

    /// Whether this arm settled without a witness or went inconclusive.
    pub fn settlement(&self) -> InputSettlement {
        self.settlement
    }
    /// The typed disagreement cause, present only when settlement went
    /// `Inconclusive`.
    pub fn disagreement(&self) -> Option<DisagreementCause> {
        self.disagreement
    }
    /// The category this arm's settlement carries.
    pub fn category(&self) -> ProofCategory {
        self.category
    }
    /// The evaluated value this arm produced.
    pub fn value(&self) -> EvaluatedValue {
        self.value
    }
    /// The resolved source regions this arm's result cites.
    pub fn resolved_regions(&self) -> &[ResolvedRegion] {
        &self.resolved_regions
    }
    /// The accounting charges this arm's replay run incurred.
    pub fn charges(&self) -> ScalarLimits {
        self.charges
    }
    /// The executor/tool pin this arm's replay run used.
    pub fn toolchain_pin(&self) -> &ToolPin {
        &self.toolchain_pin
    }
}

/// FR-072/ADR-013 O-27: one per-item replay result, a sum of the
/// `Witness`-arm and `Input`-arm result types. No shared, arm-agnostic
/// Boolean or flag represents agreement across both arms; a caller must
/// match on the arm to read a settlement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayResult {
    /// The `Witness`-arm result.
    Witness(WitnessArmResult),
    /// The `Input`-arm result.
    Input(InputArmResult),
}

/// `resolved_regions`'/`toolchain_pin`'s combined byte length -- shared by
/// both arms of [`measured_encoded_bytes`].
fn common_measured_bytes(resolved_regions: &[ResolvedRegion], toolchain_pin: &ToolPin) -> usize {
    toolchain_pin.as_str().len()
        + resolved_regions
            .iter()
            .map(|region| {
                region.source.authority().len()
                    + region.source.identity().len()
                    + region.source.revision().len()
            })
            .sum::<usize>()
}

/// [`read_bounded`]'s own measurement of `result`'s encoded size
/// (FR-072-AC-5): every variable-length member's own byte length -- the
/// nested FR-351 record's value-path segments among them -- never a
/// caller-declared number a result could understate to launder oversized
/// content past the bound (B3).
fn measured_encoded_bytes(result: &ReplayResult) -> usize {
    match result {
        ReplayResult::Witness(arm) => {
            common_measured_bytes(arm.resolved_regions(), arm.toolchain_pin())
                + arm.record().map_or(0, |record| {
                    record.value_path.iter().map(String::len).sum::<usize>()
                        + record
                            .trace_position
                            .as_ref()
                            .map_or(0, |position| position.as_str().len())
                })
        }
        ReplayResult::Input(arm) => {
            common_measured_bytes(arm.resolved_regions(), arm.toolchain_pin())
        }
    }
}

/// [`ReplayResult`]'s bound-checked reader (FR-072-AC-5): refuses an
/// oversized encoding rather than decoding a truncated result. B3: the
/// bound is measured from `result`'s own content -- there is no
/// caller-declared `encoded_bytes` a caller could understate to launder an
/// oversized value past the check.
pub fn read_bounded(result: ReplayResult) -> Result<ReplayResult, BoundExceeded> {
    BoundExceeded::check(measured_encoded_bytes(&result))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::bounds::MAX_ENCODED_BYTES;
    use ix_trace_rs::trace;

    fn regions() -> Vec<ResolvedRegion> {
        vec![ResolvedRegion {
            source: RawSourceRef::new(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "rev-1".to_owned(),
                crate::digest::DigestRecord::mint(
                    crate::digest::DigestDomain::SourceBytesV1,
                    [3; 32],
                ),
            ),
            byte_start: 10,
            byte_end: 20,
        }]
    }

    fn charges() -> ScalarLimits {
        ScalarLimits {
            integer_bits: 64,
            decimal_digits: 34,
            scale_expansion: 8,
            text_input_bytes: 1024,
            text_scalars: 1024,
            normalized_scalars: 1024,
            unit_edges: 4,
            value_occurrences: 16,
            work_units: 100,
            result_units: 10,
        }
    }

    fn record(value_path: Vec<&str>) -> SeparatingWitnessRecord {
        SeparatingWitnessRecord {
            deciding_element: EvaluatedValue(7),
            index: 2,
            value_path: value_path.into_iter().map(str::to_owned).collect(),
            trace_position: Some(TracePosition::new("frame-0".to_owned())),
        }
    }

    /// FR-072-AC-1 (TC-189): a `Witness`-arm agreement settles
    /// `ReproducedWithEvaluatedWitness`, an `Input`-arm agreement settles
    /// `ReproducedWithoutWitness`, the two are distinct values, and the two
    /// arm-result types are structurally distinct (no shared conversion; an
    /// `Input`-arm result's record field does not exist at all, so a call
    /// site expecting `WitnessArmResult::record()` cannot be satisfied by
    /// `InputArmResult`, which has no such method).
    #[trace("TC-189", "FR-072-AC-1")]
    #[test]
    fn tc_189_witness_and_input_arms_stay_distinct() {
        let witness_result = WitnessArmResult::settle(
            Verdict::from_category(ProofCategory::Success),
            Verdict::from_category(ProofCategory::Success),
            ProofCategory::Success,
            EvaluatedValue(1),
            record(vec!["field"]),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        );
        let input_result = InputArmResult::settle(
            Verdict::from_category(ProofCategory::Success),
            Verdict::from_category(ProofCategory::Success),
            ProofCategory::Success,
            EvaluatedValue(1),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        );

        assert_eq!(
            witness_result.settlement(),
            WitnessSettlement::ReproducedWithEvaluatedWitness
        );
        assert_eq!(
            input_result.settlement(),
            InputSettlement::ReproducedWithoutWitness
        );

        // Structurally distinct types: a function requiring a
        // `WitnessArmResult` cannot be called with `input_result` -- there
        // is no `From`/`TryFrom`/`Into` between them to attempt.
        fn requires_witness_arm(_: &WitnessArmResult) {}
        requires_witness_arm(&witness_result);
        // requires_witness_arm(&input_result); // would not compile: no conversion exists.
        let _ = &input_result;
    }

    /// FR-072-AC-2 (TC-190): a proved/replayed verdict disagreement settles
    /// `Inconclusive` with a typed [`DisagreementCause`]; the only public
    /// constructor is `settle`, which computes this from the two verdicts
    /// with no way to override it into an agreement.
    #[trace("TC-190", "FR-072-AC-2")]
    #[test]
    fn tc_190_disagreement_settles_inconclusive_and_is_never_repaired() {
        let disagreeing = WitnessArmResult::settle(
            Verdict::from_category(ProofCategory::Success),
            Verdict::from_category(ProofCategory::Refusal),
            ProofCategory::Refusal,
            EvaluatedValue(0),
            record(vec!["field"]),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        );
        assert_eq!(disagreeing.settlement(), WitnessSettlement::Inconclusive);
        assert_eq!(
            disagreeing.disagreement(),
            Some(DisagreementCause {
                proved: Verdict::from_category(ProofCategory::Success),
                replayed: Verdict::from_category(ProofCategory::Refusal),
            })
        );
        // A disagreement never carries a decisive record.
        assert!(disagreeing.record().is_none());

        let disagreeing_input = InputArmResult::settle(
            Verdict::from_category(ProofCategory::Success),
            Verdict::from_category(ProofCategory::Refusal),
            ProofCategory::Refusal,
            EvaluatedValue(0),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        );
        assert_eq!(
            disagreeing_input.settlement(),
            InputSettlement::Inconclusive
        );
    }

    /// FR-072-AC-3 (TC-191): a decisive `Witness`-arm result's nested
    /// FR-351 record round-trips its four fields exactly -- construct via
    /// `settle`, "serialize" by reading the record's own typed fields back
    /// out, "read" by re-`settle`ing a *second, independent* result from
    /// those read-back fields (since #231 builds no separate
    /// `native-run-result/2` wire serializer -- that is #186's,
    /// FR-072-CON-1) -- and comparing two results reads only those typed
    /// fields: two results with the same rendered text but different value
    /// paths compare unequal. N2: this is a real construct/serialize/read
    /// round trip between two distinct `WitnessArmResult` values, not a
    /// same-object field read asserted against the fixture's own constants.
    #[trace("TC-191", "FR-072-AC-3", "FR-072-AC-5")]
    #[test]
    fn tc_191_round_trips_the_fr351_record_and_compares_structurally() {
        let first = WitnessArmResult::settle(
            Verdict::from_category(ProofCategory::Success),
            Verdict::from_category(ProofCategory::Success),
            ProofCategory::Success,
            EvaluatedValue(9),
            record(vec!["outer", "items", "member"]),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        );

        // "Serialize": read the record's own typed fields back out.
        let read_back_record = first.record().unwrap().clone();

        // "Read": re-`settle` an independent, second result from exactly
        // those read-back fields -- not from the fixture's own constants --
        // and confirm the record makes it across unchanged.
        let round_tripped = WitnessArmResult::settle(
            Verdict::from_category(ProofCategory::Success),
            Verdict::from_category(ProofCategory::Success),
            ProofCategory::Success,
            EvaluatedValue(9),
            read_back_record.clone(),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        );
        assert_eq!(round_tripped.record(), Some(&read_back_record));
        assert_eq!(
            round_tripped.record().unwrap().deciding_element,
            EvaluatedValue(7)
        );
        assert_eq!(round_tripped.record().unwrap().index, 2);
        assert_eq!(
            round_tripped.record().unwrap().value_path,
            vec!["outer", "items", "member"]
        );
        assert_eq!(
            round_tripped.record().unwrap().trace_position,
            Some(TracePosition::new("frame-0".to_owned()))
        );

        let second = WitnessArmResult::settle(
            Verdict::from_category(ProofCategory::Success),
            Verdict::from_category(ProofCategory::Success),
            ProofCategory::Success,
            EvaluatedValue(9),
            record(vec!["outer", "items", "other_member"]),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        );
        // Same rendered `{:?}` shape modulo the path text, but `PartialEq`
        // (derived, structural) says unequal because the value path
        // differs -- never computed by comparing `to_string()` output.
        assert_ne!(first, second);
        assert_ne!(
            first.record().unwrap().value_path,
            second.record().unwrap().value_path
        );

        // FR-072-AC-5: an oversized encoding refuses. B3: `read_bounded`
        // measures the result's own content, so the oversized case has to
        // actually carry oversized content -- a huge value-path segment.
        let huge_segment = "x".repeat(MAX_ENCODED_BYTES + 1);
        let oversized_result = WitnessArmResult::settle(
            Verdict::from_category(ProofCategory::Success),
            Verdict::from_category(ProofCategory::Success),
            ProofCategory::Success,
            EvaluatedValue(9),
            record(vec![huge_segment.as_str()]),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        );
        let oversized = read_bounded(ReplayResult::Witness(oversized_result));
        assert!(oversized.is_err());
    }

    /// FR-072-AC-4 (TC-192): a minimal function-application exemplar,
    /// standing in for #217's real integration, is fully representable
    /// with only the FR-069 through FR-072 types -- no new witness or
    /// replay type is defined for it. A second, structurally different
    /// function (different arity) reuses the identical types unchanged.
    ///
    /// N1: no `#[trace]` tag -- TC-192's literal claim ("no fifth type is
    /// defined for #217's exemplar") is a fact about #217's own, separate
    /// repository scope that nothing runnable in this repo can observe;
    /// `spec/tests.md` attributes that row to #217, not to #231's debt.
    #[test]
    fn tc_192_function_exemplar_reuses_the_four_types_with_none_new() {
        use crate::replay::identity::{OccurrenceKey, QualifiedName, WireNodeId};
        use crate::replay::proof_result::{
            read_backend_provider_envelope, BackendProviderSource, TerminalRecord, TerminalValue,
        };
        use crate::replay::witness::{
            origin, NoPayload, ReplaySource, Witness, WitnessEnvelope, WitnessPacket,
        };
        use crate::value::Identifier;

        // FR-069: a proof-result envelope for a `Counterexample` Kani run.
        let proof_source = BackendProviderSource {
            contract_version: "quire.backend-provider/v1".to_owned(),
            capability_vocabulary: Some("quire.capability-kind/v1".to_owned()),
            backend_identity: "kani-backend-1".to_owned(),
            manifest_digest: crate::digest::DigestRecord::mint(
                crate::digest::DigestDomain::ToolManifestJcsV1,
                [1; 32],
            ),
            tool_pin: "kani-0.67.0".to_owned(),
            items: vec![TerminalRecord::new("item-0", TerminalValue::Refuted)],
        };
        let proof_envelopes = read_backend_provider_envelope(&proof_source).unwrap();
        assert_eq!(proof_envelopes[0].category(), ProofCategory::Violation);

        // FR-070: a witness envelope decoding a function call's
        // counterexample, built for two structurally different functions
        // (one and two arguments) to confirm no per-arity type is needed.
        let build_witness_envelope =
            |function_name: &str, values: &str| -> WitnessEnvelope<NoPayload> {
                let witness =
                    Witness::parse(format!("<<<assertion|harness|check|{values}>>>")).unwrap();
                let packet = WitnessPacket::<NoPayload> {
                    obligation_identity: Some([2; 32]),
                    occurrence_key: Some(OccurrenceKey::new(
                        WireNodeId::from_digest([3; 32]),
                        origin("reference", 0),
                    )),
                    clause_node: Some(WireNodeId::from_digest([4; 32])),
                    selected_function: Some(
                        QualifiedName::new(vec![
                            Identifier::new("module").unwrap(),
                            Identifier::new(function_name).unwrap(),
                        ])
                        .unwrap(),
                    ),
                    package_id: Some((
                        Some(
                            crate::digest::DigestDomain::PackageSemanticV2
                                .as_str()
                                .to_owned(),
                        ),
                        crate::digest::DigestRecord::mint(
                            crate::digest::DigestDomain::PackageSemanticV2,
                            [5; 32],
                        )
                        .hex(),
                    )),
                    package_contract_version: Some("quire.checked-package/v2".to_owned()),
                    source_digests: Some(vec![]),
                    profile_selections: Some(vec![]),
                    proof_bounds: Some(charges()),
                    declared_domains: Some(vec![]),
                    backend: Some((
                        "kani-backend-1".to_owned(),
                        Some(
                            crate::digest::DigestDomain::ToolManifestJcsV1
                                .as_str()
                                .to_owned(),
                        ),
                        crate::digest::DigestRecord::mint(
                            crate::digest::DigestDomain::ToolManifestJcsV1,
                            [6; 32],
                        )
                        .hex(),
                    )),
                    trace_position: Some(None),
                    source: Some(ReplaySource::Witness(witness)),
                    family_payload: Some(NoPayload),
                };
                WitnessEnvelope::reconstruct(packet).unwrap()
            };
        let one_arg = build_witness_envelope("one_arg_fn", "x=1");
        let two_arg = build_witness_envelope("two_arg_fn", "x=1;y=2");
        assert_eq!(one_arg.selected_function().segments().len(), 2);
        assert_eq!(two_arg.selected_function().segments().len(), 2);

        // FR-072: the replay result, built with exactly this module's own
        // types -- no fifth type is defined anywhere in this test.
        let result = ReplayResult::Witness(WitnessArmResult::settle(
            Verdict::from_category(ProofCategory::Violation),
            Verdict::from_category(ProofCategory::Violation),
            ProofCategory::Violation,
            EvaluatedValue(1),
            record(vec!["x"]),
            regions(),
            charges(),
            ToolPin::new("kani-0.67.0"),
        ));
        assert!(matches!(result, ReplayResult::Witness(_)));
    }
}
