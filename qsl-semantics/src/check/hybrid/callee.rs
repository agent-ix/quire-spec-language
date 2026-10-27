// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447 "Invariants, guards, resets and expression forms": a call of a pure
//! function (FR-062) whose parameters and result are `Boolean` or numeric
//! and whose body takes the hybrid expression forms.

use std::collections::BTreeSet;

use qsl_forms::{BinaryOperator, ExprId, ExprNode, Expression};
use qsl_foundation::Span;
use quire_exact::ValueType;

use super::term::{Kind, Lowered};
use super::{CheckedFunctionRef, HybridCause};
use crate::check::Signatures;

/// The package's functions a hybrid term may call: their signatures and the
/// bodies those signatures were checked for, index-aligned.
pub(in crate::check) struct Callees<'a> {
    pub(in crate::check) signatures: &'a Signatures,
    pub(in crate::check) bodies: &'a [Expression],
}

/// The kind a value of `declared` has, or `None` for a type that is neither
/// `Boolean` nor numeric.
pub(super) fn kind_of(declared: &ValueType) -> Option<Kind> {
    match declared {
        ValueType::Boolean => Some(Kind::Boolean),
        ValueType::Integer
        | ValueType::Int(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_) => Some(Kind::Real),
        ValueType::Quantity(_)
        | ValueType::BoundedQuantity(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Map(_)
        | ValueType::Reference(_)
        | ValueType::Population(_) => None,
    }
}

impl Callees<'_> {
    /// The function `name` a call at `span` applies to `arguments`, and the
    /// kind of its result, or why the call is not a hybrid term.
    pub(super) fn resolve(
        &self,
        name: &str,
        arguments: &[Lowered],
        span: Span,
    ) -> Result<(CheckedFunctionRef, Kind), HybridCause> {
        let Some((index, signature)) = self.signatures.callable(name) else {
            return Err(HybridCause::MissingName {
                name: name.to_owned(),
                span,
            });
        };
        let mismatch = HybridCause::TypeMismatch { span };
        if signature.parameters.len() != arguments.len() {
            return Err(mismatch);
        }
        for ((_, declared), argument) in signature.parameters.iter().zip(arguments) {
            // A real argument is passed as the real it is; an `Integer` or
            // `Int` parameter takes none (QSpec FR-193).
            let wanted = match declared {
                ValueType::Integer | ValueType::Int(_) => return Err(mismatch),
                other => kind_of(other).ok_or(mismatch.clone())?,
            };
            if wanted != argument.kind {
                return Err(mismatch);
            }
        }
        let result = kind_of(&signature.result).ok_or(mismatch)?;
        if let Some(form) = self.first_foreign_form(index) {
            return Err(HybridCause::CalleeForm { form, span });
        }
        Ok((CheckedFunctionRef::new(index), result))
    }

    /// The first form, in a body of the function `start` or of a function it
    /// calls, that the hybrid expression forms do not admit.
    fn first_foreign_form(&self, start: usize) -> Option<&'static str> {
        let mut visited = BTreeSet::from([start]);
        let mut functions = vec![start];
        while let Some(function) = functions.pop() {
            let body = self.bodies.get(function)?;
            let mut pending: Vec<ExprId> = vec![body.root_id()];
            while let Some(id) = pending.pop() {
                let node = body.at(id).node();
                match node {
                    ExprNode::Boolean(_)
                    | ExprNode::Integer(_)
                    | ExprNode::Rational(..)
                    | ExprNode::Let { .. }
                    | ExprNode::If { .. }
                    | ExprNode::Negate(_)
                    | ExprNode::Not(_) => {}
                    ExprNode::Name(name) => {
                        if name.is_qualified() {
                            return Some("a qualified name");
                        }
                    }
                    ExprNode::Binary { operator, .. } => match operator {
                        BinaryOperator::IntegerDiv => return Some("div"),
                        BinaryOperator::IntegerRem => return Some("rem"),
                        BinaryOperator::IntegerMod => return Some("mod"),
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
                        | BinaryOperator::Implies => {}
                    },
                    ExprNode::Call { name, .. } => {
                        let Some((callee, _)) = self.signatures.callable(&name.name) else {
                            return Some("a call of a function not in scope");
                        };
                        if visited.insert(callee) {
                            functions.push(callee);
                        }
                    }
                    other => return Some(form_name(other)),
                }
                pending.extend(node.children());
            }
        }
        None
    }
}

/// The name a refusal gives a form the hybrid expression forms do not admit.
fn form_name(node: &ExprNode) -> &'static str {
    match node {
        ExprNode::String(_) => "a string literal",
        ExprNode::Quantity(_) => "a quantity literal",
        ExprNode::Field { .. } => "a field read",
        ExprNode::Record { .. } => "a record literal",
        ExprNode::Collection { .. } => "a collection literal",
        ExprNode::Convert { .. } => "a conversion",
        ExprNode::Query { .. }
        | ExprNode::Flatten(_)
        | ExprNode::Accumulate { .. }
        | ExprNode::Count { .. }
        | ExprNode::Sum { .. }
        | ExprNode::Population { .. }
        | ExprNode::Seq { .. }
        | ExprNode::Size(_)
        | ExprNode::Contains { .. } => "a collection operation",
        ExprNode::Present(_) | ExprNode::Value(_) => "an option operation",
        ExprNode::Pre(_) => "`pre`",
        _ => "an expression form outside the hybrid expression forms",
    }
}
