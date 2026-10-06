// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098 (ADR-013 O-26, C-13, TK-01; ADR-011 §2.1 E9, §4, §6.1): the
//! replay executor entry, [`replay`].
//!
//! It reads the request (FR-071), recompiles the one source unit the
//! package reference names through the spine ([`crate::spine::parse`],
//! S1 to S4) from the digest-addressed byte provision, requires the
//! recompiled `package_id` to equal the request's, selects the function by
//! its `QualifiedName` in the recompiled package's declarations, joins the
//! replay source's arguments to the function's parameters by parameter node
//! id (each `WireNodeId` becomes a `NodeKey` only by lookup in the
//! recompiled package), calls it through S6a (`CheckedPackage::call`) and
//! settles the verdict against the counterexample's. No `CheckedPackage` is
//! built from wire bytes, and nothing is read from a path, environment
//! variable or search location.
//!
//! Every failure before a verdict is a [`ReplayRefusal`], and no partial
//! result accompanies it. A replay that runs and disagrees with the
//! counterexample, including an S6a outcome that completed no value,
//! settles `inconclusive` with its typed cause and is never repaired
//! ([`crate::ReplayResult`]).

use qsl_eval::value::{input_refusal_code, CallFailure, CheckedPackageEvaluation};
use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::{Code, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::library::{LibraryName, PackageId};
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{
    Cancel, Integer, LimitKind, Meter, NodeKey, Outcome, ScalarLimits, Value, ValueType,
};
use quire_semantic_value::call::InputRefusal;

use crate::bounds::MAX_ENCODED_BYTES;
use crate::identity::{ObligationIdentity, QualifiedName, RawSourceRef};
use crate::request::{ReplayRequest, ReplayRequestRefusal, ReplayRequestWire, StageLimits};
use crate::result::{
    EvaluatedValue, InputArmResult, ReplayResult, Verdict, WitnessArmResult, WitnessCheck,
};
use crate::spine::{
    self, CompileRefusal, DependencyInput, DependencyInputRefusal, ParseRequest, SpineLimits,
    SuppliedLibrary,
};
use crate::witness::{
    DecodeRefusal, FrameOperation, ReplaySource, WitnessBinding, WitnessValue, WitnessValueType,
};
use qsl_forms::StateClauseKind;
use qsl_foundation::diagnostic::Category;
use qsl_semantics::model::observation::{AdmissionFailure, ObservationForm};
use quire_exact::Identifier;

mod frame;
pub use frame::{replay_frame, FrameIdentityMismatch, FrameReplayResult};

mod operator_parity;
pub use operator_parity::{replay_operator_parity, ScalarIdentityMismatch};

mod value_parity;
pub use value_parity::{replay_value_parity, ValueParityResult};

mod state_clause;
pub use state_clause::{replay_state_clause, ClauseIdentityMismatch, StateClauseReplayResult};
#[cfg(test)]
pub(crate) use state_clause::{separate, settle_separation, stopped_reason, SeparationOutcome};

/// The S1 limit a request names that is above this executor's reader
/// limit (ADR-013 O-26: "a limit above the reader limit").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LimitAboveReader {
    /// The request's S1 `text_input_bytes`.
    pub requested: u64,
    /// The reader limit: [`MAX_ENCODED_BYTES`], the most source bytes any
    /// admitted request can carry.
    pub reader: u64,
}

