// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315: `encode_smt_query`, the proof query of a temporal item over a
//! model subject in canonical SMT-LIB 2.
//!
//! The script is, in order: `(set-logic QF_LIA)`; the `declare-const`
//! commands position by position (a copy's constants in ascending (object,
//! field) order, then `m<i>`, then `h<i>`); the range assertions; for
//! `Unrolling` and `InductionBase` the initial-state predicate on copy 0, then
//! `(not h0)` for a finite behaviour; one step assertion per step; the
//! monitor implications; for `InductionStep` the pairwise distinctness and the
//! property assertions at the positions before the last; the negated property;
//! `(check-sat)`.
//!
//! Under infinite-trace the step relation has the proof-query stutter and
//! no copy is cut short. Under event-position false-extension `h<i>` marks the
//! positions past the end of a finite behaviour, which closes only at a
//! terminal copy, and every atom reads false there.

use std::ops::Not;

use qsl_semantics::check::{
    Activation, CheckedNode, CheckedTemporalClause, FormulaNodeId, TemporalClauseNode,
    TemporalProfile, TerminalDeclaration,
};
use qsl_semantics::model::clause_observation::Observation;
use quire_exact::Integer;
use quire_semantic_value::fairness::PropertyForm;

use crate::automaton::{bounded_shape, invariant_atom, AutomatonError, BoundedKind};
use crate::model_subject::ModelSubject;
use crate::simulation::model::{ModelSystem, SymbolicSubject};
use crate::smt::expression::{Reader, Sym, Views};
use crate::smt::layout::{StateCopy, StateLayout, StateRef};
use crate::smt::monitor::{MonitorTable, MAX_ATOMS, MAX_STATES};
use crate::smt::relation::{applications, Application, CopyFacts, StepRelation};
use crate::smt::script::{Script, Sort, Term, MAX_SCRIPT_BYTES};
use crate::smt::{SmtEncodingRefusal, SmtQueryKind};

/// The most constants a script declares.
pub const MAX_CONSTANTS: u64 = 1 << 20;

/// What a proof query is about (FR-315 Inputs).
#[derive(Clone, Copy)]
pub enum SmtItem<'a> {
    /// A checked temporal clause with its property form (FR-123).
    Clause(&'a CheckedTemporalClause),
    /// FR-124's deadlock-freedom item.
    DeadlockFreedom,
}

/// How an item's property is read at a position.
enum Property {
    /// TP-1: the state predicate of the one atom.
    Invariant(FormulaNodeId),
    /// TP-2 `on origin`: `always[0,u]` or `eventually[0,u]` of the one atom.
    Bounded(BoundedKind, u64, FormulaNodeId),
    /// TP-3: the monitor not rejecting.
    Monitor(MonitorTable),
    /// FR-124: `always holds(not deadlocked)`.
    Deadlock,
}

/// What reading an item's atoms needs: the clause node that holds the atoms'
/// conditions and the objects its instances bind.
struct Atoms<'q, 'a> {
    node: &'q TemporalClauseNode,
    clause: &'q str,
    layout: &'q StateLayout,
    subject: &'q SymbolicSubject<'a>,
    instances: Vec<usize>,
}

