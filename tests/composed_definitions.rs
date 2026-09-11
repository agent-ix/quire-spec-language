// SPDX-License-Identifier: AGPL-3.0-only
//! TC-114: supplied definition/rule closure through the real source namespace.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_spec_language::linking::composed::binding_work::{
    Dimension, Limits as BindingLimits, Work, ACCOUNTING_VERSION,
};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::linking::composed::definitions::{
    self, Artifact, Cause, Inventory, RuleInput,
};
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, SourceInventory, WorkLimits,
};
use quire_spec_language::{ByteDigest, Limits, Source, SourceIdentity, Span};

fn definitions() -> Vec<Artifact<'static>> {
    R::all()
        .iter()
        .map(|definition| Artifact {
            selection: definition.selection(),
            bytes: definition.bytes(),
        })
        .collect()
}

fn rules() -> Vec<RuleInput<'static>> {
    R::all()
        .iter()
        .flat_map(|definition| definition.rules())
        .map(|rule| {
            (
                rule.path,
                RuleInput {
                    path: rule.path,
                    digest: ByteDigest::of(rule.bytes),
                    bytes: rule.bytes,
                },
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect()
}

fn profile(alias: &str, definition: R) -> String {
    let selected = definition.selection();
    format!(
        "profile {alias} = \"{}\" version \"{}\" digest \"{}\";\n",
        selected.identity, selected.revision, selected.digest
    )
}

fn source(name: &str, body: &str) -> Source {
    Source::read(
        SourceIdentity {
            identity: name.into(),
            revision: "test:original".into(),
        },
        format!("{name}.native"),
        format!("language \"ix:native\" edition \"1-draft\";\n{body}").as_bytes(),
        Limits::default().source_bytes,
    )
    .unwrap()
}

fn inventory(sources: &[Source]) -> SourceInventory {
    SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: sources
            .iter()
            .map(|source| ExpectedSource {
                authority: source.identity().identity.clone(),
                identity: source.identity().clone(),
                digest: source.digest(),
            })
            .collect(),
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-1")]
fn exact_multi_unit_roots_keep_local_aliases_and_callee_profiles() {
    let sources = [
        source("state", &(profile("P", R::StateQueries) + "predicate Allowed using P (): Boolean { true }")),
        source("temporal", &(profile("P", R::EventPosition) + "temporal Due using P over (view: M::View) clock \"orders\" on origin { eventually[0,1] holds(Allowed()) }")),
    ];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(namespace.issues().is_empty(), "{:?}", namespace.issues());
    let definitions = definitions();
    let rules = rules();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(report.complete);
    assert!(report.edition_refusal.is_none());
    assert_eq!(report.declarations.len(), 2);
    let state = &report.declarations[0].uses[0];
    let temporal = &report.declarations[1].uses[0];
    assert!(state.refusal.is_none() && temporal.refusal.is_none());
    assert_ne!(state.unit, temporal.unit);
    assert_eq!(state.alias.value, temporal.alias.value);
    assert_eq!(state.closure[0], R::StateQueries);
    assert_eq!(temporal.closure[0], R::EventPosition);
    assert!(state.closure.contains(&R::StateCore));
    assert!(!state.closure.contains(&R::EventPosition));
    assert!(temporal.closure.contains(&R::TemporalFacet));
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn installed_definition_bytes_never_fill_an_omitted_dependency() {
    let sources = [source(
        "queries",
        &(profile("Q", R::StateQueries) + "predicate Check using Q (): Boolean { true }"),
    )];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let mut definitions = definitions();
    definitions.retain(|artifact| artifact.selection.identity != R::StateCore.identity());
    let rules = rules();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(report.complete && report.edition_refusal.is_none());
    assert!(
        matches!(&report.declarations[0].uses[0].refusal, Some(Cause::MissingDefinition(selection)) if selection == &R::StateCore.selection())
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn stale_raw_bytes_and_resealed_unknown_interpretations_both_refuse() {
    let sources = [source(
        "queries",
        &(profile("Q", R::StateQueries) + "predicate Check using Q (): Boolean { true }"),
    )];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let rules = rules();
    let mut altered = R::StateQueries.bytes().to_vec();
    altered.extend_from_slice(b"\nChanged meaning.\n");
    for resealed in [false, true] {
        let mut definitions = definitions();
        let index = definitions
            .iter()
            .position(|entry| entry.selection.identity == R::StateQueries.identity())
            .unwrap();
        definitions[index].bytes = &altered;
        if resealed {
            definitions[index].selection.digest = ByteDigest::of(&altered);
        }
        let selected = Inventory {
            edition: R::Edition.selection(),
            definitions: &definitions,
            rules: &rules,
        };
        let report = definitions::resolve(
            namespace.namespace().unwrap(),
            &selected,
            &mut Work::new(BindingLimits::default()),
        );
        let cause = report.declarations[0].uses[0].refusal.as_ref().unwrap();
        if resealed {
            assert!(matches!(cause, Cause::SelectionMismatch { .. }));
        } else {
            assert!(matches!(cause, Cause::DefinitionDigest { .. }));
        }
    }
    // Matching the source to the resealed bytes still cannot select new semantics.
    let changed = source("resealed", &format!("profile Q = \"{}\" version \"{}\" digest \"{}\"; predicate Check using Q (): Boolean {{ true }}", R::StateQueries.identity(), R::StateQueries.revision(), ByteDigest::of(&altered)));
    let changed = [changed];
    let selection = inventory(&changed);
    let namespace = admit_namespace(
        &selection,
        &changed,
        WorkLimits::default(),
        Limits::default(),
    );
    let mut definitions = definitions();
    let index = definitions
        .iter()
        .position(|entry| entry.selection.identity == R::StateQueries.identity())
        .unwrap();
    definitions[index].bytes = &altered;
    definitions[index].selection.digest = ByteDigest::of(&altered);
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(matches!(
        report.declarations[0].uses[0].refusal,
        Some(Cause::UnsupportedDefinition { .. })
    ));
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn missing_or_reencoded_selected_rules_refuse_only_their_closures() {
    let sources = [source(
        "queries",
        &(profile("Q", R::StateQueries) + "predicate Check using Q (): Boolean { true }"),
    )];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let definitions = definitions();
    let target = R::StateQueries.rules()[0];
    let mut changed = target.bytes.to_vec();
    changed.push(b'\n');
    for missing in [false, true] {
        let mut rules = rules();
        if missing {
            rules.retain(|rule| rule.path != target.path);
        } else {
            let rule = rules
                .iter_mut()
                .find(|rule| rule.path == target.path)
                .unwrap();
            rule.bytes = &changed;
            rule.digest = ByteDigest::of(&changed);
        }
        let selected = Inventory {
            edition: R::Edition.selection(),
            definitions: &definitions,
            rules: &rules,
        };
        let report = definitions::resolve(
            namespace.namespace().unwrap(),
            &selected,
            &mut Work::new(BindingLimits::default()),
        );
        assert!(report.edition_refusal.is_none());
        let cause = report.declarations[0].uses[0].refusal.as_ref().unwrap();
        if missing {
            assert!(matches!(cause, Cause::MissingRule { path } if *path == target.path));
        } else {
            assert!(matches!(cause, Cause::RuleMismatch { path, .. } if *path == target.path));
        }
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-3")]
fn duplicate_aliases_and_definition_entries_do_not_select_first_candidate() {
    let sources = [source(
        "duplicate",
        &(profile("Q", R::StateQueries)
            + &profile("Q", R::StateGraph)
            + "predicate Check using Q (): Boolean { true }"),
    )];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let mut definitions = definitions();
    definitions.push(Artifact {
        selection: R::Edition.selection(),
        bytes: R::Edition.bytes(),
    });
    let rules = rules();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(
        matches!(&report.edition_refusal, Some(Cause::AmbiguousDefinition { entries, .. }) if entries.len() == 2)
    );
    assert!(
        matches!(&report.declarations[0].uses[0].refusal, Some(Cause::AmbiguousAlias { imports }) if imports == &[0, 1])
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn known_state_definition_cannot_substitute_for_the_edition() {
    let sources = [source(
        "edition",
        &(profile("Q", R::StateQueries) + "predicate Check using Q (): Boolean { true }"),
    )];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let definitions = definitions();
    let rules = rules();
    let selected = Inventory {
        edition: R::StateCore.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(report.complete);
    assert_eq!(report.exhaustion, None);
    assert_eq!(
        report.edition_refusal,
        Some(Cause::WrongEdition(R::StateCore.selection()))
    );
    assert!(report.edition.is_empty());
    assert_eq!(report.inventory.edition, R::StateCore.selection());
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn duplicate_rule_entries_refuse_even_when_their_exact_bytes_agree() {
    let sources = [source(
        "rules",
        &(profile("Q", R::StateQueries)
            + &profile("S", R::StateCore)
            + "predicate Check using Q (): Boolean { true }\n\
               invariant Independent using S on M::View at current { true }"),
    )];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let definitions = definitions();
    let mut rules = rules();
    let path = "spec/functional/FR-033-admit-reusable-predicates.md";
    let first = rules.iter().position(|rule| rule.path == path).unwrap();
    let repeated = rules.len();
    rules.push(rules[first]);
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(report.complete);
    assert_eq!(report.exhaustion, None);
    assert_eq!(report.edition_refusal, None);
    assert_eq!(report.declarations.len(), 2);
    let dependent = &report.declarations[0].uses[0];
    assert_eq!(
        dependent.refusal,
        Some(Cause::AmbiguousRule {
            path,
            entries: vec![first, repeated],
        })
    );
    assert!(dependent.closure.is_empty());
    let independent = &report.declarations[1].uses[0];
    assert!(independent.complete);
    assert_eq!(independent.refusal, None);
    assert_eq!(independent.closure, [R::StateCore, R::Edition]);
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-3")]
fn absent_local_profile_alias_is_not_filled_from_another_unit() {
    let sources = [
        // A source must declare at least one profile. P keeps this unit valid
        // syntax while Q remains absent from its own profile namespace.
        source(
            "missing",
            &(profile("P", R::StateQueries) + "predicate Missing using Q (): Boolean { true }"),
        ),
        source(
            "supplied",
            &(profile("Q", R::StateQueries) + "predicate Healthy using Q (): Boolean { true }"),
        ),
    ];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(namespace.issues().is_empty(), "{:?}", namespace.issues());
    assert_eq!(namespace.exhaustion(), None);
    let definitions = definitions();
    let rules = rules();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(report.complete);
    assert_eq!(report.exhaustion, None);
    assert_eq!(report.edition_refusal, None);
    assert_eq!(report.declarations.len(), 2);
    let missing = &report.declarations[0].uses[0];
    assert!(missing.complete);
    assert_eq!(missing.refusal, Some(Cause::MissingAlias));
    assert!(missing.closure.is_empty());
    assert_eq!(missing.alias.value, "Q");
    let start = sources[0].text().find("using Q").unwrap() + "using ".len();
    assert_eq!(
        missing.alias.span,
        Span {
            start,
            end: start + 1
        }
    );
    assert_eq!(
        missing.unit,
        namespace.namespace().unwrap().declarations()[0].unit()
    );
    let healthy = &report.declarations[1].uses[0];
    assert_eq!(healthy.refusal, None);
    assert!(healthy.complete);
    assert_eq!(healthy.closure, [R::StateQueries, R::StateCore, R::Edition]);
    assert_ne!(missing.unit, healthy.unit);
}

#[test]
#[trace("TC-114", "FR-036-AC-1")]
fn profile_kind_does_not_follow_the_callers_preferred_spelling() {
    for definition in [
        R::StateCore,
        R::TemporalFacet,
        R::EventPosition,
        R::Protocol,
    ] {
        let sources = [source(
            "wrong",
            &(profile("Q", definition) + "predicate Check using Q (): Boolean { true }"),
        )];
        let selection = inventory(&sources);
        let namespace = admit_namespace(
            &selection,
            &sources,
            WorkLimits::default(),
            Limits::default(),
        );
        let definitions = definitions();
        let rules = rules();
        let selected = Inventory {
            edition: R::Edition.selection(),
            definitions: &definitions,
            rules: &rules,
        };
        let report = definitions::resolve(
            namespace.namespace().unwrap(),
            &selected,
            &mut Work::new(BindingLimits::default()),
        );
        assert!(matches!(
            report.declarations[0].uses[0].refusal,
            Some(Cause::WrongProfileKind { .. })
        ));
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-7")]
fn zero_and_exact_limits_keep_immutable_partial_reports_and_fresh_retries() {
    assert_eq!(ACCOUNTING_VERSION, "composed-binding-work/1");
    let sources = [source(
        "queries",
        &(profile("Q", R::StateQueries) + "predicate Check using Q (): Boolean { true }"),
    )];
    let selection = inventory(&sources);
    let namespace = admit_namespace(
        &selection,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let chain = [R::Edition, R::StateCore, R::StateQueries];
    let definitions: Vec<_> = chain
        .iter()
        .map(|definition| Artifact {
            selection: definition.selection(),
            bytes: definition.bytes(),
        })
        .collect();
    let rule_paths: Vec<_> = chain
        .iter()
        .flat_map(|definition| definition.rules().iter().map(|rule| rule.path))
        .collect();
    let rules: Vec<_> = rules()
        .into_iter()
        .filter(|rule| rule_paths.contains(&rule.path))
        .collect();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let mut work = Work::new(BindingLimits {
        definitions: 0,
        ..BindingLimits::default()
    });
    let unfinished = definitions::resolve(namespace.namespace().unwrap(), &selected, &mut work);
    assert!(!unfinished.complete);
    assert!(unfinished.declarations.is_empty());
    assert_eq!(
        unfinished.exhaustion.unwrap().dimension,
        Dimension::Definitions
    );
    assert_eq!(work.usage().definitions, 0);
    // Independent expected costs for Edition and Queries -> Core -> Edition.
    // Registry recognition probes: 1 + 2 + 3. Edition lookup + enter/leave: 3.
    // One alias candidate + root lookup: 2. Profile closure enter/leave: 6;
    // its two dependency lookups: 2. Every normative rule lookup costs one.
    let bytes = selected.edition.identity.len()
        + selected.edition.revision.len()
        + definitions
            .iter()
            .map(|artifact| {
                artifact.selection.identity.len()
                    + artifact.selection.revision.len()
                    + artifact.bytes.len()
            })
            .sum::<usize>()
        + rules
            .iter()
            .map(|rule| rule.path.len() + rule.bytes.len())
            .sum::<usize>();
    let exact = BindingLimits {
        bytes,
        definitions: 3,
        models: 0,
        bindings: rules.len() + 2,
        references: 19
            + 2 * R::Edition.rules().len()
            + R::StateCore.rules().len()
            + R::StateQueries.rules().len(),
        edges: 2,
    };
    let mut exact_work = Work::new(exact);
    let complete = definitions::resolve(namespace.namespace().unwrap(), &selected, &mut exact_work);
    assert!(complete.complete);
    let usage = exact_work.usage();
    assert_eq!(usage.bytes, exact.bytes);
    assert_eq!(usage.definitions, exact.definitions);
    assert_eq!(usage.bindings, exact.bindings);
    assert_eq!(usage.references, exact.references);
    assert_eq!(usage.edges, exact.edges);
    let one_short = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits { edges: 1, ..exact }),
    );
    assert!(!one_short.complete);
    assert!(!one_short.declarations.last().unwrap().complete);
    assert!(!unfinished.complete);
    assert_eq!(
        unfinished.exhaustion.unwrap().dimension,
        Dimension::Definitions
    );
    assert_eq!(selected.definitions.len(), 3);
}
