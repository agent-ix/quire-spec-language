// SPDX-License-Identifier: AGPL-3.0-only
//! FR-033: fallible borrowed views of the published primitive IR wire contract.

use super::{failure, LoweringCode, Result};
use crate::checking::ClauseBinding;
use crate::formal_source::FormalSource;
use quire_contract_ir as ir;
use serde::{ser::SerializeMap, Serialize, Serializer};

#[derive(Serialize)]
pub(super) struct Projection<'a> {
    format: &'static str,
    package: &'a ir::ContractPackage<ir::ReferenceBody>,
    bindings: Vec<Binding<'a>>,
}

impl<'a> Projection<'a> {
    pub fn new(
        package: &'a ir::ContractPackage<ir::ReferenceBody>,
        prepared: &'a [super::Prepared<'a>],
        source: &FormalSource,
    ) -> Result<Self> {
        let bindings = prepared
            .iter()
            .map(|item| {
                let conversion = Conversion {
                    binding: item.binding,
                    source,
                };
                Ok(Binding {
                    clause: super::identity(item.binding),
                    expression: Input {
                        owner: item.environment.owner(),
                        types: [],
                        values: item
                            .environment
                            .values()
                            .iter()
                            .map(|value| conversion.declaration(value))
                            .collect::<Result<_>>()?,
                        functions: [],
                        expression: conversion.expression(&item.expression)?,
                        expected_type: ir::ValueType::Boolean,
                        execution_point: &item.binding.execution_point,
                        clause_root: true,
                    },
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            format: ir::EXECUTABLE_PROJECTION_FORMAT,
            package,
            bindings,
        })
    }
}

#[derive(Serialize)]
struct Binding<'a> {
    clause: ir::ClauseRef,
    expression: Input<'a>,
}

#[derive(Serialize)]
struct Input<'a> {
    owner: &'a ir::RequirementRef,
    types: [(); 0],
    values: Vec<ValueDeclaration<'a>>,
    functions: [(); 0],
    expression: Expression<'a>,
    expected_type: ir::ValueType,
    execution_point: &'a ir::ExecutionPoint,
    clause_root: bool,
}

#[derive(Serialize)]
struct ValueDeclaration<'a> {
    name: &'a ir::SymbolName,
    kind: ir::ValueDeclarationKind,
    value_type: ValueType<'a>,
    source: &'a ir::SourceSpan,
}

enum ValueType<'a> {
    Boolean,
    Integer(&'a ir::IntegerType),
}

impl Serialize for ValueType<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Boolean => ir::ValueType::Boolean.serialize(serializer),
            Self::Integer(value) => IntegerType(value).serialize(serializer),
        }
    }
}

// IR's public integer type has a nested value; its input wire uses flattened bounds.
struct IntegerType<'a>(&'a ir::IntegerType);

impl Serialize for IntegerType<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("kind", "integer")?;
        map.serialize_entry("domain", &self.0.domain())?;
        map.serialize_entry("minimum", &self.0.minimum())?;
        map.serialize_entry("maximum", &self.0.maximum())?;
        map.serialize_entry("overflow", &self.0.overflow())?;
        map.end()
    }
}

#[derive(Serialize)]
struct Expression<'a> {
    source: &'a ir::SourceSpan,
    #[serde(flatten)]
    kind: ExpressionKind<'a>,
}

#[derive(Serialize)]
#[serde(tag = "node", rename_all = "snake_case")]
enum ExpressionKind<'a> {
    BooleanLiteral {
        value: bool,
    },
    IntegerLiteral {
        value: i64,
        value_type: IntegerType<'a>,
    },
    ValueReference {
        name: &'a ir::SymbolName,
        observation: ir::StateObservation,
    },
    BooleanNot {
        operand: Box<Expression<'a>>,
    },
    NumericNegate {
        operand: Box<Expression<'a>>,
    },
    Numeric {
        operator: ir::NumericOperator,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
    },
    Compare {
        operator: ir::ComparisonOperator,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
    },
    Boolean {
        operator: ir::BooleanOperator,
        left: Box<Expression<'a>>,
        right: Box<Expression<'a>>,
    },
}

struct Conversion<'a> {
    binding: &'a ClauseBinding,
    source: &'a FormalSource,
}

