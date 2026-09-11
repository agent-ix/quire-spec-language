// SPDX-License-Identifier: AGPL-3.0-only
//! Constraint generation over each original declaration's contiguous arena region.

mod dependencies;
mod expressions;
mod literals;
mod models;
mod origins;
mod roots;
mod validation;

use super::sources::Correspondence;
use super::work::{Dimension as D, Work};
use super::*;
use crate::checking::{variables::Variables, Catalog};
use crate::linking::composed::models::{ModelInput, ModelTarget};
use crate::linking::composed::scopes::{self, DeclarationScope};
use crate::linking::composed::{DependencyKind, DependencySite};
use crate::native_model::{NativeModel, ScalarKind, ScalarSite, Unit};
use crate::syntax::{composed as c, BinaryOp, Builtin, ExprKind, UnaryOp};
use quire_contract_ir as ir;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, Exhaustion>;

pub(super) fn admit<'r, 'a>(
    binding: &'r binding::Report<'a>,
    sources: &[FormalSource],
    limits: TypeLimits,
) -> TypeReport<'r, 'a> {
    let mut work = Work::new(limits);
    let mut report = TypeReport {
        binding,
        declarations: Vec::new(),
        exhaustion: None,
        limits: work.limits(),
        usage: work.usage(),
    };
    let run = (|| -> Result<()> {
        let Some(first) = binding.declarations().first() else {
            return Ok(());
        };
        let first_id = first.declaration();
        let entry = binding
            .namespace()
            .declaration(first_id)
            .expect("original binding declaration");
        let first_site = Site {
            declaration: first_id,
            unit: entry.unit(),
            expression: None,
            span: binding
                .namespace()
                .syntax(first_id)
                .expect("original syntax")
                .span,
        };
        let formal = Correspondence::new(binding, sources, &mut work, first_site)?;
        let model_bounds = models::ModelBounds::new(binding, &mut work, first_site)?;
        for bound in binding.declarations() {
            let id = bound.declaration();
            let entry = binding
                .namespace()
                .declaration(id)
                .expect("original binding declaration");
            let syntax = binding.namespace().syntax(id).expect("original syntax");
            let site = Site {
                declaration: id,
                unit: entry.unit(),
                expression: None,
                span: syntax.span,
            };
            work.charge(D::Declarations, 1, site)?;
            work.charge(D::Records, 1, site)?;
            report.declarations.push(DeclarationTypes {
                declaration: id,
                unit: entry.unit(),
                nodes: Vec::new(),
                binders: Vec::new(),
                causes: Vec::new(),
                obligations: Vec::new(),
                complete: false,
                local_complete: false,
            });
            let output = report
                .declarations
                .last_mut()
                .expect("created declaration result");
            match binding.disposition(id) {
                Some(binding::Disposition::Refused) => {
                    work.charge(D::Records, 1, site)?;
                    output.causes.push(TypeCause {
                        site,
                        kind: CauseKind::UpstreamBinding,
                        profile: 0,
                    });
                    output.complete = true;
                }
                Some(binding::Disposition::Unfinished) | None => {}
                Some(binding::Disposition::NamesResolved) => {
                    let scope = binding
                        .scopes()
                        .and_then(|scopes| scopes.declaration(id))
                        .expect("completed lexical binding");
                    let unit = binding
                        .namespace()
                        .unit(entry.unit())
                        .expect("original unit");
                    let range = owned(
                        unit.expressions(),
                        syntax.span,
                        |node| node.span,
                        &mut work,
                        site,
                    )?;
                    let mut solver = Solver::new(
                        binding,
                        unit,
                        scope,
                        output,
                        range,
                        formal.get(entry.unit()),
                        &model_bounds,
                        &mut work,
                    )?;
                    solver.solve(syntax)?;
                }
            }
        }
        dependencies::finish(&mut report, &mut work, first_site)?;
        Ok(())
    })();
    if let Err(error) = run {
        report.exhaustion = Some(error);
    }
    report.usage = work.usage();
    report
}

