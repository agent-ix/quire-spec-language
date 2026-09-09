// SPDX-License-Identifier: AGPL-3.0-only
//! FR-016: union-find contextual types, lexical availability and runtime obligations.

use std::collections::{BTreeMap, VecDeque};

use quire_contract_ir as ir;

use super::inputs::Populations;
use super::types::Catalog;
use super::{
    failure, CheckLimits, CheckUsage, NativeType, Observation, Result, RuntimeRequirements,
};
use crate::linking::{DeclarationKey, DeclarationLocation, LinkedPackage, ResolutionTarget};
use crate::native_model::{NativeModel, ObjectRole, OperationRole, ScalarKind, ScalarSite};
use crate::syntax::{BinaryOp, Builtin, ClauseKind, ExprId, ExprKind, UnaryOp};
use crate::{Code, ParsedUnit, Span};

#[derive(Debug)]
pub(super) struct NodeType<'a> {
    pub expression: ExprId,
    pub ty: NativeType<'a>,
    pub observation: Observation,
}

pub(super) struct TypedClause<'a> {
    pub nodes: Vec<NodeType<'a>>,
    pub runtime: RuntimeRequirements<'a>,
}

struct Variables<'a> {
    parents: Vec<usize>,
    ranks: Vec<u8>,
    known: Vec<Option<NativeType<'a>>>,
}

impl<'a> Variables<'a> {
    fn new(count: usize) -> Self {
        Self {
            parents: (0..count).collect(),
            ranks: vec![0; count],
            known: vec![None; count],
        }
    }
    fn fresh(&mut self) -> usize {
        let id = self.parents.len();
        self.parents.push(id);
        self.ranks.push(0);
        self.known.push(None);
        id
    }
    fn root(&mut self, mut var: usize) -> usize {
        while self.parents[var] != var {
            self.parents[var] = self.parents[self.parents[var]];
            var = self.parents[var];
        }
        var
    }
    fn get(&mut self, var: usize) -> Option<NativeType<'a>> {
        let root = self.root(var);
        self.known[root].clone()
    }
    fn assign(&mut self, var: usize, ty: NativeType<'a>) -> std::result::Result<bool, ()> {
        let root = self.root(var);
        match &self.known[root] {
            Some(prior) if prior != &ty => Err(()),
            Some(_) => Ok(false),
            None => {
                self.known[root] = Some(ty);
                Ok(true)
            }
        }
    }
    fn unify(&mut self, left: usize, right: usize) -> std::result::Result<(), ()> {
        let mut a = self.root(left);
        let mut b = self.root(right);
        if a == b {
            return Ok(());
        }
        if matches!((&self.known[a], &self.known[b]), (Some(a), Some(b)) if a != b) {
            return Err(());
        }
        if self.ranks[a] < self.ranks[b] {
            std::mem::swap(&mut a, &mut b);
        }
        self.parents[b] = a;
        if self.ranks[a] == self.ranks[b] {
            self.ranks[a] += 1;
        }
        if self.known[a].is_none() {
            self.known[a] = self.known[b].take();
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Relation {
    Option {
        input: usize,
        output: Option<usize>,
        at: ExprId,
    },
    Element {
        input: usize,
        output: usize,
        at: ExprId,
    },
    Deref {
        input: usize,
        output: usize,
        at: ExprId,
    },
}

impl Relation {
    fn input(self) -> usize {
        match self {
            Self::Option { input, .. }
            | Self::Element { input, .. }
            | Self::Deref { input, .. } => input,
        }
    }
    fn at(self) -> ExprId {
        match self {
            Self::Option { at, .. } | Self::Element { at, .. } | Self::Deref { at, .. } => at,
        }
    }
}

#[derive(Clone, Copy)]
struct Local<'u> {
    name: &'u str,
    var: usize,
    observation: Observation,
}

struct Solver<'u, 'a> {
    unit: &'u ParsedUnit,
    catalogs: &'u [Catalog<'a>],
    current: usize,
    kind: ClauseKind,
    context: &'a ObjectRole,
    operation: Option<&'a OperationRole>,
    occurrences: BTreeMap<usize, &'u DeclarationLocation>,
    variables: Variables<'a>,
    observations: Vec<Option<Observation>>,
    locals: Vec<Local<'u>>,
    relations: Vec<Relation>,
    visited: Vec<ExprId>,
    limits: CheckLimits,
    max_depth: usize,
}

