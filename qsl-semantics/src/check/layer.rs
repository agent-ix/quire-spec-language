// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-110 "Layer selection": S3 admits a declaration whose `using` alias
//! names a header profile resolved to a layer under exactly the
//! admitted-form set of that layer's `requires` closure
//! ([`DefinitionLock::admitted_forms`]), and refuses a form outside it with
//! `unsupported_construct` at the form's span, naming the layer.
//!
//! The classification of the unit language's forms is [`expression_form`]:
//! one exhaustive `match` over [`ExprNode`] with no wildcard arm, so a new
//! expression form does not compile until it is classified.

use std::collections::BTreeSet;

use qsl_forms::{
    BinaryOperator, BinderQuery, DeclarationKind, ExprNode, Expression, ExpressionSpans,
    FunctionDeclaration, StateClauseForm,
};
use qsl_foundation::Span;

use super::assemble::{AssemblyCause, AssemblyError};
use crate::value::{DefinitionLock, HeaderLayer, LayerForm};

/// The admitted-form set one `using` alias selects, with the names of the
/// unit's own predicates, which tell a named predicate call from any other
/// call.
pub(crate) struct LayerAdmission<'a> {
    layer: HeaderLayer,
    admitted: BTreeSet<LayerForm>,
    predicates: &'a BTreeSet<&'a str>,
}

impl<'a> LayerAdmission<'a> {
    /// The admission under `layer`'s `requires` closure.
    pub(crate) fn new(
        lock: &DefinitionLock,
        layer: HeaderLayer,
        predicates: &'a BTreeSet<&'a str>,
    ) -> Self {
        Self {
            layer,
            admitted: lock.admitted_forms(layer),
            predicates,
        }
    }

    /// The refusal of the first form of `function` outside the set, if any:
    /// the declaration form at the declaration's span, else the first
    /// expression form of the body or `decreases` measure, in source order,
    /// at its span.
    pub(crate) fn function(&self, function: &FunctionDeclaration) -> Option<AssemblyError> {
        let declaration = match function.kind() {
            DeclarationKind::Function => LayerForm::FunctionDeclaration,
            DeclarationKind::Predicate => LayerForm::PredicateDeclaration,
        };
        let spans = function.spans();
        let whole = spans.map(|spans| spans.declaration).or_else(|| {
            // A declaration read from source carries its spans; the alias
            // is the nearest located part of one built by hand.
            function.using().map(|using| using.span)
        })?;
        if let Some(refusal) = self.declaration(declaration, whole) {
            return Some(refusal);
        }
        let spans = spans?;
        self.expression(&function.body, &spans.body, whole)
            .or_else(|| {
                let (measure, measure_spans) =
                    function.measure.as_ref().zip(spans.measure.as_ref())?;
                self.expression(measure, measure_spans, whole)
            })
    }

    /// The refusal of the first form of a state clause outside the set, if
    /// any, as [`Self::function`] gives it.
    pub(crate) fn state_clause(&self, clause: &StateClauseForm) -> Option<AssemblyError> {
        let whole = clause.spans.declaration;
        self.declaration(LayerForm::StateClause, whole)
            .or_else(|| self.expression(&clause.body, &clause.spans.body, whole))
    }

    fn declaration(&self, form: LayerForm, span: Span) -> Option<AssemblyError> {
        (!self.admitted.contains(&form)).then(|| self.refusal(form, span))
    }

    fn expression(
        &self,
        body: &Expression,
        spans: &ExpressionSpans,
        whole: Span,
    ) -> Option<AssemblyError> {
        let mut pending = vec![(body.root(), Some(spans.root()))];
        while let Some((node, located)) = pending.pop() {
            if let Some(form) = expression_form(node.node(), self.predicates) {
                if !self.admitted.contains(&form) {
                    let span = located.and_then(|id| spans.span(id)).unwrap_or(whole);
                    return Some(self.refusal(form, span));
                }
            }
            // Source order: the first child is popped first.
            for (index, child) in node.children().into_iter().enumerate().rev() {
                pending.push((child, located.and_then(|id| spans.child(id, index))));
            }
        }
        None
    }

    fn refusal(&self, form: LayerForm, span: Span) -> AssemblyError {
        AssemblyError {
            cause: AssemblyCause::UnsupportedLayerForm {
                layer: self.layer,
                form,
            },
            span,
        }
    }
}

/// The form an expression node is, where a layer's admitted-form set
/// decides it. `None` is a form every layer admits (a state-core scalar,
/// presence or control form) or one another requirement gates: FR-361 owns
/// the model constructs (a dispatched call, `lookup`, `allInstances`) and the
/// protocol clause family owns a protocol reference.
#[qsl_attrs::string_edge]
fn expression_form(node: &ExprNode, predicates: &BTreeSet<&str>) -> Option<LayerForm> {
    match node {
        ExprNode::Call { name, .. } => Some(if predicates.contains(name.as_str()) {
            LayerForm::NamedPredicateCall
        } else {
            LayerForm::ValueConstruct
        }),
        ExprNode::Binary { operator, .. } => match operator {
            BinaryOperator::IntegerDiv
            | BinaryOperator::IntegerRem
            | BinaryOperator::IntegerMod => Some(LayerForm::ValueConstruct),
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::LessOrEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterOrEqual
            | BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Implies => None,
        },
        ExprNode::Query { query, .. } => match query {
            BinderQuery::Map | BinderQuery::Filter => Some(LayerForm::SequenceQuery),
            BinderQuery::FlatMap => Some(LayerForm::ValueConstruct),
            BinderQuery::Forall | BinderQuery::Exists => None,
        },
        ExprNode::Count { .. } | ExprNode::Sum { .. } => Some(LayerForm::SequenceQuery),
        ExprNode::Deref(_) | ExprNode::Reaches { .. } => Some(LayerForm::ObjectNavigation),
        ExprNode::Case(_)
        | ExprNode::Record { .. }
        | ExprNode::Collection { .. }
        | ExprNode::Convert { .. }
        | ExprNode::Flatten(_)
        | ExprNode::Accumulate { .. } => Some(LayerForm::ValueConstruct),
        ExprNode::Boolean(_)
        | ExprNode::Integer(_)
        | ExprNode::Rational(..)
        | ExprNode::String(_)
        | ExprNode::Name(_)
        | ExprNode::Let { .. }
        | ExprNode::If { .. }
        | ExprNode::Negate(_)
        | ExprNode::Not(_)
        | ExprNode::Field { .. }
        | ExprNode::Present(_)
        | ExprNode::Value(_)
        | ExprNode::Size(_)
        | ExprNode::Contains { .. }
        | ExprNode::Pre(_)
        | ExprNode::SelfRef
        | ExprNode::Result
        | ExprNode::AllInstances { .. }
        | ExprNode::Lookup { .. }
        | ExprNode::Dispatch { .. }
        | ExprNode::ProtocolReference { .. } => None,
        #[cfg(seam_probe)]
        ExprNode::__SeamProbe => None,
    }
}
