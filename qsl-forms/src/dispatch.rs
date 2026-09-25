// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S2 `forms` core: the closed, family-keyed dispatch entry table over
//! the closed leading-token-kind enum (ADR-012 §3, §4.3, §5.1 row S2), and
//! the shared parsed-form contract (ADR-011 §2.1 E2, §2.2 E2 row).
//!
//! S2 visits a unit's `Declaration` nodes in source order, and each
//! declaration's first significant token selects its dispatch entry
//! (FR-091, refining FR-067's "root construct's leading token"). The `Value`
//! family owns the `function`, `type`, `record` and `tuple` entries; every
//! other leading token has none and refuses the whole unit with
//! [`FormsCause::NoDispatchEntry`]. The families' own productions are in
//! their own modules (`value` for `Value`); this module holds no grammar.
//!
//! The mechanism itself — refusal on a recovering CST, span-only/no-identity
//! output, edition and declared-bound/extent carried unchanged, and a
//! single-call thin dispatch entry — is also exercised through a
//! `#[cfg(test)]`-only leading-token-kind variant and stub production
//! function (TC-167), whose fixture CSTs carry shapes the real grammar
//! cannot produce.

use qsl_cst::{CstElement, CstNode, LosslessCst, ParsedSource, Production, Recovery, TokenClass};
use qsl_foundation::diagnostic::{LimitExceeded, LimitKind, Locus};
use qsl_foundation::selection::SourceSelections;
use qsl_foundation::{Code, Source, Span};

use super::syntax::DeclarationForm;
use super::value;

/// The default S2 nesting-depth bound: the checker's own maximum
/// expression depth, so a form S2 builds is one check can admit.
pub const DEFAULT_FORMS_NESTING_DEPTH: u64 = 128;

/// The explicit limits the S2 entry takes (ADR-011 §2.3 Limits).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormsLimits {
    /// The deepest `Expression` or type-form node S2 builds: a root is at
    /// depth 1, and each child one deeper than its parent (FR-091 "S2 depth
    /// limit").
    pub nesting_depth: u64,
}

impl Default for FormsLimits {
    fn default() -> Self {
        Self {
            nesting_depth: DEFAULT_FORMS_NESTING_DEPTH,
        }
    }
}

/// A parsed form built from one declaration of a lossless CST with no
/// error or recovery node (ADR-011 §2.1 E2). It carries only the span of
/// its originating `Declaration` CST node (ADR-011 §2.2 E2 row, identity:
/// "none: forms carry position only"), its CST's edition unchanged
/// (version: "Edition carried"), and, when the originating construct
/// declared one, its bound or extent carried as syntax (proof metadata:
/// "Declared bounds and extents carried as syntax"). It exposes no accessor
/// whose return type is `NodeKey`, `DeclarationKey`, or any other ADR-013
/// O-03/O-04/O-07 check-time-minted identity type (FR-067-AC-2): identity
/// is minted only at check (S3), and this type has no field, and so no
/// accessor, that could return one.
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
    form: DeclarationForm,
}

impl ParsedForm {
    /// The span of the originating `Declaration` CST node.
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

    /// The family-produced declaration form.
    pub fn form(&self) -> &DeclarationForm {
        &self.form
    }

    /// The family-produced declaration form, by value.
    pub fn into_form(self) -> DeclarationForm {
        self.form
    }
}

/// S2's output for one source unit (FR-091 "Outputs"): the header's edition,
/// the unit's profile, import and model selections as spelled, and one
/// parsed form per declaration in source order.
#[derive(Clone, Debug)]
pub struct ParsedUnit {
    edition: Option<String>,
    selections: SourceSelections,
    forms: Vec<ParsedForm>,
}

impl ParsedUnit {
    /// The header's edition, as written.
    pub fn edition(&self) -> Option<&str> {
        self.edition.as_deref()
    }

    /// The unit's profile, import and model selections in source order, each
    /// with its alias, its definition reference and its span. S2 resolves
    /// none of them.
    pub fn selections(&self) -> &SourceSelections {
        &self.selections
    }

