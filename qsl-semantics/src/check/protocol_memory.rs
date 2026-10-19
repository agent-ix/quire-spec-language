// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-219 "Source" and FR-220 "Admitted orderings": the S3 checks of a
//! `parallel`'s `memory` clause and of a `fence` control (ADR-025 MM-1,
//! MM-3, MM-6).
//!
//! A `parallel` with no clause has the default `sc`. A `parallel` nested in
//! another declares the default of its outermost `parallel` or none; any
//! other default refuses, naming both controls. A `parallel` that declares
//! a weak default (`tso` or `ra`) joins `all` with no `outstanding` clause.
//! A fence takes `acquire`, `release`, `acq_rel` or `seq_cst`; the grammar
//! admits no other spelling, so every fence S3 sees is recorded as written.

use qsl_forms::{
    FenceOrdering, JoinPolicy, MemoryModelName, Outstanding, ParallelForm, ProtocolDeclarationForm,
};
use qsl_foundation::Span;

use super::protocol_clause::ProtocolNodeId;
use super::refusal::{CheckCause, CheckRefusal, ProtocolAnchorCause};
use quire_semantic_value::location::{Location, Origin};

/// One checked `parallel` and its source default memory model (FR-219
/// "Outputs"). The model the checker runs it under is the request's
/// selection when it names the outermost `parallel`, else this default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedParallel {
    /// The `parallel` control.
    pub node: ProtocolNodeId,
    /// The `memory` clause's model, `Sc` when the clause is absent.
    pub memory: MemoryModelName,
}

/// One checked `fence` control and its ordering (FR-220 "Outputs").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedFence {
    /// The `fence` control.
    pub node: ProtocolNodeId,
    /// The fence's ordering.
    pub ordering: FenceOrdering,
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // A protocol has no expression tree for a `Location` path to walk;
        // the cause's own span is read directly.
        location: Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// The `parallel` that encloses no other and encloses `inner`, when `inner`
/// is nested in any.
fn outermost<'a>(parallels: &'a [ParallelForm], inner: &ParallelForm) -> Option<&'a ParallelForm> {
    parallels
        .iter()
        .filter(|outer| outer.span.start <= inner.span.start && inner.span.end <= outer.span.end)
        .filter(|outer| outer.declaration != inner.declaration)
        .min_by_key(|outer| outer.span.start)
}

/// How a weak `parallel`'s join reads, when it is not `join all` alone.
fn unsound_policy(parallel: &ParallelForm) -> Option<String> {
    let join = match &parallel.join.value {
        JoinPolicy::All => None,
        JoinPolicy::Any => Some("any".to_owned()),
        JoinPolicy::Quorum(count) => Some(format!("quorum({count})")),
        JoinPolicy::Predicate(name) => Some(format!("predicate({name})")),
    };
    let outstanding = parallel.outstanding.as_ref().map(|rule| match rule.value {
        Outstanding::Continue => "outstanding continue",
        Outstanding::Cancel => "outstanding cancel",
    });
    match (join, outstanding) {
        (None, None) => None,
        (Some(join), None) => Some(join),
        (None, Some(rule)) => Some(format!("all {rule}")),
        (Some(join), Some(rule)) => Some(format!("{join} {rule}")),
    }
}

/// Check every `parallel`'s `memory` clause and record every `parallel`'s
/// default and every `fence`'s ordering.
///
/// # Errors
///
/// A nested `parallel` whose default differs from its outermost
/// `parallel`'s refuses `invalid_model_binding`/`conflicting-binding`; a
/// `parallel` declaring `tso` or `ra` whose join is not `all`, or that has
/// an `outstanding` clause, refuses `unsupported_construct`/
/// `declaration-form` at the join. Refusals are in source position order.
pub fn check(
    protocol: &ProtocolDeclarationForm,
) -> Result<(Vec<CheckedParallel>, Vec<CheckedFence>), Vec<CheckRefusal>> {
    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();
    let mut parallels = Vec::with_capacity(protocol.parallels.len());
    for parallel in &protocol.parallels {
        let default = parallel
            .memory
            .as_ref()
            .map_or(MemoryModelName::Sc, |memory| memory.value);
        parallels.push(CheckedParallel {
            node: ProtocolNodeId(parallel.declaration),
            memory: default,
        });
        let Some(declared) = &parallel.memory else {
            continue;
        };
        if let Some(outer) = outermost(&protocol.parallels, parallel) {
            let outer_model = outer
                .memory
                .as_ref()
                .map_or(MemoryModelName::Sc, |memory| memory.value);
            if declared.value != outer_model {
                refusals.push((
                    declared.span,
                    refusal(ProtocolAnchorCause::ConflictingMemory {
                        parallel: parallel.name.name.clone(),
                        model: declared.value,
                        outermost: outer.name.name.clone(),
                        outermost_model: outer_model,
                        span: declared.span,
                    }),
                ));
            }
        }
        if declared.value.is_weak() {
            if let Some(policy) = unsound_policy(parallel) {
                refusals.push((
                    parallel.join.span,
                    refusal(ProtocolAnchorCause::WeakJoin {
                        parallel: parallel.name.name.clone(),
                        model: declared.value,
                        policy,
                        span: parallel.join.span,
                    }),
                ));
            }
        }
    }
    let fences = protocol
        .fences
        .iter()
        .map(|fence| CheckedFence {
            node: ProtocolNodeId(fence.declaration),
            ordering: fence.ordering.value,
        })
        .collect();
    if refusals.is_empty() {
        return Ok((parallels, fences));
    }
    refusals.sort_by_key(|(span, _)| span.start);
    Err(refusals.into_iter().map(|(_, refusal)| refusal).collect())
}

#[cfg(test)]
mod tests;
