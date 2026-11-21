// SPDX-License-Identifier: AGPL-3.0-or-later
//! The selection sidecar, `filament.checked-handoff.selection/1` (FR-517), and
//! its comparison with an admitted package.

use std::collections::BTreeMap;
use std::fmt;

use filament_contract_model::{
    CheckedNodeId, CheckedNodeKind, CheckedPackageReadLimits, CheckedPackageV2, CheckedTerm,
    ClaimForm, ClockConfiguration, ExecutionPoint, ProtocolForm, TemporalForm,
};
use fsl_semantics::check::{Activation, TemporalProfile};
use serde::{Deserialize, Serialize};

use super::{Error, SELECTION_FORMAT};
use crate::emit_checked_v2::{ClockSelection, Compiled, Inputs, Request};

/// The authority of every profile definition FSL selects.
const PROFILE_AUTHORITY: &str = "agent-ix";

/// The read limits the writer used.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitRows {
    bytes: u64,
    nodes: u64,
    edges: u64,
    occurrences: u64,
    diagnostics: u64,
    work: u64,
}

impl From<CheckedPackageReadLimits> for LimitRows {
    fn from(limits: CheckedPackageReadLimits) -> Self {
        Self {
            bytes: limits.bytes,
            nodes: limits.nodes,
            edges: limits.edges,
            occurrences: limits.occurrences,
            diagnostics: limits.diagnostics,
            work: limits.work,
        }
    }
}

/// One domain package document under `models/`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelRow {
    file: String,
    identity: String,
}

/// One library source under `sources/`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryRow {
    identity: String,
    file: String,
}

/// Which files are evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceRows {
    models: Vec<ModelRow>,
    libraries: Vec<LibraryRow>,
}

/// The package-level rows.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageRows {
    contract_version: String,
    /// The package's manifest name.
    package_name: String,
    /// The unit's file under `sources/`.
    unit: String,
    /// The unit's caller-selected identity, retained for byte-identical
    /// command-handoff recompilation.
    source_identity: SourceIdentityRow,
    /// The accepted state-clause execution points, by authored clause name.
    execution_points: BTreeMap<String, ExecutionPoint>,
    read_limits: LimitRows,
    evidence: EvidenceRows,
    /// Nodes by form, for the forms the checked unit names.
    node_counts: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceIdentityRow {
    authority: String,
    identity: String,
}

/// A protocol requirement entry that names a clause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequirementRow {
    protocol: String,
    node: CheckedNodeId,
}

/// How a clause activates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivationRow {
    /// `origin` or `each`.
    form: String,
    /// The trigger binder of an `each` activation.
    trigger: Option<String>,
    /// Whether a guard is written.
    guarded: bool,
}

/// The profile selection of a clause's law.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileRow {
    authority: String,
    identity: String,
    revision: String,
}

/// The expected `TemporalBinding` row.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingRow {
    clock_name: String,
    clock: ClockConfiguration,
}

/// One temporal clause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClauseRow {
    node: CheckedNodeId,
    name: String,
    requirements: Vec<RequirementRow>,
    activation: ActivationRow,
    profile: ProfileRow,
    binding: Option<BindingRow>,
}

/// One `compensate` template: its node and expected clock row.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompensationRow {
    node: CheckedNodeId,
    name: String,
    binding: Option<BindingRow>,
}

/// One quantifier of a hyper clause and the `TemporalBinding` row its trace
/// is expected to have (FR-488 "Clause names and clock bindings").
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceRow {
    trace: String,
    binding: Option<BindingRow>,
}

/// One hyper clause: its `hyperproperty` node, the `claim` occurrence of that
/// node it is, and one expected row per quantifier in quantifier order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HyperRow {
    node: CheckedNodeId,
    name: String,
    occurrence: u64,
    traces: Vec<TraceRow>,
}

