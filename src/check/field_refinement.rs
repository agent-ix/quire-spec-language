// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-151 field-refinement obligation
//! (`quire.model.conformance.refinement/v1`): moved here from
//! `crate::model::conformance` by ADR-011 §7.3 M-2 (QSL-7), the other of
//! M-2's two moves (alongside [`crate::check::checked_dispatch`]) that make
//! `model` sit below `check` (ADR-011 §6.1). It was the only `conformance`
//! code that read `value::expression`/`check` facts, through an interim
//! `model` -> `check` edge M-5 (QSL-139/FR-068) declared and bounded to
//! exactly two files; M-2 closes that edge (see `crate::check`'s own module
//! doc).
//!
//! [`check_field_refinement_obligation`] needs the writing operation's
//! *established* postcondition facts. `crate::model` has no FR-146
//! expression parser, so a postcondition clause is not parsed from source:
//! the caller states one accepted single-relation guard form directly, as
//! [`PostconditionClause`]. What a clause actually establishes is not
//! caller-trusted, though — this rebuilds the small typed guard tree each
//! clause describes and runs it through this module's own FR-146
//! fact-derivation primitive ([`established_field_fact`]), the identical
//! guard-fact propagation a real checked postcondition's `Definedness::walk`
//! already uses, then decides discharge from what that derivation actually
//! proves.
//!
//! This reaches back into `crate::model::conformance`'s own
//! `ConformanceIndex`, [`AxisFailure`], [`ConformanceOutcome`] and
//! `missing_member`, each widened to `pub(crate)` for exactly this call —
//! `check` sits above `model` in ADR-011 §6.1's layer-3 order, so `check`
//! depending back on `model` is legal; the reverse (what M-5 left as the
//! interim edge) was not.
#![allow(
    clippy::large_enum_variant,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline"
)]
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use super::facts::{established_field_fact, Established};
use super::ir::{Connective, Node, NodeKind, OrderedKind};
use super::refusal::{Location, Origin, ProvedInterval};
use crate::diagnostic::Code;
use crate::model::conformance::{
    missing_member, AxisFailure, ConformanceIndex, ConformanceOutcome,
};
use crate::model::domain_package::{DomainPackage, PostconditionClause};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::value::composite::{Value, ValueType};
use crate::value::numeric::OrderingOperator;
use quire_exact::{Integer, IntegerInterval};

/// `self`, bound as local slot 0, for every synthetic guard tree
/// [`self_field_node`] builds — the one variable a [`PostconditionClause`]
/// ever names.
const CLAUSE_SELF_SLOT: usize = 0;

fn clause_location() -> Location {
    Location {
        origin: Origin::Expression,
        path: Vec::new(),
    }
}

/// The synthetic `self.<field>` projection node every [`PostconditionClause`]
/// guard is built over: `self` at [`CLAUSE_SELF_SLOT`], projected at stable
/// path step zero (the one field a clause ever names, so the step index
/// itself carries no further meaning), declared at `value_type`.
fn self_field_node(value_type: ValueType) -> Node {
    let self_node = Node {
        kind: NodeKind::Local(CLAUSE_SELF_SLOT),
        value_type: ValueType::Integer,
        location: clause_location(),
    };
    Node {
        kind: NodeKind::Field {
            operand: Box::new(self_node),
            index: 0,
            optional: false,
        },
        value_type,
        location: clause_location(),
    }
}

/// `domain`'s value type: `None` (a non-scalar, or unknown, domain) is the
/// unbounded `ValueType::Integer` fallback, per FR-146's rule that a
/// projection onto the field a narrowing redefinition redefines carries the
/// declared facts of the *redefined* parent member, never the narrowing
/// type — so callers seed this from `redefined`'s own declared scalar
/// bounds, not `redefining`'s. `Some((lower, upper))` is the closed interval
/// type, or a [`ModelRefusal`] when a domain package's `ScalarTypeRecord` is
/// malformed (its own lower greater than its upper) — a real defect in
/// caller-supplied domain package data, refused rather than panicked on.
fn field_domain_type(domain: Option<(i64, i64)>) -> Result<ValueType, ModelRefusal> {
    match domain {
        Some((lower, upper)) => {
            let interval =
                IntegerInterval::new(Integer::from(lower), Integer::from(upper)).map_err(|_| {
                    ModelRefusal {
                        code: Code::InvalidModelBinding,
                        // FR-272's `invalid_model_binding` cause list is
                        // closed; there is no dedicated scalar-domain
                        // variant, so this is the catalogued
                        // `malformed-declaration` (its own payload: "the IR
                        // node identity and invalid member path" — here the
                        // scalar type's own declaration).
                        cause: ModelRefusalCause::MalformedDeclaration,
                        detail: format!(
                            "a scalar type's declared domain has lower {lower} greater than its upper {upper}"
                        ),
                    }
                })?;
            Ok(ValueType::Int(interval))
        }
        None => Ok(ValueType::Integer),
    }
}

