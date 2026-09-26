// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098 (ADR-013 O-26, C-13, TK-01; ADR-011 §2.1 E9, §4, §6.1): the
//! replay executor entry, [`replay`].
//!
//! It reads the request (FR-071), recompiles the one source unit the
//! package reference names through the spine ([`crate::spine::compile`],
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

use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, InputRefusal};
use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::{Code, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::library::{LibraryName, PackageId};
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Integer, LimitKind, Meter, NodeKey, Outcome, ScalarLimits, Value, ValueType};

use crate::bounds::MAX_ENCODED_BYTES;
use crate::identity::{QualifiedName, RawSourceRef};
use crate::proof_result::{ProofCategory, ToolPin};
use crate::request::{ReplayRequest, ReplayRequestRefusal, ReplayRequestWire, StageLimits};
use crate::result::{
    EvaluatedValue, InputArmResult, ReplayResult, SeparatingWitnessRecord, Verdict,
    WitnessArmResult,
};
use crate::spine::{
    compile, CompileRefusal, Compiled, DependencyInput, DependencyInputRefusal, SpineLimits,
    SuppliedLibrary,
};
use crate::witness::{DecodeRefusal, ReplaySource};

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
    /// The request's S1 source limit is above the reader limit.
    #[error(
        "stage_limit_exceeded: the request's S1 text_input_bytes {} is above the reader limit {}",
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
    /// (`stale_dependency`/`byte-digest-mismatch`).
    #[error("stale_dependency: the request names {identity} as {} but it recompiles to {}", .requested.hex(), .recompiled.hex())]
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
    #[error("stale_dependency: the request names package {} but the source recompiles to {}", .requested.hex(), .recompiled.hex())]
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
    /// parameters.
    #[error("the witness does not decode: {0}")]
    Witness(DecodeRefusal),
    /// An argument refused admission as `WrongValueKind`: its value is not
    /// of the parameter's declared type, whether that is found before the
    /// call (an integer bound to a Boolean parameter other than 0 or 1, or
    /// to a parameter of a kind an integer never is) or by S6a admission in
    /// `CheckedPackage::call` (a value outside the declared domain, such as
    /// `12` for `Int[0, 9]`) -- carried as ADR-011 §2.3's
    /// `StageFailure::Refused` cause.
    #[error("{code} ({cause}): {refusal}", code = .0.code().as_str(), cause = .0.cause(), refusal = .0)]
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
            Self::LimitAboveReader(_) => Code::StageLimitExceeded,
            Self::NotASource(_) | Self::SourceCount(_) => Code::InvalidRequest,
            Self::Recompile(refusal) => refusal.code(),
            Self::PackageIdMismatch { .. } | Self::DependencyIdentityMismatch { .. } => {
                Code::StaleDependency
            }
            Self::DependencySelections(_) => Code::InvalidPackage,
            Self::DependencyInput(refusal) => refusal.code(),
            Self::UnknownFunction { .. } => Code::MissingDeclaration,
            Self::UnknownParameter(_)
            | Self::DuplicateArgument(_)
            | Self::UnboundParameter(_)
            | Self::Witness(_)
            | Self::NotAPredicate { .. } => Code::InvalidRuntimeInput,
            Self::Input(refusal) => refusal.code(),
            Self::Fault(_) => Code::RuntimeInvariant,
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

/// The executor's toolchain pin in every result it settles (ADR-013 O-27).
const TOOLCHAIN: &str = concat!("qsl-replay/", env!("CARGO_PKG_VERSION"));

