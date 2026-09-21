// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-114: supplied definition/rule closure through the real source namespace.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_spec_language::linking::composed::binding_work::{
    Dimension, Limits as BindingLimits, Work, ACCOUNTING_VERSION,
};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::linking::composed::definitions::{
    self, Artifact, Cause, Inventory, RuleInput, Selection,
};
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, SourceInventory, WorkLimits,
};
use quire_spec_language::{ByteDigest, Limits, Source, SourceIdentity, Span};

/// Synthetic, deliberately-not-the-real-standard-text bytes for a registered
/// definition's supplied artifact. QSL recognizes a definition by identity and
/// revision, never by comparing its bytes to any particular snapshot
/// (PLAT-887), so any distinct, self-consistent content proves the point
/// better than real standard text would.
fn definition_bytes(definition: R) -> &'static [u8] {
    definition.identity().as_bytes()
}

fn definition_selection(definition: R) -> Selection {
    Selection {
        identity: definition.identity().into(),
        revision: definition.revision().into(),
        digest: ByteDigest::of(definition_bytes(definition)),
    }
}

fn definitions() -> Vec<Artifact<'static>> {
    R::all()
        .iter()
        .map(|&definition| Artifact {
            selection: definition_selection(definition),
            bytes: definition_bytes(definition),
        })
        .collect()
}

fn rules() -> Vec<RuleInput<'static>> {
    R::all()
        .iter()
        .flat_map(|definition| definition.rules())
        .map(|rule| {
            let bytes = rule.path.as_bytes();
            (
                rule.path,
                RuleInput {
                    path: rule.path,
                    digest: ByteDigest::of(bytes),
                    bytes,
                },
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect()
}

fn profile(alias: &str, definition: R) -> String {
    let selected = definition_selection(definition);
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
        edition: definition_selection(R::Edition),
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
        edition: definition_selection(R::Edition),
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
        matches!(&report.declarations[0].uses[0].refusal, Some(Cause::MissingDependency(dependency)) if *dependency == R::StateCore)
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn stale_raw_bytes_refuse_but_a_resealed_identity_match_still_resolves() {
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
    let mut altered = definition_bytes(R::StateQueries).to_vec();
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
            edition: definition_selection(R::Edition),
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
    // PLAT-887: QSL is a graph of specs and resolves definitions by reference.
    // A source that cites the resealed digest for the registered
    // identity/revision, matched by a self-consistent supplied artifact with
    // the SAME resealed digest, still resolves to StateQueries: recognition
    // never compares the supplied bytes to any particular frozen snapshot.
    // Before PLAT-887 this refused with `UnsupportedDefinition` because the
    // altered bytes did not equal the vendored copy's bytes.
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
        edition: definition_selection(R::Edition),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(report.edition_refusal.is_none());
    let resolved = &report.declarations[0].uses[0];
    assert_eq!(resolved.refusal, None);
    assert!(resolved.closure.contains(&R::StateQueries));
}

/// PLAT-887: `StateCore` is `StateQueries`' inherited registry dependency,
/// never an author-declared [`Selection`] the source cites directly. QSL
/// resolves an inherited dependency by identity and revision alone; a
/// supplied `StateCore` artifact with different, but self-consistent, bytes
/// than any other invocation's copy still resolves the closure. Before
/// PLAT-887, `Catalog::new` additionally required a supplied artifact's bytes
/// to equal the registry's frozen vendored snapshot, so this exact scenario
/// refused with `Cause::UnsupportedDefinition`.
#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn an_inherited_dependency_resolves_by_identity_despite_different_supplied_bytes() {
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
    let index = definitions
        .iter()
        .position(|entry| entry.selection.identity == R::StateCore.identity())
        .unwrap();
    let different = b"a different, self-consistent StateCore artifact, not any snapshot".to_vec();
    definitions[index].bytes = &different;
    definitions[index].selection.digest = ByteDigest::of(&different);
    let rules = rules();
    let selected = Inventory {
        edition: definition_selection(R::Edition),
        definitions: &definitions,
        rules: &rules,
    };
    let report = definitions::resolve(
        namespace.namespace().unwrap(),
        &selected,
        &mut Work::new(BindingLimits::default()),
    );
    assert!(report.edition_refusal.is_none());
    let resolved = &report.declarations[0].uses[0];
    assert_eq!(resolved.refusal, None);
    assert!(resolved.closure.contains(&R::StateCore));
}

/// PLAT-887: a rule's digest is a self-consistency check only -- does the
/// caller's own claimed digest match the caller's own supplied bytes -- never
/// a comparison to a frozen snapshot. A missing rule still refuses with
/// `MissingRule`; re-encoded-but-self-consistent bytes (a claimed digest that
/// matches them) no longer refuse at all, since recognition is by path
/// identity, not content. `RuleMismatch` fires only when the claimed digest
/// disagrees with the caller's own supplied bytes.
#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn missing_or_self_inconsistent_selected_rules_refuse_only_their_closures() {
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
    let mut changed = target.path.as_bytes().to_vec();
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
            // Self-inconsistent: the bytes change but the claimed digest does
            // not, so it no longer matches the caller's own supplied bytes.
            rule.bytes = &changed;
        }
        let selected = Inventory {
            edition: definition_selection(R::Edition),
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
        selection: definition_selection(R::Edition),
        bytes: definition_bytes(R::Edition),
    });
    let rules = rules();
    let selected = Inventory {
        edition: definition_selection(R::Edition),
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
        edition: definition_selection(R::StateCore),
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
        Some(Cause::WrongEdition(definition_selection(R::StateCore)))
    );
    assert!(report.edition.is_empty());
    assert_eq!(report.inventory.edition, definition_selection(R::StateCore));
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
        edition: definition_selection(R::Edition),
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
        edition: definition_selection(R::Edition),
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
            edition: definition_selection(R::Edition),
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
        .map(|&definition| Artifact {
            selection: definition_selection(definition),
            bytes: definition_bytes(definition),
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
        edition: definition_selection(R::Edition),
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