/// The selection sidecar: what a consumer expects of the package beside it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    format: String,
    package: PackageRows,
    clauses: Vec<ClauseRow>,
    compensations: Vec<CompensationRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hypers: Vec<HyperRow>,
}

/// The axis along which a package departs from its sidecar.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SelectionAxis {
    /// A clause's declared name.
    ClauseName,
    /// The protocol requirement entries that name a clause.
    ClauseRequirement,
    /// A clause's node id.
    ClauseNode,
    /// A clause's activation form.
    ClauseActivation,
    /// The identity of a clause's profile law.
    DefinitionIdentity,
    /// The revision of a clause's profile law.
    DefinitionRevision,
    /// The authority of a clause's profile law.
    DefinitionAuthority,
    /// The sequence authority of an event-position row.
    ClockSequenceAuthority,
    /// The epoch of a fixed-sample row.
    ClockEpoch,
    /// The period of a fixed-sample row.
    ClockPeriod,
    /// The unit of a fixed-sample row.
    ClockUnit,
    /// The timestamp unit of a timestamped-event row.
    ClockTimestampUnit,
    /// The clock alternative of a clause's binding row.
    ClockKind,
    /// The clock name of a clause's binding row.
    ClockName,
    /// A clause's binding row is missing, or the package holds one the
    /// sidecar does not.
    ClauseBinding,
    /// The package's count of nodes of a form the sidecar counts.
    PackageNodes,
}

impl SelectionAxis {
    /// The dotted name the corpus manifest writes.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ClauseName => "clause.name",
            Self::ClauseRequirement => "clause.requirement",
            Self::ClauseNode => "clause.node",
            Self::ClauseActivation => "clause.activation",
            Self::DefinitionIdentity => "definition.identity",
            Self::DefinitionRevision => "definition.revision",
            Self::DefinitionAuthority => "definition.authority",
            Self::ClockSequenceAuthority => "clock.sequence_authority",
            Self::ClockEpoch => "clock.epoch",
            Self::ClockPeriod => "clock.period",
            Self::ClockUnit => "clock.unit",
            Self::ClockTimestampUnit => "clock.timestamp_unit",
            Self::ClockKind => "clock.kind",
            Self::ClockName => "clock.name",
            Self::ClauseBinding => "clause.binding",
            Self::PackageNodes => "package.nodes",
        }
    }

    /// The axis a manifest names, if it is one.
    pub fn from_name(name: &str) -> Option<Self> {
        [
            Self::ClauseName,
            Self::ClauseRequirement,
            Self::ClauseNode,
            Self::ClauseActivation,
            Self::DefinitionIdentity,
            Self::DefinitionRevision,
            Self::DefinitionAuthority,
            Self::ClockSequenceAuthority,
            Self::ClockEpoch,
            Self::ClockPeriod,
            Self::ClockUnit,
            Self::ClockTimestampUnit,
            Self::ClockKind,
            Self::ClockName,
            Self::ClauseBinding,
            Self::PackageNodes,
        ]
        .into_iter()
        .find(|axis| axis.name() == name)
    }
}

impl fmt::Display for SelectionAxis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// A package the sidecar does not admit.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the package departs from the selection on {axis} (clause {clause:?})")]
pub struct SelectionRefusal {
    /// The one axis of the departure.
    pub axis: SelectionAxis,
    /// The authored name of the clause the sidecar row names, when the
    /// departure belongs to one.
    pub clause: Option<String>,
}