impl Atoms<'_, '_> {
    /// The value and definedness of `atom` for `instance` at `copy`.
    fn read(
        &self,
        atom: FormulaNodeId,
        instance: usize,
        copy: &StateCopy,
    ) -> Result<(Term, Term), SmtEncodingRefusal> {
        let (condition, _): (CheckedNode<'_>, usize) =
            self.node.condition(atom).ok_or(SmtEncodingRefusal::Fault(
                qsl_foundation::InternalFault::new("smt-encoding", "atom-has-a-condition"),
            ))?;
        let views = Views::of(StateRef::Copy(copy.clone()));
        Reader::new(
            self.layout,
            self.subject,
            self.clause,
            Observation::Current,
            Vec::new(),
            &views,
            vec![Some(Sym::Object(self.instances[instance]))],
        )
        .boolean(condition)
    }
}

fn fault(invariant: &'static str) -> SmtEncodingRefusal {
    SmtEncodingRefusal::Fault(qsl_foundation::InternalFault::new(
        "smt-encoding",
        invariant,
    ))
}

fn too_large(measure: &'static str, value: u64, limit: u64) -> SmtEncodingRefusal {
    SmtEncodingRefusal::TooLarge {
        measure,
        value,
        limit,
    }
}

fn monitor_constant(position: u64) -> Term {
    Term::constant(format!("m{position}"))
}

fn ended_constant(position: u64) -> Term {
    Term::constant(format!("h{position}"))
}

/// The literals of `letter` over `terms`: each atom's term when its bit is
/// set, its negation otherwise.
fn letter_term(letter: u64, terms: &[Term]) -> Term {
    Term::all(terms.iter().enumerate().map(|(index, term)| {
        if letter >> index & 1 == 1 {
            term.clone()
        } else {
            term.clone().not()
        }
    }))
}

/// `terms` without a repeated term, first occurrence kept.
fn distinct(terms: Vec<Term>) -> Vec<Term> {
    let mut kept: Vec<Term> = Vec::with_capacity(terms.len());
    for term in terms {
        if !kept.contains(&term) {
            kept.push(term);
        }
    }
    kept
}

/// Encode the proof query `kind` of `item` over `subject` as an SMT-LIB 2
/// script in the canonical printing (FR-315).
///
/// # Errors
///
/// [`SmtEncodingRefusal`], with the code of FR-315's table, naming the
/// construct and its locus; no script is written.
pub fn encode_smt_query(
    subject: &ModelSubject<'_>,
    item: SmtItem<'_>,
    kind: SmtQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    let clause = match item {
        SmtItem::Clause(clause) => Some(clause),
        SmtItem::DeadlockFreedom => None,
    };
    let mut bounded = None;
    if let Some(clause) = clause {
        let form = clause.property_form().ok_or(SmtEncodingRefusal::ItemForm {
            form: "a clause with no property form",
        })?;
        if form == PropertyForm::Liveness {
            return Err(SmtEncodingRefusal::ItemForm {
                form: "a liveness (TP-4) clause, whose refutations are lassos",
            });
        }
        if form == PropertyForm::BoundedMltl {
            if clause.activation() != Activation::Origin {
                return Err(SmtEncodingRefusal::ItemForm {
                    form: "a TP-2 `on each` clause",
                });
            }
            bounded = Some(bounded_shape(clause).ok_or(SmtEncodingRefusal::ItemForm {
                form: "a TP-2 `on origin` clause other than a bounded `always` or `eventually` of one atom",
            })?);
        }
        if matches!(clause.activation(), Activation::Each { guarded: true }) {
            return Err(SmtEncodingRefusal::ItemForm {
                form: "a clause guarded by a `when` condition",
            });
        }
        if !matches!(
            clause.profile(),
            TemporalProfile::InfiniteTrace | TemporalProfile::EventPositionFalseExtension
        ) {
            return Err(SmtEncodingRefusal::Profile);
        }
        if bounded.is_some() && !matches!(kind, SmtQueryKind::Unrolling { .. }) {
            return Err(SmtEncodingRefusal::QueryKind {
                kind: kind.name(),
                form: "TP-2 `on origin`",
            });
        }
    }
    let system = ModelSystem::new(subject.simulation.clone())
        .map_err(SmtEncodingRefusal::SubjectAdmission)?;
    let symbolic = system.symbolic()?;
    let layout = StateLayout::of(&symbolic)?;
    // Parameter domains, in object, operation and parameter order.
    for object in &symbolic.objects {
        for operation in &object.operations {
            for ((name, _), domain) in operation.parameters.iter().zip(&operation.domains) {
                if domain.is_none() {
                    return Err(SmtEncodingRefusal::ParameterDomain {
                        operation: operation.name.clone(),
                        parameter: name.clone(),
                    });
                }
            }
        }
    }

    let mut instances = Vec::new();
    let mut node = None;
    let mut atom_property = None;
    if let Some(clause) = clause {
        let over = match &clause.over().1 {
            quire_exact::ValueType::Reference(over) => *over,
            _ => return Err(fault("over-is-a-reference")),
        };
        let found = system
            .instances(&clause.over().1)
            .map_err(SmtEncodingRefusal::RequiresBound)?;
        if found.is_empty() {
            return Err(SmtEncodingRefusal::NoInstance);
        }
        for instance in &found {
            let position = symbolic
                .objects
                .iter()
                .position(|object| {
                    object.population == instance.population && object.key == instance.key
                })
                .filter(|position| {
                    symbolic
                        .types
                        .conforms(symbolic.objects[*position].type_id, over)
                })
                .ok_or_else(|| SmtEncodingRefusal::InstanceBinding {
                    key: instance.key.clone(),
                })?;
            instances.push(position);
        }
        node = Some(
            symbolic
                .package
                .graph()
                .temporal_clauses()
                .iter()
                .find(|node| node.clause() == clause)
                .ok_or(SmtEncodingRefusal::ItemForm {
                    form: "a clause the package does not emit",
                })?,
        );
        atom_property = Some(match (invariant_atom(clause), bounded) {
            (Some(atom), _) if bounded.is_none() => Some(Property::Invariant(atom)),
            (_, Some((shape, upper, atom))) => Some(Property::Bounded(shape, upper, atom)),
            _ => None,
        });
    }
    let monitored = clause.is_some() && matches!(atom_property, Some(None));
    if monitored && instances.len() != 1 {
        return Err(SmtEncodingRefusal::SeveralInstances {
            count: instances.len(),
        });
    }

    // Size limits, in the order of FR-315 "Size limits".
    let depth = kind.depth();
    let property_hint_monitor = monitored;
    let infinite = clause.is_none_or(|clause| clause.profile() == TemporalProfile::InfiniteTrace);
    let per_position = layout
        .objects()
        .iter()
        .flat_map(|object| object.fields())
        .map(|field| 1 + u64::from(field.is_optional()))
        .sum::<u64>()
        + u64::from(property_hint_monitor)
        + u64::from(!infinite);
    let constants = depth.saturating_add(1).saturating_mul(per_position);
    if constants > MAX_CONSTANTS {
        return Err(too_large("constants", constants, MAX_CONSTANTS));
    }
    let apps = applications(&symbolic)?;
    let property = match (clause, atom_property) {
        (None, _) => Property::Deadlock,
        (Some(_), Some(Some(property))) => property,
        (Some(clause), _) => {
            Property::Monitor(MonitorTable::of(clause).map_err(|refusal| match refusal {
                SmtEncodingRefusal::ItemTranslation(AutomatonError::StateLimit { limit }) => {
                    too_large("monitor-states", limit.saturating_add(1), MAX_STATES as u64)
                }
                other => other,
            })?)
        }
    };
    if let Property::Monitor(table) = &property {
        if table.atoms().len() > MAX_ATOMS {
            return Err(too_large(
                "monitor-atoms",
                table.atoms().len() as u64,
                MAX_ATOMS as u64,
            ));
        }
    }
    Query {
        kind,
        layout: &layout,
        subject: &symbolic,
        apps,
        atoms: node.map(|node| Atoms {
            node,
            clause: clause.map_or("", |clause| clause.name()),
            layout: &layout,
            subject: &symbolic,
            instances,
        }),
        property,
        infinite,
    }
    .script()
    .map(Script::render)
}

/// One query under construction.
struct Query<'q, 'a> {
    kind: SmtQueryKind,
    layout: &'q StateLayout,
    subject: &'q SymbolicSubject<'a>,
    apps: Vec<Application<'q, 'a>>,
    atoms: Option<Atoms<'q, 'a>>,
    property: Property,
    /// Whether the profile is infinite-trace. Under event-position
    /// false-extension a behaviour may end before `depth`.
    infinite: bool,
}

impl Query<'_, '_> {
    fn monitor(&self) -> Option<&MonitorTable> {
        match &self.property {
            Property::Monitor(table) => Some(table),
            _ => None,
        }
    }

