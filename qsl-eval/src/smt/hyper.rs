// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Hyper self-composition queries": the script of one `k`-induction
//! query of an HP-2 clause's self-composition product.
//!
//! The script is, in order: `(set-logic QF_LIA)`; the `declare-const`
//! commands position by position (each trace variable's copy `s<i>.<t>` in
//! quantifier order, then `m<i>`); the range assertions; for `InductionBase`
//! each variable's initial-state predicate on its copy 0; one joint-step
//! assertion per step; the monitor implications; for `InductionStep` the
//! pairwise distinctness and the property assertions at the positions before
//! the last; the negated property; `(check-sat)`.
//!
//! The identity checks of `encode_hyper_query` (hyper-form, subject-binding,
//! instance-binding, product-identity) are `qsl-replay`'s; this entry runs
//! the stages after them.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Not;

use qsl_semantics::check::{
    CheckedFormula, CheckedHyperClause, FormulaNode, FormulaNodeId, UnaryTemporal,
};
use qsl_semantics::model::clause_observation::Observation;
use quire_exact::{Integer, Meter, ValueType};
use quire_semantic_value::location::Origin;
use quire_semantic_value::model_transition::ModelTransition;

use crate::hyper::{CheckedStepMatch, MatchReading, ParameterBinding, StepLabel, StepMatch};
use crate::model_subject::ModelSubject;
use crate::simulation::model::{ModelSystem, SymbolicSubject};
use crate::smt::expression::{Components, Reader, Sym, Views};
use crate::smt::layout::{StateCopy, StateLayout, StateRef};
use crate::smt::monitor::MonitorTable;
use crate::smt::query::{
    fault, joined, letter_term, monitor_constant, script_bytes, too_large, Disjuncts,
};
use crate::smt::relation::{
    applications, check_domains, count_applications, Application, CopyFacts, StepRelation,
};
use crate::smt::script::{Script, Sort, Term};
use crate::smt::{Measure, Refusal, SmtEncodingLimits, SmtEncodingRefusal, SmtQueryKind};

/// What the script of one hyper query is encoded from: the request of
/// `encode_hyper_query` after its identity checks, and the query kind.
pub struct HyperScriptRequest<'r, 'a> {
    /// The checked hyper clause, HP-2.
    pub clause: &'r CheckedHyperClause,
    /// One subject per alias, in `clause.aliases()` order, over one checked
    /// package.
    pub subjects: &'r [ModelSubject<'a>],
    /// One `(parameter, key)` pair per object parameter, in parameter order,
    /// each key in the parameter's universe.
    pub instance: &'r [(String, String)],
    /// The budget `μ_U` is evaluated against.
    pub meter: &'r mut Meter,
    /// The encoding limits.
    pub encoding: SmtEncodingLimits,
    /// The query kind.
    pub kind: SmtQueryKind,
}

/// How the body's property is read at a joint position.
enum Property {
    /// `always` over a state predicate: the predicate at the position. It is
    /// the operand and the `holds` atoms under it.
    Predicate(FormulaNodeId, Vec<FormulaNodeId>),
    /// Any other safety body: the monitor not rejecting.
    Monitor(MonitorTable),
}

/// One joint disjunct: the component disjunct of each trace variable, and
/// whether `μ_U` is undefined on it.
struct Joint {
    digits: Vec<usize>,
    undefined: bool,
}

