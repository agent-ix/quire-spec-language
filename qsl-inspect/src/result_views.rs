// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed projections of producer results that are not package or node views.

use alloc::{string::String, vec::Vec};
use qsl_foundation::{
    diagnostic::{StageFailure, Staged},
    source::provenance::{OccurrenceKey, SourceRegion},
};
use quire_exact::Cancel;

use crate::{check_cancel, EntryBudget, InspectCause, InspectLimits, View};

/// An outcome document and source loci resolved from its producing values.
#[derive(Clone, Debug)]
pub struct OutcomeView<'a> {
    /// The typed terminal document.
    pub document: &'a qsl_replay::OutcomeDocument,
    /// The package identity associated with occurrence loci, when supplied.
    pub package_id: Option<qsl_semantics::library::PackageId>,
    /// The document's loci and their corresponding source regions.
    pub loci: Vec<(Option<qsl_replay::OutcomeLocus>, Vec<SourceRegion>)>,
}

/// Inspect an outcome while retaining loci from its typed document.
pub fn inspect_outcome<'a>(
    document: &'a qsl_replay::OutcomeDocument,
    occurrence_keys: &[Option<OccurrenceKey>],
    package_id: Option<qsl_semantics::library::PackageId>,
    emission: Option<&'a qsl_package::Emission>,
    limits: InspectLimits,
    cancel: &Cancel,
) -> Result<Staged<View<'a>>, StageFailure<InspectCause>> {
    check_cancel(cancel)?;
    if occurrence_keys.len() != document.diagnostics().len() {
        return Err(StageFailure::Refused(InspectCause::IncompatibleSelection {
            selector: String::from("outcome"),
        }));
    }
    if let (Some(package_id), Some(emission)) = (package_id, emission) {
        if package_id != emission.package().package_id() {
            return Err(StageFailure::Refused(InspectCause::PackageEmissionMismatch));
        }
    }
    let mut budget = EntryBudget::new(limits);
    let mut projected = Vec::with_capacity(occurrence_keys.len());
    for (diagnostic, occurrence_key) in document.diagnostics().iter().zip(occurrence_keys) {
        check_cancel(cancel)?;
        budget.charge(cancel)?;
        let encoded_locus = diagnostic.locus().cloned();
        let regions = match (encoded_locus.as_ref(), occurrence_key.as_ref()) {
            (Some(qsl_replay::OutcomeLocus::Region { source, start, end }), None) => {
                vec![
                    SourceRegion::new(source.clone(), *start, *end).map_err(|_| {
                        StageFailure::Refused(InspectCause::SourceCorrespondence {
                            selector: String::from("outcome region"),
                            package_id: package_id.map_or_else(String::new, |id| id.hex()),
                        })
                    })?,
                ]
            }
            (Some(qsl_replay::OutcomeLocus::Occurrence { occurrence }), Some(key)) => {
                if occurrence != &key.to_string() {
                    return Err(StageFailure::Refused(InspectCause::IncompatibleSelection {
                        selector: String::from("outcome occurrence identity mismatch"),
                    }));
                }
                let Some(emission) = emission else {
                    return Err(StageFailure::Refused(InspectCause::SourceCorrespondence {
                        selector: alloc::format!("{}", key),
                        package_id: package_id.map_or_else(String::new, |id| id.hex()),
                    }));
                };
                if let Some(package_id) = package_id {
                    if package_id != emission.package().package_id() {
                        return Err(StageFailure::Refused(InspectCause::SourceCorrespondence {
                            selector: alloc::format!("{}", key),
                            package_id: package_id.hex(),
                        }));
                    }
                }
                match emission.source_map().regions(key) {
                    Some(regions) => regions.to_vec(),
                    None => {
                        return Err(StageFailure::Refused(InspectCause::SourceCorrespondence {
                            selector: alloc::format!("{}", key),
                            package_id: package_id.map_or_else(
                                || emission.package().package_id().hex(),
                                |id| id.hex(),
                            ),
                        }))
                    }
                }
            }
            (Some(qsl_replay::OutcomeLocus::Occurrence { occurrence }), None) => {
                return Err(StageFailure::Refused(InspectCause::SourceCorrespondence {
                    selector: occurrence.clone(),
                    package_id: package_id.map_or_else(String::new, |id| id.hex()),
                }));
            }
            (Some(qsl_replay::OutcomeLocus::Artifact { .. }), None)
            | (Some(qsl_replay::OutcomeLocus::Located { .. }), None)
            | (None, None) => Vec::new(),
            _ => {
                return Err(StageFailure::Refused(InspectCause::IncompatibleSelection {
                    selector: String::from("outcome occurrence key does not match its typed locus"),
                }));
            }
        };
        for _ in &regions {
            budget.charge(cancel)?;
        }
        projected.push((encoded_locus, regions));
    }
    Ok(Staged::new(View::Outcome(OutcomeView {
        document,
        package_id,
        loci: projected,
    })))
}

