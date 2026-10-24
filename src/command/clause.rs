// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-312: turn one closed native-run clause selection into a spine request.

use std::collections::BTreeMap;

use qsl_replay::spine::{
    ClauseRequestParts, ClauseRunRequest, ClauseRunSource, DependencyInput, SpineLimits,
    SuppliedLibrary, SuppliedObservation,
};

use super::{output, wire, FormalSource, Intake, Result, RunCause, RunResult};

fn invalid(member: impl Into<String>, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.into(),
        reason: reason.into(),
    }
}

fn observation(
    intake: &mut Intake<'_>,
    key: &str,
    file: &str,
    document_bytes: u64,
) -> Result<SuppliedObservation> {
    let bytes =
        intake.observation_file(file, usize::try_from(document_bytes).unwrap_or(usize::MAX))?;
    SuppliedObservation::from_document(key, bytes)
        .map_err(|error| invalid(error.member, error.reason))
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
    let observation_limits =
        qsl_replay::spine::ObservationLimitsWire::resolve(clause.observation_limits.as_ref())
            .map_err(|error| invalid(error.member, error.reason))?;
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
        selection: clause
            .selection
            .into_parts()
            .map_err(|error| invalid(error.member, error.reason))?,
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
