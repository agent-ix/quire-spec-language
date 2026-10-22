// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Behavior, ADR-018 UE-1, UE-4, UE-6: a checked clause body read as a
//! linear term and the condition under which it is defined.
//!
//! The encoding reads undefinedness as a refutation, never as `false` and
//! never as a disabled step, so every expression is encoded as an
//! [`Encoded`]: its value and its definedness condition (a divisor not zero,
//! a `value` of a present option, a value inside its sort's range). A caller
//! that asserts the negated property includes the negation of the
//! definedness condition of every atom it reads and of every guard a step
//! reads.

use qsl_semantics::check::{
    Arithmetic, CheckedLiteral, CheckedNode, Connective, IntegerDivisionOperator, NodeKind,
    OrderedKind,
};
use qsl_semantics::model::clause_observation::Observation;
use quire_exact::{Integer, OrderingOperator, Value, ValueType};
use quire_semantic_value::declaration::EqualityOperator;
use quire_semantic_value::location::Location;
use std::ops::Not;

use crate::simulation::model::SymbolicSubject;
use crate::smt::layout::{FieldSort, StateCopy, StateLayout};
use crate::smt::refusal::ExpressionLocus;
use crate::smt::script::{Comparison, Term};
use crate::smt::SmtEncodingRefusal;

/// The deepest expression nesting the encoding reads; a deeper body refuses
/// rather than recursing.
const MAX_DEPTH: usize = 256;

/// What a sub-expression denotes.
#[derive(Clone, Debug)]
pub enum Sym {
    /// A Boolean term.
    Bool(Term),
    /// An integer or enumeration-rank term.
    Int(Term),
    /// An option: its presence and, where present, its payload.
    Option {
        /// Whether a payload is present.
        present: Term,
        /// The payload.
        value: Box<Sym>,
    },
    /// A universe object, by its index in the layout.
    Object(usize),
}

/// An expression's value and the condition under which it is defined.
#[derive(Clone, Debug)]
pub struct Encoded {
    /// What the expression denotes where it is defined.
    pub sym: Sym,
    /// The condition under which it is defined.
    pub defined: Term,
}

impl Encoded {
    fn total(sym: Sym) -> Self {
        Self {
            sym,
            defined: Term::truth(true),
        }
    }
}

/// The state copies a body's reads observe (FR-104): `pre` and `post` are the
/// operation's, `current` the state an invariant or an atom reads.
#[derive(Clone, Debug)]
pub struct Views {
    /// The copy `Observation::Pre` reads.
    pub pre: StateCopy,
    /// The copy `Observation::Post` reads.
    pub post: StateCopy,
    /// The copy `Observation::Current` reads.
    pub current: StateCopy,
}

impl Views {
    fn copy(&self, observation: Observation) -> &StateCopy {
        match observation {
            Observation::Pre => &self.pre,
            Observation::Post => &self.post,
            Observation::Current => &self.current,
        }
    }
}