/// Encode the query `request.kind` of the HP-2 product `request` names as an
/// SMT-LIB 2 script in the canonical printing (FR-315).
///
/// # Errors
///
/// [`SmtEncodingRefusal`], with the code of FR-315's table, naming the
/// construct and its locus; no script is written.
pub fn encode_hyper_script(
    request: HyperScriptRequest<'_, '_>,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    let HyperScriptRequest {
        clause,
        subjects,
        instance,
        meter,
        encoding,
        kind,
    } = request;
    let limits = &encoding;
    let name = clause.name();

    // A subject FR-120 does not admit is `subject-binding` at its alias.
    let mut systems = Vec::new();
    for (index, alias) in clause.aliases().iter().enumerate() {
        let subject = subjects
            .get(index)
            .ok_or_else(|| fault("one subject for each alias"))?;
        let system =
            ModelSystem::new(subject.simulation.clone()).map_err(|_| Refusal::SubjectBinding {
                alias: Some(alias.as_str().to_owned()),
            })?;
        systems.push(system);
    }
    if !clause.body().is_safety() {
        return Err(Refusal::HyperBody {
            clause: name.to_owned(),
        }
        .into());
    }
    if matches!(kind, SmtQueryKind::Unrolling { .. }) {
        return Err(Refusal::QueryKind {
            kind: kind.name(),
            form: "an HP-2 item",
        }
        .into());
    }

    // The subjects of the trace variables, one reading for each alias used.
    let quantifiers = clause.quantifiers();
    let variables = quantifiers.len();
    let mut used: Vec<usize> = Vec::new();
    let mut dense = Vec::new();
    for quantifier in quantifiers {
        let slot = clause
            .aliases()
            .iter()
            .position(|alias| *alias == quantifier.alias)
            .ok_or_else(|| fault("a quantifier names an alias of the clause"))?;
        let at = match used.iter().position(|seen| *seen == slot) {
            Some(at) => at,
            None => {
                used.push(slot);
                used.len() - 1
            }
        };
        dense.push(at);
    }
    let mut symbolics = Vec::new();
    for slot in &used {
        let system = systems
            .get(*slot)
            .ok_or_else(|| fault("a system for each alias"))?;
        symbolics.push(system.symbolic()?);
    }

    let property = shape(clause.body());

    // `too-large` for `constants`, over every copy.
    let depth = kind.depth();
    let monitored = property.is_none();
    let per_position = dense
        .iter()
        .map(|at| fields_of(&symbolics[*at]))
        .sum::<u64>()
        + u64::from(monitored);
    let constants = depth.saturating_add(1).saturating_mul(per_position);
    if constants > limits.max_constants {
        return Err(too_large(
            Measure::Constants,
            constants,
            limits.max_constants,
        ));
    }

    // `field-sort`, `parameter-domain` and `field-domain`, per copy.
    let mut layouts = Vec::new();
    for symbolic in &symbolics {
        layouts.push(StateLayout::of(symbolic)?);
    }
    for symbolic in &symbolics {
        check_domains(symbolic)?;
    }
    let mut built: Vec<Option<Vec<Application<'_, '_>>>> = Vec::new();
    for symbolic in &symbolics {
        built.push(
            (count_applications(symbolic) <= limits.max_applications)
                .then(|| applications(symbolic, limits.max_applications))
                .transpose()?,
        );
    }
    let mut firsts = Vec::new();
    for (variable, quantifier) in quantifiers.iter().enumerate() {
        let at = dense[variable];
        let Some(apps) = &built[at] else {
            firsts.push(None);
            continue;
        };
        let read = CopyFacts::read(
            &symbolics[at],
            &layouts[at],
            apps,
            &StateCopy::hyper(0, &quantifier.variable),
            false,
            limits.max_candidates,
        );
        if let Err(refusal @ SmtEncodingRefusal::Refused(Refusal::FieldDomain { .. })) = &read {
            return Err(refusal.clone());
        }
        firsts.push(Some(read));
    }

    // `match-conjunct`: every tuple of component disjuncts, `μ_U` evaluated
    // on each.
    let mut apps_of = Vec::new();
    for (at, symbolic) in symbolics.iter().enumerate() {
        match &built[at] {
            Some(apps) => apps_of.push(apps),
            None => {
                return Err(too_large(
                    Measure::Applications,
                    count_applications(symbolic),
                    limits.max_applications,
                ))
            }
        }
    }
    let mut transitions: Vec<Vec<ModelTransition>> = Vec::new();
    let mut names: Vec<Vec<String>> = Vec::new();
    for (at, symbolic) in symbolics.iter().enumerate() {
        let mut shown = Vec::new();
        let mut identities = Vec::new();
        for application in apps_of[at] {
            identities.push(application.transition(symbolic)?);
            shown.push(application.label(symbolic));
        }
        shown.push("stutter".to_owned());
        transitions.push(identities);
        names.push(shown);
    }
    let sizes: Vec<usize> = dense.iter().map(|at| names[*at].len()).collect();
    let universal = clause.mu_universal();
    let mut bindings = Vec::new();
    for (parameter, (_, key)) in clause.parameters().iter().zip(instance) {
        let slot = clause
            .aliases()
            .iter()
            .position(|alias| *alias == parameter.alias)
            .ok_or_else(|| fault("a parameter names an alias of the clause"))?;
        let listed = systems
            .get(slot)
            .ok_or_else(|| fault("a system for each alias"))?
            .instances(&ValueType::Reference(parameter.population.object()))
            .map_err(|_| fault("an instance the identity checks list"))?;
        let bound = listed
            .into_iter()
            .find(|candidate| candidate.key == *key)
            .ok_or_else(|| fault("an instance the identity checks list"))?;
        bindings.push(ParameterBinding {
            parameter: parameter.name.clone(),
            instance: bound,
        });
    }
    if universal.is_empty() {
        let product = sizes
            .iter()
            .fold(1u64, |total, size| total.saturating_mul(*size as u64));
        if product > limits.max_applications {
            return Err(too_large(
                Measure::Applications,
                product,
                limits.max_applications,
            ));
        }
    }
    let first_package = subjects
        .first()
        .ok_or_else(|| fault("one subject for each alias"))?
        .simulation
        .package;
    let matcher = CheckedStepMatch::new(first_package, clause);
    let mut kept: Vec<Joint> = Vec::new();
    let mut digits = vec![0usize; variables];
    loop {
        let tuple = |digits: &[usize]| -> Vec<String> {
            digits
                .iter()
                .enumerate()
                .map(|(variable, digit)| names[dense[variable]][*digit].clone())
                .collect()
        };
        let refused = |conjunct: usize, digits: &[usize]| -> SmtEncodingRefusal {
            let Some(predicate) = universal.get(conjunct) else {
                return fault("a refused conjunct is a conjunct of mu_universal");
            };
            let site = match &predicate.condition().0.location().origin {
                Origin::TypeDeclaration { name } => name.clone(),
                _ => format!("hyper match {conjunct}"),
            };
            Refusal::MatchConjunct {
                clause: name.to_owned(),
                conjunct,
                site,
                tuple: tuple(digits),
            }
            .into()
        };
        let reading = if universal.is_empty() {
            Ok(MatchReading::Holds)
        } else {
            let labels: Vec<StepLabel<'_>> = digits
                .iter()
                .enumerate()
                .map(|(variable, digit)| {
                    let at = dense[variable];
                    match transitions[at].get(*digit) {
                        Some(transition) => StepLabel::Operation(transition),
                        None => StepLabel::Stutter,
                    }
                })
                .collect();
            matcher.universal(&bindings, &labels, meter)
        };
        match reading {
            Ok(MatchReading::Holds) => kept.push(Joint {
                digits: digits.clone(),
                undefined: false,
            }),
            Ok(MatchReading::Fails) => {}
            Ok(MatchReading::Undefined { conjunct, .. }) => {
                if conjunct >= universal.len() {
                    return Err(fault("an undefined conjunct is a conjunct of mu_universal"));
                }
                kept.push(Joint {
                    digits: digits.clone(),
                    undefined: true,
                });
            }
            Ok(MatchReading::Refused { conjunct, .. }) => return Err(refused(conjunct, &digits)),
            Ok(MatchReading::Fault(internal)) => return Err(SmtEncodingRefusal::Fault(internal)),
            Err(incomplete) => return Err(refused(incomplete.conjunct, &digits)),
        }
        // The first variable varies slowest.
        let mut position = variables;
        loop {
            let Some(previous) = position.checked_sub(1) else {
                break;
            };
            position = previous;
            digits[position] += 1;
            if digits[position] < sizes[position] {
                break;
            }
            digits[position] = 0;
        }
        if digits.iter().all(|digit| *digit == 0) {
            break;
        }
    }

    // `too-large` for `applications`, over the joint disjuncts of one step.
    let joint_count = kept.len() as u64;
    if joint_count > limits.max_applications {
        return Err(too_large(
            Measure::Applications,
            joint_count,
            limits.max_applications,
        ));
    }

    // `monitor-atoms` and `monitor-states`.
    let property = match property {
        Some((operand, atoms)) => Property::Predicate(operand, atoms),
        None => Property::Monitor(
            match MonitorTable::over(
                clause.body(),
                clause.profile(),
                limits.max_monitor_atoms,
                limits.max_monitor_states,
            ) {
                Err(SmtEncodingRefusal::Refused(Refusal::ItemTranslation(_))) => {
                    return Err(fault("a safety body translates to a monitor"))
                }
                other => other?,
            },
        ),
    };

    // `candidates`, then `out-of-fragment` of the operations' clauses.
    let mut first_facts = Vec::new();
    let mut pending = None;
    for read in firsts {
        match read.ok_or_else(|| fault("applications read before their count"))? {
            Ok(facts) => first_facts.push(facts),
            Err(refusal @ SmtEncodingRefusal::Refused(Refusal::OutOfFragment { .. })) => {
                pending.get_or_insert(refusal);
            }
            Err(refusal) => return Err(refusal),
        }
    }

    Product {
        clause,
        instance,
        kind,
        limits,
        variables: quantifiers
            .iter()
            .map(|quantifier| quantifier.variable.as_str())
            .collect(),
        dense,
        symbolics: &symbolics,
        layouts: &layouts,
        apps: apps_of,
        kept,
        property,
        first_facts,
        pending,
    }
    .script()
    .map(Script::render)
}

