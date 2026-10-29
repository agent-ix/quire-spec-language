// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Behavior: the step relation of one subject between two state
//! copies, and the facts about one copy that the relation reads.
//!
//! An application is one operation applied to one receiver and one argument
//! vector, in ascending order of receiver key, then operation name, then
//! argument vector in FR-120's domain order. Its disjunct conjoins the
//! operation's precondition clauses on the `from` copy, its postcondition
//! clauses relating `from` and `to`, and its frame, each clause printed as its
//! definedness condition conjoined with its value. Under infinite-trace the
//! proof-query stutter, which equates every field of the two copies and has no
//! other conjunct, comes last.
//!
//! The facts about a copy are read from that copy and literals alone: each
//! application's precondition and postcondition failures (ADR-018 UE-6) and,
//! when a query needs it, whether the copy is terminal, by FR-120's candidate
//! post-states expanded as literals.

use std::ops::Not;
use std::rc::Rc;

use qsl_semantics::check::CheckedStateClause;
use quire_exact::Value;
use quire_semantic_value::model_transition::ModelTransition;

use crate::simulation::model::{ParameterValues, ResultValues, SymbolicOperation, SymbolicSubject};
use crate::smt::expression::{Reader, Sym, Views};
use crate::smt::layout::{value_term, FieldSort, FieldTerms, StateCopy, StateLayout, StateRef};
use crate::smt::script::Term;
use crate::smt::{Measure, Refusal, SmtEncodingRefusal};

/// One application: a receiver, an operation of it and an argument vector.
pub struct Application<'s, 'a> {
    receiver: usize,
    operation: &'s SymbolicOperation<'a>,
    arguments: Vec<Value>,
}

/// Refuse `parameter-domain` for the first parameter, in object, operation
/// and parameter order, or declared result, that has no finite domain.
///
/// # Errors
///
/// [`Refusal::ParameterDomain`] naming the operation and the
/// parameter, or `result`.
pub fn check_domains(subject: &SymbolicSubject<'_>) -> Result<(), SmtEncodingRefusal> {
    for object in &subject.objects {
        for operation in &object.operations {
            for ((name, _), domain) in operation.parameters.iter().zip(&operation.domains) {
                if matches!(domain, ParameterValues::Unbounded) {
                    return Err(Refusal::ParameterDomain {
                        operation: operation.name.clone(),
                        parameter: name.clone(),
                    }
                    .into());
                }
            }
            if let ResultValues::Declared(
                ParameterValues::Unbounded | ParameterValues::TooMany(_),
            ) = &operation.result
            {
                return Err(Refusal::ParameterDomain {
                    operation: operation.name.clone(),
                    parameter: "result".to_owned(),
                }
                .into());
            }
        }
    }
    Ok(())
}

/// The applications of one step: receivers times operations times argument
/// vectors, summed over the operations. Saturates.
pub fn count_applications(subject: &SymbolicSubject<'_>) -> u64 {
    let mut total: u64 = 0;
    for object in &subject.objects {
        for operation in &object.operations {
            let mut count: u64 = 1;
            for domain in &operation.domains {
                let size = match domain {
                    ParameterValues::Values(values) => {
                        u64::try_from(values.len()).unwrap_or(u64::MAX)
                    }
                    ParameterValues::TooMany(size) => u64::try_from(*size).unwrap_or(u64::MAX),
                    ParameterValues::Unbounded => 0,
                };
                count = count.saturating_mul(size);
            }
            total = total.saturating_add(count);
        }
    }
    total
}

