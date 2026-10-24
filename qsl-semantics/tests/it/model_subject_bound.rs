// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-336 (TC-846): a temporal clause's requirement record is computed from
//! checked types alone, so a clause over a population with no declared
//! maximum keeps that population's domain, over TC-458's `ConfigVersion`
//! fixture package (`crate::model_operations`).

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::{
    check_clauses, Capability, CheckedGraph, CheckedTemporalClause, TemporalTraceFamily,
};
use qsl_semantics::family::{ClaimExtent, FamilyContract};
use quire_exact::Identifier;
use quire_semantic_value::checking::CheckingLimits;

use crate::model_operations::{
    admit_and_assemble, admit_and_assemble_with_profiles, archive_population,
    attempt_update_modifies_version_number, config_version_document,
    config_version_document_with_population,
};

const INFINITE: &str = "quire.temporal.infinite-trace/v1";

/// ADR-018 §6's `ReachesTwo` under `fair weak each`, with `declarations`
/// (e.g. a `bound`) written before it.
fn reaches_two(declarations: &str) -> (Vec<u8>, CheckedTemporalClause) {
    let document = config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        serde_json::json!([]),
    );
    let clause = "temporal ReachesTwo using t over (c: Config::ConfigVersion) \
         clock \"model-steps\" on origin \
         { fair weak each Config::ConfigVersion::attemptUpdate; \
         always eventually holds(c.versionNumber = 2) }";
    let package = admit_and_assemble_with_profiles(
        &document,
        &format!("profile t = \"{INFINITE}\";\n"),
        &format!("{declarations}\n{clause}"),
    )
    .unwrap_or_else(|refusal| panic!("the unit assembles: {refusal:?}"));
    let mut checked = check_clauses(&package);
    match checked.pop() {
        Some(Ok(checked)) if checked.name() == "ReachesTwo" => (document, checked),
        other => panic!("one checked ReachesTwo, not {other:?}"),
    }
}

/// The unit's package checked with no temporal clause: the graph that keys
/// the `config_history` population.
fn graph(document: &[u8]) -> CheckedGraph {
    admit_and_assemble(document)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the unit checks")
}

fn identifier(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

/// FR-336-AC-1: `ReachesTwo` over `config_history`, declared with no
/// maximum, records (`temporal-satisfaction`, `Unbounded`) with the
/// population's domain beside the formula's. The record is a function of
/// the checked clause and its nodes alone: a request's universe is no
/// input to it.
#[trace("TC-846", "FR-336-AC-1")]
#[test]
fn an_unbounded_population_stays_in_the_requirement_record() {
    let (document, clause) = reaches_two("");
    let graph = graph(&document);
    let population = graph
        .population_domain(&identifier("Config"), &identifier("config_history"))
        .expect("keyed")
        .expect("config_history resolves");
    let [claim] = TemporalTraceFamily::requirements(&clause)
        .try_into()
        .expect("one claim");
    let [domain] = claim.populations() else {
        panic!("config_history has no maximum: {:?}", claim.populations());
    };
    assert_eq!(graph.population_key(*domain).as_ref(), Ok(&population));

    let (clause_node, over) = (
        WireNodeId::from_digest([1; 32]),
        WireNodeId::from_digest([2; 32]),
    );
    let record = claim
        .requirements(clause_node, &[over], std::slice::from_ref(&population), &[])
        .expect("keyed");
    assert_eq!(record.kind(), Capability::TemporalSatisfaction);
    let ClaimExtent::Unbounded(domains) = record.extent() else {
        panic!("unbounded: {:?}", record.extent());
    };
    assert_eq!(domains.kind(&population), Some(DomainKind::Population));
    assert_eq!(
        domains.kind(&DomainKey::Node {
            node: clause_node,
            path: Vec::new()
        }),
        Some(DomainKind::InfiniteTrace)
    );
    // A population domain is keyed by the caller; none is invented.
    assert!(claim.requirements(clause_node, &[over], &[], &[]).is_err());
}

/// FR-380: an authored `bound` makes the population finite, so the record
/// holds no population domain and needs no key for one.
#[trace("TC-846")]
#[test]
fn a_bounded_population_joins_no_domain() {
    let (_, clause) = reaches_two("bound Cap on Config::config_history [3]");
    let [claim] = TemporalTraceFamily::requirements(&clause)
        .try_into()
        .expect("one claim");
    assert!(claim.populations().is_empty());
    let record = claim
        .requirements(
            WireNodeId::from_digest([1; 32]),
            &[WireNodeId::from_digest([2; 32])],
            &[],
            &[],
        )
        .expect("keyed");
    let ClaimExtent::Unbounded(domains) = record.extent() else {
        panic!("the formula is infinite-trace: {:?}", record.extent());
    };
    assert_eq!(domains.len(), 1);
}

/// A second population covering the `over` type does not refuse the clause:
/// the checked clause ranges over both unbounded populations, and its
/// requirement record carries one population domain for each beside the
/// formula's (FR-125 Instancing admits several populations).
#[trace("TC-846")]
#[test]
fn two_populations_of_the_over_type_are_both_domains_of_the_clause() {
    let document = config_version_document_with_population(
        attempt_update_modifies_version_number(),
        archive_population(),
    );
    let clause = "temporal ReachesTwo using t over (c: Config::ConfigVersion) \
         clock \"model-steps\" on origin { always eventually holds(c.versionNumber = 2) }";
    let package = admit_and_assemble_with_profiles(
        &document,
        &format!("profile t = \"{INFINITE}\";\n"),
        clause,
    )
    .unwrap_or_else(|refusal| panic!("the unit assembles: {refusal:?}"));
    let checked = match check_clauses(&package).pop() {
        Some(Ok(checked)) => checked,
        other => panic!("the clause is admitted: {other:?}"),
    };
    let [claim] = TemporalTraceFamily::requirements(&checked)
        .try_into()
        .expect("one claim");
    let graph = package
        .check(CheckingLimits::default())
        .expect("the unit checks");
    let keys: Vec<DomainKey> = ["config_history", "archive"]
        .iter()
        .map(|name| {
            graph
                .population_domain(&identifier("Config"), &identifier(name))
                .expect("keyed")
                .expect("the population resolves")
        })
        .collect();
    assert_eq!(claim.populations().len(), 2);
    // The production record the checker keys for the clause names each
    // population as its own domain.
    let [production] = graph
        .requirements()
        .values()
        .filter(|record| record.requirements().kind() == Capability::TemporalSatisfaction)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_else(|records: Vec<_>| panic!("one clause record: {}", records.len()));
    let ClaimExtent::Unbounded(produced) = production.requirements().extent() else {
        panic!("unbounded: {:?}", production.requirements().extent());
    };
    for key in &keys {
        assert_eq!(produced.kind(key), Some(DomainKind::Population));
    }
    assert_ne!(keys[0], keys[1]);
    let record = claim
        .requirements(
            WireNodeId::from_digest([1; 32]),
            &[WireNodeId::from_digest([2; 32])],
            &keys,
            &[],
        )
        .expect("keyed");
    let ClaimExtent::Unbounded(domains) = record.extent() else {
        panic!("unbounded: {:?}", record.extent());
    };
    for key in &keys {
        assert_eq!(domains.kind(key), Some(DomainKind::Population));
    }
    assert!(claim
        .requirements(
            WireNodeId::from_digest([1; 32]),
            &[WireNodeId::from_digest([2; 32])],
            &keys[..1],
            &[],
        )
        .is_err());
}
