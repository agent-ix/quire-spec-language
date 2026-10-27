// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Collection-valued writes", executed: the collection a
//! postcondition conjunct `self.f = E` writes, computed from the pre-state,
//! and the provenance of each of its slots (QSpec FR-181 "Timed subjects":
//! a carried clock moves with its element, a fresh one takes its literal's
//! constant).
//!
//! The written collection is evaluated whole, and each occurrence of `E` is
//! walked beside its checked [`Written`] record to say where it comes from.
//! Which occurrence each slot of the result is, the caller decides by value
//! (see [`WrittenCollection::Computed`]).

use qsl_semantics::check::{
    CheckedGraph, CheckedNode, ClockId, Connective, NodeKind, Visit, Written,
};
use quire_eval::FamilyOutcome;
use quire_exact::{Outcome, Rational, Value};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::declaration::EqualityOperator;

use super::evaluate::Machine;
use super::{CallFailure, ClauseSetup, Meter};

/// Where one slot of a written collection comes from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Provenance {
    /// Carried from the slot `slot` of the pre-state collection `field` of
    /// the receiver: its clocks keep their values.
    Carried {
        /// The receiver's collection field.
        field: String,
        /// The slot in the pre-state.
        slot: usize,
    },
    /// Fresh, each clock at its literal's constant.
    Fresh(Vec<(ClockId, Rational)>),
}

/// What a collection write writes at one pre- and post-state.
#[derive(Debug)]
pub enum WrittenCollection {
    /// The written collection and its occurrences. A slot of `value` is the
    /// occurrence it equals: the `k`-th slot equal to a value is the `k`-th
    /// occurrence equal to it, so a `Set` or `OrderedSet` keeps the first of
    /// equal occurrences and a `Bag` or `Sequence` keeps them all, by value
    /// and never by spelling.
    Computed {
        /// The written value.
        value: Value,
        /// Each occurrence of the written expression, in FR-145's visiting
        /// order: its value and provenance.
        occurrences: Vec<(Value, Provenance)>,
    },
    /// The write reads a collection it carries from in the post-state
    /// (`self.g` rather than `pre(self.g)`, FR-104), so it constrains the
    /// post-state and is no function of the pre-state (STD-493 item 28).
    ReadsPostState,
    /// An evaluation the write needs had no value.
    Undecided(Box<FamilyOutcome<Value>>),
}

/// An occurrence of the written expression: its value and provenance.
type Occurrence = (Value, Provenance);

