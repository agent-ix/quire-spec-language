// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 document analysis and canonical-format round-tripping.
use ix_trace_rs::trace;
use qsl_cst::{
    parse, CompleteCause, CompleteCode, CompleteDiagnostic, DefinitionDigest, DefinitionRef,
    HostCause, Limits, TokenClass,
};
use qsl_foundation::{SourceIdentity, Span};
use quire_spec_language::complete::{
    self, analyze_document, format_document, DocumentBinding, ProfileCatalog, SourceEdit,
};

fn ugly(profile: &DefinitionRef) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\"; profile Complete=\"{}\" version \"{}\" digest \"{}\"; record Reading{{datum:Decimal[-2,4;18,4;nearest-even];}} function Bits using Complete ():Float32[nearest-even] pure{{float32(bits:0x7fc00001)}}",
        profile.identity(),
        profile.version(),
        profile.digest().digest(),
    )
}

fn identity(revision: &str) -> SourceIdentity {
    SourceIdentity {
        identity: "test:editor".into(),
        revision: revision.into(),
    }
}

fn binding(revision: &str, profile: &DefinitionRef) -> DocumentBinding {
    DocumentBinding {
        identity: "test:editor".into(),
        revision: revision.into(),
        profile: profile.clone(),
    }
}

fn catalog() -> (ProfileCatalog, DefinitionRef) {
    let selected = DefinitionRef::new(
        "quire.value.complete/v1",
        "1",
        DefinitionDigest::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
    )
    .unwrap();
    (
        ProfileCatalog::new(vec![selected.clone()]).unwrap(),
        selected,
    )
}

