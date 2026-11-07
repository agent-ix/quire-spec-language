// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251: canonical model-step order values from the original checked owner.

use qsl_semantics::check::wire_id::structural_id;
use qsl_semantics::check::{ModelTimeSource, TemporalClauseNode, TimeSourceKind};
use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::{NodeKey, Rational};

use super::PlanRoot;

mod charge;

/// Original canonical model-step order value, admitted by the QSL producer.
/// Its node components are original checker keys, not final writer addresses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelStepOrder {
    bytes: Vec<u8>,
}

impl ModelStepOrder {
    /// The complete canonical value, never a bare identity or source document.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Stable reason a checked owner supplies no model-step order value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelStepOrderRefusalCode {
    /// The retained unit does not contain the selected clause and occurrence.
    MissingClause,
    /// The selected clause has no checked model-time binding.
    MissingModelTime,
    /// The selected binding does not select exactly one checked time source.
    MissingTimeSource,
    /// A digital source has no checked operation anchor.
    MissingTickOperation,
    /// The canonical encoder refused its finite allowance or value.
    Encoding,
}

impl ModelStepOrderRefusalCode {
    /// Every stable model-step admission refusal code.
    pub const fn all() -> &'static [Self] {
        &[
            Self::MissingClause,
            Self::MissingModelTime,
            Self::MissingTimeSource,
            Self::MissingTickOperation,
            Self::Encoding,
        ]
    }

    /// The stable catalog spelling of this condition.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingClause => "missing_clause",
            Self::MissingModelTime => "missing_model_time",
            Self::MissingTimeSource => "missing_time_source",
            Self::MissingTickOperation => "missing_tick_operation",
            Self::Encoding => "encoding",
        }
    }

    /// Resolve a catalog spelling without accepting unknown conditions.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|value| value.as_str() == code)
    }
}

/// Typed refusal without a partial admitted order value.
#[derive(Debug, thiserror::Error)]
#[error("model-step order refused: {code:?}")]
pub struct ModelStepOrderRefusal {
    /// Stable condition.
    pub code: ModelStepOrderRefusalCode,
    /// Original canonical encoder refusal, when encoding failed.
    #[source]
    pub encoding: Option<Error>,
}

pub(super) fn refuse(code: ModelStepOrderRefusalCode) -> ModelStepOrderRefusal {
    ModelStepOrderRefusal {
        code,
        encoding: None,
    }
}

/// Resolve the original clause and exactly its checked model time source.
/// This selection is shared by order encoding and progress-field comparison.
pub(super) fn selected_time_source(
    root: &PlanRoot,
) -> Result<(&TemporalClauseNode, &ModelTimeSource, usize), ModelStepOrderRefusal> {
    select_time_source(root, &mut OriginalSelection)
}

trait Selection {
    type Error: From<ModelStepOrderRefusal>;
    fn work(&mut self, amount: u128) -> Result<(), Self::Error>;
    fn text(&mut self, left: &str, right: &str) -> Result<bool, Self::Error>;
    fn node(&mut self, left: &NodeKey, right: &NodeKey) -> Result<bool, Self::Error>;
    fn kind(&mut self, left: &TimeSourceKind, right: &TimeSourceKind) -> Result<bool, Self::Error>;
}

struct OriginalSelection;

impl Selection for OriginalSelection {
    type Error = ModelStepOrderRefusal;
    fn work(&mut self, _: u128) -> Result<(), Self::Error> {
        Ok(())
    }
    fn text(&mut self, left: &str, right: &str) -> Result<bool, Self::Error> {
        Ok(left == right)
    }
    fn node(&mut self, left: &NodeKey, right: &NodeKey) -> Result<bool, Self::Error> {
        Ok(left == right)
    }
    fn kind(&mut self, left: &TimeSourceKind, right: &TimeSourceKind) -> Result<bool, Self::Error> {
        Ok(left == right)
    }
}

