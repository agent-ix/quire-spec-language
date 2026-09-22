// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lossless complete-V1 CST construction, incremental editing and recovery.
use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::{SourceIdentity, Span};
use quire_spec_language::complete::{
    self, CompleteCause, CompleteCode, HostCause, Limits, Production, SourceEdit, TokenClass,
};

const SOURCE: &str = concat!(
    "language \"ix:native\" edition \"1-draft\";\r\n",
    "// exact trivia remains authored\r\n",
    "profile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\r\n",
    "import \"acme/base\" version \"1\" digest \"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\" as Base;\r\n",
    "model M = \"acme/model\" version \"1\" digest \"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\";\r\n",
    "dimension Length = M::Length;\r\n",
    "unit meter: M::Length = rational(1, 1) * M::meter;\r\n",
    "ordered enum Color { Red = \"red\", Blue, }\r\n",
    "record Reading { amount: Decimal[-2, 4; 18, 4; nearest-even]; label: Option<Text[0, 32; nfc]>?; }\r\n",
    "tuple Pair(Integer, Reference<M::Thing>);\r\n",
    "type Readings = Sequence<Reading>[0, 100];\r\n",
    "function choose using Complete (x: Integer): Integer pure { if true then x else 0 }\r\n",
    "predicate Good using Complete (x: Integer): Boolean { choose(x) >= 0 }\r\n",
    "relation Same using Complete over (left: M::Execution, right: M::Execution) { true }\r\n",
    "hyper Secret using Complete over traces in M::Execution bounded 2 forall trace a exists trace b { true }\r\n",
    "hybrid Plant using Complete { mode Idle invariant { true } flow { M::x' = 0; } transition to Idle when { true } reset { true }; }\r\n",
    "synthesis Find using Complete grammar M::Grammar domain M::Domain satisfies { true };\r\n",
    "verify Portfolio using Complete { check Safety claim M::Claim method M::Method domain M::Domain depends [Earlier] bound 10; }",
);

fn identity(revision: &str) -> SourceIdentity {
    SourceIdentity {
        identity: "test:complete-cst".into(),
        revision: revision.into(),
    }
}