/// Why [`replay`] settled no verdict. Each variant is one ADR-013 O-26
/// refusal, with its typed cause; none carries a partial result.
#[derive(Debug, thiserror::Error)]
pub enum ReplayRefusal {
    /// FR-071: the request did not decode -- an unknown version, an input
    /// absent from the byte provision or whose bytes do not match their
    /// digest, or an encoding above the reader bound.
    #[error(transparent)]
    Request(#[from] ReplayRequestRefusal),
    /// The request's S1 source limit is above the reader limit: the request
    /// asks for more than the reader allows, which is invalid input
    /// (`invalid-request`, exit 20), not an exhausted limit.
    #[error(
        "invalid-request: the request's S1 text_input_bytes {} is above the reader limit {}",
        .0.requested,
        .0.reader
    )]
    LimitAboveReader(LimitAboveReader),
    /// A package reference entry is not a `quire.source.bytes/v1` source.
    /// The recompile reads no definition document -- the definitions are
    /// QSL's pinned catalog, which `package_id` already covers -- so such a
    /// digest is one the executor cannot recompute and compare (QSpec
    /// FR-323-AC-5).
    #[error("the package reference names {} under {}, which is not a source the replay recompiles", .0.identity(), .0.digest().domain())]
    NotASource(Box<RawSourceRef>),
    /// The package reference does not name exactly one source unit. The
    /// spine recompiles one unit: a package over more than one waits on
    /// E3 import resolution (FR-087-AC-13).
    #[error("the package reference names {0} source units; the spine recompiles exactly one")]
    SourceCount(usize),
    /// The recompile refused at one spine stage, or reached one of the
    /// stage limits the request carries (`stage_limit_exceeded`).
    #[error("the recompile refused at stage {stage}: {refusal}", stage = .0.stage().as_str(), refusal = .0)]
    Recompile(Box<CompileRefusal>),
    /// ADR-015 D-4 rules 1 and 6: the package reference's `dependencies`
    /// are not in strictly ascending UTF-8 byte order of identity, or name
    /// an identity the recompiled package does not select
    /// (`invalid_package`/`invalid-value` at `/package/dependencies`).
    #[error("invalid_package/invalid-value at /package/dependencies: {0}")]
    DependencySelections(DependencySelectionsCause),
    /// ADR-015 D-4 rule 3: the `dependencies` entries do not form a
    /// dependency input (ADR-015 D-1).
    #[error("the package reference's dependencies are no dependency input: {0}")]
    DependencyInput(DependencyInputRefusal),
    /// ADR-015 D-4 rule 7: an entry's `package_id` is not the recompiled
    /// closure's selection of its identity
    /// (`stale_dependency`/`content-mismatch`).
    #[error("stale_dependency/content-mismatch: the request names {identity} as {} but it recompiles to {}", .requested.hex(), .recompiled.hex())]
    DependencyIdentityMismatch {
        /// The entry's identity.
        identity: LibraryName,
        /// The `package_id` the entry names.
        requested: DigestRecord,
        /// The closure's recomputed selection of that identity.
        recompiled: PackageId,
    },
    /// The recompiled `package_id` is not the request's: the source's
    /// meaning changed since the proving run.
    #[error("stale_dependency/content-mismatch: the request names package {} but the source recompiles to {}", .requested.hex(), .recompiled.hex())]
    PackageIdMismatch {
        /// The `package_id` the request names.
        requested: DigestRecord,
        /// The `package_id` the recompile computed.
        recompiled: PackageId,
    },
    /// The selection names no function node of the recompiled package.
    #[error("missing_declaration/missing-name: package {} declares no function {selection}", .package.hex())]
    UnknownFunction {
        /// The request's selection.
        selection: QualifiedName,
        /// The recompiled package it was looked up in.
        package: PackageId,
    },
    /// An argument names a node that is not a parameter of the selected
    /// function.
    #[error("invalid_runtime_input: node {0} is not a parameter of the selected function")]
    UnknownParameter(WireNodeId),
    /// Two arguments name the same parameter.
    #[error("invalid_runtime_input: parameter {0} is bound twice")]
    DuplicateArgument(WireNodeId),
    /// A parameter of the selected function has no argument.
    #[error("invalid_runtime_input: parameter {0} has no argument")]
    UnboundParameter(WireNodeId),
    /// The backend witness does not decode against the selected function's
    /// parameters. Carries the obligation the request replays (ADR-013
    /// O-25), so a caller replaying many obligations knows which failed.
    #[error("the witness for obligation {obligation} does not decode: {refusal}")]
    Witness {
        /// The request's obligation identity (ADR-013 O-09).
        obligation: ObligationIdentity,
        /// The decode's cause.
        refusal: DecodeRefusal,
    },
    /// An argument refused admission as `WrongValueKind`: its value is not
    /// of the parameter's declared type, whether that is found before the
    /// call (an integer bound to a Boolean parameter, a Boolean to an
    /// integer one, or any value to a parameter of a kind no witness value
    /// is) or by S6a admission in
    /// `CheckedPackage::call` (a value outside the declared domain, such as
    /// `12` for `Int[0, 9]`) -- carried as ADR-011 §2.3's
    /// `StageFailure::Refused` cause.
    #[error("{code} ({cause}): {refusal}", code = input_refusal_code(&.0).as_str(), cause = .0.cause(), refusal = .0)]
    Input(InputRefusal),
    /// The selected function's declared result is not `Boolean`, so it
    /// states no property a counterexample refutes. Refused before the
    /// call, from the declaration alone.
    #[error("the selected function {selection} of package {} is not a predicate: its declared result is not Boolean", .package.hex())]
    NotAPredicate {
        /// The request's selection.
        selection: QualifiedName,
        /// The recompiled package that declares it.
        package: PackageId,
    },
    /// FR-357: the selected function's declared result is `Boolean`, so its
    /// claim is a predicate's, replayed by [`replay`], not a value-parity
    /// claim. Refused before the call, from the declaration alone.
    #[error("the selected function {selection} of package {} is a predicate: its declared result is Boolean, so it has no value-parity replay", .package.hex())]
    NotAValueFunction {
        /// The request's selection.
        selection: QualifiedName,
        /// The recompiled package that declares it.
        package: PackageId,
    },
    /// FR-357: an operator-level claim's scalar node, enclosing function or
    /// operator is not the recompiled package's.
    #[error("stale_dependency/revision-mismatch: {0}")]
    ScalarIdentity(Box<ScalarIdentityMismatch>),
    /// FR-116: a frame counterexample's operation names no operation
    /// frame of the recompiled package.
    #[error("missing_declaration/missing-name: package {} holds no operation frame {operation}", .package.hex())]
    UnknownOperation {
        /// The payload's operation.
        operation: FrameOperation,
        /// The recompiled package it was looked up in.
        package: PackageId,
    },
    /// FR-116: a frame counterexample's anchor, frame or occurrence
    /// identity is not the recompiled package's (refused before any
    /// admission), or its envelope's clause node or occurrence key is not
    /// the payload's frame node or occurrence (refused before recompiling).
    #[error("stale_dependency/revision-mismatch: {0}")]
    FrameIdentity(Box<FrameIdentityMismatch>),
    /// FR-122: a state-clause counterexample's clause names no state clause
    /// of the recompiled package.
    #[error("missing_declaration/missing-name: package {} declares no state clause {}", .package.hex(), .clause.as_str())]
    UnknownClause {
        /// The payload's clause.
        clause: Identifier,
        /// The recompiled package it was looked up in.
        package: PackageId,
    },
    /// FR-122: a state-clause counterexample's envelope names a clause node
    /// or `claim` occurrence that is not the recompiled clause's (refused
    /// before any admission).
    #[error("stale_dependency/revision-mismatch: {0}")]
    ClauseIdentity(Box<ClauseIdentityMismatch>),
    /// FR-122: a state-clause counterexample's observation form is not the
    /// one its clause's kind takes (refused before any admission).
    #[error("wrong_snapshot/wrong-observation: a {kind:?} clause takes no {form} observation")]
    WrongObservation {
        /// The resolved clause's kind.
        kind: StateClauseKind,
        /// The payload's observation form.
        form: ObservationForm,
    },
    /// FR-116, FR-122: FR-106 admission of the counterexample's documents
    /// failed, with its own record.
    #[error(transparent)]
    Admission(AdmissionFailure),
    /// An executor or S6a invariant broke.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

