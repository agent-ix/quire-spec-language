// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S2 `forms` core: the closed, family-keyed dispatch entry table over
//! the closed leading-token-kind enum (ADR-012 §3, §4.3, §5.1 row S2), and
//! the shared parsed-form contract (ADR-011 §2.1 E2, §2.2 E2 row).
//!
//! This module owns no family's grammar production function and no
//! family's variant of the parsed-form enum (FR-067-CON-2, M-3b). Until a
//! family's own migration ticket adds its variant and production function,
//! [`LeadingTokenKind`] carries no production variant at all, and the
//! dispatch `match` in [`dispatch`] has no production arm: every real input
//! refuses with [`FormsCause::NoDispatchEntry`], which is expected and
//! correct — the S2 boundary exists, but nothing is wired onto it yet. The
//! mechanism itself — refusal on a recovering CST, span-only/no-identity
//! output, edition and declared-bound/extent carried unchanged, and a
//! single-call thin dispatch entry — is exercised in this module's own
//! tests through a `#[cfg(test)]`-only leading-token-kind variant and stub
//! production function (TC-167), so it is proved without depending on any
//! family having migrated onto S2 forms.

use super::Expression;
use qsl_cst::{LosslessCst, Recovery, TokenClass};
use qsl_foundation::Span;

/// A parsed form built from a lossless CST with no error or recovery node
/// (ADR-011 §2.1 E2). It carries only the span of its originating CST node
/// (ADR-011 §2.2 E2 row, identity: "none: forms carry position only"), its
/// CST's edition unchanged (version: "Edition carried"), and, when the
/// originating construct declared one, its bound or extent carried as
/// syntax (proof metadata: "Declared bounds and extents carried as
/// syntax"). It exposes no accessor whose return type is `NodeKey`,
/// `DeclarationKey`, or any other ADR-013 O-03/O-04/O-07 check-time-minted
/// identity type (FR-067-AC-2): identity is minted only at check (S3), and
/// this type has no field, and so no accessor, that could return one.
///
/// No accessor here returns a `NodeKey` or a `DeclarationKey` (FR-067-AC-2,
/// TC-167 step 3); this fails to compile because neither method exists:
///
/// ```compile_fail
/// use qsl_forms::ParsedForm;
/// fn read_identity(form: &ParsedForm) {
///     let _ = form.node_key();
/// }
/// ```
///
/// ```compile_fail
/// use qsl_forms::ParsedForm;
/// fn read_declaration(form: &ParsedForm) {
///     let _ = form.declaration_key();
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ParsedForm {
    span: Span,
    edition: Option<String>,
    declared_extent: Option<Box<str>>,
    expression: Expression,
}

impl ParsedForm {
    /// The span of the originating CST node.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The edition the originating CST recorded, unchanged; `None` when the
    /// CST's header declared none (absence is never defaulted to a
    /// particular edition string).
    pub fn edition(&self) -> Option<&str> {
        self.edition.as_deref()
    }

    /// The originating construct's declared bound or extent, carried as
    /// syntax, unchanged from the CST; `None` when the construct declared
    /// none.
    pub fn declared_extent(&self) -> Option<&str> {
        self.declared_extent.as_deref()
    }

    /// The family-produced parsed content (ADR-012 §4.3: the one
    /// `Expression` enum, shared across every family at S2).
    pub fn expression(&self) -> &Expression {
        &self.expression
    }
}

/// The closed leading-token-kind enum a CST root construct's first
/// significant token selects (ADR-012 §3, §5.1 row S2). It carries no
/// production variant in M-3a (FR-067-CON-2): each family's own migration
/// ticket adds its variant when that family migrates onto S2 forms (M-3b).
/// The `#[cfg(test)]` variant exists only to exercise the dispatch
/// mechanism (TC-167) without depending on any family having migrated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeadingTokenKind {
    /// Test-only: never constructed outside this crate's own tests, and
    /// absent from every non-test build.
    #[cfg(test)]
    TestProbe,
}

/// Why the forms stage refused to build a parsed form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormsCause {
    /// The CST carries an error or recovery node; no partial form is built
    /// (ADR-011 §2.3 E2: "No. A form is built from a complete CST or not at
    /// all.").
    RecoveringCst,
    /// The CST's leading token has no entry in the dispatch table. Every
    /// input takes this arm until a family's migration ticket adds its
    /// variant and production function (M-3b).
    NoDispatchEntry,
}