    /// The last position the claim reads.
    fn horizon(&self) -> u64 {
        let depth = self.kind.depth();
        match &self.property {
            Property::Bounded(_, upper, _) => depth.min(*upper),
            _ => depth,
        }
    }

    /// The flag that marks `position` as past the end of a finite behaviour.
    fn ended(&self, position: u64) -> Option<Term> {
        (!self.infinite).then(|| ended_constant(position))
    }

    /// `term` read where the behaviour is live at `position`.
    fn live(&self, position: u64, term: Term) -> Term {
        match self.ended(position) {
            Some(ended) => Term::all([ended.not(), term]),
            None => term,
        }
    }

    /// The value and definedness of `atom` of `instance` at `position`; past
    /// the end of a finite behaviour the atom is false and defined.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: usize,
        position: u64,
    ) -> Result<(Term, Term), SmtEncodingRefusal> {
        let atoms = self.atoms.as_ref().ok_or_else(|| fault("item-has-atoms"))?;
        let (value, defined) = atoms.read(atom, instance, &StateCopy::at(position))?;
        Ok(match self.ended(position) {
            Some(ended) => (
                Term::all([ended.clone().not(), value]),
                Term::any([ended, defined]),
            ),
            None => (value, defined),
        })
    }

    fn instances(&self) -> usize {
        self.atoms.as_ref().map_or(0, |atoms| atoms.instances.len())
    }

