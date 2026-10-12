// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-312: turn one closed native-run clause selection into a spine request.

use std::collections::BTreeMap;

use qsl_replay::spine::{
    ClauseRequestParts, ClauseRunRequest, ClauseRunSource, DependencyInput, SpineLimits,
    SuppliedLibrary, SuppliedObservation,
};
use qsl_semantics::model::observation::{
    AnchorKind, DocumentRef, ObservationLimits, PostStateRange, SelectedAnchor, SelectedObject,
};
use quire_canonical::{Node, NodeRef};

use super::{output, wire, FormalSource, Intake, Result, RunCause, RunResult};

fn invalid(member: impl Into<String>, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.into(),
        reason: reason.into(),
    }
}

#[qsl_attrs::string_edge]
fn observation(
    intake: &mut Intake<'_>,
    key: &str,
    file: &str,
    document_bytes: u64,
) -> Result<SuppliedObservation> {
    let bytes =
        intake.observation_file(file, usize::try_from(document_bytes).unwrap_or(usize::MAX))?;
    let parsed =
        quire_canonical::read(&bytes, u64::MAX).map_err(|error| invalid(key, error.to_string()))?;
    let digest = *quire_canonical::sha256(&parsed, quire_canonical::Limits::new(u64::MAX))
        .map_err(|error| invalid(key, error.to_string()))?
        .as_bytes();
    // Reuse the canonical reader's stack-backed tree. Observation values may
    // exceed serde_json's default 128-level recursion ceiling (FR-261).
    let root = parsed.root();
    let text = |path: &str, value: Option<NodeRef<'_>>| match value.map(|node| node.node()) {
        Some(Node::String(value)) => Ok(value.to_owned()),
        _ => Err(invalid(
            format!("observations.{key}.{path}"),
            "expected a string",
        )),
    };
    let format = text("format", root.get("format"))?;
    if format != "quire.state.snapshot/v1" && format != "quire.state.invocation/v1" {
        return Err(invalid(format!("observations.{key}.format"), format));
    }
    let identity = root.get("identity");
    let reference = DocumentRef {
        authority: text(
            "identity.authority",
            identity.and_then(|id| id.get("authority")),
        )?,
        identity: text(
            "identity.identity",
            identity.and_then(|id| id.get("identity")),
        )?,
        revision_namespace: text(
            "identity.revision_namespace",
            identity.and_then(|id| id.get("revision_namespace")),
        )?,
        revision: text(
            "identity.revision",
            identity.and_then(|id| id.get("revision")),
        )?,
        digest,
    };
    let role = root.get("observation").and_then(|node| match node.node() {
        Node::String(value) => Some(value.to_owned()),
        _ => None,
    });
    Ok(SuppliedObservation {
        reference,
        format,
        role,
        bytes,
    })
}

fn selected_object(value: &wire::ObjectReference) -> SelectedObject {
    SelectedObject {
        population: value.population.clone(),
        key: value.key.clone(),
    }
}

fn selection_parts(
    wire: &wire::ClauseRunSelection,
) -> Result<qsl_replay::spine::ClauseSelectionPartsKind> {
    use qsl_replay::spine::{ClauseSelectionParts, ClauseSelectionPartsKind};
    let count = usize::from(wire.clause.is_some())
        + usize::from(wire.frame.is_some())
        + usize::from(wire.function.is_some());
    if count != 1 {
        return Err(invalid(
            "selection",
            "select exactly one clause, frame or function",
        ));
    }
    if let Some(clause) = &wire.clause {
        let anchor = clause.anchor.as_ref().map(|anchor| {
            let (kind, name) = match anchor {
                wire::Anchor::Initialization { name } => (AnchorKind::Initialization, name),
                wire::Anchor::Handler { name } => (AnchorKind::Handler, name),
            };
            SelectedAnchor {
                kind,
                name: name.clone(),
            }
        });
        return Ok(ClauseSelectionPartsKind::Clause(ClauseSelectionParts {
            name: clause.name.clone(),
            observation: clause.observation.clone(),
            self_object: clause.self_object.as_ref().map(selected_object),
            anchor,
            parameters: clause.parameters.clone(),
        }));
    }
    if let Some(frame) = &wire.frame {
        return Ok(ClauseSelectionPartsKind::Frame {
            operation: frame.operation.clone(),
            invocation: frame.invocation.clone(),
        });
    }
    let function = wire.function.as_ref().expect("exactly one variant");
    Ok(ClauseSelectionPartsKind::Function {
        name: function.name.clone(),
        arguments: function
            .arguments
            .iter()
            .map(|arg| (arg.parameter.clone(), arg.value.clone()))
            .collect(),
        snapshot: function.snapshot.clone(),
    })
}

