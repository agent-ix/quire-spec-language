// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Quantified reads": `forall(e in self.f: B)` and `exists(e in
//! self.f: B)` over a clock collection `self.f` in a precondition, `B` a
//! Boolean combination of data predicates over `e`'s data part and atomic
//! clock constraints over `e`'s clocks.
//!
//! The guard of a `forall` is, for each size `s` from the collection's
//! minimum to its maximum, the part "the collection holds `s` elements"
//! conjoined with `B` read at each slot below `s` (true on an empty
//! collection). The guard of an `exists` is one part for each slot `j`,
//! "the collection holds slot `j`" conjoined with `B` read at slot `j`
//! (false on an empty collection). `B` read at slot `j` constrains the
//! clocks of slot `j` and reads its data through an [`ElementBinding`],
//! the binder renamed for the slot.

use std::collections::BTreeSet;
use std::rc::Rc;

use qsl_forms::{
    BinaryOperator, BinderQuery, ExprId, ExprNode, Expression, ExpressionForm, ExpressionSpans,
};

use super::reading::free_occurrences;
use super::{
    self_path, timed, ClockConstraint, ClockId, ClockOp, Clocks, DataLiteral, Dnf, ElementBinding,
    FamilyDraft, Located, PartBudget, ResetDraft,
};
use crate::check::lowering::{CollectionShape, PlaceStep};
use crate::check::refusal::{CheckCause, TemporalCause};
use quire_checked::key::DeclarationKey;
use quire_exact::EffectiveId;

/// A quantifier over a clock collection field of the receiver.
struct Quantifier<'f> {
    query: BinderQuery,
    binder: &'f str,
    /// The field names from the receiver to the collection: its field, or
    /// a collection below a record field.
    path: Vec<String>,
    shape: CollectionShape,
    body: ExprId,
    /// The quantifier's span.
    span: qsl_foundation::Span,
}

/// The quantifier at node `id` when it ranges over a clock collection
/// field of an object of type `context`: `forall` or `exists`, its source
/// `self.f` with `f` holding clock places below its slot.
fn quantifier<'f>(
    located: &Located<'f>,
    id: ExprId,
    clocks: &Clocks<'_>,
    context: EffectiveId,
) -> Option<Quantifier<'f>> {
    let Some(ExprNode::Query {
        query: query @ (BinderQuery::Forall | BinderQuery::Exists),
        binder,
        source,
        body,
    }) = located.node(id)
    else {
        return None;
    };
    let path = self_path(located.form, *source)?;
    let shape = clocks.fields.iter().find_map(|clock| {
        if !clocks.model.conforms(context, clock.owner) || !clock.path.starts_with(&path) {
            return None;
        }
        match clock.path.get(path.len()) {
            Some(PlaceStep::Slot(shape)) => Some(*shape),
            _ => None,
        }
    })?;
    let path = path
        .into_iter()
        .map(|step| match step {
            PlaceStep::Field(name) => Some(name),
            PlaceStep::Slot(_) => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Quantifier {
        query: *query,
        binder: binder.as_str(),
        path,
        shape,
        body: *body,
        span: located.span(id),
    })
}

/// The body nodes of every quantifier over a clock collection in
/// `located`: [`read`] reads them, so the guard's own reading skips them.
pub(super) fn bodies(
    located: &Located<'_>,
    clocks: &Clocks<'_>,
    context: EffectiveId,
) -> BTreeSet<ExprId> {
    let mut found = BTreeSet::new();
    for node in located.form.expression.iter() {
        let Some(quantifier) = quantifier(located, node.id(), clocks, context) else {
            continue;
        };
        let mut pending = vec![quantifier.body];
        while let Some(at) = pending.pop() {
            if found.insert(at) {
                if let Some(node) = located.node(at) {
                    pending.extend(node.children());
                }
            }
        }
    }
    found
}