    fn check_size(script: &Script) -> Result<(), SmtEncodingRefusal> {
        if script.exceeded() {
            Err(too_large(
                "script-bytes",
                MAX_SCRIPT_BYTES as u64 + 1,
                MAX_SCRIPT_BYTES as u64,
            ))
        } else {
            Ok(())
        }
    }

    fn script(&self) -> Result<Script, SmtEncodingRefusal> {
        let depth = self.kind.depth();
        let mut script = Script::new();
        for position in 0..=depth {
            let copy = StateCopy::at(position);
            self.layout.declare(&copy, &mut script);
            if self.monitor().is_some() {
                script.declare(&format!("m{position}"), Sort::Int);
            }
            if !self.infinite {
                script.declare(&format!("h{position}"), Sort::Bool);
            }
            Self::check_size(&script)?;
        }
        for position in 0..=depth {
            for range in self.layout.ranges(&StateCopy::at(position)) {
                script.assert(&range);
            }
            if let Some(table) = self.monitor() {
                script.assert(&monitor_constant(position).within(
                    &Integer::zero(),
                    &Integer::from(table.states().saturating_sub(1)),
                ));
            }
            Self::check_size(&script)?;
        }
        if self.kind.starts_initial() {
            script.assert(&self.layout.initial(&StateCopy::at(0), self.subject)?);
            if let Some(ended) = self.ended(0) {
                script.assert(&ended.not());
            }
        }

        let need_terminal = !self.infinite || matches!(self.property, Property::Deadlock);
        let mut facts = Vec::new();
        for position in 0..=depth {
            facts.push(CopyFacts::read(
                self.subject,
                self.layout,
                &self.apps,
                &StateCopy::at(position),
                need_terminal,
            )?);
            Self::check_size(&script)?;
        }
        for step in 0..depth {
            let relation = StepRelation::build(
                self.subject,
                self.layout,
                &self.apps,
                &StateCopy::at(step),
                &StateCopy::at(step + 1),
                self.infinite,
            )?;
            let index = usize::try_from(step).map_err(|_| fault("position"))?;
            script.assert(&self.step(step, &relation, &facts[index]));
            Self::check_size(&script)?;
        }
        if let Some(table) = self.monitor() {
            self.assert_monitor(table, &mut script)?;
        }
        if matches!(self.kind, SmtQueryKind::InductionStep { .. }) {
            self.assert_distinct(&mut script)?;
            for position in 0..depth {
                let index = usize::try_from(position).map_err(|_| fault("position"))?;
                let violated = Term::any(self.violations(position, &facts[index])?);
                let failed = Term::any(self.failures(position, &facts[index])?);
                script.assert(&Term::all([violated.not(), failed.not()]));
                Self::check_size(&script)?;
            }
        }
        let negated = self.negated(&facts)?;
        script.assert(&Term::any(negated));
        Self::check_size(&script)?;
        Ok(script)
    }