#[qsl_attrs::string_edge]
fn limits(overrides: Option<&wire::ObservationLimitOverrides>) -> Result<ObservationLimits> {
    let mut limits = ObservationLimits::default();
    if let Some(overrides) = overrides {
        if let Some(value) = overrides.document_bytes {
            limits.document_bytes = value;
        }
        if let Some(value) = overrides.objects_per_document {
            limits.objects_per_document = value;
        }
        if let Some(value) = overrides.values_per_document {
            limits.values_per_document = value;
        }
        if let Some(value) = &overrides.post_state {
            limits.post_state = match value.as_str() {
                "refuse" => PostStateRange::Refuse,
                "witness" => PostStateRange::Witness,
                _ => return Err(invalid("observation_limits.post_state", "unknown policy")),
            };
        }
    }
    Ok(limits)
}

pub(super) fn read(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    clause: &wire::ClauseRun,
    source: FormalSource,
) -> Result<ClauseRunRequest> {
    if request.selection.is_some()
        || request.snapshots.is_some()
        || request.invocations.is_some()
        || request.package.is_some()
        || request.program.clauses.is_some()
    {
        return Err(invalid("clause", "legacy run members are forbidden"));
    }
    if request
        .models
        .iter()
        .any(|model| !super::is_domain_package(model))
    {
        return Err(invalid("models", "expected semantic-ir/2.0.0"));
    }
    let documents = request
        .models
        .iter()
        .map(|model| intake.source(&model.source))
        .collect::<Result<Vec<_>>>()?;
    let packages = qsl_semantics::model::intake::package_input(
        documents
            .iter()
            .map(|document| document.source().text().as_bytes()),
    );
    let libraries = request
        .libraries
        .iter()
        .map(|library| Ok((library, intake.source(&library.source)?)))
        .collect::<Result<Vec<_>>>()?;
    let dependencies =
        DependencyInput::new(libraries.iter().map(|(library, read)| SuppliedLibrary {
            identity: library.identity.clone(),
            source: read.source().identity().clone(),
            path: read.source().path().to_owned(),
            bytes: read.source().text().as_bytes().to_vec(),
        }))
        .map_err(|error| invalid("libraries", error.to_string()))?;
    let observation_limits = limits(clause.observation_limits.as_ref())?;
    let mut by_file = BTreeMap::<String, SuppliedObservation>::new();
    let mut observations = BTreeMap::new();
    for (key, file) in &clause.observations {
        let observed = if let Some(existing) = by_file.get(file) {
            existing.clone()
        } else {
            let observed = observation(intake, key, file, observation_limits.document_bytes)?;
            by_file.insert(file.clone(), observed.clone());
            observed
        };
        observations.insert(key.clone(), observed);
    }
    let source = ClauseRunSource::Program {
        identity: source.source().identity().clone(),
        path: source.source().path().to_owned(),
        bytes: source.source().text().as_bytes().to_vec(),
    };
    qsl_replay::spine::read_clause_run_request(ClauseRequestParts {
        source,
        packages,
        dependencies,
        observations,
        selection: selection_parts(&clause.selection)?,
        expected_package_id_claim: clause.package_id.clone(),
        limits: SpineLimits::default(),
        observation_limits,
        accounting: super::call_accounting(&clause.accounting),
    })
    .map_err(|error| invalid(error.member, error.reason))
}

pub(super) fn run(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    clause: &wire::ClauseRun,
    source: FormalSource,
) -> Result<RunResult> {
    let request = read(intake, request, clause, source)?;
    let report = qsl_replay::spine::run_clause(request)
        .map_err(|error| invalid("clause", error.to_string()))?;
    output::clause_report(&report)
}
