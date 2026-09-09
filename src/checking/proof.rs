// SPDX-License-Identifier: AGPL-3.0-only
//! FR-016: shared symbolic proof graph and actual bounded IR discharge.

mod facts;
mod graph;
mod walk;

use std::collections::BTreeMap;
use std::rc::Rc;

use quire_contract_ir as ir;

use super::constraints::NodeType;
use super::types::Catalog;
use super::{
    failure, CheckLimits, CheckUsage, ClauseBinding, NativeType, Observation, PresencePremise,
    ProofGoal, ProofValue, Result,
};
use crate::formal_source::FormalSource;
use crate::linking::{DeclarationKey, DeclarationLocation, LinkedPackage, ResolutionTarget};
use crate::syntax::{BinaryOp, Builtin, ExprId, ExprKind, UnaryOp};
use crate::{Code, Source, Span};
use facts::{Facts, Outcomes};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ValueKey(usize);
#[derive(Clone, Copy, Debug)]
struct GraphId(usize);

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum Step<'a> {
    Field(
        &'a ir::RequirementRef,
        &'a ir::SymbolName,
        &'a ir::SymbolName,
    ),
    Unwrap,
    Deref,
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum Key<'a> {
    Context(&'a ir::RequirementRef, &'a ir::SymbolName, u8),
    Declaration(&'a ir::RequirementRef, &'a ir::SymbolName, u8),
    Path(ValueKey, Step<'a>),
    Local(usize),
    Expression(usize),
}

struct KeyInfo {
    native: ExprId,
    graph: Option<GraphId>,
    symbol: Option<ir::SymbolName>,
}

enum Kind<'a> {
    Boolean(bool),
    Integer(i64, &'a ir::IntegerType),
    Input(ValueKey),
    Present(GraphId),
    Unwrap(GraphId),
    Negate(GraphId),
    Not(GraphId),
    Numeric(ir::NumericOperator, GraphId, GraphId),
    Compare(ir::ComparisonOperator, GraphId, GraphId),
    BooleanOp(ir::BooleanOperator, GraphId, GraphId),
    Witness(GraphId, ir::ValueType),
}

struct Node<'a> {
    kind: Kind<'a>,
    native: ExprId,
}

#[derive(Clone)]
struct Value {
    key: ValueKey,
    graph: GraphId,
    stable: bool,
    outcomes: Rc<Outcomes>,
}

#[derive(Clone, Copy)]
struct Guard {
    graph: GraphId,
    truth: bool,
    native: ExprId,
}

#[derive(Clone)]
struct Path {
    facts: Rc<Facts>,
    guards: Vec<Guard>,
}

struct PendingGoal {
    native: ExprId,
    graph: GraphId,
    premises: Vec<PresencePremise>,
}

struct Meter<'s> {
    source: &'s Source,
    limits: CheckLimits,
    usage: &'s mut CheckUsage,
}

fn charge(
    source: &Source,
    used: &mut usize,
    amount: usize,
    maximum: usize,
    span: Span,
    dimension: &str,
) -> Result<()> {
    let next = used
        .checked_add(amount)
        .filter(|next| *next <= maximum)
        .ok_or_else(|| {
            failure(
                source,
                Code::ResourceExhausted,
                span,
                format!("checking {dimension} limit exceeded"),
            )
        })?;
    *used = next;
    Ok(())
}

impl Meter<'_> {
    fn facts(&mut self, count: usize, span: Span) -> Result<()> {
        charge(
            self.source,
            &mut self.usage.presence_work,
            count,
            self.limits.presence_work,
            span,
            "presence work",
        )
    }
    fn materialize(&mut self, count: &mut usize, depth: usize, span: Span) -> Result<()> {
        if depth > self.limits.depth {
            return Err(failure(
                self.source,
                Code::ResourceExhausted,
                span,
                "expanded proof depth limit exceeded",
            ));
        }
        charge(
            self.source,
            count,
            1,
            self.limits.goal_nodes,
            span,
            "per-goal nodes",
        )?;
        charge(
            self.source,
            &mut self.usage.materialized_nodes,
            1,
            self.limits.materialized_nodes,
            span,
            "materialized nodes",
        )?;
        self.usage.max_goal_nodes = self.usage.max_goal_nodes.max(*count);
        self.usage.max_proof_depth = self.usage.max_proof_depth.max(depth);
        Ok(())
    }
}