/// The fields of the subject's objects, each optional field counted twice:
/// the constants of one copy.
fn fields_of(subject: &SymbolicSubject<'_>) -> u64 {
    subject
        .objects
        .iter()
        .flat_map(|object| &object.fields)
        .map(|field| 1 + u64::from(field.optional))
        .sum()
}

/// The operand and `holds` atoms of a body that is `always` over a state
/// predicate: constants, `holds` atoms and the Boolean connectives, with no
/// temporal operator under the `always`.
fn shape(body: &CheckedFormula) -> Option<(FormulaNodeId, Vec<FormulaNodeId>)> {
    let nodes = body.nodes();
    let FormulaNode::Unary {
        operator: UnaryTemporal::Always,
        interval: None,
        operand,
    } = nodes.get(body.root().index())?
    else {
        return None;
    };
    let mut atoms = BTreeSet::new();
    let mut pending = vec![*operand];
    while let Some(id) = pending.pop() {
        match nodes.get(id.index())? {
            FormulaNode::Constant(_) => {}
            FormulaNode::Holds { .. } => {
                atoms.insert(id.index());
            }
            FormulaNode::Not(only) => pending.push(*only),
            FormulaNode::And(left, right)
            | FormulaNode::Or(left, right)
            | FormulaNode::Implies(left, right) => pending.extend([*left, *right]),
            _ => return None,
        }
    }
    let atoms = atoms
        .into_iter()
        .map(|index| body.node_id(index))
        .collect::<Option<Vec<_>>>()?;
    Some((*operand, atoms))
}