impl ReplayRefusal {
    /// The catalog code of this refusal: the request's, the recompile
    /// stage's or S6a admission's own where one refused, and the executor's
    /// otherwise.
    pub fn code(&self) -> Code {
        match self {
            Self::Request(refusal) => refusal.code(),
            Self::LimitAboveReader(_) => Code::InvalidRequest,
            Self::NotASource(_) | Self::SourceCount(_) => Code::InvalidRequest,
            Self::Recompile(refusal) => refusal.code(),
            Self::PackageIdMismatch { .. } | Self::DependencyIdentityMismatch { .. } => {
                Code::StaleDependency
            }
            Self::DependencySelections(_) => Code::InvalidPackage,
            Self::DependencyInput(refusal) => refusal.code(),
            Self::UnknownFunction { .. } | Self::UnknownOperation { .. } => {
                Code::MissingDeclaration
            }
            Self::FrameIdentity(_) | Self::ClauseIdentity(_) | Self::ScalarIdentity(_) => {
                Code::StaleDependency
            }
            Self::UnknownClause { .. } => Code::MissingDeclaration,
            Self::WrongObservation { .. } => Code::WrongSnapshot,
            Self::Admission(
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
            ) => record.code,
            Self::Admission(AdmissionFailure::Fault(_)) => Code::RuntimeInvariant,
            Self::UnknownParameter(_)
            | Self::DuplicateArgument(_)
            | Self::UnboundParameter(_)
            | Self::Witness { .. }
            | Self::NotAPredicate { .. }
            | Self::NotAValueFunction { .. } => Code::InvalidRuntimeInput,
            Self::Input(refusal) => input_refusal_code(refusal),
            Self::Fault(_) => Code::RuntimeInvariant,
        }
    }

    /// The catalog cause tag under [`Self::code`], where the refusing stage
    /// or request check names one.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::Recompile(refusal) => refusal.cause(),
            Self::DependencySelections(_) => Some("invalid-value"),
            Self::DependencyInput(refusal) => refusal.cause(),
            Self::DependencyIdentityMismatch { .. } | Self::PackageIdMismatch { .. } => {
                Some("content-mismatch")
            }
            Self::UnknownFunction { .. }
            | Self::UnknownOperation { .. }
            | Self::UnknownClause { .. } => Some("missing-name"),
            Self::FrameIdentity(_) | Self::ClauseIdentity(_) | Self::ScalarIdentity(_) => {
                Some("revision-mismatch")
            }
            Self::WrongObservation { .. } => Some("wrong-observation"),
            Self::Input(refusal) => Some(refusal.cause()),
            Self::Admission(
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
            ) => Some(record.cause),
            Self::Request(_)
            | Self::LimitAboveReader(_)
            | Self::NotASource(_)
            | Self::SourceCount(_)
            | Self::Admission(AdmissionFailure::Fault(_))
            | Self::UnknownParameter(_)
            | Self::DuplicateArgument(_)
            | Self::UnboundParameter(_)
            | Self::Witness { .. }
            | Self::NotAPredicate { .. }
            | Self::NotAValueFunction { .. }
            | Self::Fault(_) => None,
        }
    }
}

/// Why the package reference's `dependencies` refused (ADR-015 D-4 rules 1
/// and 6).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DependencySelectionsCause {
    /// Rule 1: the entry at `index` does not follow the one before it in
    /// strictly ascending UTF-8 byte order of identity; a repeated identity
    /// included.
    #[error("entry {index} ({identity}) is not after the entry before it")]
    Unordered {
        /// The entry's position.
        index: usize,
        /// The entry's identity.
        identity: LibraryName,
    },
    /// Rule 6: the recompiled package selects no library of this identity.
    #[error("the recompiled package selects no {identity}")]
    Unselected {
        /// The entry's identity.
        identity: LibraryName,
    },
}

