// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251: canonical model-step order values from the original checked owner.

use qsl_semantics::check::wire_id::structural_id;
use qsl_semantics::check::{ModelTimeSource, TemporalClauseNode, TimeSourceKind};
use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::NodeKey;

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

fn refuse(code: ModelStepOrderRefusalCode) -> ModelStepOrderRefusal {
    ModelStepOrderRefusal {
        code,
        encoding: None,
    }
}

fn selected_clause(root: &PlanRoot) -> Option<&TemporalClauseNode> {
    root.checked_unit()
        .package()
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| {
            node.identity() == *root.plan.subject().clause_node()
                && node.claim() == root.plan.subject().occurrence().origin()
        })
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
        let clause = selected_clause(self)
            .ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingClause))?;
        let binding = clause
            .clause()
            .time_binding()
            .ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingModelTime))?;
        let mut sources = self
            .checked_unit()
            .package()
            .graph()
            .time_sources()
            .iter()
            .filter(|source| {
                source.model == binding.model
                    && source.source == binding.source
                    && source.unit == binding.unit
            });
        let source = sources
            .next()
            .ok_or_else(|| refuse(ModelStepOrderRefusalCode::MissingTimeSource))?;
        if sources.next().is_some() {
            return Err(refuse(ModelStepOrderRefusalCode::MissingTimeSource));
        }
        if matches!(source.source, TimeSourceKind::Tick { .. })
            && source.tick_operation_anchor.is_none()
        {
            return Err(refuse(ModelStepOrderRefusalCode::MissingTickOperation));
        }
        let bytes = quire_canonical::to_vec(&OrderValue(source), limits).map_err(|encoding| {
            ModelStepOrderRefusal {
                code: ModelStepOrderRefusalCode::Encoding,
                encoding: Some(encoding),
            }
        })?;
        Ok(ModelStepOrder { bytes })
    }
}

/// A borrowed encoding of existing checked facts, with no second model carrier.
struct OrderValue<'a>(&'a ModelTimeSource);

fn node<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, key: &NodeKey) -> Result<(), Error> {
    structural_id(key).encode_into(writer)
}

impl Encode for OrderValue<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let source = self.0;
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
        match &source.source {
            TimeSourceKind::Dense => {
                writer.name("kind")?;
                writer.string("dense")?;
            }
            TimeSourceKind::Tick { period, .. } => {
                writer.name("kind")?;
                writer.string("tick")?;
                writer.name("operation")?;
                // The selected source was checked for this anchor before encoding.
                if let Some(operation) = &source.tick_operation_anchor {
                    node(writer, operation)?;
                } else {
                    return Err(Error::Protocol(
                        quire_canonical::ProtocolViolation::Incomplete,
                    ));
                }
                writer.name("period")?;
                writer.begin_object()?;
                writer.name("numerator")?;
                writer.string(&period.numerator().to_string())?;
                writer.name("denominator")?;
                writer.string(&period.denominator().to_string())?;
                writer.end_object()?;
            }
        }
        writer.end_object()?;
        writer.end_object()
    }
}
