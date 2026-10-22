// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315: `encode_smt_query`, the proof query of a temporal item over a
//! model subject in canonical SMT-LIB 2.
//!
//! The script is, in order: `(set-logic QF_LIA)`; the `declare-const`
//! commands in ascending (position, object, field) order, with each monitor
//! state `m<i>` after the copy at position `i`; one range assertion per ranged
//! constant; the initial-state predicate on copy 0 (and the initial monitor
//! step) for `Unrolling` and `InductionBase`; one transition assertion per
//! step, each followed by its monitor steps; for `InductionStep` the pairwise
//! distinctness of the copies and the property and definedness conditions at
//! the positions before the last; the negated property; `(check-sat)`.

use qsl_semantics::check::{
    CheckedNode, CheckedTemporalClause, FormulaNodeId, TemporalClauseNode, TemporalProfile,
};
use qsl_semantics::model::clause_observation::Observation;
use quire_exact::Integer;
use quire_semantic_value::fairness::PropertyForm;
use std::ops::Not;

use crate::automaton::{bounded_shape, invariant_atom, BoundedKind};
use crate::model_subject::ModelSubject;
use crate::simulation::model::{ModelSystem, SymbolicSubject};
use crate::smt::expression::{Reader, Sym, Views};
use crate::smt::layout::{StateCopy, StateLayout};
use crate::smt::monitor::MonitorTable;
use crate::smt::relation::StepRelation;
use crate::smt::script::{Script, Sort, Term};
use crate::smt::{SmtEncodingRefusal, SmtQueryKind};

/// The most constants a script declares.
pub const MAX_CONSTANTS: usize = 1 << 20;

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
    /// TP-3 and TP-2 `on each`: the monitor not rejecting.
    Monitor(MonitorTable),
}

/// The property at one position: its violation and its definedness.
struct Reading {
    violation: Term,
    defined: Term,
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
        let views = Views {
            pre: copy.clone(),
            post: copy.clone(),
            current: copy.clone(),
        };
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

    /// The conjunction over instances of `atom` at `copy`, and its
    /// definedness.
    fn all_instances(
        &self,
        atom: FormulaNodeId,
        copy: &StateCopy,
    ) -> Result<(Term, Term), SmtEncodingRefusal> {
        let mut values = Vec::new();
        let mut defined = Vec::new();
        for instance in 0..self.instances.len() {
            let (value, definition) = self.read(atom, instance, copy)?;
            values.push(value);
            defined.push(definition);
        }
        Ok((Term::all(values), Term::all(defined)))
    }
}

fn fault(invariant: &'static str) -> SmtEncodingRefusal {
    SmtEncodingRefusal::Fault(qsl_foundation::InternalFault::new(
        "smt-encoding",
        invariant,
    ))
}