fn proof_type(ty: &NativeType<'_>) -> std::result::Result<ir::ValueType, ir::Diagnostic> {
    Ok(match ty {
        NativeType::Scalar {
            representation: ir::ValueType::Integer { value },
            ..
        } => ir::ValueType::integer(value.clone()),
        NativeType::Option(value) => ir::ValueType::option(proof_type(value)?),
        NativeType::Sequence { element, maximum } => ir::ValueType::Collection {
            value: ir::CollectionType::new(proof_type(element)?, *maximum)?,
        },
        NativeType::Boolean
        | NativeType::Scalar { .. }
        | NativeType::Enumeration { .. }
        | NativeType::Record { .. }
        | NativeType::Object { .. }
        | NativeType::Reference { .. } => ir::ValueType::Boolean,
    })
}

struct Builder<'u, 'a> {
    linked: &'u LinkedPackage<'a>,
    catalogs: &'u [Catalog<'a>],
    formal: &'u FormalSource,
    types: &'u [NodeType<'a>],
    occurrences: BTreeMap<usize, &'u DeclarationLocation>,
    keys: BTreeMap<Key<'a>, ValueKey>,
    key_info: Vec<KeyInfo>,
    graph: Vec<Node<'a>>,
    inputs: Vec<ir::ValueDeclaration>,
    values: Vec<ProofValue>,
    goals: Vec<PendingGoal>,
    locals: Vec<(&'u str, Value)>,
    meter: Meter<'u>,
}