/// A sidecar that does not parse.
#[derive(Debug, thiserror::Error)]
pub enum SelectionReadError {
    /// The bytes are not a sidecar of this format.
    #[error("not a selection sidecar: {0}")]
    Malformed(#[from] serde_json::Error),
    /// The sidecar is of another format.
    #[error("selection format {0:?} is not {SELECTION_FORMAT}")]
    Format(String),
}

fn text(value: &str) -> String {
    value.to_owned()
}

/// A profile identity split into the definition and its trailing revision.
fn split_revision(identity: &str) -> (&str, &str) {
    identity.rsplit_once('/').unwrap_or((identity, ""))
}

/// Whether `format` spells the selection sidecar format this reader admits.
#[ix_trace_rs::string_edge]
fn is_selection_format(format: &str) -> bool {
    format == SELECTION_FORMAT
}

/// The path of the `sources/<kind><path>` file `file`, for the label kind
/// `kind` (`source:` or `library:`) the writer gives it.
#[ix_trace_rs::string_edge]
fn labelled_path(file: &str, kind: &str) -> Option<std::path::PathBuf> {
    file.strip_prefix("sources/")
        .and_then(|label| label.strip_prefix(kind))
        .map(std::path::PathBuf::from)
}

impl Selection {
    /// Parse a sidecar.
    pub fn read(bytes: &[u8]) -> Result<Self, SelectionReadError> {
        let selection: Self = serde_json::from_slice(bytes)?;
        if !is_selection_format(&selection.format) {
            return Err(SelectionReadError::Format(selection.format));
        }
        Ok(selection)
    }