/// The disjunctive forms, read positively and negated, of the quantifier
/// over a clock collection at node `id`; `None` when the node is none.
///
/// # Errors
///
/// The refusals of [`slot`] for its body. Over a collection with no
/// declared maximum the read is the family of the entries indexed by size,
/// a [`DataLiteral::Family`] whose body is read once at
/// [`ElementBinding::FAMILY_SLOT`].
pub(super) fn read(
    located: &Located<'_>,
    id: ExprId,
    clocks: &Clocks<'_>,
    context: EffectiveId,
    budget: &mut PartBudget,
) -> Result<Option<(Dnf, Dnf)>, CheckCause> {
    let Some(quantifier) = quantifier(located, id, clocks, context) else {
        return Ok(None);
    };
    let span = located.span(id);
    let Some(max) = quantifier.shape.max else {
        // The family of the entries indexed by size: the body read once at
        // the template slot, instantiated at each held slot by the reading.
        let binding = ElementBinding {
            name: format!("{}#{}#n", quantifier.binder, quantifier.path.join(".")),
            path: quantifier.path.clone(),
            slot: ElementBinding::FAMILY_SLOT,
        };
        let (positive, negative) = slot(
            located,
            quantifier.body,
            &quantifier,
            &binding,
            clocks,
            budget,
        )?;
        let family = Rc::new(FamilyDraft {
            forall: quantifier.query == BinderQuery::Forall,
            path: quantifier.path.clone(),
            positive,
            negative,
        });
        return Ok(Some((
            vec![(
                vec![(DataLiteral::Family(family.clone()), true)],
                Vec::new(),
            )],
            vec![(vec![(DataLiteral::Family(family), false)], Vec::new())],
        )));
    };
    let mut slots = Vec::new();
    for at in 0..max {
        let binding = ElementBinding {
            name: format!("{}#{}#{at}", quantifier.binder, quantifier.path.join(".")),
            path: quantifier.path.clone(),
            slot: at,
        };
        slots.push(slot(
            located,
            quantifier.body,
            &quantifier,
            &binding,
            clocks,
            budget,
        )?);
    }
    let size = |operator: BinaryOperator, count: u64| -> Result<DataLiteral, CheckCause> {
        let expression = Expression::binary(
            operator,
            Expression::size(
                quantifier
                    .path
                    .iter()
                    .fold(Expression::self_ref(), |operand, name| {
                        Expression::field(operand, name.clone())
                    }),
            ),
            Expression::integer(count),
        );
        let spans = ExpressionSpans::at_point(&expression, span.start)
            .map_err(|_| timed(located.outside(id)))?;
        Ok(DataLiteral::Formed(Rc::new(ExpressionForm {
            expression,
            spans,
        })))
    };
    // `size(self.f) = s` conjoined with `pick(j)` of each slot below `s`.
    let each =
        |pick: &dyn Fn(&(Dnf, Dnf)) -> Dnf, budget: &mut PartBudget| -> Result<Dnf, CheckCause> {
            let mut parts = Vec::new();
            for count in quantifier.shape.min..=max {
                let mut entry: Dnf = vec![(
                    vec![(size(BinaryOperator::Equal, count)?, true)],
                    Vec::new(),
                )];
                for read in slots
                    .iter()
                    .take(usize::try_from(count).unwrap_or(usize::MAX))
                {
                    entry = budget.product(&entry, &pick(read))?;
                }
                parts.extend(entry);
            }
            Ok(parts)
        };
    // `size(self.f) > j` conjoined with `pick(j)`, for each slot `j`.
    let some = |pick: &dyn Fn(&(Dnf, Dnf)) -> Dnf,
                budget: &mut PartBudget|
     -> Result<Dnf, CheckCause> {
        let mut parts = Vec::new();
        for (at, read) in (0_u64..).zip(&slots) {
            let held: Dnf = vec![(vec![(size(BinaryOperator::Greater, at)?, true)], Vec::new())];
            parts.extend(budget.product(&held, &pick(read))?);
        }
        Ok(parts)
    };
    let positive = |read: &(Dnf, Dnf)| read.0.clone();
    let negative = |read: &(Dnf, Dnf)| read.1.clone();
    Ok(Some(match quantifier.query {
        BinderQuery::Forall => (each(&positive, budget)?, some(&negative, budget)?),
        _ => (some(&positive, budget)?, each(&negative, budget)?),
    }))
}