impl<'u, 'a> Builder<'u, 'a> {
    fn span(&self, id: ExprId) -> Span {
        self.linked.unit().expressions()[id.0].span
    }
    fn source(&self, id: ExprId) -> Result<ir::SourceSpan> {
        self.formal
            .to_ir(self.linked.unit().source(), self.span(id))
    }
    fn ty(&self, id: ExprId) -> Result<&'u NodeType<'a>> {
        self.types
            .binary_search_by_key(&id.0, |node| node.expression.0)
            .ok()
            .map(|index| &self.types[index])
            .ok_or_else(|| {
                failure(
                    self.linked.unit().source(),
                    Code::InvalidModelBinding,
                    self.span(id),
                    "proof has no native type correspondence",
                )
            })
    }
    fn node(&mut self, kind: Kind<'a>, native: ExprId) -> Result<GraphId> {
        let span = self.span(native);
        charge(
            self.meter.source,
            &mut self.meter.usage.proof_graph_nodes,
            1,
            self.meter.limits.proof_graph_nodes,
            span,
            "proof graph nodes",
        )?;
        let id = GraphId(self.graph.len());
        self.graph.push(Node { kind, native });
        Ok(id)
    }
    fn key(&mut self, key: Key<'a>, native: ExprId) -> ValueKey {
        if let Some(id) = self.keys.get(&key) {
            return *id;
        }
        let id = ValueKey(self.key_info.len());
        self.keys.insert(key, id);
        self.key_info.push(KeyInfo {
            native,
            graph: None,
            symbol: None,
        });
        id
    }
    fn symbolic(
        &mut self,
        key: Key<'a>,
        native: ExprId,
        ty: &NativeType<'a>,
        stable: bool,
    ) -> Result<Value> {
        let key = self.key(key, native);
        if let Some(graph) = self.key_info[key.0].graph {
            return Ok(Value {
                key,
                graph,
                stable,
                outcomes: Rc::new(Outcomes::unknown()),
            });
        }
        let span = self.span(native);
        charge(
            self.meter.source,
            &mut self.meter.usage.proof_values,
            1,
            self.meter.limits.proof_values,
            span,
            "proof values",
        )?;
        let symbol = ir::SymbolName::new(format!("proof{}", key.0)).map_err(|upstream| {
            let mut error = failure(
                self.meter.source,
                Code::InvalidModelBinding,
                span,
                "generated proof symbol is invalid",
            );
            error.upstream = Some(Box::new(upstream));
            error
        })?;
        let source = self.source(native)?;
        let representation = proof_type(ty).map_err(|error| {
            upstream(
                self.meter.source,
                span,
                vec![error],
                "native proof type conversion failed",
            )
        })?;
        self.inputs.push(ir::ValueDeclaration::new(
            symbol.clone(),
            ir::ValueDeclarationKind::Input,
            representation,
            source.clone(),
        ));
        let declarations = self
            .occurrences
            .get(&native.0)
            .map(|location| vec![(*location).clone()])
            .unwrap_or_default();
        self.values.push(ProofValue {
            symbol: symbol.clone(),
            expression: native,
            source,
            declarations,
        });
        self.key_info[key.0].symbol = Some(symbol);
        let graph = self.node(Kind::Input(key), native)?;
        self.key_info[key.0].graph = Some(graph);
        Ok(Value {
            key,
            graph,
            stable,
            outcomes: Rc::new(Outcomes::unknown()),
        })
    }
    fn expression_value(
        &mut self,
        graph: GraphId,
        native: ExprId,
        stable: bool,
        outcomes: Outcomes,
    ) -> Value {
        let key = self.key(Key::Expression(native.0), native);
        self.key_info[key.0].graph.get_or_insert(graph);
        Value {
            key,
            graph,
            stable,
            outcomes: Rc::new(outcomes),
        }
    }
    fn catalog(&self, native: ExprId) -> Result<(&'u Catalog<'a>, &'u DeclarationLocation)> {
        let location = self.occurrences.get(&native.0).copied().ok_or_else(|| {
            failure(
                self.meter.source,
                Code::InvalidModelBinding,
                self.span(native),
                "proof read has no linked model occurrence",
            )
        })?;
        let catalog = self
            .catalogs
            .iter()
            .find(|catalog| catalog.model.environment().owner() == &location.identity.owner)
            .ok_or_else(|| {
                failure(
                    self.meter.source,
                    Code::InvalidModelBinding,
                    self.span(native),
                    "proof read has no selected model",
                )
            })?;
        Ok((catalog, location))
    }
    fn observed(&self, id: ExprId) -> Result<u8> {
        match self.ty(id)?.observation {
            Observation::Snapshot(ir::StateObservation::Current) => Ok(0),
            Observation::Snapshot(ir::StateObservation::Pre) => Ok(1),
            Observation::Snapshot(ir::StateObservation::Post) => Ok(2),
            Observation::Independent | Observation::Selected(_) => Err(failure(
                self.meter.source,
                Code::InvalidModelBinding,
                self.span(id),
                "direct model read has no concrete observation",
            )),
        }
    }
    fn declaration(&mut self, native: ExprId) -> Result<Value> {
        let (catalog, location) = self.catalog(native)?;
        let DeclarationKey::Value(name) = &location.identity.key else {
            return Err(failure(
                self.meter.source,
                Code::InvalidModelBinding,
                self.span(native),
                "expected a value declaration",
            ));
        };
        let value = catalog.values.get(name).copied().ok_or_else(|| {
            failure(
                self.meter.source,
                Code::InvalidModelBinding,
                self.span(native),
                "declared proof read is absent",
            )
        })?;
        let key = Key::Declaration(
            catalog.model.environment().owner(),
            value.name(),
            self.observed(native)?,
        );
        self.symbolic(key, native, &self.ty(native)?.ty, true)
    }
    fn premise(&mut self, key: ValueKey, present: bool, at: ExprId) -> Result<GraphId> {
        let value = self.key_info[key.0].graph.ok_or_else(|| {
            failure(
                self.meter.source,
                Code::InvalidModelBinding,
                self.span(at),
                "presence premise has no proof value",
            )
        })?;
        let presence = self.node(Kind::Present(value), at)?;
        if present {
            Ok(presence)
        } else {
            self.node(Kind::Not(presence), at)
        }
    }
    fn protect(
        &mut self,
        mut graph: GraphId,
        facts: &Facts,
        at: ExprId,
        operator: ir::BooleanOperator,
    ) -> Result<GraphId> {
        if let Some(facts) = facts {
            self.meter.facts(facts.len(), self.span(at))?;
            for (&key, fact) in facts.iter().rev() {
                let premise = self.premise(key, fact.present, fact.guard)?;
                graph = self.node(Kind::BooleanOp(operator, premise, graph), at)?;
            }
        }
        Ok(graph)
    }
    fn assume(&mut self, path: &Path, value: &Value, truth: bool, native: ExprId) -> Result<Path> {
        let span = self.span(native);
        let combined = facts::sequential(
            &path.facts,
            value.outcomes.selected(truth),
            &mut self.meter,
            span,
        )?;
        let mut guards = path.guards.clone();
        guards.push(Guard {
            graph: value.graph,
            truth,
            native,
        });
        Ok(Path {
            facts: Rc::new(combined),
            guards,
        })
    }
    fn goal(&mut self, native: ExprId, graph: GraphId, path: &Path) -> Result<()> {
        if path.facts.is_none() {
            return Ok(());
        }
        let representation = proof_type(&self.ty(native)?.ty).map_err(|error| {
            upstream(
                self.meter.source,
                self.span(native),
                vec![error],
                "native goal type conversion failed",
            )
        })?;
        let mut graph = self.node(Kind::Witness(graph, representation), native)?;
        graph = self.protect(graph, &path.facts, native, ir::BooleanOperator::Implication)?;
        for guard in path.guards.iter().rev() {
            let condition = if guard.truth {
                guard.graph
            } else {
                self.node(Kind::Not(guard.graph), guard.native)?
            };
            graph = self.node(
                Kind::BooleanOp(ir::BooleanOperator::Implication, condition, graph),
                native,
            )?;
        }
        let premises = path
            .facts
            .as_ref()
            .as_ref()
            .map(|facts| {
                facts
                    .iter()
                    .map(|(&key, fact)| PresencePremise {
                        guard: fact.guard,
                        outcome: fact.outcome,
                        value: self.key_info[key.0].native,
                        present: fact.present,
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.goals.push(PendingGoal {
            native,
            graph,
            premises,
        });
        Ok(())
    }
}

pub(super) struct Proven {
    pub environment: ir::DeclarationEnvironment,
    pub values: Vec<ProofValue>,
    pub goals: Vec<ProofGoal>,
}

pub(super) struct Request<'u, 'a> {
    pub linked: &'u LinkedPackage<'a>,
    pub catalogs: &'u [Catalog<'a>],
    pub index: usize,
    pub binding: &'u ClauseBinding,
    pub formal: &'u FormalSource,
    pub types: &'u [NodeType<'a>],
    pub limits: CheckLimits,
    pub usage: &'u mut CheckUsage,
}

pub(super) fn discharge(
    Request {
        linked,
        catalogs,
        index,
        binding,
        formal,
        types,
        limits,
        usage,
    }: Request<'_, '_>,
) -> Result<Proven> {
    let clause = &linked.unit().clauses()[index];
    let occurrences = linked.clauses()[index]
        .occurrences()
        .iter()
        .filter_map(
            |occurrence| match (&occurrence.expression, &occurrence.target) {
                (Some(id), ResolutionTarget::Formal(location)) => Some((id.0, location)),
                _ => None,
            },
        )
        .collect();
    let mut builder = Builder {
        linked,
        catalogs,
        formal,
        types,
        occurrences,
        keys: BTreeMap::new(),
        key_info: Vec::new(),
        graph: Vec::new(),
        inputs: Vec::new(),
        values: Vec::new(),
        goals: Vec::new(),
        locals: Vec::new(),
        meter: Meter {
            source: linked.unit().source(),
            limits,
            usage,
        },
    };
    builder.visit(
        clause.expression,
        &Path {
            facts: Rc::new(Some(BTreeMap::new())),
            guards: Vec::new(),
        },
    )?;
    let environment = ir::DeclarationEnvironment::new(
        binding.requirement.clone(),
        Vec::new(),
        std::mem::take(&mut builder.inputs),
        Vec::new(),
    )
    .map_err(|diagnostics| {
        upstream(
            builder.meter.source,
            clause.span,
            diagnostics,
            "proof environment admission failed",
        )
    })?;
    let pending = std::mem::take(&mut builder.goals);
    let mut goals = Vec::with_capacity(pending.len());
    for goal in pending {
        let expression = builder.materialize(goal.graph, 1, &mut 0)?;
        let checked = environment
            .check_expression(
                &expression,
                &ir::ValueType::Boolean,
                &binding.execution_point,
                true,
            )
            .map_err(|diagnostics| {
                upstream(
                    builder.meter.source,
                    builder.span(goal.native),
                    diagnostics,
                    "native expression definedness could not be proved",
                )
            })?;
        goals.push(ProofGoal {
            native: goal.native,
            source: builder.source(goal.native)?,
            checked,
            premises: goal.premises,
        });
    }
    Ok(Proven {
        environment,
        values: builder.values,
        goals,
    })
}

fn upstream(
    source: &Source,
    span: Span,
    diagnostics: Vec<ir::Diagnostic>,
    message: &str,
) -> Box<crate::Diagnostic> {
    let cause = diagnostics.into_iter().next();
    let code = match cause.as_ref().map(|diagnostic| diagnostic.code) {
        Some(
            ir::DiagnosticCode::ExpressionTooLarge
            | ir::DiagnosticCode::SemanticInputTooLarge
            | ir::DiagnosticCode::CanonicalizationResourceExhausted,
        ) => Code::ResourceExhausted,
        Some(ir::DiagnosticCode::PotentiallyUndefined) => Code::UndefinedExpression,
        _ => Code::InvalidModelBinding,
    };
    let mut error = failure(source, code, span, message);
    error.upstream = cause.map(Box::new);
    error
}
