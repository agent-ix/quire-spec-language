// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-362 item 3 and item 10: the owner-issued source inventory of QSL's
//! checked requirements (IR FR-052), built from a [`spine::CheckedUnit`] and
//! from nothing else.
//!
//! The rows are one `RequirementRecord` per entry of the checked graph's
//! requirement map, in the map's order, under the unit's formal source
//! identity. The spine's type phase (S3) either checks a unit or refuses it,
//! and a refused unit is an error outcome of [`spine::check`] that yields no
//! `CheckedUnit` and so no rows (FR-362-AC-8), so the traversal of a checked
//! unit always reaches its end.

use qsl_semantics::check::wire_id::structural_id;
use quire_contract_model::source_obligation::{
    SourceInventoryEnumerator, SourceInventoryIdentity, SourceInventoryIncompleteReason,
    SourceInventoryOutcome, SourceInventoryOutcomeError, SourceInventoryOwner, SourceInventoryRow,
    SourceInventoryScope, SourceRequirementOccurrenceId,
};
use quire_contract_model::SourceIdentity;
use qsl_foundation::source::provenance::OccurrenceKey;

use crate::spine::CheckedUnit;

/// Why a checked unit has no source inventory.
#[derive(Debug, thiserror::Error)]
pub enum InventoryRefusal {
    /// The unit carries no formal source identity: the caller never selected
    /// one, so the inventory has no identity to be issued under.
    #[error("the checked unit carries no formal source identity")]
    NoFormalIdentity,
    /// The enumeration was inconsistent, which IR's outcome constructor
    /// reports.
    #[error("the source inventory enumeration was inconsistent: {0}")]
    Outcome(#[from] SourceInventoryOutcomeError),
}

/// QSL's own traversal of the checked unit's requirement records.
struct Traversal<'u> {
    identity: &'u SourceIdentity,
    rows: Box<dyn Iterator<Item = &'u OccurrenceKey> + 'u>,
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
        let Some(key) = self.rows.next() else {
            return Ok(None);
        };
        self.observed += 1;
        Ok(Some(SourceInventoryRow::new(
            self.identity.clone(),
            SourceInventoryIdentity::RequirementRecord(SourceRequirementOccurrenceId {
                node: structural_id(key.node()),
                role: key.origin().role().as_str().into(),
                ordinal: key.origin().ordinal(),
            }),
        )))
    }

    fn observed_count(&self) -> usize {
        self.observed
    }
}