/// A forms-stage refusal: its cause, and the CST's own recovery evidence
/// when the cause is [`FormsCause::RecoveringCst`] (empty otherwise).
#[derive(Clone, Debug)]
pub struct FormsRefusal {
    /// The typed cause.
    pub cause: FormsCause,
    /// The CST's own proposed recovery edits, when any caused the refusal.
    pub recoveries: Vec<Recovery>,
}

/// Build a parsed form from a lossless CST with no error or recovery node
/// (ADR-011 §2.1 E2 admitted input), or refuse with diagnostics (ADR-011
/// §2.3 E2: "No. A form is built from a complete CST or not at all.").
pub fn build_form(cst: &LosslessCst) -> Result<ParsedForm, FormsRefusal> {
    if !cst.recoveries().is_empty() {
        return Err(FormsRefusal {
            cause: FormsCause::RecoveringCst,
            recoveries: cst.recoveries().to_vec(),
        });
    }
    let Some(kind) = leading_token_kind(cst) else {
        return Err(FormsRefusal {
            cause: FormsCause::NoDispatchEntry,
            recoveries: Vec::new(),
        });
    };
    Ok(ParsedForm {
        span: cst.root().span(),
        edition: declared_edition(cst),
        declared_extent: declared_extent(cst),
        expression: dispatch(kind, cst),
    })
}

/// The closed, family-keyed dispatch entry table (ADR-012 §3). Each entry
/// makes exactly one call into its family's own production function and
/// holds no other conditional, lookup or loop (ADR-012 §4.3 thin-dispatch
/// rule; FR-067-AC-3). It has no production entry in M-3a (FR-067-CON-2):
/// [`LeadingTokenKind`] carries no production variant, so this `match` has
/// no production arm either, and is still exhaustive.
#[cfg_attr(not(test), allow(unused_variables))]
fn dispatch(kind: LeadingTokenKind, cst: &LosslessCst) -> Expression {
    match kind {
        #[cfg(test)]
        LeadingTokenKind::TestProbe => test_support::stub_production(cst),
    }
}

/// The leading-token kind the CST root construct's first significant token
/// selects, or `None` when it selects no dispatch entry.
fn leading_token_kind(cst: &LosslessCst) -> Option<LeadingTokenKind> {
    let spelling = leading_token_spelling(cst)?;
    from_spelling(spelling)
}

/// The first significant (non-whitespace, non-comment) token wholly inside
/// the CST's root node's own span — not merely at or after its start, so a
/// root with no token of its own (once M-3b wires a real family, a
/// construct whose own leading token happens to be absent, malformed, or
/// otherwise not covered by the root's span) never silently reads past the
/// root's end into the next construct's leading token instead.
fn leading_token_spelling(cst: &LosslessCst) -> Option<&[u8]> {
    let root_span = cst.root().span();
    cst.tokens()
        .iter()
        .find(|token| {
            token.class() == TokenClass::Token
                && token.span().start >= root_span.start
                && token.span().end <= root_span.end
        })
        .map(|token| token.spelling())
}

/// Maps a leading-token spelling to its dispatch-table entry, or `None`
/// when it selects no entry. It has no production arm in M-3a
/// (FR-067-CON-2): every real (non-`#[cfg(test)]`) spelling maps to `None`
/// until a family's migration ticket adds its own arm here (M-3b).
fn from_spelling(spelling: &[u8]) -> Option<LeadingTokenKind> {
    #[cfg(test)]
    if spelling == test_support::PROBE_SPELLING.as_bytes() {
        return Some(LeadingTokenKind::TestProbe);
    }
    let _ = spelling;
    None
}

/// The header's declared edition literal, decoded, from the CST's own
/// token stream: `language <name> edition <text>`. `None` when the header
/// has no edition clause, or when its literal is not valid UTF-8 (ADR-011
/// §2.2 E2 row, version: "Edition carried" — this reads the value E1
/// minted, unchanged; a missing value stays absent rather than becoming a
/// default string, since a default would itself be a recomputation FR-067
/// forbids).
fn declared_edition(cst: &LosslessCst) -> Option<String> {
    let significant: Vec<_> = cst
        .tokens()
        .iter()
        .filter(|token| token.class() == TokenClass::Token)
        .collect();
    for (index, token) in significant.iter().enumerate() {
        if token.spelling() == b"language"
            && significant
                .get(index + 2)
                .is_some_and(|edition| edition.spelling() == b"edition")
        {
            let literal = significant.get(index + 3)?;
            return unquote(literal.spelling());
        }
    }
    None
}

