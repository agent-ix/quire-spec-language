// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-016/040: one bounded materializer for historical and composed proof graphs.

use super::*;

/// A source/charge adapter; neither implementation supplies executable semantics.
pub(in crate::checking) trait Context<'a> {
    type Error;
    fn graph_node(&self, id: GraphId) -> &Node<'a>;
    fn symbol(&self, key: ValueKey) -> Option<&ir::SymbolName>;
    fn source_span(&mut self, at: ExprId) -> std::result::Result<ir::SourceSpan, Self::Error>;
    fn materialized(
        &mut self,
        count: &mut usize,
        depth: usize,
        at: ExprId,
    ) -> std::result::Result<(), Self::Error>;
    fn prepare(&mut self, id: GraphId) -> std::result::Result<(), Self::Error>;
    fn invalid(&self, at: ExprId, message: &str, upstream: Option<ir::Diagnostic>) -> Self::Error;
}

pub(in crate::checking) fn materialize<'a, C: Context<'a>>(
    context: &mut C,
    id: GraphId,
    depth: usize,
    count: &mut usize,
) -> std::result::Result<ir::Expression, C::Error> {
    let at = context.graph_node(id).native;
    context.materialized(count, depth, at)?;
    context.prepare(id)?;
    let source = context.source_span(at)?;
    let kind = match context.graph_node(id).kind.clone() {
        Kind::Boolean(value) => ir::ExpressionKind::BooleanLiteral { value },
        Kind::Integer(value, ty) => ir::ExpressionKind::IntegerLiteral {
            value,
            value_type: ty.clone(),
        },
        Kind::Rational(numerator, denominator, ty) => ir::ExpressionKind::RationalLiteral {
            numerator,
            denominator,
            value_type: ty.clone(),
        },
        Kind::Input(key) => ir::ExpressionKind::ValueReference {
            name: context
                .symbol(key)
                .cloned()
                .ok_or_else(|| context.invalid(at, "proof input has no symbol", None))?,
            observation: ir::StateObservation::Current,
        },
        Kind::Present(value) => ir::ExpressionKind::IsPresent {
            option: Box::new(materialize(context, value, depth + 1, count)?),
        },
        Kind::Unwrap(value) => ir::ExpressionKind::Unwrap {
            option: Box::new(materialize(context, value, depth + 1, count)?),
        },
        Kind::Negate(value) => ir::ExpressionKind::NumericNegate {
            operand: Box::new(materialize(context, value, depth + 1, count)?),
        },
        Kind::Not(value) => ir::ExpressionKind::BooleanNot {
            operand: Box::new(materialize(context, value, depth + 1, count)?),
        },
        Kind::Numeric(operator, left, right) => ir::ExpressionKind::Numeric {
            operator,
            left: Box::new(materialize(context, left, depth + 1, count)?),
            right: Box::new(materialize(context, right, depth + 1, count)?),
        },
        Kind::Compare(operator, left, right) => ir::ExpressionKind::Compare {
            operator,
            left: Box::new(materialize(context, left, depth + 1, count)?),
            right: Box::new(materialize(context, right, depth + 1, count)?),
        },
        Kind::BooleanOp(operator, left, right) => ir::ExpressionKind::Boolean {
            operator,
            left: Box::new(materialize(context, left, depth + 1, count)?),
            right: Box::new(materialize(context, right, depth + 1, count)?),
        },
        Kind::Witness(value, ty) => {
            context.materialized(count, depth + 1, at)?;
            let item = materialize(context, value, depth + 2, count)?;
            let collection_type = ir::CollectionType::new(ty, 1).map_err(|error| {
                context.invalid(at, "proof witness type is invalid", Some(error))
            })?;
            let collection = ir::Expression::new(
                ir::ExpressionKind::CollectionLiteral {
                    value_type: collection_type,
                    items: vec![item],
                },
                source.clone(),
            );
            context.materialized(count, depth + 1, at)?;
            let predicate = ir::Expression::new(
                ir::ExpressionKind::BooleanLiteral { value: true },
                source.clone(),
            );
            let local = ir::SymbolName::new("witness").map_err(|error| {
                context.invalid(at, "proof witness symbol is invalid", Some(error))
            })?;
            ir::ExpressionKind::Quantifier {
                quantifier: ir::QuantifierKind::ForAll,
                domain: ir::QuantifierDomain::Elements,
                collection: Box::new(collection),
                local,
                local_source: source.clone(),
                predicate: Box::new(predicate),
            }
        }
    };
    Ok(ir::Expression::new(kind, source))
}

impl<'u, 'a> Context<'a> for Builder<'u, 'a> {
    type Error = Box<crate::Diagnostic>;
    fn graph_node(&self, id: GraphId) -> &Node<'a> {
        &self.graph[id.0]
    }
    fn symbol(&self, key: ValueKey) -> Option<&ir::SymbolName> {
        self.key_info[key.0].symbol.as_ref()
    }
    fn source_span(&mut self, at: ExprId) -> Result<ir::SourceSpan> {
        self.source(at)
    }
    fn materialized(&mut self, count: &mut usize, depth: usize, at: ExprId) -> Result<()> {
        self.meter.materialize(count, depth, self.span(at))
    }
    fn prepare(&mut self, _id: GraphId) -> Result<()> {
        Ok(())
    }
    fn invalid(&self, at: ExprId, message: &str, upstream: Option<ir::Diagnostic>) -> Self::Error {
        let message = match upstream {
            Some(upstream) => format!("{message}: {upstream}"),
            None => message.to_owned(),
        };
        failure(self.meter.source, Code::InvalidModelBinding, self.span(at), message)
    }
}
impl Builder<'_, '_> {
    pub(super) fn materialize(
        &mut self,
        id: GraphId,
        depth: usize,
        count: &mut usize,
    ) -> Result<ir::Expression> {
        materialize(self, id, depth, count)
    }
}
