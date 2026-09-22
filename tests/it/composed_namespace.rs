// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-114: exact source inventory and bounded namespace admission.
//! Unresolved profile/model strings are syntax inputs, not model-intake evidence.

use ix_trace_rs::trace;
use quire_spec_language::linking::composed::{
    admit_namespace, ConflictKind, DeclarationDisposition, DeclarationRefusal, Dimension,
    Exhaustion, ExpectedSource, InventoryIssue, NamespaceReport, SourceInventory, Usage, Work,
    WorkLimits, ACCOUNTING_VERSION, DEFAULT_LIMITS, HARD_LIMITS,
};
use quire_spec_language::syntax::composed::NativeUnit;
use quire_spec_language::{ByteDigest, Code, Limits, Source, SourceIdentity, Span};

const LANGUAGE: &str = "ix:native";
const EDITION: &str = "1-draft";
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile S = \"test:unresolved-profile\" version \"selected\" digest \"unresolved\";\n";

fn source(identity: &str, revision: &str, path: &str, text: &str) -> Source {
    Source::read(
        SourceIdentity {
            identity: identity.into(),
            revision: revision.into(),
        },
        path,
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .unwrap()
}

fn predicates(names: &[&str]) -> String {
    let mut text = HEADER.to_owned();
    for name in names {
        text.push_str(&format!(
            "predicate {name} using S (): Boolean {{ true }}\n"
        ));
    }
    text
}

fn inventory(sources: &[(&str, &Source)]) -> SourceInventory {
    SourceInventory {
        language: LANGUAGE.into(),
        edition: EDITION.into(),
        units: sources
            .iter()
            .map(|(authority, source)| ExpectedSource {
                authority: (*authority).into(),
                identity: source.identity().clone(),
                digest: source.digest(),
            })
            .collect(),
    }
}

fn admit<'a>(selected: &'a SourceInventory, supplied: &'a [Source]) -> NamespaceReport<'a> {
    admit_namespace(selected, supplied, WorkLimits::default(), Limits::default())
}

fn token_span(text: &str, token: &str) -> Span {
    let start = text.find(token).unwrap();
    Span {
        start,
        end: start + token.len(),
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-7")]
fn exact_input_accounting_admits_without_discovering_external_sources() {
    const ID: &str = "test:source-α";
    const REVISION: &str = "selected:β";
    const AUTHORITY: &str = "editable:γ";
    const PATH: &str = "/not-supplied-by-filesystem/δ.native";
    let text = predicates(&["First", "Second"]);
    let supplied = source(ID, REVISION, PATH, &text);
    let selected = inventory(&[(AUTHORITY, &supplied)]);
    // Each literal occurrence below is a distinct charged metadata component;
    // lengths count UTF-8 bytes, including non-ASCII identity and path text.
    let bytes = [
        LANGUAGE, EDITION, AUTHORITY, ID, REVISION, ID, REVISION, PATH, &text,
    ]
    .iter()
    .map(|part| part.len())
    .sum();
    let limits = WorkLimits {
        source_bytes: bytes,
        units: 2, // One expected entry plus one supplied entry.
        declarations: 2,
        references: 2, // One Boolean syntax node per predicate body.
        dependency_edges: 0,
    };
    let supplied = [supplied];
    let report = admit_namespace(&selected, &supplied, limits, Limits::default());
    let namespace = report.namespace().unwrap();
    assert_eq!(namespace.units().len(), 1);
    assert_eq!(namespace.declarations().len(), 2);
    assert_eq!(namespace.units()[0].source().text(), text);
    assert_eq!(namespace.units()[0].source().path(), PATH);
    assert_eq!(namespace.lookup("First").len(), 1);
    assert_eq!(namespace.lookup("Second").len(), 1);
    assert!(report.issues().is_empty());
    assert_eq!(report.exhaustion(), None);
    assert_eq!(report.limits(), limits);
    assert_eq!(
        report.usage(),
        Usage {
            source_bytes: bytes,
            units: 2,
            declarations: 2,
            references: 2,
            dependency_edges: 0,
        }
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-2")]
fn reordered_sources_match_exact_selections_without_using_paths_as_identity() {
    let first = source("test:first", "r1", "same.native", &predicates(&["First"]));
    let second = source("test:second", "r2", "same.native", &predicates(&["Second"]));
    let selected = inventory(&[("editable:first", &first), ("editable:second", &second)]);
    let supplied = [second, first];
    let report = admit(&selected, &supplied);
    assert!(report.issues().is_empty());
    assert_eq!(report.exhaustion(), None);
    assert_eq!(report.inventory(), &selected);
    assert_eq!(report.supplied().len(), 2);
    let namespace = report.namespace().unwrap();
    assert_eq!(namespace.units().len(), 2);
    assert!(namespace.conflicts().is_empty());
    for (name, expected, offered) in [("First", 0, 1), ("Second", 1, 0)] {
        assert_eq!(namespace.lookup(name).len(), 1);
        let id = namespace.lookup(name)[0];
        let entry = namespace.declaration(id).unwrap();
        let unit = namespace.unit(entry.unit()).unwrap();
        assert_eq!(namespace.expected_index(entry.unit()), Some(expected));
        assert_eq!(namespace.supplied_index(entry.unit()), Some(offered));
        assert_eq!(unit.source().identity(), &selected.units[expected].identity);
        assert_eq!(unit.source().digest(), selected.units[expected].digest);
        assert_eq!(unit.source().text(), supplied[offered].text());
        assert_eq!(
            namespace.disposition(id),
            Some(DeclarationDisposition::Available)
        );
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn missing_and_unexpected_sources_refuse_the_closed_namespace() {
    let first = source("test:first", "r1", "a.native", &predicates(&["First"]));
    let second = source("test:second", "r1", "b.native", &predicates(&["Second"]));
    let selected_both = inventory(&[("a", &first), ("b", &second)]);
    let selected_first = inventory(&[("a", &first)]);
    let supplied_first = [first.clone()];
    let missing = admit(&selected_both, &supplied_first);
    assert!(missing.namespace().is_none());
    assert!(matches!(
        missing.issues(),
        [InventoryIssue::MissingSource { expected: 1 }]
    ));
    assert!(!missing.is_incomplete());
    let supplied_both = [first, second];
    let unexpected = admit(&selected_first, &supplied_both);
    assert!(unexpected.namespace().is_none());
    assert!(matches!(
        unexpected.issues(),
        [InventoryIssue::UnexpectedSource { supplied: 1 }]
    ));
    assert!(!unexpected.is_incomplete());
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn identity_revision_and_digest_substitutions_refuse_independently() {
    let text = predicates(&["Selected"]);
    let original = source("test:selected", "r1", "selected.native", &text);
    let selected = inventory(&[("selected", &original)]);
    for replacement in [
        source("test:foreign", "r1", "selected.native", &text),
        source("test:selected", "r2", "selected.native", &text),
    ] {
        let supplied = [replacement];
        let report = admit(&selected, &supplied);
        assert!(report.namespace().is_none());
        assert_eq!(report.issues().len(), 2);
        assert!(report
            .issues()
            .iter()
            .any(|issue| matches!(issue, InventoryIssue::MissingSource { expected: 0 })));
        assert!(report
            .issues()
            .iter()
            .any(|issue| matches!(issue, InventoryIssue::UnexpectedSource { supplied: 0 })));
    }
    let supplied = [original];
    let mut wrong_digest = selected.clone();
    wrong_digest.units[0].digest = ByteDigest::of(b"different exact source bytes");
    let report = admit(&wrong_digest, &supplied);
    assert!(report.namespace().is_none());
    assert!(matches!(
        report.issues(),
        [InventoryIssue::DigestMismatch {
            expected: 0,
            supplied: 0
        }]
    ));
    assert!(admit(&selected, &supplied).namespace().is_some());
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn duplicate_expected_and_offered_source_selections_retain_all_candidates() {
    let original = source("test:unit", "r1", "one.native", &predicates(&["Only"]));
    let selected = inventory(&[("one", &original)]);
    let mut repeated = selected.clone();
    repeated.units.push(repeated.units[0].clone());
    let supplied = [original.clone()];
    let report = admit(&repeated, &supplied);
    assert!(report.namespace().is_none());
    let expected = report
        .issues()
        .iter()
        .find_map(|issue| match issue {
            InventoryIssue::DuplicateExpectedSource { expected } => Some(expected),
            _ => None,
        })
        .unwrap();
    assert_eq!(expected, &[0, 1]);
    assert_eq!(report.usage().units, 3);

    // A different display path cannot make the same exact source unique.
    let another_path = source("test:unit", "r1", "two.native", original.text());
    let supplied = [original, another_path];
    let report = admit(&selected, &supplied);
    assert!(report.namespace().is_none());
    let offered = report
        .issues()
        .iter()
        .find_map(|issue| match issue {
            InventoryIssue::DuplicateSuppliedSource { supplied } => Some(supplied),
            _ => None,
        })
        .unwrap();
    assert_eq!(offered, &[0, 1]);
    assert_eq!(report.usage().units, 3);
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn empty_inventory_and_blank_editable_authority_do_not_establish_a_namespace() {
    let empty = inventory(&[]);
    let report = admit(&empty, &[]);
    assert!(report.namespace().is_none());
    assert!(matches!(report.issues(), [InventoryIssue::EmptyInventory]));
    let unit = source("test:one", "r1", "one.native", &predicates(&["One"]));
    let selected = inventory(&[("  ", &unit)]);
    let supplied = [unit];
    let report = admit(&selected, &supplied);
    assert!(report.namespace().is_none());
    assert!(matches!(
        report.issues(),
        [InventoryIssue::InvalidExpectedSource { expected: 0 }]
    ));
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn malformed_source_retains_its_diagnostic_and_the_other_successful_parse() {
    let bad_text = format!("{HEADER}predicate Broken using S (): Boolean {{ }}");
    let bad = source("test:bad", "r1", "bad.native", &bad_text);
    let good = source("test:good", "r1", "good.native", &predicates(&["Good"]));
    let selected = inventory(&[("bad", &bad), ("good", &good)]);
    let supplied = [bad, good];
    let report = admit(&selected, &supplied);
    assert!(report.namespace().is_none());
    assert_eq!(report.issues().len(), 1);
    let InventoryIssue::ParseFailure {
        supplied: index,
        diagnostic,
    } = &report.issues()[0]
    else {
        panic!("malformed native input must retain its parser diagnostic");
    };
    assert_eq!(*index, 0);
    assert_eq!(diagnostic.code, Code::InvalidSyntax);
    assert_eq!(diagnostic.source, supplied[0].identity().clone());
    assert_eq!(diagnostic.path, "bad.native");
    assert_eq!(
        &bad_text[diagnostic.span.start.byte..diagnostic.span.end.byte],
        "}"
    );
    assert_eq!(report.parsed_sources().len(), 1);
    assert_eq!(report.parsed_sources()[0].supplied, 1);
    let NativeUnit::Composed(parsed) = &report.parsed_sources()[0].unit else {
        panic!("the independent composed parse must survive");
    };
    assert_eq!(parsed.source().text(), supplied[1].text());
    assert_eq!(parsed.declarations()[0].name.value, "Good");
    assert!(!report.is_incomplete());
}

#[test]
#[trace("TC-114", "FR-036-AC-1")]
fn mixed_historical_and_composed_headers_refuse_with_original_token_spans() {
    let historical = "language \"ix:native\" edition \"0-draft\"; profile \"state-finite/0-draft\"; model M = \"test:model\" version \"1\" digest \"unresolved\"; invariant Old on M::Thing at current { true }";
    let old = source("test:old", "r1", "old.native", historical);
    let new = source("test:new", "r1", "new.native", &predicates(&["New"]));
    let selected = inventory(&[("old", &old), ("new", &new)]);
    let supplied = [old, new];
    let report = admit(&selected, &supplied);
    assert!(report.namespace().is_none());
    assert_eq!(report.issues().len(), 1);
    let InventoryIssue::HeaderConflict {
        supplied: index,
        language,
        edition,
    } = &report.issues()[0]
    else {
        panic!("an available historical edition must parse before header comparison");
    };
    assert_eq!(*index, 0);
    assert_eq!(language.value, LANGUAGE);
    assert_eq!(language.span, token_span(historical, "\"ix:native\""));
    assert_eq!(edition.value, "0-draft");
    assert_eq!(edition.span, token_span(historical, "\"0-draft\""));
    assert_eq!(report.inventory().edition, EDITION);
    assert_eq!(report.parsed_sources().len(), 2);
    assert!(matches!(
        report.parsed_sources()[0].unit,
        NativeUnit::Historical(_)
    ));
    assert!(matches!(
        report.parsed_sources()[1].unit,
        NativeUnit::Composed(_)
    ));
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-2")]
fn inventory_language_or_edition_conflict_retains_every_matching_header() {
    let first = source("test:first", "r1", "a.native", &predicates(&["First"]));
    let second = source("test:second", "r1", "b.native", &predicates(&["Second"]));
    let matching = inventory(&[("a", &first), ("b", &second)]);
    let supplied = [first, second];
    assert!(admit(&matching, &supplied).namespace().is_some());
    let mut wrong_language = matching.clone();
    wrong_language.language = "test:other-language".into();
    let mut wrong_edition = matching.clone();
    wrong_edition.edition = "0-draft".into();
    for selected in [wrong_language, wrong_edition] {
        let report = admit(&selected, &supplied);
        assert!(report.namespace().is_none());
        assert_eq!(report.inventory(), &selected);
        assert_eq!(report.issues().len(), 3);
        assert!(report
            .issues()
            .iter()
            .any(|issue| matches!(issue, InventoryIssue::UnsupportedSelection)));
        let conflicts: Vec<_> = report
            .issues()
            .iter()
            .filter_map(|issue| match issue {
                InventoryIssue::HeaderConflict {
                    supplied,
                    language,
                    edition,
                } => Some((*supplied, language, edition)),
                _ => None,
            })
            .collect();
        assert_eq!(conflicts.len(), 2);
        for (expected, (offered, language, edition)) in conflicts.iter().enumerate() {
            assert_eq!(*offered, expected);
            assert_eq!(language.value, LANGUAGE);
            assert_eq!(edition.value, EDITION);
            assert_eq!(
                language.span,
                token_span(supplied[expected].text(), "\"ix:native\"")
            );
            assert_eq!(
                edition.span,
                token_span(supplied[expected].text(), "\"1-draft\"")
            );
        }
        assert_eq!(report.parsed_sources().len(), 2);
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn duplicate_native_names_retain_every_locus_and_unrelated_declarations() {
    let first = source(
        "test:first",
        "r1",
        "a.native",
        &predicates(&["Shared", "Independent"]),
    );
    let second = source("test:second", "r1", "b.native", &predicates(&["Shared"]));
    let selected = inventory(&[("a", &first), ("b", &second)]);
    let supplied = [first, second];
    let report = admit(&selected, &supplied);
    assert!(report.issues().is_empty());
    let namespace = report.namespace().unwrap();
    assert_eq!(namespace.declarations().len(), 3);
    assert_eq!(namespace.conflicts().len(), 1);
    let conflict = &namespace.conflicts()[0];
    assert_eq!(conflict.kind, ConflictKind::NativeName);
    assert_eq!(namespace.lookup("Shared").len(), 2);
    assert_eq!(conflict.declarations, namespace.lookup("Shared"));
    for (offered, id) in conflict.declarations.iter().enumerate() {
        let entry = namespace.declaration(*id).unwrap();
        assert_eq!(namespace.supplied_index(entry.unit()), Some(offered));
        assert_eq!(
            namespace.syntax(*id).unwrap().name.span,
            token_span(supplied[offered].text(), "Shared")
        );
        assert!(matches!(
            entry.refusals(),
            [DeclarationRefusal::DuplicateName { group: 0 }]
        ));
        assert_eq!(
            namespace.disposition(*id),
            Some(DeclarationDisposition::Refused)
        );
    }
    assert_eq!(namespace.lookup("Independent").len(), 1);
    assert_eq!(
        namespace.disposition(namespace.lookup("Independent")[0]),
        Some(DeclarationDisposition::Available)
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn duplicate_editable_authority_refuses_its_sources_without_erasing_the_third() {
    let first = source("test:first", "r1", "a.native", &predicates(&["First"]));
    let second = source("test:second", "r1", "b.native", &predicates(&["Second"]));
    let third = source(
        "test:third",
        "r1",
        "c.native",
        &predicates(&["Independent"]),
    );
    let selected = inventory(&[
        ("same-authority", &first),
        ("same-authority", &second),
        ("other", &third),
    ]);
    let supplied = [first, second, third];
    let report = admit(&selected, &supplied);
    assert!(report.issues().is_empty());
    let namespace = report.namespace().unwrap();
    assert_eq!(namespace.units().len(), 3);
    assert_eq!(namespace.conflicts().len(), 1);
    let conflict = &namespace.conflicts()[0];
    assert_eq!(conflict.kind, ConflictKind::SourceAuthority);
    assert_eq!(conflict.expected, [0, 1]);
    assert_eq!(conflict.declarations.len(), 2);
    for (offered, name) in ["First", "Second"].into_iter().enumerate() {
        assert_eq!(namespace.lookup(name).len(), 1);
        let id = namespace.lookup(name)[0];
        assert!(conflict.declarations.contains(&id));
        let entry = namespace.declaration(id).unwrap();
        assert_eq!(namespace.supplied_index(entry.unit()), Some(offered));
        assert_eq!(
            namespace.syntax(id).unwrap().name.span,
            token_span(supplied[offered].text(), name)
        );
        assert!(matches!(
            entry.refusals(),
            [DeclarationRefusal::DuplicateAuthority { group: 0 }]
        ));
        assert_eq!(
            namespace.disposition(id),
            Some(DeclarationDisposition::Refused)
        );
    }
    assert_eq!(namespace.lookup("Independent").len(), 1);
    assert_eq!(
        namespace.disposition(namespace.lookup("Independent")[0]),
        Some(DeclarationDisposition::Available)
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-7")]
fn byte_unit_and_declaration_boundaries_refuse_before_work_and_retry_immutably() {
    const ID: &str = "test:budget";
    const REVISION: &str = "r1";
    const AUTHORITY: &str = "editable";
    const PATH: &str = "budget.native";
    let text = predicates(&["First", "Second"]);
    let unit = source(ID, REVISION, PATH, &text);
    let selected = inventory(&[(AUTHORITY, &unit)]);
    let original_inventory = selected.clone();
    let original_digest = ByteDigest::of(text.as_bytes());
    let supplied = [unit];
    let header_bytes = LANGUAGE.len() + EDITION.len();
    let expected_bytes = header_bytes + AUTHORITY.len() + ID.len() + REVISION.len();
    let metadata_bytes = expected_bytes + ID.len() + REVISION.len() + PATH.len();
    let all_bytes = metadata_bytes + text.len();
    let exact = WorkLimits {
        source_bytes: all_bytes,
        units: 2,
        declarations: 2,
        references: 2,
        dependency_edges: 0,
    };
    // Byte charges occur per metadata/content string; entry and declaration
    // charges are one each. These vectors come from the supplied literals.
    let controls = [
        (
            Dimension::SourceBytes,
            0,
            0,
            LANGUAGE.len(),
            Usage::default(),
        ),
        (
            Dimension::SourceBytes,
            all_bytes - 1,
            metadata_bytes,
            text.len(),
            Usage {
                source_bytes: metadata_bytes,
                units: 2,
                ..Usage::default()
            },
        ),
        (
            Dimension::Units,
            0,
            0,
            1,
            Usage {
                source_bytes: header_bytes,
                ..Usage::default()
            },
        ),
        (
            Dimension::Units,
            1,
            1,
            1,
            Usage {
                source_bytes: expected_bytes,
                units: 1,
                ..Usage::default()
            },
        ),
        (
            Dimension::Declarations,
            0,
            0,
            1,
            Usage {
                source_bytes: all_bytes,
                units: 2,
                ..Usage::default()
            },
        ),
        (
            Dimension::Declarations,
            1,
            1,
            1,
            Usage {
                source_bytes: all_bytes,
                units: 2,
                declarations: 1,
                ..Usage::default()
            },
        ),
    ];
    for (dimension, limit, used, requested, expected_usage) in controls {
        let mut limited = exact;
        match dimension {
            Dimension::SourceBytes => limited.source_bytes = limit,
            Dimension::Units => limited.units = limit,
            Dimension::Declarations => limited.declarations = limit,
            _ => panic!("only namespace intake dimensions are controlled here"),
        }
        let failed = admit_namespace(&selected, &supplied, limited, Limits::default());
        let expected_exhaustion = Exhaustion {
            dimension,
            used,
            requested,
            limit,
        };
        assert_eq!(failed.exhaustion(), Some(&expected_exhaustion));
        assert_eq!(failed.usage(), expected_usage);
        assert!(failed.is_incomplete());
        assert!(failed.namespace().is_none());
        assert!(failed.issues().is_empty());

        let retried = admit_namespace(&selected, &supplied, exact, Limits::default());
        assert_eq!(retried.exhaustion(), None);
        assert!(retried.issues().is_empty());
        assert_eq!(retried.namespace().unwrap().declarations().len(), 2);
        assert_eq!(
            retried.usage(),
            Usage {
                source_bytes: all_bytes,
                units: 2,
                declarations: 2,
                references: 2,
                dependency_edges: 0,
            }
        );
        assert_eq!(failed.exhaustion(), Some(&expected_exhaustion));
        assert_eq!(failed.usage(), expected_usage);
        assert_eq!(selected, original_inventory);
        assert_eq!(supplied[0].digest(), original_digest);
        assert_eq!(supplied[0].text(), text);
        assert_eq!(supplied[0].identity().identity, ID);
        assert_eq!(supplied[0].identity().revision, REVISION);
        assert_eq!(supplied[0].path(), PATH);
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-7")]
fn parser_budget_exhaustion_keeps_its_distinct_source_bound_cause() {
    let unit = source("test:parser", "r1", "parser.native", &predicates(&["One"]));
    let selected = inventory(&[("parser", &unit)]);
    let supplied = [unit];
    let parser = Limits {
        tokens: 0,
        ..Limits::default()
    };
    let report = admit_namespace(&selected, &supplied, WorkLimits::default(), parser);
    assert!(report.namespace().is_none());
    assert!(report.is_incomplete());
    assert_eq!(report.exhaustion(), None);
    assert_eq!(report.parser_limits().tokens, 0);
    assert_eq!(report.issues().len(), 1);
    let InventoryIssue::ParseFailure {
        supplied: offered,
        diagnostic,
    } = &report.issues()[0]
    else {
        panic!("parser exhaustion must retain its actual parser cause");
    };
    assert_eq!(*offered, 0);
    assert_eq!(diagnostic.code, Code::ResourceExhausted);
    assert_eq!(&diagnostic.source, supplied[0].identity());
    assert_eq!(diagnostic.path, "parser.native");
    assert!(admit(&selected, &supplied).namespace().is_some());
}

#[test]
#[trace("TC-114", "FR-036-AC-7")]
fn accounting_caps_and_overflow_are_public_and_refuse_before_charging() {
    assert_eq!(ACCOUNTING_VERSION, "composed-namespace-work/1");
    let expected = WorkLimits {
        source_bytes: 16_777_216,
        units: 512,
        declarations: 8_192,
        references: 1_000_000,
        dependency_edges: 1_000_000,
    };
    assert_eq!(HARD_LIMITS, expected);
    assert_eq!(DEFAULT_LIMITS, expected);
    let mut work = Work::new(WorkLimits {
        source_bytes: usize::MAX,
        units: usize::MAX,
        declarations: usize::MAX,
        references: usize::MAX,
        dependency_edges: usize::MAX,
    });
    assert_eq!(work.limits(), expected);
    work.charge(Dimension::SourceBytes, 1).unwrap();
    let overflow = work.charge(Dimension::SourceBytes, usize::MAX).unwrap_err();
    assert_eq!(
        overflow,
        Exhaustion {
            dimension: Dimension::SourceBytes,
            used: 1,
            requested: usize::MAX,
            limit: 16_777_216,
        }
    );
    assert_eq!(overflow.code(), Code::ResourceExhausted);
    assert_eq!(
        work.usage(),
        Usage {
            source_bytes: 1,
            ..Usage::default()
        }
    );
    assert_eq!(Work::new(expected).usage(), Usage::default());
}
