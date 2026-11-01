// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-044: activation dispositions, instance identity and immutable captures.
//!
//! Instance identity is the admitted semantic trigger identity or the admitted
//! execution-origin identity, never a timestamp, display name, receipt or equal
//! payload. Captures are established once at the authored anchor and are not
//! recomputed by any later evaluation.

use super::budget::{Dimension as Charge, Work};
use super::result::{
    Activation, Capture, Completeness, Dimension, Error, Incomplete, Refusal, Subject,
};
use super::trace::{CaptureInput, Closure, Evidence, Trace, Trigger};
use std::collections::BTreeMap;

/// One obligation instance, keyed by its semantic identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Instance {
    pub identity: String,
    /// Delivery receipts in delivery order. A repeat adds provenance only.
    pub receipts: Vec<String>,
    pub captures: Vec<Capture>,
    /// Exactly one for an activated instance. Incremental re-evaluation,
    /// restoration and replay never increment it.
    pub capture_evaluations: usize,
    /// Opaque payload of the first admitted delivery, compared only to detect
    /// a conflicting redelivery under one semantic trigger identity.
    pub payload: String,
}

/// An admitted trigger whose activation could not be established. Its semantic
/// identity is retained: it is known whenever the trigger was admitted, which
/// is every case an activation can fail in.
pub(super) struct Failed {
    pub identity: String,
    pub error: Error,
}

/// One admitted trigger's resolution. A retained instance is represented by
/// identity and newly received receipts so activation does not deep-clone its
/// history before formula evaluation settles.
pub(super) enum Resolved {
    /// Newly established instance, held locally until its formula settles.
    Owned(Instance),
    /// Existing instance kept in the ledger; only new delivery receipts are
    /// held locally while formula evaluation is pending.
    Retained {
        identity: String,
        receipts: Vec<String>,
    },
    /// An activation that could not be established for this trigger.
    Failed(Failed),
}

/// What activation produced: either instances to assess, or one disposition
/// that carries no obligation.
pub(super) enum Outcome {
    /// One entry per instance-creating trigger. Activation failing for one
    /// leaves every sibling entry inspectable.
    Instances(Vec<Resolved>),
    Disposition(Activation),
}

/// Whether the declaration activates on a whole-execution origin or on a
/// trigger binder with an optional guard.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Shape {
    pub origin: bool,
    pub guarded: bool,
    /// Number of captures the declaration authored, in source order.
    pub captures: usize,
}

/// Resolve activation for one declaration against one trace.
///
/// A guard evaluating false creates no instance and is not a refusal. An
/// unestablished guard is not a false guard. Only a closed-complete trigger
/// scope with no instance-creating trigger is inactive.
pub(super) fn resolve(
    shape: Shape,
    trace: &Trace,
    declaration: usize,
    work: &mut Work,
    mut retained: Option<&mut BTreeMap<String, Instance>>,
) -> Result<Outcome, Error> {
    let subject = Subject {
        declaration,
        ..Subject::default()
    };
    work.subject = subject;

    match trace.trigger_evidence {
        Evidence::Missing => {
            return Ok(Outcome::Disposition(Activation::Unknown {
                completeness: Completeness::Incomplete,
                execution: trace.execution,
            }))
        }
        Evidence::Refused => {
            return Ok(Outcome::Disposition(Activation::Unknown {
                completeness: trace.completeness,
                execution: super::result::Execution::Failed,
            }))
        }
        Evidence::Admitted => {}
    }

    let mut instances: Vec<Resolved> = Vec::new();
    let mut created = 0usize;
    for (ordinal, trigger) in trace.triggers.iter().enumerate() {
        work.visit()?;
        work.subject = Subject {
            instance: ordinal,
            ..subject
        };

        if let Some(existing) = instances.iter_mut().find_map(|resolved| match resolved {
            Resolved::Owned(instance) if instance.identity == trigger.identity => Some(instance),
            _ => None,
        }) {
            // A repeated delivery of one semantic trigger creates no second
            // instance. Conflicting payloads under one identity are a typed
            // contradiction rather than a silent alias.
            if existing.payload != trigger.payload {
                return Err(Refusal::Contradiction {
                    identity: trigger.identity.clone(),
                    subject: work.subject,
                }
                .into());
            }
            existing.receipts.push(trigger.receipt.clone());
            continue;
        }

        if let Some(receipts) = instances.iter_mut().find_map(|resolved| match resolved {
            Resolved::Retained { identity, receipts } if identity == &trigger.identity => {
                Some(receipts)
            }
            _ => None,
        }) {
            let existing = retained
                .as_deref()
                .and_then(|retained| retained.get(&trigger.identity))
                .expect("a retained resolution has a ledger instance");
            if existing.payload != trigger.payload {
                return Err(Refusal::Contradiction {
                    identity: trigger.identity.clone(),
                    subject: work.subject,
                }
                .into());
            }
            receipts.push(trigger.receipt.clone());
            continue;
        }

        if let Some(retained_instances) = retained.as_deref_mut() {
            if let Some(existing) = retained_instances.get(&trigger.identity) {
                if existing.payload != trigger.payload {
                    return Err(Refusal::Contradiction {
                        identity: trigger.identity.clone(),
                        subject: work.subject,
                    }
                    .into());
                }
                let mut evicted_capture = None;
                for evicted in &trace.evicted {
                    work.visit()?;
                    if let super::trace::Eviction::Capture { instance, capture } = evicted {
                        if instance == &trigger.identity {
                            evicted_capture = Some(*capture);
                            break;
                        }
                    }
                }
                if let Some(capture) = evicted_capture {
                    work.subject.capture = Some(capture);
                    if let Some(saved) = retained_instances.get_mut(&trigger.identity) {
                        saved.receipts.push(trigger.receipt.clone());
                    }
                    instances.push(Resolved::Failed(Failed {
                        identity: trigger.identity.clone(),
                        error: Incomplete {
                            dimension: Dimension::Capture,
                            subject: work.subject,
                        }
                        .into(),
                    }));
                    work.subject.capture = None;
                    continue;
                }
                instances.push(Resolved::Retained {
                    identity: trigger.identity.clone(),
                    receipts: vec![trigger.receipt.clone()],
                });
                continue;
            }
        }

        if shape.guarded && !shape.origin {
            match trigger.guard {
                // A false guard creates no instance and is not a refusal.
                Some(false) => continue,
                Some(true) => {}
                None => {
                    // An unestablished guard is not a false guard. It is
                    // reported against its own trigger, so a healthy sibling
                    // instance neither erases it nor is erased by it.
                    instances.push(Resolved::Failed(Failed {
                        identity: trigger.identity.clone(),
                        error: Incomplete {
                            dimension: Dimension::Guard,
                            subject: work.subject,
                        }
                        .into(),
                    }));
                    continue;
                }
            }
        }

        created = created.saturating_add(1);
        work.charge(Charge::Instances, created)?;
        match establish(shape, trigger, trace, work) {
            Ok(captures) => {
                work.charge(Charge::Captures, captures.len())?;
                let instance = Instance {
                    identity: trigger.identity.clone(),
                    receipts: vec![trigger.receipt.clone()],
                    captures,
                    capture_evaluations: 1,
                    payload: trigger.payload.clone(),
                };
                instances.push(Resolved::Owned(instance));
            }
            // An exhausted ceiling stops the whole evaluation; an unestablished
            // capture stops only its own instance.
            Err(Error::Exhausted(exhaustion)) => return Err(Error::Exhausted(exhaustion)),
            Err(error) => instances.push(Resolved::Failed(Failed {
                identity: trigger.identity.clone(),
                error,
            })),
        }
    }

    if instances.is_empty() {
        return Ok(Outcome::Disposition(disposition(trace)));
    }
    Ok(Outcome::Instances(instances))
}