/// The disjunctive forms, read positively and negated, of the quantifier
/// body node `id` at the slot of `binding`.
///
/// # Errors
///
/// `clock-read-outside-constraint` at a clock read no constraint takes,
/// `clock-comparison` for a constraint whose bound reads a clock, and the
/// refusals of a constant bound.
fn slot(
    located: &Located<'_>,
    id: ExprId,
    quantifier: &Quantifier<'_>,
    binding: &ElementBinding,
    clocks: &Clocks<'_>,
    budget: &mut PartBudget,
) -> Result<(Dnf, Dnf), CheckCause> {
    if !located.reads_clock(id) {
        let literal = element_literal(located, id, quantifier, binding)?;
        return Ok((
            vec![(vec![(literal.clone(), true)], Vec::new())],
            vec![(vec![(literal, false)], Vec::new())],
        ));
    }
    match located.node(id) {
        Some(ExprNode::Not(operand)) => {
            let (positive, negative) =
                slot(located, *operand, quantifier, binding, clocks, budget)?;
            Ok((negative, positive))
        }
        Some(ExprNode::Binary {
            operator:
                operator @ (BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies),
            left,
            right,
        }) => {
            let (lp, ln) = slot(located, *left, quantifier, binding, clocks, budget)?;
            let (rp, rn) = slot(located, *right, quantifier, binding, clocks, budget)?;
            Ok(match operator {
                BinaryOperator::And => (budget.product(&lp, &rp)?, [ln, rn].concat()),
                BinaryOperator::Or => ([lp, rp].concat(), budget.product(&ln, &rn)?),
                _ => ([ln, rp].concat(), budget.product(&lp, &rn)?),
            })
        }
        Some(ExprNode::Binary {
            operator,
            left,
            right,
        }) => {
            let span = located.span(id);
            let (clock, op, constant) = match (
                element_clock(located, *left, quantifier, clocks),
                element_clock(located, *right, quantifier, clocks),
            ) {
                (Some(_), Some(_)) => return Err(timed(TemporalCause::ClockComparison { span })),
                (Some(clock), None) => (clock, ClockOp::of(*operator), *right),
                (None, Some(clock)) => {
                    (clock, ClockOp::of(*operator).map(ClockOp::mirrored), *left)
                }
                (None, None) => return Err(timed(located.outside(id))),
            };
            let Some(op) = op else {
                return Err(timed(located.outside(id)));
            };
            if located.reads_clock(constant) {
                return Err(timed(TemporalCause::ClockComparison { span }));
            }
            let bound = located.constant(constant, clocks).map_err(timed)?;
            let constraint = |op: ClockOp| ClockConstraint {
                clock,
                via: Vec::new(),
                slots: vec![binding.slot],
                op,
                bound: bound.clone(),
                span,
            };
            Ok((
                vec![(Vec::new(), vec![constraint(op)])],
                op.negated()
                    .into_iter()
                    .map(|negated| (Vec::new(), vec![constraint(negated)]))
                    .collect(),
            ))
        }
        _ => Err(timed(located.outside(id))),
    }
}

/// The clock place body node `id` reads of an element bound by the
/// quantifier: `e.x`, `e.r.x`, a clock of the collection's element type.
fn element_clock(
    located: &Located<'_>,
    id: ExprId,
    quantifier: &Quantifier<'_>,
    clocks: &Clocks<'_>,
) -> Option<ClockId> {
    let mut steps = Vec::new();
    let mut at = id;
    loop {
        match located.node(at)? {
            ExprNode::Field { operand, field } => {
                steps.push(field.clone());
                at = *operand;
            }
            ExprNode::Name(name) if name.segments.len() == 1 && name.name == quantifier.binder => {
                break;
            }
            _ => return None,
        }
    }
    // The element itself, a clock of a bare clock collection
    // (`b: Bag<Clock>`, latent until the IR admits one: IR-1036), or a
    // clock below it.
    steps.reverse();
    let position = clocks.fields.iter().position(|clock| {
        let Some((head, rest)) = clock.path.split_at_checked(quantifier.path.len()) else {
            return false;
        };
        let [PlaceStep::Slot(_), below @ ..] = rest else {
            return false;
        };
        head.iter()
            .zip(&quantifier.path)
            .all(|(step, name)| matches!(step, PlaceStep::Field(field) if field == name))
            && below.len() == steps.len()
            && below
                .iter()
                .zip(&steps)
                .all(|(step, name)| matches!(step, PlaceStep::Field(below) if below == name))
    })?;
    Some(ClockId(position))
}