/// One query under construction.
struct Product<'q, 'a> {
    clause: &'q CheckedHyperClause,
    instance: &'q [(String, String)],
    kind: SmtQueryKind,
    limits: &'q SmtEncodingLimits,
    /// The trace variables in quantifier order.
    variables: Vec<&'q str>,
    /// For each trace variable, the index of its subject's reading.
    dense: Vec<usize>,
    symbolics: &'q [SymbolicSubject<'a>],
    layouts: &'q [StateLayout],
    apps: Vec<&'q Vec<Application<'q, 'a>>>,
    kept: Vec<Joint>,
    property: Property,
    /// The facts of each trace variable's copy 0.
    first_facts: Vec<CopyFacts>,
    /// An `out-of-fragment` refusal of an operation's clause, kept until the
    /// atoms are known to break no invariant.
    pending: Option<SmtEncodingRefusal>,
}

impl Product<'_, '_> {
    fn monitor(&self) -> Option<&MonitorTable> {
        match &self.property {
            Property::Monitor(table) => Some(table),
            Property::Predicate(..) => None,
        }
    }

    fn copy(&self, position: u64, variable: usize) -> StateCopy {
        StateCopy::hyper(position, self.variables[variable])
    }

    /// Every `holds` node of the body.
    fn holds(&self) -> Vec<FormulaNodeId> {
        let body = self.clause.body();
        body.nodes()
            .iter()
            .enumerate()
            .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
            .filter_map(|(index, _)| body.node_id(index))
            .collect()
    }