fn owned<T>(
    nodes: &[T],
    owner: Span,
    span: impl Fn(&T) -> Span,
    work: &mut Work,
    site: Site,
) -> Result<std::ops::Range<usize>> {
    let mut boundary = |at| -> Result<usize> {
        let (mut low, mut high) = (0, nodes.len());
        while low < high {
            work.charge(D::Expressions, 1, site)?;
            let middle = low + (high - low) / 2;
            if span(&nodes[middle]).start < at {
                low = middle + 1
            } else {
                high = middle
            }
        }
        Ok(low)
    };
    Ok(boundary(owner.start)?..boundary(owner.end)?)
}

#[derive(Clone, Copy)]
enum Relation {
    Option {
        input: usize,
        output: Option<usize>,
    },
    Sequence {
        input: usize,
        element: usize,
    },
    Field {
        input: usize,
        output: usize,
    },
    Deref {
        input: usize,
        output: usize,
    },
    Map {
        input: usize,
        body: usize,
        output: usize,
    },
}
#[derive(Clone, Copy)]
struct Pending {
    relation: Relation,
    at: ExprId,
}

struct Solver<'b, 'a, 's, 'w> {
    binding: &'b binding::Report<'a>,
    unit: &'b c::ComposedUnit,
    scope: &'b DeclarationScope,
    output: &'w mut DeclarationTypes<'a>,
    range: std::ops::Range<usize>,
    formal: Option<&'s FormalSource>,
    work: &'w mut Work,
    vars: Variables<'a>,
    relations: Vec<Pending>,
    profiles: Vec<usize>,
    depths: Vec<usize>,
    captures: BTreeMap<usize, ExprId>,
    occurrences: BTreeMap<usize, usize>,
    model_bounds: &'b models::ModelBounds,
}

