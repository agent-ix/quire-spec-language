// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facade's call-aware requirement association (FR-395): run one call as
//! FR-100's [`run`](crate::spine::run) does and return, with its outcome, the
//! identities of the requirement items that belong to the selected function.
//!
//! The members come from the same compile as the call and the `package_id`.
//! The check stage recorded each `value-validity` record's owner (FR-062
//! "Owner"); this module reads that field and recomputes nothing. A consumer
//! reaches [`run_with_requirements`] and its result types through the
//! `qsl_replay` root and never names [`crate::spine`] (FR-060 T12-A).

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{RequestIndex, SourceIdentity};
use qsl_semantics::check::{Capability, CheckedGraph, RequirementRecord};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{planned_equality, Cancel, Meter, Outcome, Value};

use crate::spine::{
    default_accounting, run_reading, Call, CallArgument, CallOutcome, DependencyInput, RunRefusal,
    SpineLimits,
};

/// One requirement item of the selected call: the record's occurrence key and
/// the item's request index (FR-075), the position of the key in the
/// package's records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallRequirement {
    occurrence: OccurrenceKey,
    request_index: RequestIndex,
}

impl CallRequirement {
    /// The record's occurrence key, which is the item's `occurrence()`.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The item's request index in the package order.
    pub fn request_index(&self) -> RequestIndex {
        self.request_index
    }
}

/// The requirement items of the selected call (FR-395 Outputs): the
/// `package_id` of the compile that produced them, the call as received and
/// its members in ascending occurrence-key order. The pairing key of a member
/// and its item is (`package_id`, request index).
#[derive(Clone, Debug)]
pub struct CallRequirements {
    package_id: PackageId,
    function: String,
    arguments: Vec<CallArgument>,
    members: Vec<CallRequirement>,
}

impl CallRequirements {
    /// The `package_id` of the compile that produced the members.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    /// The selected function's name, as received.
    pub fn function(&self) -> &str {
        &self.function
    }

    /// The selected call's arguments, in the order they were received.
    pub fn arguments(&self) -> &[CallArgument] {
        &self.arguments
    }

    /// The members, in ascending occurrence-key order. Empty for a function
    /// with no scalar operation application; that is not a refusal.
    pub fn members(&self) -> &[CallRequirement] {
        &self.members
    }

    /// Whether `other` is the same call: the same function name and the same
    /// set of (parameter, value) pairs, whatever order the arguments arrived
    /// in (FR-395 "Selected call"). Values compare by the kernel's value
    /// equality, and populations by their id, so a call is the same call as
    /// itself whatever its argument kinds.
    pub fn same_call(&self, other: &Self) -> bool {
        if self.function != other.function || self.arguments.len() != other.arguments.len() {
            return false;
        }
        self.arguments.iter().all(|argument| {
            let mut named = other
                .arguments
                .iter()
                .filter(|candidate| candidate.parameter == argument.parameter);
            matches!(
                (named.next(), named.next()),
                (Some(candidate), None)
                    if same_value(&argument.value, &candidate.value)
            )
        })
    }
}

/// Whether two argument values are equal: a population by its id (the
/// kernel's equality refuses a population pair), any other value by the
/// kernel's planned equality under unlimited accounting.
fn same_value(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Population(left), Value::Population(right)) => left == right,
        _ => matches!(
            planned_equality(left, right, &mut Meter::new(default_accounting(u64::MAX))),
            Outcome::Completed(true)
        ),
    }
}

/// What [`run_with_requirements`] returns: `run`'s `package_id` and outcome
/// and the selected call's [`CallRequirements`], all from one compile.
#[derive(Debug)]
pub struct RunWithRequirements {
    package_id: PackageId,
    outcome: CallOutcome,
    requirements: CallRequirements,
}

impl RunWithRequirements {
    /// The compiled package's id, as `run` returns it for the same inputs.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    /// The call's outcome, as `run` returns it for the same inputs.
    pub fn outcome(&self) -> &CallOutcome {
        &self.outcome
    }

    /// The selected call's requirement items.
    pub fn requirements(&self) -> &CallRequirements {
        &self.requirements
    }

    /// The package id, the outcome and the requirement items.
    pub fn into_parts(self) -> (PackageId, CallOutcome, CallRequirements) {
        (self.package_id, self.outcome, self.requirements)
    }
}