/// One named assignment in a scalar counterexample.
#[derive(Clone, Debug)]
pub struct AssignmentView<'a> {
    /// The declared parameter name.
    pub parameter: &'a str,
    /// The parameter's checked wire identity.
    pub identity: qsl_foundation::digest::WireNodeId,
    /// The typed assignment value.
    pub value: &'a qsl_replay::WitnessValue,
}

/// A counterexample with names resolved through its checked function site.
#[derive(Clone, Debug)]
pub struct CounterexampleView<'a> {
    /// The function's checked qualified name.
    pub function: &'a qsl_replay::QualifiedName,
    /// Assignments in producer order, named by declared parameter.
    pub assignments: Vec<AssignmentView<'a>>,
    /// The producer's certification label, where one applies.
    pub certification: Option<qsl_replay::Certification>,
}

/// Inspect scalar assignments against a checked function signature.
pub fn inspect_counterexample<'a>(
    function: &'a qsl_replay::QualifiedName,
    site: &'a qsl_replay::FunctionSite,
    assignments: &'a [qsl_replay::CanonicalAssignment],
    certification: Option<qsl_replay::Certification>,
    limits: InspectLimits,
    cancel: &Cancel,
) -> Result<Staged<View<'a>>, StageFailure<InspectCause>> {
    check_cancel(cancel)?;
    let mut budget = EntryBudget::new(limits);
    let mut named = Vec::with_capacity(assignments.len());
    for assignment in assignments {
        budget.charge(cancel)?;
        let Some((name, _)) = site
            .parameters
            .iter()
            .find(|(_, id)| *id == assignment.parameter)
        else {
            return Err(StageFailure::Refused(InspectCause::IncompatibleSelection {
                selector: String::from("counterexample"),
            }));
        };
        named.push(AssignmentView {
            parameter: name.as_str(),
            identity: assignment.parameter,
            value: &assignment.value,
        });
    }
    Ok(Staged::new(View::Counterexample(CounterexampleView {
        function,
        assignments: named,
        certification,
    })))
}

/// A position of a temporal trace, retaining its snapshot identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePositionView<'a> {
    /// Zero-based position.
    pub position: usize,
    /// The typed snapshot identity.
    pub state: &'a qsl_semantics::model::observation::DocumentRef,
}

/// A finite trace or lasso with the producer's decision position.
#[derive(Clone, Debug)]
pub struct TraceView<'a> {
    /// The typed finite or lasso shape.
    pub shape: qsl_eval::temporal::TemporalTrace,
    /// Positions in source order, including repeated state identities.
    pub positions: Vec<TracePositionView<'a>>,
    /// The loop entry, absent for finite traces.
    pub loop_start: Option<usize>,
    /// The deciding position, when the producer recorded one.
    pub deciding_position: Option<usize>,
}

/// Inspect a supplied trace without executing or monitoring it.
pub fn inspect_trace<'a>(
    shape: qsl_eval::temporal::TemporalTrace,
    states: &'a [qsl_semantics::model::observation::DocumentRef],
    deciding_position: Option<qsl_eval::temporal::TemporalPosition>,
    limits: InspectLimits,
    cancel: &Cancel,
) -> Result<Staged<View<'a>>, StageFailure<InspectCause>> {
    check_cancel(cancel)?;
    if shape.represented() != u64::try_from(states.len()).ok()
        || deciding_position.is_some_and(|position| position.index() >= states.len() as u64)
    {
        return Err(StageFailure::Refused(InspectCause::IncompatibleSelection {
            selector: String::from("trace"),
        }));
    }
    let mut budget = EntryBudget::new(limits);
    let mut positions = Vec::with_capacity(states.len());
    for (position, state) in states.iter().enumerate() {
        budget.charge(cancel)?;
        positions.push(TracePositionView { position, state });
    }
    let loop_start = match shape {
        qsl_eval::temporal::TemporalTrace::Finite { .. } => None,
        qsl_eval::temporal::TemporalTrace::Lasso { prefix, .. } => usize::try_from(prefix).ok(),
    };
    let deciding_position =
        deciding_position.and_then(|position| usize::try_from(position.index()).ok());
    Ok(Staged::new(View::Trace(TraceView {
        shape,
        positions,
        loop_start,
        deciding_position,
    })))
}

