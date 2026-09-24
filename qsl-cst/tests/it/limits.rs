// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-199: every [`Limits`] field is enforced as the caller supplies it,
//! above or below [`Limits::default`] (NFR-001: an implementation ceiling is
//! not a domain bound), and [`qsl_cst::ParsedSource::effective_limits`]
//! records exactly the limits the parse was checked against. Each test goes
//! through the public [`qsl_cst::parse`]/[`qsl_cst::parse_source`] entry
//! points, so a ceiling clamped anywhere on that path turns it red.
//!
//! Raising `nesting` is safe because both the lexer and the parser keep
//! explicit stacks (QSL-197); the nesting tests below run on a 512 KiB
//! thread to prove a raised ceiling can never abort the process.
use ix_trace_rs::trace;
use qsl_cst::diagnostic::read_source;
use qsl_cst::{CompleteCode, CompleteDiagnostic, Limits, ParsedSource};
use qsl_foundation::{SourceIdentity, SyntaxLimit};

/// The smallest admissible unit prefix these fixtures grow from: an edition
/// line and the profile every function names.
const HEADER: &str = concat!(
    "language \"ix:native\" edition \"1-draft\";\n",
    "profile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n",
);

fn identity(id: &str) -> SourceIdentity {
    SourceIdentity {
        identity: format!("test:limits:{id}"),
        revision: "1".into(),
    }
}

/// Limits large enough that no ceiling is reached, for measuring a source.
fn unbounded() -> Limits {
    Limits {
        source_bytes: usize::MAX,
        tokens: usize::MAX,
        nodes: usize::MAX,
        nesting: Limits::default().nesting,
    }
}

fn parse(id: &str, text: &str, limits: Limits) -> Result<ParsedSource, CompleteCode> {
    qsl_cst::parse(identity(id), "limits.native", text.as_bytes(), limits)
        .map_err(|refusal| refusal.code)
}

/// A valid unit carrying `lines` comment lines: each is a comment leaf plus a
/// whitespace leaf and adds no syntax node, so the leaf count grows with no
/// node growth.
fn comment_padded(lines: usize) -> String {
    let mut text = String::from(HEADER);
    text.push_str("function f using Complete (x: Integer): Integer pure { x }\n");
    text.push_str(&"// c\n".repeat(lines));
    text
}

/// A valid unit whose one function body is a flat `+` chain of `terms`
/// operands: nesting depth 0, with syntax nodes growing faster than leaves.
fn plus_chain(terms: usize) -> String {
    let mut text = String::from(HEADER);
    text.push_str("function f using Complete (x: Integer): Integer pure { x");
    text.push_str(&"+x".repeat(terms.saturating_sub(1)));
    text.push_str(" }\n");
    text
}

/// A caller-raised `source_bytes` admits a source past the 1 MiB default,
/// with no hidden foundation-layer ceiling underneath, and the parse records
/// the raised limit.
#[trace("TC-012", "NFR-001-M-2")]
#[test]
fn a_caller_raised_source_byte_ceiling_admits_past_the_default() {
    let mut text = comment_padded(0);
    text.push_str("// ");
    text.push_str(&"a".repeat(Limits::default().source_bytes));
    text.push('\n');
    assert_eq!(
        parse("bytes-default", &text, Limits::default()).unwrap_err(),
        CompleteCode::ResourceExhausted,
        "the default source-byte ceiling must refuse a source past it"
    );

    let raised = Limits {
        source_bytes: text.len(),
        ..Limits::default()
    };
    let parsed = parse("bytes-raised", &text, raised)
        .expect("a caller-raised source-byte ceiling must admit a source at it");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.effective_limits(), raised);

    let refusal = qsl_cst::parse(
        identity("bytes-past"),
        "limits.native",
        text.as_bytes(),
        Limits {
            source_bytes: text.len() - 1,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::ResourceExhausted);
    assert_eq!(
        refusal.limit(),
        Some(SyntaxLimit::SourceBytes {
            bound: text.len() - 1
        })
    );
}

