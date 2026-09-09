// SPDX-License-Identifier: AGPL-3.0-only
//! Borrowed serialization views for the published IR wire shape, not a second AST.

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
    ) -> Self {
        Self {
            format: ir::EXECUTABLE_PROJECTION_FORMAT,
            package,
            bindings: prepared
                .iter()
                .map(|item| Binding {
                    clause: super::identity(item.binding),
                    expression: Input {
                        owner: item.environment.owner(),
                        types: [],
                        values: item
                            .environment
                            .values()
                            .iter()
                            .map(ValueDeclaration)
                            .collect(),
                        functions: [],
                        expression: Expression(&item.expression),
                        expected_type: ir::ValueType::Boolean,
                        execution_point: &item.binding.execution_point,
                        clause_root: true,
                    },
                })
                .collect(),
        }
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

struct Expression<'a>(&'a ir::Expression);

struct ValueDeclaration<'a>(&'a ir::ValueDeclaration);

impl Serialize for ValueDeclaration<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("name", self.0.name())?;
        map.serialize_entry("kind", &self.0.kind())?;
        map.serialize_entry("value_type", &ValueType(self.0.value_type()))?;
        map.serialize_entry("source", self.0.source())?;
        map.end()
    }
}

struct ValueType<'a>(&'a ir::ValueType);

impl Serialize for ValueType<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            ir::ValueType::Boolean => self.0.serialize(serializer),
            ir::ValueType::Integer { value } => IntegerType(value).serialize(serializer),
            _ => Err(serde::ser::Error::custom("unadmitted primitive wire type")),
        }
    }
}

// IR's public integer value type serializes with a nested value; its input wire
// contract instead uses these flattened constructor fields (FR-033).
struct IntegerType<'a>(&'a ir::IntegerType);

impl Serialize for IntegerType<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("kind", "integer")?;
        map.serialize_entry("domain", &self.0.domain())?;
        map.serialize_entry("minimum", &self.0.minimum())?;
        map.serialize_entry("maximum", &self.0.maximum())?;
        map.serialize_entry("overflow", &self.0.overflow())?;
        map.end()
    }
}

impl Serialize for Expression<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("source", self.0.source())?;
        match self.0.kind() {
            ir::ExpressionKind::BooleanLiteral { value } => {
                map.serialize_entry("node", "boolean_literal")?;
                map.serialize_entry("value", value)?;
            }
            ir::ExpressionKind::IntegerLiteral { value, value_type } => {
                map.serialize_entry("node", "integer_literal")?;
                map.serialize_entry("value", value)?;
                map.serialize_entry("value_type", &IntegerType(value_type))?;
            }
            ir::ExpressionKind::ValueReference { name, observation } => {
                map.serialize_entry("node", "value_reference")?;
                map.serialize_entry("name", name)?;
                map.serialize_entry("observation", observation)?;
            }
            ir::ExpressionKind::BooleanNot { operand } => {
                map.serialize_entry("node", "boolean_not")?;
                map.serialize_entry("operand", &Expression(operand))?;
            }
            ir::ExpressionKind::NumericNegate { operand } => {
                map.serialize_entry("node", "numeric_negate")?;
                map.serialize_entry("operand", &Expression(operand))?;
            }
            ir::ExpressionKind::Numeric {
                operator,
                left,
                right,
            } => {
                map.serialize_entry("node", "numeric")?;
                map.serialize_entry("operator", operator)?;
                map.serialize_entry("left", &Expression(left))?;
                map.serialize_entry("right", &Expression(right))?;
            }
            ir::ExpressionKind::Compare {
                operator,
                left,
                right,
            } => {
                map.serialize_entry("node", "compare")?;
                map.serialize_entry("operator", operator)?;
                map.serialize_entry("left", &Expression(left))?;
                map.serialize_entry("right", &Expression(right))?;
            }
            ir::ExpressionKind::Boolean {
                operator,
                left,
                right,
            } => {
                map.serialize_entry("node", "boolean")?;
                map.serialize_entry("operator", operator)?;
                map.serialize_entry("left", &Expression(left))?;
                map.serialize_entry("right", &Expression(right))?;
            }
            _ => {
                return Err(serde::ser::Error::custom(
                    "unadmitted expression reached primitive wire serialization",
                ))
            }
        }
        map.end()
    }
}