impl Conversion<'_> {
    fn unsupported(&self, source: &ir::SourceSpan, message: &str) -> Box<super::LoweringError> {
        match self.source.to_native(source) {
            Ok(span) => failure(
                LoweringCode::Unsupported,
                Some(self.binding),
                Some(span),
                message,
            ),
            Err(_) => failure(
                LoweringCode::InvalidCorrespondence,
                Some(self.binding),
                None,
                "prepared wire source differs from native correspondence",
            ),
        }
    }

    fn declaration<'a>(&self, value: &'a ir::ValueDeclaration) -> Result<ValueDeclaration<'a>> {
        let value_type = match value.value_type() {
            ir::ValueType::Boolean => ValueType::Boolean,
            ir::ValueType::Integer { value } => ValueType::Integer(value),
            _ => {
                return Err(self.unsupported(
                    value.source(),
                    "unadmitted primitive declaration at the wire boundary",
                ))
            }
        };
        Ok(ValueDeclaration {
            name: value.name(),
            kind: value.kind(),
            value_type,
            source: value.source(),
        })
    }

    // Prepared expressions come from the bounded native traversal (10,000 nodes,
    // depth 64). These views borrow their data and add no new semantic authority.
    fn expression<'a>(&self, value: &'a ir::Expression) -> Result<Expression<'a>> {
        let kind = match value.kind() {
            ir::ExpressionKind::BooleanLiteral { value } => {
                ExpressionKind::BooleanLiteral { value: *value }
            }
            ir::ExpressionKind::IntegerLiteral { value, value_type } => {
                ExpressionKind::IntegerLiteral {
                    value: *value,
                    value_type: IntegerType(value_type),
                }
            }
            ir::ExpressionKind::ValueReference { name, observation } => {
                ExpressionKind::ValueReference {
                    name,
                    observation: *observation,
                }
            }
            ir::ExpressionKind::BooleanNot { operand } => ExpressionKind::BooleanNot {
                operand: Box::new(self.expression(operand)?),
            },
            ir::ExpressionKind::NumericNegate { operand } => ExpressionKind::NumericNegate {
                operand: Box::new(self.expression(operand)?),
            },
            ir::ExpressionKind::Numeric {
                operator,
                left,
                right,
            } => ExpressionKind::Numeric {
                operator: *operator,
                left: Box::new(self.expression(left)?),
                right: Box::new(self.expression(right)?),
            },
            ir::ExpressionKind::Compare {
                operator,
                left,
                right,
            } => ExpressionKind::Compare {
                operator: *operator,
                left: Box::new(self.expression(left)?),
                right: Box::new(self.expression(right)?),
            },
            ir::ExpressionKind::Boolean {
                operator,
                left,
                right,
            } => ExpressionKind::Boolean {
                operator: *operator,
                left: Box::new(self.expression(left)?),
                right: Box::new(self.expression(right)?),
            },
            _ => {
                return Err(self.unsupported(
                    value.source(),
                    "unadmitted expression at the primitive wire boundary",
                ))
            }
        };
        Ok(Expression {
            source: value.source(),
            kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Source, SourceIdentity, Span};
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-111", "FR-033-AC-3")]
    fn unadmitted_prepared_wire_values_keep_clause_and_source_context() {
        let source = FormalSource::new(
            Source::read(
                SourceIdentity {
                    identity: "test:wire".into(),
                    revision: "1".into(),
                },
                "rule.native",
                b"true",
                64,
            )
            .unwrap(),
            ir::SourceIdentity::new(
                ir::SourceDocumentId::new("WireSource").unwrap(),
                ir::SourceRevision::new(1).unwrap(),
            ),
        );
        let binding = ClauseBinding {
            name: "Rule".into(),
            requirement: ir::RequirementRef::parse("example/wire", "Rule", 1).unwrap(),
            clause: ir::ClauseId::new("rule").unwrap(),
            execution_point: ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new("validate").unwrap(),
            },
        };
        let span = source
            .to_ir(source.source(), Span { start: 0, end: 4 })
            .unwrap();
        let conversion = Conversion {
            binding: &binding,
            source: &source,
        };
        let declaration = ir::ValueDeclaration::new(
            ir::SymbolName::new("text").unwrap(),
            ir::ValueDeclarationKind::State,
            ir::ValueType::Text,
            span.clone(),
        );
        let Err(error) = conversion.declaration(&declaration) else {
            panic!("text declaration must not reach serialization");
        };
        assert_eq!(error.code, LoweringCode::Unsupported);
        assert_eq!(error.clause, Some(super::super::identity(&binding)));
        assert_eq!(error.source, Some(Span { start: 0, end: 4 }));

        let text = ir::Expression::new(
            ir::ExpressionKind::TextLiteral {
                value: "bad".into(),
            },
            span.clone(),
        );
        let nested = ir::Expression::new(
            ir::ExpressionKind::BooleanNot {
                operand: Box::new(text),
            },
            span.clone(),
        );
        let Err(error) = conversion.expression(&nested) else {
            panic!("unadmitted nested expression must not reach serialization");
        };
        assert_eq!(error.code, LoweringCode::Unsupported);
        assert_eq!(error.clause, Some(super::super::identity(&binding)));
        assert_eq!(error.source, Some(Span { start: 0, end: 4 }));

        let valid = ir::Expression::new(ir::ExpressionKind::BooleanLiteral { value: true }, span);
        let wire = serde_json::to_value(conversion.expression(&valid).unwrap()).unwrap();
        assert_eq!(wire["node"], "boolean_literal");
        assert_eq!(wire["value"], true);
    }
}
