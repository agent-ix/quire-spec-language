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

use std::collections::HashSet;
use std::ops::Not;

use qsl_semantics::check::{
    Activation, CheckedNode, CheckedTemporalClause, FormulaNodeId, TemporalClauseNode,
    TemporalProfile, TerminalDeclaration,
};
use qsl_semantics::model::clause_observation::Observation;
use quire_exact::Integer;
use quire_semantic_value::fairness::PropertyForm;

use crate::automaton::{bounded_shape, invariant_atom, BoundedKind};
use crate::model_subject::ModelSubject;
use crate::simulation::model::{ModelSystem, SymbolicSubject};
use crate::smt::expression::{Reader, Sym, Views};
use crate::smt::layout::{StateCopy, StateLayout, StateRef};
use crate::smt::monitor::MonitorTable;
use crate::smt::relation::{
    applications, check_domains, count_applications, Application, CopyFacts, StepRelation,
};
use crate::smt::script::{Script, Sort, Term};
use crate::smt::SmtEncodingLimits;
use crate::smt::{Measure, Refusal, SmtEncodingRefusal, SmtQueryKind};

/// What a proof query is about (FR-315 Inputs).
#[derive(Clone, Copy)]
pub enum SmtItem<'a> {
    /// A checked temporal clause with its property form (FR-123).
    Clause(&'a CheckedTemporalClause),
    /// FR-124's deadlock-freedom item.
    DeadlockFreedom,
    /// FR-224's race-freedom item, whose accesses are steps of a memory
    /// model that this encoding has no form for.
    RaceFreedom,
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
        let (condition, _): (CheckedNode<'_>, usize) = self
            .node
            .condition(atom)
            .ok_or(SmtEncodingRefusal::fault("an atom with no condition"))?;
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

pub(super) fn fault(invariant: &'static str) -> SmtEncodingRefusal {
    SmtEncodingRefusal::fault(invariant)
}

pub(super) fn too_large(measure: Measure, value: u64, limit: u64) -> SmtEncodingRefusal {
    Refusal::TooLarge {
        measure,
        value,
        limit,
        setting: measure.setting(),
    }
    .into()
}

pub(super) fn script_bytes(value: u64, limit: u64) -> SmtEncodingRefusal {
    too_large(Measure::ScriptBytes, value, limit)
}

pub(super) fn monitor_constant(position: u64) -> Term {
    Term::constant(format!("m{position}"))
}

fn ended_constant(position: u64) -> Term {
    Term::constant(format!("h{position}"))
}

/// The literals of `letter` over `terms`: each atom's term when its bit is
/// set, its negation otherwise.
pub(super) fn letter_term(letter: u64, terms: &[Term]) -> Term {
    Term::all(terms.iter().enumerate().map(|(index, term)| {
        if letter >> index & 1 == 1 {
            term.clone()
        } else {
            term.clone().not()
        }
    }))
}

/// The printed disjunctions and conjunctions of FR-315 "Canonical printing":
/// one operand prints as itself and none as the constant.
pub(super) fn joined(head: &str, parts: &[String], empty: &str) -> String {
    match parts {
        [] => empty.to_owned(),
        [only] => only.clone(),
        _ => format!("({head} {})", parts.join(" ")),
    }
}

/// The printed disjuncts of one disjunction, each equal to an earlier one
/// printed once, stopping at the first byte past the script limit.
pub(super) struct Disjuncts {
    pub(super) parts: Vec<String>,
    seen: HashSet<String>,
    bytes: usize,
    max_bytes: u64,
}

impl Disjuncts {
    pub(super) fn new(max_bytes: u64) -> Self {
        Self {
            parts: Vec::new(),
            seen: HashSet::new(),
            bytes: 0,
            max_bytes,
        }
    }

    pub(super) fn push(&mut self, term: &Term) -> Result<(), SmtEncodingRefusal> {
        // A construct that cannot fail contributes no term.
        if term.is_truth(false) {
            return Ok(());
        }
        let text = term.text();
        if self.seen.contains(&text) {
            return Ok(());
        }
        self.bytes = self.bytes.saturating_add(text.len() + 1);
        if u64::try_from(self.bytes).unwrap_or(u64::MAX) > self.max_bytes {
            return Err(script_bytes(
                self.max_bytes.saturating_add(1),
                self.max_bytes,
            ));
        }
        self.seen.insert(text.clone());
        self.parts.push(text);
        Ok(())
    }
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
    limits: &SmtEncodingLimits,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    let clause = match item {
        SmtItem::Clause(clause) => Some(clause),
        SmtItem::DeadlockFreedom => None,
        SmtItem::RaceFreedom => {
            return Err(Refusal::ItemForm {
                form: "the race-freedom item, which reads a weak memory model's race summary",
            }
            .into())
        }
    };
    let mut bounded = None;
    let mut monitor = None;
    if let Some(clause) = clause {
        // Row 1: item-form.
        let form = clause.property_form().ok_or(Refusal::ItemForm {
            form: "a clause with no recorded property form",
        })?;
        if form == PropertyForm::Liveness {
            return Err(Refusal::ItemForm {
                form: "a TP-4 liveness clause, whose refutations are lassos",
            }
            .into());
        }
        if form == PropertyForm::BoundedMltl {
            if clause.activation() != Activation::Origin {
                return Err(Refusal::ItemForm {
                    form: "a TP-2 `on each` clause",
                }
                .into());
            }
            bounded = Some(bounded_shape(clause).ok_or(Refusal::ItemForm {
                form: "a TP-2 `on origin` clause other than a bounded `always` or `eventually` of one state predicate",
            })?);
        }
        if matches!(clause.activation(), Activation::Each { guarded: true }) {
            return Err(Refusal::ItemForm {
                form: "a clause guarded by a `when` condition",
            }
            .into());
        }
        // A TP-1, TP-3 or deadlock-freedom item binds infinite-trace; only a
        // TP-2 item binds false-extension (FR-315 "Internal faults").
        if bounded.is_none() && clause.profile() == TemporalProfile::EventPositionFalseExtension {
            return Err(SmtEncodingRefusal::fault(
                "a TP-1 or TP-3 item binds infinite-trace",
            ));
        }
        if bounded.is_none() {
            if form == PropertyForm::ReachableInvariant && invariant_atom(clause).is_none() {
                return Err(SmtEncodingRefusal::fault("a TP-1 item holds its one atom"));
            }
            if invariant_atom(clause).is_none() {
                let table =
                    MonitorTable::of(clause, limits.max_monitor_atoms, limits.max_monitor_states);
                if let Err(refusal @ SmtEncodingRefusal::Refused(Refusal::ItemTranslation(_))) =
                    &table
                {
                    return Err(refusal.clone());
                }
                if table.as_ref().is_ok_and(|table| table.atoms().is_empty()) {
                    return Err(SmtEncodingRefusal::fault("a TP-3 item has an atom"));
                }
                monitor = Some(table);
            }
        }
        // Row 2: profile.
        if !matches!(
            clause.profile(),
            TemporalProfile::InfiniteTrace | TemporalProfile::EventPositionFalseExtension
        ) {
            return Err(Refusal::Profile.into());
        }
        // Row 3: query-kind.
        if bounded.is_some() && !matches!(kind, SmtQueryKind::Unrolling { .. }) {
            return Err(Refusal::QueryKind {
                kind: kind.name(),
                form: "a TP-2 `on origin` item",
            }
            .into());
        }
        if let (Some((BoundedKind::Eventually, upper, _)), SmtQueryKind::Unrolling { depth }) =
            (bounded, kind)
        {
            if depth < upper {
                return Err(Refusal::QueryKind {
                    kind: kind.name(),
                    form: "an `eventually[0,u]` item below its horizon",
                }
                .into());
            }
        }
    }
    // Only a TP-2 item is read past the end of a finite behaviour.
    let infinite = bounded.is_none();
    // Rows 4 to 8: the subject.
    let system = ModelSystem::new(subject.simulation.clone()).map_err(Refusal::SubjectAdmission)?;
    let symbolic = system.symbolic()?;
    let layout = StateLayout::of(&symbolic)?;
    // Row 9: parameter-domain.
    check_domains(&symbolic)?;
    // Row 10: field-domain. The copy-0 reading expands what the query
    // expands, so it finds the refusal before the rows after it.
    let expands = !infinite || clause.is_none();
    let mut apps = None;
    let mut first = None;
    if count_applications(&symbolic) <= limits.max_applications {
        let built = applications(&symbolic, limits.max_applications)?;
        let read = CopyFacts::read(
            &symbolic,
            &layout,
            &built,
            &StateCopy::at(0),
            expands,
            limits.max_candidates,
        );
        if let Err(refusal @ SmtEncodingRefusal::Refused(Refusal::FieldDomain { .. })) = &read {
            return Err(refusal.clone());
        }
        first = Some(read);
        apps = Some(built);
    }

    // Rows 11 to 14: the instances.
    let mut instances = Vec::new();
    let mut node = None;
    if let Some(clause) = clause {
        let over = match &clause.over().1 {
            quire_exact::ValueType::Reference(over) => *over,
            _ => return Err(fault("an `over` that is no reference")),
        };
        let found = system
            .instances(&clause.over().1)
            .map_err(Refusal::RequiresBound)?;
        if found.is_empty() {
            return Err(Refusal::NoInstance.into());
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
                .ok_or_else(|| Refusal::InstanceType {
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
                .ok_or(Refusal::ItemForm {
                    form: "a clause the package does not emit",
                })?,
        );
    }
    if monitor.is_some() && instances.len() != 1 {
        return Err(Refusal::SeveralInstances {
            count: instances.len(),
        }
        .into());
    }

    // Row 15: the size measures, in order.
    let depth = kind.depth();
    let per_position = layout
        .objects()
        .iter()
        .flat_map(|object| object.fields())
        .map(|field| 1 + u64::from(field.is_optional()))
        .sum::<u64>()
        + u64::from(monitor.is_some())
        + u64::from(!infinite);
    let constants = depth.saturating_add(1).saturating_mul(per_position);
    if constants > limits.max_constants {
        return Err(too_large(
            Measure::Constants,
            constants,
            limits.max_constants,
        ));
    }
    let applications_count = count_applications(&symbolic);
    if applications_count > limits.max_applications {
        return Err(too_large(
            Measure::Applications,
            applications_count,
            limits.max_applications,
        ));
    }
    let property = match (clause, monitor) {
        (None, _) => Property::Deadlock,
        (Some(_), Some(table)) => Property::Monitor(table?),
        (Some(clause), None) => match (bounded, invariant_atom(clause)) {
            (Some((shape, upper, atom)), _) => Property::Bounded(shape, upper, atom),
            (None, Some(atom)) => Property::Invariant(atom),
            (None, None) => return Err(fault("an item with no property")),
        },
    };
    let (Some(apps), Some(first)) = (apps, first) else {
        return Err(fault("applications read before their count"));
    };
    // Row 15 `candidates`, then row 16 `out-of-fragment` of the reading.
    let first = first?;
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
        first,
        limits,
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
    /// The facts of copy 0, read before the instances were checked.
    first: CopyFacts,
    limits: &'q SmtEncodingLimits,
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

    /// A failure `failure` at `position`, which needs the copy to be live:
    /// `(and (not h<p>) F)` under false-extension.
    fn live(&self, position: u64, failure: Term) -> Term {
        match self.ended(position) {
            Some(ended) => Term::all([ended.not(), failure]),
            None => failure,
        }
    }

    /// The facts of copy `position`.
    fn facts(&self, position: u64) -> Result<CopyFacts, SmtEncodingRefusal> {
        if position == 0 {
            return Ok(self.first.clone());
        }
        CopyFacts::read(
            self.subject,
            self.layout,
            &self.apps,
            &StateCopy::at(position),
            !self.infinite || matches!(self.property, Property::Deadlock),
            self.limits.max_candidates,
        )
    }

    /// The value and definedness of `atom` of `instance` at `position`.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: usize,
        position: u64,
    ) -> Result<(Term, Term), SmtEncodingRefusal> {
        let atoms = self
            .atoms
            .as_ref()
            .ok_or_else(|| fault("an item with no atoms"))?;
        atoms.read(atom, instance, &StateCopy::at(position))
    }

    /// An atom of the monitor's letter: false past the end of a finite
    /// behaviour.
    fn letter_atom(&self, atom: FormulaNodeId, position: u64) -> Result<Term, SmtEncodingRefusal> {
        let (value, _) = self.atom(atom, 0, position)?;
        Ok(self.live(position, value))
    }

    fn instances(&self) -> usize {
        self.atoms.as_ref().map_or(0, |atoms| atoms.instances.len())
    }

    /// Every distinctness assertion is at least `(assert (not x))\n`, so the
    /// pairs alone can pass the limit long before any is built.
    fn check_pairs(&self, depth: u64) -> Result<(), SmtEncodingRefusal> {
        let pairs = u128::from(depth) * u128::from(depth.saturating_add(1)) / 2;
        if pairs.saturating_mul(16) > u128::from(self.limits.max_script_bytes) {
            return Err(script_bytes(
                self.limits.max_script_bytes.saturating_add(1),
                self.limits.max_script_bytes,
            ));
        }
        Ok(())
    }

    fn check_size(&self, script: &Script) -> Result<(), SmtEncodingRefusal> {
        if script.exceeded() {
            Err(script_bytes(
                self.limits.max_script_bytes.saturating_add(1),
                self.limits.max_script_bytes,
            ))
        } else {
            Ok(())
        }
    }

    fn script(&self) -> Result<Script, SmtEncodingRefusal> {
        let depth = self.kind.depth();
        let mut script = Script::with_max_bytes(self.limits.max_script_bytes);
        for position in 0..=depth {
            let copy = StateCopy::at(position);
            self.layout.declare(&copy, &mut script);
            if self.monitor().is_some() {
                script.declare(&format!("m{position}"), Sort::Int);
            }
            if !self.infinite {
                script.declare(&format!("h{position}"), Sort::Bool);
            }
            self.check_size(&script)?;
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
            self.check_size(&script)?;
        }
        if self.kind.starts_initial() {
            script.assert(&self.layout.initial(&StateCopy::at(0), self.subject)?);
            if let Some(ended) = self.ended(0) {
                script.assert(&ended.not());
            }
        }
        if depth > 0 && matches!(self.kind, SmtQueryKind::InductionStep { .. }) {
            // Read the first step and position once, so that a construct
            // outside the fragment is refused before the size is, then see
            // whether the pairwise distinctness alone passes the limit.
            StepRelation::build(
                self.subject,
                self.layout,
                &self.apps,
                &StateCopy::at(0),
                &StateCopy::at(1),
                self.infinite,
            )?;
            let facts = self.facts(0)?;
            self.disjuncts_at(0, &facts)?;
            self.check_pairs(depth)?;
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
            script.assert(&self.step(step, &relation)?);
            self.check_size(&script)?;
        }
        if let Some(table) = self.monitor() {
            self.assert_monitor(table, &mut script)?;
        }
        if matches!(self.kind, SmtQueryKind::InductionStep { .. }) {
            self.assert_distinct(&mut script)?;
            for position in 0..depth {
                let facts = self.facts(position)?;
                let mut parts = Disjuncts::new(self.limits.max_script_bytes);
                for disjunct in self.disjuncts_at(position, &facts)? {
                    if !disjunct.is_truth(false) {
                        parts.push(&disjunct.not())?;
                    }
                }
                script.assert_text(&joined("and", &parts.parts, "true"));
                self.check_size(&script)?;
            }
        }
        self.assert_negated(&mut script)?;
        self.check_size(&script)?;
        if script.closing_command_exceeded() {
            return Err(script_bytes(
                self.limits.max_script_bytes.saturating_add(1),
                self.limits.max_script_bytes,
            ));
        }
        Ok(script)
    }

    /// The step assertion from `step` to `step + 1`.
    fn step(&self, step: u64, relation: &StepRelation) -> Result<Term, SmtEncodingRefusal> {
        if self.infinite {
            return Ok(relation.transition());
        }
        let (was, is) = (ended_constant(step), ended_constant(step + 1));
        let terminal = self
            .facts(step)?
            .terminal()
            .ok_or_else(|| fault("a terminal read that was not asked for"))?;
        Ok(Term::all([
            is.clone().equal(Term::any([was, terminal])),
            Term::any([is, relation.operation_step()]),
        ]))
    }

    /// The monitor's implications, position by position.
    fn assert_monitor(
        &self,
        table: &MonitorTable,
        script: &mut Script,
    ) -> Result<(), SmtEncodingRefusal> {
        let depth = self.kind.depth();
        for position in 0..=depth {
            let mut letter = Vec::new();
            for atom in table.atoms() {
                letter.push(self.letter_atom(*atom, position)?);
            }
            for step in table.moves() {
                let reads = letter_term(step.letter, &letter);
                let reached = monitor_constant(position).equal(Term::int(
                    i64::try_from(step.to).map_err(|_| fault("a monitor state index"))?,
                ));
                let condition = if position == 0 {
                    if !self.kind.starts_initial() || step.from != table.initial() {
                        continue;
                    }
                    reads
                } else {
                    let before = monitor_constant(position - 1).equal(Term::int(
                        i64::try_from(step.from).map_err(|_| fault("a monitor state index"))?,
                    ));
                    Term::all([before, reads])
                };
                script.assert(&condition.implies(reached));
            }
            self.check_size(script)?;
        }
        Ok(())
    }

    /// The copies at positions `0..=depth` are pairwise distinct: one
    /// assertion per pair `(j, k)`, `j < k`, `(not (and Q1 ... Qn))` with the
    /// field equalities of copy `k` with copy `j`, then the monitor states'.
    fn assert_distinct(&self, script: &mut Script) -> Result<(), SmtEncodingRefusal> {
        let depth = self.kind.depth();
        self.check_pairs(depth)?;
        for left in 0..=depth {
            for right in left + 1..=depth {
                let mut agreements = self
                    .layout
                    .agreements(&StateCopy::at(right), &StateCopy::at(left));
                if self.monitor().is_some() {
                    agreements.push(monitor_constant(right).equal(monitor_constant(left)));
                }
                script.assert(&Term::all(agreements).not());
                self.check_size(script)?;
            }
        }
        Ok(())
    }

    /// The violations of the property at `position`, as disjuncts: for an
    /// `always` item `h<p>` and then the negation of each instance's atom;
    /// for the monitor the rejecting state; for deadlock-freedom a terminal
    /// copy no declaration intends.
    fn violations(
        &self,
        position: u64,
        facts: &CopyFacts,
    ) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let mut violations = Vec::new();
        match &self.property {
            Property::Invariant(atom) | Property::Bounded(BoundedKind::Always, _, atom) => {
                if matches!(&self.property, Property::Bounded(_, upper, _) if position > *upper) {
                    return Ok(violations);
                }
                violations.extend(self.ended(position));
                for instance in 0..self.instances() {
                    violations.push(self.atom(*atom, instance, position)?.0.not());
                }
            }
            Property::Bounded(BoundedKind::Eventually, ..) => {}
            Property::Monitor(table) => {
                if let Some(rejecting) = table.rejecting() {
                    violations.push(monitor_constant(position).equal(Term::int(
                        i64::try_from(rejecting).map_err(|_| fault("a monitor state index"))?,
                    )));
                }
            }
            Property::Deadlock => {
                let terminal = facts
                    .terminal()
                    .ok_or_else(|| fault("a terminal read that was not asked for"))?;
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
                        return Err(fault("a terminal predicate with no state object"));
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

    /// The disjuncts of the negated property at `position`, in FR-315's
    /// order: the violations, the atoms' definedness failures, the
    /// preconditions' failures, the postconditions' failures, and `P`'s.
    fn disjuncts_at(
        &self,
        position: u64,
        facts: &CopyFacts,
    ) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let mut disjuncts = Vec::new();
        if position <= self.horizon() {
            disjuncts.extend(self.violations(position, facts)?);
            match &self.property {
                Property::Invariant(atom) | Property::Bounded(_, _, atom) => {
                    for instance in 0..self.instances() {
                        let failure = self.atom(*atom, instance, position)?.1.not();
                        disjuncts.push(self.live(position, failure));
                    }
                }
                Property::Monitor(table) => {
                    for atom in table.atoms() {
                        let failure = self.atom(*atom, 0, position)?.1.not();
                        disjuncts.push(self.live(position, failure));
                    }
                }
                Property::Deadlock => {}
            }
        }
        for failure in facts.precondition_failures() {
            disjuncts.push(self.live(position, failure.clone()));
        }
        for failure in facts.postcondition_failures() {
            disjuncts.push(self.live(position, failure.clone()));
        }
        if matches!(self.property, Property::Deadlock) {
            let terminal = facts
                .terminal()
                .ok_or_else(|| fault("a terminal read that was not asked for"))?;
            disjuncts.push(Term::all([terminal, self.intended(position)?.1]));
        }
        Ok(disjuncts)
    }

    /// The negated property: one disjunction, a disjunct equal to an earlier
    /// one printed once.
    fn assert_negated(&self, script: &mut Script) -> Result<(), SmtEncodingRefusal> {
        let depth = self.kind.depth();
        let positions = if matches!(self.kind, SmtQueryKind::InductionStep { .. }) {
            depth..=depth
        } else {
            0..=depth
        };
        let mut parts = Disjuncts::new(self.limits.max_script_bytes);
        if let Property::Bounded(BoundedKind::Eventually, _, atom) = &self.property {
            // Per instance, the atom is false or past the end at every
            // position of the horizon.
            for instance in 0..self.instances() {
                let mut every = Vec::new();
                for position in 0..=self.horizon() {
                    let missed = self.atom(*atom, instance, position)?.0.not();
                    every.push(Term::any(self.ended(position).into_iter().chain([missed])));
                }
                parts.push(&Term::all(every))?;
            }
        }
        for position in positions {
            let facts = self.facts(position)?;
            for disjunct in self.disjuncts_at(position, &facts)? {
                parts.push(&disjunct)?;
            }
        }
        script.assert_text(&joined("or", &parts.parts, "false"));
        Ok(())
    }
}