impl<'b, 'a, 's, 'w> Solver<'b, 'a, 's, 'w> {
    fn new(
        binding: &'b binding::Report<'a>,
        unit: &'b c::ComposedUnit,
        scope: &'b DeclarationScope,
        output: &'w mut DeclarationTypes<'a>,
        range: std::ops::Range<usize>,
        formal: Option<&'s FormalSource>,
        model_bounds: &'b models::ModelBounds,
        work: &'w mut Work,
    ) -> Result<Self> {
        let site = Site {
            declaration: output.declaration,
            unit: output.unit,
            expression: None,
            span: binding
                .namespace()
                .syntax(output.declaration)
                .expect("syntax")
                .span,
        };
        work.charge(D::Constraints, range.len() + scope.binders.len(), site)?;
        let count = range.len() + scope.binders.len();
        let mut occurrences = BTreeMap::new();
        for (index, value) in scope.values.iter().enumerate() {
            work.charge(D::Constraints, 1, site)?;
            work.charge(D::Records, 1, site)?;
            occurrences.insert(value.expression.0, index);
        }
        Ok(Self {
            binding,
            unit,
            scope,
            output,
            profiles: vec![0; range.len()],
            depths: vec![0; range.len()],
            range,
            formal,
            work,
            vars: Variables::new(count),
            relations: Vec::new(),
            captures: BTreeMap::new(),
            occurrences,
            model_bounds,
        })
    }
    fn site(&self, at: ExprId) -> Site {
        Site {
            declaration: self.output.declaration,
            unit: self.output.unit,
            expression: Some(at),
            span: self.unit.expressions()[at.0].span,
        }
    }
    fn declaration_site(&self) -> Site {
        Site {
            declaration: self.output.declaration,
            unit: self.output.unit,
            expression: None,
            span: self
                .binding
                .namespace()
                .syntax(self.output.declaration)
                .expect("syntax")
                .span,
        }
    }
    fn var(&self, at: ExprId) -> usize {
        debug_assert!(self.range.contains(&at.0));
        at.0 - self.range.start
    }
    fn binder_var(&self, index: usize) -> usize {
        self.range.len() + index
    }
    fn occurrence(&mut self, at: ExprId) -> Result<Option<&'b scopes::ValueOccurrence>> {
        self.work.charge(D::Constraints, 1, self.site(at))?;
        Ok(self
            .occurrences
            .get(&at.0)
            .map(|index| &self.scope.values[*index]))
    }
    fn cause(&mut self, at: ExprId, kind: CauseKind) -> Result<()> {
        let site = self.site(at);
        self.work.charge(D::Records, 1, site)?;
        self.output.causes.push(TypeCause {
            site,
            kind,
            profile: self.profiles[self.var(at)],
        });
        Ok(())
    }
    fn obligation(&mut self, at: ExprId, kind: ObligationKind) -> Result<()> {
        let site = self.site(at);
        self.work.charge(D::Records, 1, site)?;
        self.output.obligations.push(Obligation { site, kind });
        Ok(())
    }
    fn get(&mut self, var: usize, at: ExprId) -> Result<Option<NativeType<'a>>> {
        let site = self.site(at);
        self.work.charge(D::Constraints, 1, site)?;
        if let Some(ty) = self.vars.peek(var) {
            charge_type(ty, self.work, site, 1)?;
        }
        Ok(self.vars.get(var))
    }
    fn assign(&mut self, var: usize, ty: NativeType<'a>, at: ExprId) -> Result<bool> {
        charge_type(&ty, self.work, self.site(at), 1)?;
        self.work.charge(D::Constraints, 1, self.site(at))?;
        match self.vars.assign(var, ty) {
            Ok(changed) => Ok(changed),
            Err(()) => {
                self.cause(at, CauseKind::TypeMismatch)?;
                Ok(false)
            }
        }
    }
    fn unify(&mut self, left: usize, right: usize, at: ExprId) -> Result<()> {
        self.get(left, at)?;
        self.get(right, at)?;
        self.work.charge(D::Constraints, 1, self.site(at))?;
        if self.vars.unify(left, right).is_err() {
            self.cause(at, CauseKind::TypeMismatch)?;
        }
        Ok(())
    }
    fn boolean(&mut self, at: ExprId) -> Result<()> {
        self.assign(self.var(at), NativeType::Boolean, at)?;
        Ok(())
    }
    fn relation(&mut self, relation: Relation, at: ExprId) -> Result<()> {
        self.work.charge(D::Constraints, 1, self.site(at))?;
        self.relations.push(Pending { relation, at });
        Ok(())
    }
    fn profile(&self, at: ExprId) -> RegisteredDefinition {
        self.binding
            .definitions()
            .expect("resolved definitions")
            .declarations[self.output.declaration.index()]
        .uses[self.profiles[self.var(at)]]
        .closure[0]
    }
    fn require(&mut self, at: ExprId, queries: bool, graph: bool) -> Result<()> {
        let (q, g) = permissions(self.profile(at));
        if (queries && !q) || (graph && !g) {
            self.cause(at, CauseKind::ProfilePermission)?;
        }
        Ok(())
    }
    fn catalog(&mut self, model: &'a NativeModel, at: ExprId) -> Result<&'b Catalog<'a>> {
        let models = self.binding.models().expect("resolved models");
        for (index, input) in models.inputs().iter().enumerate() {
            self.work.charge(D::Constraints, 1, self.site(at))?;
            if let ModelInput::Native(candidate) = input {
                if std::ptr::eq(*candidate, model) {
                    return Ok(models.catalog_at(index).expect("admitted catalog"));
                }
            }
        }
        unreachable!("bound native type belongs to input catalog")
    }
    fn qualified(&mut self, name: &c::QualifiedName, at: ExprId) -> Result<Option<NativeType<'a>>> {
        let span = Span {
            start: name.model.span.start,
            end: name.name.span.end,
        };
        for occurrence in &self.binding.exports()[self.output.declaration.index()].occurrences {
            self.work.charge(D::Constraints, 1, self.site(at))?;
            if occurrence.span == span {
                if let ModelTarget::Type(bound) = &occurrence.target {
                    charge_type(bound.native(), self.work, self.site(at), 1)?;
                    return Ok(Some(bound.native().clone()));
                }
            }
        }
        self.cause(at, CauseKind::UpstreamBinding)?;
        Ok(None)
    }
    fn binder_type(&mut self, index: usize, at: ExprId) -> Result<Option<NativeType<'a>>> {
        match &self.scope.binders[index].ty {
            scopes::BinderType::Declared(c::ParameterType::Boolean(_)) => {
                Ok(Some(NativeType::Boolean))
            }
            scopes::BinderType::Declared(c::ParameterType::Model(name))
            | scopes::BinderType::Context(name) => {
                // Operation selectors retain a bound operation rather than a duplicate context export.
                if matches!(self.scope.binders[index].ty, scopes::BinderType::Context(_)) {
                    for occurrence in
                        &self.binding.exports()[self.output.declaration.index()].occurrences
                    {
                        self.work.charge(D::Constraints, 1, self.site(at))?;
                        if let ModelTarget::Operation(operation) = &occurrence.target {
                            if occurrence.span.start == name.model.span.start {
                                let catalog = self.catalog(operation.model(), at)?;
                                return Ok(catalog.record_type(&operation.role().context));
                            }
                        }
                    }
                }
                self.qualified(name, at)
            }
            scopes::BinderType::ModelValue(location) => {
                let models = self.binding.models().expect("bound models");
                for (input_index, input) in models.inputs().iter().enumerate() {
                    self.work.charge(D::Constraints, 1, self.site(at))?;
                    if let ModelInput::Native(model) = input {
                        if model.environment().owner() == &location.identity.owner {
                            let catalog = models.catalog_at(input_index).expect("admitted catalog");
                            if let crate::linking::DeclarationKey::Value(name) =
                                &location.identity.key
                            {
                                if let Some(value) = catalog.values.get(name) {
                                    return self.formal_type(
                                        catalog,
                                        value.value_type(),
                                        ScalarSite::Value { name: name.clone() },
                                        at,
                                    );
                                }
                            }
                        }
                    }
                }
                self.cause(at, CauseKind::UpstreamBinding)?;
                Ok(None)
            }
            scopes::BinderType::Initializer(value) => {
                self.unify(self.binder_var(index), self.var(*value), at)?;
                Ok(None)
            }
            scopes::BinderType::ElementOf(domain) => {
                self.relation(
                    Relation::Sequence {
                        input: self.var(*domain),
                        element: self.binder_var(index),
                    },
                    at,
                )?;
                Ok(None)
            }
        }
    }
    fn formal_type(
        &mut self,
        catalog: &Catalog<'a>,
        ty: &'a ir::ValueType,
        site: ScalarSite,
        at: ExprId,
    ) -> Result<Option<NativeType<'a>>> {
        charge_formal(ty, self.work, self.site(at), 1)?;
        Ok(catalog.formal(ty, &site))
    }
    fn solve(&mut self, syntax: &c::Declaration) -> Result<()> {
        self.model_bounds()?;
        if self.range.is_empty() {
            self.empty_binders()?;
            self.output.local_complete = true;
            self.output.complete = self
                .binding
                .namespace()
                .declaration(self.output.declaration)
                .expect("bound declaration")
                .references()
                .is_empty();
            return Ok(());
        }
        let first = ExprId(self.range.start);
        for index in 0..self.scope.binders.len() {
            if let Some(ty) = self.binder_type(index, first)? {
                self.assign(self.binder_var(index), ty, first)?;
            }
        }
        self.roots(syntax)?;
        for index in self.range.clone() {
            self.expression(ExprId(index))?;
        }
        self.solve_relations()?;
        for index in self.range.clone() {
            self.validate(ExprId(index))?;
        }
        for index in self.range.clone() {
            let at = ExprId(index);
            let ty = self.get(self.var(at), at)?;
            if ty.is_none() {
                self.cause(at, CauseKind::AmbiguousType)?;
            }
            self.work.charge(D::Records, 1, self.site(at))?;
            let occurrence = self.occurrence(at)?;
            self.output.nodes.push(NodeType {
                expression: at,
                span: self.unit.expressions()[index].span,
                ty,
                binder: occurrence.map(|value| value.target),
                anchor: occurrence.map(|value| self.scope.binders[value.target.index()].anchor),
                origin: None,
                normalized_rational: None,
                profile: self.profiles[self.var(at)],
            });
        }
        self.literals()?;
        self.origins()?;
        for index in 0..self.scope.binders.len() {
            let ty = self.get(self.binder_var(index), first)?;
            self.work.charge(D::Records, 1, self.declaration_site())?;
            self.output.binders.push(BinderType {
                binder: index,
                ty,
                anchor: self.scope.binders[index].anchor,
            });
        }
        self.output.local_complete = true;
        self.output.complete = self
            .binding
            .namespace()
            .declaration(self.output.declaration)
            .expect("bound declaration")
            .references()
            .is_empty();
        Ok(())
    }
}