    /// The sidecar's bytes, deterministic.
    pub(super) fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = serde_json::to_vec_pretty(self).map_err(|source| Error::Encode {
            what: "selection.json",
            source,
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// The sidecar of `compiled` under `request`, from the in-process checked
    /// unit. `bytes` only joins each node to its wire id.
    pub(super) fn derive(
        compiled: &Compiled,
        request: &Request,
        bytes: &[u8],
        command_source_file: Option<&str>,
    ) -> Result<Self, Error> {
        let graph = compiled.checked.package().graph();
        let wire = fsl_semantics::check::wire_id::WireGraph::of_package_bytes(bytes)
            .map_err(|what| Error::Unnameable { what })?;
        let id = |key: &filament_exact::NodeKey, what: &str| {
            wire.wire_id(key).map_err(|_| Error::Unnameable {
                what: what.to_owned(),
            })
        };
        let clauses = graph.temporal_clauses();
        let mut rows = Vec::with_capacity(clauses.len());
        for (position, node) in clauses.iter().enumerate() {
            let clause = node.clause();
            let mut requirements = Vec::new();
            for protocol in graph.protocols() {
                let named = protocol
                    .requirements
                    .iter()
                    .any(|requirement| requirement.temporal == position);
                if !named {
                    continue;
                }
                let key = graph
                    .protocol_node_keys(&protocol.name)
                    .and_then(|keys| keys.clause.as_ref())
                    .ok_or_else(|| Error::Unnameable {
                        what: format!("protocol clause node of {}", protocol.name),
                    })?;
                requirements.push(RequirementRow {
                    protocol: protocol.name.clone(),
                    node: id(key, "protocol clause id")?,
                });
            }
            let (form, trigger, guarded) = match clause.activation() {
                Activation::Origin => ("origin", None, false),
                Activation::Each { guarded } => (
                    "each",
                    // The trigger binder's name only: the sidecar does not
                    // record the trigger's type, which has no stable spelling.
                    clause.trigger().map(|(name, _)| text(name)),
                    guarded,
                ),
            };
            let profile = clause.profile().identity();
            let (definition, revision) = split_revision(profile);
            let binding = request
                .temporal_selections
                .iter()
                .find(|selection| selection.clause == clause.name())
                .map(|selection| BindingRow {
                    clock_name: text(clause.clock()),
                    clock: selection.clock.clone(),
                });
            rows.push(ClauseRow {
                node: id(&node.identity(), "clause id")?,
                name: text(clause.name()),
                requirements,
                activation: ActivationRow {
                    form: text(form),
                    trigger,
                    guarded,
                },
                profile: ProfileRow {
                    authority: text(PROFILE_AUTHORITY),
                    identity: text(definition),
                    revision: text(revision),
                },
                binding,
            });
        }
        let mut compensation_rows = Vec::new();
        for protocol in graph.protocols() {
            let keys = graph.protocol_node_keys(&protocol.name);
            for (index, template) in protocol.obligations.compensations.iter().enumerate() {
                let key = keys
                    .and_then(|keys| keys.compensations.get(&index))
                    .ok_or_else(|| Error::Unnameable {
                        what: format!("compensation node of {}", template.name),
                    })?;
                let binding = request
                    .temporal_selections
                    .iter()
                    .find(|selection| selection.clause == template.name)
                    .map(|selection| BindingRow {
                        clock_name: text(&template.clock),
                        clock: selection.clock.clone(),
                    });
                compensation_rows.push(CompensationRow {
                    node: id(key, "compensation id")?,
                    name: text(&template.name),
                    binding,
                });
            }
        }
        let mut hyper_rows = Vec::new();
        let derived =
            fsl_replay::spine::RequestClocks::model_selections(compiled.checked.package())
                .map_err(|refusal| Error::Unnameable {
                    what: format!("model clock selections: {refusal}"),
                })?;
        let mut hyper_nodes = std::collections::BTreeSet::new();
        let mut hyper_wrappers = std::collections::BTreeSet::new();
        for hyper in graph.hyper_clauses() {
            let occurrence = hyper
                .assembled_occurrence()
                .ok_or_else(|| Error::Unnameable {
                    what: format!("hyperproperty node of {}", hyper.name()),
                })?;
            hyper_nodes.insert(occurrence.node().clone());
            hyper_wrappers.insert(
                fsl_package::hyper_wrapper(graph, occurrence.node()).map_err(|_| {
                    Error::Unnameable {
                        what: format!("body wrapper of {}", hyper.name()),
                    }
                })?,
            );
            let clocked = matches!(
                hyper.profile(),
                TemporalProfile::EventPositionFalseExtension
                    | TemporalProfile::FixedSampleFalseExtension
                    | TemporalProfile::TimestampedEventFiniteWindow
            );
            let named = request
                .temporal_selections
                .iter()
                .find(|selection| clocked && selection.clause == hyper.name());
            let clock_name = || {
                fsl_package::hyper_clock_name(graph, occurrence.node())
                    .map(text)
                    .ok_or_else(|| Error::Unnameable {
                        what: format!("clock of {}", hyper.name()),
                    })
            };
            let named_binding = named
                .map(|selection| {
                    clock_name().map(|clock_name| BindingRow {
                        clock_name,
                        clock: selection.clock.clone(),
                    })
                })
                .transpose()?;
            let mut traces = Vec::with_capacity(hyper.quantifiers().len());
            for quantifier in hyper.quantifiers() {
                // A clock the model derives is not named by the request: each
                // trace's row is the one its model derives (FR-640).
                let binding = match &named_binding {
                    Some(binding) => Some(binding.clone()),
                    None if clocked => derived
                        .iter()
                        .find(|held| {
                            held.declaration_span == hyper.span()
                                && held.trace.as_deref() == Some(quantifier.variable.as_str())
                        })
                        .map(|held| {
                            clock_name().map(|clock_name| BindingRow {
                                clock_name,
                                clock: held.clock.clone(),
                            })
                        })
                        .transpose()?,
                    None => None,
                };
                traces.push(TraceRow {
                    trace: text(&quantifier.variable),
                    binding,
                });
            }
            hyper_rows.push(HyperRow {
                node: id(occurrence.node(), "hyperproperty id")?,
                name: text(hyper.name()),
                occurrence: occurrence.origin().ordinal(),
                traces,
            });
        }
        let mut node_counts = BTreeMap::new();
        // Structurally equal clauses share one node (FSpec FR-322 "Temporal
        // clause names"), so the package counts nodes, not declarations. A
        // hybrid claim and a hyper clause's body wrapper mint the same form
        // (FR-517 "node counts").
        let clause_nodes: std::collections::BTreeSet<filament_exact::NodeKey> = clauses
            .iter()
            .map(|clause| clause.identity())
            .chain(
                graph
                    .hybrid_claim_sites()
                    .iter()
                    .map(|site| site.identity.clone()),
            )
            .chain(hyper_wrappers)
            .collect();
        node_counts.insert(text("claim/hyperproperty"), hyper_nodes.len() as u64);
        node_counts.insert(text("temporal/temporal_clause"), clause_nodes.len() as u64);
        let protocol_clauses = graph
            .protocols()
            .iter()
            .filter(|protocol| {
                graph
                    .protocol_node_keys(&protocol.name)
                    .is_some_and(|keys| keys.clause.is_some())
            })
            .count();
        node_counts.insert(text("protocol/protocol_clause"), protocol_clauses as u64);
        let compensations: usize = graph
            .protocols()
            .iter()
            .filter_map(|protocol| graph.protocol_node_keys(&protocol.name))
            .map(|keys| keys.compensations.len())
            .sum();
        node_counts.insert(text("protocol/compensation"), compensations as u64);
        let mut models = Vec::with_capacity(compiled.models.len());
        for (index, model) in compiled.models.iter().enumerate() {
            let input =
                fsl_semantics::model::intake::package_input([model.as_slice()]).map_err(|_| {
                    Error::Unnameable {
                        what: format!("identity of models/{index}.json"),
                    }
                })?;
            let identity = input
                .keys()
                .next()
                .map(|identity| text(identity.as_str()))
                .ok_or_else(|| Error::Unnameable {
                    what: format!("identity of models/{index}.json"),
                })?;
            models.push(ModelRow {
                file: format!("models/{index}.json"),
                identity,
            });
        }
        let mut libraries: Vec<_> = compiled
            .libraries
            .iter()
            .map(|library| LibraryRow {
                identity: text(&library.identity),
                file: format!("sources/{}", library.source.identity),
            })
            .collect();
        libraries.sort_by(|left, right| left.identity.cmp(&right.identity));
        Ok(Self {
            format: text(SELECTION_FORMAT),
            package: PackageRows {
                contract_version: text(filament_contract_model::CHECKED_PACKAGE_V2),
                package_name: text(&request.package_name),
                unit: command_source_file.map_or_else(
                    || format!("sources/{}", compiled.checked.source().identity().identity),
                    |file| format!("sources/source:{file}"),
                ),
                source_identity: SourceIdentityRow {
                    authority: compiled.checked.source().identity().authority.clone(),
                    identity: compiled.checked.source().identity().identity.clone(),
                },
                execution_points: compiled.points.clone(),
                read_limits: super::READ_LIMITS.into(),
                evidence: EvidenceRows { models, libraries },
                node_counts,
            },
            clauses: rows,
            compensations: compensation_rows,
            hypers: hyper_rows,
        })
    }

    /// The compile inputs the sidecar names: the unit and library sources and
    /// the domain package documents, each read by `read` from its path inside
    /// the handoff directory, with the package name, the execution points and
    /// the clock selection of each binding row. A file label that is not the
    /// `source:` or `library:` label the writer gives is [`Error::Label`].
    pub(super) fn inputs(
        &self,
        read: impl Fn(&str) -> Result<Vec<u8>, Error>,
    ) -> Result<Inputs, Error> {
        let labelled = |file: &str, kind: &str| {
            labelled_path(file, kind).ok_or_else(|| Error::Label {
                label: file.to_owned(),
            })
        };
        let rows = &self.package;
        let libraries = rows
            .evidence
            .libraries
            .iter()
            .map(|row| {
                let path = labelled(&row.file, "library:")?;
                Ok(crate::emit_checked_v2::supplied_library(
                    row.identity.clone(),
                    &path,
                    read(&row.file)?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let models = rows
            .evidence
            .models
            .iter()
            .map(|row| read(&row.file))
            .collect::<Result<Vec<_>, Error>>()?;
        let temporal_selections = self
            .clauses
            .iter()
            .map(|row| (&row.name, &row.binding))
            .chain(
                self.compensations
                    .iter()
                    .map(|row| (&row.name, &row.binding)),
            )
            .chain(self.hypers.iter().filter_map(|row| {
                // Traces whose clocks differ are the model's: the compile
                // derives them again, so no clause-name selection is read.
                let first = row.traces.first()?;
                row.traces
                    .iter()
                    .all(|trace| trace.binding == first.binding)
                    .then_some((&row.name, &first.binding))
            }))
            .filter_map(|(name, binding)| {
                binding.as_ref().map(|binding| ClockSelection {
                    clause: name.clone(),
                    clock: binding.clock.clone(),
                })
            })
            .collect();
        Ok(Inputs {
            source_path: labelled(&rows.unit, "source:")?,
            source_identity: Some(fsl_foundation::SourceIdentity::new(
                rows.source_identity.authority.clone(),
                rows.source_identity.identity.clone(),
            )),
            source: read(&rows.unit)?,
            models,
            libraries,
            package_name: rows.package_name.clone(),
            points: rows.execution_points.clone(),
            temporal_selections,
        })
    }

    /// Compare an admitted package with the sidecar.
    pub fn check(&self, package: &CheckedPackageV2) -> Result<(), SelectionRefusal> {
        let counts = form_counts(package);
        for (form, expected) in &self.package.node_counts {
            if counts.get(form.as_str()).copied().unwrap_or(0) != *expected {
                return Err(refusal(SelectionAxis::PackageNodes, None));
            }
        }
        for row in &self.clauses {
            check_clause(row, package)?;
        }
        for row in &self.compensations {
            check_binding(&row.node, &row.name, row.binding.as_ref(), package)?;
        }
        for row in &self.hypers {
            check_hyper(row, package)?;
        }
        let rows = package.temporal_bindings().map_or(0, <[_]>::len);
        let expected = self
            .clauses
            .iter()
            .map(|row| &row.binding)
            .chain(self.compensations.iter().map(|row| &row.binding))
            .filter(|binding| binding.is_some())
            .count()
            + self
                .hypers
                .iter()
                .flat_map(|row| &row.traces)
                .filter(|row| row.binding.is_some())
                .count();
        if rows != expected {
            return Err(refusal(SelectionAxis::ClauseBinding, None));
        }
        Ok(())
    }
}

fn refusal(axis: SelectionAxis, row: Option<&ClauseRow>) -> SelectionRefusal {
    SelectionRefusal {
        axis,
        clause: row.map(|row| row.name.clone()),
    }
}

/// The package's node count for each form the sidecar counts.
fn form_counts(package: &CheckedPackageV2) -> BTreeMap<&'static str, u64> {
    let mut counts = BTreeMap::new();
    for kind in package.node_kinds() {
        let form = match kind {
            CheckedNodeKind::Temporal(TemporalForm::TemporalClause) => "temporal/temporal_clause",
            CheckedNodeKind::Protocol(ProtocolForm::ProtocolClause) => "protocol/protocol_clause",
            CheckedNodeKind::Protocol(ProtocolForm::Compensation) => "protocol/compensation",
            CheckedNodeKind::Claim(ClaimForm::Hyperproperty) => "claim/hyperproperty",
            _ => continue,
        };
        *counts.entry(form).or_insert(0) += 1;
    }
    counts
}

/// The argument terms of a node whose body is an application.
fn arguments<'a>(package: &'a CheckedPackageV2, id: &CheckedNodeId) -> Option<&'a [CheckedTerm]> {
    match package.term(id)? {
        CheckedTerm::Application { arguments, .. } => Some(arguments),
        _ => None,
    }
}

/// The text of a literal term.
fn literal_text(term: &CheckedTerm) -> Option<&str> {
    match term {
        CheckedTerm::Literal { value, .. } => value.as_str(),
        _ => None,
    }
}

/// The members of the aggregate bound to `name` among `arguments`.
fn binding_members<'a>(arguments: &'a [CheckedTerm], name: &str) -> Option<&'a [CheckedTerm]> {
    arguments.iter().find_map(|argument| match argument {
        CheckedTerm::Binding { name: bound, value, .. }
            if matches!(bound.kind(), filament_contract_model::CheckedBindingNameKind::Other(other) if other == name) =>
        {
            match value.as_ref() {
                CheckedTerm::Aggregate { members } => Some(members.as_slice()),
                _ => None,
            }
        }
        _ => None,
    })
}

fn named_literal<'a>(members: &'a [CheckedTerm], name: &str) -> Option<&'a str> {
    members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name: bound, value, .. }
            if matches!(bound.kind(), filament_contract_model::CheckedBindingNameKind::Other(other) if other == name) =>
        {
            literal_text(value)
        }
        _ => None,
    })
}

