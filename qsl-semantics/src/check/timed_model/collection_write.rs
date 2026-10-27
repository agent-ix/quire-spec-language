// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Collection-valued writes": a postcondition conjunct `self.f =
//! E`, `f` a clock collection field of the receiver, with `E` a
//! place-preserving collection expression. The checker records what each
//! occurrence of the written value is: carried from the slot of the
//! pre-state element it comes from, or fresh with the exact constants of a
//! record literal's clock components.

use std::collections::BTreeSet;

use qsl_forms::{BinderQuery, ExprId, ExprNode, FieldInitializer};
use qsl_foundation::Span;
use quire_checked::key::DeclarationKey;
use quire_exact::{CollectionKind, EffectiveId, Rational};

use super::{self_path, ClockId, Clocks, Located, RecordWrite, ResetCut};
use crate::check::lowering::PlaceStep;
use crate::check::refusal::TemporalCause;

/// What a clock collection write writes (FR-230 `Provenance`, read over
/// the written expression): each occurrence carried from its pre-state
/// slot or fresh.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Written {
    /// `self.g`: each element carried from its slot of the pre-state
    /// collection `g`.
    Field(String),
    /// The element a traversal binds, carried from its slot.
    Element,
    /// A record literal: fresh, its clocks at the exact constants of its
    /// clock components.
    Fresh(Vec<(ClockId, Rational)>),
    /// A collection literal of `kind`, its occurrences in source order. Of
    /// a `Set` or `OrderedSet`, an occurrence equal by data part to an
    /// earlier one is left out, the first kept.
    Literal {
        /// The literal's kind.
        kind: CollectionKind,
        /// The occurrences, in source order.
        occurrences: Vec<Written>,
    },
    /// `filter(e in S: P)`: the occurrences of `S` whose element satisfies
    /// `P`, each keeping its provenance.
    Filter(Box<Written>),
    /// `flatten(S)`: the occurrences of each collection of `S`, in order.
    Flatten(Box<Written>),
    /// `convert<T>(S)`: the occurrences of `S` in the target collection.
    Convert(Box<Written>),
    /// `map(e in S: B)`: `B` for each occurrence of `S`, the bound element
    /// carried or a fresh literal.
    Map {
        /// The mapped collection.
        source: Box<Written>,
        /// What the body writes for each occurrence.
        body: Box<Written>,
    },
}

/// A write `self.f = E` of a clock collection field (FR-230
/// "Collection-valued writes").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionWrite {
    field: String,
    written: Written,
    span: Span,
}

impl CollectionWrite {
    /// The receiver's written collection field.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// What it writes.
    pub fn written(&self) -> &Written {
        &self.written
    }

    /// The span of the conjunct.
    pub fn span(&self) -> Span {
        self.span
    }
}

/// The receiver's clock collection field node `id` names: `self.f` whose
/// clock places lie below a slot of `f`.
pub(super) fn collection_field(
    located: &Located<'_>,
    id: ExprId,
    clocks: &Clocks<'_>,
    context: EffectiveId,
) -> Option<(String, Vec<PlaceStep>)> {
    let path = self_path(located.form, id)?;
    let [PlaceStep::Field(field)] = path.as_slice() else {
        return None;
    };
    clocks.fields.iter().find_map(|clock| {
        if !clocks.model.conforms(context, clock.owner) || !clock.path.starts_with(&path) {
            return None;
        }
        match clock.path.get(path.len()) {
            Some(PlaceStep::Slot(_)) => Some((field.clone(), clock.path[..=path.len()].to_vec())),
            _ => None,
        }
    })
}

/// One collection write being read: the conjunct's write context and the
/// slot path of the written field.
pub(super) struct Writing<'w, 'f, 'c> {
    pub(super) record: RecordWrite<'w, 'f, 'c>,
    /// `[f, Slot]`: the path to an element of the written field.
    pub(super) element: Vec<PlaceStep>,
}

