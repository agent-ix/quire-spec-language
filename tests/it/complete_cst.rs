// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 incremental editing over the lossless CST (`complete::{apply_edit,
//! apply_edits}`). The layer-1-only CST construction/recovery tests that used to
//! live here moved to `qsl-cst/tests/it/complete_cst.rs` (QSL-178 review F3).
use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode, HostCause, Limits, SourceChange};
use qsl_foundation::{SourceIdentity, Span};
use quire_spec_language::complete::{self, SourceEdit};

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
        authority: "test".into(),
        identity: "test:complete-cst".into(),
        revision_namespace: "test".into(),
        revision: revision.into(),
    }
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn incremental_edit_reuses_only_unchanged_byte_correspondent_nodes() {
    let parsed = qsl_cst::parse(
        identity("r1"),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let start = SOURCE.find("if true then").unwrap() + 3;
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
    let map = parsed
        .cst()
        .reuse_map(
            edited.cst(),
            SourceChange {
                range: Span {
                    start,
                    end: start + 4,
                },
                inserted: "false".len(),
            },
        )
        .expect("the edited source is this edit applied");
    assert!(map.iter().any(Option::is_some));
    let replaced = parsed
        .cst()
        .node_covering(Span {
            start,
            end: start + 4,
        })
        .unwrap();
    assert_eq!(map[replaced.identity().node.get()], None);
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn stable_identity_survives_unrelated_preceding_sibling_insertion() {
    let parsed = qsl_cst::parse(
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
    let map = parsed
        .cst()
        .reuse_map(
            inserted.cst(),
            SourceChange {
                range: Span {
                    start: record_start,
                    end: record_start,
                },
                inserted: "record Earlier { datum: Integer; }\r\n".len(),
            },
        )
        .expect("the edited source is this edit applied");
    assert_eq!(
        map[original.identity().node.get()],
        Some(unchanged.identity().node)
    );
    assert!(parsed
        .cst()
        .ancestor_productions(original)
        .eq(inserted.cst().ancestor_productions(unchanged)));
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn stable_identity_survives_unrelated_whitespace_inside_one_ancestor() {
    let parsed = qsl_cst::parse(
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
    let map = parsed
        .cst()
        .reuse_map(
            edited.cst(),
            SourceChange {
                range: Span {
                    start: label - 1,
                    end: label - 1,
                },
                inserted: 1,
            },
        )
        .expect("the edited source is this edit applied");
    assert_eq!(
        map[original.identity().node.get()],
        Some(unchanged.identity().node)
    );
}

#[trace("Task-047")]
#[test]
fn stale_and_overlapping_incremental_changes_refuse_with_typed_codes() {
    let parsed = qsl_cst::parse(
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
    let parsed = qsl_cst::parse(
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
        assert_eq!(refusal.byte_span().unwrap().start, 0);
        assert_eq!(refusal.byte_span().unwrap().end, 0);
    }
}

#[trace("Task-047")]
#[test]
fn oversized_single_and_aggregate_replacements_refuse_before_reparse() {
    let parsed = qsl_cst::parse(
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
    let parsed = qsl_cst::parse(
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
    let full = qsl_cst::parse(
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
        let parsed = qsl_cst::parse(
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
        let full = qsl_cst::parse(
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
    let parsed = qsl_cst::parse(
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
