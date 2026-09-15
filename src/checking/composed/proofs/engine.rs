// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-owned composed evaluation order feeding the shared proof kernel.

mod graph;
mod queries;
mod roots;
mod walk;

use super::work::{Dimension as D, Work};
use super::*;
use crate::checking::composed::{DeclarationTypes, ObservationOrigin, TypeDisposition};
use crate::checking::proof::{
    self,
    facts::{self, Facts, Outcomes},
    GraphId, Kind, Node, ValueKey,
};
use crate::checking::{NativeType, PresencePremise};
use crate::formal_source::FormalSource;
use crate::linking::composed::{
    binding,
    scopes::{self, Anchor, DeclarationScope},
};
use crate::syntax::{composed as c, BinaryOp, Builtin, ExprId, ExprKind, UnaryOp};
use crate::Span;
use std::collections::BTreeMap;
use std::rc::Rc;

pub(super) enum Error {
    Exhaustion(Exhaustion),
    Cause(Cause),
}
type Result<T> = std::result::Result<T, Error>;
impl From<Exhaustion> for Error {
    fn from(value: Exhaustion) -> Self {
        Self::Exhaustion(value)
    }
}

fn upstream(site: Site) -> Error {
    Error::Cause(Cause {
        site,
        kind: CauseKind::UpstreamType,
    })
}