/// Why the walk stopped short of a provenance.
enum Short {
    PostState,
    Undecided(Box<FamilyOutcome<Value>>),
    Fault(&'static str),
}

/// The write of `field` in `setup`'s clause, `written` its checked record.
pub(super) fn evaluate(
    graph: &CheckedGraph,
    setup: &ClauseSetup<'_>,
    field: &str,
    written: &Written,
    meter: &mut Meter,
) -> Result<Option<WrittenCollection>, CallFailure> {
    let Some(right) = written_side(setup.declaration.body(), field) else {
        return Ok(None);
    };
    let mut frame: Vec<Option<Value>> = setup.bindings.iter().cloned().map(Some).collect();
    frame.resize(setup.declaration.slots().max(frame.len()), None);
    let mut walk = Walk {
        graph,
        setup,
        meter,
        frame,
        carried: Vec::new(),
    };
    let result = walk
        .value(right)
        .and_then(|value| Ok((value, walk.occurrences(written, right)?)));
    let (value, occurrences) = match result {
        Ok(found) => found,
        Err(Short::PostState) => return Ok(Some(WrittenCollection::ReadsPostState)),
        Err(Short::Undecided(outcome)) => return Ok(Some(WrittenCollection::Undecided(outcome))),
        Err(Short::Fault(what)) => {
            return Err(CallFailure::Fault(InternalFault::new("S6a", what)));
        }
    };
    Ok(Some(WrittenCollection::Computed { value, occurrences }))
}

/// The value a postcondition conjunct `self.f = E` writes.
#[derive(Debug)]
pub enum WrittenValue {
    /// `E`'s value.
    Computed(Value),
    /// `E` had no value.
    Undecided(Box<FamilyOutcome<Value>>),
    /// `E` reads the written field in the post-state, so it constrains the
    /// field and computes no value of it.
    ReadsPostState,
}

/// The value the top-level conjunct `self.field = E` of `setup`'s clause
/// writes, `E` evaluated with the clause's pre- and post-states; `None`
/// when the clause has no such conjunct.
pub(super) fn evaluate_write(
    graph: &CheckedGraph,
    setup: &ClauseSetup<'_>,
    field: &str,
    meter: &mut Meter,
) -> Result<Option<WrittenValue>, CallFailure> {
    let Some(right) = written_side(setup.declaration.body(), field) else {
        return Ok(None);
    };
    // `self.f = E` where `E` reads `self.f` in the post-state constrains
    // `f` and computes no value of it.
    if reads_post_field(right, field) {
        return Ok(Some(WrittenValue::ReadsPostState));
    }
    let mut frame: Vec<Option<Value>> = setup.bindings.iter().cloned().map(Some).collect();
    frame.resize(setup.declaration.slots().max(frame.len()), None);
    let mut walk = Walk {
        graph,
        setup,
        meter,
        frame,
        carried: Vec::new(),
    };
    match walk.value(right) {
        Ok(value) => Ok(Some(WrittenValue::Computed(value))),
        Err(Short::Undecided(outcome)) => Ok(Some(WrittenValue::Undecided(outcome))),
        Err(Short::PostState) => Err(CallFailure::Fault(InternalFault::new(
            "S6a",
            "write-evaluation-post-state",
        ))),
        Err(Short::Fault(what)) => Err(CallFailure::Fault(InternalFault::new("S6a", what))),
    }
}

/// Whether `node` reads `self.field` outside a `pre(...)`.
fn reads_post_field(node: CheckedNode<'_>, field: &str) -> bool {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        match node.kind() {
            NodeKind::Pre(_) => {}
            NodeKind::Attribute {
                reference,
                field: read,
                ..
            } if read.name == field && matches!(node.at(*reference).kind(), NodeKind::Local(0)) => {
                return true;
            }
            _ => pending.extend(node.children()),
        }
    }
    false
}