/// A caller-raised `tokens` ceiling admits a unit with more CST leaves than
/// the 100,000 default while every other ceiling stays at its default, and
/// one leaf fewer refuses naming the kind and the raised bound.
#[trace("TC-012", "NFR-001-M-2")]
#[test]
fn a_caller_raised_token_ceiling_admits_past_the_default_and_refuses_one_less() {
    let text = comment_padded(Limits::default().tokens / 2 + 10);
    let leaves = parse("tokens-measure", &text, unbounded())
        .unwrap()
        .cst()
        .tokens()
        .len();
    assert!(leaves > Limits::default().tokens);
    assert_eq!(
        parse("tokens-default", &text, Limits::default()).unwrap_err(),
        CompleteCode::ResourceExhausted,
    );

    let raised = Limits {
        tokens: leaves,
        ..Limits::default()
    };
    let parsed = parse("tokens-raised", &text, raised)
        .expect("a caller-raised token ceiling must admit a unit at it");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.effective_limits(), raised);

    let refusal = qsl_cst::parse(
        identity("tokens-past"),
        "limits.native",
        text.as_bytes(),
        Limits {
            tokens: leaves - 1,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::ResourceExhausted);
    assert_eq!(
        refusal.limit(),
        Some(SyntaxLimit::Tokens { bound: leaves - 1 })
    );
}

/// A caller-raised `nodes` ceiling admits, through
/// [`qsl_cst::parse_source`], a unit with more syntax nodes than the 50,000
/// default while tokens stay within their default, and one node fewer
/// refuses.
#[trace("TC-012", "NFR-001-M-2")]
#[test]
fn a_caller_raised_node_ceiling_admits_past_the_default_and_refuses_one_less() {
    // Each `+x` operand adds five syntax nodes and two leaves.
    let text = plus_chain(Limits::default().nodes / 5 + 100);
    let measured = parse("nodes-measure", &text, unbounded()).unwrap();
    let nodes = measured.cst().nodes().len();
    assert!(nodes > Limits::default().nodes, "{nodes} nodes");
    assert!(measured.cst().tokens().len() <= Limits::default().tokens);

    let source = || {
        read_source(
            identity("nodes"),
            "limits.native",
            text.as_bytes(),
            Limits::default().source_bytes,
        )
        .unwrap()
    };
    assert_eq!(
        qsl_cst::parse_source(source(), Limits::default())
            .unwrap_err()
            .code,
        CompleteCode::ResourceExhausted,
    );

    let raised = Limits {
        nodes,
        ..Limits::default()
    };
    let parsed = qsl_cst::parse_source(source(), raised)
        .expect("a caller-raised node ceiling must admit a unit at it");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.effective_limits(), raised);

    let refusal = qsl_cst::parse_source(
        source(),
        Limits {
            nodes: nodes - 1,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::ResourceExhausted);
    assert_eq!(
        refusal.limit(),
        Some(SyntaxLimit::Nodes { bound: nodes - 1 })
    );
}

/// The whitespace fast path records its predecessor's limits only when the
/// edit asks for exactly those limits; any other limits fall back to a full
/// reparse (`Ok(None)`), so `effective_limits` never names a limit that was
/// not enforced against the result.
#[trace("TC-012", "NFR-001-M-2")]
#[test]
fn the_whitespace_fast_path_records_only_limits_it_enforced() {
    let text = comment_padded(3);
    let raised = Limits {
        tokens: Limits::default().tokens + 1,
        ..Limits::default()
    };
    let parsed = parse("fast-path", &text, raised).unwrap();
    let at = text.len() - 1;
    let mut edited = text.clone();
    edited.insert(at, ' ');

    assert!(
        parsed
            .with_whitespace_insertion(
                identity("fast-2"),
                at,
                " ",
                edited.as_bytes(),
                Limits::default()
            )
            .unwrap()
            .is_none(),
        "limits other than the predecessor's must fall back to a full reparse"
    );

    let fast = parsed
        .with_whitespace_insertion(identity("fast-2"), at, " ", edited.as_bytes(), raised)
        .unwrap()
        .expect("the predecessor's own limits take the fast path");
    assert_eq!(fast.effective_limits(), raised);
    assert_eq!(fast.source().text(), edited);
}