fn charge_type(ty: &NativeType<'_>, work: &mut Work, site: Site, depth: usize) -> Result<()> {
    work.charge(D::Depth, depth, site)?;
    work.charge(D::Constraints, 1, site)?;
    let mut bytes = 0;
    match ty {
        NativeType::Boolean => {}
        NativeType::Option(value) => charge_type(value, work, site, depth + 1)?,
        NativeType::Sequence { element, .. } => charge_type(element, work, site, depth + 1)?,
        NativeType::Scalar { model, role, .. } => {
            bytes += role.name.as_str().len()
                + model.environment().owner().package().as_str().len()
                + model.environment().owner().requirement().as_str().len();
            match &role.kind {
                ScalarKind::Integer { unit } | ScalarKind::Rational { unit } => {
                    if let Unit::Named(name) = unit {
                        bytes += name.as_str().len()
                    }
                }
                ScalarKind::Text { .. } => {}
            }
        }
        NativeType::Record { model, declaration } => {
            bytes += declaration.name().as_str().len()
                + model.environment().owner().package().as_str().len()
                + model.environment().owner().requirement().as_str().len();
        }
        NativeType::Enumeration { model, declaration } => {
            bytes += declaration.name().as_str().len()
                + model.environment().owner().package().as_str().len()
                + model.environment().owner().requirement().as_str().len();
        }
        NativeType::Object { model, role } | NativeType::Reference { model, role } => {
            bytes += role.record.as_str().len()
                + role.universe.as_str().len()
                + model.environment().owner().package().as_str().len()
                + model.environment().owner().requirement().as_str().len();
        }
    }
    work.charge(D::Bytes, bytes, site)
}
fn charge_formal(ty: &ir::ValueType, work: &mut Work, site: Site, depth: usize) -> Result<()> {
    work.charge(D::Depth, depth, site)?;
    work.charge(D::Constraints, 1, site)?;
    match ty {
        ir::ValueType::Option { value } => charge_formal(value, work, site, depth + 1)?,
        ir::ValueType::Collection { value } => {
            charge_formal(value.element(), work, site, depth + 1)?
        }
        ir::ValueType::Record { name } | ir::ValueType::Enum { name } => {
            work.charge(D::Bytes, name.as_str().len(), site)?
        }
        ir::ValueType::Boolean
        | ir::ValueType::Integer { .. }
        | ir::ValueType::Rational { .. }
        | ir::ValueType::Text => {}
    }
    Ok(())
}
