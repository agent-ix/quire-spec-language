// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-139 (TC-544): the S3 check of the population-valued forms inside a
//! refinement row, and the subject-admission check of the bounds that depend
//! on a population's universe size, over ADR-020 §8's ring buffer refined to
//! a queue (`tests/support/ring_queue.rs`).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::Code;
use qsl_semantics::check::{
    CheckedRefinement, CheckedRow, PopulationUse, PopulationValue, RefinementCause,
};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{CardinalityBound, CollectionKind, CollectionType, IntegerInterval, ValueType};

#[path = "../../../tests/support/ring_queue.rs"]
mod ring_queue;

use ring_queue::{all_rows, checked, key, refused, ring_is_queue, ITEMS, RING};

/// `Int[lower, upper]`.
fn interval(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(lower.into(), upper.into()).expect("an interval"))
}

fn slots() -> DeclarationKey {
    key(RING, "slots")
}

/// The field rows of the `Queue` object row, in `all_rows` order: `items`,
/// `used`, `busy`, `cells`, `lone`, `ghost`, `solid`, `total`.
fn queue_row(refinement: &CheckedRefinement, field_index: usize) -> &CheckedRow {
    let [queue, _cell] = refinement.objects() else {
        panic!("the queue and the cell rows: {:?}", refinement.objects());
    };
    &queue.fields[field_index].expression
}

/// FR-139-AC-1 (TC-544 step 1): the `items` row checks at S3 with `n`
/// symbolic: it depends on no universe size, so it records no bound, and its
/// `seq` is the declared `Sequence<Int[0, 1]>[0, 2]` its maximum length 2
/// gives. `count` checks as an integer, `exists` as a Boolean and the
/// population as its objects: the rows whose value depends on `n` record the
/// bound the subject's admission checks.
#[trace(
    "TC-544",
    "FR-139-AC-1",
    "QSpec-TC-311",
    "QSpec-FR-376-AC-1",
    "QSpec-FR-376-AC-2"
)]
#[test]
fn the_ring_rows_check_at_s3_with_the_universe_size_symbolic() {
    let graph = checked(&ring_is_queue(&all_rows())).graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let items = queue_row(refinement, 0);
    assert!(
        items.population_uses().is_empty(),
        "{:?}",
        items.population_uses()
    );
    assert_eq!(
        items.body().value_type(),
        &ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            interval(0, 1),
            Some(CardinalityBound::new(0, 2).expect("a bound")),
        ))
    );
    let busy = queue_row(refinement, 2);
    assert_eq!(busy.body().value_type(), &ValueType::Boolean);
    assert!(busy.population_uses().is_empty());
    let used = queue_row(refinement, 1);
    assert_eq!(used.body().value_type(), &ValueType::Integer);

    let [count] = used.population_uses() else {
        panic!("the count row records its bound");
    };
    assert_eq!(count.value(), &PopulationValue::Count);
    assert_eq!(count.population(), "R::slots");
    assert_eq!(count.population_key(), &slots());
    let [cells] = queue_row(refinement, 3).population_uses() else {
        panic!("the population row records its bound");
    };
    assert_eq!(cells.value(), &PopulationValue::Objects);
}

/// FR-139-AC-1 (TC-544 step 1), at subject admission: with `slots = {s0,
/// s1}`, `R::slots` is typed `Set<Reference<R::Slot>>[0, 2]` and `count` is
/// `Int[0, 2]`, which conform to the types they are used as.
#[trace("TC-544", "FR-139-AC-1")]
#[test]
fn admission_substitutes_the_universe_size_into_the_recorded_bounds() {
    let graph = checked(&ring_is_queue(&all_rows())).graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let uses: Vec<&PopulationUse> = [1, 3]
        .into_iter()
        .flat_map(|index| queue_row(refinement, index).population_uses())
        .collect();
    let [count, objects] = uses[..] else {
        panic!("two recorded bounds");
    };
    assert_eq!(count.admitted_type(2), Some(interval(0, 2)));
    let admitted = objects.admitted_type(2);
    let Some(ValueType::Collection(set)) = &admitted else {
        panic!("a collection");
    };
    assert_eq!(set.kind(), CollectionKind::Set);
    assert_eq!(
        set.bound(),
        Some(CardinalityBound::new(0, 2).expect("a bound"))
    );
    assert_eq!(admitted.as_ref(), Some(objects.required()));
    let universes = BTreeMap::from([(slots(), 2)]);
    assert_eq!(refinement.admit_universes(&universes), Ok(()));
}