/// A projection of retained states and transitions from a graph producer.
#[derive(Clone, Debug)]
pub struct StateGraphView<'a, S, T, F> {
    /// Retained states in producer order.
    pub states: Vec<&'a qsl_analyze::model_check::RetainedState<S, F>>,
    /// Retained edges in producer order, including duplicates.
    pub transitions: Vec<&'a qsl_analyze::model_check::RetainedEdge<T>>,
    /// The producer's exact completion status.
    pub outcome: &'a qsl_eval::simulation::Outcome,
    /// Whether the producer stopped before exhausting the graph.
    pub partial: bool,
}

/// Inspect a graph retained by an exploration; no exploration is rerun.
pub fn inspect_state_graph<'a, S, T, F>(
    exploration: Option<&'a qsl_analyze::model_check::GraphExploration<S, T, F>>,
    limits: InspectLimits,
    cancel: &Cancel,
) -> Result<Staged<View<'a, S, T, F>>, StageFailure<InspectCause>> {
    let Some(exploration) = exploration else {
        return Err(StageFailure::Refused(
            InspectCause::UnavailableRetainedContent,
        ));
    };
    check_cancel(cancel)?;
    let mut budget = EntryBudget::new(limits);
    let mut states = Vec::with_capacity(exploration.graph.states().len());
    for state in exploration.graph.states() {
        budget.charge(cancel)?;
        states.push(state);
    }
    let mut transitions = Vec::with_capacity(exploration.graph.edges().len());
    for edge in exploration.graph.edges() {
        budget.charge(cancel)?;
        transitions.push(edge);
    }
    let partial = !matches!(
        exploration.outcome,
        qsl_eval::simulation::Outcome::Exhaustive(_)
    );
    Ok(Staged::new(View::StateGraph(StateGraphView {
        states,
        transitions,
        outcome: &exploration.outcome,
        partial,
    })))
}

/// The retained graph from an EN-1 model-check run.
#[derive(Clone, Debug)]
pub struct ModelCheckStateGraphView<'a> {
    /// Retained states in the producer's order.
    pub states: Vec<&'a qsl_analyze::model_check::GraphNode>,
    /// Retained transitions in the producer's order.
    pub transitions: Vec<&'a qsl_analyze::model_check::GraphEdge>,
    /// Open nodes and their reasons, preserving the producer's partial frontier.
    pub open: &'a alloc::collections::BTreeMap<
        qsl_analyze::model_check::NodeIndex,
        qsl_replay::OpenCause,
    >,
    /// The producer's original completion status.
    pub end: &'a qsl_replay::RunEnd,
    /// Whether the producer left any graph frontier open.
    pub partial: bool,
}

