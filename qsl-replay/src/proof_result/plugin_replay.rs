// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-290 "Replay context": the package-wide members of a plugin
//! counterexample's replay request, and the ordered refusals of building it.
//!
//! A plugin counterexample settles only through its S6a replay (FR-289 item
//! 4), and FR-098's `replay` decides one claim: that a Boolean function
//! returns `true`. So [`PluginReplayContext::request`] builds a request only
//! for a function-contract item, the `declaration` occurrence of a checked
//! `Boolean` function. Every other item is non-replayable and settles
//! `inconclusive` with cause `replay-refused` (ADR-029 PL-7).

use std::collections::{BTreeMap, BTreeSet};

use super::{InconclusiveCause, TerminalRecord, TerminalValue};
use crate::execute::{process_provider_obligation, BoundEntries, IdentityEncodeError};
use crate::identity::{DeclaredDomain, QualifiedName, RawSourceRef};
use crate::limits::CallerLimits;
use crate::outcome::OutcomeDiagnostic;
use crate::request::{DependencyEntryWire, ReplayRequestWire, StateEnvironment};
use crate::spine::{AdmittedModels, CheckedUnit, EmittedUnit};
use crate::witness::{CanonicalAssignment, ReplaySource};
use qsl_foundation::bound::ProofBound;
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::source::{Source, SourceLabel};
use qsl_foundation::{BackendId, Code, InternalFault, RequestIndex, Setting, SettingLimits};
use quire_exact::{Identifier, Origin, Role, ScalarLimits, ValueType};

/// The occurrence role and ordinal of a function's declaration (ADR-013 O-09).
const DECLARATION_ROLE: &str = "declaration";

/// A checked function declaration the context can build a request for.
#[derive(Clone, Debug)]
struct FunctionDeclaration {
    /// `None` when the declared name is no identifier, which a checked
    /// package never holds; `request` then reports an internal fault.
    name: Option<Identifier>,
    /// Whether the declared result is `Boolean`: the function-contract
    /// subject (ADR-013 O-09).
    predicate: bool,
}

/// One resolved library selection of the checked package, with the source
/// `check` resolved for it, if it resolved one.
#[derive(Clone, Debug)]
struct DependencySelection {
    identity: String,
    package_id: DigestRecord,
    source: Option<RawSourceRef>,
}

/// The package-wide members of a plugin counterexample's replay request
/// (FR-290 "Replay context"). It copies what it needs from the units and
/// refuses nothing: a pair of units that do not match is caught by the
/// replay's own recompile, which requires the recompiled `package_id` to
/// equal the request's (FR-098).
#[derive(Clone, Debug)]
pub struct PluginReplayContext {
    package_id: DigestRecord,
    source: RawSourceRef,
    dependencies: Vec<DependencySelection>,
    functions: BTreeMap<WireNodeId, FunctionDeclaration>,
    requirements: BTreeSet<OccurrenceKey>,
    byte_provision: Vec<(Option<String>, String, Vec<u8>)>,
    stage_limits: BTreeMap<String, u64>,
    accounting_limits: ScalarLimits,
}

/// The reference a replay request names `source` by: its four FR-001 labels
/// and its `quire.source.bytes/v1` digest.
fn source_reference(source: &Source) -> RawSourceRef {
    let identity = source.identity();
    RawSourceRef::new(
        identity.label(SourceLabel::Authority).to_owned(),
        identity.label(SourceLabel::Identity).to_owned(),
        identity.label(SourceLabel::RevisionNamespace).to_owned(),
        identity.label(SourceLabel::Revision).to_owned(),
        source.reference().digest(),
    )
}

