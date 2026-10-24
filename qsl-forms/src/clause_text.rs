// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-103: the expression of a domain-package operation's `requires` or
//! `ensures` clause text, built by the production a unit `pre`/`post`
//! clause uses (FR-102, FR-146).
//!
//! The text is not a declaration, so this wraps it in the one clause
//! declaration the grammar has for a Boolean expression and reads the body
//! back out. S1 and S2 each run exactly as they do over a unit; nothing here
//! reads an expression a second way.

use qsl_cst::{CompleteDiagnostic, Limits};
use qsl_foundation::SourceIdentity;

use crate::dispatch::{build_unit, FormsRefusal};
use crate::syntax::{DeclarationForm, Expression};

/// Why a clause text did not become an [`Expression`].
#[derive(Clone, Debug)]
pub enum ClauseTextRefusal {
    /// S1 refused the text, or read it with a diagnostic.
    Source(Box<CompleteDiagnostic>),
    /// S2 refused the parsed text.
    Forms(FormsRefusal),
    /// The text closed the wrapping clause early and declared more, or
    /// declared nothing: it is not one expression.
    NotOneExpression,
}

/// The expression `text` spells, parsed by the unit clause production. The
/// text is the body of a one-clause wrapper, so a text that is not exactly
/// one expression refuses [`ClauseTextRefusal::NotOneExpression`] or the
/// S1/S2 cause that read it.
pub fn parse_clause_text(text: &str, limits: Limits) -> Result<Expression, ClauseTextRefusal> {
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile p = \"quire.value.complete/v1\";\npre Clause using p on M::T::op {{ {text} }}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("clause-text", "operation-clause", "fixture", "1"),
        "clause.native",
        unit.as_bytes(),
        limits,
    )
    .map_err(ClauseTextRefusal::Source)?;
    let built = build_unit(&parsed).map_err(ClauseTextRefusal::Forms)?;
    let (_, forms) = built.into_parts();
    let mut forms = forms.into_iter();
    match (forms.next(), forms.next()) {
        (Some(parsed), None) => match parsed.into_form() {
            DeclarationForm::StateClause(clause) => Ok(clause.body),
            _ => Err(ClauseTextRefusal::NotOneExpression),
        },
        _ => Err(ClauseTextRefusal::NotOneExpression),
    }
}
