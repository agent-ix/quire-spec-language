// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Refinement queries" (QSpec FR-468, FR-146): the per-step
//! simulation queries of a refinement, whose refutations FR-314's
//! `check_refinement_smt_proof` checks.
//!
//! [`encode_refinement_query`] encodes the `Initial` query or the `Step`
//! query of one row of the refinement node's `steps` array over the concrete
//! subject's state copies `s0` (and `s1`). `map(s)` is read as terms: each
//! abstract object is the key-preserving image of a concrete object, and
//! each of its fields is its mapping row read over the concrete copy with
//! `self` bound to that object, together with the row's definedness.
//!
//! The script is, in order: `(set-logic QF_LIA)`; the `declare-const`
//! commands of `s0`, then of `s1`; the range assertions of `s0`, then of
//! `s1`; for `Initial` the concrete initial-state predicate on `s0`; one
//! assertion per hypothesis instance on `s0` (the hypotheses in ascending
//! node-key order, each over the objects of its context type in layout
//! order); then one assertion:
//!
//! - `Initial`: `(or (not D) (not (or E1 ... Em)))`, `D` the definedness of
//!   every mapping row read and `Ej` the mapped state agreeing, field by
//!   field, with the abstract initial state `j` in list order;
//! - `Step { row }`: the disjunction, over the row's concrete operation's
//!   applications in step-disjunct order, of the application's step
//!   disjunct conjoined with the negation of the row's rule for it;
//!
//! and `(check-sat)`. FR-315 states no assertion order for these queries;
//! this one is the encoding's own (spec gap STD-556).
//!
//! The encoding reads no encoding limit (FR-314 "Encoding limits"). A
//! refinement with a hidden field or a history field, a row of a protocol
//! subject and a row that binds the abstract result to the concrete one have
//! no form here and refuse `out-of-fragment`, naming the refinement or the
//! row; FR-146 writes no per-step record for the first two.

use std::ops::Not;
use std::rc::Rc;

use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CheckedRefinement, CheckedRow, CheckedStateClause, StepClass, StepTarget,
};
use qsl_semantics::model::clause_observation::Observation;
use quire_exact::NodeKey;

use crate::model_subject::ModelSubject;
use crate::simulation::model::{ModelSystem, SymbolicSubject};
use crate::smt::expression::{Encoded, Reader, Sym, Views};
use crate::smt::layout::{value_term, FieldTerms, StateCopy, StateLayout, StateRef};
use crate::smt::refusal::ExpressionLocus;
use crate::smt::relation::{
    applications, check_domains, read_bound, result_values, symbol_of, writes, Application,
};
use crate::smt::script::{Script, Term};
use crate::smt::{Measure, Refusal, SmtEncodingRefusal};

/// FR-314's request of a refinement item: the recompiled package, the
/// refinement node, and the re-admitted concrete and abstract subjects. It
/// is not FR-338's `CertificateRequest`, which holds one model subject and a
/// temporal item.
pub struct RefinementCertificateRequest<'a> {
    /// The recompiled package that declares the refinement.
    pub package: &'a CheckedPackage,
    /// The refinement's node key (QSpec FR-375).
    pub refinement: NodeKey,
    /// The concrete subject.
    pub concrete: &'a ModelSubject<'a>,
    /// The abstract subject.
    pub abstract_: &'a ModelSubject<'a>,
}

/// Which refinement query to encode.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RefinementQueryKind {
    /// The initial query: every concrete initial state maps to an abstract
    /// initial state.
    Initial,
    /// The step query of the row at this zero-based position of the
    /// refinement node's `steps` array.
    Step {
        /// The row's position.
        row: u64,
    },
}

fn fault(invariant: &'static str) -> SmtEncodingRefusal {
    SmtEncodingRefusal::fault(invariant)
}

/// `out-of-fragment` at the construct `form` of `clause`.
fn out_of_fragment(clause: &str, form: &'static str) -> SmtEncodingRefusal {
    Refusal::OutOfFragment {
        form,
        locus: ExpressionLocus {
            clause: clause.to_owned(),
            node: 0,
            component: None,
        },
    }
    .into()
}

/// The refinement `request` names, and the source row of each entry of its
/// `steps` array.
fn refinement_of<'p>(
    request: &RefinementCertificateRequest<'p>,
) -> Result<(&'p CheckedRefinement, Vec<usize>), SmtEncodingRefusal> {
    request
        .package
        .graph()
        .refinements()
        .iter()
        .find_map(|refinement| {
            let emission = refinement.v2_emission().ok()?;
            (emission.identity == request.refinement).then_some((refinement, emission.step_sources))
        })
        .ok_or_else(|| fault("the request names a refinement of its package"))
}