/// Report QSL's inventory of one checked unit as an owner-issued outcome:
/// one `RequirementRecord` row per requirement record of the unit's checked
/// graph, `Complete` (FR-362-AC-4).
///
/// # Errors
///
/// [`InventoryRefusal::NoFormalIdentity`] when the unit carries no formal
/// source identity, and [`InventoryRefusal::Outcome`] when IR's outcome
/// constructor rejects the enumeration.
pub fn qsl_inventory(unit: &CheckedUnit) -> Result<SourceInventoryOutcome, InventoryRefusal> {
    let identity = unit
        .formal_identity()
        .ok_or(InventoryRefusal::NoFormalIdentity)?;
    let traversal = Traversal {
        identity,
        rows: Box::new(unit.package().graph().requirements().keys()),
        observed: 0,
    };
    Ok(SourceInventoryOutcome::from_enumerator(traversal)?)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::StageFailure;
    use qsl_foundation::SourceIdentity as NativeIdentity;
    use quire_contract_model::source_obligation::SourceInventoryCompleteness;
    use quire_contract_model::{SourceDocumentId, SourceRevision};
    use quire_exact::Cancel;

    use super::*;
    use crate::spine::{
        check, parse, select, AdmittedModels, DependencyInput, ParseRequest, SpineLimits,
    };

    const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n";

    /// Two claim sites: one guarded application per branch of one node.
    const GUARDED: &str = "function f using v(x: Int[0, 9], p: Boolean): Integer pure \
        { if p then x + 1 else x + 1 }";

    fn formal() -> SourceIdentity {
        SourceIdentity::new(
            SourceDocumentId::new("NativeTypes0").expect("a valid document id"),
            SourceRevision::new(1).expect("a valid revision"),
        )
    }

    fn checked(text: &str) -> Result<CheckedUnit, StageFailure<Box<crate::spine::CompileRefusal>>> {
        let cancel = Cancel::new();
        let limits = SpineLimits::default();
        let identity = NativeIdentity::new("test", "unit:main", "test", "authored");
        let parsed = parse(
            &ParseRequest {
                source: &identity,
                path: "unit.native",
                bytes: text.as_bytes(),
            },
            limits.source,
            &cancel,
        )?
        .into_value();
        let models: AdmittedModels = select(
            &parsed,
            &qsl_semantics::model::intake::PackageInput::default(),
            limits.model,
            &cancel,
        )?
        .into_value();
        Ok(check(
            &parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &cancel,
        )?
        .into_value())
    }

    /// FR-362-AC-4 (TC-1027 step 1): the inventory's rows are the unit's own
    /// requirement records, issued `Complete` under the selected identity.
    #[trace("FR-362-AC-4", "TC-1027")]
    #[test]
    fn a_checked_unit_yields_the_rows_of_its_own_graph() {
        let unit = checked(&format!("{HEADER}{GUARDED}\n"))
            .expect("the fixture checks")
            .with_formal_identity(formal());
        let outcome = qsl_inventory(&unit).expect("a consistent enumeration");
        assert_eq!(outcome.owner(), SourceInventoryOwner::Qsl);
        assert_eq!(
            outcome.scope(),
            SourceInventoryScope::QslCheckedRequirementsAndPendingObligations
        );
        assert_eq!(outcome.source(), &formal());
        assert_eq!(outcome.completeness(), &SourceInventoryCompleteness::Complete);
        assert_eq!(outcome.observed_count(), outcome.items().len());
        let expected: Vec<_> = unit
            .package()
            .graph()
            .requirements()
            .keys()
            .map(|key| (structural_id(key.node()), key.origin().ordinal()))
            .collect();
        assert!(expected.len() >= 2, "both guarded occurrences are records");
        let reported: Vec<_> = outcome
            .items()
            .iter()
            .map(|row| match row.identity() {
                SourceInventoryIdentity::RequirementRecord(record) => {
                    (record.node.clone(), record.ordinal)
                }
                other => panic!("a spine unit reports no {other:?} row"),
            })
            .collect();
        assert_eq!(reported, expected);
        let distinct: BTreeSet<_> = reported.iter().map(|(_, ordinal)| *ordinal).collect();
        assert!(!distinct.is_empty());
    }

    /// FR-362-AC-4: the rows come from the unit alone, so a unit whose source
    /// holds no claim site reports no rows and is still `Complete`.
    #[trace("FR-362-AC-4", "TC-1027")]
    #[test]
    fn a_unit_with_no_requirement_records_is_complete_with_no_rows() {
        let unit = checked(&format!("{HEADER}function g using v(): Integer pure {{ 1 }}\n"))
            .expect("the fixture checks")
            .with_formal_identity(formal());
        let outcome = qsl_inventory(&unit).expect("a consistent enumeration");
        assert_eq!(outcome.items().len(), unit.package().graph().requirements().len());
        assert_eq!(outcome.completeness(), &SourceInventoryCompleteness::Complete);
    }

    /// FR-362 item 3: the inventory is issued under the formal identity the
    /// unit carries, and a unit without one has none to issue it under.
    #[trace("FR-362-AC-4", "TC-1027")]
    #[test]
    fn a_unit_without_a_formal_identity_has_no_inventory() {
        let unit = checked(&format!("{HEADER}{GUARDED}\n")).expect("the fixture checks");
        assert!(unit.formal_identity().is_none());
        assert!(matches!(
            qsl_inventory(&unit),
            Err(InventoryRefusal::NoFormalIdentity)
        ));
    }

    /// FR-362-AC-8: a unit the type phase refuses is an error outcome of
    /// `check`, so there is no checked unit and no inventory row.
    #[trace("FR-362-AC-8", "TC-1027")]
    #[test]
    fn a_unit_the_type_phase_refuses_yields_no_checked_unit() {
        let refused = checked(&format!(
            "{HEADER}function h using v(x: Int[0, 9]): Integer pure {{ x + true }}\n"
        ));
        assert!(matches!(refused, Err(StageFailure::Refused(_))));
    }

    /// FR-362-AC-8: a type phase that stops on a reached stage limit returns
    /// the limit, not a unit, so no partial rows are issued.
    #[trace("FR-362-AC-8", "TC-1027")]
    #[test]
    fn a_unit_whose_type_phase_reaches_a_limit_yields_no_checked_unit() {
        let cancel = Cancel::new();
        let mut limits = SpineLimits::default();
        limits.checking = quire_semantic_value::checking::CheckingLimits::new(1);
        let identity = NativeIdentity::new("test", "unit:main", "test", "authored");
        let text = format!("{HEADER}{GUARDED}\n");
        let parsed = parse(
            &ParseRequest {
                source: &identity,
                path: "unit.native",
                bytes: text.as_bytes(),
            },
            limits.source,
            &cancel,
        )
        .expect("S1 reads the unit")
        .into_value();
        let models = select(
            &parsed,
            &qsl_semantics::model::intake::PackageInput::default(),
            limits.model,
            &cancel,
        )
        .expect("I1 admits no model")
        .into_value();
        let outcome = check(
            &parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &cancel,
        );
        assert!(matches!(outcome, Err(StageFailure::Limit(_))));
    }
}