/// The right side of the top-level conjunct `self.field = E` of `body`.
fn written_side<'a>(body: CheckedNode<'a>, field: &str) -> Option<CheckedNode<'a>> {
    let mut pending = vec![body];
    while let Some(node) = pending.pop() {
        match node.kind() {
            NodeKind::Connective(Connective::And, left, right) => {
                pending.push(node.at(*right));
                pending.push(node.at(*left));
            }
            NodeKind::Equality(EqualityOperator::Equal, _, left, right) => {
                let left = node.at(*left);
                if let NodeKind::Attribute {
                    reference,
                    field: written,
                    ..
                } = left.kind()
                {
                    if written.name == field
                        && matches!(left.at(*reference).kind(), NodeKind::Local(0))
                    {
                        return Some(node.at(*right));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

struct Walk<'w, 'a, 'm> {
    graph: &'w CheckedGraph,
    setup: &'w ClauseSetup<'a>,
    meter: &'m mut Meter,
    /// The clause's frame, each traversal's binder set while its body is
    /// walked.
    frame: Vec<Option<Value>>,
    /// The provenance of each bound traversal element, by binder slot.
    carried: Vec<(usize, Provenance)>,
}

impl Walk<'_, '_, '_> {
    /// `node`'s value on the current frame.
    fn value(&mut self, node: CheckedNode<'_>) -> Result<Value, Short> {
        let graph = self.graph;
        let reads: std::collections::BTreeMap<_, _> = self
            .setup
            .declaration
            .reads()
            .map(|(location, observation)| (location.clone(), observation))
            .collect();
        let evaluation = Machine::with_pre(
            graph.scope(),
            graph,
            self.setup.current,
            self.setup.pre,
            Some(&reads),
            self.meter,
            graph.dispatch_tables(),
        )
        .run_frame(node, self.frame.clone())
        .map_err(|_| Short::Fault("collection-write-evaluation-fault"))?;
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => Ok(value),
            other => Err(Short::Undecided(Box::new(other))),
        }
    }

    /// The occurrences of the collection `node`, checked as `written`.
    fn occurrences(
        &mut self,
        written: &Written,
        node: CheckedNode<'_>,
    ) -> Result<Vec<Occurrence>, Short> {
        let shape = || Short::Fault("collection-write-shape");
        match (written, node.kind()) {
            (Written::Field { pre: false, .. }, _) => Err(Short::PostState),
            (Written::Field { field, pre: true }, _) => {
                let Value::Collection(collection) = self.value(node)? else {
                    return Err(shape());
                };
                Ok(collection
                    .elements()
                    .iter()
                    .enumerate()
                    .map(|(slot, element)| {
                        (
                            element.clone(),
                            Provenance::Carried {
                                field: field.clone(),
                                slot,
                            },
                        )
                    })
                    .collect())
            }
            (Written::Literal { occurrences, .. }, NodeKind::Collection { elements, .. }) => {
                if occurrences.len() != elements.len() {
                    return Err(shape());
                }
                occurrences
                    .iter()
                    .zip(elements)
                    .map(|(written, element)| self.element(written, node.at(*element)))
                    .collect()
            }
            (Written::Flatten(inner), NodeKind::Flatten(operand)) => {
                let operand = node.at(*operand);
                let (Written::Literal { occurrences, .. }, NodeKind::Collection { elements, .. }) =
                    (&**inner, operand.kind())
                else {
                    return Err(shape());
                };
                if occurrences.len() != elements.len() {
                    return Err(shape());
                }
                let mut flat = Vec::new();
                for (written, element) in occurrences.iter().zip(elements) {
                    flat.extend(self.occurrences(written, operand.at(*element))?);
                }
                Ok(flat)
            }
            (Written::Convert(inner), NodeKind::ConvertCollection { operand, .. }) => {
                self.occurrences(inner, node.at(*operand))
            }
            (
                Written::Filter(source),
                NodeKind::Query {
                    visit: Visit::Filter,
                    slot,
                    source: from,
                    body,
                },
            ) => {
                let mut kept = Vec::new();
                for (value, provenance) in self.occurrences(source, node.at(*from))? {
                    if matches!(
                        self.bound(*slot, value.clone(), |walk| walk.value(node.at(*body)))?,
                        Value::Boolean(true)
                    ) {
                        kept.push((value, provenance));
                    }
                }
                Ok(kept)
            }
            (
                Written::Map { source, body },
                NodeKind::Query {
                    visit: Visit::Map,
                    slot,
                    source: from,
                    body: mapped,
                },
            ) => {
                let mut out = Vec::new();
                for (value, provenance) in self.occurrences(source, node.at(*from))? {
                    self.carried.push((*slot, provenance));
                    let element =
                        self.bound(*slot, value, |walk| walk.element(body, node.at(*mapped)));
                    self.carried.pop();
                    out.push(element?);
                }
                Ok(out)
            }
            _ => Err(shape()),
        }
    }

    /// The one occurrence the element `node`, checked as `written`, is.
    fn element(&mut self, written: &Written, node: CheckedNode<'_>) -> Result<Occurrence, Short> {
        match (written, node.kind()) {
            (Written::Element(_), NodeKind::Local(slot)) => {
                let value = self
                    .frame
                    .get(*slot)
                    .cloned()
                    .flatten()
                    .ok_or(Short::Fault("collection-write-unbound-element"))?;
                let provenance = self
                    .carried
                    .iter()
                    .rev()
                    .find(|(bound, _)| bound == slot)
                    .map(|(_, provenance)| provenance.clone())
                    .ok_or(Short::Fault("collection-write-unbound-element"))?;
                Ok((value, provenance))
            }
            (Written::Fresh(clocks), NodeKind::Record { .. }) => {
                Ok((self.value(node)?, Provenance::Fresh(clocks.clone())))
            }
            _ => Err(Short::Fault("collection-write-shape")),
        }
    }

    /// `run` with the binder slot `slot` bound to `value`.
    fn bound<T>(
        &mut self,
        slot: usize,
        value: Value,
        run: impl FnOnce(&mut Self) -> Result<T, Short>,
    ) -> Result<T, Short> {
        if self.frame.len() <= slot {
            self.frame.resize(slot + 1, None);
        }
        let held = self.frame.get_mut(slot).map(|held| held.replace(value));
        let result = run(self);
        if let (Some(previous), Some(held)) = (held, self.frame.get_mut(slot)) {
            *held = previous;
        }
        result
    }
}