/// Inspect a model-check graph retained by EN-1 without rerunning the checker.
pub fn inspect_model_check_graph<'a>(
    run: Option<&'a qsl_analyze::model_check::StateGraphRun>,
    limits: InspectLimits,
    cancel: &Cancel,
) -> Result<Staged<View<'a>>, StageFailure<InspectCause>> {
    let Some(run) = run else {
        return Err(StageFailure::Refused(
            InspectCause::UnavailableRetainedContent,
        ));
    };
    check_cancel(cancel)?;
    let mut budget = EntryBudget::new(limits);
    let mut states = Vec::with_capacity(run.graph.nodes.len());
    for state in &run.graph.nodes {
        budget.charge(cancel)?;
        states.push(state);
    }
    let mut transitions = Vec::with_capacity(run.graph.edges.len());
    for edge in &run.graph.edges {
        budget.charge(cancel)?;
        transitions.push(edge);
    }
    let partial = !run.graph.open.is_empty();
    Ok(Staged::new(View::ModelCheckStateGraph(
        ModelCheckStateGraphView {
            states,
            transitions,
            open: &run.graph.open,
            end: &run.graph.end,
            partial,
        },
    )))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::{
        diagnostic::StageFailure,
        digest::WireNodeId,
        source::provenance::{RawSourceRef, SourceRegion},
    };
    use qsl_semantics::model::observation::DocumentRef;
    use quire_exact::{Identifier, Origin, Role};

    use super::*;
    use crate::InspectLimits;

    fn snapshot(digest: u8) -> DocumentRef {
        DocumentRef {
            authority: String::from("tests"),
            identity: String::from("snapshot"),
            revision_namespace: String::from("test"),
            revision: digest.to_string(),
        }
    }

    #[trace("TC-782", "FR-297-AC-7")]
    #[test]
    fn finite_and_lasso_trace_projection_preserve_order_and_repeated_states() {
        let repeated = snapshot(1);
        let states = [repeated.clone(), repeated];
        let result = inspect_trace(
            qsl_eval::temporal::TemporalTrace::Lasso {
                prefix: 1,
                loop_len: 1,
            },
            &states,
            Some(qsl_eval::temporal::TemporalPosition::new(1)),
            InspectLimits {
                max_view_entries: 2,
            },
            &Cancel::new(),
        )
        .expect("the exact position bound succeeds");
        let View::Trace(view) = result.value() else {
            panic!("trace request gives trace view");
        };
        assert_eq!(view.positions[0].state, view.positions[1].state);
        assert_eq!(view.loop_start, Some(1));
        assert_eq!(view.deciding_position, Some(1));
    }

    #[trace("TC-1168", "FR-297-AC-10")]
    #[test]
    fn trace_limit_reports_bound_plus_one_without_returning_a_view() {
        let states = [snapshot(1), snapshot(2)];
        let result = inspect_trace(
            qsl_eval::temporal::TemporalTrace::Finite { len: 2 },
            &states,
            None,
            InspectLimits {
                max_view_entries: 1,
            },
            &Cancel::new(),
        );
        assert!(matches!(
            result,
            Err(StageFailure::Limit(limit))
                if limit.setting() == qsl_foundation::setting::Setting::InspectMaxViewEntries
                    && limit.configured_bound() == 1
                    && limit.actual() == 2
        ));
    }

    #[trace("TC-782", "FR-297-AC-3")]
    #[test]
    fn counterexample_assignments_join_to_parameter_names_by_identity() {
        let function = qsl_replay::QualifiedName::new(vec![
            Identifier::new("math").unwrap(),
            Identifier::new("seven").unwrap(),
        ])
        .unwrap();
        let parameter = WireNodeId::from_digest([9; 32]);
        let site = qsl_replay::FunctionSite {
            function: WireNodeId::from_digest([8; 32]),
            declaration: qsl_foundation::source::provenance::OccurrenceKey::new(
                WireNodeId::from_digest([8; 32]),
                Origin::new(Role::new("declaration"), 0),
            ),
            parameters: vec![(Identifier::new("x").unwrap(), parameter)],
            value_parity: None,
        };
        let assignments = [qsl_replay::CanonicalAssignment {
            parameter,
            value: qsl_replay::WitnessValue::Boolean(true),
        }];
        let result = inspect_counterexample(
            &function,
            &site,
            &assignments,
            Some(qsl_replay::Certification::Trusted),
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect("the typed function site resolves the assignment");
        let View::Counterexample(view) = result.value() else {
            panic!("counterexample request gives counterexample view");
        };
        assert_eq!(view.function, &function);
        assert_eq!(view.assignments[0].parameter, "x");
        assert_eq!(view.assignments[0].identity, parameter);
        assert_eq!(view.certification, Some(qsl_replay::Certification::Trusted));
    }

    #[trace("TC-782", "FR-297-AC-6")]
    #[test]
    fn outcome_region_locus_is_retained_without_source_bytes() {
        let source = RawSourceRef::new("tests", "source").unwrap();
        let region = SourceRegion::new(source.clone(), 4, 11).unwrap();
        let diagnostic = qsl_replay::OutcomeDiagnostic::new(
            None,
            "invalid_runtime_input",
            Some(qsl_replay::OutcomeLocus::Region {
                source,
                start: 4,
                end: 11,
            }),
            "fixture",
        );
        let document = qsl_replay::OutcomeDocument::new(
            qsl_replay::Operation::Monitor,
            None,
            quire_semantic_value::catalog::Category::Success,
        )
        .with_diagnostics(vec![diagnostic]);
        let result = inspect_outcome(
            &document,
            &[None],
            None,
            None,
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect("a region locus needs no package source map");
        let View::Outcome(view) = result.value() else {
            panic!("outcome request gives outcome view");
        };
        assert_eq!(view.loci[0].0, document.diagnostics()[0].locus().cloned());
        assert_eq!(view.loci[0].1, vec![region]);
    }
}