/// ADR-013 C-13: replay `wire` and settle its verdict. The counterexample a
/// request replays refuted its property, so the proved verdict is
/// `violation`; the replayed verdict is the selected predicate's: `false`
/// is `violation`, `true` is `success`, and an S6a outcome that completed
/// no value is its own category, which never agrees.
///
/// FR-063 seam: adding a `FamilyOutcome` variant with no arm here fails
/// `--cfg seam_probe` with `E0004`; FR-063-AC-7's `#[deny(...)]` closes the
/// wildcard-arm escape the probe alone cannot see.
///
/// # Function selection (TC-166)
///
/// FR-062-AC-10, FR-065-AC-6: the request selects its function by a typed
/// [`QualifiedName`], resolved against the recompiled package's own
/// declarations; a name that resolves to none refuses
/// [`ReplayRefusal::UnknownFunction`]. No entry takes a bare `&str` in its
/// place: this request, which differs from the one after it only in its
/// `selected_function` line, does not compile.
///
/// ```compile_fail,E0308
/// # use qsl_replay::*;
/// # let unit = "language \"ix:native\" edition \"1-draft\";\n\
/// #     profile v = \"quire.value.complete/v1\";\n\
/// #     function small using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
/// # let small = QualifiedName::new(vec![Identifier::new("small").unwrap()]).unwrap();
/// # let site = call_site(
/// #     SourceIdentity::new("a", "u", "git", "1"),
/// #     "u",
/// #     unit.as_bytes(),
/// #     [],
/// #     &DependencyInput::default(),
/// #     &small,
/// # )
/// # .unwrap();
/// # let digest = DigestRecord::mint(
/// #     DigestDomain::SourceBytesV1,
/// #     ByteDigest::of(unit.as_bytes()).as_bytes(),
/// # );
/// # let unlimited = ScalarLimits {
/// #     integer_bits: u64::MAX,
/// #     decimal_digits: u64::MAX,
/// #     scale_expansion: u64::MAX,
/// #     text_input_bytes: u64::MAX,
/// #     text_scalars: u64::MAX,
/// #     normalized_scalars: u64::MAX,
/// #     unit_edges: u64::MAX,
/// #     value_occurrences: u64::MAX,
/// #     work_units: u64::MAX,
/// #     result_units: u64::MAX,
/// # };
/// # let s1 = ScalarLimits {
/// #     text_input_bytes: u64::try_from(MAX_ENCODED_BYTES).unwrap(),
/// #     ..unlimited
/// # };
/// let wire = ReplayRequestWire {
///     selected_function: "small",
/// #   profile_selections: vec![],
/// #   package_id: (
/// #       Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
/// #       site.package_id.hex(),
/// #   ),
/// #   source_digests: vec![(
/// #       "a".to_owned(),
/// #       "u".to_owned(),
/// #       "git".to_owned(),
/// #       "1".to_owned(),
/// #       Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
/// #       digest.hex(),
/// #   )],
/// #   dependencies: Vec::new(),
/// #   source: ReplaySource::Input(vec![CanonicalAssignment {
/// #       parameter: site.site.parameters[0].1,
/// #       value: WitnessValue::Integer(7),
/// #   }]),
/// #   obligation_identity: [0; 32],
/// #   backend: "kani-backend-1".to_owned(),
/// #   state_environment: StateEnvironment::new(vec![]),
/// #   accounting_limits: unlimited,
/// #   stage_limits: StageLimits { s1, s2: unlimited, s3: unlimited, s4: unlimited },
/// #   byte_provision: vec![(
/// #       Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
/// #       digest.hex(),
/// #       unit.as_bytes().to_vec(),
/// #   )],
///     // ...every other member names the proved package and its input.
/// };
/// assert!(replay(wire).is_ok());
/// ```
///
/// The same request selecting `small` by its `QualifiedName` compiles and
/// replays:
///
/// ```
/// # use qsl_replay::*;
/// # let unit = "language \"ix:native\" edition \"1-draft\";\n\
/// #     profile v = \"quire.value.complete/v1\";\n\
/// #     function small using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
/// # let small = QualifiedName::new(vec![Identifier::new("small").unwrap()]).unwrap();
/// # let site = call_site(
/// #     SourceIdentity::new("a", "u", "git", "1"),
/// #     "u",
/// #     unit.as_bytes(),
/// #     [],
/// #     &DependencyInput::default(),
/// #     &small,
/// # )
/// # .unwrap();
/// # let digest = DigestRecord::mint(
/// #     DigestDomain::SourceBytesV1,
/// #     ByteDigest::of(unit.as_bytes()).as_bytes(),
/// # );
/// # let unlimited = ScalarLimits {
/// #     integer_bits: u64::MAX,
/// #     decimal_digits: u64::MAX,
/// #     scale_expansion: u64::MAX,
/// #     text_input_bytes: u64::MAX,
/// #     text_scalars: u64::MAX,
/// #     normalized_scalars: u64::MAX,
/// #     unit_edges: u64::MAX,
/// #     value_occurrences: u64::MAX,
/// #     work_units: u64::MAX,
/// #     result_units: u64::MAX,
/// # };
/// # let s1 = ScalarLimits {
/// #     text_input_bytes: u64::try_from(MAX_ENCODED_BYTES).unwrap(),
/// #     ..unlimited
/// # };
/// let wire = ReplayRequestWire {
///     selected_function: small,
/// #   profile_selections: vec![],
/// #   package_id: (
/// #       Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
/// #       site.package_id.hex(),
/// #   ),
/// #   source_digests: vec![(
/// #       "a".to_owned(),
/// #       "u".to_owned(),
/// #       "git".to_owned(),
/// #       "1".to_owned(),
/// #       Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
/// #       digest.hex(),
/// #   )],
/// #   dependencies: Vec::new(),
/// #   source: ReplaySource::Input(vec![CanonicalAssignment {
/// #       parameter: site.site.parameters[0].1,
/// #       value: WitnessValue::Integer(7),
/// #   }]),
/// #   obligation_identity: [0; 32],
/// #   backend: "kani-backend-1".to_owned(),
/// #   state_environment: StateEnvironment::new(vec![]),
/// #   accounting_limits: unlimited,
/// #   stage_limits: StageLimits { s1, s2: unlimited, s3: unlimited, s4: unlimited },
/// #   byte_provision: vec![(
/// #       Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
/// #       digest.hex(),
/// #       unit.as_bytes().to_vec(),
/// #   )],
///     // ...every other member names the proved package and its input.
/// };
/// assert!(replay(wire).is_ok());
/// ```
#[deny(clippy::wildcard_enum_match_arm)]
pub fn replay(wire: ReplayRequestWire) -> Result<ReplayResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire)?;
    let compiled = recompile(&request)?;
    let package = compiled.checked.package();
    let call = select(&compiled, request.selected_function(), Claim::Predicate)?;
    let arguments = arguments(
        package,
        &call,
        request.source(),
        request.obligation_identity(),
    )?;
    let mut meter = Meter::new(request.accounting_limits());
    let evaluation = package
        .call(
            &call.name,
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .map_err(call_failure_to_replay_refusal)?;
    let (replayed, value) = match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds))) => (
            if holds {
                Category::Success
            } else {
                Category::Violation
            },
            Some(EvaluatedValue::Boolean(holds)),
        ),
        // `select` admitted only a function declared `Boolean`.
        FamilyOutcome::Evaluated(Outcome::Completed(_)) => {
            return Err(ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "boolean-function-completes-a-boolean",
            )));
        }
        FamilyOutcome::Evaluated(Outcome::Refused(_)) => (Category::Refusal, None),
        FamilyOutcome::Evaluated(Outcome::Incomplete(_)) => (Category::Incomplete, None),
        // O-16's `undefined` row is not a proof category, and a
        // family-owned result is not a kernel verdict: neither agrees.
        FamilyOutcome::Evaluated(Outcome::Undefined(_)) | FamilyOutcome::FamilyEvaluated(_) => {
            (Category::Inconclusive, None)
        }
        // FR-063: no arm for the probe variant under `--cfg seam_probe`
        // alone -- this match is the replay facade's own seam over
        // `FamilyOutcome` (`E0004` in `xtask seam-probe`'s build of this
        // crate): a family whose S6a result is new widens the facade here
        // (ADR-013 TK-01). The arm below exists only in the probe's build of
        // the root crate (`--cfg seam_probe_replay_downstream`), which
        // depends on this crate. Do not add a catch-all to make it compile.
        #[cfg(seam_probe_replay_downstream)]
        FamilyOutcome::__SeamProbe => {
            unreachable!("never constructed outside the probe build")
        }
    };
    let proved = Verdict::from_category(Category::Violation);
    let regions = evaluation
        .location
        .as_ref()
        .and_then(|location| package.graph().region(location))
        .into_iter()
        .collect();
    let charges = consumed(&meter);
    Ok(match request.source() {
        ReplaySource::Witness(_) => ReplayResult::Witness(WitnessArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            value,
            // ADR-031 SW-7: a function call's or a frame check's result
            // has no decisive occurrence, so it carries no record.
            WitnessCheck::Agrees(None),
            regions,
            charges,
        )),
        ReplaySource::Input(_) => ReplayResult::Input(InputArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            value,
            &WitnessCheck::Agrees(None),
            regions,
            charges,
        )),
    })
}