/// The originating construct's declared bound or extent, carried as syntax
/// (unparsed, unchanged) from the CST's own token stream: the literal
/// token following a `bound` keyword token (the one declared-bound
/// construct in the current grammar, `grammar.rs`'s `VerificationStep`).
/// `None` when the CST declares no such clause (ADR-011 §2.2 E2 row, proof
/// metadata: "Declared bounds and extents carried as syntax").
fn declared_extent(cst: &LosslessCst) -> Option<Box<str>> {
    let significant: Vec<_> = cst
        .tokens()
        .iter()
        .filter(|token| token.class() == TokenClass::Token)
        .collect();
    for (index, token) in significant.iter().enumerate() {
        if token.spelling() == b"bound" {
            if let Some(literal) = significant.get(index + 1) {
                return String::from_utf8(literal.spelling().to_vec())
                    .ok()
                    .map(String::into_boxed_str);
            }
        }
    }
    None
}

/// Strips one matching pair of leading/trailing double quotes, if present.
/// `None` when `spelling` is not valid UTF-8 — the same explicit disposition
/// [`declared_extent`] already gives non-UTF-8 syntax, rather than the
/// lossy substitution a `from_utf8_lossy` decode would silently apply. The
/// complete-V1 text-literal token pattern admits no raw control character
/// (`grammar.rs`), and this fixture-facing decode does not need to unescape
/// a JSON escape sequence beyond stripping the surrounding quotes.
fn unquote(spelling: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(spelling).ok()?;
    Some(
        text.strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(text)
            .to_owned(),
    )
}

#[cfg(test)]
mod test_support {
    use super::Expression;
    use qsl_cst::LosslessCst;

    /// The test-only leading token spelling `from_spelling` maps to
    /// [`super::LeadingTokenKind::TestProbe`].
    pub(super) const PROBE_SPELLING: &str = "__forms_test_probe__";