impl Writing<'_, '_, '_> {
    /// The write `self.f = value` of the clock collection field `field`.
    ///
    /// # Errors
    ///
    /// `clock-write` when the operation's frame does not name `f`, and the
    /// refusals of [`Self::written`].
    pub(super) fn write(
        &self,
        field: String,
        key: Option<&DeclarationKey>,
        value: ExprId,
        cut: &mut ResetCut,
    ) -> Result<CollectionWrite, TemporalCause> {
        let span = self.record.span;
        if !key.is_some_and(|key| self.record.written.contains(key)) {
            return Err(TemporalCause::ClockWrite { span });
        }
        let written = self.written(value, &BTreeSet::new(), cut)?;
        Ok(CollectionWrite {
            field,
            written,
            span,
        })
    }

    /// What node `id` of the written value writes, with `bound` the
    /// binders of the traversals of clock collections around it.
    ///
    /// # Errors
    ///
    /// `clock-write` at an occurrence that is neither carried nor a record
    /// literal with constant clock components.
    fn written(
        &self,
        id: ExprId,
        bound: &BTreeSet<&str>,
        cut: &mut ResetCut,
    ) -> Result<Written, TemporalCause> {
        let located = self.record.located;
        let refused = || TemporalCause::ClockWrite {
            span: located.span(id),
        };
        match located.node(id) {
            Some(ExprNode::Name(name))
                if name.segments.len() == 1 && bound.contains(name.name.as_str()) =>
            {
                Ok(Written::Element)
            }
            Some(ExprNode::Record { .. }) => {
                let mut components = Vec::new();
                self.record
                    .components(self.element.clone(), id, cut, &mut components)
                    .map_err(|cause| match cause {
                        // A clock component that is no constant, or left
                        // out: the occurrence is neither carried nor fresh.
                        TemporalCause::ClockReadOutsideConstraint { .. }
                        | TemporalCause::ClockWrite { .. } => refused(),
                        other => other,
                    })?;
                Ok(Written::Fresh(
                    components
                        .into_iter()
                        .map(|(clock, value, _)| (clock, value))
                        .collect(),
                ))
            }
            Some(ExprNode::Collection { kind, elements }) => {
                let mut occurrences: Vec<(ExprId, Written)> = Vec::new();
                for element in elements {
                    let written = self.written(*element, bound, cut)?;
                    let unique = matches!(kind, CollectionKind::Set | CollectionKind::OrderedSet);
                    if unique
                        && occurrences
                            .iter()
                            .any(|(earlier, _)| self.equal_data(*earlier, *element))
                    {
                        continue;
                    }
                    occurrences.push((*element, written));
                }
                Ok(Written::Literal {
                    kind: *kind,
                    occurrences: occurrences
                        .into_iter()
                        .map(|(_, written)| written)
                        .collect(),
                })
            }
            Some(ExprNode::Flatten(operand)) => Ok(Written::Flatten(Box::new(
                self.written(*operand, bound, cut)?,
            ))),
            Some(ExprNode::Convert { operand, .. }) => Ok(Written::Convert(Box::new(
                self.written(*operand, bound, cut)?,
            ))),
            Some(ExprNode::Query {
                query: query @ (BinderQuery::Filter | BinderQuery::Map),
                binder,
                source,
                body,
            }) => {
                let source = Box::new(self.written(*source, bound, cut)?);
                let mut inner = bound.clone();
                inner.insert(binder.as_str());
                if *query == BinderQuery::Filter {
                    if let Some(span) = located.first(*body) {
                        return Err(TemporalCause::ClockReadOutsideConstraint { span });
                    }
                    return Ok(Written::Filter(source));
                }
                let body = self.written(*body, &inner, cut)?;
                if !matches!(body, Written::Element | Written::Fresh(_)) {
                    return Err(refused());
                }
                Ok(Written::Map {
                    source,
                    body: Box::new(body),
                })
            }
            _ => match self_path(located.form, id) {
                Some(path) => match path.as_slice() {
                    [PlaceStep::Field(field)]
                        if self.record.clocks.fields.iter().any(|clock| {
                            clock.path.len() > 1
                                && clock.path[0] == path[0]
                                && matches!(clock.path[1], PlaceStep::Slot(_))
                        }) =>
                    {
                        Ok(Written::Field(field.clone()))
                    }
                    _ => Err(refused()),
                },
                None => Err(refused()),
            },
        }
    }

    /// Whether the record literals `left` and `right` are equal by data
    /// part: each data component the same literal.
    fn equal_data(&self, left: ExprId, right: ExprId) -> bool {
        let located = self.record.located;
        let data = |id: ExprId| -> Option<Vec<(&str, &ExprNode)>> {
            let ExprNode::Record { fields, .. } = located.node(id)? else {
                return None;
            };
            let mut components = Vec::new();
            for (name, initializer) in fields {
                let mut below = self.element.clone();
                below.push(PlaceStep::Field(name.clone()));
                if self
                    .record
                    .places_below(&self.element)
                    .any(|(_, clock)| clock.path == below)
                {
                    continue;
                }
                let FieldInitializer::Value(value) = initializer else {
                    return None;
                };
                let node = located.node(*value)?;
                if !node.children().is_empty() {
                    return None;
                }
                components.push((name.as_str(), node));
            }
            components.sort_by_key(|(name, _)| *name);
            Some(components)
        };
        matches!((data(left), data(right)), (Some(left), Some(right)) if left == right)
    }
}
