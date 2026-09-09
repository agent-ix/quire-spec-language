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
                        values: item.environment.values(),
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
    // All admitted values are Boolean, whose public serialization is the wire form.
    values: &'a [ir::ValueDeclaration],
    functions: [(); 0],
    expression: Expression<'a>,
    expected_type: ir::ValueType,
    execution_point: &'a ir::ExecutionPoint,
    clause_root: bool,
}

struct Expression<'a>(&'a ir::Expression);

impl Serialize for Expression<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("source", self.0.source())?;
        match self.0.kind() {
            ir::ExpressionKind::BooleanLiteral { value } => {
                map.serialize_entry("node", "boolean_literal")?;
                map.serialize_entry("value", value)?;
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
                    "unqualified expression reached Boolean wire serialization",
                ))
            }
        }
        map.end()
    }
}