#[trace("TC-180", "TC-222", "FR-339-AC-1", "FR-339-AC-2", "FR-302-AC-1")]
#[test]
fn complete_source_builds_a_byte_exact_trivia_preserving_cst() {
    let parsed = complete::parse(
        identity("r1"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .expect("valid UTF-8 and resource bounds");

    assert!(
        parsed.diagnostics().is_empty(),
        "{:?}",
        parsed.diagnostics()
    );
    assert!(parsed.is_admissible());
    assert_eq!(parsed.cst().render(), SOURCE.as_bytes());
    assert_eq!(parsed.source().text(), SOURCE);
    assert_eq!(
        parsed.cst().stable_node_ids().len(),
        parsed.cst().nodes().len(),
        "distinct CST occurrences must never collapse to one stable identity"
    );
    assert!(parsed
        .cst()
        .tokens()
        .iter()
        .any(|token| token.class() == TokenClass::Comment));
    assert!(parsed
        .cst()
        .tokens()
        .iter()
        .any(|token| token.class() == TokenClass::Whitespace));

    let productions: BTreeSet<_> = parsed
        .cst()
        .nodes()
        .iter()
        .map(|node| node.production())
        .collect();
    for expected in [
        Production::CompleteUnit,
        Production::ImportDeclaration,
        Production::DimensionDeclaration,
        Production::UnitDeclaration,
        Production::EnumDeclaration,
        Production::RecordDeclaration,
        Production::TupleDeclaration,
        Production::AliasDeclaration,
        Production::FunctionDeclaration,
        Production::RelationClause,
        Production::HyperClause,
        Production::HybridDeclaration,
        Production::SynthesisDeclaration,
        Production::VerificationPlan,
    ] {
        assert!(productions.contains(&expected), "missing {expected:?}");
    }
}

#[trace("TC-222", "FR-302-AC-1")]
#[test]
fn exact_syntax_node_limit_admits_the_boundary_and_refuses_one_less() {
    let baseline = complete::parse(
        identity("node-count"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let node_count = baseline.cst().nodes().len();
    let exact = Limits {
        nodes: node_count,
        ..Limits::default()
    };
    assert!(complete::parse(
        identity("node-exact"),
        "complete.native",
        SOURCE.as_bytes(),
        exact,
    )
    .unwrap()
    .is_admissible());
    let below = Limits {
        nodes: node_count - 1,
        ..Limits::default()
    };
    assert_eq!(
        complete::parse(
            identity("node-below"),
            "complete.native",
            SOURCE.as_bytes(),
            below,
        )
        .unwrap_err()
        .code,
        CompleteCode::ResourceExhausted
    );
}

#[trace("TC-222", "FR-302-AC-1")]
#[test]
fn token_limit_charges_every_retained_cst_leaf_at_the_exact_boundary() {
    let baseline = complete::parse(
        identity("leaf-count"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let leaf_count = baseline.cst().tokens().len();
    let exact = Limits {
        tokens: leaf_count,
        ..Limits::default()
    };
    let accepted = complete::parse(
        identity("leaf-exact"),
        "complete.native",
        SOURCE.as_bytes(),
        exact,
    )
    .unwrap();
    assert_eq!(accepted.cst().tokens().len(), leaf_count);
    assert!(accepted.is_admissible());

    let first_excess = baseline.cst().tokens()[leaf_count - 1].span();
    let below = Limits {
        tokens: leaf_count - 1,
        ..Limits::default()
    };
    let refusal = complete::parse(
        identity("leaf-below"),
        "complete.native",
        SOURCE.as_bytes(),
        below,
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::ResourceExhausted);
    assert_eq!(refusal.span.start.byte, first_excess.start);
    assert_eq!(refusal.span.end.byte, first_excess.end);

    for (revision, source) in [
        ("whitespace-leaf", " "),
        ("comment-leaf", "// retained"),
        ("invalid-leaf", "@"),
    ] {
        let refusal = complete::parse(
            identity(revision),
            "complete.native",
            source.as_bytes(),
            Limits {
                tokens: 0,
                ..Limits::default()
            },
        )
        .unwrap_err();
        assert_eq!(refusal.code, CompleteCode::ResourceExhausted, "{revision}");
        assert_eq!(refusal.span.start.byte, 0, "{revision}");
        assert_eq!(refusal.span.end.byte, source.len(), "{revision}");
    }
}

#[trace("TC-222", "FR-302-AC-1")]
#[test]
fn large_single_lexeme_refuses_at_the_first_excess_leaf() {
    let source = format!("0x{}", "f".repeat(900_000));
    let refusal = complete::parse(
        identity("large-hex-leaf-budget"),
        "large.native",
        source.as_bytes(),
        Limits {
            tokens: 8,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::ResourceExhausted);
    assert_eq!(refusal.span.start.byte, 9);
    assert_eq!(refusal.span.end.byte, 10);
}

#[trace("TC-222", "FR-302-AC-2")]
#[test]
fn invalid_source_retains_bytes_and_exposes_recovery_without_admission() {
    let source = "language \"ix:native\" edition \"1-draft\"; profile C = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"; record Broken { value: Integer }";
    let parsed = complete::parse(
        identity("broken"),
        "broken.native",
        source.as_bytes(),
        Limits::default(),
    )
    .expect("recoverable syntax is still a parsed source artifact");

    assert_eq!(parsed.cst().render(), source.as_bytes());
    assert!(!parsed.is_admissible());
    assert_eq!(
        (parsed.diagnostics()[0].code, parsed.diagnostics()[0].cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert!(!parsed.cst().recoveries().is_empty());
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn incremental_edit_reuses_only_unchanged_byte_correspondent_nodes() {
    let parsed = complete::parse(
        identity("r1"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let start = SOURCE.find("if true then").unwrap() + 3;
    let before = parsed.cst().stable_node_ids();
    let edited = complete::apply_edit(
        &parsed,
        "r1",
        identity("r2"),
        SourceEdit {
            range: Span {
                start,
                end: start + 4,
            },
            replacement: "false".into(),
        },
        Limits::default(),
    )
    .unwrap();

    assert!(
        edited.diagnostics().is_empty(),
        "{:?}",
        edited.diagnostics()
    );
    assert_ne!(parsed.source().digest(), edited.source().digest());
    assert!(before
        .intersection(&edited.cst().stable_node_ids())
        .next()
        .is_some());
    assert!(!edited.cst().stable_node_ids().contains(
        parsed
            .cst()
            .node_covering(Span {
                start,
                end: start + 4
            })
            .unwrap()
            .stable_id()
    ));
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn stable_identity_survives_unrelated_preceding_sibling_insertion() {
    let parsed = complete::parse(
        identity("r1"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let record_start = SOURCE.find("record Reading").unwrap();
    let original = parsed
        .cst()
        .node_covering(Span {
            start: record_start,
            end: record_start + "record Reading".len(),
        })
        .unwrap();
    let inserted = complete::apply_edit(
        &parsed,
        "r1",
        identity("r2"),
        SourceEdit {
            range: Span {
                start: record_start,
                end: record_start,
            },
            replacement: "record Earlier { datum: Integer; }\r\n".into(),
        },
        Limits::default(),
    )
    .unwrap();
    let shifted_start = record_start + "record Earlier { datum: Integer; }\r\n".len();
    let unchanged = inserted
        .cst()
        .node_covering(Span {
            start: shifted_start,
            end: shifted_start + "record Reading".len(),
        })
        .unwrap();
    assert_eq!(original.production(), unchanged.production());
    assert_eq!(original.stable_id(), unchanged.stable_id());
    assert_eq!(
        original.identity().ancestor_productions,
        unchanged.identity().ancestor_productions
    );
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn stable_identity_survives_unrelated_whitespace_inside_one_ancestor() {
    let parsed = complete::parse(
        identity("r1"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let option = SOURCE.find("Option<Text").unwrap();
    let original = parsed
        .cst()
        .node_covering(Span {
            start: option,
            end: option + "Option<Text[0, 32; nfc]>?".len(),
        })
        .unwrap();
    let label = SOURCE.find("label: Option").unwrap();
    let edited = complete::apply_edit(
        &parsed,
        "r1",
        identity("r2"),
        SourceEdit {
            range: Span {
                start: label - 1,
                end: label - 1,
            },
            replacement: " ".into(),
        },
        Limits::default(),
    )
    .unwrap();
    let unchanged = edited
        .cst()
        .node_covering(Span {
            start: option + 1,
            end: option + 1 + "Option<Text[0, 32; nfc]>?".len(),
        })
        .unwrap();
    assert_eq!(original.production(), unchanged.production());
    assert_eq!(original.stable_id(), unchanged.stable_id());
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn rendering_a_foreign_cst_node_is_a_typed_refusal() {
    let first = complete::parse(
        identity("first"),
        "first.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let second = complete::parse(
        SourceIdentity {
            identity: "test:other-cst".into(),
            revision: "second".into(),
        },
        "second.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let foreign = first.cst().render_node(second.cst().root()).unwrap_err();
    assert_eq!(
        (foreign.code, foreign.cause),
        (
            CompleteCode::InvalidSourceIdentity,
            CompleteCause::Host(HostCause::ForeignNode)
        )
    );
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn rendering_an_exchanged_clone_from_the_same_cst_succeeds() {
    let parsed = complete::parse(
        identity("clone"),
        "clone.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let exchanged = parsed.cst().root().clone();
    assert_eq!(
        parsed.cst().render_node(&exchanged).unwrap(),
        SOURCE.as_bytes()
    );
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn revision_bound_node_identity_includes_the_document_identity() {
    let first = complete::parse(
        SourceIdentity {
            identity: "test:first-document".into(),
            revision: "shared-revision".into(),
        },
        "first.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let second = complete::parse(
        SourceIdentity {
            identity: "test:second-document".into(),
            revision: "shared-revision".into(),
        },
        "second.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert_ne!(
        first.cst().root().identity(),
        second.cst().root().identity()
    );

    let changed = complete::parse(
        SourceIdentity {
            identity: "test:first-document".into(),
            revision: "shared-revision".into(),
        },
        "first.native",
        SOURCE.replace("Reading", "Changed").as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert_ne!(
        first.cst().root().identity(),
        changed.cst().root().identity(),
        "opaque caller labels cannot substitute for the exact source digest"
    );
}

#[trace("Task-047")]
#[test]
fn stale_and_overlapping_incremental_changes_refuse_with_typed_codes() {
    let parsed = complete::parse(
        identity("r1"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let edit = SourceEdit {
        range: Span { start: 0, end: 0 },
        replacement: " ".into(),
    };
    assert_eq!(
        complete::apply_edits(
            &parsed,
            "stale",
            identity("r2"),
            std::slice::from_ref(&edit),
            Limits::default(),
        )
        .unwrap_err()
        .cause,
        CompleteCause::Host(HostCause::EditPredecessor)
    );
    assert_eq!(
        complete::apply_edits(
            &parsed,
            "r1",
            identity("r2"),
            &[edit.clone(), edit],
            Limits::default(),
        )
        .unwrap_err()
        .cause,
        CompleteCause::Host(HostCause::EditRanges)
    );
}

#[trace("Task-047")]
#[test]
fn invalid_caller_edit_ranges_refuse_without_panicking() {
    let text = format!("{SOURCE}\r\n// 😀");
    let parsed = complete::parse(
        identity("invalid-ranges"),
        "complete.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let scalar = text.find('😀').unwrap();
    for (revision, range) in [
        (
            "split-scalar",
            Span {
                start: scalar + 1,
                end: scalar + 1,
            },
        ),
        (
            "reversed",
            Span {
                start: text.len(),
                end: text.len() - 1,
            },
        ),
        (
            "out-of-range",
            Span {
                start: text.len() + 1,
                end: text.len() + 1,
            },
        ),
    ] {
        let refusal = complete::apply_edit(
            &parsed,
            "invalid-ranges",
            identity(revision),
            SourceEdit {
                range,
                replacement: String::new(),
            },
            Limits::default(),
        )
        .unwrap_err();
        assert_eq!(
            (refusal.code, refusal.cause),
            (
                CompleteCode::InvalidSourceMap,
                CompleteCause::Host(HostCause::EditRanges)
            )
        );
        assert_eq!(refusal.span.start.byte, 0);
        assert_eq!(refusal.span.end.byte, 0);
    }
}

#[trace("Task-047")]
#[test]
fn oversized_single_and_aggregate_replacements_refuse_before_reparse() {
    let parsed = complete::parse(
        identity("bounded-edits"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let limits = Limits {
        source_bytes: SOURCE.len(),
        ..Limits::default()
    };
    let single = complete::apply_edit(
        &parsed,
        "bounded-edits",
        identity("single-too-large"),
        SourceEdit {
            range: Span { start: 0, end: 0 },
            replacement: "x".into(),
        },
        limits,
    )
    .unwrap_err();
    assert_eq!(single.code, CompleteCode::ResourceExhausted);

    let aggregate = complete::apply_edits(
        &parsed,
        "bounded-edits",
        identity("aggregate-too-large"),
        &[
            SourceEdit {
                range: Span { start: 0, end: 0 },
                replacement: "x".into(),
            },
            SourceEdit {
                range: Span { start: 1, end: 1 },
                replacement: "y".into(),
            },
        ],
        limits,
    )
    .unwrap_err();
    assert_eq!(aggregate.code, CompleteCode::ResourceExhausted);
}

#[trace("Task-047")]
#[test]
fn insertion_inside_crlf_falls_back_to_full_lexing() {
    let parsed = complete::parse(
        identity("crlf-r1"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let between = SOURCE.find("\r\n").unwrap() + 1;
    let edited = complete::apply_edit(
        &parsed,
        "crlf-r1",
        identity("crlf-r2"),
        SourceEdit {
            range: Span {
                start: between,
                end: between,
            },
            replacement: " ".into(),
        },
        Limits::default(),
    )
    .unwrap();
    assert!(!edited.is_incremental_result());
    let full = complete::parse(
        identity("crlf-r2"),
        "complete.native",
        edited.source().text().as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(edited.diagnostics(), full.diagnostics());
    assert_eq!(edited.cst().render(), full.cst().render());
}

#[trace("Task-047")]
#[test]
fn incremental_boundary_whitespace_keeps_the_root_on_the_full_document() {
    for (case, source, at, replacement) in [
        ("leading", format!(" {SOURCE}"), 0, "\t"),
        ("trailing", format!("{SOURCE}\n"), SOURCE.len() + 1, "\n"),
    ] {
        let original_revision = format!("{case}-r1");
        let edited_revision = format!("{case}-r2");
        let parsed = complete::parse(
            identity(&original_revision),
            "complete.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap();
        let edited = complete::apply_edit(
            &parsed,
            &original_revision,
            identity(&edited_revision),
            SourceEdit {
                range: Span { start: at, end: at },
                replacement: replacement.into(),
            },
            Limits::default(),
        )
        .unwrap();
        assert!(edited.is_incremental_result(), "{case}");
        let full = complete::parse(
            identity(&edited_revision),
            "complete.native",
            edited.source().text().as_bytes(),
            Limits::default(),
        )
        .unwrap();
        let expected_span = Span {
            start: 0,
            end: edited.source().text().len(),
        };
        assert_eq!(edited.cst().root().span(), expected_span, "{case}");
        assert_eq!(edited.cst().root().identity(), full.cst().root().identity());
        assert_eq!(
            edited.cst().render_node(edited.cst().root()).unwrap(),
            edited.source().text().as_bytes(),
            "{case}"
        );
    }
}

#[trace("Task-047")]
#[test]
fn adjacent_half_open_edits_are_not_overlaps() {
    let parsed = complete::parse(
        identity("r1"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let edited = complete::apply_edits(
        &parsed,
        "r1",
        identity("r2"),
        &[
            SourceEdit {
                range: Span { start: 0, end: 1 },
                replacement: "l".into(),
            },
            SourceEdit {
                range: Span { start: 1, end: 2 },
                replacement: "a".into(),
            },
        ],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(edited.source().text(), SOURCE);
}
