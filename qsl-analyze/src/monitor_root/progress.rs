// SPDX-License-Identifier: AGPL-3.0-or-later
//! Same-emission admission of original model order and observation progress.

use qsl_semantics::check::{ModelTimeSource, TimeSourceKind};
use quire_observation::authority::{clock, progress, SubjectSelection};
use quire_observation::Identity;

/// An independently selected progress field that does not match.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProgressSelectionRefusalCode {
    #[error("progress belongs to another selected subject")]
    /// The observation scope or population differs.
    Subject,
    #[error("progress belongs to another selected clock or revision")]
    /// The selected observation clock or revision differs.
    Clock,
    #[error("progress row names another observation source")]
    /// The selected observation source differs.
    Source,
    #[error("progress row names another model")]
    /// The checked model selection differs.
    Model,
    #[error("progress row names another time unit")]
    /// The checked time unit differs.
    Unit,
    #[error("progress row names another time source")]
    /// The checked dense/tick source, operation or period differs.
    TimeSource,
    /// A complete structural model, unit, dimension or operation key differs.
    #[error("progress row names another structural node")]
    Node,
    /// No row names the selected complete claim occurrence.
    #[error("progress has no row for the selected claim occurrence")]
    Claim,
    /// More than one row names the complete selected claim and source.
    #[error("progress has ambiguous rows for the selected claim and source")]
    Ambiguous,
}

impl ProgressSelectionRefusalCode {
    /// Every stable independent selection refusal.
    pub const fn all() -> &'static [Self] {
        &[
            Self::Subject,
            Self::Clock,
            Self::Source,
            Self::Model,
            Self::Unit,
            Self::TimeSource,
            Self::Node,
            Self::Claim,
            Self::Ambiguous,
        ]
    }

    /// Stable catalog spelling for this compared field.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Clock => "clock",
            Self::Source => "source",
            Self::Model => "model",
            Self::Unit => "unit",
            Self::TimeSource => "time_source",
            Self::Node => "node",
            Self::Claim => "claim",
            Self::Ambiguous => "ambiguous",
        }
    }

    /// Resolve a catalog spelling without accepting unknown fields.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|value| value.as_str() == code)
    }
}

/// Check independent observation selections against the owner's actual value.
/// This borrows the original value; it never reconstructs progress authority.
pub(super) fn check_observation_selection(
    association: &progress::AssociationRef<'_>,
    subject: &SubjectSelection,
    clock: &clock::Selection,
) -> Result<(), ProgressSelectionRefusalCode> {
    if association.scope_identity() != subject.scope_identity.as_str()
        || association.population_identity() != subject.population_identity.as_str()
    {
        return Err(ProgressSelectionRefusalCode::Subject);
    }
    if association.clock_identity() != clock.clock_identity().as_str()
        || association.clock_revision() != clock.clock_revision().as_str()
    {
        return Err(ProgressSelectionRefusalCode::Clock);
    }
    Ok(())
}

/// Check only the non-node time-source fields of an asserted row against the
/// original checker facts. A successful component comparison does not admit
/// the row's structural node keys or its claim occurrence.
pub(super) fn check_time_source_fields(
    source: &ModelTimeSource,
    row: &progress::AssociationRow,
    observation_source: &Identity,
    clock: &clock::Selection,
) -> Result<(), ProgressSelectionRefusalCode> {
    if row.observation_source != observation_source.as_str() {
        return Err(ProgressSelectionRefusalCode::Source);
    }
    if row.observation_clock != clock.clock_identity().as_str()
        || row.observation_revision != clock.clock_revision().as_str()
    {
        return Err(ProgressSelectionRefusalCode::Clock);
    }
    if row.model != source.model {
        return Err(ProgressSelectionRefusalCode::Model);
    }
    if row.unit != source.unit {
        return Err(ProgressSelectionRefusalCode::Unit);
    }
    match (&source.source, &row.source) {
        (TimeSourceKind::Dense, progress::TimeSource::Dense) => Ok(()),
        (
            TimeSourceKind::Tick { operation, period },
            progress::TimeSource::Tick {
                operation: selected_operation,
                period_numerator,
                period_denominator,
                ..
            },
        ) if operation == selected_operation
            && period.numerator().to_string() == *period_numerator
            && period.denominator().to_string() == *period_denominator =>
        {
            Ok(())
        }
        _ => Err(ProgressSelectionRefusalCode::TimeSource),
    }
}