    /// A stub family production function (FR-067-AC-1, TC-167): the only
    /// producer `LeadingTokenKind::TestProbe` dispatches to. It ignores the
    /// CST's content and returns a fixed parsed value; this is legitimate
    /// because CON-2 forbids a real family production function in M-3a, so
    /// this stub demonstrates the dispatch call itself, not any family's
    /// grammar.
    pub(super) fn stub_production(_cst: &LosslessCst) -> Expression {
        Expression::Boolean(true)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::PROBE_SPELLING;
    use super::*;
    use ix_trace_rs::trace;

    fn probe_cst(recoveries: Vec<Recovery>) -> LosslessCst {
        LosslessCst::fixture(
            &[
                "language",
                "\"ix:native\"",
                "edition",
                "\"1-draft\"",
                PROBE_SPELLING,
            ],
            recoveries,
        )
    }

    #[trace("TC-167", "FR-067-AC-1")]
    #[test]
    fn happy_path_builds_a_form_and_recovery_refuses_the_same_entry() {
        let clean = probe_cst(Vec::new());
        let built = build_form(&clean).expect("a clean CST with a dispatch entry builds a form");
        assert!(matches!(built.expression(), Expression::Boolean(true)));

        let recovering = probe_cst(vec![Recovery {
            kind: qsl_cst::RecoveryKind::Insert,
            span: Span { start: 0, end: 0 },
            expected: "test recovery".into(),
        }]);
        let refusal = build_form(&recovering)
            .expect_err("a recovering CST refuses even though the same entry exists");
        assert_eq!(refusal.cause, FormsCause::RecoveringCst);
        assert_eq!(refusal.recoveries.len(), 1);
    }

    #[trace("TC-167", "FR-067-AC-2")]
    #[test]
    fn parsed_form_exposes_span_and_no_identity_accessor() {
        let cst = probe_cst(Vec::new());
        let built = build_form(&cst).unwrap();
        assert_eq!(built.span(), cst.root().span());
        // `ParsedForm` has no accessor returning `NodeKey`, `DeclarationKey`
        // or any other check-time-minted identity type: see the
        // `compile_fail` doctest on `ParsedForm` itself.
    }

    #[trace("TC-167", "FR-067-AC-3")]
    #[test]
    fn dispatch_entry_is_a_single_thin_call() {
        let source = include_str!("dispatch.rs");
        let file = syn::parse_file(source).expect("dispatch.rs parses as a Rust file");
        let dispatch_fn = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == "dispatch" => Some(function),
                _ => None,
            })
            .expect("fn dispatch exists");
        let match_expr = dispatch_fn
            .block
            .stmts
            .iter()
            .find_map(|statement| match statement {
                syn::Stmt::Expr(syn::Expr::Match(match_expr), _) => Some(match_expr),
                _ => None,
            })
            .expect("dispatch's body is one match expression");
        assert!(
            !match_expr.arms.is_empty(),
            "the test-only arm must be present under cfg(test)"
        );
        for arm in &match_expr.arms {
            assert!(
                !matches!(arm.pat, syn::Pat::Wild(_)),
                "a dispatch entry arm must not be a catch-all `_` arm"
            );
            let shape = match &*arm.body {
                syn::Expr::Call(_) => "Call",
                syn::Expr::MethodCall(_) => "MethodCall",
                syn::Expr::Path(_) => "Path",
                syn::Expr::If(_) => "If",
                syn::Expr::Match(_) => "Match",
                syn::Expr::ForLoop(_) => "ForLoop",
                syn::Expr::While(_) => "While",
                syn::Expr::Loop(_) => "Loop",
                _ => "other",
            };
            assert!(
                matches!(shape, "Call" | "MethodCall" | "Path"),
                "a dispatch entry must make exactly one call and hold no other \
                 conditional, lookup or loop; arm body shape was {shape}"
            );
        }
    }

    #[trace("TC-167", "FR-067-AC-7")]
    #[test]
    fn edition_is_carried_from_the_cst_unchanged() {
        let draft0 = LosslessCst::fixture(
            &[
                "language",
                "\"ix:native\"",
                "edition",
                "\"0-draft\"",
                PROBE_SPELLING,
            ],
            Vec::new(),
        );
        let draft1 = LosslessCst::fixture(
            &[
                "language",
                "\"ix:native\"",
                "edition",
                "\"1-draft\"",
                PROBE_SPELLING,
            ],
            Vec::new(),
        );
        let form0 = build_form(&draft0).unwrap();
        let form1 = build_form(&draft1).unwrap();
        assert_eq!(form0.edition(), Some("0-draft"));
        assert_eq!(form1.edition(), Some("1-draft"));
    }

    #[trace("TC-167")]
    #[test]
    fn edition_is_absent_rather_than_defaulted_when_the_cst_declares_none() {
        let no_header = LosslessCst::fixture(&[PROBE_SPELLING], Vec::new());
        let form = build_form(&no_header).unwrap();
        assert_eq!(form.edition(), None);
    }

    #[trace("TC-167", "FR-067-AC-8")]
    #[test]
    fn declared_extent_is_carried_from_the_cst_unchanged() {
        let bound10 = LosslessCst::fixture(
            &[
                "language",
                "\"ix:native\"",
                "edition",
                "\"1-draft\"",
                "bound",
                "10",
                PROBE_SPELLING,
            ],
            Vec::new(),
        );
        let bound20 = LosslessCst::fixture(
            &[
                "language",
                "\"ix:native\"",
                "edition",
                "\"1-draft\"",
                "bound",
                "20",
                PROBE_SPELLING,
            ],
            Vec::new(),
        );
        let form10 = build_form(&bound10).unwrap();
        let form20 = build_form(&bound20).unwrap();
        assert_eq!(form10.declared_extent(), Some("10"));
        assert_eq!(form20.declared_extent(), Some("20"));
    }

    #[trace("TC-167")]
    #[test]
    fn no_dispatch_entry_refuses_a_clean_cst_with_no_matching_leading_token() {
        let cst = LosslessCst::fixture(
            &[
                "language",
                "\"ix:native\"",
                "edition",
                "\"1-draft\"",
                "record",
            ],
            Vec::new(),
        );
        let refusal = build_form(&cst).expect_err("no entry matches \"record\"");
        assert_eq!(refusal.cause, FormsCause::NoDispatchEntry);
    }
}