/// Every application of the subject, in step-disjunct order.
///
/// # Errors
///
/// [`Refusal::TooLarge`] for `applications`, before any vector is
/// built.
pub fn applications<'s, 'a>(
    subject: &'s SymbolicSubject<'a>,
    max_applications: u64,
) -> Result<Vec<Application<'s, 'a>>, SmtEncodingRefusal> {
    let total = count_applications(subject);
    if total > max_applications {
        return Err(Refusal::TooLarge {
            measure: Measure::Applications,
            value: total,
            limit: max_applications,
            setting: Measure::Applications.setting(),
        }
        .into());
    }
    let mut applications = Vec::new();
    for (receiver, object) in subject.objects.iter().enumerate() {
        for operation in &object.operations {
            for arguments in argument_vectors(operation) {
                applications.push(Application {
                    receiver,
                    operation,
                    arguments,
                });
            }
        }
    }
    Ok(applications)
}

/// Every argument vector of `operation`, the first parameter most
/// significant, each parameter's values in domain order.
fn argument_vectors(operation: &SymbolicOperation<'_>) -> Vec<Vec<Value>> {
    let mut vectors: Vec<Vec<Value>> = vec![Vec::new()];
    for domain in &operation.domains {
        let ParameterValues::Values(values) = domain else {
            return Vec::new();
        };
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
    vectors
}

/// The step disjuncts between two copies.
#[derive(Clone, Debug)]
pub struct StepRelation {
    operations: Vec<Term>,
    stutter: Option<Term>,
}

impl StepRelation {
    /// The relation between `from` and `to` over `applications`; the
    /// proof-query stutter last when `stutter`.
    ///
    /// # Errors
    ///
    /// [`Refusal::OutOfFragment`] naming the first construct of an
    /// applicable clause outside the fragment.
    pub(crate) fn build(
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        applications: &[Application<'_, '_>],
        from: &StateCopy,
        to: &StateCopy,
        stutter: bool,
    ) -> Result<Self, SmtEncodingRefusal> {
        let before = Views::of(StateRef::Copy(from.clone()));
        let across = Views {
            pre: StateRef::Copy(from.clone()),
            post: StateRef::Copy(to.clone()),
            current: StateRef::Copy(to.clone()),
        };
        let mut operations = Vec::new();
        for application in applications {
            operations.push(application.disjunct(
                subject,
                layout,
                (from, to),
                (&before, &across),
            )?);
        }
        let stutter = stutter.then(|| Term::all(layout.agreements(to, from)));
        Ok(Self {
            operations,
            stutter,
        })
    }

    /// The disjuncts in order, the stutter last.
    pub fn disjuncts(&self) -> impl Iterator<Item = &Term> {
        self.operations.iter().chain(self.stutter.iter())
    }

    /// Some operation disjunct holds (no stutter).
    pub fn operation_step(&self) -> Term {
        Term::any(self.operations.iter().cloned())
    }

    /// The transition assertion: some disjunct holds.
    pub fn transition(&self) -> Term {
        Term::any(self.disjuncts().cloned())
    }
}

/// What reading one copy gives: each application's failures, and whether the
/// copy is terminal.
#[derive(Clone, Debug)]
pub struct CopyFacts {
    precondition_failures: Vec<Term>,
    postcondition_failures: Vec<Term>,
    terminal: Option<Term>,
}

/// Keep the refusal of the earliest row of FR-315's table.
fn keep(best: &mut Option<SmtEncodingRefusal>, refusal: SmtEncodingRefusal) {
    if best.as_ref().is_none_or(|held| refusal.row() < held.row()) {
        *best = Some(refusal);
    }
}

impl CopyFacts {
    /// Read `copy`. The terminal condition is built when `terminal`.
    ///
    /// # Errors
    ///
    /// The refusal of the earliest row of FR-315's table that any
    /// application's reading meets: [`Refusal::FieldDomain`],
    /// [`Refusal::TooLarge`] for `candidates`, or
    /// [`Refusal::OutOfFragment`].
    pub(crate) fn read(
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        applications: &[Application<'_, '_>],
        copy: &StateCopy,
        terminal: bool,
        max_candidates: u64,
    ) -> Result<Self, SmtEncodingRefusal> {
        let mut best = None;
        let facts = Self::read_each(
            subject,
            layout,
            applications,
            copy,
            terminal,
            max_candidates,
            &mut best,
        );
        match best {
            Some(refusal) => Err(refusal),
            None => Ok(facts),
        }
    }

    fn read_each(
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        applications: &[Application<'_, '_>],
        copy: &StateCopy,
        terminal: bool,
        max_candidates: u64,
        best: &mut Option<SmtEncodingRefusal>,
    ) -> Self {
        let before = Views::of(StateRef::Copy(copy.clone()));
        let mut precondition_failures = Vec::new();
        let mut postcondition_failures = Vec::new();
        let mut enabled = Vec::new();
        for application in applications {
            match Self::read_application(
                subject,
                layout,
                application,
                copy,
                &before,
                terminal,
                max_candidates,
            ) {
                Ok((pre, post, enable)) => {
                    precondition_failures.push(pre);
                    postcondition_failures.push(post);
                    enabled.extend(enable);
                }
                Err(refusal) => keep(best, refusal),
            }
        }
        let terminal = terminal.then(|| Term::any(enabled).not());
        Self {
            precondition_failures,
            postcondition_failures,
            terminal,
        }
    }

    /// The precondition failure, the postcondition failure and, when
    /// `terminal`, the enabledness of one application at `copy`.
    fn read_application(
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        application: &Application<'_, '_>,
        copy: &StateCopy,
        before: &Views,
        terminal: bool,
        max_candidates: u64,
    ) -> Result<(Term, Term, Option<Term>), SmtEncodingRefusal> {
        let mut earlier: Vec<Term> = Vec::new();
        let mut failures = Vec::new();
        for clause in &application.operation.preconditions {
            let (value, defined) = application.read(subject, layout, clause, before, None)?;
            failures.push(Term::all(
                earlier.iter().cloned().chain([defined.clone().not()]),
            ));
            earlier.push(Term::all([defined, value]));
        }
        let precondition_failure = Term::any(failures);
        let results = application.results();

        // A postcondition read on the copy itself shows whether any of its
        // clauses holds a construct with a definedness condition.
        let probe = Views {
            pre: StateRef::Copy(copy.clone()),
            post: StateRef::Copy(copy.clone()),
            current: StateRef::Copy(copy.clone()),
        };
        let mut partial = false;
        for result in &results {
            for clause in &application.operation.postconditions {
                partial |= !application
                    .read(subject, layout, clause, &probe, result.as_ref())?
                    .1
                    .is_truth(true);
            }
        }
        if !partial && !terminal {
            return Ok((precondition_failure, Term::truth(false), None));
        }
        let mut ready = Vec::new();
        let mut broken = Vec::new();
        for candidate in application.candidates(subject, layout, copy, max_candidates)? {
            let views = Views {
                pre: StateRef::Copy(copy.clone()),
                post: candidate.clone(),
                current: candidate,
            };
            let mut accepted: Vec<Term> = Vec::new();
            for result in &results {
                let mut holds_all = Vec::new();
                let mut kept = Vec::new();
                let mut undefined = Vec::new();
                for clause in &application.operation.postconditions {
                    let (value, defined) =
                        application.read(subject, layout, clause, &views, result.as_ref())?;
                    holds_all.push(Term::all([defined.clone(), value.clone()]));
                    kept.push(defined.clone().implies(value));
                    undefined.push(defined.not());
                }
                let holds_here = Term::all(holds_all);
                // FR-120 stops at the first result the postcondition
                // accepts, so a later result's failure needs every earlier
                // one rejected.
                let failure = Term::all(
                    accepted
                        .iter()
                        .cloned()
                        .map(Not::not)
                        .chain([Term::all([Term::all(kept), Term::any(undefined)])]),
                );
                broken.push(failure);
                ready.push(holds_here.clone());
                accepted.push(holds_here);
            }
        }
        let enabled =
            terminal.then(|| Term::all(earlier.iter().cloned().chain([Term::any(ready)])));
        let postcondition_failure = if partial {
            Term::all(earlier.into_iter().chain([Term::any(broken)]))
        } else {
            Term::truth(false)
        };
        Ok((precondition_failure, postcondition_failure, enabled))
    }

    /// Some application's precondition is undefined on the copy, after the
    /// clauses before it hold.
    pub fn precondition_failures(&self) -> &[Term] {
        &self.precondition_failures
    }

    /// Some application is a candidate step with an undefined postcondition
    /// and no false one.
    pub fn postcondition_failures(&self) -> &[Term] {
        &self.postcondition_failures
    }

    /// The copy is terminal: no application is enabled.
    pub fn terminal(&self) -> Option<Term> {
        self.terminal.clone()
    }
}

impl Application<'_, '_> {
    /// The transition FR-120 gives this application: the step label `μ_U`
    /// reads (FR-315 "Hyper self-composition queries").
    pub(crate) fn transition(
        &self,
        subject: &SymbolicSubject<'_>,
    ) -> Result<ModelTransition, SmtEncodingRefusal> {
        let receiver = subject
            .objects
            .get(self.receiver)
            .ok_or_else(|| SmtEncodingRefusal::fault("an application has a receiver"))?;
        self.operation
            .transition(&receiver.reference, &self.arguments)
    }

    /// The application as a refusal names it: the operation applied to its
    /// arguments, led by the receiver's key when the subject holds several
    /// objects.
    pub(crate) fn label(&self, subject: &SymbolicSubject<'_>) -> String {
        let receiver = if subject.objects.len() > 1 {
            subject
                .objects
                .get(self.receiver)
                .map(|object| object.key.as_str())
        } else {
            None
        };
        self.operation.label(receiver, &self.arguments)
    }
}

impl<'s, 'a> Application<'s, 'a> {
    /// The receiver's index among the subject's objects.
    pub(crate) const fn receiver(&self) -> usize {
        self.receiver
    }

    /// The operation applied.
    pub(crate) const fn operation(&self) -> &'s SymbolicOperation<'a> {
        self.operation
    }

    /// The argument values, one per declared parameter.
    pub(crate) fn arguments(&self) -> &[Value] {
        &self.arguments
    }

    /// The application's step disjunct between `from` and `to`, as one
    /// conjunction of [`Self::conjuncts`].
    pub(crate) fn disjunct(
        &self,
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        copies: (&StateCopy, &StateCopy),
        views: (&Views, &Views),
    ) -> Result<Term, SmtEncodingRefusal> {
        Ok(Term::all(self.conjuncts(subject, layout, copies, views)?))
    }

    /// The conjuncts of the application's step disjunct between `from` and `to`: its
    /// precondition clauses on `from` (read through `before`), its
    /// postcondition relating the two copies (read through `across`), and
    /// its frame, each clause its definedness conjoined with its value.
    pub(crate) fn conjuncts(
        &self,
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        (from, to): (&StateCopy, &StateCopy),
        (before, across): (&Views, &Views),
    ) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let mut conjuncts = Vec::new();
        for clause in &self.operation.preconditions {
            let (value, defined) = self.read(subject, layout, clause, before, None)?;
            conjuncts.push(Term::all([defined, value]));
        }
        if matches!(self.operation.result, ResultValues::None) {
            for clause in &self.operation.postconditions {
                let (value, defined) = self.read(subject, layout, clause, across, None)?;
                conjuncts.push(Term::all([defined, value]));
            }
        } else {
            let mut alternatives = Vec::new();
            for result in &self.results() {
                let mut clauses = Vec::new();
                for clause in &self.operation.postconditions {
                    let (value, defined) =
                        self.read(subject, layout, clause, across, result.as_ref())?;
                    clauses.push(Term::all([defined, value]));
                }
                alternatives.push(Term::all(clauses));
            }
            conjuncts.push(Term::any(alternatives));
        }
        conjuncts.extend(self.frame(subject, layout, from, to));
        Ok(conjuncts)
    }
}