/// Compile `bytes` once, call `call`'s function as FR-100's
/// [`run`](crate::spine::run) does, and return the call's `package_id` and
/// outcome with the requirement items whose owner is the selected function
/// (FR-395). Arguments do not select items: the members of a call whose
/// outcome is refused, undefined or incomplete are those of a completed call.
///
/// The refusals are `run`'s, and a refusal before the call returns no
/// members. A `value-validity` record with no owner, or one whose owner names
/// no function of the package, is a `RunRefusal::Fault` of stage `spine-run`
/// and invariant `requirement-owner-known`.
#[allow(
    clippy::too_many_arguments,
    reason = "run's signature (FR-100): the request's fields, the object context and the cancel handle"
)]
pub fn run_with_requirements(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    cancel: &Cancel,
) -> Result<RunWithRequirements, Box<RunRefusal>> {
    let (package_id, outcome, members) = run_reading(
        source,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        call,
        objects,
        cancel,
        |graph| {
            associate(graph.requirements(), graph, &call.function)
                .map_err(|fault| Box::new(RunRefusal::Fault(fault)))
        },
    )?;
    Ok(RunWithRequirements {
        package_id,
        outcome,
        requirements: CallRequirements {
            package_id,
            function: call.function.clone(),
            arguments: call.arguments.clone(),
            members,
        },
    })
}

/// The association step (FR-395 "Association rule"): the members of
/// `function`, one per record it owns, in ascending key order with each
/// record's position in `records` as its request index. Every
/// `value-validity` record must have an owner that names a function of
/// `graph`, a function synthesized from a clause included; otherwise no member list is returned.
pub(crate) fn associate(
    records: &BTreeMap<OccurrenceKey, RequirementRecord>,
    graph: &CheckedGraph,
    function: &str,
) -> Result<Vec<CallRequirement>, InternalFault> {
    let mut members = Vec::new();
    for (position, (occurrence, record)) in records.iter().enumerate() {
        let owner = record.owner();
        if record.requirements().kind() == Capability::ValueValidity
            && !owner.is_some_and(|name| graph.declares_function(name))
        {
            return Err(InternalFault::new("spine-run", "requirement-owner-known"));
        }
        if owner == Some(function) {
            members.push(CallRequirement {
                occurrence: occurrence.clone(),
                request_index: RequestIndex::new(position),
            });
        }
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::SourceIdentity;

    use super::*;
    use crate::spine::{self, ParseRequest};

    const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n\
        function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }\n";

    fn checked(test: impl FnOnce(&CheckedGraph)) {
        let source = SourceIdentity::new("a", "u", "git", "1");
        let limits = SpineLimits::default();
        let cancel = Cancel::new();
        let parsed = spine::parse(
            &ParseRequest {
                source: &source,
                path: "unit.native",
                bytes: SOURCE.as_bytes(),
            },
            limits.source,
            &cancel,
        )
        .expect("parses")
        .into_value();
        let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)
            .expect("selects")
            .into_value();
        let unit = spine::check(
            &parsed,
            &models,
            &DependencyInput::default(),
            &spine::LockEvidence::default(),
            limits,
            &cancel,
        )
        .expect("checks")
        .into_value();
        test(unit.package().graph());
    }

    /// FR-395-AC-10: a `value-validity` record with no owner, or with an
    /// owner no function bears, is a `spine-run` fault with invariant
    /// `requirement-owner-known` and no partial member list; unaltered
    /// records associate.
    #[trace("TC-1089", "FR-395-AC-10")]
    #[test]
    fn a_record_with_a_missing_or_unknown_owner_is_a_fault() {
        checked(|graph| {
            let records = graph.requirements();
            let members = associate(records, graph, "sq").expect("unaltered records associate");
            assert_eq!(members.len(), 3);
            for owner in [None, Some("ghost".to_owned())] {
                let mut altered = records.clone();
                let first = altered.values_mut().next().expect("a record");
                *first = first.clone().with_owner(owner.clone());
                let fault = associate(&altered, graph, "sq").expect_err("a fault");
                assert_eq!(fault.stage(), "spine-run", "{owner:?}");
                assert_eq!(fault.invariant(), "requirement-owner-known", "{owner:?}");
            }
        });
    }

    /// FR-395-AC-11: argument values compare by kernel equality, and a
    /// population by its id, so a call is the same call as itself.
    #[trace("TC-1089", "FR-395-AC-11")]
    #[test]
    fn a_population_argument_is_equal_to_itself_and_not_to_another() {
        use quire_exact::{Integer, PopulationId};
        let population = |fill| Value::Population(PopulationId::from_digest([fill; 32]));
        assert!(same_value(&population(1), &population(1)));
        assert!(!same_value(&population(1), &population(2)));
        let integer = |value: i64| Value::Integer(Integer::from(value));
        assert!(same_value(&integer(3), &integer(3)));
        assert!(!same_value(&integer(3), &integer(4)));
    }
}