impl Selection for super::progress::charge::Charge<'_> {
    type Error = super::PlanAuthorityRefusal;
    fn work(&mut self, amount: u128) -> Result<(), Self::Error> {
        self.work(amount)
    }
    fn text(&mut self, left: &str, right: &str) -> Result<bool, Self::Error> {
        self.text(left, right)
    }
    fn node(&mut self, left: &NodeKey, right: &NodeKey) -> Result<bool, Self::Error> {
        self.node(left, right)
    }
    fn kind(&mut self, left: &TimeSourceKind, right: &TimeSourceKind) -> Result<bool, Self::Error> {
        self.work(2)?;
        match (left, right) {
            (TimeSourceKind::Dense, TimeSourceKind::Dense) => Ok(true),
            (
                TimeSourceKind::Tick {
                    operation: left_operation,
                    period: left,
                },
                TimeSourceKind::Tick {
                    operation: right_operation,
                    period: right,
                },
            ) => {
                if !self.text(left_operation, right_operation)? {
                    return Ok(false);
                }
                // copy_footprint inspects the retained limbs without warming a
                // cold integer cache. Integer equality itself borrows those limbs.
                for (left, right) in [
                    (left.numerator(), right.numerator()),
                    (left.denominator(), right.denominator()),
                ] {
                    self.work(
                        2 + left.copy_footprint().limbs as u128
                            + right.copy_footprint().limbs as u128,
                    )?;
                    if left != right {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

pub(super) fn selected_time_source_charged<'a>(
    root: &'a PlanRoot,
    charge: &mut super::progress::charge::Charge<'_>,
) -> Result<(&'a TemporalClauseNode, &'a ModelTimeSource, usize), super::PlanAuthorityRefusal> {
    select_time_source(root, charge)
}

fn select_time_source<'a, Policy: Selection>(
    root: &'a PlanRoot,
    policy: &mut Policy,
) -> Result<(&'a TemporalClauseNode, &'a ModelTimeSource, usize), Policy::Error> {
    let graph = root.checked_unit().package().graph();
    let subject = root.plan().subject();
    let mut selected = None;
    for clause in graph.temporal_clauses() {
        policy.work(1)?;
        if !policy.node(&clause.identity(), subject.clause_node())?
            || !policy.text(
                clause.claim().role().as_str(),
                subject.occurrence().origin().role().as_str(),
            )?
        {
            continue;
        }
        policy.work(2)?;
        if clause.claim().ordinal() == subject.occurrence().origin().ordinal() {
            selected = Some(clause);
            break;
        }
    }
    let clause = selected.ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingClause))?;
    let binding = clause
        .clause()
        .time_binding()
        .ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingModelTime))?;
    let mut model = None;
    for candidate in graph.timed_models() {
        policy.work(1)?;
        if policy.text(candidate.model(), &binding.model)? {
            model = Some(candidate);
            break;
        }
    }
    let model = model.ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingModelTime))?;
    let mut selected = None;
    for (slot, source) in graph.time_sources().iter().enumerate() {
        policy.work(1)?;
        if policy.text(&source.model, model.model())?
            && policy.node(&source.model_node, model.model_node())?
            && policy.kind(&source.source, &binding.source)?
            && policy.text(&source.unit, &binding.unit)?
        {
            if selected.is_some() {
                return Err(refuse(ModelStepOrderRefusalCode::MissingTimeSource).into());
            }
            selected = Some((slot, source));
        }
    }
    let (slot, source) =
        selected.ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingTimeSource))?;
    Ok((clause, source, slot))
}

impl PlanRoot {
    /// Admit the model-step order of this root's original checked clause.
    ///
    /// QSpec FR-252 establishes step order for both dense and digital model
    /// time. No event arrival order, numeric tick comparison or caller label
    /// supplies authority. Encoding is bounded by `limits`; original checker
    /// node components must be joined to final identities through the emission.
    pub fn model_step_order(
        &self,
        limits: Limits,
    ) -> Result<ModelStepOrder, ModelStepOrderRefusal> {
        self.model_step_order_with_period(limits, None)
    }

    pub(super) fn model_step_order_with_period_charged(
        &self,
        limits: Limits,
        admitted_period: Option<&quire_contract_model::CheckedRational>,
        budget: &mut super::progress::charge::Charge<'_>,
    ) -> Result<ModelStepOrder, super::PlanAuthorityRefusal> {
        let (_, source, _) = selected_time_source_charged(self, budget)?;
        if matches!(source.source, TimeSourceKind::Tick { .. }) && admitted_period.is_none() {
            return Err(super::PlanAuthorityRefusal::FinalEmission);
        }
        let value = order_value(source, admitted_period, |key| budget.node_id(key))?;
        charge::encode(&value, limits, budget)
    }