/// FR-096-AC-15, SR-746 FND-002: a broken S6a invariant -- including a
/// kernel `Refusal::CheckedInvariant`, which the seam itself now turns into
/// `CallFailure::Fault` before `package.call` ever returns -- settles
/// `Err(ReplayRefusal::Fault(_))`, never `Ok(ReplayResult::..)` with
/// `Category::Refusal`. `CallFailure::Input` passes its own refusal
/// through unchanged.
fn call_failure_to_replay_refusal(failure: CallFailure) -> ReplayRefusal {
    match failure {
        CallFailure::Input(refusal) => ReplayRefusal::Input(refusal),
        CallFailure::Fault(fault) => ReplayRefusal::Fault(fault),
        // The replay meter holds no `Cancel` handle, so no charge is
        // cancelled.
        CallFailure::Cancelled(_) => {
            ReplayRefusal::Fault(InternalFault::new("replay", "cancelled-without-a-handle"))
        }
    }
}

/// The recompile's stage limits: the request's S1 `text_input_bytes`
/// bounds S1's source bytes, and its S3 `work_units` bounds the checker's
/// work budget. No `quire.value.accounting/v1` counter names an S2 or I1
/// limit, and the v2 emitter takes none, so those stages run under their
/// published defaults, as do S1's token, node and nesting ceilings and
/// S3's node, depth and input-byte ceilings.
fn spine_limits(stages: StageLimits) -> Result<SpineLimits, ReplayRefusal> {
    let reader = u64::try_from(MAX_ENCODED_BYTES).unwrap_or(u64::MAX);
    let above = || {
        ReplayRefusal::LimitAboveReader(LimitAboveReader {
            requested: stages.s1.text_input_bytes,
            reader,
        })
    };
    if stages.s1.text_input_bytes > reader {
        return Err(above());
    }
    let source_bytes = usize::try_from(stages.s1.text_input_bytes).map_err(|_| above())?;
    let defaults = SpineLimits::default();
    Ok(SpineLimits {
        source: qsl_cst::Limits {
            source_bytes,
            ..defaults.source
        },
        checking: defaults.checking.with_work_budget(stages.s3.work_units),
        ..defaults
    })
}