    /// The step assertion from `step` to `step + 1`.
    fn step(&self, step: u64, relation: &StepRelation, facts: &CopyFacts) -> Term {
        if self.infinite {
            return relation.transition();
        }
        let (was, is) = (ended_constant(step), ended_constant(step + 1));
        let terminal = facts.terminal().unwrap_or(Term::truth(false));
        Term::all([
            Term::any([was.equal(is.clone()), terminal]),
            Term::any([is, relation.operation_step()]),
        ])
    }

    /// The monitor's initial step and its steps along the path.
    fn assert_monitor(
        &self,
        table: &MonitorTable,
        script: &mut Script,
    ) -> Result<(), SmtEncodingRefusal> {
        let depth = self.kind.depth();
        for position in 0..=depth {
            let mut letter = Vec::new();
            for atom in table.atoms() {
                letter.push(self.atom(*atom, 0, position)?.0);
            }
            for step in table.moves() {
                let reads = letter_term(step.letter, &letter);
                let reached = monitor_constant(position).equal(Term::int(
                    i64::try_from(step.to).map_err(|_| fault("monitor-state-index"))?,
                ));
                let condition = if position == 0 {
                    if !self.kind.starts_initial() || step.from != table.initial() {
                        continue;
                    }
                    reads
                } else {
                    let before = monitor_constant(position - 1).equal(Term::int(
                        i64::try_from(step.from).map_err(|_| fault("monitor-state-index"))?,
                    ));
                    Term::all([before, reads])
                };
                script.assert(&condition.implies(reached));
            }
            Self::check_size(script)?;
        }
        Ok(())
    }

    /// The copies at positions `0..=depth` are pairwise distinct (some value
    /// or presence constant, or the monitor state, differs between every two).
    fn assert_distinct(&self, script: &mut Script) -> Result<(), SmtEncodingRefusal> {
        let depth = self.kind.depth();
        for left in 0..=depth {
            for right in left + 1..=depth {
                let mut differences = vec![self
                    .layout
                    .differs(&StateCopy::at(left), &StateCopy::at(right))];
                if self.monitor().is_some() {
                    differences.push(monitor_constant(left).equal(monitor_constant(right)).not());
                }
                script.assert(&Term::any(differences));
                Self::check_size(script)?;
            }
        }
        Ok(())
    }