    // Only the same-emission constructor supplies an already compared IR period.
    // This does not expose a caller substitution path for the original order.
    pub(super) fn model_step_order_with_period(
        &self,
        limits: Limits,
        admitted_period: Option<&quire_contract_model::CheckedRational>,
    ) -> Result<ModelStepOrder, ModelStepOrderRefusal> {
        let (_, source, _) = selected_time_source(self)?;
        let value = order_value::<_, ModelStepOrderRefusal>(source, admitted_period, |key| {
            Ok(OriginalNode(key))
        })?;
        let bytes =
            quire_canonical::to_vec(&value, limits).map_err(|encoding| ModelStepOrderRefusal {
                code: ModelStepOrderRefusalCode::Encoding,
                encoding: Some(encoding),
            })?;
        Ok(ModelStepOrder { bytes })
    }
}

fn order_value<'a, Node, Error>(
    source: &'a ModelTimeSource,
    admitted_period: Option<&'a quire_contract_model::CheckedRational>,
    mut convert: impl FnMut(&'a NodeKey) -> Result<Node, Error>,
) -> Result<OrderValue<'a, Node>, Error>
where
    Error: From<ModelStepOrderRefusal>,
{
    let kind = match &source.source {
        TimeSourceKind::Dense => SourceEncoding::Dense,
        TimeSourceKind::Tick { period, .. } => SourceEncoding::Tick {
            operation: source
                .tick_operation_anchor
                .as_ref()
                .ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingTickOperation))?,
            period: admitted_period
                .map_or(PeriodEncoding::Kernel(period), PeriodEncoding::Admitted),
        },
    };
    let model = convert(&source.model_node)?;
    let unit = convert(&source.unit_node)?;
    let dimension = convert(&source.time_dimension)?;
    let kind = match kind {
        SourceEncoding::Dense => SourceEncoding::Dense,
        SourceEncoding::Tick { operation, period } => SourceEncoding::Tick {
            operation: convert(operation)?,
            period,
        },
    };
    Ok(OrderValue {
        model,
        unit,
        dimension,
        kind,
    })
}

/// A borrowed encoding of existing checked facts, with no second model carrier.
struct OrderValue<'a, Node> {
    model: Node,
    unit: Node,
    dimension: Node,
    kind: SourceEncoding<'a, Node>,
}

enum SourceEncoding<'a, Node> {
    Dense,
    Tick {
        operation: Node,
        period: PeriodEncoding<'a>,
    },
}

enum PeriodEncoding<'a> {
    Kernel(&'a Rational),
    Admitted(&'a quire_contract_model::CheckedRational),
}

struct OriginalNode<'a>(&'a NodeKey);

impl Encode for OriginalNode<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        structural_id(self.0).encode_into(writer)
    }
}

impl<Node: Encode> Encode for OrderValue<'_, Node> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("kind")?;
        writer.string("model-step")?;
        writer.name("model")?;
        self.model.encode_into(writer)?;
        writer.name("unit")?;
        self.unit.encode_into(writer)?;
        writer.name("dimension")?;
        self.dimension.encode_into(writer)?;
        writer.name("source")?;
        writer.begin_object()?;
        match &self.kind {
            SourceEncoding::Dense => {
                writer.name("kind")?;
                writer.string("dense")?;
            }
            SourceEncoding::Tick { operation, period } => {
                writer.name("kind")?;
                writer.string("tick")?;
                writer.name("operation")?;
                operation.encode_into(writer)?;
                writer.name("period")?;
                writer.begin_object()?;
                writer.name("numerator")?;
                match period {
                    PeriodEncoding::Kernel(value) => {
                        writer.string(&value.numerator().to_string())?
                    }
                    PeriodEncoding::Admitted(value) => writer.string(&value.numerator)?,
                }
                writer.name("denominator")?;
                match period {
                    PeriodEncoding::Kernel(value) => {
                        writer.string(&value.denominator().to_string())?
                    }
                    PeriodEncoding::Admitted(value) => writer.string(&value.denominator)?,
                }
                writer.end_object()?;
            }
        }
        writer.end_object()?;
        writer.end_object()
    }
}