/// The synthetic `present(self.<field>)` guard a [`PostconditionClause::Presence`]
/// clause describes. Always unbounded (`ValueType::Integer`): presence
/// never depends on a scalar domain, so this can never fail.
fn presence_condition() -> Node {
    Node {
        kind: NodeKind::Present(Box::new(self_field_node(ValueType::Integer))),
        value_type: ValueType::Boolean,
        location: clause_location(),
    }
}

/// The synthetic `self.<field> <operator> <literal>` guard a
/// [`PostconditionClause::Comparison`] clause describes.
fn comparison_condition(
    domain: Option<(i64, i64)>,
    operator: OrderingOperator,
    literal: i64,
) -> Result<Node, ModelRefusal> {
    let field = self_field_node(field_domain_type(domain)?);
    let literal_node = Node {
        kind: NodeKind::Literal(Value::Integer(Integer::from(literal))),
        value_type: ValueType::Integer,
        location: clause_location(),
    };
    Ok(Node {
        kind: NodeKind::Order(
            operator,
            OrderedKind::Integers,
            Box::new(field),
            Box::new(literal_node),
        ),
        value_type: ValueType::Boolean,
        location: clause_location(),
    })
}

/// Derives what `clauses` together actually establish about the field they
/// all name, seeding each synthetic guard's declared domain from `domain`
/// (the redefined parent member's own scalar bounds, when it has one). The
/// effective postcondition is a conjunction (see the module docs), so every
/// clause's guard is folded into one `Connective::And` tree and
/// [`established_field_fact`] runs once over that tree — not once per
/// clause with only the first surviving result kept — so a two-clause bound
/// such as `cs >= 0` and `cs <= 5` is proved together instead of only
/// whichever clause happened to be checked first. `Err` when `domain`
/// itself is malformed (see [`field_domain_type`]).
fn established_facts(
    clauses: &[&PostconditionClause],
    domain: Option<(i64, i64)>,
) -> Result<Established, ModelRefusal> {
    let condition = |clause: &&PostconditionClause| -> Result<Node, ModelRefusal> {
        match clause {
            PostconditionClause::Presence { .. } => Ok(presence_condition()),
            PostconditionClause::Comparison {
                operator, literal, ..
            } => comparison_condition(domain, *operator, *literal),
        }
    };
    let mut conditions = clauses.iter().map(condition);
    let Some(first) = conditions.next() else {
        return Ok(Established::default());
    };
    let mut folded = first?;
    for next in conditions {
        folded = Node {
            kind: NodeKind::Connective(Connective::And, Box::new(folded), Box::new(next?)),
            value_type: ValueType::Boolean,
            location: clause_location(),
        };
    }
    Ok(established_field_fact(&folded, CLAUSE_SELF_SLOT))
}

/// A human-readable `[lower, upper]` rendering of a [`ProvedInterval`], with
/// an unbounded end spelled out rather than omitted.
fn format_interval(interval: &ProvedInterval) -> String {
    let lower = interval
        .lower
        .as_ref()
        .map_or_else(|| "unbounded".to_owned(), Integer::to_string);
    let upper = interval
        .upper
        .as_ref()
        .map_or_else(|| "unbounded".to_owned(), Integer::to_string);
    format!("[{lower}, {upper}]")
}