    /// The parsed forms in source order.
    pub fn forms(&self) -> &[ParsedForm] {
        &self.forms
    }

    /// The selections and the parsed forms, by value.
    pub fn into_parts(self) -> (SourceSelections, Vec<ParsedForm>) {
        (self.selections, self.forms)
    }
}

/// The closed leading-token-kind enum a declaration's first significant
/// token selects (ADR-012 §3, §5.1 row S2). Each family's own migration
/// ticket adds its variants; `Value` owns the four below (FR-091). The
/// `#[cfg(test)]` variant exists only to exercise the dispatch mechanism
/// (TC-167) over CSTs the real grammar cannot produce.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeadingTokenKind {
    /// `function`: the `Value` function form.
    Function,
    /// `type`: the `Value` alias form.
    Type,
    /// `record`: the `Value` record form.
    Record,
    /// `tuple`: the `Value` tuple form.
    Tuple,
    /// Test-only: never constructed outside this crate's own tests, and
    /// absent from every non-test build.
    #[cfg(test)]
    TestProbe,
}

/// Why the forms stage refused to build a parsed unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormsCause {
    /// The CST carries an error or recovery node; no partial form is built
    /// (ADR-011 §2.3 E2: "No. A form is built from a complete CST or not at
    /// all.").
    RecoveringCst,
    /// The source carries a diagnostic and no recovery (for example a
    /// profile refusal added by `ParsedSource::prepend_diagnostic`). Holds
    /// the first diagnostic's code.
    DiagnosedSource(Code),
    /// A declaration's leading token has no entry in the dispatch table.
    NoDispatchEntry {
        /// The leading token's spelling.
        spelling: String,
    },
    /// A construct no family's `Expression` variant represents (FR-091
    /// "Expression mapping").
    UnrepresentedConstruct {
        /// The construct's CST production.
        production: Production,
    },
    /// The CST does not have the shape its production's grammar rule
    /// gives it: a broken invariant between `qsl-cst` and this stage, never
    /// a property of the source.
    UnexpectedShape {
        /// The production whose node had the unexpected shape.
        production: Production,
    },
}

impl FormsCause {
    /// The catalog code of this cause (FR-091 "Catalog codes").
    pub fn catalog_code(&self) -> Code {
        match self {
            Self::RecoveringCst => Code::InvalidSyntax,
            Self::DiagnosedSource(code) => *code,
            Self::NoDispatchEntry { .. } | Self::UnrepresentedConstruct { .. } => {
                Code::UnsupportedConstruct
            }
            Self::UnexpectedShape { .. } => Code::RuntimeInvariant,
        }
    }
}

/// A forms-stage refusal: its cause, the byte span it concerns, and the
/// CST's own recovery evidence when the cause is
/// [`FormsCause::RecoveringCst`] (empty otherwise).
#[derive(Clone, Debug)]
pub struct FormsRefusal {
    /// The typed cause.
    pub cause: FormsCause,
    /// The span the refusal concerns, when it concerns one.
    pub span: Option<Span>,
    /// The CST's own proposed recovery edits, when any caused the refusal.
    pub recoveries: Vec<Recovery>,
}

/// Why S2 returned no parsed unit (ADR-013 T-4): a refusal, or a limit
/// reached first.
#[derive(Clone, Debug)]
pub enum FormsFailure {
    /// The unit was refused.
    Refused(FormsRefusal),
    /// The S2 nesting-depth bound was reached (FR-091 "S2 depth limit").
    Limit {
        /// The limit kind, configured bound and the depth reached.
        limit: LimitExceeded,
        /// The span of the first node past the bound.
        span: Span,
    },
}

impl FormsFailure {
    /// The catalog code: the refusal cause's, or `stage_limit_exceeded`.
    pub fn catalog_code(&self) -> Code {
        match self {
            Self::Refused(refusal) => refusal.cause.catalog_code(),
            Self::Limit { .. } => Code::StageLimitExceeded,
        }
    }

    pub(crate) fn refused(cause: FormsCause, span: Span) -> Self {
        Self::Refused(FormsRefusal {
            cause,
            span: Some(span),
            recoveries: Vec::new(),
        })
    }