    /// The value and definedness of `atom` at joint position `position`,
    /// read symbolically: slot (`t`, `p`) is the layout object of the key the
    /// instance holds for `p` in `t`'s subject, and each model read is read
    /// in the copy of the component `component_reads` records for it.
    fn atom(&self, atom: FormulaNodeId, position: u64) -> Result<(Term, Term), SmtEncodingRefusal> {
        let clause = self.clause;
        let (condition, slots) = clause
            .condition(atom)
            .ok_or_else(|| fault("a holds atom has a condition"))?;
        let reads = clause
            .component_reads(atom)
            .ok_or_else(|| fault("a holds atom has component reads"))?;
        if slots != clause.atom_slots().len() {
            return Err(fault("a condition binds one value for each atom slot"));
        }
        let mut bound = Vec::new();
        for slot in clause.atom_slots() {
            let at = *self
                .dense
                .get(slot.trace)
                .ok_or_else(|| fault("an atom slot names a trace variable"))?;
            bound.push(Some(Sym::Object(self.object(at, slot.parameter)?)));
        }
        let site = match &condition.location().origin {
            Origin::TypeDeclaration { name } => name.clone(),
            _ => clause.name().to_owned(),
        };
        let states: Vec<StateRef> = (0..self.variables.len())
            .map(|variable| StateRef::Copy(self.copy(position, variable)))
            .collect();
        let views = Views::of(states[0].clone());
        let first = self.dense[0];
        Reader::new(
            &self.layouts[first],
            &self.symbolics[first],
            &site,
            Observation::Current,
            Vec::new(),
            &views,
            bound,
        )
        .in_components(Components {
            layouts: self
                .dense
                .iter()
                .map(|at| (&self.layouts[*at], &self.symbolics[*at]))
                .collect(),
            states,
            reads,
            variables: self
                .variables
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
        })
        .boolean(condition)
    }

    /// The layout object of the subject at `at` that the instance binds
    /// parameter `parameter` to.
    fn object(&self, at: usize, parameter: usize) -> Result<usize, SmtEncodingRefusal> {
        let declared = self
            .clause
            .parameters()
            .get(parameter)
            .ok_or_else(|| fault("an atom slot names a parameter"))?;
        let key = self
            .instance_key(&declared.name)
            .ok_or_else(|| fault("the instance binds every parameter"))?;
        self.symbolics[at]
            .objects
            .iter()
            .position(|object| object.key == key)
            .ok_or_else(|| fault("an instance key names an object of the layout"))
    }

    fn instance_key(&self, parameter: &str) -> Option<&str> {
        self.instance
            .iter()
            .find(|(name, _)| name == parameter)
            .map(|(_, key)| key.as_str())
    }

