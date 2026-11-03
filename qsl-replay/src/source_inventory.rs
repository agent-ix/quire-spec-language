// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-362 item 3 and FR-514 "The inventory on exhaustion": the owner-issued
//! source inventory of QSL's checked requirements and pending obligations
//! (IR FR-052), built from the outcome of the spine's `check` and from
//! nothing else.
//!
//! A checked unit gives a `Complete` inventory of its own
//! [`TypeReport`](crate::spine::TypeReport): one `RequirementRecord` row per
//! requirement record, then one `PendingObligation` row per pending
//! obligation. A stage limit gives an `Incomplete` inventory with one
//! requirement row per S2 requirement form and the limit as the typed cause.
//! Any other refusal gives no inventory.

use qsl_foundation::diagnostic::{LimitExceeded, LimitKind, Locus, StageFailure, Staged};
use qsl_semantics::check::wire_id::structural_id;
use quire_contract_model::source_obligation::{
    SourceDeclarationHandle, SourceInventoryEnumerator, SourceInventoryIdentity,
    SourceInventoryIncompleteReason, SourceInventoryOutcome, SourceInventoryOutcomeError,
    SourceInventoryOwner, SourceInventoryRow, SourceInventoryScope, SourceObligationSite,
    SourcePendingObligationId, SourcePendingObligationKind, SourceRequirementOccurrenceId,
    SourceTypeExhaustion, SourceTypeWorkDimension, SourceUnitHandle,
};
use quire_contract_model::{CheckedNodeId, CheckedNodePathSegment, SourceIdentity};

use crate::spine::{CheckedUnit, FrontEndFailure, ParsedUnitSource, PendingObligation};

/// Why a check outcome has no source inventory.
#[derive(Debug, thiserror::Error)]
pub enum InventoryRefusal {
    /// The check outcome is neither a checked unit nor a stage limit: the
    /// unit was refused, cancelled or faulted, so there are no rows.
    #[error("the check outcome is neither a checked unit nor a stage limit")]
    NoInventory,
    /// The enumeration was inconsistent, which IR's outcome constructor
    /// reports.
    #[error("the source inventory enumeration was inconsistent: {0}")]
    Outcome(#[from] SourceInventoryOutcomeError),
}

/// QSL's own traversal: the rows in order, then the stop cause when the
/// traversal did not reach its end.
struct Traversal<'u> {
    identity: &'u SourceIdentity,
    rows: std::vec::IntoIter<SourceInventoryIdentity>,
    stop: Option<SourceInventoryIncompleteReason>,
    observed: usize,
}

impl SourceInventoryEnumerator for Traversal<'_> {
    fn source(&self) -> &SourceIdentity {
        self.identity
    }

    fn owner(&self) -> SourceInventoryOwner {
        SourceInventoryOwner::Qsl
    }

    fn scope(&self) -> SourceInventoryScope {
        SourceInventoryScope::QslCheckedRequirementsAndPendingObligations
    }

    fn next_row(&mut self) -> Result<Option<SourceInventoryRow>, SourceInventoryIncompleteReason> {
        if let Some(identity) = self.rows.next() {
            self.observed += 1;
            return Ok(Some(SourceInventoryRow::new(
                self.identity.clone(),
                identity,
            )));
        }
        match self.stop.take() {
            Some(reason) => Err(reason),
            None => Ok(None),
        }
    }

    fn observed_count(&self) -> usize {
        self.observed
    }
}

/// The IR dimension a QSL stage limit kind charges, or `None` for a kind
/// that is not a type-admission stage limit. `TickCount` is a runtime
/// temporal-evidence limit (FR-523) reported as `Incomplete`; FR-514 maps no
/// type-work dimension to it, so it never becomes a `TypeWorkExhausted` cause.
fn dimension(kind: LimitKind) -> Option<SourceTypeWorkDimension> {
    match kind {
        LimitKind::InputBytes => Some(SourceTypeWorkDimension::Bytes),
        LimitKind::TokenCount | LimitKind::NodeCount => Some(SourceTypeWorkDimension::Expressions),
        LimitKind::EdgeCount => Some(SourceTypeWorkDimension::Edges),
        LimitKind::OccurrenceCount | LimitKind::DiagnosticCount => {
            Some(SourceTypeWorkDimension::Records)
        }
        LimitKind::WorkBudget => Some(SourceTypeWorkDimension::Constraints),
        LimitKind::RationalBits => Some(SourceTypeWorkDimension::Normalization),
        LimitKind::TickCount => None,
    }
}