fn has_binding(members: &[CheckedTerm], name: &str) -> bool {
    members.iter().any(|member| {
        matches!(member, CheckedTerm::Binding { name: bound, .. }
            if matches!(bound.kind(), filament_contract_model::CheckedBindingNameKind::Other(other) if other == name))
    })
}

fn check_clause(row: &ClauseRow, package: &CheckedPackageV2) -> Result<(), SelectionRefusal> {
    let node = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.node_id == row.node)
        .ok_or_else(|| refusal(SelectionAxis::ClauseNode, Some(row)))?;
    if !package
        .temporal_clause_names()
        .unwrap_or_default()
        .iter()
        .any(|name| name.declaration == node.node_id && name.name.as_ref() == row.name.as_str())
    {
        return Err(refusal(SelectionAxis::ClauseName, Some(row)));
    }
    let mut named_by = Vec::new();
    for (index, kind) in package.node_kinds().iter().enumerate() {
        if *kind != CheckedNodeKind::Protocol(ProtocolForm::ProtocolClause) {
            continue;
        }
        let Some(protocol) = package.resolved_graph().nodes.get(index) else {
            continue;
        };
        let requires = arguments(package, &protocol.node_id)
            .and_then(|arguments| binding_members(arguments, "requirements"))
            .is_some_and(|members| {
                members.iter().any(|member| {
                    matches!(member, CheckedTerm::Reference { target } if *target == row.node)
                })
            });
        if requires {
            named_by.push(&protocol.node_id);
        }
    }
    let mut expected: Vec<&CheckedNodeId> = row.requirements.iter().map(|r| &r.node).collect();
    expected.sort();
    named_by.sort();
    if expected != named_by {
        return Err(refusal(SelectionAxis::ClauseRequirement, Some(row)));
    }
    let activation = arguments(package, &row.node)
        .and_then(|arguments| binding_members_or_aggregate(arguments, 2));
    let form = activation.and_then(|members| named_literal(members, "kind"));
    let guarded = activation.is_some_and(|members| has_binding(members, "guard"));
    if form != Some(row.activation.form.as_str()) || guarded != row.activation.guarded {
        return Err(refusal(SelectionAxis::ClauseActivation, Some(row)));
    }
    check_profile(row, node)?;
    check_binding(&row.node, &row.name, row.binding.as_ref(), package)
}