/// Encode the refinement query `kind` of `request` as an SMT-LIB 2 script
/// in the canonical printing (FR-315 "Refinement queries").
///
/// # Errors
///
/// [`SmtEncodingRefusal`]: `refinement-any-row` for an `any` row,
/// `refinement-open-argument` for a row with a `_` argument,
/// `out-of-fragment` for a mapping row, receiver or argument outside
/// `QF_LIA`, the subject refusals of FR-315's table, or a fault when the
/// request names no refinement of its package; no script is written.
pub fn encode_refinement_query(
    request: &RefinementCertificateRequest<'_>,
    kind: RefinementQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    let (refinement, step_sources) = refinement_of(request)?;
    let row = match kind {
        RefinementQueryKind::Initial => None,
        RefinementQueryKind::Step { row } => {
            let source = usize::try_from(row)
                .ok()
                .and_then(|row| step_sources.get(row))
                .and_then(|source| refinement.steps().get(*source))
                .ok_or_else(|| fault("the step row is a row of the refinement"))?;
            Some((row, source))
        }
    };
    if let Some((position, row)) = row {
        match &row.target {
            StepTarget::Any => {
                return Err(Refusal::RefinementAnyRow { row: position }.into());
            }
            StepTarget::Abstract { arguments, .. } => {
                if let Some(open) = arguments.iter().position(Option::is_none) {
                    return Err(Refusal::RefinementOpenArgument {
                        row: position,
                        argument: u64::try_from(open).unwrap_or(u64::MAX),
                    }
                    .into());
                }
            }
            _ => {}
        }
    }
    let name = refinement.name();
    if refinement
        .objects()
        .iter()
        .any(|object| !object.hidden.is_empty())
    {
        return Err(out_of_fragment(name, "a refinement with a hidden field"));
    }
    if !refinement.history().is_empty() {
        return Err(out_of_fragment(name, "a refinement with a history field"));
    }

    let concrete_system =
        ModelSystem::new(request.concrete.simulation.clone()).map_err(Refusal::SubjectAdmission)?;
    let abstract_system = ModelSystem::new(request.abstract_.simulation.clone())
        .map_err(Refusal::SubjectAdmission)?;
    let concrete = concrete_system.symbolic()?;
    let abstract_ = abstract_system.symbolic()?;
    let concrete_layout = StateLayout::of(&concrete)?;
    let abstract_layout = StateLayout::of(&abstract_)?;
    let sides = Sides {
        refinement,
        concrete: (&concrete, &concrete_layout),
        abstract_: (&abstract_, &abstract_layout),
        preimages: preimages(refinement, &concrete, &abstract_)?,
    };

    let first = StateCopy::at(0);
    let second = StateCopy::at(1);
    let copies: Vec<&StateCopy> = match row {
        None => vec![&first],
        Some(_) => vec![&first, &second],
    };
    let mut script = Script::new();
    for copy in &copies {
        concrete_layout.declare(copy, &mut script);
    }
    for copy in &copies {
        for range in concrete_layout.ranges(copy) {
            script.assert(&range);
        }
    }
    if row.is_none() {
        script.assert(&concrete_layout.initial(&first, &concrete)?);
    }
    for hypothesis in sides.hypotheses(request.package, &first)? {
        script.assert(&hypothesis);
    }
    let negated = match row {
        None => sides.initial_negation(&first)?,
        Some((position, row)) => {
            let StepClass::Operation(operation) = &row.class else {
                return Err(out_of_fragment(
                    row.spelling(),
                    "a step row of a protocol subject",
                ));
            };
            check_domains(&concrete)?;
            let built = applications(&concrete, u64::MAX)?;
            let mut disjuncts = Vec::new();
            for application in built.iter().filter(|found| found.operation().is(operation)) {
                let step = application.disjunct(
                    &concrete,
                    &concrete_layout,
                    (&first, &second),
                    (
                        &Views::of(StateRef::Copy(first.clone())),
                        &Views {
                            pre: StateRef::Copy(first.clone()),
                            post: StateRef::Copy(second.clone()),
                            current: StateRef::Copy(second.clone()),
                        },
                    ),
                )?;
                let rule = sides.rule(position, row, application, (&first, &second))?;
                disjuncts.push(Term::all([step, rule.not()]));
            }
            Term::any(disjuncts)
        }
    };
    script.assert(&negated);
    if script.closing_command_exceeded() {
        return Err(Refusal::TooLarge {
            measure: Measure::ScriptBytes,
            value: u64::MAX,
            limit: crate::smt::script::MAX_SCRIPT_BYTES as u64,
            setting: Measure::ScriptBytes.setting(),
        }
        .into());
    }
    Ok(script.render())
}