/// The cause IR's `TypeWorkExhausted` retains for a reached stage limit: the
/// kind as a dimension, the configured bound as the limit, the bound as the
/// charged usage and the excess past it as the unpaid increment, at the
/// limit's source region when it has one. `None` when the limit's kind is
/// not a type-admission stage limit (see [`dimension`]).
fn exhaustion(limit: &LimitExceeded) -> Option<SourceInventoryIncompleteReason> {
    let dimension = dimension(limit.kind())?;
    let bound = usize::try_from(limit.configured_bound()).unwrap_or(usize::MAX);
    let reached = usize::try_from(limit.actual()).unwrap_or(usize::MAX);
    let (start, end) = match limit.locus() {
        Some(Locus::Region(region)) => (
            usize::try_from(region.start()).unwrap_or(usize::MAX),
            usize::try_from(region.end()).unwrap_or(usize::MAX),
        ),
        _ => (0, 0),
    };
    Some(SourceInventoryIncompleteReason::TypeWorkExhausted(
        SourceTypeExhaustion {
            dimension,
            limit: bound,
            prior: bound,
            requested: reached.saturating_sub(bound),
            site: SourceObligationSite {
                declaration: SourceDeclarationHandle(0),
                unit: SourceUnitHandle(0),
                expression: None,
                start,
                end,
            },
        },
    ))
}

fn requirement(key: &qsl_foundation::source::provenance::OccurrenceKey) -> SourceInventoryIdentity {
    SourceInventoryIdentity::RequirementRecord(SourceRequirementOccurrenceId {
        node: structural_id(key.node()),
        role: key.origin().role().as_str().into(),
        ordinal: key.origin().ordinal(),
    })
}

/// The identity of one pending obligation: its report position as the
/// declaration handle and its source span as the site. IR's kind vocabulary
/// has no member for QSL's capability kinds, so every obligation carries the
/// one fixed kind and the site alone tells obligations apart.
fn obligation(position: usize, pending: &PendingObligation) -> SourceInventoryIdentity {
    let span = pending.span();
    SourceInventoryIdentity::PendingObligation(SourcePendingObligationId {
        site: SourceObligationSite {
            declaration: SourceDeclarationHandle(position),
            unit: SourceUnitHandle(0),
            expression: None,
            start: span.start,
            end: span.end,
        },
        kind: SourcePendingObligationKind::GraphClosure,
    })
}

/// The identity of the `index`th S2 requirement form: a requirement row
/// before any checked node exists, in the role `form`.
fn form_requirement(index: usize, name: Option<&str>) -> SourceInventoryIdentity {
    SourceInventoryIdentity::RequirementRecord(SourceRequirementOccurrenceId {
        node: CheckedNodeId {
            package: "".into(),
            path: vec![CheckedNodePathSegment::Name(name.unwrap_or("").into())],
            ordinal: u64::try_from(index).unwrap_or(u64::MAX),
        },
        role: "form".into(),
        ordinal: u64::try_from(index).unwrap_or(u64::MAX),
    })
}

/// The S2 forms that carry a requirement.
fn is_requirement_form(form: &qsl_forms::DeclarationForm) -> bool {
    use qsl_forms::DeclarationForm as Form;
    matches!(
        form,
        Form::Function(_)
            | Form::StateClause(_)
            | Form::OperationBody(_)
            | Form::Protocol(_)
            | Form::Temporal(_)
            | Form::Terminal(_)
            | Form::Refinement(_)
            | Form::HyperBehaviours(_)
            | Form::HyperFinite(_)
            | Form::ModelRelation(_)
            | Form::Probabilistic(_)
            | Form::StateGraphClaim(_)
            | Form::Schedulable(_)
            | Form::Response(_)
            | Form::TimeInvariant(_)
    )
}