impl<'u, 'a> Solver<'u, 'a> {
    fn invalid(&self, id: ExprId, message: &str) -> Box<crate::Diagnostic> {
        failure(
            self.unit.source(),
            Code::IllTyped,
            self.unit.expressions()[id.0].span,
            message,
        )
    }
    fn unify(&mut self, a: usize, b: usize, at: ExprId) -> Result<()> {
        self.variables
            .unify(a, b)
            .map_err(|()| self.invalid(at, "native operands require the same exact type"))
    }
    fn assign(&mut self, var: usize, ty: NativeType<'a>, at: ExprId) -> Result<()> {
        self.variables
            .assign(var, ty)
            .map(|_| ())
            .map_err(|()| self.invalid(at, "native expression has incompatible contextual types"))
    }
    fn boolean(&mut self, id: ExprId) -> Result<()> {
        self.assign(id.0, NativeType::Boolean, id)
    }
    fn catalog_for(&self, location: &DeclarationLocation) -> Result<&'u Catalog<'a>> {
        self.catalogs
            .iter()
            .find(|catalog| catalog.model.environment().owner() == &location.identity.owner)
            .ok_or_else(|| {
                failure(
                    self.unit.source(),
                    Code::InvalidModelBinding,
                    Span { start: 0, end: 0 },
                    "linked declaration has no selected native model",
                )
            })
    }
    fn selected(&self, id: ExprId) -> Result<&'u DeclarationLocation> {
        self.occurrences
            .get(&id.0)
            .copied()
            .ok_or_else(|| self.invalid(id, "native expression has no linked declaration"))
    }
    fn declared_value(
        &mut self,
        id: ExprId,
        observed: ir::StateObservation,
    ) -> Result<Observation> {
        let selected = self.selected(id)?;
        let catalog = self.catalog_for(selected)?;
        let DeclarationKey::Value(name) = &selected.identity.key else {
            return Err(self.invalid(id, "expected a linked value declaration"));
        };
        let value = catalog
            .values
            .get(name)
            .copied()
            .ok_or_else(|| self.invalid(id, "linked value is absent"))?;
        let ty = catalog
            .formal(
                value.value_type(),
                &ScalarSite::Value { name: name.clone() },
            )
            .ok_or_else(|| self.invalid(id, "linked value has no native type"))?;
        self.assign(id.0, ty, id)?;
        Ok(Observation::Snapshot(
            if value.kind() == ir::ValueDeclarationKind::Input {
                if matches!(self.unit.expressions()[id.0].kind, ExprKind::ResultValue) {
                    ir::StateObservation::Post
                } else {
                    ir::StateObservation::Pre
                }
            } else {
                observed
            },
        ))
    }
    fn enter_local(
        &mut self,
        name: &'u str,
        var: usize,
        observation: Observation,
        at: ExprId,
    ) -> Result<()> {
        if self.locals.iter().any(|local| local.name == name) {
            return Err(self.invalid(at, "active native local names cannot shadow"));
        }
        self.locals.push(Local {
            name,
            var,
            observation,
        });
        Ok(())
    }

    fn visit(
        &mut self,
        id: ExprId,
        depth: usize,
        observed: ir::StateObservation,
    ) -> Result<Observation> {
        let expression = &self.unit.expressions()[id.0];
        if depth > self.limits.depth {
            return Err(failure(
                self.unit.source(),
                Code::ResourceExhausted,
                expression.span,
                "native checking depth limit exceeded",
            ));
        }
        self.max_depth = self.max_depth.max(depth);
        self.visited.push(id);
        let next = depth + 1;
        let origin = match &expression.kind {
            ExprKind::Group { inner } => {
                let origin = self.visit(*inner, next, observed)?;
                self.unify(id.0, inner.0, id)?;
                origin
            }
            ExprKind::Boolean(_) => {
                self.boolean(id)?;
                Observation::Independent
            }
            ExprKind::Integer(_) | ExprKind::Text(_) => Observation::Independent,
            ExprKind::SelfValue => {
                self.assign(
                    id.0,
                    NativeType::Object {
                        model: self.catalogs[self.current].model,
                        role: self.context,
                    },
                    id,
                )?;
                Observation::Snapshot(observed)
            }
            ExprKind::ResultValue => {
                if self.kind != ClauseKind::Postcondition {
                    return Err(failure(
                        self.unit.source(),
                        Code::WrongSnapshot,
                        expression.span,
                        "operation result is available only at post",
                    ));
                }
                self.declared_value(id, observed)?
            }
            ExprKind::Name(name) => {
                if let Some(local) = self
                    .locals
                    .iter()
                    .rev()
                    .find(|local| local.name == name.value)
                    .copied()
                {
                    self.unify(id.0, local.var, id)?;
                    local.observation
                } else {
                    self.declared_value(id, observed)?
                }
            }
            ExprKind::EnumValue { .. } => {
                let selected = self.selected(id)?;
                let catalog = self.catalog_for(selected)?;
                let DeclarationKey::Variant { enumeration, .. } = &selected.identity.key else {
                    return Err(self.invalid(id, "expected a linked enumeration variant"));
                };
                let declaration = catalog
                    .enumerations
                    .get(enumeration)
                    .copied()
                    .ok_or_else(|| self.invalid(id, "linked enumeration is absent"))?;
                self.assign(
                    id.0,
                    NativeType::Enumeration {
                        model: catalog.model,
                        declaration,
                    },
                    id,
                )?;
                Observation::Independent
            }
            ExprKind::Field { base, .. } => {
                let origin = self.visit(*base, next, observed)?;
                let selected = self.selected(id)?;
                let catalog = self.catalog_for(selected)?;
                let DeclarationKey::Field { record, field } = &selected.identity.key else {
                    return Err(self.invalid(id, "expected a linked field declaration"));
                };
                let receiver = catalog
                    .record_type(record)
                    .ok_or_else(|| self.invalid(id, "linked field receiver is absent"))?;
                self.assign(base.0, receiver, *base)?;
                let field_decl = catalog
                    .records
                    .get(record)
                    .and_then(|r| r.fields().iter().find(|f| f.name() == field))
                    .ok_or_else(|| self.invalid(id, "linked field is absent"))?;
                let ty = catalog
                    .formal(
                        field_decl.value_type(),
                        &ScalarSite::Field {
                            record: record.clone(),
                            field: field.clone(),
                        },
                    )
                    .ok_or_else(|| self.invalid(id, "linked field has no native type"))?;
                self.assign(id.0, ty, id)?;
                origin
            }
            ExprKind::Unary { op, argument } => {
                self.visit(*argument, next, observed)?;
                match op {
                    UnaryOp::Not => {
                        self.boolean(*argument)?;
                        self.boolean(id)?;
                    }
                    UnaryOp::Negate => self.unify(id.0, argument.0, id)?,
                }
                Observation::Independent
            }
            ExprKind::Binary { op, left, right } => {
                self.visit(*left, next, observed)?;
                self.visit(*right, next, observed)?;
                match op {
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Implies => {
                        self.boolean(*left)?;
                        self.boolean(*right)?;
                        self.boolean(id)?;
                    }
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        self.unify(left.0, right.0, id)?;
                        self.boolean(id)?;
                    }
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => {
                        self.unify(left.0, right.0, id)?;
                        self.unify(id.0, left.0, id)?;
                    }
                }
                Observation::Independent
            }
            ExprKind::Call { builtin, argument } => {
                if *builtin == Builtin::Pre && self.kind != ClauseKind::Postcondition {
                    return Err(failure(
                        self.unit.source(),
                        Code::WrongSnapshot,
                        expression.span,
                        "pre is available only in a postcondition",
                    ));
                }
                let scope = if *builtin == Builtin::Pre {
                    ir::StateObservation::Pre
                } else {
                    observed
                };
                let origin = self.visit(*argument, next, scope)?;
                match builtin {
                    Builtin::Pre => {
                        self.unify(id.0, argument.0, id)?;
                        origin
                    }
                    Builtin::Present => {
                        self.relations.push(Relation::Option {
                            input: argument.0,
                            output: None,
                            at: id,
                        });
                        self.boolean(id)?;
                        Observation::Independent
                    }
                    Builtin::Value => {
                        self.relations.push(Relation::Option {
                            input: argument.0,
                            output: Some(id.0),
                            at: id,
                        });
                        origin
                    }
                    Builtin::Deref => {
                        self.relations.push(Relation::Deref {
                            input: argument.0,
                            output: id.0,
                            at: id,
                        });
                        origin
                    }
                    Builtin::Size => Observation::Independent,
                }
            }
            ExprKind::Let { name, value, body } => {
                let captured = self.visit(*value, next, observed)?;
                self.enter_local(&name.value, value.0, captured, id)?;
                let origin = self.visit(*body, next, observed)?;
                self.locals.pop();
                self.unify(id.0, body.0, id)?;
                origin
            }
            ExprKind::If {
                condition,
                then_value,
                else_value,
            } => {
                self.visit(*condition, next, observed)?;
                self.boolean(*condition)?;
                let then_origin = self.visit(*then_value, next, observed)?;
                let else_origin = self.visit(*else_value, next, observed)?;
                self.unify(then_value.0, else_value.0, id)?;
                self.unify(id.0, then_value.0, id)?;
                if then_origin == else_origin {
                    then_origin
                } else {
                    Observation::Selected(id)
                }
            }
            ExprKind::Quantifier {
                name,
                domain,
                predicate,
                ..
            } => {
                let origin = self.visit(*domain, next, observed)?;
                let element = self.variables.fresh();
                self.relations.push(Relation::Element {
                    input: domain.0,
                    output: element,
                    at: id,
                });
                self.enter_local(&name.value, element, origin, id)?;
                self.visit(*predicate, next, observed)?;
                self.locals.pop();
                self.boolean(*predicate)?;
                self.boolean(id)?;
                Observation::Independent
            }
            ExprKind::Reaches { start, target, .. } => {
                self.visit(*start, next, observed)?;
                self.visit(*target, next, observed)?;
                self.boolean(id)?;
                Observation::Independent
            }
        };
        self.observations[id.0] = Some(origin);
        Ok(origin)
    }

    fn resolve_relations(&mut self) -> Result<()> {
        // Direct unions are complete. Each relation watches one root; each root
        // can become known once, so notification work is linear in this table.
        let mut watchers = vec![Vec::new(); self.variables.parents.len()];
        for (index, relation) in self.relations.iter().enumerate() {
            watchers[self.variables.root(relation.input())].push(index);
        }
        let mut queue: VecDeque<_> = (0..self.relations.len()).collect();
        while let Some(index) = queue.pop_front() {
            let relation = self.relations[index];
            let Some(input) = self.variables.get(relation.input()) else {
                continue;
            };
            let assignment = match (relation, input) {
                (Relation::Option { output, .. }, NativeType::Option(value)) => {
                    output.map(|var| (var, *value))
                }
                (Relation::Element { output, .. }, NativeType::Sequence { element, .. }) => {
                    Some((output, *element))
                }
                (Relation::Deref { output, .. }, NativeType::Reference { model, role }) => {
                    Some((output, NativeType::Object { model, role }))
                }
                _ => return Err(self.invalid(
                    relation.at(),
                    "native operation has an incompatible option, sequence or reference operand",
                )),
            };
            if let Some((var, ty)) = assignment {
                let changed = self.variables.assign(var, ty).map_err(|()| {
                    self.invalid(
                        relation.at(),
                        "native wrapper result disagrees with its context",
                    )
                })?;
                if changed {
                    let root = self.variables.root(var);
                    queue.extend(watchers[root].iter().copied());
                }
            }
        }
        Ok(())
    }

    fn known(&mut self, id: ExprId) -> Result<NativeType<'a>> {
        self.variables.get(id.0).ok_or_else(|| {
            self.invalid(id, "native scalar inference has no unique declared context")
        })
    }
    fn validate_node(&mut self, id: ExprId) -> Result<()> {
        let ty = self.known(id)?;
        let valid = match &self.unit.expressions()[id.0].kind {
            ExprKind::Integer(text) => ty.integer().is_some_and(|integer| {
                text.parse::<i64>()
                    .is_ok_and(|n| n >= integer.minimum() && n <= integer.maximum())
            }),
            ExprKind::Text(text) => {
                matches!(&ty, NativeType::Scalar { role, .. } if matches!(role.kind, ScalarKind::Text { max_scalars } if text.chars().count() <= usize::try_from(max_scalars).unwrap_or(usize::MAX)))
            }
            ExprKind::Unary {
                op: UnaryOp::Negate,
                ..
            } => ty.integer().is_some(),
            ExprKind::Binary { op, left, .. } => {
                let operand = self.known(*left)?;
                match op {
                    BinaryOp::Add | BinaryOp::Subtract => operand.integer().is_some(),
                    BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => {
                        operand.integer().is_some() && operand.dimensionless()
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => matches!(operand, NativeType::Scalar { .. }),
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        let catalog = match &operand {
                            NativeType::Record { model, .. } => {
                                self.catalogs.iter().find(|c| owner_equal(c.model, model))
                            }
                            _ => Some(&self.catalogs[self.current]),
                        };
                        catalog.is_some_and(|catalog| catalog.equality(&operand))
                    }
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Implies => true,
                }
            }
            ExprKind::Call {
                builtin: Builtin::Size,
                argument,
            } => {
                let input = self.known(*argument)?;
                matches!(input, NativeType::Sequence { maximum, .. } if ty.dimensionless() && ty.integer().is_some_and(|integer| integer.minimum() <= 0 && integer.maximum() >= i64::from(maximum)))
            }
            ExprKind::Reaches { start, target, .. } => {
                let start_ty = self.known(*start)?;
                let target_ty = self.known(*target)?;
                let endpoints = match (&start_ty, &target_ty) {
                    (
                        NativeType::Object { model: a, role: ar }
                        | NativeType::Reference { model: a, role: ar },
                        NativeType::Object { model: b, role: br }
                        | NativeType::Reference { model: b, role: br },
                    ) => owner_equal(a, b) && ar.record == br.record && ar.universe == br.universe,
                    _ => false,
                };
                endpoints && self.observations[start.0] == self.observations[target.0]
            }
            ExprKind::Group { .. }
            | ExprKind::Boolean(_)
            | ExprKind::Name(_)
            | ExprKind::SelfValue
            | ExprKind::ResultValue
            | ExprKind::EnumValue { .. }
            | ExprKind::Field { .. }
            | ExprKind::Unary {
                op: UnaryOp::Not, ..
            }
            | ExprKind::Call { .. }
            | ExprKind::Let { .. }
            | ExprKind::If { .. }
            | ExprKind::Quantifier { .. } => true,
        };
        if valid {
            Ok(())
        } else {
            Err(self.invalid(
                id,
                "native literal, operator or observation does not satisfy its exact declared type",
            ))
        }
    }
}

