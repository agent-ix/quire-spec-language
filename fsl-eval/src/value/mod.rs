// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S6a value evaluator (ADR-011 §6.1 layer 5).
//!
//! The layer-3 value semantics this evaluator runs over -- the §6.2
//! `semantic_value` submodules and `value::model_query` -- live in the `fsl-semantics` crate's own
//! `value` module (X-6b). This module re-exports none of them
//! (ADR-011 §7.2): callers name them at `fsl_semantics::value::...`. It holds
//! `value::expression`, which `fsl-eval` takes at X-8, and re-exports only
//! its own items. The layer-4 `CheckedPackage` it evaluates is
//! `fsl_package::CheckedPackage` (X-7), re-exported by none of this
//! crate.

pub mod collections;
pub mod composite;
mod expression;
pub(crate) use expression::evaluate_clause_set_with_cancel;
pub(crate) use expression::evaluate_hyper_holds_with_component_bindings;
pub use expression::{
    ModelRelationEvaluation, S6aRewardEvaluator, evaluate_model_relation,
    evaluate_model_relation_transitions,
};

/// FSpec FR-144 canonical key order of two map keys. Float keys use their
/// membership identity's IEEE total order; integer and decimal keys use their
/// mathematical ordering through `compare_keys`.
pub(crate) fn compare_map_keys(
    left: &filament_exact::Value,
    right: &filament_exact::Value,
) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (filament_exact::Value::Float(left), filament_exact::Value::Float(right)) => {
            Some(left.total_order_key().cmp(&right.total_order_key()))
        }
        _ => filament_exact::compare_keys(left, right),
    }
}

/// The value of the entry of `map` whose key equals `key` in FSpec FR-144
/// canonical key order: an entry is located by its key's canonical key, so no
/// member is walked or tested.
pub(crate) fn map_entry<'m>(
    map: &'m filament_exact::MapValue,
    key: &filament_exact::Value,
) -> Option<&'m filament_exact::Value> {
    map.entries()
        .iter()
        .find(|(held, _)| compare_map_keys(held, key) == Some(std::cmp::Ordering::Equal))
        .map(|(_, value)| value)
}

/// Map entries in FSpec FR-144 canonical key order. Float keys use their
/// membership identity's IEEE total order; integer and decimal keys use their
/// mathematical ordering through `compare_keys`.
pub(crate) fn ordered_map_entries(
    map: &filament_exact::MapValue,
) -> Option<Vec<&(filament_exact::Value, filament_exact::Value)>> {
    use std::cmp::Ordering;

    let mut entries = Vec::new();
    entries.try_reserve(map.entries().len()).ok()?;
    entries.extend(map.entries().iter());
    let mut incomparable = false;
    entries.sort_unstable_by(|(left, _), (right, _)| {
        compare_map_keys(left, right).unwrap_or_else(|| {
            incomparable = true;
            Ordering::Equal
        })
    });
    (!incomparable).then_some(entries)
}

// `CheckedPackageEvaluation` is the trait that carries `call` and `evaluate`
// over the foreign-to-`value` `CheckedPackage` typestate, since an inherent
// impl here is E0116: `CheckedPackage` is `fsl-package`'s own type (X-7).
pub use expression::collection_write::{Provenance, WrittenCollection};
pub(crate) use expression::evaluate_over_admitted_environment;
pub(crate) use expression::evaluate_state_predicate_at_absent_key_with_reference_bindings;
pub(crate) use expression::evaluate_state_predicate_with_reference_bindings;
pub use expression::object_environment_refusal;
pub use expression::post_targets::{PostTarget, PostTargets};
pub use expression::{
    AdmittedEvaluationViewEvaluation, ApplicationObservation, ApplicationObservationOptions,
    BINDER_ACCOUNTING_VERSION, BinderCharges, ChargeAmounts, CheckedPackageEvaluation,
    ClauseEvaluation, ClauseRead, ClauseSetEntry, ClauseSetEvaluation, ClauseSetOutcome,
    ComparisonReport, FrameEvaluation, HoldsAtom, HoldsBinders, InvalidQualifiedName,
    InvocationBinding, ObservationIdentity, OperationCharge, OperationLedger, PackageClosure,
    PackageTask, ProtocolReferenceOccurrence, ProtocolSlot, ProtocolSlotEvaluation,
    ProtocolSuppliedValue, ProtocolSupply, QualifiedName, RuntimeValuePath, Separation,
    SeparationStep, StopReport, SuppliedValue, TruncatedOperationLedger, ValueInvariantEvaluation,
    ValueInvariantOutcome, ValueInvariantResult, ValuePathStep, ValuePathSubject, WitnessClaim,
    admit_arguments, call_checked_package_with_session_limits, evaluate_activation_guard,
    evaluate_activation_guard_at_absent_key,
    evaluate_capture, evaluate_closed_with_frame, evaluate_history_update, evaluate_join_predicate,
    evaluate_operation_contract, evaluate_plan_body, evaluate_plan_node, evaluate_protocol_slot,
    evaluate_refinement_row, evaluate_reward,
    evaluate_state_predicate, evaluate_state_predicate_at_absent_key,
    evaluate_state_predicate_with, evaluate_terminal_predicate, input_refusal_code, refusal_record,
    supplied_stop_cause, supplied_stop_code, undefined_contract_code,
};
pub use expression::{PackageOperations, ProjectedReward, bare_evaluate_frame};
pub(crate) use expression::{
    ProjectedClause, StateGraphOver, bare_post_targets, evaluate_bare_state_graph_predicate,
    evaluate_bare_terminal_predicate, package_constraint_reads, package_guard_reads,
    package_operation_footprint, package_term_reads, source_post_targets,
};
pub use expression::{bare_evaluate_selected_protocol_slot, evaluate_bare_expression};

#[cfg(test)]
mod map_order_tests {
    use super::{map_entry, ordered_map_entries};
    use filament_exact::{Integer, MapType, Value, ValueType, from_admitted_map};
    use ix_trace_rs::trace;

    fn integer(value: i64) -> Value {
        Value::Integer(Integer::from(value))
    }

    fn map(entries: &[(i64, i64)]) -> filament_exact::MapValue {
        let map_type = MapType::new(ValueType::Integer, ValueType::Integer, None)
            .expect("an integer key is scalar");
        let entries = entries
            .iter()
            .map(|(key, value)| (integer(*key), integer(*value)))
            .collect();
        match from_admitted_map(map_type, entries).expect("distinct keys") {
            Value::Map(map) => (*map).clone(),
            other => panic!("a map value, not {other:?}"),
        }
    }

    fn whole(value: &Value) -> Integer {
        match value {
            Value::Integer(value) => value.clone(),
            other => panic!("an Integer, not {other:?}"),
        }
    }

    /// FR-650-AC-17: a walk of a map visits its entries in the mathematical
    /// key order, so key 9 comes before key 10, although the run form array
    /// sorts the spelling of 10 before that of 9.
    #[trace("TC-2950", "FR-650-AC-17")]
    #[test]
    fn a_map_walk_visits_values_in_mathematical_key_order() {
        let held = map(&[(10, 100), (9, 90)]);
        let walked: Vec<Integer> = ordered_map_entries(&held)
            .expect("integer keys are ordered")
            .into_iter()
            .map(|(_, value)| whole(value))
            .collect();
        assert_eq!(walked, vec![Integer::from(90_i64), Integer::from(100_i64)]);
        assert_eq!(
            whole(map_entry(&held, &integer(10)).expect("key 10 is held")),
            Integer::from(100_i64)
        );
        assert!(map_entry(&held, &integer(11)).is_none());
    }
}