#[cfg(test)]
mod tests;

pub(super) mod charge;
mod time_source;

use qsl_foundation::SourceIdentity;
use qsl_replay::MonitorSubject;
use quire_contract_model::{
    read_checked_package, AdmissionBinding, CheckedMonitorClockReadError, CheckedOccurrence,
    CheckedOccurrenceRole, CheckedPackageDispatchResult, CheckedPackageIncomplete,
    CheckedPackageReadLimits, CheckedPackageRefusal, CheckedPackageV2, CheckedRational,
    NativeJoinBudget, NativeJoinCharge, NativeJoinDeniedCharge, NativeJoinLocus,
};

use super::{order::selected_time_source, ModelStepOrder, ModelStepOrderRefusal, PlanRoot};

/// Independent selections checked before a same-emission authority is issued.
/// QSL source provenance and observation source identity are distinct domains.
#[derive(Clone, Copy, Debug)]
pub struct PlanAuthoritySelection<'a> {
    /// The original QSL source authority and identity, not an observation source label.
    pub source: &'a SourceIdentity,
    /// The original checked clause node and exact claim occurrence.
    pub claim: &'a MonitorSubject,
    /// Independently selected observation scope and population.
    pub observation_subject: &'a SubjectSelection,
    /// Independently selected observation clock and revision.
    pub observation_clock: &'a clock::Selection,
    /// Independently selected observation source.
    pub observation_source: &'a Identity,
}

/// Additional owner structural validation caps, passed through unchanged.
/// These caps neither account for preparation nor replenish its native budget.
#[derive(Clone, Copy, Debug)]
pub struct PlanAuthorityValidationCaps {
    /// Bounds for admitting this final emission through the original IR reader.
    pub read: CheckedPackageReadLimits,
    /// Bounds for the canonical model-step order value.
    pub order: quire_canonical::Limits,
}

/// Stable family of a complete plan-authority refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanAuthorityRefusalCode {
    /// No independently admitted progress view was supplied.
    MissingProgress,
    /// The requested original QSL source differs from the retained source.
    Source,
    /// The requested original checked clause or occurrence differs.
    Claim,
    /// An independent observation or model-time field differs.
    Selection,
    /// The final emission does not carry the retained clause binding.
    FinalEmission,
    /// Original checked model-step order admission refused.
    Order,
    /// The original observation owner refused its association or byte bound.
    Progress,
    /// The original strict package reader refused this final emission.
    ReadRefused,
    /// The original strict package reader exhausted a caller bound.
    ReadIncomplete,
    /// The original admitted monitor clock lookup refused.
    ClockRead,
    /// The caller's actual shared native preparation budget denied a charge.
    Budget,
    /// A paid temporary container could not reserve storage.
    Allocation,
}

impl PlanAuthorityRefusalCode {
    /// Every stable refusal family.
    pub const fn all() -> &'static [Self] {
        &[
            Self::MissingProgress,
            Self::Source,
            Self::Claim,
            Self::Selection,
            Self::FinalEmission,
            Self::Order,
            Self::Progress,
            Self::ReadRefused,
            Self::ReadIncomplete,
            Self::ClockRead,
            Self::Budget,
            Self::Allocation,
        ]
    }

    /// The stable catalog spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingProgress => "missing_progress",
            Self::Source => "source",
            Self::Claim => "claim",
            Self::Selection => "selection",
            Self::FinalEmission => "final_emission",
            Self::Order => "order",
            Self::Progress => "progress",
            Self::ReadRefused => "read_refused",
            Self::ReadIncomplete => "read_incomplete",
            Self::ClockRead => "clock_read",
            Self::Budget => "budget",
            Self::Allocation => "allocation",
        }
    }

    /// Resolve a catalog spelling without accepting aliases.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|value| value.as_str() == code)
    }
}