fn owner_equal(a: &NativeModel, b: &NativeModel) -> bool {
    a.environment().owner() == b.environment().owner()
}

pub(super) fn solve<'a>(
    linked: &LinkedPackage<'a>,
    catalogs: &[Catalog<'a>],
    index: usize,
    limits: CheckLimits,
    usage: &mut CheckUsage,
) -> Result<TypedClause<'a>> {
    let clause = &linked.unit().clauses()[index];
    let resolved = &linked.clauses()[index];
    let catalog = &catalogs[resolved.model()];
    let DeclarationKey::Type(context_name) = &resolved.context().identity.key else {
        return Err(failure(
            linked.unit().source(),
            Code::InvalidModelBinding,
            clause.span,
            "native context is not a record declaration",
        ));
    };
    let context = catalog.objects.get(context_name).copied().ok_or_else(|| {
        failure(
            linked.unit().source(),
            Code::InvalidModelBinding,
            clause.span,
            "native context has no object role",
        )
    })?;
    let operation = clause.operation.as_ref().and_then(|name| {
        catalog
            .model
            .roles()
            .operations
            .iter()
            .find(|op| op.context == context.record && op.name.as_str() == name.value)
    });
    let observed = match clause.kind {
        ClauseKind::Invariant => ir::StateObservation::Current,
        ClauseKind::Precondition => ir::StateObservation::Pre,
        ClauseKind::Postcondition => ir::StateObservation::Post,
    };
    let mut solver = Solver {
        unit: linked.unit(),
        catalogs,
        current: resolved.model(),
        kind: clause.kind,
        context,
        operation,
        occurrences: resolved
            .occurrences()
            .iter()
            .filter_map(
                |occurrence| match (&occurrence.expression, &occurrence.target) {
                    (Some(expression), ResolutionTarget::Formal(location)) => {
                        Some((expression.0, location))
                    }
                    _ => None,
                },
            )
            .collect(),
        variables: Variables::new(linked.unit().expressions().len()),
        observations: vec![None; linked.unit().expressions().len()],
        locals: Vec::new(),
        relations: Vec::new(),
        visited: Vec::new(),
        limits,
        max_depth: 0,
    };
    solver.visit(clause.expression, 1, observed)?;
    solver.boolean(clause.expression)?;
    solver.resolve_relations()?;
    let mut visited = std::mem::take(&mut solver.visited);
    for &id in &visited {
        solver.validate_node(id)?;
    }
    visited.sort_by_key(|id| id.0);
    let mut nodes = Vec::with_capacity(visited.len());
    let mut universes = Populations::new(catalogs, linked.unit().source());
    let context_observations = if clause.kind == ClauseKind::Postcondition {
        vec![ir::StateObservation::Pre, ir::StateObservation::Post]
    } else {
        vec![observed]
    };
    for &observation in &context_observations {
        universes.require(catalog.model, &context.record, observation)?;
    }
    for id in visited {
        let ty = solver.known(id)?;
        let observation = solver.observations[id.0]
            .ok_or_else(|| solver.invalid(id, "native value has no observation correspondence"))?;
        if let (Some((model, record)), Observation::Snapshot(observed)) =
            (ty.population_record(), observation)
        {
            universes.require(model, record, observed)?;
        }
        nodes.push(NodeType {
            expression: id,
            ty,
            observation,
        });
    }
    if let Some(operation) = operation {
        for name in operation
            .frame
            .fields
            .iter()
            .map(|(record, _)| record)
            .chain(&operation.frame.created)
            .chain(&operation.frame.deleted)
            .chain(std::iter::once(&operation.context))
        {
            universes.require(catalog.model, name, ir::StateObservation::Pre)?;
            universes.require(catalog.model, name, ir::StateObservation::Post)?;
        }
        for name in &operation.parameters {
            universes.require_value(catalog.model, name, ir::StateObservation::Pre)?;
        }
        if let Some(name) = &operation.result {
            universes.require_value(catalog.model, name, ir::StateObservation::Post)?;
        }
    }
    usage.max_native_depth = usage.max_native_depth.max(solver.max_depth);
    let runtime = RuntimeRequirements {
        context: resolved.context().clone(),
        model: catalog.model,
        context_observations,
        universes: universes.into_requirements(),
        operation: solver.operation,
        validate_frame: operation.is_some(),
    };
    Ok(TypedClause { nodes, runtime })
}