/// The aggregate at argument position `index`.
fn binding_members_or_aggregate(arguments: &[CheckedTerm], index: usize) -> Option<&[CheckedTerm]> {
    match arguments.get(index)? {
        CheckedTerm::Aggregate { members } => Some(members),
        _ => None,
    }
}

fn check_profile(
    row: &ClauseRow,
    node: &filament_contract_model::CheckedSemanticNodeV2,
) -> Result<(), SelectionRefusal> {
    let law = match &node.body {
        CheckedTerm::Application { operation, .. } => operation
            .law_definitions()
            .ok()
            .and_then(|laws| laws.first()),
        _ => None,
    }
    .ok_or_else(|| refusal(SelectionAxis::DefinitionIdentity, Some(row)))?;
    let (definition, revision) = split_revision(&law.identity);
    if definition != row.profile.identity {
        return Err(refusal(SelectionAxis::DefinitionIdentity, Some(row)));
    }
    if revision != row.profile.revision {
        return Err(refusal(SelectionAxis::DefinitionRevision, Some(row)));
    }
    if *law.authority != *row.profile.authority {
        return Err(refusal(SelectionAxis::DefinitionAuthority, Some(row)));
    }
    Ok(())
}

/// A hyper clause's rows: one per quantifier, keyed on its `hyperproperty`
/// node with the quantifier's `trace`.
fn check_hyper(row: &HyperRow, package: &CheckedPackageV2) -> Result<(), SelectionRefusal> {
    for expected in &row.traces {
        let actual = package.temporal_bindings().and_then(|rows| {
            rows.iter().find(|held| {
                held.declaration == row.node
                    && held.occurrence == row.occurrence
                    && held.trace.as_deref() == Some(expected.trace.as_str())
            })
        });
        compare_binding(&row.name, expected.binding.as_ref(), actual)?;
    }
    Ok(())
}

