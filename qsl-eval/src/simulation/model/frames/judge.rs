// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 "Abstract-operation evaluation" (ADR-020 RS-4): the calls
//! `ModelSystem` makes when it expands a state, applied to one candidate
//! post-state instead of every candidate.
//!
//! A refinement's step check names the abstract state `a` before a concrete
//! step and the abstract state `a'` after it. [`ModelSystem::judge_application`]
//! asks, for each abstract transition identity the step row names, whether the
//! effective precondition holds at `a`, whether the frame decision accepts
//! `(a, a')`, and whether the effective postcondition is true over
//! `(a, a', delta)`. Those are the calls [`super::apply`] and the candidate
//! loop make, so a step that passes here is a successor of `a` in the
//! abstract system's own expansion and the two never disagree.

use qsl_foundation::CatalogCode;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{AnchorKind, SelectedAnchor};
use qsl_semantics::model::state::ModelState;
use quire_canonical::Limits as CanonicalLimits;
use quire_exact::{ObjectReference, Value};
use quire_semantic_value::model_transition::ModelTransition;

use super::{pre_observations, text_limits, text_of, transition_of, FrameRefused, Run};
use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::model::contracts::{conjunction, Conjunction};
use crate::simulation::model::domain::{Domain, Odometer};
use crate::simulation::model::encode::{declaration_text, reference_enc};
use crate::simulation::model::walk::{expand_state, rows_of, stop, walk_of};
use crate::simulation::model::{ClauseOutcome, ModelFinding, ModelSimState, ModelSystem, BROKEN};

/// One abstract operation applied to `receiver` at `pre`, judged against the
/// candidate post-state `post`.
#[derive(Clone, Copy, Debug)]
pub struct AbstractApplication<'q> {
    /// The abstract state before the step.
    pub pre: &'q ModelState,
    /// The receiver, an object of `pre`.
    pub receiver: &'q ObjectReference,
    /// The abstract operation's declaration.
    pub operation: &'q DeclarationKey,
    /// One entry for each parameter, in declaration order; `None` is `_`,
    /// which ranges over the parameter's domain in FR-120's order.
    pub arguments: &'q [Option<Value>],
    /// The abstract state after the step.
    pub post: &'q ModelState,
    /// The abstract result, when the row binds it to the concrete step's;
    /// `None` lets the result range over its domain.
    pub result: Option<&'q Value>,
}

/// The first check that rejected a transition identity (ADR-020 RC-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rejection {
    /// The named effective precondition is false at `pre`.
    Precondition {
        /// The first false clause, by ascending name.
        clause: String,
    },
    /// The frame decision refuses `(pre, post)`.
    Frame {
        /// The refusal's catalog code.
        code: CatalogCode,
    },
    /// No result makes the effective postcondition true.
    Postcondition,
}

/// How one transition identity fared.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Judgement {
    /// The identity is a transition from `pre` to `post`.
    Accepted,
    /// A check rejected it.
    Rejected(Rejection),
    /// No check was false and these conjuncts did not decide, in name order
    /// (ADR-020 RE-4 `UndecidedSuccessor`).
    Undecided(Vec<(String, ClauseOutcome)>),
}

/// One abstract transition identity and its judgement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Judged {
    /// The identity: the operation, the receiver and one argument vector.
    pub transition: ModelTransition,
    /// Its judgement.
    pub judgement: Judgement,
}

/// What a concrete step applied: the receiver, the arguments and the result
/// a transition identity and the post-state it reached stand for.
#[derive(Clone, Debug)]
pub struct ResolvedStep {
    /// The receiver, an object of the pre-state.
    pub receiver: ObjectReference,
    /// The arguments, in declaration order.
    pub arguments: Vec<Value>,
    /// The step's result; `None` for an operation with no result.
    pub result: Option<Value>,
}

/// The identity text of the operation declaration `key` (FR-181): the text
/// a [`ModelTransition::operation`] carries for it.
///
/// # Errors
///
/// An [`ExpansionStop`] when the identity has no canonical text.
pub fn operation_identity_text(key: &DeclarationKey) -> Result<String, ExpansionStop> {
    declaration_text(key, CanonicalLimits::new(u64::MAX)).map_err(|_| stop(BROKEN))
}

/// The anchor a state built outside an expansion is read under. Only an
/// invariant finding reads it, and a judgement evaluates none.
fn mapped_anchor() -> SelectedAnchor {
    SelectedAnchor {
        kind: AnchorKind::Initialization,
        name: "refinement".to_owned(),
    }
}