#[trace("Task-047")]
#[test]
fn formatting_is_idempotent_and_preserves_semantic_tokens() {
    let (catalog, profile) = catalog();
    let source = ugly(&profile);
    let parsed = parse(
        identity("r1"),
        "editor.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let first = format_document(
        &parsed,
        binding("r1", &profile),
        &catalog,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(first.edits.len(), 1);
    let formatted = complete::apply_edits(
        &parsed,
        "r1",
        identity("r2"),
        &first.edits,
        Limits::default(),
    )
    .unwrap();
    assert!(formatted.is_admissible(), "{:?}", formatted.diagnostics());
    assert!(formatted.source().text().contains("nearest-even"));
    assert!(!formatted.source().text().contains("nearest - even"));
    assert!(formatted.source().text().contains("0x7fc00001"));
    assert!(!formatted.source().text().contains("0x 7"));
    let second = format_document(
        &formatted,
        binding("r2", &profile),
        &catalog,
        Limits::default(),
    )
    .unwrap();
    assert!(second.edits.is_empty());
}

#[trace("TC-184", "FR-134-AC-1")]
#[test]
fn lf_crlf_and_absent_final_newline_retain_format_correspondence() {
    let (catalog, profile) = catalog();
    let source = ugly(&profile);
    let ugly = parse(
        identity("seed"),
        "editor.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let seed = format_document(
        &ugly,
        binding("seed", &profile),
        &catalog,
        Limits::default(),
    )
    .unwrap();
    let canonical = seed.edits[0].replacement.clone();
    let variants = [
        canonical.clone(),
        canonical.replace('\n', "\r\n"),
        canonical.trim_end_matches('\n').to_string(),
    ];
    for (index, source) in variants.into_iter().enumerate() {
        let revision = format!("variant-{index}");
        let parsed = parse(
            identity(&revision),
            "editor.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap();
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let response = format_document(
            &parsed,
            binding(&revision, &profile),
            &catalog,
            Limits::default(),
        )
        .unwrap();
        let formatted = complete::apply_edits(
            &parsed,
            &revision,
            identity(&format!("{revision}-formatted")),
            &response.edits,
            Limits::default(),
        )
        .unwrap();
        for mapping in response.correspondence {
            assert_eq!(
                parsed.source().slice(mapping.original),
                formatted.source().slice(mapping.formatted)
            );
        }
    }
}

#[trace("Task-047")]
#[test]
fn incremental_and_full_reparse_publish_identical_language_outputs() {
    let (catalog, profile) = catalog();
    let source = ugly(&profile);
    let parsed = parse(
        identity("r1"),
        "editor.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let record = source.find("record Reading").unwrap();
    let incremental = complete::apply_edit(
        &parsed,
        "r1",
        identity("r2"),
        SourceEdit {
            range: Span {
                start: record - 1,
                end: record - 1,
            },
            replacement: " ".into(),
        },
        Limits::default(),
    )
    .unwrap();
    assert!(incremental.is_incremental_result());
    let full = parse(
        identity("r2"),
        "editor.native",
        incremental.source().text().as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        analyze_document(&incremental, binding("r2", &profile), false, &catalog).unwrap(),
        analyze_document(&full, binding("r2", &profile), false, &catalog).unwrap()
    );
    let snapshot = analyze_document(&full, binding("r2", &profile), false, &catalog).unwrap();
    assert!(!snapshot.navigation.is_empty());
    assert!(snapshot
        .completions
        .iter()
        .any(|item| item.spelling == "record"));
    assert!(snapshot
        .completions
        .iter()
        .any(|item| item.spelling == "Complete"));
}

#[trace("Task-047")]
#[test]
fn non_lexical_whitespace_falls_back_and_matches_full_reparse() {
    let (catalog, profile) = catalog();
    let source = ugly(&profile);
    let parsed = parse(
        identity("r1"),
        "editor.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let record = source.find("record Reading").unwrap();
    for (revision, replacement) in [("nbsp", "\u{00a0}"), ("bare-cr", "\r")] {
        let edited = complete::apply_edit(
            &parsed,
            "r1",
            identity(revision),
            SourceEdit {
                range: Span {
                    start: record - 1,
                    end: record - 1,
                },
                replacement: replacement.into(),
            },
            Limits::default(),
        )
        .unwrap();
        assert!(!edited.is_incremental_result());
        let full = parse(
            identity(revision),
            "editor.native",
            edited.source().text().as_bytes(),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(
            analyze_document(&edited, binding(revision, &profile), false, &catalog).unwrap(),
            analyze_document(&full, binding(revision, &profile), false, &catalog).unwrap()
        );
    }
}

#[trace("Task-047")]
#[test]
fn lowered_parse_limits_force_full_incremental_validation() {
    let (_, profile) = catalog();
    let source = ugly(&profile);
    let parsed = parse(
        identity("r1"),
        "editor.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let record = source.find("record Reading").unwrap();
    for (revision, limits) in [
        (
            "node-limit",
            Limits {
                nodes: 1,
                ..Limits::default()
            },
        ),
        (
            "token-limit",
            Limits {
                tokens: 1,
                ..Limits::default()
            },
        ),
    ] {
        let edit = SourceEdit {
            range: Span {
                start: record - 1,
                end: record - 1,
            },
            replacement: " ".into(),
        };
        let incremental =
            complete::apply_edit(&parsed, "r1", identity(revision), edit, limits).unwrap_err();
        let mut candidate = source.clone();
        candidate.insert(record - 1, ' ');
        let full = parse(
            identity(revision),
            "editor.native",
            candidate.as_bytes(),
            limits,
        )
        .unwrap_err();
        assert_eq!(incremental.code, full.code);
        assert_eq!(incremental.span, full.span);
    }
}

#[trace("Task-047")]
#[test]
fn stale_profile_and_cancelled_editor_requests_are_typed() {
    let (catalog, profile) = catalog();
    let source = ugly(&profile);
    let parsed = parse(
        identity("r1"),
        "editor.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let unbound =
        analyze_document(&parsed, binding("stale", &profile), false, &catalog).unwrap_err();
    assert_eq!(
        (unbound.code, unbound.cause),
        (
            CompleteCode::InvalidSourceIdentity,
            CompleteCause::Host(HostCause::RequestRevision)
        )
    );
    let mut wrong_profile = binding("r1", &profile);
    wrong_profile.profile = DefinitionRef::new(
        "unknown",
        "1",
        DefinitionDigest::parse(&format!("sha256:{}", "f".repeat(64))).unwrap(),
    )
    .unwrap();
    let unknown = format_document(&parsed, wrong_profile, &catalog, Limits::default()).unwrap_err();
    assert_eq!(
        (unknown.code, unknown.cause),
        (
            CompleteCode::UnknownProfile,
            CompleteCause::UnsupportedSelection
        )
    );
    let cancelled = analyze_document(&parsed, binding("r1", &profile), true, &catalog).unwrap_err();
    assert_eq!(
        (cancelled.code, cancelled.cause),
        (CompleteCode::Cancelled, CompleteCause::CallerCancelled)
    );
}

#[trace("Task-047")]
#[test]
fn formatter_reparse_uses_the_callers_explicit_limits() {
    let (catalog, profile) = catalog();
    let source = ugly(&profile);
    let parsed = parse(
        identity("r1"),
        "editor.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let refusal = format_document(
        &parsed,
        binding("r1", &profile),
        &catalog,
        Limits {
            source_bytes: source.len(),
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(
        (refusal.code, refusal.cause),
        (
            CompleteCode::ResourceExhausted,
            CompleteCause::InsufficientNextCharge
        )
    );
}

#[trace("Task-047")]
#[test]
fn every_catalog_aware_editor_path_refuses_the_exact_failing_profile_selection() {
    let (catalog, profile) = catalog();
    let known_digest = profile.digest().digest().to_string();
    let other_digest = format!("sha256:{}", "b".repeat(64));
    for (revision, selected_identity, selected_version, digest, expected_code, expected_cause) in [
        (
            "unknown-profile",
            "acme.unknown.complete/v1",
            "1",
            &known_digest,
            CompleteCode::UnknownProfile,
            CompleteCause::UnsupportedSelection,
        ),
        (
            "stale-profile",
            profile.identity(),
            "2",
            &known_digest,
            CompleteCode::StaleDependency,
            CompleteCause::RevisionMismatch,
        ),
        (
            "stale-digest",
            profile.identity(),
            "1",
            &other_digest,
            CompleteCode::StaleDependency,
            CompleteCause::ByteDigestMismatch,
        ),
    ] {
        let source = ugly(&profile).replacen(
            " record Reading",
            &format!(
                " profile Secondary = \"{selected_identity}\" version \"{selected_version}\" digest \"{digest}\"; record Reading"
            ),
            1,
        );
        let parsed = parse(
            identity(revision),
            "editor.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap();
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let failing_span = parsed.selections().profiles[1].identity_span;
        let assert_failure = |failure: &CompleteDiagnostic| {
            assert_eq!(failure.code, expected_code, "{revision}");
            assert_eq!(failure.cause, expected_cause, "{revision}");
            assert_eq!(failure.source, *parsed.source().identity(), "{revision}");
            assert_eq!(failure.span.start.byte, failing_span.start, "{revision}");
            assert_eq!(failure.span.end.byte, failing_span.end, "{revision}");
        };

        assert_failure(
            analyze_document(&parsed, binding(revision, &profile), false, &catalog)
                .unwrap_err()
                .as_ref(),
        );
        assert_failure(
            format_document(
                &parsed,
                binding(revision, &profile),
                &catalog,
                Limits::default(),
            )
            .unwrap_err()
            .as_ref(),
        );

        let record = source.find(" record Reading").unwrap() + 1;
        let edit = SourceEdit {
            range: Span {
                start: record,
                end: record,
            },
            replacement: " ".into(),
        };
        assert_failure(
            complete::apply_edit_with_catalog(
                &parsed,
                revision,
                identity(&format!("{revision}-next")),
                edit.clone(),
                &catalog,
                Limits::default(),
            )
            .unwrap_err()
            .as_ref(),
        );
        assert_failure(
            complete::apply_edit_with_catalog(
                &parsed,
                "superseded-revision",
                identity(&format!("{revision}-other")),
                edit,
                &catalog,
                Limits::default(),
            )
            .unwrap_err()
            .as_ref(),
        );
    }
}

#[trace("Task-047")]
#[test]
fn formatter_checks_trailing_comment_and_newline_capacity_before_append() {
    let (catalog, profile) = catalog();
    for (revision, source, expected_failure_span) in [
        ("trailing-newline", ugly(&profile), None),
        (
            "trailing-comment",
            format!("{} // trailing comment retained exactly", ugly(&profile)),
            Some(TokenClass::Comment),
        ),
    ] {
        let parsed = parse(
            identity(revision),
            "editor.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap();
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unrestricted = format_document(
            &parsed,
            binding(revision, &profile),
            &catalog,
            Limits::default(),
        )
        .unwrap();
        let formatted = &unrestricted.edits[0].replacement;
        let exact = Limits {
            source_bytes: formatted.len(),
            ..Limits::default()
        };
        let exact_response =
            format_document(&parsed, binding(revision, &profile), &catalog, exact).unwrap();
        assert_eq!(
            exact_response.edits[0].replacement, *formatted,
            "{revision}"
        );

        let one_short = format_document(
            &parsed,
            binding(revision, &profile),
            &catalog,
            Limits {
                source_bytes: formatted.len() - 1,
                ..Limits::default()
            },
        )
        .unwrap_err();
        assert_eq!(one_short.code, CompleteCode::ResourceExhausted);
        let expected = expected_failure_span.map_or(
            Span {
                start: source.len(),
                end: source.len(),
            },
            |class| {
                parsed
                    .cst()
                    .tokens()
                    .iter()
                    .find(|token| token.class() == class)
                    .unwrap()
                    .span()
            },
        );
        assert_eq!(one_short.span.start.byte, expected.start, "{revision}");
        assert_eq!(one_short.span.end.byte, expected.end, "{revision}");
    }
}