    /// FR-096: a limit reached in `source`'s CST is located at
    /// `Locus::Region` over the span of the first node past the bound,
    /// under the source's `RawSourceRef`.
    fn located(self, source: &Source) -> Self {
        match self {
            Self::Limit { limit, span } => Self::Limit {
                limit: limit.at(source.region(span).map(Locus::Region)),
                span,
            },
            refused @ Self::Refused(_) => refused,
        }
    }

    pub(crate) fn depth(limits: FormsLimits, span: Span) -> Self {
        Self::Limit {
            limit: LimitExceeded::new(
                LimitKind::NestingDepth,
                limits.nesting_depth,
                u128::from(limits.nesting_depth) + 1,
            ),
            span,
        }
    }
}

/// One declaration a family production reads: the CST, the declaration's
/// own `Declaration` node, and the S2 limits.
#[derive(Clone, Copy)]
pub(crate) struct Construct<'c> {
    /// The unit's CST.
    pub(crate) cst: &'c LosslessCst,
    /// The declaration's `Declaration` node.
    pub(crate) node: &'c CstNode,
    /// The S2 limits.
    pub(crate) limits: FormsLimits,
}

/// Build the parsed unit of an admissible S1 output (ADR-011 §2.1 E2
/// admitted input), or refuse with its cause (ADR-011 §2.3 E2: "No. A form
/// is built from a complete CST or not at all.").
pub fn build_unit(parsed: &ParsedSource, limits: FormsLimits) -> Result<ParsedUnit, FormsFailure> {
    if !parsed.cst().recoveries().is_empty() {
        return Err(recovering(parsed.cst()));
    }
    if let Some(diagnostic) = parsed.diagnostics().first() {
        let span = diagnostic.region.as_ref().and_then(|region| {
            Some(Span {
                start: usize::try_from(region.start()).ok()?,
                end: usize::try_from(region.end()).ok()?,
            })
        });
        return Err(FormsFailure::Refused(FormsRefusal {
            cause: FormsCause::DiagnosedSource(diagnostic.code),
            span,
            recoveries: Vec::new(),
        }));
    }
    let forms =
        build_forms(parsed.cst(), limits).map_err(|failure| failure.located(parsed.source()))?;
    Ok(ParsedUnit {
        edition: declared_edition(parsed.cst()),
        selections: parsed.selections().clone(),
        forms,
    })
}

fn recovering(cst: &LosslessCst) -> FormsFailure {
    FormsFailure::Refused(FormsRefusal {
        cause: FormsCause::RecoveringCst,
        span: cst.recoveries().first().map(|recovery| recovery.span),
        recoveries: cst.recoveries().to_vec(),
    })
}

/// One parsed form per `Declaration` node under the CST root, in source
/// order; the first declaration with no dispatch entry, or the first
/// refusal a production gives, refuses the whole unit.
fn build_forms(cst: &LosslessCst, limits: FormsLimits) -> Result<Vec<ParsedForm>, FormsFailure> {
    if !cst.recoveries().is_empty() {
        return Err(recovering(cst));
    }
    let edition = declared_edition(cst);
    let mut forms = Vec::new();
    for node in declarations(cst) {
        let spelling = leading_token_spelling(cst, node).unwrap_or_default();
        let Some(kind) = from_spelling(spelling) else {
            return Err(FormsFailure::refused(
                FormsCause::NoDispatchEntry {
                    spelling: String::from_utf8_lossy(spelling).into_owned(),
                },
                node.span(),
            ));
        };
        let construct = Construct { cst, node, limits };
        let form = dispatch(kind, construct)?;
        forms.push(ParsedForm {
            span: node.span(),
            edition: edition.clone(),
            declared_extent: declared_extent(cst, node),
            form,
        });
    }
    Ok(forms)
}

/// The root's `Declaration` child nodes, in source order.
fn declarations(cst: &LosslessCst) -> impl Iterator<Item = &CstNode> {
    cst.root()
        .children()
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(index) => cst.nodes().get(*index),
            CstElement::Token(_) => None,
        })
        .filter(|node| node.production() == Production::Declaration)
}