impl ModelSystem<'_> {
    /// Every successor of `state` and every finding of its expansion, for a
    /// state built outside this system's own exploration (a mapped abstract
    /// state): the successor relation RS-5 enables an `any` row by.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the successor set cannot be known.
    pub fn expand_state(
        &self,
        state: &ModelState,
    ) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
        let state = ModelSimState {
            state: state.clone(),
            anchor: mapped_anchor(),
            reached_by: None,
        };
        expand_state(self, &state, None)
    }

    /// The receiver, arguments and result that `transition` applied at `pre`
    /// to reach `post` (FR-181 writes them only as canonical texts, so they
    /// are found by enumerating the same argument domains the expansion did).
    /// `None` when no argument vector or result of those domains has the
    /// texts the identity and the step's effect carry.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when `pre` is no state of this system, the
    /// identity names no operation of the receiver's type, or a result
    /// domain is too large to enumerate.
    pub fn resolve_step(
        &self,
        pre: &ModelState,
        transition: &ModelTransition,
        post: &ModelSimState,
    ) -> Result<Option<ResolvedStep>, ExpansionStop> {
        let state = ModelSimState {
            state: pre.clone(),
            anchor: mapped_anchor(),
            reached_by: None,
        };
        let walk = walk_of(self, &state, None)?;
        let references = walk.references();
        for row in &walk.rows {
            let receiver = reference_enc(&row.reference)
                .canonical_text(text_limits())
                .map_err(|_| stop(BROKEN))?;
            if transition.receiver() != Some(receiver.as_str()) {
                continue;
            }
            let Some(application) = self.applications.get(&row.type_id()).and_then(|found| {
                found
                    .iter()
                    .find(|application| application.text == transition.operation())
            }) else {
                continue;
            };
            let parameters = application.declaration.parameters();
            let domains = parameters
                .iter()
                .map(|(_, value_type)| Domain::of(value_type, &references, self.types))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| stop(BROKEN))?;
            let mut arguments = Odometer::new(domains.iter().map(Domain::len).collect());
            while let Some(digits) = arguments.next_digits() {
                let args = parameters
                    .iter()
                    .zip(&domains)
                    .zip(&digits)
                    .map(|(((name, _), domain), index)| {
                        domain.nth(*index).map(|value| (name.clone(), value))
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| stop(BROKEN))?;
                if transition_of(row, application, &args)? != *transition {
                    continue;
                }
                let result = self.result_of(application, post)?;
                return Ok(result.map(|result| ResolvedStep {
                    receiver: row.reference.clone(),
                    arguments: args.into_iter().map(|(_, value)| value).collect(),
                    result,
                }));
            }
        }
        Ok(None)
    }

    /// The result of the step that reached `post` through `application`:
    /// `Some(None)` for an operation with no result, `Some(Some(value))` for
    /// the value of its result domain whose text the step's effect carries,
    /// and `None` when no value has that text.
    fn result_of(
        &self,
        application: &super::super::Application,
        post: &ModelSimState,
    ) -> Result<Option<Option<Value>>, ExpansionStop> {
        let Some(result_type) = application.declaration.result() else {
            return Ok(Some(None));
        };
        let recorded = post
            .reached_by
            .as_ref()
            .and_then(|effect| effect.result.as_deref());
        let references: Vec<ObjectReference> = rows_of(self, &post.state)?
            .into_iter()
            .map(|row| row.reference)
            .collect();
        let domain =
            Domain::of(result_type, &references, self.types).ok_or_else(|| stop(BROKEN))?;
        if domain.len() > super::RESULT_ORDER_LIMIT {
            return Err(stop(crate::simulation::model::EXHAUSTED));
        }
        for index in 0..domain.len() {
            let value = domain.nth(index).ok_or_else(|| stop(BROKEN))?;
            if Some(text_of(&value)?.as_str()) == recorded {
                return Ok(Some(Some(value)));
            }
        }
        Ok(None)
    }

    /// Judge `request.operation` at `request.receiver` against the candidate
    /// post-state: one [`Judged`] for each argument vector, in FR-120's
    /// argument order.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when a clause cannot be evaluated: a spent
    /// evaluation meter, or a broken invariant (a receiver that is no object
    /// of `pre`, or an operation the receiver's type does not have).
    pub fn judge_application(
        &self,
        request: &AbstractApplication<'_>,
    ) -> Result<Vec<Judged>, ExpansionStop> {
        let state = ModelSimState {
            state: request.pre.clone(),
            anchor: mapped_anchor(),
            reached_by: None,
        };
        let walk = walk_of(self, &state, None)?;
        let row = walk
            .rows
            .iter()
            .find(|row| row.reference == *request.receiver)
            .ok_or_else(|| stop(BROKEN))?;
        let text = declaration_text(request.operation, CanonicalLimits::new(u64::MAX))
            .map_err(|_| stop(BROKEN))?;
        let application = self
            .applications
            .get(&row.type_id())
            .and_then(|applications| applications.iter().find(|found| found.text == text))
            .ok_or_else(|| stop(BROKEN))?;
        let parameters = application.declaration.parameters();
        if parameters.len() != request.arguments.len() {
            return Err(stop(BROKEN));
        }
        let references = walk.references();
        let domains = parameters
            .iter()
            .zip(request.arguments)
            .map(|((_, value_type), supplied)| match supplied {
                Some(_) => Ok(None),
                None => Domain::of(value_type, &references, self.types)
                    .map(Some)
                    .ok_or_else(|| stop(BROKEN)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut arguments = Odometer::new(
            domains
                .iter()
                .map(|domain| domain.as_ref().map_or(1, Domain::len))
                .collect(),
        );
        let preconditions = walk.applicable(
            &self.clauses.preconditions,
            row,
            application.declaring,
            &application.name,
        );
        let postconditions = walk.applicable(
            &self.clauses.postconditions,
            row,
            application.declaring,
            &application.name,
        );
        let mut judged = Vec::new();
        while let Some(digits) = arguments.next_digits() {
            let args = parameters
                .iter()
                .zip(request.arguments)
                .zip(&domains)
                .zip(&digits)
                .map(|((((name, _), supplied), domain), index)| {
                    match (supplied, domain) {
                        (Some(value), _) => Some(value.clone()),
                        (None, Some(domain)) => domain.nth(*index),
                        (None, None) => None,
                    }
                    .map(|value| (name.clone(), value))
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| stop(BROKEN))?;
            let transition = transition_of(row, application, &args)?;
            let transition_bytes =
                quire_canonical::to_vec(&transition, text_limits()).map_err(|_| stop(BROKEN))?;
            let run = Run {
                walk: &walk,
                row,
                application,
                args: &args,
                transition: &transition,
                transition_bytes: &transition_bytes,
                postconditions: &postconditions,
                footprint: None,
            };
            let judgement = run.judge(&preconditions, request)?;
            judged.push(Judged {
                transition,
                judgement,
            });
        }
        Ok(judged)
    }
}

impl Run<'_, '_, '_> {
    /// This application's three checks over `request.post`, in the order
    /// expansion makes them (FR-120 "Post-states").
    fn judge(
        &self,
        preconditions: &[&super::ClauseInfo],
        request: &AbstractApplication<'_>,
    ) -> Result<Judgement, ExpansionStop> {
        match conjunction(self.walk, preconditions, |info| {
            pre_observations(self.walk, info, self.row, self.args)
        })? {
            Conjunction::True => {}
            Conjunction::False(clause) => {
                return Ok(Judgement::Rejected(Rejection::Precondition { clause }))
            }
            Conjunction::Undecided(undecided) => return Ok(Judgement::Undecided(undecided)),
        }
        let delta = match self.decide(request.post)? {
            Ok(delta) => delta,
            Err(FrameRefused(code)) => return Ok(Judgement::Rejected(Rejection::Frame { code })),
        };
        let effect = self.effect_of(request.post, &delta)?;
        let post_references: Vec<ObjectReference> = rows_of(self.walk.system, request.post)?
            .into_iter()
            .map(|row| row.reference)
            .collect();
        let results: Vec<Option<Value>> = match request.result {
            Some(bound) => vec![Some(bound.clone())],
            None => match self.result_order(&post_references)? {
                None => vec![None],
                Some((domain, order)) => order
                    .into_iter()
                    .map(|index| domain.nth(index).map(Some).ok_or_else(|| stop(BROKEN)))
                    .collect::<Result<_, _>>()?,
            },
        };
        let mut evidence = None;
        let mut undecided = None;
        for result in results {
            match self.post_conjunction(&mut evidence, request.post, &effect, result.as_ref())? {
                Conjunction::True => return Ok(Judgement::Accepted),
                Conjunction::False(_) => {}
                Conjunction::Undecided(found) => {
                    undecided.get_or_insert(found);
                }
            }
        }
        Ok(match undecided {
            Some(undecided) => Judgement::Undecided(undecided),
            None => Judgement::Rejected(Rejection::Postcondition),
        })
    }
}