/// The recompile of a request's unit: its checked package and the bytes it
/// emits.
pub(crate) struct Recompiled {
    /// The recompiled in-process package, with the sources it came from.
    pub(crate) checked: spine::CheckedUnit,
    /// The recompiled package's v2 bytes and `package_id`.
    pub(crate) emitted: spine::EmittedUnit,
}

/// Recompile the request's one source unit from the byte provision against
/// the dependency input its `dependencies` entries build, with every domain
/// package the provision carries under its `sha256-jcs` digest as I1's
/// package input, applying ADR-015 D-4's seven rules in order: each rule
/// over every entry, in entry order, before the next.
fn recompile(request: &ReplayRequest) -> Result<Recompiled, ReplayRefusal> {
    // Rule 1: strictly ascending identities, before anything is built.
    for (index, pair) in request.dependencies().windows(2).enumerate() {
        if let [before, entry] = pair {
            if entry.identity() <= before.identity() {
                return Err(ReplayRefusal::DependencySelections(
                    DependencySelectionsCause::Unordered {
                        index: index + 1,
                        identity: entry.identity().clone(),
                    },
                ));
            }
        }
    }
    // Rule 2: the proved package's sources, and each entry's, name one
    // source unit.
    let source = one_source(request.source_digests())?;
    let library_sources = request
        .dependencies()
        .iter()
        .map(|entry| one_source(entry.sources()))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = spine_limits(request.stage_limits())?;
    let provided = |reference: &RawSourceRef| {
        request
            .byte_provision()
            .get(reference.digest())
            .ok_or_else(|| {
                // `ReplayRequest::decode` refuses an incomplete provision.
                ReplayRefusal::Fault(InternalFault::new(
                    "replay",
                    "decoded-request-byte-provision-complete",
                ))
            })
    };
    // Rule 3: the dependency input.
    let libraries = request
        .dependencies()
        .iter()
        .zip(library_sources)
        .map(|(entry, reference)| {
            Ok(SuppliedLibrary {
                identity: entry.identity().as_str().to_owned(),
                source: labels(reference),
                path: reference.identity().to_owned(),
                bytes: provided(reference)?.to_vec(),
            })
        })
        .collect::<Result<Vec<_>, ReplayRefusal>>()?;
    let dependencies = DependencyInput::new(libraries).map_err(ReplayRefusal::DependencyInput)?;
    dependencies
        .check_unit_owner(&labels(source))
        .map_err(ReplayRefusal::DependencyInput)?;
    let bytes = provided(source)?;
    let packages = domain_packages(request);
    // Rule 4: the recompile.
    let cancel = Cancel::new();
    let refusal = |failure| match spine::refusal_or_fault(failure) {
        Ok(refusal) => ReplayRefusal::Recompile(refusal),
        Err(fault) => ReplayRefusal::Fault(fault),
    };
    let unit = labels(source);
    let parsed = spine::parse(
        &ParseRequest {
            source: &unit,
            path: source.identity(),
            bytes,
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(refusal)?
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        &dependencies,
        &spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let emitted = spine::package(&checked, spine::PackageLimits::default(), &cancel)
        .map_err(refusal)?
        .into_value();
    let compiled = Recompiled { checked, emitted };
    // Rules 6 and 7: every entry names a selection of the recompiled
    // closure, then at its recomputed `package_id`, before rule 5.
    let selections = compiled.checked.package().dependency_selections();
    for entry in request.dependencies() {
        if !selections.contains_key(entry.identity()) {
            return Err(ReplayRefusal::DependencySelections(
                DependencySelectionsCause::Unselected {
                    identity: entry.identity().clone(),
                },
            ));
        }
    }
    for entry in request.dependencies() {
        let Some(selected) = selections.get(entry.identity()) else {
            return Err(ReplayRefusal::DependencySelections(
                DependencySelectionsCause::Unselected {
                    identity: entry.identity().clone(),
                },
            ));
        };
        let selected = selected.package_id;
        if !selected.matches(&entry.package_id()) {
            return Err(ReplayRefusal::DependencyIdentityMismatch {
                identity: entry.identity().clone(),
                requested: entry.package_id(),
                recompiled: selected,
            });
        }
    }
    // Rule 5: the proved package's `package_id`. It runs after rules 6 and 7,
    // so a stale dependency is named by its identity before the proved
    // package's own mismatch, which the dependency's id also changes.
    let requested = request.package_id();
    let recompiled = compiled.emitted.package().package_id();
    if !recompiled.matches(&requested) {
        return Err(ReplayRefusal::PackageIdMismatch {
            requested,
            recompiled,
        });
    }
    Ok(compiled)
}

/// I1's package input: every entry the byte provision carries under a
/// `sha256-jcs` digest (QC-1). A state document there is keyed by its raw
/// bytes' digest, which no model selection names.
fn domain_packages(request: &ReplayRequest) -> std::collections::BTreeMap<[u8; 32], Vec<u8>> {
    package_input(
        request
            .byte_provision()
            .entries()
            .filter(|(digest, _)| digest.domain() == DigestDomain::Sha256Jcs)
            .map(|(_, bytes)| bytes),
    )
}

/// The one `quire.source.bytes/v1` source `references` names (ADR-015 D-4
/// rule 2), else [`ReplayRefusal::NotASource`] or
/// [`ReplayRefusal::SourceCount`].
fn one_source(references: &[RawSourceRef]) -> Result<&RawSourceRef, ReplayRefusal> {
    if let Some(other) = references
        .iter()
        .find(|reference| reference.digest().domain() != DigestDomain::SourceBytesV1)
    {
        return Err(ReplayRefusal::NotASource(Box::new(other.clone())));
    }
    match references {
        [source] => Ok(source),
        _ => Err(ReplayRefusal::SourceCount(references.len())),
    }
}

/// A source reference's four FR-001 labels.
fn labels(reference: &RawSourceRef) -> SourceIdentity {
    SourceIdentity::new(
        reference.authority(),
        reference.identity(),
        reference.revision_namespace(),
        reference.revision(),
    )
}

/// What a replay claims of the function it selects.
#[derive(Clone, Copy)]
enum Claim {
    /// A Boolean predicate is false at the bindings ([`replay`]).
    Predicate,
    /// The generated code's outcome equals QSL's `f(b)`
    /// ([`replay_value_parity`]).
    ValueParity,
}

/// The selected function: its S6a name, and its parameter nodes and
/// declared types in declared order.
struct Selected {
    name: qsl_eval::value::QualifiedName,
    parameters: Vec<NodeKey>,
    types: Vec<ValueType>,
}

/// `callable`'s own parameter node keys, checked against its own declared
/// signature: the node lookup [`select`] and [`crate::call_site::call_site`]
/// each need to turn a `CallableFunction` into the parameter node ids a
/// `CanonicalAssignment` or a witness transcript names (ADR-013 O-25, C-11).
/// Shared so the two entries cannot silently drift apart (SR-780 FND-002).
pub(crate) fn callable_parameter_keys(
    package: &CheckedPackage,
    callable: &qsl_semantics::check::CallableFunction<'_>,
) -> Result<Vec<NodeKey>, InternalFault> {
    let parameters = package
        .graph()
        .semantic_graph()
        .node(callable.identity)
        .and_then(|node| node.function_parameters())
        .ok_or_else(|| InternalFault::new("replay", "callable-identity-is-a-function-node"))?;
    if parameters.len() != callable.parameters.len() {
        return Err(InternalFault::new(
            "replay",
            "function-node-parameters-match-signature",
        ));
    }
    Ok(parameters)
}

/// The function `name` selects in the recompiled package, by name lookup in
/// its declarations (OQ-5): its one segment and its callable. A name that
/// resolves to none refuses [`ReplayRefusal::UnknownFunction`].
pub(crate) fn lookup<'a>(
    compiled: &'a Recompiled,
    name: &'a QualifiedName,
) -> Result<(&'a Identifier, qsl_semantics::check::CallableFunction<'a>), ReplayRefusal> {
    let unknown = || ReplayRefusal::UnknownFunction {
        selection: name.clone(),
        package: compiled.emitted.package().package_id(),
    };
    let [segment] = name.segments() else {
        return Err(unknown());
    };
    let callable = compiled
        .checked
        .package()
        .graph()
        .callable(segment.as_str())
        .ok_or_else(unknown)?;
    Ok((segment, callable))
}

/// OQ-5: resolve `name` by name lookup in the recompiled package's
/// declarations -- the one name lookup after the check stage (R-06).
/// Complete-V1 declares no qualified names, so only a one-segment name
/// can resolve. A predicate replay of a function whose declared result is
/// not `Boolean` states no property, and a value-parity replay of one whose
/// result is `Boolean` is a predicate's; each refuses here, before any call
/// or charge.
fn select(
    compiled: &Recompiled,
    name: &QualifiedName,
    claim: Claim,
) -> Result<Selected, ReplayRefusal> {
    let package = compiled.checked.package();
    let (segment, callable) = lookup(compiled, name)?;
    match (claim, *callable.result == ValueType::Boolean) {
        (Claim::Predicate, true) | (Claim::ValueParity, false) => {}
        (Claim::Predicate, false) => {
            return Err(ReplayRefusal::NotAPredicate {
                selection: name.clone(),
                package: compiled.emitted.package().package_id(),
            });
        }
        (Claim::ValueParity, true) => {
            return Err(ReplayRefusal::NotAValueFunction {
                selection: name.clone(),
                package: compiled.emitted.package().package_id(),
            });
        }
    }
    let types: Vec<ValueType> = callable
        .parameters
        .iter()
        .map(|(_, value_type)| value_type.clone())
        .collect();
    let parameters = callable_parameter_keys(package, &callable).map_err(ReplayRefusal::Fault)?;
    let name = qsl_eval::value::QualifiedName::unqualified(segment.as_str()).map_err(|_| {
        ReplayRefusal::UnknownFunction {
            selection: name.clone(),
            package: compiled.emitted.package().package_id(),
        }
    })?;
    Ok(Selected {
        name,
        parameters,
        types,
    })
}

/// The call's arguments in declared parameter order (ADR-013 O-25, C-11):
/// an `Input` source's assignments joined by parameter node id, or a
/// `Witness` source's transcript decoded against one binding per
/// parameter, each naming the parameter's node id and declared type.
fn arguments(
    package: &CheckedPackage,
    call: &Selected,
    source: &ReplaySource,
    obligation: ObligationIdentity,
) -> Result<Vec<Value>, ReplayRefusal> {
    let values = match source {
        ReplaySource::Input(assignments) => {
            let mut values: Vec<Option<WitnessValue>> = vec![None; call.parameters.len()];
            for assignment in assignments {
                let position = package
                    .graph()
                    .semantic_graph()
                    .resolve_wire(assignment.parameter)
                    .and_then(|key| call.parameters.iter().position(|&p| p == key))
                    .ok_or(ReplayRefusal::UnknownParameter(assignment.parameter))?;
                let slot = values
                    .get_mut(position)
                    .ok_or(ReplayRefusal::UnknownParameter(assignment.parameter))?;
                if slot.replace(assignment.value).is_some() {
                    return Err(ReplayRefusal::DuplicateArgument(assignment.parameter));
                }
            }
            values
                .into_iter()
                .zip(&call.parameters)
                .map(|(value, parameter)| {
                    value.ok_or(ReplayRefusal::UnboundParameter(wire_id(*parameter)))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        ReplaySource::Witness(witness) => {
            let bindings = call
                .parameters
                .iter()
                .zip(&call.types)
                .enumerate()
                .map(|(position, (parameter, value_type))| {
                    Ok(WitnessBinding {
                        parameter: wire_id(*parameter),
                        value_type: witness_type(position, value_type)?,
                    })
                })
                .collect::<Result<Vec<_>, ReplayRefusal>>()?;
            witness
                .decode(&bindings)
                .map_err(|refusal| ReplayRefusal::Witness {
                    obligation,
                    refusal,
                })?
        }
    };
    values
        .into_iter()
        .zip(&call.types)
        .enumerate()
        .map(|(parameter, (value, value_type))| argument(parameter, value, value_type))
        .collect()
}

/// The witness type a parameter of `value_type` is read as: `Boolean` for
/// `Boolean`, `I64` for an integer type. A type no witness value is a value
/// of refuses `WrongValueKind` before the call.
fn witness_type(
    parameter: usize,
    value_type: &ValueType,
) -> Result<WitnessValueType, ReplayRefusal> {
    match value_type {
        ValueType::Boolean => Ok(WitnessValueType::Boolean),
        ValueType::Integer | ValueType::Int(_) => Ok(WitnessValueType::I64),
        ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::Enum(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Reference(_)
        | ValueType::Population(_) => Err(ReplayRefusal::Input(InputRefusal::WrongValueKind {
            parameter,
        })),
    }
}

/// A typed argument value as a value of its parameter's declared type: a
/// Boolean for `Boolean`, and an integer, widened without loss, for an
/// integer type. Any other pairing refuses `WrongValueKind` before the call
/// -- the refusal S6a admission gives an argument of the wrong kind.
fn argument(
    parameter: usize,
    value: WitnessValue,
    value_type: &ValueType,
) -> Result<Value, ReplayRefusal> {
    match (witness_type(parameter, value_type)?, value) {
        (WitnessValueType::Boolean, WitnessValue::Boolean(value)) => Ok(Value::Boolean(value)),
        (WitnessValueType::I64, WitnessValue::Integer(value)) => {
            Ok(Value::Integer(Integer::from(value)))
        }
        (WitnessValueType::Boolean, WitnessValue::Integer(_))
        | (WitnessValueType::I64, WitnessValue::Boolean(_)) => {
            Err(ReplayRefusal::Input(InputRefusal::WrongValueKind {
                parameter,
            }))
        }
    }
}

/// A checked node's wire spelling, for reporting and for the witness's
/// binding names. Wire ids carry no claim that they resolve.
fn wire_id(key: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

/// The replay's charges: what `meter` consumed of each counter.
pub(crate) fn consumed(meter: &Meter) -> ScalarLimits {
    charges(
        LimitKind::ALL
            .iter()
            .map(|kind| (*kind, meter.consumed(*kind))),
    )
}

/// Per-counter consumed totals as a replay result's charges: one
/// [`ScalarLimits`] member per [`LimitKind`], zero where none is given.
fn charges(consumed: impl IntoIterator<Item = (LimitKind, u64)>) -> ScalarLimits {
    let mut charges = ScalarLimits {
        integer_bits: 0,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 0,
        work_units: 0,
        result_units: 0,
    };
    for (kind, amount) in consumed {
        let counter = match kind {
            LimitKind::IntegerBits => &mut charges.integer_bits,
            LimitKind::DecimalDigits => &mut charges.decimal_digits,
            LimitKind::ScaleExpansion => &mut charges.scale_expansion,
            LimitKind::TextInputBytes => &mut charges.text_input_bytes,
            LimitKind::TextScalars => &mut charges.text_scalars,
            LimitKind::NormalizedScalars => &mut charges.normalized_scalars,
            LimitKind::UnitEdges => &mut charges.unit_edges,
            LimitKind::ValueOccurrences => &mut charges.value_occurrences,
            LimitKind::WorkUnits => &mut charges.work_units,
            LimitKind::ResultUnits => &mut charges.result_units,
        };
        *counter = amount;
    }
    charges
}

#[cfg(test)]
mod tests;