/// FR-139-AC-5 (TC-544 step 5): the `count` row mapped to an abstract field
/// of type `Int[0, 2]` checks at S3 with `n` symbolic and is admitted with
/// `slots = {s0, s1}`; with `{s0, s1, s2}` it is refused
/// `ill_typed`/`type-mismatch` before exploration, naming the row,
/// `R::slots` and the size 3. This is verified at the `admit_universes`
/// seam; a request that calls it is wired by QSL-544, which must add the
/// request-level assertion.
#[trace("TC-544")]
#[test]
fn a_universe_that_overflows_the_used_type_is_refused_at_admission() {
    let graph = checked(&ring_is_queue(&format!(
        "{ITEMS}\nused = count(s in R::slots where s.ring = self);"
    )))
    .graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    assert_eq!(
        refinement.admit_universes(&BTreeMap::from([(slots(), 2)])),
        Ok(())
    );
    let Err(found) = refinement.admit_universes(&BTreeMap::from([(slots(), 3)])) else {
        panic!("a universe of three slots overflows Int[0, 2]");
    };
    let [refusal] = &found[..] else {
        panic!("one refusal: {found:?}");
    };
    assert_eq!(
        refusal.cause,
        RefinementCause::PopulationBound {
            row: "Q::Queue.used".to_owned(),
            population: "R::slots".to_owned(),
            size: 3,
        }
    );
    assert_eq!(
        (refusal.cause.code(), refusal.cause.cause()),
        (Code::IllTyped, Some("type-mismatch"))
    );
    // A subject that supplies no universe for the population admits nothing
    // about it.
    let Err(missing) = refinement.admit_universes(&BTreeMap::new()) else {
        panic!("no universe for R::slots");
    };
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].cause.code(), Code::MissingDeclaration);
}

/// The one refusal of `rows`, with its code, cause and the text it is at.
fn only_refusal(rows: &str) -> (Code, Option<&'static str>, String) {
    let found = refused(&ring_is_queue(rows));
    let [refusal] = &found[..] else {
        panic!(
            "one refusal: {:?}",
            found.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        );
    };
    (
        refusal.refusal.cause.code(),
        refusal.refusal.cause.cause(),
        refusal.at.clone(),
    )
}

/// FR-139-AC-2 (TC-544 step 2): `only` over a condition that is not Boolean
/// refuses `ill_typed`/`type-mismatch` at the condition.
#[trace("TC-544", "FR-139-AC-2", "QSpec-TC-311", "QSpec-FR-376-AC-4")]
#[test]
fn a_non_boolean_condition_refuses_at_the_condition() {
    assert_eq!(
        only_refusal("lone = only(s in R::slots where s.index: s.value);"),
        (Code::IllTyped, Some("type-mismatch"), "s.index".to_owned())
    );
}

/// FR-139-AC-2 (TC-544 step 2): an invariant of `R::Ring` reading `count(s
/// in R::slots where s.ring = self)` refuses `unsupported_construct`/
/// `expression-form`: state-clause bodies keep ADR-016 FE-4.
#[trace("TC-544", "FR-139-AC-2", "QSpec-TC-311", "QSpec-FR-376-AC-4")]
#[test]
fn a_population_form_in_an_invariant_refuses_as_an_expression_form() {
    let found = refused(
        "invariant Total using v on R::Ring at current { \
         count(s in R::slots where s.ring = self) = 2 }",
    );
    let [refusal] = &found[..] else {
        panic!(
            "one refusal: {:?}",
            found.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        );
    };
    assert_eq!(
        (refusal.refusal.cause.code(), refusal.refusal.cause.cause()),
        (Code::UnsupportedConstruct, Some("expression-form"))
    );
    assert_eq!(refusal.at, "count(s in R::slots where s.ring = self)");
}

/// FR-139 behavior: a binder over a name that is not a concrete population
/// of the refinement's concrete model refuses `missing_declaration`/
/// `missing-name`.
#[trace("TC-544", "QSpec-TC-311", "QSpec-FR-376-AC-4")]
#[test]
fn a_binder_over_an_unknown_population_refuses_as_a_missing_name() {
    let (code, cause, _) = only_refusal("busy = exists(s in R::nowhere where s.value = 1);");
    assert_eq!(
        (code, cause),
        (Code::MissingDeclaration, Some("missing-name"))
    );
}