    /// The violations of the property at `position`, one per instance in
    /// instance order (the monitor and deadlock items have one).
    fn violations(
        &self,
        position: u64,
        facts: &CopyFacts,
    ) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let ended = self.ended(position);
        let mut violations = Vec::new();
        match &self.property {
            Property::Invariant(atom) | Property::Bounded(BoundedKind::Always, _, atom) => {
                if matches!(&self.property, Property::Bounded(_, upper, _) if position > *upper) {
                    return Ok(violations);
                }
                for instance in 0..self.instances() {
                    let (value, _) = self.atom(*atom, instance, position)?;
                    let broken = value.not();
                    violations.push(match &ended {
                        Some(ended) => Term::any([ended.clone(), broken]),
                        None => broken,
                    });
                }
            }
            Property::Bounded(BoundedKind::Eventually, ..) => {}
            Property::Monitor(table) => violations.push(match table.rejecting() {
                Some(rejecting) => monitor_constant(position).equal(Term::int(
                    i64::try_from(rejecting).map_err(|_| fault("monitor-state-index"))?,
                )),
                None => Term::truth(false),
            }),
            Property::Deadlock => {
                let terminal = facts.terminal().ok_or_else(|| fault("terminal-read"))?;
                let (intended, _) = self.intended(position)?;
                violations.push(Term::all([terminal, intended.not()]));
            }
        }
        Ok(violations)
    }

    /// `P` of DL-1 at `position`: whether a terminal state is intended, and
    /// the condition under which `P` is undefined there.
    fn intended(&self, position: u64) -> Result<(Term, Term), SmtEncodingRefusal> {
        let layout = self.layout;
        let views = Views::of(StateRef::Copy(StateCopy::at(position)));
        let mut value = Vec::new();
        let mut failure = Vec::new();
        for terminal in self.subject.package.graph().terminals() {
            let (holds, defined) = match terminal.declaration() {
                TerminalDeclaration::None => continue,
                TerminalDeclaration::Any => (Term::truth(true), Term::truth(true)),
                TerminalDeclaration::When(predicate) => {
                    let Some((_, quire_exact::ValueType::Reference(context))) =
                        predicate.parameters().first()
                    else {
                        return Err(fault("terminal-parameter"));
                    };
                    let context = *context;
                    let mut holds = Vec::new();
                    let mut defined = Vec::new();
                    for (index, object) in self.subject.objects.iter().enumerate() {
                        if !self.subject.types.conforms(object.type_id, context) {
                            continue;
                        }
                        let (value, definition) = Reader::new(
                            layout,
                            self.subject,
                            terminal.name().unwrap_or("terminal"),
                            Observation::Current,
                            Vec::new(),
                            &views,
                            vec![Some(Sym::Object(index))],
                        )
                        .boolean(predicate.body())?;
                        holds.push(value);
                        defined.push(definition);
                    }
                    (Term::all(holds), Term::all(defined))
                }
            };
            // An undefined `P` fails only where no earlier declaration
            // already intended the state.
            failure.push(Term::all([
                Term::any(value.iter().cloned()).not(),
                defined.clone().not(),
            ]));
            value.push(Term::all([defined, holds]));
        }
        Ok((Term::any(value), Term::any(failure)))
    }

    /// The definedness failures at `position`: the atoms the claim reads,
    /// then each application's precondition failures and postcondition
    /// failures (FR-315 "Undefined evaluation").
    fn failures(&self, position: u64, facts: &CopyFacts) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let mut failures = Vec::new();
        if position <= self.horizon() {
            match &self.property {
                Property::Invariant(atom) | Property::Bounded(_, _, atom) => {
                    for instance in 0..self.instances() {
                        failures.push(self.atom(*atom, instance, position)?.1.not());
                    }
                }
                Property::Monitor(table) => {
                    for atom in table.atoms() {
                        failures.push(self.atom(*atom, 0, position)?.1.not());
                    }
                }
                Property::Deadlock => {
                    let terminal = facts.terminal().ok_or_else(|| fault("terminal-read"))?;
                    failures.push(Term::all([terminal, self.intended(position)?.1]));
                }
            }
        }
        for failure in facts
            .precondition_failures()
            .iter()
            .chain(facts.postcondition_failures())
        {
            failures.push(self.live(position, failure.clone()));
        }
        Ok(failures)
    }

    /// The negated property: a violation or a definedness failure at some
    /// position, or for `InductionStep` at the last.
    fn negated(&self, facts: &[CopyFacts]) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let depth = self.kind.depth();
        let last = matches!(self.kind, SmtQueryKind::InductionStep { .. });
        let positions = if last { depth..=depth } else { 0..=depth };
        let mut disjuncts = Vec::new();
        if let Property::Bounded(BoundedKind::Eventually, ..) = &self.property {
            // Per instance, the atom is false or past the end at every
            // position of the horizon.
            let Property::Bounded(_, _, atom) = &self.property else {
                return Err(fault("eventually-has-an-atom"));
            };
            for instance in 0..self.instances() {
                let mut every = Vec::new();
                for position in 0..=self.horizon() {
                    let (value, _) = self.atom(*atom, instance, position)?;
                    every.push(value.not());
                }
                disjuncts.push(Term::all(every));
            }
        }
        for position in positions {
            let index = usize::try_from(position).map_err(|_| fault("position"))?;
            let at = facts
                .get(index)
                .ok_or_else(|| fault("facts-per-position"))?;
            if position <= self.horizon() {
                disjuncts.extend(self.violations(position, at)?);
            }
            disjuncts.extend(self.failures(position, at)?);
        }
        Ok(distinct(disjuncts))
    }
}
