// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `Probabilistic` family's S2 productions (FR-311, FR-392): a `random`
//! parameter, a `reward`, a `workload` and a `probabilistic` claim.
//!
//! Each function reads the CST of one declaration and builds its form. None
//! selects a meaning, the same contract `value`'s productions keep
//! (FR-091): the profile, subject, workload, operation, reward, predicate
//! and fairness names stay spelled and spanned, a confidence item is kept as
//! written (no default, no de-duplication) and every threshold, weight and
//! range is left to S3 (FR-185, FR-186).
//!
//! The words that lead these declarations and the words inside them
//! (`weight`, `under`, `alpha`, `accumulate` and the rest) are contextual, so
//! the builders read them as identifier tokens by position.

use qsl_cst::{CstNode, CstToken, LosslessCst, Production, TokenKind};
use qsl_foundation::Spanned;

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::ActivationForm;
use super::syntax::{
    BoundOp, ConfidenceItemForm, ConfidenceKey, DeclarationForm, DeclaredName, EventForm,
    ExactValueForm, ExpectedQuantity, FractionForm, HorizonForm, MeasureForm, MeasureWindowForm,
    ProbabilisticBody, ProbabilisticForm, RandomForm, RewardForm, SchedulerForm, UsingAlias,
    WeightedValueForm, WeightingForm, WorkloadForm, WorkloadWeightForm,
};
use super::temporal_clause::{expression_form, fairness_form, formula_form, parameter_form};
use super::value::{
    exact_value, items, name_form, nodes_of, only, production_node, quantity_form, text, tokens_of,
    unexpected, Item,
};

/// `random Op(p) ~ { value: weight, ... };` (FR-311).
pub(crate) fn random_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `random Op ( p ) ~ ...`: the leading word, then the parameter.
    let [_, parameter] = tokens_of(&declaration_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let support = nodes_of(&declaration_items, Production::WeightedValue)
        .into_iter()
        .map(|pair| weighted_value(cst, pair))
        .collect::<Result<_, _>>()?;
    Ok(DeclarationForm::Random(Box::new(RandomForm {
        operation: name_form(
            cst,
            only(&declaration_items, Production::QualifiedName, node)?,
        )?,
        parameter: name_of(parameter, node)?,
        support,
        span: node.span(),
    })))
}

fn weighted_value(cst: &LosslessCst, pair: &CstNode) -> Result<WeightedValueForm, FormsRefusal> {
    let pair_items = items(cst, pair);
    Ok(WeightedValueForm {
        value: expression_form(cst, only(&pair_items, Production::Expression, pair)?)?,
        weight: exact_value(cst, only(&pair_items, Production::ExactValue, pair)?)?,
        span: pair.span(),
    })
}

/// `reward name on Op = expression;` (FR-311).
pub(crate) fn reward_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    let [_, name] = tokens_of(&declaration_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    Ok(DeclarationForm::Reward(Box::new(RewardForm {
        name: name_of(name, node)?,
        operation: name_form(
            cst,
            only(&declaration_items, Production::QualifiedName, node)?,
        )?,
        expression: expression_form(cst, only(&declaration_items, Production::Expression, node)?)?,
        span: node.span(),
    })))
}

/// `workload Name on Model { weight Op = value; ... }` (FR-311).
pub(crate) fn workload_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    let [_, name, model] = tokens_of(&declaration_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let weights = nodes_of(&declaration_items, Production::WorkloadWeight)
        .into_iter()
        .map(|weight| {
            let weight_items = items(cst, weight);
            Ok(WorkloadWeightForm {
                operation: name_form(cst, only(&weight_items, Production::QualifiedName, weight)?)?,
                weight: exact_value(cst, only(&weight_items, Production::ExactValue, weight)?)?,
                span: weight.span(),
            })
        })
        .collect::<Result<_, FormsRefusal>>()?;
    Ok(DeclarationForm::Workload(Box::new(WorkloadForm {
        name: name_of(name, node)?,
        model: name_of(model, node)?,
        weights,
        span: node.span(),
    })))
}