/// FR-139 behavior: a `seq` bound that is not an integer refuses
/// `ill_typed`/`type-mismatch` at the bound.
#[trace("TC-544", "QSpec-TC-311", "QSpec-FR-376-AC-4")]
#[test]
fn a_seq_bound_that_is_not_an_integer_refuses_at_the_bound() {
    assert_eq!(
        only_refusal("items = seq(i in 0 .. self: i);"),
        (Code::IllTyped, Some("type-mismatch"), "self".to_owned())
    );
}

/// FR-139-AC-1/AC-5 at S3: a row whose result shape cannot fit the type it
/// is used as for any universe size refuses at S3, not at admission: a
/// `count` into a Boolean field.
#[trace("TC-544", "FR-139-AC-5")]
#[test]
fn a_count_used_as_a_boolean_refuses_whatever_the_universe_size() {
    let (code, cause, at) = only_refusal("busy = count(s in R::slots where s.value = 1);");
    assert_eq!((code, cause), (Code::IllTyped, Some("type-mismatch")));
    assert_eq!(at, "count(s in R::slots where s.value = 1)");
}

/// FR-139 behavior: a bound that depends on a population's size through a
/// form other than the row's whole value is refused as not yet checked,
/// rather than admitted unchecked.
#[trace("TC-544")]
#[test]
fn a_size_dependent_bound_inside_a_larger_expression_is_refused_as_unbuilt() {
    let (code, cause, _) = only_refusal("used = count(s in R::slots where s.value = 1) + 0;");
    assert_eq!(
        (code, cause),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}

/// FR-139 (admission): a population whose universe is smaller than the
/// abstract field's maximum is admitted: `Set<Reference<T>>[0, n]` fits
/// `Set<Reference<T>>[0, 2]` for every `n` up to 2.
#[trace("TC-544", "FR-139-AC-1")]
#[test]
fn a_universe_below_the_used_maximum_is_admitted() {
    let graph = checked(&ring_is_queue(&all_rows())).graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    for size in [0, 1, 2] {
        assert_eq!(
            refinement.admit_universes(&BTreeMap::from([(slots(), size)])),
            Ok(()),
            "{size} slots"
        );
    }
    assert!(refinement
        .admit_universes(&BTreeMap::from([(slots(), 3)]))
        .is_err());
}

/// FR-139 (`sum`): a `sum` types as an integer, records its size-dependent
/// bound and is checked at admission against the type of its use. The bound
/// is `[min(0, n*lo), max(0, n*hi)]` (the open FR-139-AC-6, pending the spec
/// change).
#[trace("TC-544")]
#[test]
fn a_sum_records_its_bound_and_is_checked_at_admission() {
    let graph = checked(&ring_is_queue(&all_rows())).graph;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let total = queue_row(refinement, 7);
    assert_eq!(total.body().value_type(), &ValueType::Integer);
    let [used] = total.population_uses() else {
        panic!("the sum row records its bound");
    };
    assert_eq!(used.population(), "R::slots");
    let PopulationValue::Sum { summand } = used.value() else {
        panic!("a sum: {:?}", used.value());
    };
    assert_eq!(
        summand,
        &IntegerInterval::new(0_i64.into(), 1_i64.into()).expect("an interval")
    );
    assert_eq!(used.admitted_type(2), Some(interval(0, 2)));
    let Err(found) = refinement.admit_universes(&BTreeMap::from([(slots(), 3)])) else {
        panic!("three slots overflow Int[0, 2]");
    };
    assert!(found.iter().any(|refusal| matches!(
        &refusal.cause,
        RefinementCause::PopulationBound { row, size: 3, .. } if row == "Q::Queue.total"
    )));
}

/// FR-139: a population name or `seq` in a state clause refuses
/// `unsupported_construct`/`expression-form` at its span.
#[trace("TC-544", "FR-139-AC-2")]
#[test]
fn a_population_name_and_a_seq_in_an_invariant_refuse_as_expression_forms() {
    for (body, at) in [
        ("size(R::slots) = 2", "R::slots"),
        ("size(seq(i in 0 .. 1: i)) = 2", "seq(i in 0 .. 1: i)"),
    ] {
        let found = refused(&format!(
            "invariant Total using v on R::Ring at current {{ {body} }}"
        ));
        let [refusal] = &found[..] else {
            panic!("one refusal for {body}");
        };
        assert_eq!(
            (refusal.refusal.cause.code(), refusal.refusal.cause.cause()),
            (Code::UnsupportedConstruct, Some("expression-form")),
            "{body}"
        );
        assert_eq!(refusal.at, at);
    }
}
