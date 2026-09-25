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
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::library::PackageId;
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Integer, LimitKind, Meter, NodeKey, Outcome, ScalarLimits, Value};

use crate::bounds::MAX_ENCODED_BYTES;
use crate::identity::{QualifiedName, RawSourceRef};
use crate::proof_result::{ProofCategory, ToolPin};
use crate::request::{ReplayRequest, ReplayRequestRefusal, ReplayRequestWire, StageLimits};
use crate::result::{
    EvaluatedValue, InputArmResult, ReplayResult, SeparatingWitnessRecord, Verdict,
    WitnessArmResult,
};
use crate::spine::{compile, CompileRefusal, Compiled, SpineLimits};
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
    /// S6a admission refused an argument: its type or its declared domain
    /// (`CheckedPackage::call`, carried as ADR-011 §2.3's
    /// `StageFailure::Refused` cause).
    #[error("{code} ({cause}): {refusal}", code = .0.code().as_str(), cause = .0.cause(), refusal = .0)]
    Input(InputRefusal),
    /// The selected function completed a value that is not a Boolean, so
    /// it states no property a counterexample refutes.
    #[error("the selected function {selection} of package {} is not a predicate: it completed a non-Boolean value", .package.hex())]
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
        FamilyOutcome::Evaluated(Outcome::Completed(_)) => {
            return Err(ReplayRefusal::NotAPredicate {
                selection: request.selected_function().clone(),
                package: compiled.emitted.package_id(),
            });
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
            value,
            // A predicate's whole result decides it: the deciding element
            // is the value itself, at no index, path or trace position.
            value.map(|value| SeparatingWitnessRecord {
                deciding_element: value,
                index: 0,
                value_path: Vec::new(),
                trace_position: None,
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

/// Recompile the request's one source unit from the byte provision, with
/// every domain package the provision carries under its `sha256-jcs`
/// digest as I1's package input, and require the recompiled `package_id`
/// to equal the request's.
fn recompile(request: &ReplayRequest) -> Result<Compiled, ReplayRefusal> {
    let limits = spine_limits(request.stage_limits())?;
    if let Some(other) = request
        .source_digests()
        .iter()
        .find(|reference| reference.digest().domain() != DigestDomain::SourceBytesV1)
    {
        return Err(ReplayRefusal::NotASource(Box::new(other.clone())));
    }
    let [source] = request.source_digests() else {
        return Err(ReplayRefusal::SourceCount(request.source_digests().len()));
    };
    let bytes = request
        .byte_provision()
        .get(source.digest())
        .ok_or_else(|| {
            // `ReplayRequest::decode` refuses an incomplete provision.
            ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "decoded-request-byte-provision-complete",
            ))
        })?;
    let packages = package_input(
        request
            .byte_provision()
            .entries()
            .filter(|(digest, _)| digest.domain() == DigestDomain::Sha256Jcs)
            .map(|(_, bytes)| bytes),
    );
    let compiled = compile(
        SourceIdentity::new(
            source.authority(),
            source.identity(),
            source.revision_namespace(),
            source.revision(),
        ),
        source.identity(),
        bytes,
        &packages,
        limits,
    )
    .map_err(ReplayRefusal::Recompile)?;
    let requested = request.package_id();
    let recompiled = compiled.emitted.package_id();
    if requested.domain() != DigestDomain::PackageSemanticV2 || requested.hex() != recompiled.hex()
    {
        return Err(ReplayRefusal::PackageIdMismatch {
            requested,
            recompiled,
        });
    }
    Ok(compiled)
}

/// The selected function: its S6a name and its parameter nodes in
/// declared order.
struct Selected {
    name: qsl_eval::value::QualifiedName,
    parameters: Vec<NodeKey>,
}

/// OQ-5: resolve `name` by name lookup in the recompiled package's
/// declarations -- the one name lookup after the check stage (R-06).
/// Complete-V1 declares no qualified names, so only a one-segment name
/// can resolve.
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
    Ok(Selected { name, parameters })
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
    Ok(values
        .into_iter()
        .map(|value| Value::Integer(Integer::from(value)))
        .collect())
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
