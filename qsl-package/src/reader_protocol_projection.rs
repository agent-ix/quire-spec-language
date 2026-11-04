// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reader-owned protocol expression nodes projected from admitted I2 terms.

use std::collections::{BTreeMap, BTreeSet};

use quire_checked::node::{
    Arithmetic, CaseArm, CheckedLiteral, Connective, ConnectiveOrigin, NodeKind, OrderedKind,
    RecordField, RecordSlot,
};
use quire_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedTerm};
use quire_exact::{
    admit_text, ArithmeticOperator, Meter, OrderingOperator, ScalarLimits, TextPayload, Timestamp,
    Uuid, Value, ValueType,
};
use quire_semantic_value::declaration::{CheckedEquality, EqualityOperand, EqualityOperator};
use quire_semantic_value::location::{Location, Origin};

/// Evaluator-neutral owned projection from one admitted I2 protocol root.
/// Wire terms and producer rows stay inside qsl-package.
pub struct ReaderProtocolProjection {
    arena: ReaderArena,
    root: usize,
}

impl ReaderProtocolProjection {
    /// Index of the selected root node.
    pub fn root(&self) -> usize {
        self.root
    }

    /// The exact node kind at an arena index.
    pub fn kind(&self, index: usize) -> Option<&NodeKind<usize, CheckedLiteral>> {
        self.arena.nodes.get(index).map(|node| &node.kind)
    }

    /// The selected exact type of an arena node.
    pub fn value_type(&self, index: usize) -> Option<&ValueType> {
        self.arena.nodes.get(index).map(|node| &node.value_type)
    }

    /// The admitted protocol location of an arena node.
    pub fn location(&self, index: usize) -> Option<&Location> {
        self.arena.nodes.get(index).map(|node| &node.location)
    }

    /// Ordered child node indices.
    pub fn children(&self, index: usize) -> Option<&[usize]> {
        self.arena
            .nodes
            .get(index)
            .map(|node| node.children.as_slice())
    }

    /// Function table entry for a checked call.
    pub fn function(&self, index: usize) -> Option<ReaderProtocolFunction<'_>> {
        let function = self.arena.functions.get(index)?;
        Some(ReaderProtocolFunction {
            name: &function.name,
            body: function.body,
            slots: function.slots,
        })
    }
}

/// One admitted pure function available to a projected protocol expression.
pub struct ReaderProtocolFunction<'a> {
    /// Function name used by evaluator diagnostics.
    pub name: &'a str,
    /// Function body node index.
    pub body: usize,
    /// Number of checked parameter slots.
    pub slots: usize,
}

/// Read-only construction of the evaluator's closed operation vocabulary.
pub(crate) fn project_protocol_root<'wire>(
    source_graph: &qsl_semantics::check::CheckedGraph,
    admitted: &'wire CheckedPackageV2,
    emission: &'wire crate::Emission,
    root: &CheckedNodeId,
    locals: &BTreeMap<CheckedNodeId, usize>,
    root_provenance: Location,
    work_limit: u64,
) -> Result<Option<ReaderProtocolProjection>, quire_semantic_value::catalog::InternalFault> {
    let identities = source_graph
        .selected_exact_identities(admitted)
        .map_err(|_| fault("reader-selected-identities-are-admitted"))?;
    let root_node = admitted
        .graph()
        .nodes
        .iter()
        .find(|node| node.node_id == *root)
        .ok_or_else(|| fault("reader-selected-protocol-node-is-retained"))?;

    let mut builder = ReaderBuilder {
        admitted,
        emission,
        identities,
        work_limit,
        nodes: Vec::new(),
        functions: Vec::new(),
        function_indexes: BTreeMap::new(),
        memo: BTreeMap::new(),
    };
    // A CheckedBody location is diagnostic provenance only for a bare literal
    // root. Every composed term and function body must resolve its own
    // protocol_locations row from the admitted reader graph.
    let root_location = builder.location(root).or_else(|| {
        matches!(&root_node.body, CheckedTerm::Literal { .. }).then_some(root_provenance)
    });
    builder.read_functions(root, None)?;
    let context = builder.functions.len();
    let root_index = builder.project_node(root, context, locals, root_location)?;
    Ok(Some(ReaderProtocolProjection {
        arena: ReaderArena {
            nodes: builder.nodes,
            functions: builder.functions,
        },
        root: root_index,
    }))
}

struct ReaderBuilder<'wire> {
    admitted: &'wire CheckedPackageV2,
    emission: &'wire crate::Emission,
    identities: quire_contract_model::SelectedExactIdentities,
    work_limit: u64,
    nodes: Vec<ReaderProjectedNode>,
    functions: Vec<ReaderFunction>,
    function_indexes: BTreeMap<CheckedNodeId, usize>,
    memo: BTreeMap<(usize, CheckedNodeId), usize>,
}