/// `probabilistic Name using p on Subject [over (x: T)] scheduler activation
/// [with items] { form }` (FR-392).
pub(crate) fn probabilistic_clause(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let clause_items = items(cst, node);
    let [_, name, alias, subject] = tokens_of(&clause_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let over = match nodes_of(&clause_items, Production::Parameter)[..] {
        [] => None,
        [parameter] => Some(parameter_form(cst, parameter)?),
        _ => return Err(unexpected(node)),
    };
    let scheduler = scheduler_form(cst, only(&clause_items, Production::SchedulerClause, node)?)?;
    let activation = activation_form(cst, only(&clause_items, Production::Activation, node)?)?;
    let confidence = match nodes_of(&clause_items, Production::Confidence)[..] {
        [] => Vec::new(),
        [confidence] => confidence_items(cst, confidence)?,
        _ => return Err(unexpected(node)),
    };
    let body = probabilistic_body(
        cst,
        only(&clause_items, Production::ProbabilisticForm, node)?,
    )?;
    Ok(DeclarationForm::Probabilistic(Box::new(
        ProbabilisticForm {
            name: name_of(name, node)?,
            profile: UsingAlias {
                alias: text(alias, node)?,
                span: alias.span(),
            },
            subject: name_of(subject, node)?,
            over,
            scheduler,
            activation,
            confidence,
            body,
            span: node.span(),
        },
    )))
}

fn name_of(token: &CstToken, node: &CstNode) -> Result<DeclaredName, FormsRefusal> {
    Ok(DeclaredName {
        name: text(token, node)?,
        span: token.span(),
    })
}

/// `on origin` or `on each (p: T) [when (e)]`, as a temporal clause reads it.
fn activation_form(
    cst: &LosslessCst,
    activation: &CstNode,
) -> Result<ActivationForm, FormsRefusal> {
    let activation_items = items(cst, activation);
    Ok(
        match nodes_of(&activation_items, Production::Parameter).first() {
            None => ActivationForm::Origin,
            Some(parameter) => ActivationForm::Each {
                parameter: parameter_form(cst, parameter)?,
                when: nodes_of(&activation_items, Production::Expression)
                    .first()
                    .map(|condition| expression_form(cst, condition))
                    .transpose()?,
            },
        },
    )
}

/// `under W` or `under every scheduler fair ...`.
fn scheduler_form(cst: &LosslessCst, clause: &CstNode) -> Result<SchedulerForm, FormsRefusal> {
    let clause_items = items(cst, clause);
    let words = tokens_of(&clause_items, TokenKind::Identifier);
    match words[..] {
        [_, name] => Ok(SchedulerForm::Workload(name_of(name, clause)?)),
        [_, every, scheduler]
            if every.spelling() == b"every" && scheduler.spelling() == b"scheduler" =>
        {
            let fairness = nodes_of(&clause_items, Production::FairnessItem)
                .into_iter()
                .map(|item| fairness_form(cst, item))
                .collect::<Result<_, _>>()?;
            Ok(SchedulerForm::Every { fairness })
        }
        _ => Err(unexpected(clause)),
    }
}

/// `with alpha 0.01, beta 0.01, ...`: each item kept, in order.
fn confidence_items(
    cst: &LosslessCst,
    confidence: &CstNode,
) -> Result<Vec<ConfidenceItemForm>, FormsRefusal> {
    let confidence_items = items(cst, confidence);
    nodes_of(&confidence_items, Production::ConfidenceItem)
        .into_iter()
        .map(|item| {
            let item_items = items(cst, item);
            let [key] = tokens_of(&item_items, TokenKind::Identifier)[..] else {
                return Err(unexpected(item));
            };
            let key_kind = match key.spelling() {
                b"alpha" => ConfidenceKey::Alpha,
                b"beta" => ConfidenceKey::Beta,
                b"indifference" => ConfidenceKey::Indifference,
                _ => return Err(unexpected(item)),
            };
            Ok(ConfidenceItemForm {
                key: key_kind,
                key_span: key.span(),
                value: exact_value(cst, only(&item_items, Production::ExactValue, item)?)?,
            })
        })
        .collect()
}

/// The `<=` or `>=` among a node's own tokens.
fn bound_op(node_items: &[Item<'_>], node: &CstNode) -> Result<BoundOp, FormsRefusal> {
    let mut found = None;
    for item in node_items {
        let Item::Token(token) = item else { continue };
        let op = match token.spelling() {
            b"<=" => BoundOp::AtMost,
            b">=" => BoundOp::AtLeast,
            _ => continue,
        };
        if found.replace(op).is_some() {
            return Err(unexpected(node));
        }
    }
    found.ok_or_else(|| unexpected(node))
}

fn probabilistic_body(
    cst: &LosslessCst,
    form: &CstNode,
) -> Result<ProbabilisticBody, FormsRefusal> {
    let form_items = items(cst, form);
    let words = tokens_of(&form_items, TokenKind::Identifier);
    let Some(head) = words.first() else {
        return Err(unexpected(form));
    };
    let op = bound_op(&form_items, form)?;
    let exact = |index: usize| -> Result<ExactValueForm, FormsRefusal> {
        match nodes_of(&form_items, Production::ExactValue).get(index) {
            Some(value) => exact_value(cst, value),
            None => Err(unexpected(form)),
        }
    };
    let expression = |index: usize| match nodes_of(&form_items, Production::Expression).get(index) {
        Some(value) => expression_form(cst, value),
        None => Err(unexpected(form)),
    };
    match head.spelling() {
        b"probability" => Ok(ProbabilisticBody::Probability {
            op,
            threshold: exact(0)?,
            event: event_form(cst, only(&form_items, Production::Event, form)?)?,
        }),
        b"quantile" => Ok(ProbabilisticBody::Quantile {
            level: exact(0)?,
            measure: measure_form(cst, only(&form_items, Production::Measure, form)?)?,
            op,
            threshold: expression(0)?,
        }),
        b"mean" => Ok(ProbabilisticBody::MeanOfFraction {
            fraction: fraction_form(cst, only(&form_items, Production::Fraction, form)?)?,
            op,
            threshold: exact(0)?,
        }),
        b"long_run_fraction" => Ok(ProbabilisticBody::LongRunFraction {
            predicate: expression(0)?,
            weighting: weighting(cst, &form_items, form)?,
            op,
            threshold: exact(0)?,
        }),
        b"expected" => {
            let quantity = match words[..] {
                [_, accumulate, reward] if accumulate.spelling() == b"accumulate" => {
                    ExpectedQuantity::Accumulate(name_of(reward, form)?)
                }
                [_, elapsed] if elapsed.spelling() == b"elapsed" => ExpectedQuantity::Elapsed,
                _ => return Err(unexpected(form)),
            };
            Ok(ProbabilisticBody::ExpectedUntil {
                quantity,
                target: expression(0)?,
                op,
                threshold: expression(1)?,
            })
        }
        _ => Err(unexpected(form)),
    }
}

/// A temporal formula, or `measure op threshold`.
fn event_form(cst: &LosslessCst, event: &CstNode) -> Result<EventForm, FormsRefusal> {
    let event_items = items(cst, event);
    match nodes_of(&event_items, Production::Measure)[..] {
        [measure] => Ok(EventForm::Compare {
            measure: measure_form(cst, measure)?,
            op: bound_op(&event_items, event)?,
            threshold: expression_form(cst, only(&event_items, Production::Expression, event)?)?,
        }),
        [] => Ok(EventForm::Temporal(formula_form(
            cst,
            only(&event_items, Production::TemporalExpression, event)?,
        )?)),
        _ => Err(unexpected(event)),
    }
}

/// `accumulate R | steps | elapsed from holds(A) until holds(B) within h`, or
/// a fraction.
fn measure_form(cst: &LosslessCst, measure: &CstNode) -> Result<MeasureForm, FormsRefusal> {
    let measure_items = items(cst, measure);
    if let [fraction] = nodes_of(&measure_items, Production::Fraction)[..] {
        return Ok(MeasureForm::Fraction(fraction_form(cst, fraction)?));
    }
    let predicates = nodes_of(&measure_items, Production::Expression);
    let [from, until] = predicates[..] else {
        return Err(unexpected(measure));
    };
    let window = MeasureWindowForm {
        from: expression_form(cst, from)?,
        until: expression_form(cst, until)?,
        within: horizon_form(cst, only(&measure_items, Production::Horizon, measure)?)?,
    };
    let span = measure.span();
    match tokens_of(&measure_items, TokenKind::Identifier)[..] {
        [accumulate, reward] if accumulate.spelling() == b"accumulate" => {
            Ok(MeasureForm::Accumulate {
                reward: name_of(reward, measure)?,
                window,
                span,
            })
        }
        [steps] if steps.spelling() == b"steps" => Ok(MeasureForm::Steps { window, span }),
        [elapsed] if elapsed.spelling() == b"elapsed" => Ok(MeasureForm::Elapsed { window, span }),
        _ => Err(unexpected(measure)),
    }
}

/// `fraction holds(P) over [0, h] [weighted by R]`.
fn fraction_form(cst: &LosslessCst, fraction: &CstNode) -> Result<FractionForm, FormsRefusal> {
    let fraction_items = items(cst, fraction);
    Ok(FractionForm {
        predicate: expression_form(
            cst,
            only(&fraction_items, Production::Expression, fraction)?,
        )?,
        horizon: horizon_form(cst, only(&fraction_items, Production::Horizon, fraction)?)?,
        weighting: weighting(cst, &fraction_items, fraction)?,
        span: fraction.span(),
    })
}

/// The optional `weighted by R` child of `node`.
fn weighting(
    cst: &LosslessCst,
    node_items: &[Item<'_>],
    node: &CstNode,
) -> Result<Option<WeightingForm>, FormsRefusal> {
    match nodes_of(node_items, Production::Weighting)[..] {
        [] => Ok(None),
        [weighting] => {
            let weighting_items = items(cst, weighting);
            let [_, name] = tokens_of(&weighting_items, TokenKind::Identifier)[..] else {
                return Err(unexpected(weighting));
            };
            Ok(Some(WeightingForm {
                name: name_of(name, weighting)?,
                span: weighting.span(),
            }))
        }
        _ => Err(unexpected(node)),
    }
}

/// A step count or a quantity literal.
fn horizon_form(cst: &LosslessCst, horizon: &CstNode) -> Result<HorizonForm, FormsRefusal> {
    match items(cst, horizon)[..] {
        [Item::Node(quantity)] if quantity.production() == Production::QuantityLiteral => {
            Ok(HorizonForm::Quantity(quantity_form(cst, quantity)?))
        }
        [Item::Token(count)] if count.kind() == TokenKind::Integer => {
            // S1 admits only a `u64` here, so a count that does not parse is
            // a broken invariant between the two stages.
            let value = text(count, horizon)?
                .parse::<u64>()
                .map_err(|_| unexpected(horizon))?;
            Ok(HorizonForm::Steps(Spanned {
                value,
                span: count.span(),
            }))
        }
        _ => Err(unexpected(horizon)),
    }
}