/// The closed, family-keyed dispatch entry table (ADR-012 §3). Each entry
/// makes exactly one call into its family's own production function and
/// holds no other conditional, lookup or loop (ADR-012 §4.3 thin-dispatch
/// rule; FR-067-CON-4).
fn dispatch(
    kind: LeadingTokenKind,
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsFailure> {
    match kind {
        LeadingTokenKind::Function => value::function(construct),
        LeadingTokenKind::Type => value::alias(construct),
        LeadingTokenKind::Record => value::record(construct),
        LeadingTokenKind::Tuple => value::tuple(construct),
        #[cfg(test)]
        LeadingTokenKind::TestProbe => test_support::stub_production(construct),
    }
}

/// The first significant (non-whitespace, non-comment) token wholly inside
/// `node`'s own span, so a declaration never reads past its own end into
/// the next construct's leading token.
fn leading_token_spelling<'c>(cst: &'c LosslessCst, node: &CstNode) -> Option<&'c [u8]> {
    let span = node.span();
    cst.tokens()
        .iter()
        .find(|token| {
            token.class() == TokenClass::Token
                && token.span().start >= span.start
                && token.span().end <= span.end
        })
        .map(|token| token.spelling())
}

/// Maps a leading-token spelling to its dispatch-table entry, or `None`
/// when it selects no entry.
fn from_spelling(spelling: &[u8]) -> Option<LeadingTokenKind> {
    #[cfg(test)]
    if spelling == test_support::PROBE_SPELLING.as_bytes() {
        return Some(LeadingTokenKind::TestProbe);
    }
    match spelling {
        b"function" => Some(LeadingTokenKind::Function),
        b"type" => Some(LeadingTokenKind::Type),
        b"record" => Some(LeadingTokenKind::Record),
        b"tuple" => Some(LeadingTokenKind::Tuple),
        _ => None,
    }
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

/// The declaration's declared bound or extent, carried as syntax
/// (unparsed, unchanged) from the tokens inside its own span: the literal
/// token following a `bound` keyword token (the one declared-bound
/// construct in the current grammar, `grammar.rs`'s `VerificationStep`).
/// `None` when the declaration declares no such clause (ADR-011 §2.2 E2
/// row, proof metadata: "Declared bounds and extents carried as syntax").
fn declared_extent(cst: &LosslessCst, node: &CstNode) -> Option<Box<str>> {
    let span = node.span();
    let significant: Vec<_> = cst
        .tokens()
        .iter()
        .filter(|token| {
            token.class() == TokenClass::Token
                && token.span().start >= span.start
                && token.span().end <= span.end
        })
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
    use super::{Construct, FormsFailure};
    use crate::syntax::{AliasForm, BuiltinType, DeclarationForm, DeclaredName, TypeForm};

    /// The test-only leading token spelling `from_spelling` maps to
    /// [`super::LeadingTokenKind::TestProbe`].
    pub(super) const PROBE_SPELLING: &str = "__forms_test_probe__";

    /// A stub family production function (FR-067-AC-1, TC-167): the only
    /// producer `LeadingTokenKind::TestProbe` dispatches to. It ignores the
    /// CST's content and returns a fixed alias form, so it demonstrates the
    /// dispatch call itself, not any family's grammar.
    pub(super) fn stub_production(
        construct: Construct<'_>,
    ) -> Result<DeclarationForm, FormsFailure> {
        let span = construct.node.span();
        Ok(DeclarationForm::Alias(AliasForm {
            name: DeclaredName {
                name: "Probe".to_owned(),
                span,
            },
            target: TypeForm::builtin(BuiltinType::Boolean, span),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::PROBE_SPELLING;
    use super::*;
    use ix_trace_rs::trace;

    const HEADER: [&str; 4] = ["language", "\"ix:native\"", "edition", "\"1-draft\""];

    fn build(cst: &LosslessCst) -> Result<Vec<ParsedForm>, FormsFailure> {
        build_forms(cst, FormsLimits::default())
    }

    fn cause(failure: FormsFailure) -> FormsCause {
        match failure {
            FormsFailure::Refused(refusal) => refusal.cause,
            FormsFailure::Limit { .. } => panic!("a refusal, not a limit"),
        }
    }

    fn probe_cst(recoveries: Vec<Recovery>) -> LosslessCst {
        LosslessCst::fixture(&HEADER, &[PROBE_SPELLING], recoveries)
    }

    #[trace("TC-167", "FR-067-AC-1")]
    #[test]
    fn happy_path_builds_a_form_and_recovery_refuses_the_same_entry() {
        let clean = probe_cst(Vec::new());
        let built = build(&clean).expect("a clean CST with a dispatch entry builds a form");
        assert!(matches!(
            built.as_slice(),
            [form] if matches!(form.form(), DeclarationForm::Alias(_))
        ));

        let recovering = probe_cst(vec![Recovery {
            kind: qsl_cst::RecoveryKind::Insert,
            span: Span { start: 0, end: 0 },
            expected: "test recovery".into(),
        }]);
        let FormsFailure::Refused(refusal) = build(&recovering)
            .expect_err("a recovering CST refuses even though the same entry exists")
        else {
            panic!("a refusal");
        };
        assert_eq!(refusal.cause, FormsCause::RecoveringCst);
        assert_eq!(refusal.recoveries.len(), 1);
    }

    #[trace("TC-167", "FR-067-AC-2")]
    #[test]
    fn parsed_form_exposes_span_and_no_identity_accessor() {
        let cst = probe_cst(Vec::new());
        let built = build(&cst).unwrap();
        let declaration = declarations(&cst).next().expect("one declaration");
        assert_eq!(built[0].span(), declaration.span());
        // `ParsedForm` has no accessor returning `NodeKey`, `DeclarationKey`
        // or any other check-time-minted identity type: see the
        // `compile_fail` doctest on `ParsedForm` itself.
    }

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
            match_expr.arms.len() >= 5,
            "the four Value arms and the test-only arm are present"
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
            &["language", "\"ix:native\"", "edition", "\"0-draft\""],
            &[PROBE_SPELLING],
            Vec::new(),
        );
        let draft1 = probe_cst(Vec::new());
        let form0 = build(&draft0).unwrap();
        let form1 = build(&draft1).unwrap();
        assert_eq!(form0[0].edition(), Some("0-draft"));
        assert_eq!(form1[0].edition(), Some("1-draft"));
    }

    #[trace("TC-167")]
    #[test]
    fn edition_is_absent_rather_than_defaulted_when_the_cst_declares_none() {
        let no_header = LosslessCst::fixture(&[], &[PROBE_SPELLING], Vec::new());
        let form = build(&no_header).unwrap();
        assert_eq!(form[0].edition(), None);
    }

    #[trace("TC-167", "FR-067-AC-8")]
    #[test]
    fn declared_extent_is_carried_from_the_cst_unchanged() {
        let bound10 = LosslessCst::fixture(&HEADER, &[PROBE_SPELLING, "bound", "10"], Vec::new());
        let bound20 = LosslessCst::fixture(&HEADER, &[PROBE_SPELLING, "bound", "20"], Vec::new());
        let form10 = build(&bound10).unwrap();
        let form20 = build(&bound20).unwrap();
        assert_eq!(form10[0].declared_extent(), Some("10"));
        assert_eq!(form20[0].declared_extent(), Some("20"));
    }

    /// FR-091: the `record` entry replaced the M-3a no-entry fixture token,
    /// so FR-067-AC-3's no-entry case uses a spelling no family claims.
    #[trace("TC-167", "FR-067-AC-3")]
    #[test]
    fn no_dispatch_entry_refuses_a_clean_cst_with_no_matching_leading_token() {
        let cst = LosslessCst::fixture(&HEADER, &["invariant"], Vec::new());
        let failure = build(&cst).expect_err("no entry matches \"invariant\"");
        assert_eq!(
            cause(failure),
            FormsCause::NoDispatchEntry {
                spelling: "invariant".into()
            }
        );
    }
}