fn check_binding(
    node: &CheckedNodeId,
    name: &str,
    binding: Option<&BindingRow>,
    package: &CheckedPackageV2,
) -> Result<(), SelectionRefusal> {
    let actual = package
        .temporal_bindings()
        .and_then(|rows| rows.iter().find(|row| row.declaration == *node));
    compare_binding(name, binding, actual)
}

/// The package's row `actual` against the sidecar's expected `binding`.
fn compare_binding(
    name: &str,
    binding: Option<&BindingRow>,
    actual: Option<&filament_contract_model::TemporalBinding>,
) -> Result<(), SelectionRefusal> {
    let refused = |axis| SelectionRefusal {
        axis,
        clause: Some(name.to_owned()),
    };
    let (Some(expected), Some(actual)) = (binding, actual) else {
        return match (binding, actual) {
            (None, None) => Ok(()),
            _ => Err(refused(SelectionAxis::ClauseBinding)),
        };
    };
    if *actual.clock_name != *expected.clock_name {
        return Err(refused(SelectionAxis::ClockName));
    }
    use ClockConfiguration as C;
    match (&expected.clock, &actual.clock) {
        (
            C::EventPosition {
                sequence_authority: want,
            },
            C::EventPosition {
                sequence_authority: got,
            },
        ) if want != got => Err(refused(SelectionAxis::ClockSequenceAuthority)),
        (
            C::FixedSample {
                epoch: e1,
                period: p1,
                unit: u1,
            },
            C::FixedSample {
                epoch: e2,
                period: p2,
                unit: u2,
            },
        ) => {
            if e1 != e2 {
                Err(refused(SelectionAxis::ClockEpoch))
            } else if p1 != p2 {
                Err(refused(SelectionAxis::ClockPeriod))
            } else if u1 != u2 {
                Err(refused(SelectionAxis::ClockUnit))
            } else {
                Ok(())
            }
        }
        (
            C::TimestampedEvent {
                timestamp_unit: want,
            },
            C::TimestampedEvent {
                timestamp_unit: got,
            },
        ) if want != got => Err(refused(SelectionAxis::ClockTimestampUnit)),
        (C::Timed { unit: want }, C::Timed { unit: got }) => {
            if want == got {
                Ok(())
            } else {
                Err(refused(SelectionAxis::ClockUnit))
            }
        }
        (C::EventPosition { .. }, C::EventPosition { .. })
        | (C::TimestampedEvent { .. }, C::TimestampedEvent { .. }) => Ok(()),
        _ => Err(refused(SelectionAxis::ClockKind)),
    }
}
