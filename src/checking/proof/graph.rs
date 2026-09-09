// SPDX-License-Identifier: AGPL-3.0-only
//! FR-016: bounded materialization into actual IR expressions.

use super::*;

impl<'u, 'a> Builder<'u, 'a> {
    pub(super) fn materialize(
        &mut self,
        id: GraphId,
        depth: usize,
        count: &mut usize,
    ) -> Result<ir::Expression> {
        let at = self.graph[id.0].native;
        let span = self.span(at);
        self.meter.materialize(count, depth, span)?;
        let source = self.source(at)?;
        let kind = match &self.graph[id.0].kind {
            Kind::Boolean(value) => ir::ExpressionKind::BooleanLiteral { value: *value },
            Kind::Integer(value, ty) => ir::ExpressionKind::IntegerLiteral {
                value: *value,
                value_type: (*ty).clone(),
            },
            Kind::Input(key) => ir::ExpressionKind::ValueReference {
                name: self.key_info[key.0].symbol.clone().ok_or_else(|| {
                    failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        span,
                        "proof input has no symbol",
                    )
                })?,
                observation: ir::StateObservation::Current,
            },
            Kind::Present(value) => {
                let value = *value;
                ir::ExpressionKind::IsPresent {
                    option: Box::new(self.materialize(value, depth + 1, count)?),
                }
            }
            Kind::Unwrap(value) => {
                let value = *value;
                ir::ExpressionKind::Unwrap {
                    option: Box::new(self.materialize(value, depth + 1, count)?),
                }
            }
            Kind::Negate(value) => {
                let value = *value;
                ir::ExpressionKind::NumericNegate {
                    operand: Box::new(self.materialize(value, depth + 1, count)?),
                }
            }
            Kind::Not(value) => {
                let value = *value;
                ir::ExpressionKind::BooleanNot {
                    operand: Box::new(self.materialize(value, depth + 1, count)?),
                }
            }
            Kind::Numeric(operator, left, right) => {
                let (operator, left, right) = (*operator, *left, *right);
                ir::ExpressionKind::Numeric {
                    operator,
                    left: Box::new(self.materialize(left, depth + 1, count)?),
                    right: Box::new(self.materialize(right, depth + 1, count)?),
                }
            }
            Kind::Compare(operator, left, right) => {
                let (operator, left, right) = (*operator, *left, *right);
                ir::ExpressionKind::Compare {
                    operator,
                    left: Box::new(self.materialize(left, depth + 1, count)?),
                    right: Box::new(self.materialize(right, depth + 1, count)?),
                }
            }
            Kind::BooleanOp(operator, left, right) => {
                let (operator, left, right) = (*operator, *left, *right);
                ir::ExpressionKind::Boolean {
                    operator,
                    left: Box::new(self.materialize(left, depth + 1, count)?),
                    right: Box::new(self.materialize(right, depth + 1, count)?),
                }
            }
            Kind::Witness(value, ty) => {
                let (value, ty) = (*value, ty.clone());
                self.meter.materialize(count, depth + 1, span)?;
                let item = self.materialize(value, depth + 2, count)?;
                let collection_type = ir::CollectionType::new(ty, 1).map_err(|upstream| {
                    let mut error = failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        span,
                        "proof witness type is invalid",
                    );
                    error.upstream = Some(Box::new(upstream));
                    error
                })?;
                let collection = ir::Expression::new(
                    ir::ExpressionKind::CollectionLiteral {
                        value_type: collection_type,
                        items: vec![item],
                    },
                    source.clone(),
                );
                self.meter.materialize(count, depth + 1, span)?;
                let predicate = ir::Expression::new(
                    ir::ExpressionKind::BooleanLiteral { value: true },
                    source.clone(),
                );
                let local = ir::SymbolName::new("witness").map_err(|upstream| {
                    let mut error = failure(
                        self.meter.source,
                        Code::InvalidModelBinding,
                        span,
                        "proof witness symbol is invalid",
                    );
                    error.upstream = Some(Box::new(upstream));
                    error
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
}