/// ADR-013 C-13: replay `wire` and settle its verdict. The counterexample a
/// request replays refuted its property, so the proved verdict is
/// `violation`; the replayed verdict is the selected predicate's: `false`
/// is `violation`, `true` is `success`, and an S6a outcome that completed
/// no value is its own category, which never agrees.
///
/// FR-063 seam: adding a `FamilyOutcome` variant with no arm here fails
/// `--cfg seam_probe` with `E0004`; FR-063-AC-7's `#[deny(...)]` closes the
/// wildcard-arm escape the probe alone cannot see.
#[deny(clippy::wildcard_enum_match_arm)]
pub fn replay(wire: ReplayRequestWire) -> Result<ReplayResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire)?;
    let compiled = recompile(&request)?;
    let package = &compiled.package;
    let call = select(&compiled, request.selected_function())?;
    let arguments = arguments(package, &call, request.source())?;
    let mut meter = Meter::new(request.accounting_limits());
    let evaluation = package
        .call(
            &call.name,
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .map_err(|failure| match failure {
            CallFailure::Input(refusal) => ReplayRefusal::Input(refusal),
            CallFailure::Fault(fault) => ReplayRefusal::Fault(fault),
        })?;
    let (replayed, value) = match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds))) => (
            if holds {
                ProofCategory::Success
            } else {
                ProofCategory::Violation
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
        FamilyOutcome::Evaluated(Outcome::Refused(_)) => (ProofCategory::Refusal, None),
        FamilyOutcome::Evaluated(Outcome::Incomplete(_)) => (ProofCategory::Incomplete, None),
        // O-16's `undefined` row is not a proof category, and a
        // family-owned result is not a kernel verdict: neither agrees.
        FamilyOutcome::Evaluated(Outcome::Undefined(_)) | FamilyOutcome::FamilyEvaluated(_) => {
            (ProofCategory::Inconclusive, None)
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
    let proved = Verdict::from_category(ProofCategory::Violation);
    let regions = evaluation
        .location
        .as_ref()
        .and_then(|location| package.graph().region(location))
        .into_iter()
        .collect();
    let charges = consumed(&meter);
    let pin = ToolPin::new(TOOLCHAIN);
    Ok(match request.source() {
        ReplaySource::Witness(_) => ReplayResult::Witness(WitnessArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            // A predicate's whole result decides it: the deciding element
            // is the value itself, at no index, path or trace position.
            value.map(|value| {
                (
                    value,
                    SeparatingWitnessRecord {
                        deciding_element: value,
                        index: 0,
                        value_path: Vec::new(),
                        trace_position: None,
                    },
                )
            }),
            regions,
            charges,
            pin,
        )),
        ReplaySource::Input(_) => ReplayResult::Input(InputArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            value,
            regions,
            charges,
            pin,
        )),
    })
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

/// Recompile the request's one source unit from the byte provision against
/// the dependency input its `dependencies` entries build, with every domain
/// package the provision carries under its `sha256-jcs` digest as I1's
/// package input, applying ADR-015 D-4's seven rules in order: each rule
/// over every entry, in entry order, before the next.
fn recompile(request: &ReplayRequest) -> Result<Compiled, ReplayRefusal> {
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
                version: entry.version().to_owned(),
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
    let packages = package_input(
        request
            .byte_provision()
            .entries()
            .filter(|(digest, _)| digest.domain() == DigestDomain::Sha256Jcs)
            .map(|(_, bytes)| bytes),
    );
    // Rule 4: the recompile.
    let compiled = compile(
        labels(source),
        source.identity(),
        bytes,
        &packages,
        &dependencies,
        limits,
    )
    .map_err(ReplayRefusal::Recompile)?;
    // Rule 5: the proved package's `package_id`.
    let requested = request.package_id();
    let recompiled = compiled.emitted.package_id();
    if !recompiled.matches(&requested) {
        return Err(ReplayRefusal::PackageIdMismatch {
            requested,
            recompiled,
        });
    }
    // Rules 6 and 7: every entry names a selection of the recompiled
    // closure, then at its recomputed `package_id`.
    let selections = compiled.package.dependency_selections();
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
        let selected = selected.selection.package_id;
        if !selected.matches(&entry.package_id()) {
            return Err(ReplayRefusal::DependencyIdentityMismatch {
                identity: entry.identity().clone(),
                requested: entry.package_id(),
                recompiled: selected,
            });
        }
    }
    Ok(compiled)
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

/// The selected function: its S6a name, and its parameter nodes and
/// declared types in declared order.
struct Selected {
    name: qsl_eval::value::QualifiedName,
    parameters: Vec<NodeKey>,
    types: Vec<ValueType>,
}

/// OQ-5: resolve `name` by name lookup in the recompiled package's
/// declarations -- the one name lookup after the check stage (R-06).
/// Complete-V1 declares no qualified names, so only a one-segment name
/// can resolve. A function whose declared result is not `Boolean` states
/// no property, and refuses here, before any call or charge.
fn select(compiled: &Compiled, name: &QualifiedName) -> Result<Selected, ReplayRefusal> {
    let package = &compiled.package;
    let unknown = || ReplayRefusal::UnknownFunction {
        selection: name.clone(),
        package: compiled.emitted.package_id(),
    };
    let [segment] = name.segments() else {
        return Err(unknown());
    };
    let callable = package
        .graph()
        .callable(segment.as_str())
        .ok_or_else(unknown)?;
    if *callable.result != ValueType::Boolean {
        return Err(ReplayRefusal::NotAPredicate {
            selection: name.clone(),
            package: compiled.emitted.package_id(),
        });
    }
    let types: Vec<ValueType> = callable
        .parameters
        .iter()
        .map(|(_, value_type)| value_type.clone())
        .collect();
    let parameters = package
        .graph()
        .semantic_graph()
        .node(callable.identity)
        .and_then(|node| node.function_parameters())
        .ok_or(ReplayRefusal::Fault(InternalFault::new(
            "replay",
            "callable-identity-is-a-function-node",
        )))?;
    let name =
        qsl_eval::value::QualifiedName::unqualified(segment.as_str()).map_err(|_| unknown())?;
    if parameters.len() != types.len() {
        return Err(ReplayRefusal::Fault(InternalFault::new(
            "replay",
            "function-node-parameters-match-signature",
        )));
    }
    Ok(Selected {
        name,
        parameters,
        types,
    })
}

/// The call's arguments in declared parameter order (ADR-013 O-25, C-11):
/// an `Input` source's assignments joined by parameter node id, or a
/// `Witness` source's transcript decoded by each parameter's node id.
fn arguments(
    package: &CheckedPackage,
    call: &Selected,
    source: &ReplaySource,
) -> Result<Vec<Value>, ReplayRefusal> {
    let values = match source {
        ReplaySource::Input(assignments) => {
            let mut values: Vec<Option<i64>> = vec![None; call.parameters.len()];
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
            let order: Vec<String> = call
                .parameters
                .iter()
                .map(|parameter| wire_id(*parameter).to_string())
                .collect();
            witness.decode(&order).map_err(ReplayRefusal::Witness)?
        }
    };
    values
        .into_iter()
        .zip(&call.types)
        .enumerate()
        .map(|(parameter, (value, value_type))| argument(parameter, value, value_type))
        .collect()
}

/// A canonical integer assignment as a value of its parameter's declared
/// type: an integer for an integer type, and `0` or `1` for `Boolean`
/// (`false`, `true`). Any other value, or a type no integer is a value of,
/// refuses `WrongValueKind` before the call -- the refusal S6a admission
/// gives an argument of the wrong kind.
fn argument(parameter: usize, value: i64, value_type: &ValueType) -> Result<Value, ReplayRefusal> {
    let wrong = || ReplayRefusal::Input(InputRefusal::WrongValueKind { parameter });
    match value_type {
        ValueType::Boolean => match value {
            0 => Ok(Value::Boolean(false)),
            1 => Ok(Value::Boolean(true)),
            _ => Err(wrong()),
        },
        ValueType::Integer | ValueType::Int(_) => Ok(Value::Integer(Integer::from(value))),
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
        | ValueType::Population(_) => Err(wrong()),
    }
}

/// A checked node's wire spelling, for reporting and for the witness's
/// binding names. Wire ids carry no claim that they resolve.
fn wire_id(key: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

/// The replay's charges: what `meter` consumed of each counter.
fn consumed(meter: &Meter) -> ScalarLimits {
    ScalarLimits {
        integer_bits: meter.consumed(LimitKind::IntegerBits),
        decimal_digits: meter.consumed(LimitKind::DecimalDigits),
        scale_expansion: meter.consumed(LimitKind::ScaleExpansion),
        text_input_bytes: meter.consumed(LimitKind::TextInputBytes),
        text_scalars: meter.consumed(LimitKind::TextScalars),
        normalized_scalars: meter.consumed(LimitKind::NormalizedScalars),
        unit_edges: meter.consumed(LimitKind::UnitEdges),
        value_occurrences: meter.consumed(LimitKind::ValueOccurrences),
        work_units: meter.consumed(LimitKind::WorkUnits),
        result_units: meter.consumed(LimitKind::ResultUnits),
    }
}

#[cfg(test)]
mod tests;