/// Report QSL's inventory of one `check` outcome under the selected formal
/// identity (FR-514 "The inventory on exhaustion"):
///
/// - a checked unit is `Complete`: one row per requirement record and one per
///   pending obligation of the unit's own type report;
/// - a stage limit is `Incomplete`: one requirement row per requirement form
///   of `parsed` (none when `parsed` is `None` because the limit stopped S1 or
///   S2), no obligation row, and the limit as the typed cause;
/// - a limit that is not a type-admission stage limit (`TickCount`), and any
///   other refusal, has no inventory.
///
/// # Errors
///
/// [`InventoryRefusal::NoInventory`] for an outcome that is neither a checked
/// unit nor a stage limit, and [`InventoryRefusal::Outcome`] when IR's
/// outcome constructor rejects the enumeration.
pub fn qsl_inventory(
    parsed: Option<&ParsedUnitSource>,
    outcome: &Result<Staged<CheckedUnit>, FrontEndFailure>,
    formal_identity: &SourceIdentity,
) -> Result<SourceInventoryOutcome, InventoryRefusal> {
    let (rows, stop): (Vec<SourceInventoryIdentity>, _) = match outcome {
        Ok(staged) => {
            let report = staged.value().type_report();
            let rows = report
                .requirements()
                .iter()
                .map(|required| requirement(required.occurrence()))
                .chain(
                    report
                        .obligations()
                        .iter()
                        .enumerate()
                        .map(|(position, pending)| obligation(position, pending)),
                )
                .collect();
            (rows, None)
        }
        Err(StageFailure::Limit(limit) | StageFailure::LimitWithPayload(limit, _)) => {
            let stop = exhaustion(limit).ok_or(InventoryRefusal::NoInventory)?;
            let rows = parsed
                .into_iter()
                .flat_map(|parsed| parsed.unit().forms())
                .filter(|form| is_requirement_form(form.form()))
                .enumerate()
                .map(|(index, form)| form_requirement(index, form.declared_name()))
                .collect();
            (rows, Some(stop))
        }
        Err(_) => return Err(InventoryRefusal::NoInventory),
    };
    let traversal = Traversal {
        identity: formal_identity,
        rows: rows.into_iter(),
        stop,
        observed: 0,
    };
    Ok(SourceInventoryOutcome::from_enumerator(traversal)?)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::{LimitKind, StageFailure};
    use qsl_foundation::{Setting, SourceIdentity as NativeIdentity};
    use quire_contract_model::source_obligation::SourceInventoryCompleteness;
    use quire_contract_model::{SourceDocumentId, SourceRevision};
    use quire_exact::Cancel;

    use super::*;
    use crate::spine::{
        admit_parsed_unit_list, check, check_units, parse, select, AdmittedModels, DependencyInput,
        ParseRequest, SpineLimits,
    };

    const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n";

    /// Two claim sites: one guarded application per branch of one node.
    const GUARDED: &str = "function f using v(x: Int[0, 9], p: Boolean): Integer pure \
        { if p then x + 1 else x + 1 }";

    /// The same shape under another name, for a second source unit.
    const GUARDED_G: &str = "function g using v(x: Int[0, 9], p: Boolean): Integer pure \
        { if p then x + 2 else x + 2 }";

    fn formal(document: &str) -> SourceIdentity {
        SourceIdentity::new(
            SourceDocumentId::new(document).expect("a valid document id"),
            SourceRevision::new(1).expect("a valid revision"),
        )
    }

    fn native(identity: &str) -> NativeIdentity {
        NativeIdentity::new("test", identity)
    }

    fn parse_text(identity: &NativeIdentity, text: &str, limits: SpineLimits) -> ParsedUnitSource {
        parse(
            &ParseRequest {
                source: identity,
                path: "unit.native",
                bytes: text.as_bytes(),
            },
            limits.source,
            &Cancel::new(),
        )
        .expect("S1 and S2 read the unit")
        .into_value()
    }

    fn run_check(
        parsed: &ParsedUnitSource,
        limits: SpineLimits,
    ) -> Result<Staged<CheckedUnit>, FrontEndFailure> {
        let cancel = Cancel::new();
        let models: AdmittedModels = select(
            parsed,
            &qsl_semantics::model::intake::PackageInput::default(),
            limits.model,
            &cancel,
        )
        .expect("I1 admits no model")
        .into_value();
        check(
            parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &cancel,
        )
    }

    /// Limits whose S3 work budget is one below what `parsed` measures to need.
    fn one_below_need(parsed: &ParsedUnitSource) -> SpineLimits {
        let need = run_check(parsed, SpineLimits::default())
            .expect("the unit checks with default limits")
            .work()
            .s3;
        SpineLimits {
            checking: quire_semantic_value::checking::CheckingLimits::default()
                .with_work_budget(need - 1),
            ..SpineLimits::default()
        }
    }

    fn identities(outcome: &SourceInventoryOutcome) -> Vec<SourceInventoryIdentity> {
        outcome
            .items()
            .iter()
            .map(|row| row.identity().clone())
            .collect()
    }

    fn is_obligation(identity: &SourceInventoryIdentity) -> bool {
        matches!(identity, SourceInventoryIdentity::PendingObligation(_))
    }

    /// What `unit` alone must report: its report's requirements, then its
    /// report's obligations.
    fn expected_rows(unit: &CheckedUnit) -> Vec<SourceInventoryIdentity> {
        let report = unit.type_report();
        report
            .requirements()
            .iter()
            .map(|required| requirement(required.occurrence()))
            .chain(
                report
                    .obligations()
                    .iter()
                    .enumerate()
                    .map(|(position, pending)| obligation(position, pending)),
            )
            .collect()
    }

    /// FR-362-AC-4, FR-514-AC-3 (TC-1026, TC-1309 row 6): a checked unit's
    /// inventory is `Complete` and holds one row per requirement record then
    /// one per pending obligation of its own type report.
    #[trace("FR-362-AC-4", "FR-514-AC-3", "TC-1026", "TC-1309")]
    #[test]
    fn a_checked_unit_yields_one_row_per_requirement_and_per_obligation() {
        let parsed = parse_text(
            &native("unit:main"),
            &format!("{HEADER}{GUARDED}\n"),
            SpineLimits::default(),
        );
        let checked = run_check(&parsed, SpineLimits::default());
        let unit = checked.as_ref().expect("the fixture checks").value();
        let report = unit.type_report();
        assert_eq!(report.requirements().len(), 2, "both guarded occurrences");
        assert_eq!(report.obligations().len(), 2, "one obligation per claim");
        let outcome =
            qsl_inventory(Some(&parsed), &checked, &formal("NativeTypes0")).expect("an inventory");
        assert_eq!(outcome.owner(), SourceInventoryOwner::Qsl);
        assert_eq!(
            outcome.scope(),
            SourceInventoryScope::QslCheckedRequirementsAndPendingObligations
        );
        assert_eq!(outcome.source(), &formal("NativeTypes0"));
        assert_eq!(
            outcome.completeness(),
            &SourceInventoryCompleteness::Complete
        );
        assert_eq!(outcome.observed_count(), 4);
        let rows = identities(&outcome);
        assert_eq!(rows, expected_rows(unit));
        assert!(rows[..2]
            .iter()
            .all(|row| matches!(row, SourceInventoryIdentity::RequirementRecord(_))));
        assert!(rows[2..].iter().all(is_obligation));
        let distinct: std::collections::BTreeSet<_> =
            rows.iter().map(|row| format!("{row:?}")).collect();
        assert_eq!(distinct.len(), 4, "every row is a distinct identity");
    }

    /// FR-362-AC-4 (TC-1026): a unit that declares no claim site reports
    /// exactly the rows of its report, which are none, and is `Complete`.
    #[trace("FR-362-AC-4", "TC-1026")]
    #[test]
    fn a_unit_with_no_requirement_records_is_complete_with_no_rows() {
        let parsed = parse_text(
            &native("unit:main"),
            &format!("{HEADER}function g using v(): Integer pure {{ 1 }}\n"),
            SpineLimits::default(),
        );
        let checked = run_check(&parsed, SpineLimits::default());
        let unit = checked.as_ref().expect("the fixture checks").value();
        assert!(unit.type_report().requirements().is_empty());
        assert!(unit.type_report().obligations().is_empty());
        let outcome =
            qsl_inventory(Some(&parsed), &checked, &formal("NativeTypes0")).expect("an inventory");
        assert!(outcome.items().is_empty());
        assert_eq!(
            outcome.completeness(),
            &SourceInventoryCompleteness::Complete
        );
    }

    /// FR-362-AC-8, first sentence (TC-1026, TC-1309 row 5): a unit the type
    /// phase refuses is an error outcome with no inventory rows.
    #[trace("FR-362-AC-8", "FR-514-AC-3", "TC-1026", "TC-1309")]
    #[test]
    fn a_unit_the_type_phase_refuses_yields_an_error_and_no_rows() {
        let parsed = parse_text(
            &native("unit:main"),
            &format!("{HEADER}function h using v(x: Int[0, 9]): Integer pure {{ x + true }}\n"),
            SpineLimits::default(),
        );
        let refused = run_check(&parsed, SpineLimits::default());
        assert!(matches!(refused, Err(StageFailure::Refused(_))));
        assert!(matches!(
            qsl_inventory(Some(&parsed), &refused, &formal("NativeTypes0")),
            Err(InventoryRefusal::NoInventory)
        ));
    }

    /// FR-362-AC-8, second sentence, and FR-514-AC-3 (TC-1026, TC-1309 row
    /// 3): a check that stops on the S3 work budget gives an `Incomplete`
    /// inventory with one requirement row per requirement form, no obligation
    /// row and the budget as the typed cause.
    #[trace("FR-362-AC-8", "FR-514-AC-3", "TC-1026", "TC-1309")]
    #[test]
    fn a_stage_limit_yields_an_incomplete_inventory_with_the_partial_rows() {
        let parsed = parse_text(
            &native("unit:main"),
            &format!("{HEADER}{GUARDED}\n{GUARDED_G}\nrecord Point {{ x: Integer; }}\n"),
            SpineLimits::default(),
        );
        let stopped = run_check(&parsed, one_below_need(&parsed));
        let Err(StageFailure::Limit(limit)) = &stopped else {
            panic!("the work budget stops the check");
        };
        assert_eq!(limit.setting(), Setting::S3WorkUnits);
        assert_eq!(limit.kind(), LimitKind::WorkBudget);
        let outcome =
            qsl_inventory(Some(&parsed), &stopped, &formal("NativeTypes0")).expect("an inventory");
        let rows = identities(&outcome);
        assert_eq!(rows.len(), 2, "the two functions are requirement forms");
        assert!(rows
            .iter()
            .all(|row| matches!(row, SourceInventoryIdentity::RequirementRecord(_))));
        assert!(!rows.iter().any(is_obligation));
        assert_eq!(outcome.observed_count(), 2);
        let SourceInventoryCompleteness::Incomplete(
            SourceInventoryIncompleteReason::TypeWorkExhausted(cause),
        ) = outcome.completeness()
        else {
            panic!("a stage limit is the type-work exhaustion cause");
        };
        assert_eq!(cause.dimension, SourceTypeWorkDimension::Constraints);
        assert_eq!(
            cause.limit,
            usize::try_from(limit.configured_bound()).expect("a small bound")
        );
    }

    /// FR-514-AC-3 (TC-1309 row 3), FR-523: a tick-count limit is a runtime
    /// temporal-evidence limit with no type-work dimension, so it is not a
    /// type-admission exhaustion and gives no inventory.
    #[trace("FR-514-AC-3", "TC-1309")]
    #[test]
    fn a_runtime_tick_limit_yields_no_inventory_rather_than_a_type_work_cause() {
        let limit = qsl_foundation::diagnostic::LimitExceeded::new(
            Setting::TemporalEvidenceHistorySpan,
            4,
            5,
        );
        assert_eq!(limit.kind(), LimitKind::TickCount);
        assert!(exhaustion(&limit).is_none());
        let outcome: Result<Staged<CheckedUnit>, FrontEndFailure> = Err(StageFailure::Limit(limit));
        assert!(matches!(
            qsl_inventory(None, &outcome, &formal("NativeTypes0")),
            Err(InventoryRefusal::NoInventory)
        ));
    }

    /// FR-514-AC-3 (TC-1309 row 4): a limit that stops S1 leaves no parsed
    /// unit, so the inventory is `Incomplete` with zero rows and the node
    /// count as the cause.
    #[trace("FR-514-AC-3", "TC-1309")]
    #[test]
    fn a_limit_that_stops_s1_yields_an_incomplete_inventory_with_no_rows() {
        let mut limits = SpineLimits::default();
        limits.source.nodes = 1;
        let identity = native("unit:main");
        let text = format!("{HEADER}{GUARDED}\n");
        let failure = parse(
            &ParseRequest {
                source: &identity,
                path: "unit.native",
                bytes: text.as_bytes(),
            },
            limits.source,
            &Cancel::new(),
        )
        .expect_err("one node stops S1");
        let StageFailure::Limit(limit) = &failure else {
            panic!("the node ceiling is a stage limit");
        };
        assert_eq!(limit.setting(), Setting::S1Nodes);
        let outcome = qsl_inventory(
            None,
            &Err::<Staged<CheckedUnit>, _>(failure),
            &formal("NativeTypes0"),
        )
        .expect("an inventory");
        assert!(outcome.items().is_empty());
        let SourceInventoryCompleteness::Incomplete(
            SourceInventoryIncompleteReason::TypeWorkExhausted(cause),
        ) = outcome.completeness()
        else {
            panic!("a stage limit is the type-work exhaustion cause");
        };
        assert_eq!(cause.dimension, SourceTypeWorkDimension::Expressions);
        assert_eq!(cause.limit, 1);
    }

    /// FR-514-AC-4 (TC-1309 row 7): the formal identity is the inventory's
    /// own parameter, so two identities give two inventories of one source
    /// and an `Incomplete` and a `Complete` outcome under one identity share
    /// it.
    #[trace("FR-514-AC-4", "TC-1309")]
    #[test]
    fn the_formal_identity_is_a_parameter_of_the_inventory() {
        let parsed = parse_text(
            &native("unit:main"),
            &format!("{HEADER}{GUARDED}\n"),
            SpineLimits::default(),
        );
        let checked = run_check(&parsed, SpineLimits::default());
        let first = qsl_inventory(Some(&parsed), &checked, &formal("F1")).expect("an inventory");
        let second = qsl_inventory(Some(&parsed), &checked, &formal("F2")).expect("an inventory");
        assert_ne!(first.source(), second.source());
        assert_eq!(identities(&first), identities(&second));
        let stopped = run_check(&parsed, one_below_need(&parsed));
        let partial = qsl_inventory(Some(&parsed), &stopped, &formal("F1")).expect("an inventory");
        assert_eq!(partial.source(), first.source());
        assert_ne!(partial.completeness(), first.completeness());
    }

    /// FR-362-AC-4 and FR-514-AC-2 (TC-1026): units of one multi-unit check
    /// share a package graph, and each unit's inventory lists only its own
    /// report's records.
    #[trace("FR-362-AC-4", "FR-514-AC-2", "TC-1026", "TC-1309")]
    #[test]
    fn each_unit_of_a_multi_unit_check_inventories_only_its_own_records() {
        let (id_a, id_b) = (native("unit:a"), native("unit:b"));
        let a = parse_text(
            &id_a,
            &format!("{HEADER}{GUARDED}\n"),
            SpineLimits::default(),
        );
        let b = parse_text(
            &id_b,
            &format!("{HEADER}{GUARDED_G}\n"),
            SpineLimits::default(),
        );
        let dependencies = DependencyInput::default();
        let admitted = admit_parsed_unit_list(vec![a.clone(), b.clone()], &dependencies)
            .expect("distinct sources are admitted");
        let units = check_units(
            &admitted,
            "aggregate".to_owned(),
            &AdmittedModels::default(),
            &dependencies,
            SpineLimits::default(),
            &Cancel::new(),
        )
        .expect("both units check")
        .into_value();
        let package_records = units[0].package().graph().requirements().len();
        let expected: Vec<_> = units.iter().map(expected_rows).collect();
        let report_records: Vec<_> = units
            .iter()
            .map(|unit| unit.type_report().requirements().len())
            .collect();
        assert_eq!(
            package_records,
            report_records.iter().sum::<usize>(),
            "the shared graph holds both units' records"
        );
        let mut seen = Vec::new();
        for ((unit, parsed), expected) in units.into_iter().zip([&a, &b]).zip(expected) {
            let staged = Ok(Staged::new(unit));
            let outcome = qsl_inventory(Some(parsed), &staged, &formal("NativeTypes0"))
                .expect("an inventory");
            let rows = identities(&outcome);
            assert_eq!(rows, expected);
            seen.push(rows);
        }
        let own_requirements = |rows: &[SourceInventoryIdentity]| {
            rows.iter()
                .filter(|row| matches!(row, SourceInventoryIdentity::RequirementRecord(_)))
                .cloned()
                .collect::<Vec<_>>()
        };
        let (rows_a, rows_b) = (own_requirements(&seen[0]), own_requirements(&seen[1]));
        assert_eq!(rows_a.len() + rows_b.len(), package_records);
        assert!(rows_a.iter().all(|row| !rows_b.contains(row)));
    }
}
