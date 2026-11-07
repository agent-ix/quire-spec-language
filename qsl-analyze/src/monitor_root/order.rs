// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251: canonical model-step order values from the original checked owner.

use qsl_semantics::check::wire_id::structural_id;
use qsl_semantics::check::{ModelTimeSource, TemporalClauseNode, TimeSourceKind};
use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::{NodeKey, Rational};

use super::PlanRoot;

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

pub(super) fn selected_clause(root: &PlanRoot) -> Option<&TemporalClauseNode> {
    root.checked_unit()
        .package()
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| {
            node.identity() == *root.plan().subject().clause_node()
                && node.claim() == root.plan().subject().occurrence().origin()
        })
}

/// Resolve the original clause and exactly its checked model time source.
/// This selection is shared by order encoding and progress-field comparison.
pub(super) fn selected_time_source(
    root: &PlanRoot,
) -> Result<(&TemporalClauseNode, &ModelTimeSource, usize), ModelStepOrderRefusal> {
    let clause =
        selected_clause(root).ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingClause))?;
    let binding = clause
        .clause()
        .time_binding()
        .ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingModelTime))?;
    let graph = root.checked_unit().package().graph();
    let model = graph
        .timed_model(&binding.model)
        .ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingModelTime))?;
    let mut sources = graph
        .time_sources()
        .iter()
        .enumerate()
        .filter(|(_, source)| {
            model.is_timed_by(source)
                && source.source == binding.source
                && source.unit == binding.unit
        });
    let (slot, source) = sources
        .next()
        .ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingTimeSource))?;
    if sources.next().is_some() {
        return Err(refuse(ModelStepOrderRefusalCode::MissingTimeSource));
    }
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

    // Only the same-emission constructor supplies an already compared IR period.
    // This does not expose a caller substitution path for the original order.
    pub(super) fn model_step_order_with_period(
        &self,
        limits: Limits,
        admitted_period: Option<&quire_contract_model::CheckedRational>,
    ) -> Result<ModelStepOrder, ModelStepOrderRefusal> {
        let (_, source, _) = selected_time_source(self)?;
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
        let bytes =
            quire_canonical::to_vec(&OrderValue { source, kind }, limits).map_err(|encoding| {
                ModelStepOrderRefusal {
                    code: ModelStepOrderRefusalCode::Encoding,
                    encoding: Some(encoding),
                }
            })?;
        Ok(ModelStepOrder { bytes })
    }
}

/// A borrowed encoding of existing checked facts, with no second model carrier.
struct OrderValue<'a> {
    source: &'a ModelTimeSource,
    kind: SourceEncoding<'a>,
}

enum SourceEncoding<'a> {
    Dense,
    Tick {
        operation: &'a NodeKey,
        period: PeriodEncoding<'a>,
    },
}

enum PeriodEncoding<'a> {
    Kernel(&'a Rational),
    Admitted(&'a quire_contract_model::CheckedRational),
}

fn node<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, key: &NodeKey) -> Result<(), Error> {
    structural_id(key).encode_into(writer)
}

impl Encode for OrderValue<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let source = self.source;
        writer.begin_object()?;
        writer.name("kind")?;
        writer.string("model-step")?;
        writer.name("model")?;
        node(writer, &source.model_node)?;
        writer.name("unit")?;
        node(writer, &source.unit_node)?;
        writer.name("dimension")?;
        node(writer, &source.time_dimension)?;
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
                node(writer, operation)?;
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