impl PluginReplayContext {
    /// The context of the package `checked` and `emitted`, compiled with
    /// `models` under `limits`: the `CheckedUnit` from `check`, the
    /// `EmittedUnit` that `package` emitted from it, the `AdmittedModels`
    /// from `select`, and the run's `CallerLimits`, whose `spine` limits the
    /// compile ran under (FR-278).
    pub fn new(
        checked: &CheckedUnit,
        emitted: &EmittedUnit,
        models: &AdmittedModels,
        limits: CallerLimits,
    ) -> Self {
        let graph = checked.package().graph();
        let functions = (0..)
            .map_while(|index| graph.function_state(index))
            .filter_map(|state| {
                let callable = graph.callable(state.name)?;
                let name = Identifier::new(state.name).ok();
                Some((
                    WireNodeId::from_digest(*callable.identity.as_bytes()),
                    FunctionDeclaration {
                        name,
                        predicate: *callable.result == ValueType::Boolean,
                    },
                ))
            })
            .collect();
        let dependencies = checked
            .package()
            .dependency_selections()
            .iter()
            .map(|(identity, resolved)| DependencySelection {
                identity: identity.as_str().to_owned(),
                package_id: resolved.package_id.record(),
                source: checked.library_source(identity).map(source_reference),
            })
            .collect();
        let sources = emitted.sources().iter().map(|source| {
            (
                source.reference().digest(),
                source.text().as_bytes().to_vec(),
            )
        });
        let documents = models.packages().iter().map(|(digest, bytes)| {
            (
                DigestRecord::mint(DigestDomain::Sha256Jcs, *digest),
                bytes.clone(),
            )
        });
        let byte_provision = sources
            .chain(documents)
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .map(|(digest, bytes)| {
                (
                    Some(digest.domain().as_str().to_owned()),
                    digest.hex(),
                    bytes,
                )
            })
            .collect();
        let stage_limits = limits
            .bounds()
            .into_iter()
            .filter(|(setting, _)| {
                !matches!(
                    setting,
                    Setting::ReplayInputBytes | Setting::FormatOutputBytes
                )
            })
            .map(|(setting, bound)| (setting.name().to_owned(), bound))
            .collect();
        Self {
            package_id: emitted.package().package_id().record(),
            source: source_reference(checked.source()),
            dependencies,
            functions,
            requirements: graph.requirements().keys().cloned().collect(),
            byte_provision,
            stage_limits,
            accounting_limits: limits.accounting.0,
        }
    }

    /// The `ReplayRequestWire` that replays one plugin counterexample: the
    /// routed item's `occurrence` and the substituted `proof_bounds` of its
    /// extent (which the driver wrote into the FR-331 request item), the
    /// `plugin`'s `BackendId` and the `assignments` that
    /// [`read_plugin_assignments`](crate::read_plugin_assignments) returned.
    /// It does not decode or verify the request: `replay` reads it through
    /// `ReplayRequest::decode` (FR-071).
    ///
    /// # Errors
    ///
    /// The first [`PluginReplayRefusal`], in this order: the occurrence is no
    /// function declaration and no requirement record of the package
    /// (`UnknownOccurrence`); the item is not a function-contract item
    /// (`NotReplayable`); a dependency has no resolved source
    /// (`UnresolvedDependency`, defensive); two bounds name one domain key
    /// (`DuplicateBoundKey`).
    pub fn request(
        &self,
        occurrence: &OccurrenceKey,
        proof_bounds: &[ProofBound],
        plugin: &BackendId,
        assignments: Vec<CanonicalAssignment>,
    ) -> Result<ReplayRequestWire, PluginReplayRefusal> {
        let declaration = Origin::new(Role::new(DECLARATION_ROLE), 0);
        let function = (occurrence.origin() == &declaration)
            .then(|| self.functions.get(&occurrence.node()))
            .flatten();
        if function.is_none() && !self.requirements.contains(occurrence) {
            return Err(PluginReplayRefusal::UnknownOccurrence {
                key: occurrence.clone(),
            });
        }
        let Some(function) = function.filter(|function| function.predicate) else {
            return Err(PluginReplayRefusal::NotReplayable {
                key: occurrence.clone(),
            });
        };
        let dependencies = self
            .dependencies
            .iter()
            .map(|selection| {
                let source = selection.source.as_ref().ok_or_else(|| {
                    PluginReplayRefusal::UnresolvedDependency {
                        identity: selection.identity.clone(),
                    }
                })?;
                Ok(DependencyEntryWire {
                    identity: selection.identity.clone(),
                    package_id: (
                        Some(selection.package_id.domain().as_str().to_owned()),
                        selection.package_id.hex(),
                    ),
                    sources: vec![source.to_wire()],
                })
            })
            .collect::<Result<Vec<_>, PluginReplayRefusal>>()?;
        let bounds = BoundEntries::new(proof_bounds.to_vec()).map_err(|error| match error {
            IdentityEncodeError::DuplicateKey { key } => {
                PluginReplayRefusal::DuplicateBoundKey { key }
            }
            // The preimage's other members are decimal strings and a `u32`,
            // so the encoder refuses nothing else.
            _ => PluginReplayRefusal::Fault(InternalFault::new(
                "plugin-replay",
                "obligation-identity-encodes",
            )),
        })?;
        let obligation =
            process_provider_obligation(occurrence.node(), occurrence.origin(), &bounds).map_err(
                |_| {
                    PluginReplayRefusal::Fault(InternalFault::new(
                        "plugin-replay",
                        "obligation-identity-encodes",
                    ))
                },
            )?;
        let fault = || {
            PluginReplayRefusal::Fault(InternalFault::new(
                "plugin-replay",
                "declared-function-name-is-an-identifier",
            ))
        };
        let name = function.name.clone().ok_or_else(fault)?;
        let selected_function = QualifiedName::new(vec![name]).map_err(|_| fault())?;
        Ok(ReplayRequestWire {
            profile_selections: Vec::new(),
            package_id: (
                Some(self.package_id.domain().as_str().to_owned()),
                self.package_id.hex(),
            ),
            source_digests: vec![self.source.to_wire()],
            dependencies,
            replay_target: crate::RequestTargetWire::function(
                selected_function,
                occurrence.clone(),
            ),
            source: ReplaySource::Input(assignments),
            obligation_identity: *obligation.as_bytes(),
            backend: plugin.as_str().to_owned(),
            state_environment: StateEnvironment::new(Vec::new()),
            accounting_limits: self.accounting_limits,
            stage_limits: self.stage_limits.clone(),
            supplied_limits: None,
            object_environment: None,
            declared_domains: proof_bounds
                .iter()
                .cloned()
                .map(DeclaredDomain::new)
                .collect(),
            byte_provision: self.byte_provision.clone(),
        })
    }
}