fn monitor_constant(position: u64) -> Term {
    Term::constant(format!("m{position}"))
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
/// [`SmtEncodingRefusal`] naming the construct, with its locus, when the
/// item or the subject uses a construct the encoding has no form for; no
/// script is written.
pub fn encode_smt_query(
    subject: &ModelSubject<'_>,
    item: SmtItem<'_>,
    kind: SmtQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    let clause = match item {
        SmtItem::Clause(clause) => clause,
        SmtItem::DeadlockFreedom => {
            return Err(SmtEncodingRefusal::ItemForm {
                form: "the deadlock-freedom item, whose enabledness needs a quantifier",
            })
        }
    };
    let form = clause.property_form().ok_or(SmtEncodingRefusal::ItemForm {
        form: "a clause with no property form",
    })?;
    if form == PropertyForm::Liveness {
        return Err(SmtEncodingRefusal::ItemForm {
            form: "a liveness (TP-4) item, whose refutations are lassos",
        });
    }
    let system = ModelSystem::new(subject.simulation.clone())?;
    let symbolic = system.symbolic()?;
    let layout = StateLayout::of(&symbolic)?;
    let depth = kind.depth();
    let per_copy: usize = layout
        .objects()
        .iter()
        .flat_map(|object| object.fields())
        .map(|field| 1 + usize::from(field.is_optional()))
        .sum();
    let copies = usize::try_from(depth)
        .ok()
        .and_then(|depth| depth.checked_add(1));
    if copies
        .and_then(|copies| copies.checked_mul(per_copy.max(1)))
        .is_none_or(|constants| constants > MAX_CONSTANTS)
    {
        return Err(SmtEncodingRefusal::TooLarge {
            depth,
            max: MAX_CONSTANTS,
        });
    }

    let node = symbolic
        .package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause() == clause)
        .ok_or(SmtEncodingRefusal::ItemForm {
            form: "a clause the package does not emit (S4 emits no `on each` clause)",
        })?;
    let over = match &clause.over().1 {
        quire_exact::ValueType::Reference(over) => *over,
        _ => return Err(fault("over-is-a-reference")),
    };
    let found = system
        .instances(&clause.over().1)
        .map_err(SmtEncodingRefusal::Instances)?;
    if found.is_empty() {
        return Err(SmtEncodingRefusal::NoInstance);
    }
    let mut instances = Vec::new();
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
    let atoms = Atoms {
        node,
        clause: clause.name(),
        layout: &layout,
        subject: &symbolic,
        instances,
    };

    let property = match (form, invariant_atom(clause), bounded_shape(clause)) {
        (PropertyForm::ReachableInvariant, Some(atom), _) => Property::Invariant(atom),
        (_, _, Some((bounded, upper, atom))) => {
            if !matches!(kind, SmtQueryKind::Unrolling { .. }) {
                return Err(SmtEncodingRefusal::QueryKind {
                    kind: kind.name(),
                    form: "TP-2 `on origin`",
                });
            }
            if bounded == BoundedKind::Eventually && atoms.instances.len() != 1 {
                return Err(SmtEncodingRefusal::SeveralInstances {
                    form: "bounded `eventually`",
                    count: atoms.instances.len(),
                });
            }
            Property::Bounded(bounded, upper, atom)
        }
        _ => {
            if atoms.instances.len() != 1 {
                return Err(SmtEncodingRefusal::SeveralInstances {
                    form: "monitored",
                    count: atoms.instances.len(),
                });
            }
            Property::Monitor(MonitorTable::of(clause)?)
        }
    };
    let stutter = clause.profile() == TemporalProfile::InfiniteTrace;
    Query {
        kind,
        layout: &layout,
        subject: &symbolic,
        atoms,
        property,
        stutter,
    }
    .script()
    .map(|script| script.render())
}

/// One query under construction.
struct Query<'q, 'a> {
    kind: SmtQueryKind,
    layout: &'q StateLayout,
    subject: &'q SymbolicSubject<'a>,
    atoms: Atoms<'q, 'a>,
    property: Property,
    stutter: bool,
}

impl Query<'_, '_> {
    fn monitor(&self) -> Option<&MonitorTable> {
        match &self.property {
            Property::Monitor(table) => Some(table),
            _ => None,
        }
    }

    /// How many positions the property reads: all of them, or for a bounded
    /// item up to its horizon.
    fn horizon(&self) -> u64 {
        let depth = self.kind.depth();
        match &self.property {
            Property::Bounded(_, upper, _) => depth.min(*upper),
            _ => depth,
        }
    }

    fn script(&self) -> Result<Script, SmtEncodingRefusal> {
        let depth = self.kind.depth();
        let mut script = Script::new();
        for position in 0..=depth {
            let copy = StateCopy::at(position);
            self.layout.declare(&copy, &mut script);
            if self.monitor().is_some() {
                script.declare(format!("m{position}"), Sort::Int);
            }
        }
        for position in 0..=depth {
            for range in self.layout.ranges(&StateCopy::at(position)) {
                script.assert(range);
            }
            if let Some(table) = self.monitor() {
                script.assert(monitor_constant(position).within(
                    &Integer::zero(),
                    &Integer::from(table.states().saturating_sub(1)),
                ));
            }
        }
        if self.kind.starts_initial() {
            script.assert(self.layout.initial(&StateCopy::at(0), self.subject)?);
        }
        let mut relations = Vec::new();
        for step in 0..depth {
            let relation = StepRelation::build(
                self.subject,
                self.layout,
                &StateCopy::at(step),
                &StateCopy::at(step + 1),
                self.stutter,
            )?;
            script.assert(relation.transition());
            relations.push(relation);
        }
        if let Some(table) = self.monitor() {
            self.assert_monitor(table, &mut script)?;
        }
        if matches!(self.kind, SmtQueryKind::InductionStep { .. }) {
            self.assert_distinct(&mut script);
            for position in 0..depth {
                script.assert(self.holds_before_last(position, &relations)?);
            }
        }
        script.assert(Term::any(self.negated(&relations)?));
        Ok(script)
    }