/// The two subjects of a refinement query and how their objects pair.
struct Sides<'q, 'a> {
    refinement: &'q CheckedRefinement,
    concrete: (&'q SymbolicSubject<'a>, &'q StateLayout),
    abstract_: (&'q SymbolicSubject<'a>, &'q StateLayout),
    /// For each abstract object, the index of its concrete preimage.
    preimages: Vec<usize>,
}

/// The concrete preimage of each abstract object: the object of the
/// population the abstract one's population is the image of, under the
/// same key (FR-140).
fn preimages(
    refinement: &CheckedRefinement,
    concrete: &SymbolicSubject<'_>,
    abstract_: &SymbolicSubject<'_>,
) -> Result<Vec<usize>, SmtEncodingRefusal> {
    abstract_
        .objects
        .iter()
        .map(|object| {
            let population = refinement
                .populations()
                .iter()
                .find(|row| row.abstract_population == object.population)
                .map(|row| &row.concrete_population)
                .ok_or_else(|| fault("every abstract population has a population row"))?;
            concrete
                .objects
                .iter()
                .position(|candidate| {
                    candidate.population == *population && candidate.key == object.key
                })
                .ok_or_else(|| fault("every abstract object is the image of a concrete object"))
        })
        .collect()
}

/// `map(s)` at one concrete copy: each abstract field's terms, and the
/// definedness of every row read.
struct Mapped {
    fields: Vec<((usize, usize), FieldTerms)>,
    defined: Term,
}

impl Mapped {
    fn terms(&self, at: (usize, usize)) -> Option<&FieldTerms> {
        self.fields
            .iter()
            .find(|(field, _)| *field == at)
            .map(|(_, terms)| terms)
    }

    /// The mapped state as a state the abstract clauses read.
    fn state(&self) -> StateRef {
        StateRef::Candidate {
            // Every field is written, so the base copy is never read.
            base: StateCopy::at(0),
            written: Rc::new(self.fields.clone()),
        }
    }
}

/// Field equality of two mapped fields (FR-315 "Field equality").
fn agree(later: &FieldTerms, earlier: &FieldTerms) -> Term {
    match (&later.present, &earlier.present) {
        (Some(is_present), Some(was_present)) => Term::all([
            is_present.clone().equal(was_present.clone()),
            Term::any([
                is_present.clone().not(),
                later.value.clone().equal(earlier.value.clone()),
            ]),
        ]),
        _ => later.value.clone().equal(earlier.value.clone()),
    }
}

/// The terms a mapping row's value gives a field of `optional` presence.
fn field_terms(sym: Sym, optional: bool) -> Option<FieldTerms> {
    match sym {
        Sym::Option { present, value } => {
            let (Sym::Bool(value) | Sym::Int(value)) = *value else {
                return None;
            };
            optional.then_some(FieldTerms {
                value,
                present: Some(present),
            })
        }
        Sym::Bool(value) | Sym::Int(value) => Some(FieldTerms {
            value,
            present: optional.then(|| Term::literal(true)),
        }),
        Sym::Object(_) => None,
    }
}

impl Sides<'_, '_> {
    /// Read the mapping or step row `row` named `site` over `copy`, with
    /// `slots` bound in slot order.
    fn read_row(
        &self,
        row: &CheckedRow,
        site: &str,
        copy: &StateCopy,
        slots: Vec<Option<Sym>>,
    ) -> Result<Encoded, SmtEncodingRefusal> {
        let (subject, layout) = self.concrete;
        let views = Views::of(StateRef::Copy(copy.clone()));
        Reader::new(
            layout,
            subject,
            site,
            Observation::Current,
            row.reads().collect(),
            &views,
            slots,
        )
        .value(row.body())
    }

    /// `map(copy)`.
    fn map(&self, copy: &StateCopy) -> Result<Mapped, SmtEncodingRefusal> {
        let (concrete, _) = self.concrete;
        let (abstract_, abstract_layout) = self.abstract_;
        let mut fields = Vec::new();
        let mut defined = Vec::new();
        for (index, object) in abstract_.objects.iter().enumerate() {
            let preimage = self.preimages[index];
            let source = &concrete.objects[preimage];
            let row = self
                .refinement
                .objects()
                .iter()
                .find(|row| row.concrete_type == source.type_key)
                .ok_or_else(|| {
                    out_of_fragment(
                        &source.key,
                        "a concrete object whose type no object row maps",
                    )
                })?;
            if row.abstract_type != object.type_key {
                return Err(fault("an object maps to the abstract object of its key"));
            }
            let layout = &abstract_layout.objects()[index];
            for (field, declared) in object.fields.iter().enumerate() {
                let mapping = row
                    .fields
                    .iter()
                    .find(|mapping| mapping.field == declared.key)
                    .ok_or_else(|| fault("a refinement with no hidden field maps every field"))?;
                let site = format!("{}.{}", object.key, declared.name);
                let encoded = self.read_row(
                    &mapping.expression,
                    &site,
                    copy,
                    vec![Some(Sym::Object(preimage))],
                )?;
                let optional = layout.fields()[field].is_optional();
                let terms = field_terms(encoded.sym, optional)
                    .ok_or_else(|| out_of_fragment(&site, "a mapping row of another sort"))?;
                defined.push(encoded.defined);
                fields.push(((index, field), terms));
            }
        }
        Ok(Mapped {
            fields,
            defined: Term::all(defined),
        })
    }

    /// One assertion per hypothesis instance on `copy`: the invariant
    /// clauses whose context is an object type of the concrete model, in
    /// ascending node-key order, each over the concrete objects of its
    /// context type in layout order.
    fn hypotheses(
        &self,
        package: &CheckedPackage,
        copy: &StateCopy,
    ) -> Result<Vec<Term>, SmtEncodingRefusal> {
        let (subject, layout) = self.concrete;
        let mut clauses: Vec<&CheckedStateClause> = package
            .graph()
            .state_clauses()
            .iter()
            .filter(|clause| {
                clause.is_invariant()
                    && self
                        .refinement
                        .concrete_object_types()
                        .contains(&clause.context())
            })
            .collect();
        clauses.sort_by(|left, right| left.identity().as_bytes().cmp(right.identity().as_bytes()));
        clauses.dedup_by(|later, earlier| later.identity() == earlier.identity());
        let views = Views::of(StateRef::Copy(copy.clone()));
        let mut hypotheses = Vec::new();
        for clause in clauses {
            for (index, object) in subject.objects.iter().enumerate() {
                if !subject.types.conforms(object.type_id, clause.context()) {
                    continue;
                }
                let (value, defined) = Reader::new(
                    layout,
                    subject,
                    clause.name(),
                    clause.observation(),
                    clause.reads().collect(),
                    &views,
                    vec![Some(Sym::Object(index))],
                )
                .boolean(clause.body())?;
                hypotheses.push(Term::all([defined, value]));
            }
        }
        Ok(hypotheses)
    }

    /// The negation of "`map(copy)` is defined and equals an abstract
    /// initial state".
    fn initial_negation(&self, copy: &StateCopy) -> Result<Term, SmtEncodingRefusal> {
        let (abstract_, _) = self.abstract_;
        let mapped = self.map(copy)?;
        let mut states = Vec::new();
        for state in &abstract_.initial {
            let mut conjuncts = Vec::new();
            for (index, values) in state.iter().enumerate() {
                for (field, value) in values.iter().enumerate() {
                    let terms = mapped
                        .terms((index, field))
                        .ok_or_else(|| fault("the mapped state holds every abstract field"))?;
                    match (value, &terms.present) {
                        (None, Some(present)) => conjuncts.push(present.clone().not()),
                        (None, None) => conjuncts.push(Term::literal(false)),
                        (Some(value), present) => {
                            let term = value_term(value)
                                .ok_or_else(|| fault("an initial value of no SMT sort"))?;
                            conjuncts.extend(present.clone());
                            conjuncts.push(terms.value.clone().equal(term));
                        }
                    }
                }
            }
            states.push(Term::all(conjuncts));
        }
        Ok(Term::any([mapped.defined.not(), Term::any(states).not()]))
    }

    /// The row's rule for `application` over (`map(from)`, `map(to)`):
    /// `stutter` leaves the mapped state unchanged; an abstract operation's
    /// transition holds with its receiver and arguments read at `from`.
    fn rule(
        &self,
        position: u64,
        row: &qsl_semantics::check::StepRow,
        application: &Application<'_, '_>,
        (from, to): (&StateCopy, &StateCopy),
    ) -> Result<Term, SmtEncodingRefusal> {
        let before = self.map(from)?;
        let after = self.map(to)?;
        let defined = Term::all([before.defined.clone(), after.defined.clone()]);
        match &row.target {
            StepTarget::Stutter => {
                let mut conjuncts = vec![defined];
                for (at, later) in &after.fields {
                    let earlier = before
                        .terms(*at)
                        .ok_or_else(|| fault("both mapped states hold every abstract field"))?;
                    conjuncts.push(agree(later, earlier));
                }
                Ok(Term::all(conjuncts))
            }
            StepTarget::Abstract {
                operation,
                receiver,
                arguments,
                binds_result,
            } => {
                if *binds_result {
                    return Err(out_of_fragment(
                        row.spelling(),
                        "a step row that binds the abstract result",
                    ));
                }
                let (concrete, _) = self.concrete;
                let mut bindings = vec![Some(Sym::Object(application.receiver()))];
                bindings.extend(
                    application
                        .arguments()
                        .iter()
                        .map(|value| symbol_of(concrete, value)),
                );
                let site = format!("step {}", row.spelling());
                let target = self.read_row(receiver, &site, from, bindings.clone())?;
                let Sym::Object(receiver) = target.sym else {
                    return Err(out_of_fragment(&site, "a receiver that is no object"));
                };
                let receiver = self
                    .preimages
                    .iter()
                    .position(|preimage| *preimage == receiver)
                    .ok_or_else(|| out_of_fragment(&site, "a receiver with no abstract image"))?;
                let mut conjuncts = vec![defined, target.defined];
                let mut supplied = Vec::new();
                for argument in arguments {
                    let Some(argument) = argument else {
                        return Err(Refusal::RefinementOpenArgument {
                            row: position,
                            argument: u64::try_from(supplied.len()).unwrap_or(u64::MAX),
                        }
                        .into());
                    };
                    let encoded = self.read_row(argument, &site, from, bindings.clone())?;
                    conjuncts.push(encoded.defined);
                    supplied.push(Some(encoded.sym));
                }
                conjuncts.push(self.transition(
                    operation,
                    receiver,
                    &supplied,
                    (&before, &after),
                )?);
                Ok(Term::all(conjuncts))
            }
            _ => Err(out_of_fragment(
                row.spelling(),
                "a step row of a protocol subject",
            )),
        }
    }

    /// The abstract operation `operation` applied to the abstract object
    /// `receiver` with `arguments`, from `before` to `after`: its
    /// precondition, its postcondition (some result value, for an operation
    /// that declares one) and its frame. `false` when the receiver has no
    /// such operation.
    fn transition(
        &self,
        operation: &quire_checked::key::DeclarationKey,
        receiver: usize,
        arguments: &[Option<Sym>],
        (before, after): (&Mapped, &Mapped),
    ) -> Result<Term, SmtEncodingRefusal> {
        let (subject, layout) = self.abstract_;
        let Some(declared) = subject.objects[receiver]
            .operations
            .iter()
            .find(|found| found.is(operation))
        else {
            return Ok(Term::literal(false));
        };
        let pre = Views::of(before.state());
        let across = Views {
            pre: before.state(),
            post: after.state(),
            current: after.state(),
        };
        let mut conjuncts = Vec::new();
        for clause in &declared.preconditions {
            let (value, defined) = read_bound(
                (subject, layout),
                clause,
                &pre,
                (receiver, declared, arguments, None),
            )?;
            conjuncts.push(Term::all([defined, value]));
        }
        let mut alternatives = Vec::new();
        for result in result_values(declared) {
            let result =
                match result {
                    Some(value) => Some(symbol_of(subject, &value).ok_or_else(|| {
                        Refusal::ParameterDomain {
                            operation: declared.name.clone(),
                            parameter: "result".to_owned(),
                        }
                    })?),
                    None => None,
                };
            let mut clauses = Vec::new();
            for clause in &declared.postconditions {
                let (value, defined) = read_bound(
                    (subject, layout),
                    clause,
                    &across,
                    (receiver, declared, arguments, result.clone()),
                )?;
                clauses.push(Term::all([defined, value]));
            }
            alternatives.push(Term::all(clauses));
        }
        conjuncts.push(Term::any(alternatives));
        for (at, later) in &after.fields {
            if !writes(subject, declared, receiver, *at) {
                let earlier = before
                    .terms(*at)
                    .ok_or_else(|| fault("both mapped states hold every abstract field"))?;
                conjuncts.push(agree(later, earlier));
            }
        }
        Ok(Term::all(conjuncts))
    }
}
