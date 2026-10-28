// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-390's checked protocol-slot selection and caller-supplied inputs.

use quire_eval::Evaluation;
use quire_exact::{NodeKey, Value};
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::location::Location;

use qsl_semantics::model::observation::Observation;

/// Select one expression member of an emitted protocol node (FR-390).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolSlot {
    /// The selected protocol node's identity.
    pub node: NodeKey,
    /// The exact member name that holds the expression.
    pub member: String,
    /// The list position for a list-valued member, absent for a single slot.
    pub index: Option<usize>,
}

/// One caller-supplied binder value or a source the caller cannot provide.
#[derive(Clone, Debug)]
pub enum SuppliedValue {
    /// The binder's supplied runtime value.
    Value(Value),
    /// The binder exists, but the caller's protocol state has no value for it.
    Unavailable(CatalogCode),
}

/// The values supplied for one protocol-slot evaluation (FR-390).
#[derive(Clone, Debug)]
pub struct ProtocolSupply<'a> {
    /// Binder identities and values. A sequence preserves duplicates so
    /// admission can report them before checking for missing binders.
    pub binders: Vec<(NodeKey, SuppliedValue)>,
    /// The current admitted observation, required only when a value names an
    /// object reference.
    pub observation: Option<&'a Observation>,
}

/// The result of evaluating a checked protocol slot (FR-390).
#[derive(Debug)]
pub enum ProtocolSlotEvaluation {
    /// The shared S6a evaluator's result.
    Evaluated(Evaluation),
    /// A supplied binder was unavailable and the expression read it.
    Unavailable {
        /// The binder that was unavailable.
        binder: NodeKey,
        /// The caller-provided reason it was unavailable.
        cause: CatalogCode,
        /// The checked expression read that encountered the binder.
        location: Location,
    },
}