/// A plugin counterexample [`PluginReplayContext::request`] could not build a
/// replay request for. [`Self::NotReplayable`] settles the item
/// `inconclusive`; every other refusal settles it `failed`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PluginReplayRefusal {
    /// The occurrence key is neither the `declaration` occurrence of a
    /// checked function declaration nor the occurrence of a requirement
    /// record of the package.
    #[error("missing_declaration/missing-name: the package holds no function declaration or requirement record at {key}")]
    UnknownOccurrence {
        /// The occurrence key.
        key: OccurrenceKey,
    },
    /// The item is not a function-contract item: its counterexample cannot
    /// be replayed as a predicate (FR-098 decides only that a Boolean
    /// function returns `true`).
    #[error("unknown_required_feature/unsupported-feature: the item at {key} is not a function-contract item")]
    NotReplayable {
        /// The occurrence key.
        key: OccurrenceKey,
    },
    /// A resolved library selection has no library source that `check`
    /// resolved. A defensive check: consistent inputs never reach
    /// it.
    #[error(
        "missing_import/missing-selection: no source resolved for the dependency `{identity}`"
    )]
    UnresolvedDependency {
        /// The entry's library identity.
        identity: String,
    },
    /// Two proof bounds name the same domain key, which ADR-013 O-09
    /// forbids.
    #[error("invalid_runtime_input/duplicate-member: two proof bounds name the domain key {key}")]
    DuplicateBoundKey {
        /// The repeated key, in its display form.
        key: String,
    },
    /// An invariant of the checked package failed: consistent inputs never
    /// reach it. The item settles `failed`.
    #[error("runtime_invariant: plugin replay request: {0:?}")]
    Fault(InternalFault),
}

impl PluginReplayRefusal {
    /// The outcome-document diagnostic for this refusal, from QSpec FR-272's
    /// closed table. `item_index` is the result item's index.
    pub fn diagnostic(&self, item_index: usize) -> OutcomeDiagnostic {
        let (code, cause) = match self {
            Self::UnknownOccurrence { .. } => (Code::MissingDeclaration, "missing-name"),
            Self::NotReplayable { .. } => (Code::UnknownRequiredFeature, "unsupported-feature"),
            Self::UnresolvedDependency { .. } => (Code::MissingImport, "missing-selection"),
            Self::DuplicateBoundKey { .. } => (Code::InvalidRuntimeInput, "duplicate-member"),
            Self::Fault(_) => (Code::RuntimeInvariant, "established-invariant-broken"),
        };
        OutcomeDiagnostic::new(
            Some(cause),
            code.as_str(),
            None,
            format!("result item {item_index}: {self}"),
        )
    }

    /// The item's terminal record, naming `plugin`. `NotReplayable` settles
    /// `inconclusive` with `replay-refused` and the code
    /// `unknown_required_feature`, with no replay result; every other
    /// refusal settles `failed`, its diagnostic going in the outcome
    /// document ([`Self::diagnostic`]).
    pub fn record(&self, request_index: RequestIndex, plugin: BackendId) -> TerminalRecord {
        match self {
            Self::NotReplayable { .. } => TerminalRecord::with_backend(
                request_index,
                plugin,
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                    Code::UnknownRequiredFeature,
                )),
            ),
            Self::UnknownOccurrence { .. }
            | Self::UnresolvedDependency { .. }
            | Self::DuplicateBoundKey { .. }
            | Self::Fault(_) => TerminalRecord::failed(request_index, plugin),
        }
    }
}
