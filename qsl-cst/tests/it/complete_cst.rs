// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lossless complete-V1 CST construction and recovery -- the layer-1-only
//! subset of the root crate's former `complete_cst.rs` (QSL-178 review F3):
//! these tests call only `qsl_cst::parse` and qsl-cst types, never the root
//! crate's `complete::{apply_edit, apply_edits}` incremental-editing API,
//! which stays covered by the root's own `tests/it/complete_cst.rs`.
use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode, HostCause, Limits, Production, TokenClass};
use qsl_foundation::SourceIdentity;

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

#[trace("TC-180", "TC-222", "FR-339-AC-1", "FR-339-AC-2", "FR-302-AC-1")]
#[test]
fn complete_source_builds_a_byte_exact_trivia_preserving_cst() {
    let parsed = qsl_cst::parse(
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
    // Distinct occurrences never collapse to one identity: nodes that share
    // one exact span (an `Expression` wrapping an `Implication` wrapping ...)
    // each resolve to themselves alone.
    let nodes = parsed.cst().nodes();
    let shared_span = nodes
        .iter()
        .enumerate()
        .find_map(|(index, node)| {
            nodes[index + 1..]
                .iter()
                .find(|other| other.span() == node.span())
                .map(|other| (node, other))
        })
        .expect("a production chain shares one span");
    assert_ne!(shared_span.0.identity(), shared_span.1.identity());
    for node in nodes {
        let resolved = parsed.cst().resolve(node.identity()).expect("own identity");
        assert_eq!(resolved.production(), node.production());
        assert_eq!(resolved.identity(), node.identity());
    }
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
    let baseline = qsl_cst::parse(
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
    assert!(qsl_cst::parse(
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
        qsl_cst::parse(
            identity("node-below"),
            "complete.native",
            SOURCE.as_bytes(),
            below,
        )
        .unwrap_err()
        .code,
        // QSL-236: the node ceiling is a `SyntaxLimit` kind the catalog
        // admits, so it now reports `stage_limit_exceeded`.
        CompleteCode::StageLimitExceeded
    );
}

#[trace("TC-222", "FR-302-AC-1")]
#[test]
fn token_limit_charges_every_retained_cst_leaf_at_the_exact_boundary() {
    let baseline = qsl_cst::parse(
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
    let accepted = qsl_cst::parse(
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
    let refusal = qsl_cst::parse(
        identity("leaf-below"),
        "complete.native",
        SOURCE.as_bytes(),
        below,
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::StageLimitExceeded);
    assert_eq!(refusal.byte_span().unwrap().start, first_excess.start);
    assert_eq!(refusal.byte_span().unwrap().end, first_excess.end);

    for (revision, source) in [
        ("whitespace-leaf", " "),
        ("comment-leaf", "// retained"),
        ("invalid-leaf", "@"),
    ] {
        let refusal = qsl_cst::parse(
            identity(revision),
            "complete.native",
            source.as_bytes(),
            Limits {
                tokens: 0,
                ..Limits::default()
            },
        )
        .unwrap_err();
        assert_eq!(refusal.code, CompleteCode::StageLimitExceeded, "{revision}");
        assert_eq!(refusal.byte_span().unwrap().start, 0, "{revision}");
        assert_eq!(refusal.byte_span().unwrap().end, source.len(), "{revision}");
    }
}

#[trace("TC-222", "FR-302-AC-1")]
#[test]
fn large_single_lexeme_refuses_at_the_first_excess_leaf() {
    let source = format!("0x{}", "f".repeat(900_000));
    let refusal = qsl_cst::parse(
        identity("large-hex-leaf-budget"),
        "large.native",
        source.as_bytes(),
        Limits {
            tokens: 8,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::StageLimitExceeded);
    assert_eq!(refusal.byte_span().unwrap().start, 9);
    assert_eq!(refusal.byte_span().unwrap().end, 10);
}

#[trace("TC-222", "FR-302-AC-2")]
#[test]
fn invalid_source_retains_bytes_and_exposes_recovery_without_admission() {
    let source = "language \"ix:native\" edition \"1-draft\"; profile C = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"; record Broken { value: Integer }";
    let parsed = qsl_cst::parse(
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
fn rendering_a_foreign_cst_node_is_a_typed_refusal() {
    let first = qsl_cst::parse(
        identity("first"),
        "first.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let second = qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:other-cst".into(),
            revision_namespace: "test".into(),
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
    let parsed = qsl_cst::parse(
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
    let first = qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:first-document".into(),
            revision_namespace: "test".into(),
            revision: "shared-revision".into(),
        },
        "first.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let second = qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:second-document".into(),
            revision_namespace: "test".into(),
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

    let changed = qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:first-document".into(),
            revision_namespace: "test".into(),
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

/// TC-188 R04 (FR-143-AC-10): a sum declaration is a layer-1 parse refusal
/// at `variant`. Moved here from the root crate's `composite_values.rs`
/// (QSL-183): it calls only `qsl_cst::parse`.
#[trace("TC-188", "FR-143-AC-10")]
#[test]
fn r04_a_sum_declaration_is_invalid_syntax_at_variant() {
    let text = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\nvariant V { A, B }";
    let parsed = qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:tc-188".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        "tc-188.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert!(!parsed.is_admissible());
    let diagnostic = &parsed.diagnostics()[0];
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert_eq!(
        diagnostic.byte_span().unwrap().start,
        text.find("variant").unwrap()
    );
}