/// The clause `clause` of an operation applied to the object `receiver`,
/// with `arguments` (one per declared parameter of `operation`) and
/// `result` bound as symbols, read over `views`: its Boolean and its
/// definedness. The refinement encoding binds terms evaluated over another
/// subject's copy, where a step relation binds literals.
pub(crate) fn read_bound(
    (subject, layout): (&SymbolicSubject<'_>, &StateLayout),
    clause: &CheckedStateClause,
    views: &Views,
    (receiver, operation, arguments, result): (
        usize,
        &SymbolicOperation<'_>,
        &[Option<Sym>],
        Option<Sym>,
    ),
) -> Result<(Term, Term), SmtEncodingRefusal> {
    let mut slots = vec![Some(Sym::Object(receiver))];
    let mut declared = clause.parameters().iter().skip(1);
    if clause.binds_result() {
        declared.next();
        slots.push(result);
    }
    for (name, _) in declared {
        let value = operation
            .parameters
            .iter()
            .position(|(parameter, _)| parameter == name)
            .and_then(|position| arguments.get(position))
            .cloned()
            .flatten();
        slots.push(value);
    }
    let reads = clause.reads().collect();
    Reader::new(
        layout,
        subject,
        clause.name(),
        clause.observation(),
        reads,
        views,
        slots,
    )
    .boolean(clause.body())
}

/// Whether `operation` applied to `receiver` may write `field` of `object`.
pub(crate) fn writes(
    subject: &SymbolicSubject<'_>,
    operation: &SymbolicOperation<'_>,
    receiver: usize,
    (object, field): (usize, usize),
) -> bool {
    let field = &subject.objects[object].fields[field];
    operation.modifies.contains(&field.key)
        || (object == receiver && operation.receiver_modifies.contains(&field.key))
}

/// The values an operation's result takes, in FR-120's result order; one
/// absent value for an operation with no result.
pub(crate) fn result_values(operation: &SymbolicOperation<'_>) -> Vec<Option<Value>> {
    match &operation.result {
        ResultValues::Declared(ParameterValues::Values(values)) => {
            values.iter().cloned().map(Some).collect()
        }
        _ => vec![None],
    }
}

impl<'a> Application<'_, 'a> {
    /// The values the operation's result takes, in FR-120's result order;
    /// one absent value for an operation with no result.
    fn results(&self) -> Vec<Option<Value>> {
        result_values(self.operation)
    }

    /// The clause's body as a Boolean and its definedness, with `self`, the
    /// parameters, the arguments and `result` bound.
    fn read(
        &self,
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        clause: &CheckedStateClause,
        views: &Views,
        result: Option<&Value>,
    ) -> Result<(Term, Term), SmtEncodingRefusal> {
        // An argument of no SMT sort binds no symbol, and a read of it
        // refuses.
        let arguments: Vec<Option<Sym>> = self
            .arguments
            .iter()
            .map(|value| symbol_of(subject, value))
            .collect();
        read_bound(
            (subject, layout),
            clause,
            views,
            (
                self.receiver,
                self.operation,
                &arguments,
                result.and_then(|value| symbol_of(subject, value)),
            ),
        )
    }
    /// Whether the frame lets the operation write `field` of `object`.
    fn writes(&self, subject: &SymbolicSubject<'_>, object: usize, field: usize) -> bool {
        writes(subject, self.operation, self.receiver, (object, field))
    }

    /// Every field the frame does not let the operation write is unchanged,
    /// copy `to` with copy `from`.
    fn frame(
        &self,
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        from: &StateCopy,
        to: &StateCopy,
    ) -> Vec<Term> {
        let mut frame = Vec::new();
        for (index, object) in subject.objects.iter().enumerate() {
            for field in 0..object.fields.len() {
                if !self.writes(subject, index, field) {
                    frame.extend(layout.unchanged(to, from, index, field));
                }
            }
        }
        frame
    }

    /// The candidate post-states over `base`, in FR-120's order: objects in
    /// order, the fields they may be written in declaration order, the last
    /// varying fastest, an optional field's absence before its values.
    ///
    /// # Errors
    ///
    /// [`Refusal::FieldDomain`] for the first written field, in
    /// object and field order, with no finite domain, then
    /// [`Refusal::TooLarge`] for `candidates`.
    fn candidates(
        &self,
        subject: &SymbolicSubject<'_>,
        layout: &StateLayout,
        base: &StateCopy,
        max_candidates: u64,
    ) -> Result<Vec<StateRef>, SmtEncodingRefusal> {
        for (index, object) in subject.objects.iter().enumerate() {
            for (field, symbolic) in object.fields.iter().enumerate() {
                if self.writes(subject, index, field) && symbolic.domain_len.is_none() {
                    return Err(Refusal::FieldDomain {
                        object: object.key.clone(),
                        field: symbolic.name.clone(),
                    }
                    .into());
                }
            }
        }
        // (object, field in layout order, values with absence first)
        let mut dimensions: Vec<((usize, usize), Vec<FieldTerms>)> = Vec::new();
        let mut count: u64 = 1;
        for (index, object) in subject.objects.iter().enumerate() {
            let mut order: Vec<usize> = (0..object.fields.len()).collect();
            order.sort_by_key(|field| object.fields[*field].declared);
            for field in order {
                if !self.writes(subject, index, field) {
                    continue;
                }
                let symbolic = &object.fields[field];
                let radix = symbolic
                    .domain_len
                    .unwrap_or(0)
                    .saturating_add(u128::from(symbolic.optional));
                count = count.saturating_mul(u64::try_from(radix).unwrap_or(u64::MAX));
                if count > max_candidates {
                    return Err(Refusal::TooLarge {
                        measure: Measure::Candidates,
                        value: count,
                        limit: max_candidates,
                        setting: Measure::Candidates.setting(),
                    }
                    .into());
                }
                let sort = layout.objects()[index].fields()[field].sort();
                let zero = match sort {
                    FieldSort::Bool => Term::literal(false),
                    FieldSort::Int(_) => Term::int(0),
                };
                let mut options = Vec::new();
                if symbolic.optional {
                    options.push(FieldTerms {
                        value: zero,
                        present: Some(Term::literal(false)),
                    });
                }
                for value in &symbolic.values {
                    options.push(FieldTerms {
                        value: value_term(value)
                            .ok_or_else(|| SmtEncodingRefusal::fault("candidate-value-sort"))?,
                        present: symbolic.optional.then(|| Term::literal(true)),
                    });
                }
                dimensions.push(((index, field), options));
            }
        }
        let mut candidates = Vec::new();
        let mut digits = vec![0usize; dimensions.len()];
        if dimensions.iter().any(|(_, options)| options.is_empty()) {
            return Ok(candidates);
        }
        loop {
            let written: Vec<((usize, usize), FieldTerms)> = dimensions
                .iter()
                .zip(&digits)
                .map(|((at, options), digit)| (*at, options[*digit].clone()))
                .collect();
            candidates.push(StateRef::Candidate {
                base: base.clone(),
                written: Rc::new(written),
            });
            let mut position = digits.len();
            loop {
                let Some(previous) = position.checked_sub(1) else {
                    return Ok(candidates);
                };
                position = previous;
                digits[position] += 1;
                if digits[position] < dimensions[position].1.len() {
                    break;
                }
                digits[position] = 0;
            }
        }
    }
}

pub(crate) fn symbol_of(subject: &SymbolicSubject<'_>, value: &Value) -> Option<Sym> {
    match value {
        Value::Boolean(value) => Some(Sym::Bool(Term::literal(*value))),
        Value::Integer(value) => Some(Sym::Int(Term::Int(value.clone()))),
        Value::Enum(member) => Some(Sym::Int(Term::int(i64::from(member.rank())))),
        Value::Reference(reference) => subject
            .objects
            .iter()
            .position(|object| object.reference == *reference)
            .map(Sym::Object),
        _ => None,
    }
}