/// The disposition of a scope that created no instance.
fn disposition(trace: &Trace) -> Activation {
    match (trace.trigger_scope, trace.completeness) {
        (Closure::Closed, Completeness::Complete) => Activation::Inactive,
        (Closure::Closed, Completeness::Incomplete) => Activation::Unknown {
            completeness: Completeness::Incomplete,
            execution: trace.execution,
        },
        (Closure::Open, completeness) => Activation::Unknown {
            completeness,
            execution: trace.execution,
        },
    }
}

/// Evaluate every declared capture once, in authored source order, at the
/// authored activation anchor.
fn establish(
    shape: Shape,
    trigger: &Trigger,
    trace: &Trace,
    work: &mut Work,
) -> Result<Vec<Capture>, Error> {
    work.subject.capture = None;
    if trigger.captures.len() != shape.captures {
        return Err(Incomplete {
            dimension: Dimension::Capture,
            subject: work.subject,
        }
        .into());
    }
    if trigger.anchor != trace.anchor {
        return Err(Refusal::Capture {
            dimension: Dimension::Anchor,
            subject: work.subject,
        }
        .into());
    }
    let mut established = Vec::with_capacity(shape.captures);
    for (declared, input) in trigger.captures.iter().enumerate() {
        work.visit()?;
        // Name which capture, not only that one failed.
        work.subject.capture = Some(declared);
        let mut evicted = false;
        for record in &trace.evicted {
            work.visit()?;
            if matches!(
                record,
                super::trace::Eviction::Capture { instance, capture }
                    if *instance == trigger.identity && *capture == declared
            ) {
                evicted = true;
                break;
            }
        }
        if evicted {
            return Err(Incomplete {
                dimension: Dimension::Capture,
                subject: work.subject,
            }
            .into());
        }
        match input {
            CaptureInput::Value { anchor, value } => {
                if anchor != &trace.anchor {
                    return Err(Refusal::Capture {
                        dimension: Dimension::Anchor,
                        subject: work.subject,
                    }
                    .into());
                }
                established.push(Capture {
                    declared,
                    anchor: anchor.clone(),
                    value: value.clone(),
                });
            }
            CaptureInput::Missing | CaptureInput::Null => {
                return Err(Incomplete {
                    dimension: Dimension::Capture,
                    subject: work.subject,
                }
                .into())
            }
            CaptureInput::WrongType | CaptureInput::Stale => {
                return Err(Refusal::Capture {
                    dimension: Dimension::Capture,
                    subject: work.subject,
                }
                .into())
            }
        }
    }
    work.subject.capture = None;
    Ok(established)
}