/// Reads checked expressions of one clause or atom over one set of views.
pub struct Reader<'r, 'a> {
    layout: &'r StateLayout,
    subject: &'r SymbolicSubject<'a>,
    clause: &'r str,
    default: Observation,
    reads: Vec<(&'r Location, Observation)>,
    views: &'r Views,
    slots: Vec<Option<Sym>>,
}

impl<'r, 'a> Reader<'r, 'a> {
    /// A reader of the body of the clause `clause`, whose model reads observe
    /// `reads` and otherwise `default`, with `slots` bound in slot order.
    pub fn new(
        layout: &'r StateLayout,
        subject: &'r SymbolicSubject<'a>,
        clause: &'r str,
        default: Observation,
        reads: Vec<(&'r Location, Observation)>,
        views: &'r Views,
        slots: Vec<Option<Sym>>,
    ) -> Self {
        Self {
            layout,
            subject,
            clause,
            default,
            reads,
            views,
            slots,
        }
    }

    /// The Boolean `node` denotes and its definedness.
    ///
    /// # Errors
    ///
    /// [`SmtEncodingRefusal::Expression`] naming the first construct outside
    /// the fragment.
    pub fn boolean(&mut self, node: CheckedNode<'_>) -> Result<(Term, Term), SmtEncodingRefusal> {
        let encoded = self.encode(node, 0)?;
        match encoded.sym {
            Sym::Bool(term) => Ok((term, encoded.defined)),
            _ => Err(self.refuse(node, "non-Boolean clause")),
        }
    }

    fn refuse(&self, node: CheckedNode<'_>, form: &'static str) -> SmtEncodingRefusal {
        SmtEncodingRefusal::Expression {
            form,
            locus: ExpressionLocus {
                clause: self.clause.to_owned(),
                node: node.id().index(),
            },
        }
    }

    fn observation_of(&self, location: &Location) -> Observation {
        self.reads
            .iter()
            .find(|(read, _)| *read == location)
            .map_or(self.default, |(_, observation)| *observation)
    }

    fn operand(
        &mut self,
        parent: CheckedNode<'_>,
        id: qsl_semantics::check::NodeId,
        depth: usize,
    ) -> Result<Encoded, SmtEncodingRefusal> {
        self.encode(parent.at(id), depth + 1)
    }

    fn integer(
        &mut self,
        parent: CheckedNode<'_>,
        id: qsl_semantics::check::NodeId,
        depth: usize,
    ) -> Result<(Term, Term), SmtEncodingRefusal> {
        let encoded = self.operand(parent, id, depth)?;
        match encoded.sym {
            Sym::Int(term) => Ok((term, encoded.defined)),
            _ => Err(self.refuse(parent.at(id), "non-integer operand")),
        }
    }

    fn boolean_operand(
        &mut self,
        parent: CheckedNode<'_>,
        id: qsl_semantics::check::NodeId,
        depth: usize,
    ) -> Result<(Term, Term), SmtEncodingRefusal> {
        let encoded = self.operand(parent, id, depth)?;
        match encoded.sym {
            Sym::Bool(term) => Ok((term, encoded.defined)),
            _ => Err(self.refuse(parent.at(id), "non-Boolean operand")),
        }
    }

    fn encode(
        &mut self,
        node: CheckedNode<'_>,
        depth: usize,
    ) -> Result<Encoded, SmtEncodingRefusal> {
        if depth > MAX_DEPTH {
            return Err(self.refuse(node, "expression nested too deeply"));
        }
        match node.kind() {
            NodeKind::Literal(CheckedLiteral(value)) => self.literal(node, value),
            NodeKind::Local(slot) => self
                .slots
                .get(*slot)
                .cloned()
                .flatten()
                .map(Encoded::total)
                .ok_or_else(|| self.refuse(node, "binding with no value")),
            NodeKind::Let { slot, value, body } => {
                let bound = self.operand(node, *value, depth)?;
                if self.slots.len() <= *slot {
                    self.slots.resize(*slot + 1, None);
                }
                let previous = self.slots[*slot].replace(bound.sym);
                let inner = self.operand(node, *body, depth);
                self.slots[*slot] = previous;
                let inner = inner?;
                Ok(Encoded {
                    sym: inner.sym,
                    defined: Term::all([bound.defined, inner.defined]),
                })
            }
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => {
                let (test, test_defined) = self.boolean_operand(node, *condition, depth)?;
                let taken = self.operand(node, *then, depth)?;
                let other = self.operand(node, *otherwise, depth)?;
                let defined = Term::all([
                    test_defined,
                    test.clone().select(taken.defined, other.defined),
                ]);
                let sym = match (taken.sym, other.sym) {
                    (Sym::Bool(left), Sym::Bool(right)) => Sym::Bool(test.select(left, right)),
                    (Sym::Int(left), Sym::Int(right)) => Sym::Int(test.select(left, right)),
                    _ => return Err(self.refuse(node, "conditional of options or objects")),
                };
                Ok(Encoded { sym, defined })
            }
            NodeKind::Coerce(operand, interval) => {
                let (term, defined) = self.integer(node, *operand, depth)?;
                let inside = match node.at(*operand).value_type() {
                    ValueType::Int(source) => {
                        interval.contains(source.lower()) && interval.contains(source.upper())
                    }
                    _ => false,
                };
                let range = if inside {
                    Term::truth(true)
                } else {
                    term.clone().within(interval.lower(), interval.upper())
                };
                Ok(Encoded {
                    sym: Sym::Int(term),
                    defined: Term::all([defined, range]),
                })
            }
            NodeKind::Arithmetic(operator, left, right) => {
                let (left, left_defined) = self.integer(node, *left, depth)?;
                let (right, right_defined) = self.integer(node, *right, depth)?;
                let term = match operator {
                    Arithmetic::Add => left.plus(right),
                    Arithmetic::Subtract => left.minus(right),
                    Arithmetic::Multiply => left
                        .times(right)
                        .ok_or_else(|| self.refuse(node, "nonlinear multiplication"))?,
                };
                Ok(Encoded {
                    sym: Sym::Int(term),
                    defined: Term::all([left_defined, right_defined]),
                })
            }
            NodeKind::Negate(operand) => {
                let (term, defined) = self.integer(node, *operand, depth)?;
                Ok(Encoded {
                    sym: Sym::Int(term.negated()),
                    defined,
                })
            }
            NodeKind::IntegerDivision {
                operator: IntegerDivisionOperator::Mod,
                left,
                right,
                ..
            } => {
                let (dividend, dividend_defined) = self.integer(node, *left, depth)?;
                let (divisor, divisor_defined) = self.integer(node, *right, depth)?;
                let Some(divisor) = divisor.as_literal().cloned() else {
                    return Err(self.refuse(node, "modulus by a non-literal"));
                };
                let nonzero = !divisor.is_zero();
                let term = if nonzero {
                    dividend.modulo(divisor)
                } else {
                    Term::int(0)
                };
                Ok(Encoded {
                    sym: Sym::Int(term),
                    defined: Term::all([dividend_defined, divisor_defined, Term::truth(nonzero)]),
                })
            }
            NodeKind::Order(operator, kind, left, right) => {
                if !matches!(kind, OrderedKind::Integers | OrderedKind::Enums) {
                    return Err(self.refuse(node, "ordering of non-integers"));
                }
                let (left, left_defined) = self.integer(node, *left, depth)?;
                let (right, right_defined) = self.integer(node, *right, depth)?;
                let comparison = match operator {
                    OrderingOperator::Less => Comparison::Less,
                    OrderingOperator::LessOrEqual => Comparison::LessOrEqual,
                    OrderingOperator::Greater => Comparison::Greater,
                    OrderingOperator::GreaterOrEqual => Comparison::GreaterOrEqual,
                };
                Ok(Encoded {
                    sym: Sym::Bool(left.compare(comparison, right)),
                    defined: Term::all([left_defined, right_defined]),
                })
            }
            NodeKind::Equality(operator, _, left, right) => {
                let left = self.operand(node, *left, depth)?;
                let right = self.operand(node, *right, depth)?;
                let same = match (left.sym, right.sym) {
                    (Sym::Bool(left), Sym::Bool(right)) | (Sym::Int(left), Sym::Int(right)) => {
                        left.equal(right)
                    }
                    (Sym::Object(left), Sym::Object(right)) => Term::truth(left == right),
                    _ => return Err(self.refuse(node, "equality of options or mixed sorts")),
                };
                let value = match operator {
                    EqualityOperator::Equal => same,
                    EqualityOperator::NotEqual => same.not(),
                };
                Ok(Encoded {
                    sym: Sym::Bool(value),
                    defined: Term::all([left.defined, right.defined]),
                })
            }
            NodeKind::Connective(connective, left, right) => {
                let (left, left_defined) = self.boolean_operand(node, *left, depth)?;
                let (right, right_defined) = self.boolean_operand(node, *right, depth)?;
                // The right operand is skipped, so it need only be defined
                // where the left operand does not decide the connective.
                let (value, reached) = match connective {
                    Connective::And => (Term::all([left.clone(), right]), left.clone()),
                    Connective::Or => (Term::any([left.clone(), right]), left.clone().not()),
                    Connective::Implies => (Term::any([left.clone().not(), right]), left.clone()),
                };
                Ok(Encoded {
                    sym: Sym::Bool(value),
                    defined: Term::all([left_defined, reached.implies(right_defined)]),
                })
            }
            NodeKind::Not(operand) => {
                let (term, defined) = self.boolean_operand(node, *operand, depth)?;
                Ok(Encoded {
                    sym: Sym::Bool(term.not()),
                    defined,
                })
            }
            NodeKind::Present(operand) => {
                let encoded = self.operand(node, *operand, depth)?;
                match encoded.sym {
                    Sym::Option { present, .. } => Ok(Encoded {
                        sym: Sym::Bool(present),
                        defined: encoded.defined,
                    }),
                    _ => Err(self.refuse(node, "presence of a non-option")),
                }
            }
            NodeKind::Value(operand) => {
                let encoded = self.operand(node, *operand, depth)?;
                match encoded.sym {
                    Sym::Option { present, value } => Ok(Encoded {
                        sym: *value,
                        defined: Term::all([encoded.defined, present]),
                    }),
                    _ => Err(self.refuse(node, "value of a non-option")),
                }
            }
            NodeKind::Pre(operand) => self.operand(node, *operand, depth),
            NodeKind::Attribute {
                reference,
                field,
                optional,
                ..
            } => self.attribute(node, *reference, &field.name, *optional, depth),
            other => Err(self.refuse(node, form_name(other))),
        }
    }

    fn literal(&self, node: CheckedNode<'_>, value: &Value) -> Result<Encoded, SmtEncodingRefusal> {
        let sym = match value {
            Value::Boolean(value) => Sym::Bool(Term::truth(*value)),
            Value::Integer(value) => Sym::Int(Term::Int(value.clone())),
            Value::Enum(member) => Sym::Int(Term::Int(Integer::from(u64::from(member.rank())))),
            Value::Reference(reference) => Sym::Object(
                self.subject
                    .objects
                    .iter()
                    .position(|object| object.reference == *reference)
                    .ok_or_else(|| {
                        self.refuse(node, "reference to an object outside the universe")
                    })?,
            ),
            _ => return Err(self.refuse(node, "literal of an unsupported sort")),
        };
        Ok(Encoded::total(sym))
    }

    fn attribute(
        &mut self,
        node: CheckedNode<'_>,
        reference: qsl_semantics::check::NodeId,
        name: &str,
        optional: bool,
        depth: usize,
    ) -> Result<Encoded, SmtEncodingRefusal> {
        let holder = self.operand(node, reference, depth)?;
        let Sym::Object(object) = holder.sym else {
            return Err(self.refuse(node, "attribute of a computed reference"));
        };
        let observation = self.observation_of(node.location());
        let copy = self.views.copy(observation);
        let layout = self
            .layout
            .objects()
            .get(object)
            .ok_or_else(|| self.refuse(node, "object outside the layout"))?;
        let index = layout
            .fields()
            .iter()
            .position(|field| field.name() == name)
            .ok_or_else(|| self.refuse(node, "field outside the layout"))?;
        let terms = self
            .layout
            .terms(copy, object, index)
            .ok_or_else(|| self.refuse(node, "field outside the layout"))?;
        let value = match layout.fields()[index].sort() {
            FieldSort::Bool => Sym::Bool(terms.value),
            FieldSort::Int(_) => Sym::Int(terms.value),
        };
        let sym = match (optional, terms.present) {
            (false, _) => value,
            (true, Some(present)) => Sym::Option {
                present,
                value: Box::new(value),
            },
            (true, None) => return Err(self.refuse(node, "optional read of a required field")),
        };
        Ok(Encoded {
            sym,
            defined: holder.defined,
        })
    }
}

/// The name a refusal gives the construct `kind`.
fn form_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::IntegerDivision { .. } => "integer division",
        NodeKind::Divide { .. } => "division",
        NodeKind::Rational { .. } | NodeKind::RationalNegate(..) => "rational arithmetic",
        NodeKind::Decimal { .. } | NodeKind::DecimalNegate(..) | NodeKind::ConvertDecimal(..) => {
            "decimal arithmetic"
        }
        NodeKind::Ieee(..) | NodeKind::IeeeToRational(..) => "floating-point arithmetic",
        NodeKind::Quantity(..) => "quantity arithmetic",
        NodeKind::Field { .. } | NodeKind::Record { .. } | NodeKind::Tuple { .. } => {
            "record or tuple"
        }
        NodeKind::Case { .. } | NodeKind::UnionValue { .. } => "union",
        NodeKind::Navigate { .. } | NodeKind::Reaches { .. } | NodeKind::ReachesEnd { .. } => {
            "relationship navigation"
        }
        NodeKind::Call { .. } | NodeKind::ImportedCall { .. } | NodeKind::Dispatch { .. } => "call",
        NodeKind::Collection { .. }
        | NodeKind::ConvertCollection { .. }
        | NodeKind::Query { .. }
        | NodeKind::Flatten(..)
        | NodeKind::Fold { .. }
        | NodeKind::Size(..)
        | NodeKind::Contains(..)
        | NodeKind::Seq { .. } => "collection",
        NodeKind::AllInstances { .. }
        | NodeKind::Lookup { .. }
        | NodeKind::NamedPopulation { .. }
        | NodeKind::PopulationObjects { .. } => "population",
        NodeKind::ConvertScalar(..) => "scalar conversion",
        NodeKind::ProtocolReference { .. } => "protocol reference",
        NodeKind::History { .. } => "history field",
        _ => "expression",
    }
}