    /// The monitor's initial step and its steps along the path.
    fn assert_monitor(
        &self,
        table: &MonitorTable,
        script: &mut Script,
    ) -> Result<(), SmtEncodingRefusal> {
        let depth = self.kind.depth();
        for position in 0..=depth {
            let letter = self.letter(table, position)?;
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
                script.assert(condition.implies(reached));
            }
        }
        Ok(())
    }

    /// The value of each atom of the monitor's letter at `position`.
    fn letter(&self, table: &MonitorTable, position: u64) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let copy = StateCopy::at(position);
        table
            .atoms()
            .iter()
            .map(|atom| self.atoms.read(*atom, 0, &copy).map(|(value, _)| value))
            .collect()
    }

    /// The copies at positions `0..=depth` are pairwise distinct (some field
    /// or the monitor state differs between every two).
    fn assert_distinct(&self, script: &mut Script) {
        let depth = self.kind.depth();
        for left in 0..=depth {
            for right in left + 1..=depth {
                let mut differs = self
                    .layout
                    .differs(&StateCopy::at(left), &StateCopy::at(right));
                if self.monitor().is_some() {
                    differs = Term::any([
                        differs,
                        monitor_constant(left).equal(monitor_constant(right)).not(),
                    ]);
                }
                script.assert(differs);
            }
        }
    }

    /// The property reading at `position`.
    fn reading(&self, position: u64) -> Result<Reading, SmtEncodingRefusal> {
        let copy = StateCopy::at(position);
        match &self.property {
            Property::Invariant(atom) => {
                let (value, defined) = self.atoms.all_instances(*atom, &copy)?;
                Ok(Reading {
                    violation: value.not(),
                    defined,
                })
            }
            Property::Bounded(_, upper, atom) => {
                if position > *upper {
                    return Ok(Reading {
                        violation: Term::truth(false),
                        defined: Term::truth(true),
                    });
                }
                let (value, defined) = self.atoms.all_instances(*atom, &copy)?;
                Ok(Reading {
                    violation: value.not(),
                    defined,
                })
            }
            Property::Monitor(table) => {
                let violation = match table.rejecting() {
                    Some(rejecting) => monitor_constant(position).equal(Term::int(
                        i64::try_from(rejecting).map_err(|_| fault("monitor-state-index"))?,
                    )),
                    None => Term::truth(false),
                };
                let mut defined = Vec::new();
                for atom in table.atoms() {
                    defined.push(self.atoms.read(*atom, 0, &copy)?.1);
                }
                Ok(Reading {
                    violation,
                    defined: Term::all(defined),
                })
            }
        }
    }

    /// The property, its definedness and the definedness of the step out of
    /// `position` hold (EN-3's step case before the last position).
    fn holds_before_last(
        &self,
        position: u64,
        relations: &[StepRelation],
    ) -> Result<Term, SmtEncodingRefusal> {
        let reading = self.reading(position)?;
        let mut conjuncts = vec![reading.violation.not(), reading.defined];
        if let Some(step) = relations.get(usize::try_from(position).map_err(|_| fault("position"))?)
        {
            conjuncts.push(step.guard_failure().not());
        }
        if let Some(previous) = position
            .checked_sub(1)
            .and_then(|previous| usize::try_from(previous).ok())
            .and_then(|previous| relations.get(previous))
        {
            conjuncts.push(previous.post_failure().not());
        }
        Ok(Term::all(conjuncts))
    }

    /// The negated property: a violation or a definedness failure at some
    /// position, or for `InductionStep` at the last.
    fn negated(&self, relations: &[StepRelation]) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let depth = self.kind.depth();
        let last = matches!(self.kind, SmtQueryKind::InductionStep { .. });
        let horizon = self.horizon();
        let positions = if last { depth..=depth } else { 0..=horizon };
        let mut disjuncts = Vec::new();
        let mut eventually = Vec::new();
        for position in positions {
            let reading = self.reading(position)?;
            match &self.property {
                Property::Bounded(BoundedKind::Eventually, ..) => {
                    eventually.push(reading.violation);
                }
                _ => disjuncts.push(reading.violation),
            }
            disjuncts.push(reading.defined.not());
            let index = usize::try_from(position).map_err(|_| fault("position"))?;
            if !last && position < horizon {
                if let Some(step) = relations.get(index) {
                    disjuncts.push(step.guard_failure());
                }
            }
            if let Some(previous) = index
                .checked_sub(1)
                .and_then(|previous| relations.get(previous))
            {
                disjuncts.push(previous.post_failure());
            }
        }
        if !eventually.is_empty() {
            disjuncts.insert(0, Term::all(eventually));
        }
        Ok(distinct(disjuncts))
    }
}
