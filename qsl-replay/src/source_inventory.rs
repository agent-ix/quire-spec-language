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
    SourceByteRegion, SourceDeclarationHandle, SourceInventoryEnumerator, SourceInventoryIdentity,
    SourceInventoryIncompleteReason, SourceInventoryOutcome, SourceInventoryOutcomeError,
    SourceInventoryOwner, SourceInventoryRow, SourceInventoryScope, SourceObligationSite,
    SourcePendingObligationId, SourcePendingObligationKind, SourceRequirementFormId,
    SourceRequirementOccurrenceId, SourceTypeExhaustion, SourceTypeExhaustionSite,
    SourceTypeWorkDimension, SourceUnitHandle,
};
use quire_contract_model::SourceIdentity;

use crate::spine::{CheckedUnit, FrontEndFailure, ParsedUnitSource, PendingObligation};

/// Why a check outcome has no source inventory.
#[derive(Debug, thiserror::Error)]
pub enum InventoryRefusal {
    /// The check outcome is neither a checked unit nor a stage limit: the
    /// unit was refused, cancelled or faulted, so there are no rows.
    #[error("the check outcome is neither a checked unit nor a stage limit")]
    NoInventory,
    /// No declaration of the unit's type report holds a pending
    /// obligation's source span, so it has no owning declaration to report.
    #[error("no declaration of the type report holds a pending obligation's span")]
    UnownedObligation,
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

/// The cause IR's `TypeWorkExhausted` retains for a reached stage limit
/// (FR-514): the owning stage and FR-255 setting names, the kind as a
/// dimension, the configured bound as the limit and the counter value that
/// reached it as the observed amount, at the limit's source region when it
/// has one. The spine charges no amount before the refused step and holds no
/// declaration, unit or expression handle for the limit, so `prior`,
/// `requested` and the native site are absent. `None` when the limit's kind
/// is not a type-admission stage limit (see [`dimension`]).
fn exhaustion(limit: &LimitExceeded) -> Option<SourceInventoryIncompleteReason> {
    let dimension = dimension(limit.kind())?;
    let region = match limit.locus() {
        Some(Locus::Region(region)) => Some(SourceByteRegion {
            authority: region.source().authority().into(),
            identity: region.source().identity().into(),
            start: region.start(),
            end: region.end(),
        }),
        _ => None,
    };
    Some(SourceInventoryIncompleteReason::TypeWorkExhausted(
        Box::new(SourceTypeExhaustion {
            stage: Some(limit.setting().stage().into()),
            setting: Some(limit.setting().name().into()),
            dimension,
            limit: limit.configured_bound(),
            observed: Some(limit.actual()),
            prior: None,
            requested: None,
            site: SourceTypeExhaustionSite {
                native: None,
                region,
            },
        }),
    ))
}

fn requirement(key: &qsl_foundation::source::provenance::OccurrenceKey) -> SourceInventoryIdentity {
    SourceInventoryIdentity::RequirementRecord(SourceRequirementOccurrenceId {
        node: structural_id(key.node()),
        role: key.origin().role().as_str().into(),
        ordinal: key.origin().ordinal(),
    })
}

/// The identity of one pending obligation (FR-514): its exact capability
/// kind name, the ordinal of its owning declaration in the report's
/// declaration list (the smallest declaration holding its span), unit 0 and
/// its source span. `None` when no declaration holds the span.
fn obligation(pending: &PendingObligation) -> Option<SourceInventoryIdentity> {
    let declaration = pending.declaration_ordinal()?;
    let span = pending.span();
    Some(SourceInventoryIdentity::PendingObligation(
        SourcePendingObligationId {
            site: SourceObligationSite {
                declaration: SourceDeclarationHandle(declaration),
                unit: SourceUnitHandle(0),
                expression: None,
                start: span.start,
                end: span.end,
            },
            kind: SourcePendingObligationKind::CapabilityKind(pending.capability_kind().into()),
        },
    ))
}

/// The identity of the `index`th S2 requirement form: its source-order
/// ordinal and declared name, naming no checked node.
fn form_requirement(index: usize, name: Option<&str>) -> SourceInventoryIdentity {
    SourceInventoryIdentity::RequirementForm(SourceRequirementFormId {
        ordinal: u64::try_from(index).unwrap_or(u64::MAX),
        declared_name: name.map(Into::into),
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
            let mut rows: Vec<_> = report
                .requirements()
                .iter()
                .map(|required| requirement(required.occurrence()))
                .collect();
            for pending in report.obligations() {
                rows.push(obligation(pending).ok_or(InventoryRefusal::UnownedObligation)?);
            }
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
        NativeIdentity::new("test", identity, "test", "authored")
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

    fn type_work_cause(outcome: &SourceInventoryOutcome) -> &SourceTypeExhaustion {
        let SourceInventoryCompleteness::Incomplete(
            SourceInventoryIncompleteReason::TypeWorkExhausted(cause),
        ) = outcome.completeness()
        else {
            panic!("a stage limit is the type-work exhaustion cause");
        };
        cause
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
                    .map(|pending| obligation(pending).expect("an owned obligation")),
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
        for (row, pending) in rows[2..].iter().zip(report.obligations()) {
            let SourceInventoryIdentity::PendingObligation(id) = row else {
                panic!("an obligation row");
            };
            assert_eq!(
                id.kind,
                SourcePendingObligationKind::CapabilityKind(pending.capability_kind().into()),
                "the exact capability-kind label, not a type-level kind"
            );
            assert_eq!(id.site.start, pending.span().start);
            assert_eq!(id.site.end, pending.span().end);
            assert_eq!(id.site.unit, SourceUnitHandle(0));
            assert_eq!(id.site.expression, None);
            assert_eq!(id.site.declaration, SourceDeclarationHandle(0));
        }
        assert_eq!(
            report.obligations()[0].capability_kind(),
            "value-validity",
            "the measured label of a guarded application claim"
        );
    }

    /// FR-514-AC-8 (TC-1309 row 13): each obligation row names the ordinal of
    /// the declaration that holds its span, so obligations of two functions
    /// carry two declaration handles.
    #[trace("FR-514-AC-8", "TC-1309")]
    #[test]
    fn an_obligation_row_names_the_declaration_that_holds_its_span() {
        let parsed = parse_text(
            &native("unit:main"),
            &format!("{HEADER}{GUARDED}\n{GUARDED_G}\n"),
            SpineLimits::default(),
        );
        let checked = run_check(&parsed, SpineLimits::default());
        let unit = checked.as_ref().expect("the fixture checks").value();
        let declarations = unit.type_report().declarations();
        let ordinal_of = |name: &str| {
            declarations
                .iter()
                .position(|declaration| format!("{:?}", declaration.node()).contains(name))
                .expect("the declaration is in the report")
        };
        let (f, g) = (ordinal_of("\"f\""), ordinal_of("\"g\""));
        assert_ne!(f, g);
        let outcome =
            qsl_inventory(Some(&parsed), &checked, &formal("NativeTypes0")).expect("an inventory");
        let handles: Vec<_> = identities(&outcome)
            .into_iter()
            .filter_map(|row| match row {
                SourceInventoryIdentity::PendingObligation(id) => {
                    Some((id.site.declaration, id.site.start))
                }
                _ => None,
            })
            .collect();
        assert_eq!(handles.len(), 4);
        let text = unit.source().text();
        for (handle, start) in handles {
            let expected = if start < text.find("function g").expect("g is present") {
                f
            } else {
                g
            };
            assert_eq!(handle, SourceDeclarationHandle(expected));
        }
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
        assert_eq!(
            rows,
            vec![
                SourceInventoryIdentity::RequirementForm(SourceRequirementFormId {
                    ordinal: 0,
                    declared_name: Some("f".into()),
                }),
                SourceInventoryIdentity::RequirementForm(SourceRequirementFormId {
                    ordinal: 1,
                    declared_name: Some("g".into()),
                }),
            ],
            "the two functions are requirement forms; the record is none"
        );
        assert_eq!(outcome.observed_count(), 2);
        let cause = type_work_cause(&outcome);
        assert_eq!(cause.stage.as_deref(), Some("S3"));
        assert_eq!(cause.setting.as_deref(), Some("s3.work_units"));
        assert_eq!(cause.dimension, SourceTypeWorkDimension::Constraints);
        assert_eq!(cause.limit, limit.configured_bound());
        assert_eq!(cause.observed, Some(limit.actual()));
        assert!(cause.observed > Some(u128::from(cause.limit)));
        assert_eq!(cause.prior, None);
        assert_eq!(cause.requested, None);
        assert_eq!(cause.site.native, None);
    }

    /// FR-514-AC-7 (TC-1309): a requirement form never equals a complete
    /// requirement record, even when its ordinal and name match.
    #[trace("FR-514-AC-7", "TC-1309")]
    #[test]
    fn a_requirement_form_identity_is_distinct_from_a_requirement_record() {
        let form = form_requirement(0, Some("f"));
        let unnamed = form_requirement(0, None);
        assert_ne!(form, unnamed);
        assert_ne!(form, form_requirement(1, Some("f")));
        assert_eq!(form, form_requirement(0, Some("f")));
        let parsed = parse_text(
            &native("unit:main"),
            &format!("{HEADER}{GUARDED}\n"),
            SpineLimits::default(),
        );
        let checked = run_check(&parsed, SpineLimits::default());
        let rows = expected_rows(checked.as_ref().expect("checks").value());
        assert!(rows.iter().all(|row| row != &form));
    }

    /// FR-514-AC-9 (TC-1309 row 14): an observed amount above `u64::MAX`
    /// survives unchanged and a region keeps its source authority and
    /// identity.
    #[trace("FR-514-AC-9", "TC-1309")]
    #[test]
    fn a_wide_observed_amount_and_the_source_region_survive_unchanged() {
        let region = qsl_foundation::source::provenance::SourceRegion::new(
            qsl_foundation::source::provenance::RawSourceRef::new("auth", "doc:1")
                .expect("a source ref"),
            3,
            9,
        )
        .expect("a region");
        let actual = u128::from(u64::MAX) + 7;
        let limit =
            LimitExceeded::new(Setting::S3WorkUnits, 4, actual).at(Some(Locus::Region(region)));
        let Some(SourceInventoryIncompleteReason::TypeWorkExhausted(cause)) = exhaustion(&limit)
        else {
            panic!("a work-budget limit is a type-work cause");
        };
        assert_eq!(cause.observed, Some(actual));
        assert_eq!(cause.limit, 4);
        assert_eq!(
            cause.site.region,
            Some(SourceByteRegion {
                authority: "auth".into(),
                identity: "doc:1".into(),
                start: 3,
                end: 9,
            })
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
        let observed = limit.actual();
        let outcome = qsl_inventory(
            None,
            &Err::<Staged<CheckedUnit>, _>(failure),
            &formal("NativeTypes0"),
        )
        .expect("an inventory");
        assert!(outcome.items().is_empty());
        let cause = type_work_cause(&outcome);
        assert_eq!(cause.stage.as_deref(), Some("S1"));
        assert_eq!(cause.setting.as_deref(), Some("s1.nodes"));
        assert_eq!(cause.dimension, SourceTypeWorkDimension::Expressions);
        assert_eq!(cause.limit, 1);
        assert_eq!(cause.observed, Some(observed));
        assert!(cause.observed > Some(1));
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
