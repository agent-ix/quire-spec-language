// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Behavior: the step relation of one subject between two state
//! copies, as the disjuncts of its transition assertion.
//!
//! A disjunct is one operation applied to one receiver and one argument
//! vector: the operation's preconditions on the `from` copy, its
//! postconditions relating `from` and `to`, and its frame. They come in
//! ascending order of receiver key, then operation name, then argument
//! vector in FR-120's domain order, and the terminal stutter, which holds
//! only where no operation is enabled, comes last. A product of subjects
//! (FR-315's self-composition queries) takes one component disjunct from
//! this relation for each of its subjects, in this order.

use qsl_semantics::check::CheckedStateClause;
use quire_exact::Value;
use std::ops::Not;

use crate::simulation::model::{SymbolicObject, SymbolicOperation, SymbolicSubject};
use crate::smt::expression::{Reader, Sym, Views};
use crate::smt::layout::{StateCopy, StateLayout};
use crate::smt::script::Term;
use crate::smt::SmtEncodingRefusal;

/// What a step is: the application it takes, or the terminal stutter.
#[derive(Clone, Debug)]
pub enum StepLabel {
    /// An operation applied to a receiver.
    Operation {
        /// The receiver's key.
        receiver: String,
        /// The operation's declared name.
        operation: String,
        /// One value per declared parameter.
        arguments: Vec<Value>,
    },
    /// The terminal stutter (ADR-018 SM-4).
    Stutter,
}

/// One disjunct of the step relation.
#[derive(Clone, Debug)]
pub struct StepDisjunct {
    /// The step the disjunct is.
    pub label: StepLabel,
    /// The disjunct: preconditions, postconditions and frame (or, for the
    /// stutter, no enabled operation and every field unchanged).
    pub holds: Term,
    /// The conditions under which a precondition guard of the application is
    /// undefined on the `from` copy.
    pub guard_failure: Term,
    /// The conditions under which the application is enabled and a
    /// postcondition is undefined between the two copies.
    pub post_failure: Term,
}

/// The step relation of one subject between two copies.
#[derive(Clone, Debug)]
pub struct StepRelation {
    disjuncts: Vec<StepDisjunct>,
}

impl StepRelation {
    /// The disjuncts: the operations, then the terminal stutter when
    /// `stutter` (the infinite-trace profile).
    ///
    /// # Errors
    ///
    /// [`SmtEncodingRefusal`] naming the first construct of an applicable
    /// clause outside the fragment, or the first parameter with no finite
    /// domain.
    pub(crate) fn build(
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        from: &StateCopy,
        to: &StateCopy,
        stutter: bool,
    ) -> Result<Self, SmtEncodingRefusal> {
        let mut disjuncts = Vec::new();
        let mut enabled = Vec::new();
        for (receiver, object) in subject.objects.iter().enumerate() {
            for operation in &object.operations {
                for arguments in argument_vectors(operation)? {
                    let step = Application {
                        subject,
                        layout,
                        from,
                        to,
                        receiver,
                        object,
                        operation,
                        arguments: &arguments,
                    };
                    let (disjunct, guard) = step.disjunct()?;
                    enabled.push(guard);
                    disjuncts.push(disjunct);
                }
            }
        }
        if stutter {
            let mut conjuncts: Vec<Term> = enabled.into_iter().map(Term::not).collect();
            for (object, layout_object) in layout.objects().iter().enumerate() {
                for field in 0..layout_object.fields().len() {
                    conjuncts.extend(layout.unchanged(from, to, object, field));
                }
            }
            disjuncts.push(StepDisjunct {
                label: StepLabel::Stutter,
                holds: Term::all(conjuncts),
                guard_failure: Term::truth(false),
                post_failure: Term::truth(false),
            });
        }
        Ok(Self { disjuncts })
    }

    /// The disjuncts in order.
    pub fn disjuncts(&self) -> &[StepDisjunct] {
        &self.disjuncts
    }

    /// The transition assertion: some disjunct holds.
    pub fn transition(&self) -> Term {
        Term::any(self.disjuncts.iter().map(|disjunct| disjunct.holds.clone()))
    }

    /// Some precondition guard is undefined on the `from` copy.
    pub fn guard_failure(&self) -> Term {
        Term::any(
            self.disjuncts
                .iter()
                .map(|disjunct| disjunct.guard_failure.clone()),
        )
    }

    /// Some enabled application has an undefined postcondition.
    pub fn post_failure(&self) -> Term {
        Term::any(
            self.disjuncts
                .iter()
                .map(|disjunct| disjunct.post_failure.clone()),
        )
    }
}