    fn script(&self) -> Result<Script, SmtEncodingRefusal> {
        let depth = self.kind.depth();
        let variables = self.variables.len();
        // An atom that breaks an invariant is a fault however many other
        // constructs of the stage are refused.
        let mut refused = self.pending.clone();
        for atom in self.holds() {
            match self.atom(atom, 0) {
                Err(fault @ SmtEncodingRefusal::Fault(_)) => return Err(fault),
                Err(refusal) => {
                    refused.get_or_insert(refusal);
                }
                Ok(_) => {}
            }
        }
        if let Some(refusal) = refused {
            return Err(refusal);
        }

        // The joint steps.
        let mut joint: Vec<Vec<Term>> = Vec::new();
        for step in 0..depth {
            let mut components = Vec::new();
            for variable in 0..variables {
                let at = self.dense[variable];
                let relation = StepRelation::build(
                    &self.symbolics[at],
                    &self.layouts[at],
                    self.apps[at],
                    &self.copy(step, variable),
                    &self.copy(step + 1, variable),
                    true,
                )?;
                components.push(relation.disjuncts().cloned().collect::<Vec<_>>());
            }
            joint.push(
                self.kept
                    .iter()
                    .map(|tuple| {
                        Term::all(
                            tuple
                                .digits
                                .iter()
                                .enumerate()
                                .map(|(variable, digit)| components[variable][*digit].clone()),
                        )
                    })
                    .collect(),
            );
        }
        let mut facts = vec![self.first_facts.clone()];
        for position in 1..=depth {
            let mut at_position = Vec::new();
            for variable in 0..variables {
                let at = self.dense[variable];
                at_position.push(CopyFacts::read(
                    &self.symbolics[at],
                    &self.layouts[at],
                    self.apps[at],
                    &self.copy(position, variable),
                    false,
                    self.limits.max_candidates,
                )?);
            }
            facts.push(at_position);
        }

        let mut script = Script::with_max_bytes(self.limits.max_script_bytes);
        for position in 0..=depth {
            for variable in 0..variables {
                self.layouts[self.dense[variable]]
                    .declare(&self.copy(position, variable), &mut script);
            }
            if self.monitor().is_some() {
                script.declare(&format!("m{position}"), Sort::Int);
            }
            self.check_size(&script)?;
        }
        for position in 0..=depth {
            for variable in 0..variables {
                for range in
                    self.layouts[self.dense[variable]].ranges(&self.copy(position, variable))
                {
                    script.assert(&range);
                }
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
            for variable in 0..variables {
                let at = self.dense[variable];
                script.assert(
                    &self.layouts[at].initial(&self.copy(0, variable), &self.symbolics[at])?,
                );
            }
        }
        if matches!(self.kind, SmtQueryKind::InductionStep { .. }) {
            self.check_pairs(depth)?;
        }
        for step in &joint {
            script.assert(&Term::any(step.iter().cloned()));
            self.check_size(&script)?;
        }
        if let Some(table) = self.monitor() {
            self.assert_monitor(table, &mut script)?;
        }
        if matches!(self.kind, SmtQueryKind::InductionStep { .. }) {
            self.assert_distinct(&mut script)?;
            for position in 0..depth {
                let mut parts = Disjuncts::new(self.limits.max_script_bytes);
                for disjunct in self.disjuncts_at(position, &facts, &joint)? {
                    if !disjunct.is_truth(false) {
                        parts.push(&disjunct.not())?;
                    }
                }
                script.assert_text(&joined("and", &parts.parts, "true"));
                self.check_size(&script)?;
            }
        }
        let positions = if matches!(self.kind, SmtQueryKind::InductionStep { .. }) {
            depth..=depth
        } else {
            0..=depth
        };
        let mut parts = Disjuncts::new(self.limits.max_script_bytes);
        for position in positions {
            for disjunct in self.disjuncts_at(position, &facts, &joint)? {
                parts.push(&disjunct)?;
            }
        }
        script.assert_text(&joined("or", &parts.parts, "false"));
        self.check_size(&script)?;
        if script.closing_command_exceeded() {
            return Err(script_bytes(
                self.limits.max_script_bytes.saturating_add(1),
                self.limits.max_script_bytes,
            ));
        }
        Ok(script)
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
                letter.push(self.atom(*atom, position)?.0);
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

    /// The joint states at positions `0..=depth` are pairwise distinct: one
    /// assertion per pair `(j, k)`, `j < k`, `(not (and Q1 ... Qn))` with the
    /// field equalities of copy `k` with copy `j` of each trace variable in
    /// quantifier order, then the monitor states'.
    fn assert_distinct(&self, script: &mut Script) -> Result<(), SmtEncodingRefusal> {
        let depth = self.kind.depth();
        self.check_pairs(depth)?;
        for left in 0..=depth {
            for right in left + 1..=depth {
                let mut agreements = Vec::new();
                for variable in 0..self.variables.len() {
                    agreements.extend(
                        self.layouts[self.dense[variable]]
                            .agreements(&self.copy(right, variable), &self.copy(left, variable)),
                    );
                }
                if self.monitor().is_some() {
                    agreements.push(monitor_constant(right).equal(monitor_constant(left)));
                }
                script.assert(&Term::all(agreements).not());
                self.check_size(script)?;
            }
        }
        Ok(())
    }

    /// The disjuncts of the negated property at `position`: the violation,
    /// the atoms' definedness failures, the operations' precondition and
    /// postcondition failures of each trace variable's copy, and the
    /// undefined-match failures of the step into the position.
    fn disjuncts_at(
        &self,
        position: u64,
        facts: &[Vec<CopyFacts>],
        joint: &[Vec<Term>],
    ) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let mut disjuncts = Vec::new();
        let mut definedness = Vec::new();
        match &self.property {
            Property::Predicate(operand, atoms) => {
                let mut values = BTreeMap::new();
                for atom in atoms {
                    let (value, defined) = self.atom(*atom, position)?;
                    values.insert(atom.index(), value);
                    definedness.push(defined.not());
                }
                disjuncts.push(self.predicate(*operand, &values)?.not());
            }
            Property::Monitor(table) => {
                if let Some(rejecting) = table.rejecting() {
                    disjuncts.push(monitor_constant(position).equal(Term::int(
                        i64::try_from(rejecting).map_err(|_| fault("a monitor state index"))?,
                    )));
                }
                for atom in table.atoms() {
                    definedness.push(self.atom(*atom, position)?.1.not());
                }
            }
        }
        disjuncts.extend(definedness);
        for copy in facts
            .get(usize::try_from(position).unwrap_or(usize::MAX))
            .into_iter()
            .flatten()
        {
            disjuncts.extend(copy.precondition_failures().iter().cloned());
            disjuncts.extend(copy.postcondition_failures().iter().cloned());
        }
        if let Some(step) = position.checked_sub(1) {
            let terms = joint
                .get(usize::try_from(step).unwrap_or(usize::MAX))
                .ok_or_else(|| fault("a joint step before each position"))?;
            for (term, tuple) in terms.iter().zip(&self.kept) {
                if tuple.undefined {
                    disjuncts.push(term.clone());
                }
            }
        }
        Ok(disjuncts)
    }

    /// The state predicate `operand` over the atoms' values.
    fn predicate(
        &self,
        operand: FormulaNodeId,
        atoms: &BTreeMap<usize, Term>,
    ) -> Result<Term, SmtEncodingRefusal> {
        let nodes = self.clause.body().nodes();
        let mut values: Vec<Option<Term>> = vec![None; nodes.len()];
        // The arena puts an operator before its operands, so one pass from
        // the end reaches every operand before its operator.
        for index in (operand.index()..nodes.len()).rev() {
            let value = |at: &FormulaNodeId| values.get(at.index()).cloned().flatten();
            let term = match &nodes[index] {
                FormulaNode::Constant(constant) => Some(Term::literal(*constant)),
                FormulaNode::Holds { .. } => atoms.get(&index).cloned(),
                FormulaNode::Not(only) => value(only).map(Not::not),
                FormulaNode::And(left, right) => value(left)
                    .zip(value(right))
                    .map(|(l, r)| Term::all([l, r])),
                FormulaNode::Or(left, right) => value(left)
                    .zip(value(right))
                    .map(|(l, r)| Term::any([l, r])),
                FormulaNode::Implies(left, right) => {
                    value(left).zip(value(right)).map(|(l, r)| l.implies(r))
                }
                _ => None,
            };
            values[index] = term;
        }
        values
            .get(operand.index())
            .cloned()
            .flatten()
            .ok_or_else(|| fault("a state predicate reads only atoms and connectives"))
    }
}