/// The data predicate body node `id` states of the element at the slot of
/// `binding`: the node with each free occurrence of `binder` renamed to the
/// binding's name.
fn element_literal(
    located: &Located<'_>,
    id: ExprId,
    quantifier: &Quantifier<'_>,
    binding: &ElementBinding,
) -> Result<DataLiteral, CheckCause> {
    let form = renamed(located, id, quantifier.binder, &binding.name).map_err(timed)?;
    Ok(DataLiteral::Element(
        Rc::new(form),
        binding.clone(),
        quantifier.span,
    ))
}

/// Body node `id` with each free occurrence of `binder` renamed to `name`,
/// located at the node.
fn renamed(
    located: &Located<'_>,
    id: ExprId,
    binder: &str,
    name: &str,
) -> Result<ExpressionForm, TemporalCause> {
    let names: BTreeSet<&str> = std::iter::once(binder).collect();
    let renamed = Expression::name(name.to_owned());
    let grafts = free_occurrences(&located.form.expression, id, &names)
        .into_iter()
        .map(|(occurrence, _)| (occurrence, &renamed))
        .collect();
    let expression = located
        .form
        .expression
        .graft(id, &grafts)
        .ok_or_else(|| located.outside(id))?;
    let spans = ExpressionSpans::at_point(&expression, located.span(id).start)
        .map_err(|_| located.outside(id))?;
    Ok(ExpressionForm { expression, spans })
}

/// The reset a postcondition conjunct `forall(e in self.f: P(e) implies
/// e.x = c)` or `forall(e in self.f: e.x = c)` writes (FR-230 "Resets by
/// selection"): the clock `x` of each held slot whose element satisfies
/// `P`, to the exact constant `c`. `None` when node `id` is no quantifier
/// over a clock collection.
///
/// # Errors
///
/// `clock-write` for an `exists`, a body of another shape, a value that
/// is no exact non-negative time constant or a collection field the
/// operation's frame does not name; `clock-read-outside-constraint` at a
/// clock read in `P`.
pub(super) fn reset(
    located: &Located<'_>,
    id: ExprId,
    clocks: &Clocks<'_>,
    context: EffectiveId,
    written: &BTreeSet<&DeclarationKey>,
) -> Result<Option<ResetDraft>, TemporalCause> {
    let Some(quantifier) = quantifier(located, id, clocks, context) else {
        return Ok(None);
    };
    let span = located.span(id);
    if quantifier.query != BinderQuery::Forall {
        return Err(TemporalCause::ClockWrite { span });
    }
    let (premise, write) = match located.node(quantifier.body) {
        Some(ExprNode::Binary {
            operator: BinaryOperator::Implies,
            left,
            right,
        }) => (Some(*left), *right),
        _ => (None, quantifier.body),
    };
    if let Some(premise) = premise {
        if located.reads_clock(premise) {
            return Err(located.outside(premise));
        }
    }
    let Some(ExprNode::Binary {
        operator: BinaryOperator::Equal,
        left,
        right,
    }) = located.node(write)
    else {
        return Err(TemporalCause::ClockWrite { span });
    };
    let (clock, value) = match (
        element_clock(located, *left, &quantifier, clocks),
        element_clock(located, *right, &quantifier, clocks),
    ) {
        (Some(clock), None) => (clock, *right),
        (None, Some(clock)) => (clock, *left),
        _ => return Err(TemporalCause::ClockWrite { span }),
    };
    if located.reads_clock(value) {
        return Err(TemporalCause::ClockWrite { span });
    }
    let value = match located.constant(value, clocks) {
        Ok(value) => value,
        Err(TemporalCause::ClockBoundNotConstant { .. }) => {
            return Err(TemporalCause::ClockWrite { span })
        }
        Err(cause) => return Err(cause),
    };
    let in_frame = clocks
        .fields
        .get(clock.index())
        .is_some_and(|field| written.contains(&field.field));
    if !in_frame {
        return Err(TemporalCause::ClockWrite { span });
    }
    let name = format!("{}#{}", quantifier.binder, quantifier.path.join("."));
    let predicate = premise
        .map(|premise| renamed(located, premise, quantifier.binder, &name))
        .transpose()?;
    Ok(Some(ResetDraft {
        clock,
        value,
        span,
        selection: Some((quantifier.path, name, predicate)),
    }))
}