pub(super) fn discharge<'p, 'r, 'a>(
    types: &'p TypeReport<'r, 'a>,
    bindings: &'p [CheckBindings],
    limits: ProofLimits,
) -> ProofReport<'p, 'r, 'a> {
    let mut work = Work::new(limits);
    let mut report = ProofReport {
        types,
        bindings,
        declarations: Vec::new(),
        exhaustion: None,
        limits: work.limits(),
        usage: work.usage(),
    };
    let run = (|| -> std::result::Result<(), Exhaustion> {
        let namespace = types.binding().namespace();
        let Some(first) = types.binding().declarations().first() else {
            return Ok(());
        };
        let first_id = first.declaration();
        let first_entry = namespace
            .declaration(first_id)
            .expect("retained binding owner");
        let site = Site {
            declaration: first_id,
            unit: first_entry.unit(),
            expression: None,
            span: namespace.syntax(first_id).expect("syntax").span,
        };
        let mappings = correspondence::Mappings::new(types, bindings, &mut work, site)?;
        for bound in types.binding().declarations() {
            let id = bound.declaration();
            let entry = namespace.declaration(id).expect("retained binding owner");
            let syntax = namespace.syntax(id).expect("syntax");
            let site = Site {
                declaration: id,
                unit: entry.unit(),
                expression: None,
                span: syntax.span,
            };
            work.charge(D::Declarations, 1, site)?;
            work.charge(D::Records, 1, site)?;
            report.declarations.push(DeclarationProof {
                declaration: id,
                unit: entry.unit(),
                binding: None,
                environment: None,
                values: Vec::new(),
                goals: Vec::new(),
                causes: Vec::new(),
                local_complete: false,
                complete: false,
            });
            let output = report.declarations.last_mut().expect("inserted result");
            match types.disposition(id) {
                Some(TypeDisposition::Refused) => {
                    work.charge(D::Records, 1, site)?;
                    output.causes.push(Cause {
                        site,
                        kind: CauseKind::UpstreamType,
                    });
                }
                Some(TypeDisposition::Unfinished) | None => {}
                Some(TypeDisposition::Typed) => match mappings.declarations[&id] {
                    Err(reason) => {
                        work.charge(D::Records, 1, site)?;
                        output.causes.push(Cause {
                            site,
                            kind: CauseKind::Correspondence(reason),
                        });
                    }
                    Ok(authored) => {
                        output.binding = Some(authored);
                        let source = &bindings[authored.source];
                        let result = (|| -> Result<()> {
                            let typed = types.declaration(id).ok_or_else(|| upstream(site))?;
                            let mut builder = Builder::new(
                                types.binding(),
                                typed,
                                &source.source,
                                &source.clauses[authored.clause],
                                output,
                                &mut work,
                                site,
                            )?;
                            builder.run(syntax)?;
                            builder.output.local_complete = true;
                            builder.output.complete =
                                builder.output.causes.is_empty() && entry.references().is_empty();
                            Ok(())
                        })();
                        match result {
                            Ok(()) => {}
                            Err(Error::Exhaustion(error)) => return Err(error),
                            Err(Error::Cause(cause)) => {
                                work.charge(D::Records, 1, cause.site)?;
                                output.causes.push(cause);
                            }
                        }
                    }
                },
            }
        }
        dependencies::finish(&mut report, &mut work, site)?;
        Ok(())
    })();
    if let Err(error) = run {
        report.exhaustion = Some(error);
    }
    report.usage = work.usage();
    report
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum Key<'a> {
    // Each builder is private to one original declaration/unit. The selected
    // read context additionally separates pre/post and distinct capture roots.
    Binder(usize, u8),
    Field(ValueKey, &'a ir::RequirementRef, &'a str, String),
    Unwrap(ValueKey),
    Deref(ValueKey),
    Expression(usize),
    Capture(usize),
    Element(usize, ValueKey),
}
struct KeyInfo {
    native: ExprId,
    graph: Option<GraphId>,
    symbol: Option<ir::SymbolName>,
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
impl Path {
    fn empty() -> Self {
        Self {
            facts: Rc::new(Some(BTreeMap::new())),
            guards: Vec::new(),
        }
    }
}
struct Pending {
    native: ExprId,
    graph: GraphId,
    premises: Vec<PresencePremise>,
}

struct Builder<'s, 'a> {
    unit: &'s c::ComposedUnit,
    typed: &'s DeclarationTypes<'a>,
    scope: &'s DeclarationScope,
    formal: &'s FormalSource,
    clause: &'s ClauseBinding,
    output: &'s mut DeclarationProof,
    work: &'s mut Work,
    declaration_site: Site,
    keys: BTreeMap<(usize, Key<'a>), ValueKey>,
    key_info: Vec<KeyInfo>,
    graph: Vec<Node<'a>>,
    widths: Vec<usize>,
    inputs: Vec<ir::ValueDeclaration>,
    pending: Vec<Pending>,
    locals: BTreeMap<usize, Value>,
    binders: BTreeMap<usize, usize>,
    sequences: BTreeMap<ValueKey, queries::Sequence>,
    context: usize,
    next_context: usize,
}
impl<'s, 'a> Builder<'s, 'a> {
    fn new(
        binding: &'s binding::Report<'a>,
        typed: &'s DeclarationTypes<'a>,
        formal: &'s FormalSource,
        clause: &'s ClauseBinding,
        output: &'s mut DeclarationProof,
        work: &'s mut Work,
        site: Site,
    ) -> Result<Self> {
        let unit = binding
            .namespace()
            .unit(typed.unit())
            .ok_or_else(|| upstream(site))?;
        let scope = binding
            .scopes()
            .and_then(|scopes| scopes.declaration(typed.declaration()))
            .ok_or_else(|| upstream(site))?;
        let mut binders = BTreeMap::new();
        for (index, binder) in scope.binders.iter().enumerate() {
            work.charge(D::Types, 1, site)?;
            work.charge(D::Records, 1, site)?;
            binders.insert(binder.span.start, index);
        }
        Ok(Self {
            unit,
            typed,
            scope,
            formal,
            clause,
            output,
            work,
            declaration_site: site,
            keys: BTreeMap::new(),
            key_info: Vec::new(),
            graph: Vec::new(),
            widths: Vec::new(),
            inputs: Vec::new(),
            pending: Vec::new(),
            locals: BTreeMap::new(),
            binders,
            sequences: BTreeMap::new(),
            context: 0,
            next_context: 1,
        })
    }
    fn site(&self, at: ExprId) -> Site {
        self.unit
            .expression(at)
            .map_or(self.declaration_site, |expression| Site {
                declaration: self.output.declaration,
                unit: self.output.unit,
                expression: Some(at),
                span: expression.span,
            })
    }
    fn declaration_site(&self) -> Site {
        self.declaration_site
    }
    fn ty(&self, at: ExprId) -> Result<&'s NativeType<'a>> {
        self.typed
            .node(at)
            .and_then(|node| node.ty.as_ref())
            .ok_or_else(|| upstream(self.site(at)))
    }
    fn unsupported(&self, at: ExprId, kind: Unsupported) -> Error {
        Error::Cause(Cause {
            site: self.site(at),
            kind: CauseKind::Unsupported(kind),
        })
    }
    fn key(&mut self, key: Key<'a>, at: ExprId) -> Result<ValueKey> {
        self.work.charge(D::Types, 1, self.site(at))?;
        let context = match key {
            Key::Expression(_) | Key::Capture(_) | Key::Element(..) => self.context,
            Key::Binder(..) | Key::Field(..) | Key::Unwrap(_) | Key::Deref(_) => 0,
        };
        let key = (context, key);
        if let Some(&key) = self.keys.get(&key) {
            return Ok(key);
        }
        self.work.charge(D::Records, 1, self.site(at))?;
        let id = ValueKey(self.key_info.len());
        self.keys.insert(key, id);
        self.key_info.push(KeyInfo {
            native: at,
            graph: None,
            symbol: None,
        });
        Ok(id)
    }
    fn node(&mut self, kind: Kind<'a>, at: ExprId) -> Result<GraphId> {
        self.work.charge(D::GraphNodes, 1, self.site(at))?;
        self.work.charge(D::Records, 1, self.site(at))?;
        let width = match &kind {
            Kind::Numeric(_, left, right) => {
                self.widths[left.0].saturating_mul(self.widths[right.0])
            }
            Kind::Negate(value) => self.widths[value.0],
            Kind::Unwrap(_) | Kind::Input(_) => 2,
            Kind::Boolean(_)
            | Kind::Integer(..)
            | Kind::Rational(..)
            | Kind::Present(_)
            | Kind::Not(_)
            | Kind::Compare(..)
            | Kind::BooleanOp(..)
            | Kind::Witness(..) => 1,
        };
        let id = GraphId(self.graph.len());
        self.graph.push(Node { kind, native: at });
        self.widths.push(width);
        Ok(id)
    }
    fn expression(
        &mut self,
        graph: GraphId,
        at: ExprId,
        stable: bool,
        outcomes: Outcomes,
    ) -> Result<Value> {
        let key = self.key(Key::Expression(at.0), at)?;
        self.key_info[key.0].graph.get_or_insert(graph);
        Ok(Value {
            key,
            graph,
            stable,
            outcomes: Rc::new(outcomes),
        })
    }
    fn source(&mut self, at: ExprId) -> Result<ir::SourceSpan> {
        self.work.charge(
            D::Bytes,
            2 * self.formal.identity().document().as_str().len(),
            self.site(at),
        )?;
        self.formal
            .to_ir(self.unit.source(), self.site(at).span)
            .map_err(|_| {
                Error::Cause(Cause {
                    site: self.site(at),
                    kind: CauseKind::Correspondence(CorrespondenceError::ForeignSource),
                })
            })
    }
    fn representation(&mut self, ty: &NativeType<'a>, at: ExprId) -> Result<ir::ValueType> {
        self.type_work(ty, at, 1)?;
        match proof::representation(ty, proof::Interpretation::ComposedValues) {
            Ok(value) => Ok(value),
            Err(proof::RepresentationError::Unsupported) => {
                Err(self.unsupported(at, Unsupported::ValueRepresentation))
            }
            Err(proof::RepresentationError::Ir(diagnostic)) => {
                self.work.charge(D::Records, 1, self.site(at))?;
                Err(Error::Cause(Cause {
                    site: self.site(at),
                    kind: CauseKind::Unproved {
                        diagnostics: vec![diagnostic],
                    },
                }))
            }
        }
    }
    fn type_work(&mut self, ty: &NativeType<'a>, at: ExprId, depth: usize) -> Result<()> {
        self.work.charge(D::Depth, depth, self.site(at))?;
        self.work.charge(D::Types, 1, self.site(at))?;
        match ty {
            NativeType::Option(value) => self.type_work(value, at, depth + 1)?,
            NativeType::Sequence { element, .. } => self.type_work(element, at, depth + 1)?,
            NativeType::Boolean
            | NativeType::Scalar { .. }
            | NativeType::Enumeration { .. }
            | NativeType::Record { .. }
            | NativeType::Object { .. }
            | NativeType::Reference { .. } => {}
        }
        Ok(())
    }
    fn symbolic(
        &mut self,
        key: Key<'a>,
        at: ExprId,
        ty: &NativeType<'a>,
        stable: bool,
    ) -> Result<Value> {
        let key = self.key(key, at)?;
        if let Some(graph) = self.key_info[key.0].graph {
            return Ok(Value {
                key,
                graph,
                stable,
                outcomes: Rc::new(Outcomes::unknown()),
            });
        }
        self.work.charge(D::Values, 1, self.site(at))?;
        self.work.charge(D::Records, 2, self.site(at))?;
        // Clause IDs are supplied, validated identifiers and globally unique
        // under the selected requirement. They separate upstream symbolic value
        // identities even when different clauses share one requirement owner.
        self.work.charge(
            D::Bytes,
            32 + self.clause.clause.as_str().len(),
            self.site(at),
        )?;
        let symbol = ir::SymbolName::new(format!(
            "proof.{}.value{}",
            self.clause.clause.as_str(),
            key.0
        ))
        .map_err(|_| self.unsupported(at, Unsupported::ValueRepresentation))?;
        let source = self.source(at)?;
        let representation = self.representation(ty, at)?;
        self.work.charge(
            D::Bytes,
            2 * symbol.as_str().len() + 2 * self.formal.identity().document().as_str().len(),
            self.site(at),
        )?;
        self.inputs.push(ir::ValueDeclaration::new(
            symbol.clone(),
            ir::ValueDeclarationKind::Input,
            representation,
            source.clone(),
        ));
        self.output.values.push(ProofValue {
            symbol: symbol.clone(),
            expression: at,
            source,
            declarations: Vec::new(),
        });
        self.key_info[key.0].symbol = Some(symbol);
        let graph = self.node(Kind::Input(key), at)?;
        self.key_info[key.0].graph = Some(graph);
        Ok(Value {
            key,
            graph,
            stable,
            outcomes: Rc::new(Outcomes::unknown()),
        })
    }
    fn assume(&mut self, path: &Path, value: &Value, truth: bool, at: ExprId) -> Result<Path> {
        let span = self.site(at).span;
        let facts = facts::sequential(&path.facts, value.outcomes.selected(truth), self, span)?;
        self.work
            .charge(D::Records, path.guards.len() + 1, self.site(at))?;
        let mut guards = path.guards.clone();
        guards.push(Guard {
            graph: value.graph,
            truth,
            native: at,
        });
        Ok(Path {
            facts: Rc::new(facts),
            guards,
        })
    }
    fn protect(
        &mut self,
        mut graph: GraphId,
        facts: &Facts,
        at: ExprId,
        op: ir::BooleanOperator,
    ) -> Result<GraphId> {
        if let Some(facts) = facts {
            self.work.charge(D::Facts, facts.len(), self.site(at))?;
            for (&key, fact) in facts.iter().rev() {
                let input = self.key_info[key.0].graph.expect("presence value");
                let mut premise = self.node(Kind::Present(input), fact.guard)?;
                if !fact.present {
                    premise = self.node(Kind::Not(premise), fact.guard)?;
                }
                graph = self.node(Kind::BooleanOp(op, premise, graph), at)?;
            }
        }
        Ok(graph)
    }
    fn goal(&mut self, at: ExprId, graph: GraphId, path: &Path) -> Result<()> {
        if path.facts.is_none() {
            return Ok(());
        }
        self.work.charge(D::Goals, 1, self.site(at))?;
        self.work.charge(D::Records, 1, self.site(at))?;
        let representation = self.representation(self.ty(at)?, at)?;
        let mut graph = self.node(Kind::Witness(graph, representation), at)?;
        graph = self.protect(graph, &path.facts, at, ir::BooleanOperator::Implication)?;
        self.work
            .charge(D::Facts, path.guards.len(), self.site(at))?;
        for guard in path.guards.iter().rev() {
            let condition = if guard.truth {
                guard.graph
            } else {
                self.node(Kind::Not(guard.graph), guard.native)?
            };
            graph = self.node(
                Kind::BooleanOp(ir::BooleanOperator::Implication, condition, graph),
                at,
            )?;
        }
        let mut premises = Vec::new();
        if let Some(facts) = path.facts.as_ref() {
            self.work.charge(D::Facts, facts.len(), self.site(at))?;
            self.work.charge(D::Records, facts.len(), self.site(at))?;
            for (&key, fact) in facts {
                premises.push(PresencePremise {
                    guard: fact.guard,
                    outcome: fact.outcome,
                    value: self.key_info[key.0].native,
                    present: fact.present,
                });
            }
        }
        self.pending.push(Pending {
            native: at,
            graph,
            premises,
        });
        Ok(())
    }
    fn run(&mut self, syntax: &c::Declaration) -> Result<()> {
        let clause = self.clause;
        self.roots(syntax)?;
        let site = self.declaration_site();
        self.work.charge(D::Records, 1, site)?;
        self.work.charge(D::Types, self.inputs.len(), site)?;
        self.work.charge(
            D::Bytes,
            clause.requirement.package().as_str().len()
                + clause.requirement.requirement().as_str().len(),
            site,
        )?;
        let environment = ir::DeclarationEnvironment::new(
            clause.requirement.clone(),
            Vec::new(),
            std::mem::take(&mut self.inputs),
            Vec::new(),
        )
        .map_err(|diagnostics| {
            Error::Cause(Cause {
                site,
                kind: CauseKind::Unproved { diagnostics },
            })
        })?;
        self.output.environment = Some(environment);
        for goal in std::mem::take(&mut self.pending) {
            let expression = proof::graph::materialize(self, goal.graph, 1, &mut 0)?;
            self.work.charge(D::Records, 1, self.site(goal.native))?;
            let checked = self
                .output
                .environment
                .as_ref()
                .expect("actual environment")
                .check_expression(
                    &expression,
                    &ir::ValueType::Boolean,
                    &clause.execution_point,
                    true,
                );
            match checked {
                Ok(checked) => {
                    let source = self.source(goal.native)?;
                    self.output.goals.push(ProofGoal {
                        native: goal.native,
                        source,
                        checked,
                        premises: goal.premises,
                    });
                }
                Err(diagnostics) => {
                    self.work
                        .charge(D::Records, diagnostics.len(), self.site(goal.native))?;
                    self.output.causes.push(Cause {
                        site: self.site(goal.native),
                        kind: CauseKind::Unproved { diagnostics },
                    });
                }
            }
        }
        Ok(())
    }
}

impl facts::FactMeter for Builder<'_, '_> {
    type Error = Error;
    fn facts(&mut self, count: usize, span: Span) -> Result<()> {
        let mut site = self.declaration_site();
        site.span = span;
        self.work.charge(D::Facts, count, site)?;
        self.work.charge(D::Records, count, site)?;
        Ok(())
    }
}