/// Every argument vector of `operation`, in FR-120's order: the first
/// parameter most significant, each parameter's values in domain order.
fn argument_vectors(
    operation: &SymbolicOperation<'_>,
) -> Result<Vec<Vec<Value>>, SmtEncodingRefusal> {
    let mut vectors: Vec<Vec<Value>> = vec![Vec::new()];
    for ((name, _), domain) in operation.parameters.iter().zip(&operation.domains) {
        let values = domain
            .as_ref()
            .ok_or_else(|| SmtEncodingRefusal::OperationParameter {
                operation: operation.name.clone(),
                parameter: name.clone(),
            })?;
        vectors = vectors
            .into_iter()
            .flat_map(|prefix| {
                values.iter().map(move |value| {
                    let mut extended = prefix.clone();
                    extended.push(value.clone());
                    extended
                })
            })
            .collect();
    }
    Ok(vectors)
}

/// One operation applied to one receiver and one argument vector.
struct Application<'s, 'a> {
    subject: &'s SymbolicSubject<'a>,
    layout: &'s StateLayout,
    from: &'s StateCopy,
    to: &'s StateCopy,
    receiver: usize,
    object: &'s SymbolicObject<'a>,
    operation: &'s SymbolicOperation<'a>,
    arguments: &'s [Value],
}

impl Application<'_, '_> {
    /// The disjunct and the condition under which the application is
    /// enabled on the `from` copy.
    fn disjunct(&self) -> Result<(StepDisjunct, Term), SmtEncodingRefusal> {
        let before = Views {
            pre: self.from.clone(),
            post: self.from.clone(),
            current: self.from.clone(),
        };
        let across = Views {
            pre: self.from.clone(),
            post: self.to.clone(),
            current: self.to.clone(),
        };
        let mut guard_defined = Vec::new();
        let mut guards = Vec::new();
        for clause in &self.operation.preconditions {
            let (value, defined) = self.read(clause, &before)?;
            guards.push(Term::all([defined.clone(), value]));
            guard_defined.push(defined);
        }
        let mut post_defined = Vec::new();
        let mut posts = Vec::new();
        for clause in &self.operation.postconditions {
            let (value, defined) = self.read(clause, &across)?;
            posts.push(Term::all([defined.clone(), value]));
            post_defined.push(defined);
        }
        let frame = self.frame();
        let enabled = Term::all(guards);
        let holds = Term::all(
            std::iter::once(enabled.clone())
                .chain(posts)
                .chain(std::iter::once(Term::all(frame.clone()))),
        );
        let post_failure = Term::all([
            enabled.clone(),
            Term::all(frame),
            Term::all(post_defined).not(),
        ]);
        let disjunct = StepDisjunct {
            label: StepLabel::Operation {
                receiver: self.object.key.clone(),
                operation: self.operation.name.clone(),
                arguments: self.arguments.to_vec(),
            },
            holds,
            guard_failure: Term::all(guard_defined).not(),
            post_failure,
        };
        Ok((disjunct, enabled))
    }

    /// The clause's body as a Boolean and its definedness, with `self`, the
    /// parameters and the arguments bound.
    fn read(
        &self,
        clause: &CheckedStateClause,
        views: &Views,
    ) -> Result<(Term, Term), SmtEncodingRefusal> {
        let mut slots = vec![Some(Sym::Object(self.receiver))];
        let mut declared = clause.parameters().iter().skip(1);
        if clause.binds_result() {
            declared.next();
            slots.push(None);
        }
        for (name, _) in declared {
            let value = self
                .operation
                .parameters
                .iter()
                .position(|(parameter, _)| parameter == name)
                .and_then(|position| self.arguments.get(position))
                .and_then(|value| self.symbol_of(value));
            slots.push(value);
        }
        let reads = clause.reads().collect();
        Reader::new(
            self.layout,
            self.subject,
            clause.name(),
            clause.observation(),
            reads,
            views,
            slots,
        )
        .boolean(clause.body())
    }

    fn symbol_of(&self, value: &Value) -> Option<Sym> {
        match value {
            Value::Boolean(value) => Some(Sym::Bool(Term::truth(*value))),
            Value::Integer(value) => Some(Sym::Int(Term::Int(value.clone()))),
            Value::Enum(member) => Some(Sym::Int(Term::int(i64::from(member.rank())))),
            Value::Reference(reference) => self
                .subject
                .objects
                .iter()
                .position(|object| object.reference == *reference)
                .map(Sym::Object),
            _ => None,
        }
    }

    /// Every field the frame does not let the operation write is unchanged.
    fn frame(&self) -> Vec<Term> {
        let mut frame = Vec::new();
        for (index, object) in self.subject.objects.iter().enumerate() {
            for (field_index, field) in object.fields.iter().enumerate() {
                let written = self.operation.modifies.contains(&field.key)
                    || (index == self.receiver
                        && self.operation.receiver_modifies.contains(&field.key));
                if !written {
                    frame.extend(
                        self.layout
                            .unchanged(self.from, self.to, index, field_index),
                    );
                }
            }
        }
        frame
    }
}