impl<'wire> ReaderBuilder<'wire> {
    fn read_functions(
        &mut self,
        root: &CheckedNodeId,
        fallback_location: Option<Location>,
    ) -> Result<(), quire_semantic_value::catalog::InternalFault> {
        let mut reachable = BTreeSet::new();
        let mut pending = vec![root.clone()];
        while let Some(id) = pending.pop() {
            if !reachable.insert(id.clone()) {
                continue;
            }
            let node = self
                .admitted
                .graph()
                .nodes
                .iter()
                .find(|node| node.node_id == id)
                .ok_or_else(|| fault("reader-reachable-node-is-admitted"))?;
            pending.extend(node.body.reference_targets().into_iter().cloned());
        }
        let declarations = self
            .admitted
            .graph()
            .nodes
            .iter()
            .filter(|node| {
                reachable.contains(&node.node_id)
                    && node.node_tag.as_ref() == "function"
                    && node.semantic_form.as_ref() == "pure_function"
            })
            .map(|node| {
                let (parameters, body) = function_parts(&node.body)
                    .ok_or_else(|| fault("reader-pure-function-has-checked-body"))?;
                let parameters = parameters
                    .iter()
                    .map(|parameter| match parameter {
                        CheckedTerm::Reference { target } => Ok(target.clone()),
                        _ => Err(fault("reader-function-parameter-has-checked-identity")),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let body = match body {
                    CheckedTerm::Reference { target } => target.clone(),
                    _ => return Err(fault("reader-function-body-has-checked-identity")),
                };
                let name = node
                    .declaration
                    .as_ref()
                    .and_then(|declaration| declaration.qualified_name.last())
                    .map(|name| name.to_string())
                    .ok_or_else(|| fault("reader-function-name-is-admitted"))?;
                Ok((node.node_id.clone(), name, parameters, body))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (node, name, parameters, _) in &declarations {
            let index = self.functions.len();
            self.function_indexes.insert(node.clone(), index);
            self.functions.push(ReaderFunction {
                name: name.clone(),
                body: usize::MAX,
                slots: parameters.len(),
            });
        }
        for (index, (_, _, parameters, body)) in declarations.iter().enumerate() {
            let locals = parameters
                .iter()
                .enumerate()
                .map(|(slot, id)| (id.clone(), slot))
                .collect::<BTreeMap<_, _>>();
            let body = self.project_node(body, index, &locals, fallback_location.clone())?;
            self.functions[index].body = body;
        }
        Ok(())
    }

    fn project_node(
        &mut self,
        id: &CheckedNodeId,
        context: usize,
        locals: &BTreeMap<CheckedNodeId, usize>,
        inherited_location: Option<Location>,
    ) -> Result<usize, quire_semantic_value::catalog::InternalFault> {
        if let Some(index) = self.memo.get(&(context, id.clone())) {
            return Ok(*index);
        }
        if let Some(slot) = locals.get(id).copied() {
            let location = self
                .location(id)
                .or(inherited_location)
                .ok_or_else(|| fault("reader-local-location-is-admitted"))?;
            let value_type = self.value_type(id)?;
            let index = self.nodes.len();
            self.nodes.push(ReaderProjectedNode {
                kind: NodeKind::Local(slot),
                value_type,
                location,
                children: Vec::new(),
            });
            self.memo.insert((context, id.clone()), index);
            return Ok(index);
        }
        let node = self
            .admitted
            .graph()
            .nodes
            .iter()
            .find(|node| node.node_id == *id)
            .ok_or_else(|| fault("reader-expression-child-is-admitted"))?;
        let location = self
            .location(id)
            .or(inherited_location)
            .ok_or_else(|| fault("reader-expression-location-is-admitted"))?;
        let value_type = self.value_type(id)?;
        let (kind, children) = match &node.body {
            CheckedTerm::Literal { .. } => {
                let literal = node
                    .body
                    .literal_read()
                    .and_then(|literal| {
                        literal_value(
                            literal.value,
                            literal.type_id,
                            self.admitted,
                            &self.identities,
                            self.work_limit,
                        )
                    })
                    .ok_or_else(|| fault("reader-literal-has-supported-exact-value"))?;
                (NodeKind::Literal(CheckedLiteral(literal)), Vec::new())
            }
            CheckedTerm::Aggregate { members }
                if node.node_tag.as_ref() == "value"
                    && node.semantic_form.as_ref() == "union_value" =>
            {
                let union_key = crate::checked_v2::wire_graph(self.admitted)
                    .map_err(|_| fault("reader-union-value-has-wire-identities"))?
                    .key(&node.semantic_type)
                    .map_err(|_| fault("reader-union-value-has-declaration-key"))?;
                let union_node = self
                    .admitted
                    .graph()
                    .nodes
                    .iter()
                    .find(|candidate| candidate.node_id == node.semantic_type)
                    .filter(|candidate| {
                        candidate.node_tag.as_ref() == "composite_type"
                            && candidate.semantic_form.as_ref() == "union"
                    })
                    .ok_or_else(|| fault("reader-union-value-type-is-union"))?;
                let CheckedTerm::Aggregate {
                    members: union_members,
                } = &union_node.body
                else {
                    return Err(fault("reader-union-declaration-has-checked-members"));
                };
                let [CheckedTerm::Binding { name, value }] = members.as_slice() else {
                    return Err(fault("reader-union-value-has-one-member"));
                };
                let quire_contract_model::CheckedBindingNameKind::Other(member_name) = name.kind()
                else {
                    return Err(fault("reader-union-value-has-member-name"));
                };
                let member = union_members
                    .iter()
                    .position(|candidate| match candidate {
                        CheckedTerm::Binding { name, .. } => {
                            matches!(name.kind(), quire_contract_model::CheckedBindingNameKind::Other(name) if name == member_name)
                        }
                        _ => false,
                    })
                    .ok_or_else(|| fault("reader-union-value-member-is-declared"))?;
                let CheckedTerm::Aggregate { members } = value.as_ref() else {
                    return Err(fault("reader-union-value-has-payload-list"));
                };
                let children = members
                    .iter()
                    .map(|argument| {
                        self.project_argument(argument, context, locals, location.clone())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let kind = NodeKind::UnionValue {
                    declaration: union_key,
                    member,
                    arguments: children.clone(),
                };
                (kind, children)
            }
            CheckedTerm::Aggregate { members } => {
                let ValueType::Composite(declaration) = &value_type else {
                    return Err(fault("reader-aggregate-has-checked-record-type"));
                };
                let slots = members
                    .iter()
                    .map(|member| -> Result<RecordSlot<usize>, _> {
                        let CheckedTerm::Binding { name, value } = member else {
                            return Err(fault("reader-record-value-has-checked-field-binding"));
                        };
                        if !matches!(
                            name.kind(),
                            quire_contract_model::CheckedBindingNameKind::Other(_)
                        ) {
                            return Err(fault("reader-record-value-has-declared-field-names"));
                        }
                        let child = match value.as_ref() {
                            CheckedTerm::Reference { target } => {
                                self.project_node(target, context, locals, Some(location.clone()))?
                            }
                            CheckedTerm::Literal { .. } => {
                                self.project_inline_literal(value, location.clone())?
                            }
                            _ => return Err(fault("reader-record-field-has-admitted-node")),
                        };
                        Ok(RecordSlot::Present(child))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let children = slots
                    .iter()
                    .filter_map(|slot| match slot {
                        RecordSlot::Present(child) => Some(*child),
                        RecordSlot::Absent | RecordSlot::Null => None,
                    })
                    .collect();
                (
                    NodeKind::Record {
                        declaration: declaration.clone(),
                        slots,
                    },
                    children,
                )
            }
            CheckedTerm::Reference { target } => {
                if let Some(slot) = locals.get(target) {
                    (NodeKind::Local(*slot), Vec::new())
                } else {
                    return self.project_node(target, context, locals, Some(location));
                }
            }
            CheckedTerm::Application {
                operation,
                arguments,
                ..
            } => self.project_application(
                id,
                operation,
                arguments,
                context,
                locals,
                location.clone(),
            )?,
            _ => return Err(fault("reader-expression-form-is-supported")),
        };
        let index = self.nodes.len();
        self.nodes.push(ReaderProjectedNode {
            kind,
            value_type,
            location,
            children,
        });
        self.memo.insert((context, id.clone()), index);
        Ok(index)
    }

    fn project_application(
        &mut self,
        id: &CheckedNodeId,
        operation: &quire_contract_model::CheckedApplicationOperation,
        arguments: &[CheckedTerm],
        context: usize,
        locals: &BTreeMap<CheckedNodeId, usize>,
        location: Location,
    ) -> Result<
        (NodeKind<usize, CheckedLiteral>, Vec<usize>),
        quire_semantic_value::catalog::InternalFault,
    > {
        let identity = operation.identity();
        let projected_arguments = if identity == Some("quire.op.function.call") {
            arguments
                .get(1..)
                .ok_or_else(|| fault("reader-call-has-callee"))?
        } else if identity == Some("quire.op.control.case") {
            arguments
                .get(..1)
                .ok_or_else(|| fault("reader-case-has-scrutinee"))?
        } else {
            arguments
        };
        let children = projected_arguments
            .iter()
            .map(|argument| {
                let equality_conversion = matches!(
                    identity,
                    Some("quire.op.quantity.eq" | "quire.op.quantity.ne")
                )
                .then(|| self.quantity_conversion_operand(argument).cloned())
                .flatten();
                if let Some(operand) = equality_conversion {
                    self.project_argument(&operand, context, locals, location.clone())
                } else {
                    self.project_argument(argument, context, locals, location.clone())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        match identity {
            Some("quire.op.function.call") => {
                let callee = arguments
                    .first()
                    .and_then(|argument| match argument {
                        CheckedTerm::Reference { target } => Some(target),
                        _ => None,
                    })
                    .ok_or_else(|| fault("reader-call-has-callee"))?;
                let function = self
                    .function_indexes
                    .get(callee)
                    .copied()
                    .ok_or_else(|| fault("reader-call-callee-is-admitted"))?;
                Ok((
                    NodeKind::Call {
                        function,
                        arguments: children.clone(),
                    },
                    children,
                ))
            }
            Some("quire.op.boolean.and")
            | Some("quire.op.boolean.or")
            | Some("quire.op.boolean.implies") => {
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-connective-has-two-operands"));
                };
                let connective = match identity {
                    Some("quire.op.boolean.and") => Connective::And,
                    Some("quire.op.boolean.or") => Connective::Or,
                    Some("quire.op.boolean.implies") => Connective::Implies,
                    _ => return Err(fault("reader-connective-is-catalogued")),
                };
                Ok((
                    NodeKind::Connective(connective, ConnectiveOrigin::Authored, *left, *right),
                    children,
                ))
            }
            Some("quire.op.boolean.not") => {
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-not-has-one-operand"));
                };
                Ok((NodeKind::Not(*operand), vec![*operand]))
            }
            Some("quire.op.integer.add")
            | Some("quire.op.integer.sub")
            | Some("quire.op.integer.mul") => {
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-integer-arithmetic-has-two-operands"));
                };
                let operator = match identity {
                    Some("quire.op.integer.add") => Arithmetic::Add,
                    Some("quire.op.integer.sub") => Arithmetic::Subtract,
                    Some("quire.op.integer.mul") => Arithmetic::Multiply,
                    _ => return Err(fault("reader-integer-arithmetic-is-catalogued")),
                };
                Ok((NodeKind::Arithmetic(operator, *left, *right), children))
            }
            Some("quire.op.quantity.add")
            | Some("quire.op.quantity.sub")
            | Some("quire.op.quantity.mul")
            | Some("quire.op.quantity.div") => {
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-quantity-arithmetic-has-two-operands"));
                };
                let operator = match identity {
                    Some("quire.op.quantity.add") => ArithmeticOperator::Add,
                    Some("quire.op.quantity.sub") => ArithmeticOperator::Subtract,
                    Some("quire.op.quantity.mul") => ArithmeticOperator::Multiply,
                    Some("quire.op.quantity.div") => ArithmeticOperator::Divide,
                    _ => return Err(fault("reader-quantity-arithmetic-is-catalogued")),
                };
                Ok((NodeKind::Quantity(operator, *left, *right), children))
            }
            Some("quire.op.numeric.narrow") => {
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-narrow-has-one-operand"));
                };
                let result_type = self.value_type(id)?;
                let ValueType::Int(target) = &result_type else {
                    return Err(fault("reader-narrow-has-exact-integer-target"));
                };
                Ok((NodeKind::Coerce(*operand, target.clone()), vec![*operand]))
            }
            Some("quire.op.quantity.convert") => {
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-quantity-conversion-has-one-operand"));
                };
                let operand_id = arguments
                    .first()
                    .and_then(|argument| match argument {
                        CheckedTerm::Reference { target } => Some(target),
                        _ => None,
                    })
                    .ok_or_else(|| fault("reader-quantity-conversion-has-checked-operand"))?;
                let source = self.value_type(operand_id)?;
                let target = self.value_type(id)?;
                if !matches!(source, ValueType::Quantity(_))
                    || !matches!(target, ValueType::Quantity(_))
                {
                    return Err(fault("reader-quantity-conversion-has-exact-units"));
                }
                Ok((
                    NodeKind::ConvertScalar(EqualityOperand::converted(source, target), *operand),
                    vec![*operand],
                ))
            }
            Some("quire.op.control.case") => {
                let [scrutinee] = children.as_slice() else {
                    return Err(fault("reader-union-case-has-scrutinee"));
                };
                let scrutinee_id = arguments
                    .first()
                    .and_then(|argument| match argument {
                        CheckedTerm::Reference { target } => Some(target),
                        _ => None,
                    })
                    .ok_or_else(|| fault("reader-union-case-scrutinee-has-identity"))?;
                let scrutinee_type = self.value_type(scrutinee_id)?;
                let ValueType::Composite(declaration) = &scrutinee_type else {
                    return Err(fault("reader-union-case-scrutinee-has-union-type"));
                };
                let wire_graph = crate::checked_v2::wire_graph(self.admitted)
                    .map_err(|_| fault("reader-union-case-has-wire-identities"))?;
                let union_id = self
                    .admitted
                    .graph()
                    .nodes
                    .iter()
                    .find(|candidate| {
                        candidate.node_tag.as_ref() == "composite_type"
                            && candidate.semantic_form.as_ref() == "union"
                            && wire_graph
                                .key(&candidate.node_id)
                                .is_ok_and(|key| key == *declaration)
                    })
                    .map(|candidate| candidate.node_id.clone())
                    .ok_or_else(|| fault("reader-union-case-declaration-is-admitted"))?;
                let union_node = self
                    .admitted
                    .graph()
                    .nodes
                    .iter()
                    .find(|candidate| candidate.node_id == union_id)
                    .ok_or_else(|| fault("reader-union-case-declaration-is-retained"))?;
                let CheckedTerm::Aggregate {
                    members: union_members,
                } = &union_node.body
                else {
                    return Err(fault("reader-union-declaration-has-checked-members"));
                };
                let mut arms = Vec::with_capacity(arguments.len().saturating_sub(1));
                let mut arm_children = vec![*scrutinee];
                let mut next_slot = locals
                    .values()
                    .copied()
                    .max()
                    .map_or(0, |slot| slot.saturating_add(1));
                for argument in arguments.iter().skip(1) {
                    let CheckedTerm::Binding { name, value } = argument else {
                        return Err(fault("reader-union-case-arm-is-binding"));
                    };
                    let quire_contract_model::CheckedBindingNameKind::Other(member_name) =
                        name.kind()
                    else {
                        return Err(fault("reader-union-case-arm-has-member-name"));
                    };
                    let member = union_members
                        .iter()
                        .position(|candidate| match candidate {
                            CheckedTerm::Binding { name, .. } => {
                                matches!(name.kind(), quire_contract_model::CheckedBindingNameKind::Other(name) if name == member_name)
                            }
                            _ => false,
                        })
                        .ok_or_else(|| fault("reader-union-case-arm-is-declared"))?;
                    let CheckedTerm::Aggregate { members } = value.as_ref() else {
                        return Err(fault("reader-union-case-arm-has-body"));
                    };
                    let [CheckedTerm::Aggregate { members: binders }, body] = members.as_slice()
                    else {
                        return Err(fault("reader-union-case-arm-has-binders-and-body"));
                    };
                    let mut arm_locals = locals.clone();
                    let mut arm_slots = Vec::with_capacity(binders.len());
                    for binder in binders {
                        let CheckedTerm::Reference { target } = binder else {
                            return Err(fault("reader-union-case-binder-has-identity"));
                        };
                        arm_slots.push(next_slot);
                        arm_locals.insert(target.clone(), next_slot);
                        next_slot = next_slot
                            .checked_add(1)
                            .ok_or_else(|| fault("reader-union-case-slot-index-fits"))?;
                    }
                    let body_index =
                        self.project_argument(body, context, &arm_locals, location.clone())?;
                    arm_children.push(body_index);
                    arms.push(CaseArm {
                        member,
                        binders: arm_slots,
                        body: body_index,
                    });
                }
                if let Some(function) = self.functions.get_mut(context) {
                    function.slots = function.slots.max(next_slot);
                }
                Ok((
                    NodeKind::Case {
                        scrutinee: *scrutinee,
                        declaration: declaration.clone(),
                        arms,
                    },
                    arm_children,
                ))
            }
            Some("quire.op.integer.lt")
            | Some("quire.op.integer.le")
            | Some("quire.op.integer.gt")
            | Some("quire.op.integer.ge")
            | Some("quire.op.rational.lt")
            | Some("quire.op.rational.le")
            | Some("quire.op.rational.gt")
            | Some("quire.op.rational.ge")
            | Some("quire.op.decimal.lt")
            | Some("quire.op.decimal.le")
            | Some("quire.op.decimal.gt")
            | Some("quire.op.decimal.ge")
            | Some("quire.op.enum.lt")
            | Some("quire.op.enum.le")
            | Some("quire.op.enum.gt")
            | Some("quire.op.enum.ge")
            | Some("quire.op.text.lt")
            | Some("quire.op.text.le")
            | Some("quire.op.text.gt")
            | Some("quire.op.text.ge")
            | Some("quire.op.quantity.lt")
            | Some("quire.op.quantity.le")
            | Some("quire.op.quantity.gt")
            | Some("quire.op.quantity.ge") => {
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-order-has-two-operands"));
                };
                let family = identity
                    .and_then(|identity| identity.strip_prefix("quire.op."))
                    .and_then(|identity| identity.split_once('.'))
                    .map(|(family, _)| family)
                    .ok_or_else(|| fault("reader-order-family-is-catalogued"))?;
                let kind = match family {
                    "integer" => OrderedKind::Integers,
                    "rational" => OrderedKind::Rationals,
                    "decimal" => OrderedKind::Decimals,
                    "enum" => OrderedKind::Enums,
                    "text" => OrderedKind::Texts,
                    "quantity" => OrderedKind::Quantities,
                    _ => return Err(fault("reader-order-family-is-supported")),
                };
                let operator = match identity.and_then(|identity| identity.rsplit('.').next()) {
                    Some("lt") => OrderingOperator::Less,
                    Some("le") => OrderingOperator::LessOrEqual,
                    Some("gt") => OrderingOperator::Greater,
                    Some("ge") => OrderingOperator::GreaterOrEqual,
                    _ => return Err(fault("reader-order-operator-is-catalogued")),
                };
                Ok((NodeKind::Order(operator, kind, *left, *right), children))
            }
            Some("quire.op.record.project") => {
                let operand_id = arguments
                    .first()
                    .and_then(|argument| match argument {
                        CheckedTerm::Reference { target } => Some(target),
                        _ => None,
                    })
                    .ok_or_else(|| fault("reader-record-projection-has-operand"))?;
                let (field_declaration, field_name) = operation
                    .projection_field()
                    .map_err(|_| fault("reader-record-projection-has-admitted-field"))?
                    .ok_or_else(|| fault("reader-record-projection-has-admitted-field"))?;
                let selected_record_key = self
                    .identities
                    .declaration_rows()
                    .find(|row| row.structural_node == *field_declaration)
                    .and_then(|row| match &row.exact_identity {
                        quire_contract_model::SelectedExactIdentity::Record(key) => {
                            Some(key.clone())
                        }
                        _ => None,
                    });
                let operand_type = self.value_type(operand_id)?;
                let ValueType::Composite(operand_record_key) = &operand_type else {
                    return Err(fault("reader-record-projection-operand-has-composite-type"));
                };
                let record_key = if selected_record_key.as_ref() == Some(operand_record_key) {
                    operand_record_key.clone()
                } else {
                    // The admitted operand type is an exact Composite identity. When the
                    // producer has no separate selected identity row for the field's
                    // declaring node, require its checked record declaration key to match
                    // that exact operand identity; equal field shape is never evidence.
                    let graph = crate::checked_v2::wire_graph(self.admitted)
                        .map_err(|_| fault("reader-record-field-node-has-wire-key"))?;
                    let declaration_key = graph
                        .key(field_declaration)
                        .map_err(|_| fault("reader-record-field-node-has-wire-key"))?;
                    let declaration = self
                        .admitted
                        .graph()
                        .nodes
                        .iter()
                        .find(|candidate| candidate.node_id == *field_declaration)
                        .ok_or_else(|| fault("reader-record-field-node-is-admitted"))?;
                    if !((declaration.node_tag.as_ref() == "model"
                        && declaration.semantic_form.as_ref() == "record_value_type")
                        || (declaration.node_tag.as_ref() == "composite_type"
                            && declaration.semantic_form.as_ref() == "record"))
                        || declaration_key != *operand_record_key
                    {
                        return Err(fault("reader-record-field-exact-type-matches-operand"));
                    }
                    operand_record_key.clone()
                };
                if self.value_type(operand_id)? != ValueType::Composite(record_key.clone()) {
                    return Err(fault("reader-record-projection-identity-matches-operand"));
                }
                let child = *children
                    .first()
                    .ok_or_else(|| fault("reader-record-projection-has-operand"))?;
                Ok((
                    NodeKind::Field {
                        operand: child,
                        field: RecordField::new(record_key, field_name),
                        optional: false,
                    },
                    vec![child],
                ))
            }
            Some("quire.op.string.eq")
            | Some("quire.op.string.ne")
            | Some("quire.op.uuid.eq")
            | Some("quire.op.uuid.ne")
            | Some("quire.op.timestamp.eq")
            | Some("quire.op.timestamp.ne")
            | Some("quire.op.boolean.eq")
            | Some("quire.op.boolean.ne")
            | Some("quire.op.integer.eq")
            | Some("quire.op.integer.ne")
            | Some("quire.op.rational.eq")
            | Some("quire.op.rational.ne")
            | Some("quire.op.decimal.eq")
            | Some("quire.op.decimal.ne")
            | Some("quire.op.quantity.eq")
            | Some("quire.op.quantity.ne")
            | Some("quire.op.structural.eq")
            | Some("quire.op.structural.ne") => {
                let Some(row) = self
                    .emission
                    .equality_producer_identity_rows()
                    .iter()
                    .find(|row| row.checked_node() == id)
                else {
                    return Err(fault("reader-equality-has-producer-identity-row"));
                };
                let decision = self
                    .emission
                    .admitted_equality(self.admitted, id, row.occurrence(), self.work_limit)
                    .map_err(|_| fault("reader-equality-decision-is-admitted"))?;
                let equality = CheckedEquality::from_admitted_decision(&decision);
                let operator = if identity.is_some_and(|identity| identity.ends_with(".ne")) {
                    EqualityOperator::NotEqual
                } else {
                    EqualityOperator::Equal
                };
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-equality-has-two-operands"));
                };
                Ok((
                    NodeKind::Equality(operator, Box::new(equality), *left, *right),
                    children,
                ))
            }
            _ => Err(fault("reader-operation-is-supported")),
        }
    }

    fn project_argument(
        &mut self,
        argument: &CheckedTerm,
        context: usize,
        locals: &BTreeMap<CheckedNodeId, usize>,
        location: Location,
    ) -> Result<usize, quire_semantic_value::catalog::InternalFault> {
        match argument {
            CheckedTerm::Reference { target } => {
                self.project_node(target, context, locals, Some(location))
            }
            CheckedTerm::Literal { .. } => self.project_inline_literal(argument, location),
            _ => Err(fault("reader-expression-operand-has-supported-form")),
        }
    }

    /// The admitted equality decision applies the authored conversion plan to
    /// its operand. Keep the source child here so the equality does not apply
    /// the same quantity conversion twice.
    fn quantity_conversion_operand<'a>(
        &'a self,
        argument: &'a CheckedTerm,
    ) -> Option<&'a CheckedTerm> {
        let CheckedTerm::Reference { target } = argument else {
            return None;
        };
        let conversion = self
            .admitted
            .graph()
            .nodes
            .iter()
            .find(|node| node.node_id == *target)?;
        let CheckedTerm::Application {
            operation,
            arguments,
            ..
        } = &conversion.body
        else {
            return None;
        };
        (operation.identity() == Some("quire.op.quantity.convert"))
            .then(|| arguments.first())
            .flatten()
    }

    fn value_type(
        &self,
        id: &CheckedNodeId,
    ) -> Result<ValueType, quire_semantic_value::catalog::InternalFault> {
        match self
            .admitted
            .project_exact_type(id, &self.identities, self.work_limit)
        {
            Ok(projection) => Ok(projection.value_type().clone()),
            Err(error)
                if matches!(
                    error.code,
                    quire_contract_model::ExactTypeProjectionErrorCode::UnsupportedExactType
                        | quire_contract_model::ExactTypeProjectionErrorCode::UnresolvedSelectedDeclaration
                ) && error.form.as_ref() == "scalar_type/text" =>
            {
                let type_node = error
                    .at
                    .as_deref()
                    .and_then(|at| {
                        self.admitted
                            .graph()
                            .nodes
                            .iter()
                            .find(|node| node.node_id == *at)
                    })
                    .ok_or_else(|| fault("reader-string-type-node-is-admitted"))?;
                if type_node.node_tag.as_ref() != "scalar_type"
                    || type_node.semantic_form.as_ref() != "text"
                {
                    return Err(fault("reader-string-type-has-checked-form"));
                }
                Ok(ValueType::String)
            }
            Err(error)
                if matches!(
                    error.code,
                    quire_contract_model::ExactTypeProjectionErrorCode::UnsupportedExactType
                        | quire_contract_model::ExactTypeProjectionErrorCode::UnresolvedSelectedDeclaration
                ) && error.form.as_ref() == "composite_type/record" =>
            {
                let declaration = error
                    .at
                    .as_deref()
                    .ok_or_else(|| fault("reader-record-type-error-has-node"))?;
                let record_key = self
                    .identities
                    .declaration_rows()
                    .find(|row| row.structural_node == *declaration)
                    .and_then(|row| match &row.exact_identity {
                        quire_contract_model::SelectedExactIdentity::Record(key) => {
                            Some(key.clone())
                        }
                        _ => None,
                    })
                    .or_else(|| {
                        // When this checked wire record type has no separate exact-identity
                        // row, its admitted structural node is the type identity. This path
                        // is restricted to the reader's explicit `composite_type/record`
                        // locus; it never infers identity from equal field shape.
                        let checked_type = self
                            .admitted
                            .graph()
                            .nodes
                            .iter()
                            .find(|node| node.node_id == *declaration)?;
                        (checked_type.node_tag.as_ref() == "composite_type"
                            && checked_type.semantic_form.as_ref() == "record")
                            .then(|| {
                                crate::checked_v2::wire_graph(self.admitted)
                                    .ok()?
                                    .key(declaration)
                                    .ok()
                            })
                            .flatten()
                    })
                    .ok_or_else(|| fault("reader-record-type-has-exact-identity"))?;
                Ok(ValueType::Composite(record_key))
            }
            Err(error)
                if error.code
                    == quire_contract_model::ExactTypeProjectionErrorCode::UnavailableIdentityWitness
                    && error.form.as_ref() == "composite_type/union" =>
            {
                let declaration = error
                    .at
                    .as_deref()
                    .ok_or_else(|| fault("reader-union-type-error-has-node"))?;
                let union_key = self
                    .identities
                    .declaration_rows()
                    .find(|row| row.structural_node == *declaration)
                    .and_then(|row| match &row.exact_identity {
                        quire_contract_model::SelectedExactIdentity::Union(key) => {
                            Some(key.clone())
                        }
                        _ => None,
                    })
                    .or_else(|| {
                        let checked_type = self
                            .admitted
                            .graph()
                            .nodes
                            .iter()
                            .find(|node| node.node_id == *declaration)?;
                        (checked_type.node_tag.as_ref() == "composite_type"
                            && checked_type.semantic_form.as_ref() == "union")
                            .then(|| {
                                crate::checked_v2::wire_graph(self.admitted)
                                    .ok()?
                                    .key(declaration)
                                    .ok()
                            })
                            .flatten()
                    })
                    .ok_or_else(|| fault("reader-union-type-has-exact-identity"))?;
                Ok(ValueType::Composite(union_key))
            }
            Err(error)
                if matches!(
                    error.code,
                    quire_contract_model::ExactTypeProjectionErrorCode::UnsupportedExactType
                        | quire_contract_model::ExactTypeProjectionErrorCode::UnresolvedSelectedDeclaration
                ) && error.form.as_ref() == "scalar_type/unit" =>
            {
                let unit = error
                    .at
                    .as_deref()
                    .and_then(|declaration| {
                        self.identities
                            .declaration_rows()
                            .find(|row| row.structural_node == *declaration)
                    })
                    .and_then(|row| match &row.exact_identity {
                        quire_contract_model::SelectedExactIdentity::DeclaredUnit(unit) => {
                            Some(unit.clone())
                        }
                        _ => None,
                    })
                    .or_else(|| {
                        // A checked `scalar_type/unit` node is itself the exact
                        // declaration identity when no selected identity row
                        // exists. Its structural key is package-qualified.
                        let declaration = error.at.as_deref()?;
                        let unit_node = self
                            .admitted
                            .graph()
                            .nodes
                            .iter()
                            .find(|candidate| candidate.node_id == *declaration)?;
                        (unit_node.node_tag.as_ref() == "scalar_type"
                            && unit_node.semantic_form.as_ref() == "unit")
                            .then(|| {
                                crate::checked_v2::wire_graph(self.admitted)
                                    .ok()?
                                    .key(declaration)
                                    .ok()
                            })
                            .flatten()
                            .map(quire_exact::UnitId::declared)
                    })
                    .ok_or_else(|| fault("reader-unit-has-exact-identity"))?;
                Ok(ValueType::Quantity(unit))
            }
            Err(error)
                if matches!(
                    error.code,
                    quire_contract_model::ExactTypeProjectionErrorCode::UnsupportedExactType
                        | quire_contract_model::ExactTypeProjectionErrorCode::UnresolvedSelectedDeclaration
                ) && error.form.as_ref() == "scalar_type/compound_unit" =>
            {
                let unit_node = error
                    .at
                    .as_deref()
                    .ok_or_else(|| fault("reader-compound-unit-type-has-node"))?;
                let unit = crate::checked_v2::compound_unit(self.admitted, unit_node)
                    .ok_or_else(|| fault("reader-compound-unit-has-exact-identity"))?;
                Ok(ValueType::Quantity(unit))
            }
            Err(_) => Err(fault("reader-expression-type-is-admitted")),
        }
    }

    fn location(&self, id: &CheckedNodeId) -> Option<Location> {
        let row = self
            .emission
            .protocol_location_rows()
            .iter()
            .find(|row| row.application == *id)?;
        let admitted = self
            .admitted
            .protocol_location(
                &row.application,
                &row.occurrence,
                row.term_ordinal,
                self.work_limit,
            )
            .ok()?;
        let origin = match &admitted.location.origin {
            quire_contract_model::CheckedProtocolLocationOrigin::TypeDeclaration { name } => {
                Origin::TypeDeclaration {
                    name: name.to_string(),
                }
            }
            quire_contract_model::CheckedProtocolLocationOrigin::ProtocolAttempt {
                protocol,
                attempt,
            } => Origin::ProtocolAttempt {
                protocol: usize::try_from(*protocol).ok()?,
                attempt: usize::try_from(*attempt).ok()?,
            },
        };
        let path = admitted
            .location
            .path
            .iter()
            .map(|ordinal| match ordinal {
                quire_contract_model::CheckedKeyOrdinal::Safe(value) => {
                    usize::try_from(*value).ok()
                }
                quire_contract_model::CheckedKeyOrdinal::BeyondSafe(value) => {
                    value.parse::<usize>().ok()
                }
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Location::at(origin, &path))
    }

    fn project_inline_literal(
        &mut self,
        term: &CheckedTerm,
        location: Location,
    ) -> Result<usize, quire_semantic_value::catalog::InternalFault> {
        let literal = term
            .literal_read()
            .ok_or_else(|| fault("reader-inline-literal-is-typed"))?;
        let value = literal_value(
            literal.value,
            literal.type_id,
            self.admitted,
            &self.identities,
            self.work_limit,
        )
        .ok_or_else(|| fault("reader-inline-literal-has-exact-value"))?;
        let value_type = match self.admitted.project_exact_type(
            literal.type_id,
            &self.identities,
            self.work_limit,
        ) {
            Ok(projection) => projection.value_type().clone(),
            Err(error) if error.form.as_ref() == "scalar_type/text" => ValueType::String,
            _ => return Err(fault("reader-inline-literal-type-is-admitted")),
        };
        let index = self.nodes.len();
        self.nodes.push(ReaderProjectedNode {
            kind: NodeKind::Literal(CheckedLiteral(value)),
            value_type,
            location,
            children: Vec::new(),
        });
        Ok(index)
    }
}

fn function_parts(term: &CheckedTerm) -> Option<(&[CheckedTerm], &CheckedTerm)> {
    let CheckedTerm::Aggregate { members } = term else {
        return None;
    };
    let mut parameters = None;
    let mut body = None;
    for member in members {
        let CheckedTerm::Binding { name, value } = member else {
            continue;
        };
        match name.kind() {
            quire_contract_model::CheckedBindingNameKind::Parameters => {
                let CheckedTerm::Aggregate { members } = value.as_ref() else {
                    return None;
                };
                parameters = Some(members.as_slice());
            }
            quire_contract_model::CheckedBindingNameKind::Body => body = Some(value.as_ref()),
            _ => {}
        }
    }
    Some((parameters?, body?))
}

fn literal_value(
    value: quire_contract_model::CheckedLiteralValue<'_>,
    type_id: &CheckedNodeId,
    admitted: &CheckedPackageV2,
    identities: &quire_contract_model::SelectedExactIdentities,
    work_limit: u64,
) -> Option<Value> {
    use quire_contract_model::CheckedLiteralValue as Literal;
    Some(match value {
        Literal::Boolean(value) => Value::Boolean(value),
        Literal::Integer(value) => Value::Integer(value.to_string().parse().ok()?),
        Literal::Rational {
            numerator,
            denominator,
        } => {
            let magnitude = quire_exact::Rational::new(
                numerator.to_string().parse().ok()?,
                denominator.to_string().parse().ok()?,
            )
            .ok()?;
            let literal_type = project_literal_type(type_id, admitted, identities, work_limit)?;
            match &literal_type {
                ValueType::Quantity(unit) => {
                    Value::Quantity(quire_exact::Quantity::new(magnitude, unit.clone()))
                }
                ValueType::Rational(_) => Value::Rational(magnitude),
                _ => return None,
            }
        }
        Literal::String(value) => Value::String(value.into()),
        Literal::Text(value) => {
            let checked_type = admitted
                .graph()
                .nodes
                .iter()
                .find(|node| node.node_id == *type_id)?;
            if checked_type.node_tag.as_ref() == "scalar_type"
                && checked_type.semantic_form.as_ref() == "text"
            {
                return Some(Value::String(value.into()));
            }
            let type_projection = admitted.project_exact_type(type_id, identities, work_limit);
            let projection = type_projection.ok()?;
            let ValueType::Text(text_type) = projection.value_type() else {
                return None;
            };
            let payload = TextPayload::from_utf8(value.as_bytes()).ok()?;
            let mut meter = Meter::new(ScalarLimits {
                integer_bits: u64::MAX,
                decimal_digits: u64::MAX,
                scale_expansion: u64::MAX,
                text_input_bytes: u64::MAX,
                text_scalars: u64::MAX,
                normalized_scalars: u64::MAX,
                unit_edges: u64::MAX,
                value_occurrences: u64::MAX,
                work_units: u64::MAX,
                result_units: u64::MAX,
            });
            Value::Text(admit_text(&payload, text_type, &mut meter).completed()?)
        }
        Literal::Uuid(value) => Value::Uuid(Uuid::from_canonical_text(value).ok()?),
        Literal::Timestamp(value) => {
            Value::Timestamp(Timestamp::from_canonical_text(&value.to_string()).ok()?)
        }
        _ => return None,
    })
}

fn project_literal_type(
    type_id: &CheckedNodeId,
    admitted: &CheckedPackageV2,
    identities: &quire_contract_model::SelectedExactIdentities,
    work_limit: u64,
) -> Option<ValueType> {
    match admitted.project_exact_type(type_id, identities, work_limit) {
        Ok(projection) => Some(projection.value_type().clone()),
        Err(error) => {
            if !matches!(
                error.code,
                quire_contract_model::ExactTypeProjectionErrorCode::UnsupportedExactType
                    | quire_contract_model::ExactTypeProjectionErrorCode::UnresolvedSelectedDeclaration
            ) || !matches!(error.form.as_ref(), "scalar_type/unit" | "scalar_type/dimension") {
                return None;
            }
            let declaration = type_id;
            let unit_node = admitted
                .graph()
                .nodes
                .iter()
                .find(|candidate| candidate.node_id == *declaration)?;
            if unit_node.node_tag.as_ref() != "scalar_type"
                || unit_node.semantic_form.as_ref() != "unit"
            {
                return None;
            }
            let unit = identities
                .declaration_rows()
                .find(|row| row.structural_node == *declaration)
                .and_then(|row| match &row.exact_identity {
                    quire_contract_model::SelectedExactIdentity::DeclaredUnit(unit) => {
                        Some(unit.clone())
                    }
                    _ => None,
                })
                .or_else(|| {
                    crate::checked_v2::wire_graph(admitted)
                        .ok()?
                        .key(declaration)
                        .ok()
                        .map(quire_exact::UnitId::declared)
                })?;
            Some(ValueType::Quantity(unit))
        }
    }
}

struct ReaderArena {
    nodes: Vec<ReaderProjectedNode>,
    functions: Vec<ReaderFunction>,
}

struct ReaderProjectedNode {
    kind: NodeKind<usize, CheckedLiteral>,
    value_type: ValueType,
    location: Location,
    children: Vec<usize>,
}

struct ReaderFunction {
    name: String,
    body: usize,
    slots: usize,
}

fn fault(invariant: &'static str) -> quire_semantic_value::catalog::InternalFault {
    quire_semantic_value::catalog::InternalFault::new("FR-390", invariant)
}