/// The FR-151 refinement obligation: a narrowing field redefinition must be
/// established by a fact in the effective postcondition of the exposed
/// operation (own or inherited) that writes the redefined field. See the
/// module docs for the [`PostconditionClause`] scope decision this rests on.
pub fn check_field_refinement_obligation(
    domain_package: &DomainPackage,
    redefining_key: &DeclarationKey,
    redefined_key: &DeclarationKey,
) -> Result<ConformanceOutcome, ModelRefusal> {
    let index = ConformanceIndex::build(domain_package);
    let Some(redefining) = index.fields.get(redefining_key) else {
        return Err(missing_member(
            ModelRefusalCause::UnknownRedefining {
                member: redefining_key.clone(),
            },
            &redefining_key.node,
            "redefining field",
        ));
    };
    let Some(redefined) = index.fields.get(redefined_key) else {
        return Err(missing_member(
            ModelRefusalCause::UnknownRedefined {
                member: redefined_key.clone(),
            },
            &redefined_key.node,
            "redefined field",
        ));
    };

    let same_type = redefining.value_type == redefined.value_type;
    let raises_lower = redefining.multiplicity.lower > redefined.multiplicity.lower;
    let single_valued = redefining.multiplicity.upper.is_some_and(|u| u <= 1);

    if same_type && !raises_lower {
        // No narrowing at all (or a narrower upper bound only, which this
        // rung treats under `no-proof-form` below, matching FR-151's
        // "an upper bound on a collection" example).
        if redefining
            .multiplicity
            .upper
            .is_none_or(|redefining_upper| {
                redefined
                    .multiplicity
                    .upper
                    .is_none_or(|redefined_upper| redefining_upper <= redefined_upper)
            })
        {
            return Ok(ConformanceOutcome::Compatible);
        }
    }

    let writer = index.operations.values().find(|operation| {
        operation
            .effect
            .modifies
            .iter()
            .any(|field| field == redefined_key || field == redefining_key)
    });
    let Some(writer) = writer else {
        // No exposed operation writes this field: nothing to discharge.
        return Ok(ConformanceOutcome::Compatible);
    };

    let mut clauses: Vec<&PostconditionClause> = writer.own_postcondition_clauses.iter().collect();
    for operation in index.operations.values() {
        let Some(target) = &operation.redefines else {
            continue;
        };
        if target == &writer.key && operation.owner == redefining.owner {
            clauses.extend(operation.own_postcondition_clauses.iter());
        }
    }
    let names_field =
        |key: &DeclarationKey| -> bool { key == redefined_key || key == redefining_key };

    // FR-146's own rule: a projection onto the field a narrowing redefinition
    // redefines carries the declared facts of the redefined PARENT member,
    // never the narrowing type — so every synthetic guard below is seeded
    // from `redefined`'s own declared scalar bounds, not `redefining`'s.
    let domain = redefined
        .value_type
        .as_package()
        .and_then(|key| index.scalars.get(key))
        .copied();
    let field_clauses: Vec<&PostconditionClause> = clauses
        .iter()
        .filter(|clause| names_field(clause.field()))
        .copied()
        .collect();
    let established = established_facts(&field_clauses, domain)?;

    if raises_lower && single_valued {
        return if established.presence {
            Ok(ConformanceOutcome::Compatible)
        } else {
            Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                axis: "refinement",
                code: Code::UndefinedExpression,
                cause: ModelRefusalCause::UnprovedRefinement,
                detail: format!(
                    "{} narrows the multiplicity of {} with no establishing presence fact (obligation field-presence)",
                    redefining_key.node, redefined_key.node
                ),
            }]))
        };
    }

    if raises_lower && !single_valued {
        return Ok(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "refinement",
            code: Code::UndefinedExpression,
            cause: ModelRefusalCause::UnprovedRefinement,
            detail: format!(
                "{} narrows a collection upper bound, which no FR-146 fact form expresses (obligation no-proof-form)",
                redefining_key.node
            ),
        }]));
    }

    // Value type changed: field-domain via scalar interval containment, or
    // no-proof-form when either type is not a known scalar (an object type
    // narrowing, per FR-151's own example).
    match (
        redefining
            .value_type
            .as_package()
            .and_then(|key| index.scalars.get(key)),
        redefined
            .value_type
            .as_package()
            .and_then(|key| index.scalars.get(key)),
    ) {
        (Some(&(narrow_lower, narrow_upper)), Some(_)) => {
            let interval = established.interval.clone();
            let lower_bound = Integer::from(narrow_lower);
            let upper_bound = Integer::from(narrow_upper);
            match interval {
                Some(proved) => {
                    let contained = match (&proved.lower, &proved.upper) {
                        (Some(lower), Some(upper)) => *lower >= lower_bound && *upper <= upper_bound,
                        _ => false,
                    };
                    if contained {
                        Ok(ConformanceOutcome::Compatible)
                    } else {
                        Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                            axis: "refinement",
                            code: Code::UndefinedExpression,
                            cause: ModelRefusalCause::UnprovedRefinement,
                            detail: format!(
                                "established interval {} is not contained in [{narrow_lower}, {narrow_upper}] (obligation field-domain)",
                                format_interval(&proved)
                            ),
                        }]))
                    }
                }
                None => Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                    axis: "refinement",
                    code: Code::UndefinedExpression,
                    cause: ModelRefusalCause::UnprovedRefinement,
                    detail: format!(
                        "no establishing interval fact for {} (obligation field-domain)",
                        redefining_key.node
                    ),
                }])),
            }
        }
        _ => Ok(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "refinement",
            code: Code::UndefinedExpression,
            cause: ModelRefusalCause::UnprovedRefinement,
            detail: format!(
                "{} narrows an object-typed domain, which no FR-146 fact form expresses (obligation no-proof-form)",
                redefining_key.node
            ),
        }])),
    }
}