/// A typed refusal with no partial admitted context and original owner causes.
#[derive(Debug, thiserror::Error)]
pub enum PlanAuthorityRefusal {
    /// No progress view was supplied.
    #[error("plan authority requires independently admitted progress")]
    MissingProgress,
    /// The independently requested original QSL source differs.
    #[error("plan authority selects another original QSL source")]
    Source,
    /// The independently requested original claim differs.
    #[error("plan authority selects another checked clause or claim occurrence")]
    Claim,
    /// An independent observation or checked model-time selection differs.
    #[error(transparent)]
    Selection(#[from] ProgressSelectionRefusalCode),
    /// The retained rows do not join the original clause to this final emission.
    #[error("plan authority has no same-emission final clause binding")]
    FinalEmission,
    /// The original QSL order admission refused.
    #[error(transparent)]
    Order(#[from] Box<ModelStepOrderRefusal>),
    /// The original observation owner refused.
    #[error(transparent)]
    Progress(#[from] Box<quire_observation::authority::Error>),
    /// The original strict reader refused the final package.
    #[error("the final emission was refused by the strict reader: {0:?}")]
    ReadRefused(Box<CheckedPackageRefusal>),
    /// The original strict reader exhausted its bound.
    #[error("the final emission read was incomplete: {0:?}")]
    ReadIncomplete(Box<CheckedPackageIncomplete>),
    /// The actual owner clock lookup refused under the shared native ledger.
    #[error(transparent)]
    ClockRead(#[from] CheckedMonitorClockReadError),
    /// Original fixed-size native denial metadata; constructing this allocates nothing.
    #[error("shared native preparation budget denied {denied:?}")]
    Budget {
        /// The original denied kind and full attempted amount.
        denied: NativeJoinDeniedCharge,
        /// The owner's original non-owning first-unpaid location.
        locus: Option<NativeJoinLocus>,
        /// The original configured limit for the denied resource.
        configured_bound: u64,
        /// All consumption admitted before this refusal.
        consumed: NativeJoinCharge,
    },
    /// A previously paid temporary container reserve failed.
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
}

impl From<ModelStepOrderRefusal> for PlanAuthorityRefusal {
    fn from(error: ModelStepOrderRefusal) -> Self {
        Self::Order(Box::new(error))
    }
}

impl From<quire_observation::authority::Error> for PlanAuthorityRefusal {
    fn from(error: quire_observation::authority::Error) -> Self {
        Self::Progress(Box::new(error))
    }
}

impl PlanAuthorityRefusal {
    /// The stable refusal family; inspect the variant for the original cause.
    pub const fn code(&self) -> PlanAuthorityRefusalCode {
        match self {
            Self::MissingProgress => PlanAuthorityRefusalCode::MissingProgress,
            Self::Source => PlanAuthorityRefusalCode::Source,
            Self::Claim => PlanAuthorityRefusalCode::Claim,
            Self::Selection(_) => PlanAuthorityRefusalCode::Selection,
            Self::FinalEmission => PlanAuthorityRefusalCode::FinalEmission,
            Self::Order(_) => PlanAuthorityRefusalCode::Order,
            Self::Progress(_) => PlanAuthorityRefusalCode::Progress,
            Self::ReadRefused(_) => PlanAuthorityRefusalCode::ReadRefused,
            Self::ReadIncomplete(_) => PlanAuthorityRefusalCode::ReadIncomplete,
            Self::ClockRead(_) => PlanAuthorityRefusalCode::ClockRead,
            Self::Budget { .. } => PlanAuthorityRefusalCode::Budget,
            Self::Allocation(_) => PlanAuthorityRefusalCode::Allocation,
        }
    }
}

/// Original admitted values and capabilities bound to one immutable plan root.
/// Neither the final emission nor its original checked context is replaceable.
///
/// A previously admitted context cannot be rebound to a foreign emission:
///
/// ```compile_fail,E0616
/// use qsl_analyze::monitor_root::{AdmittedPlanAuthority, PlanRoot};
/// fn replace(context: &mut AdmittedPlanAuthority, foreign: PlanRoot) {
///     context.root = foreign;
/// }
/// ```
///
/// Its canonical admitted values are immutable:
///
/// ```compile_fail,E0594
/// use qsl_analyze::monitor_root::AdmittedPlanAuthority;
/// fn replace(context: &AdmittedPlanAuthority) {
///     context.admitted_order()[0] = 0;
/// }
/// ```
#[derive(Debug)]
pub struct AdmittedPlanAuthority {
    root: PlanRoot,
    package: Box<CheckedPackageV2>,
    order: ModelStepOrder,
    progress: progress::View,
    time_source_slot: usize,
    checked_tick_period: Option<CheckedRational>,
}

impl AdmittedPlanAuthority {
    /// The original immutable plan, obligations, checked source and final emission.
    pub fn root(&self) -> &PlanRoot {
        &self.root
    }

    /// The original IR admission of that final emission, retaining its instance binding.
    pub fn admitted_package(&self) -> &CheckedPackageV2 {
        &self.package
    }

    /// Clone the original IR admission capability of this emitting package.
    /// A preparation caller accounts for the retained handle before acquiring it.
    pub fn admission_binding(&self) -> AdmissionBinding {
        self.package.admission_binding()
    }

    /// The original checked time source, borrowed in constant time.
    pub fn checked_time_source(&self) -> &ModelTimeSource {
        // The constructor selects this slot from this immutable root's slice.
        // No public method can replace the root or mutate its checked graph.
        &self.root.checked_unit().package().graph().time_sources()[self.time_source_slot]
    }

    /// The exact IR period copied once from this admitted emission; absent for dense time.
    pub fn checked_tick_period(&self) -> Option<&CheckedRational> {
        self.checked_tick_period.as_ref()
    }

    /// The complete QSL-admitted canonical model-step order VALUE.
    pub fn admitted_order(&self) -> &[u8] {
        self.order.bytes()
    }

    /// The original observation-owner canonical association VALUE, never document bytes.
    pub fn progress_context(&self) -> &[u8] {
        self.progress
            .association_canonical_bytes()
            .expect("the constructor retained this immutable owner's association bytes")
    }

    /// The original constructor-private observation owner capability.
    pub fn progress(&self) -> &progress::View {
        &self.progress
    }
}

impl PlanRoot {
    /// Admit the original order and genuine progress association for this final emission.
    ///
    /// Original source/claim selections are checked independently; node keys are
    /// compared structurally, and the original claim joins through this emission's
    /// retained rows to the strict reader's final clause. No second emission is made.
    pub fn admit_authority(
        self,
        selection: PlanAuthoritySelection<'_>,
        progress: Option<progress::View>,
        validation: PlanAuthorityValidationCaps,
        budget: &mut NativeJoinBudget,
        locus: NativeJoinLocus,
    ) -> Result<AdmittedPlanAuthority, PlanAuthorityRefusal> {
        budget.resume();
        let mut charge = charge::Charge { budget, locus };
        charge.work(1)?;
        let progress = progress.ok_or(PlanAuthorityRefusal::MissingProgress)?;
        let original_source = self.checked_unit().source().identity();
        if !charge.text(&selection.source.authority, &original_source.authority)?
            || !charge.text(&selection.source.identity, &original_source.identity)?
        {
            return Err(PlanAuthorityRefusal::Source);
        }
        if !charge.subject(selection.claim, self.plan().subject())? {
            return Err(PlanAuthorityRefusal::Claim);
        }
        // Retain one index into the original immutable checked source slice.
        charge.storage(1)?;
        let (_, source, time_source_slot) = selected_time_source(&self)?;
        let association = progress.association()?;
        check_observation_selection(
            &association,
            selection.observation_subject,
            selection.observation_clock,
        )?;
        let claim = self.plan().subject();
        let mut rows = association.rows().iter().filter(|row| {
            row.claim.clause_node().key() == claim.clause_node()
                && row.claim.occurrence() == claim.occurrence().origin()
                && row.observation_source == selection.observation_source.as_str()
        });
        let row = rows.next().ok_or_else(|| {
            if association.rows().iter().any(|row| {
                row.claim.clause_node().key() == claim.clause_node()
                    && row.claim.occurrence() == claim.occurrence().origin()
            }) {
                ProgressSelectionRefusalCode::Source
            } else {
                ProgressSelectionRefusalCode::Claim
            }
        })?;
        if rows.next().is_some() {
            return Err(ProgressSelectionRefusalCode::Ambiguous.into());
        }
        check_time_source_fields(
            source,
            row,
            selection.observation_source,
            selection.observation_clock,
        )?;
        if row.model_node.key() != &source.model_node
            || row.unit_node.key() != &source.unit_node
            || row.time_dimension.key() != &source.time_dimension
        {
            return Err(ProgressSelectionRefusalCode::Node.into());
        }
        if let progress::TimeSource::Tick { operation_node, .. } = &row.source {
            if source.tick_operation_anchor.as_ref() != Some(operation_node.key()) {
                return Err(ProgressSelectionRefusalCode::Node.into());
            }
        }
        let declaration = self
            .emission()
            .temporal_binding_rows()
            .declaration(claim.clause_node(), claim.occurrence().origin().ordinal())
            .ok_or(PlanAuthorityRefusal::FinalEmission)?;
        let package = match read_checked_package(
            self.emission().package().bytes(),
            validation.read,
            self.emission().evidence(),
        ) {
            CheckedPackageDispatchResult::AdmittedV2(package) => package,
            CheckedPackageDispatchResult::Refused(error) => {
                return Err(PlanAuthorityRefusal::ReadRefused(Box::new(error)));
            }
            CheckedPackageDispatchResult::Incomplete(error) => {
                return Err(PlanAuthorityRefusal::ReadIncomplete(Box::new(error)));
            }
        };
        let checked_tick_period = {
            let mut plans = package.monitor_plans();
            let final_plan = plans.next().ok_or(PlanAuthorityRefusal::FinalEmission)?;
            if plans.next().is_some()
                || final_plan.clause() != declaration
                || !package.temporal_clause_names().is_some_and(|rows| {
                    rows.iter().any(|row| {
                        &row.declaration == declaration
                            && row.occurrence == claim.occurrence().origin().ordinal()
                    })
                })
            {
                return Err(PlanAuthorityRefusal::FinalEmission);
            }
            if !charge.text(claim.occurrence().origin().role().as_str(), "claim")? {
                return Err(PlanAuthorityRefusal::Claim);
            }
            let occurrence = CheckedOccurrence {
                role: CheckedOccurrenceRole::Claim,
                ordinal: claim.occurrence().origin().ordinal(),
            };
            let clock = final_plan.clock_for_claim(declaration, &occurrence, charge.budget)?;
            time_source::checked_period(&package, clock.clock(), row, &mut charge)?
        };
        progress.association_canonical_bytes()?;
        let order = self.model_step_order_with_period_charged(
            validation.order,
            checked_tick_period.as_ref(),
            &mut charge,
        )?;
        Ok(AdmittedPlanAuthority {
            root: self,
            package,
            order,
            progress,
            time_source_slot,
            checked_tick_period,
        })
    }
}