/// Run `run` on a 512 KiB thread. A stack overflow aborts the whole test
/// process, so reaching `join` at all is the stack-safety evidence.
fn on_bounded_stack<F: FnOnce() + Send + 'static>(run: F) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(run)
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the parser must not overflow a 512 KiB stack");
}

/// A unit whose function body nests `depth` parenthesis pairs inside the
/// body's own `{ }` pair: `depth + 1` levels in all.
fn nested_parens(depth: usize) -> String {
    let mut text = String::from(HEADER);
    text.push_str("function f using Complete (x: Integer): Integer pure { ");
    text.push_str(&"(".repeat(depth));
    text.push('x');
    text.push_str(&")".repeat(depth));
    text.push_str(" }\n");
    text
}

/// A unit whose type alias nests `depth` `Option<...>` pairs.
fn nested_options(depth: usize) -> String {
    format!(
        "{HEADER}type T = {}Integer{};\n",
        "Option<".repeat(depth),
        ">".repeat(depth)
    )
}

/// `outcome` is admitted, with exactly `limits` recorded.
fn assert_admitted_under(outcome: Result<ParsedSource, Box<CompleteDiagnostic>>, limits: Limits) {
    let parsed = outcome.unwrap_or_else(|refusal| panic!("expected admission, got {refusal:?}"));
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.effective_limits(), limits);
    assert_eq!(parsed.effective_limits().nesting, 100_000);
}

/// QSL-199 finding 1: raising `nesting` to 100,000 never crashes the
/// parser. 6,000 nested parentheses (through [`qsl_cst::parse`]) and 6,000
/// nested `Option<...>` (through [`qsl_cst::parse_source`]) on a 512 KiB
/// thread are admitted with the raised limits recorded. Node and token
/// ceilings are raised too, so the parse must succeed outright rather than
/// stop at another ceiling.
#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn a_raised_nesting_ceiling_never_crashes_the_parser() {
    on_bounded_stack(|| {
        const DEPTH: usize = 6_000;
        let limits = Limits {
            nesting: 100_000,
            tokens: 10_000_000,
            nodes: 10_000_000,
            ..Limits::default()
        };

        let parens = nested_parens(DEPTH);
        let outcome = qsl_cst::parse(
            identity("deep-parens"),
            "limits.native",
            parens.as_bytes(),
            limits,
        );
        assert_admitted_under(outcome, limits);

        let options = nested_options(DEPTH);
        let source = read_source(
            identity("deep-options"),
            "limits.native",
            options.as_bytes(),
            limits.source_bytes,
        )
        .unwrap();
        let outcome = qsl_cst::parse_source(source, limits);
        assert_admitted_under(outcome, limits);
    });
}

/// QSL-199 finding 1: a caller-raised `nesting` ceiling admits a unit the
/// default of 64 refuses, and one pair past the raised ceiling refuses
/// naming it.
#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn a_caller_raised_nesting_ceiling_admits_past_the_default() {
    on_bounded_stack(|| {
        let default = Limits::default().nesting;
        // `default` parentheses inside the body's `{ }`: default + 1 levels.
        let text = nested_parens(default);
        let refusal = qsl_cst::parse(
            identity("nesting-default"),
            "limits.native",
            text.as_bytes(),
            Limits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal.limit(),
            Some(SyntaxLimit::NestingDepth { bound: default })
        );

        let raised = Limits {
            nesting: default + 1,
            ..Limits::default()
        };
        let parsed = parse("nesting-raised", &text, raised)
            .expect("a caller-raised nesting ceiling must admit a unit at it");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        assert_eq!(parsed.effective_limits(), raised);

        let deeper = nested_parens(default + 1);
        let refusal = qsl_cst::parse(
            identity("nesting-past"),
            "limits.native",
            deeper.as_bytes(),
            raised,
        )
        .unwrap_err();
        assert_eq!(
            refusal.limit(),
            Some(SyntaxLimit::NestingDepth { bound: default + 1 })
        );
    });
}
