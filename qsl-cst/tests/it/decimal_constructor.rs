// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091-AC-58 (TC-1232 step 2): the malformed `decimal(c, s)` spellings
//! refuse at the constructor before S2, and the well-formed ones are admitted.

use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode, Limits};
use qsl_foundation::SourceIdentity;

/// The code and cause of the first diagnostic of a function whose body is
/// `body`, or `None` when S1 admits it.
fn refusal(body: &str) -> Option<(CompleteCode, CompleteCause)> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(): Boolean pure {{ {body} }}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        Limits::default(),
    );
    match &parsed {
        Err(refused) => Some((refused.code, refused.cause)),
        Ok(parsed) => parsed
            .diagnostics()
            .first()
            .map(|each| (each.code, each.cause)),
    }
}

#[trace("FR-091-AC-58", "TC-1232")]
#[test]
fn a_missing_or_extra_argument_comma_or_parenthesis_is_an_unexpected_token() {
    for body in [
        "decimal(1)",
        "decimal(1, 2, 3)",
        "decimal 5",
        "decimal(1 2)",
        "decimal(1, x)",
        "decimal(1, 2",
    ] {
        assert_eq!(
            refusal(body),
            Some((CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)),
            "{body}"
        );
    }
}

#[trace("FR-091-AC-58", "TC-1232")]
#[test]
fn a_numeric_token_that_is_no_signed_int_or_uint_is_an_invalid_token() {
    for body in [
        "decimal(+1, 2)",
        "decimal(01, 2)",
        "decimal(-0, 2)",
        "decimal(-00, 2)",
        "decimal(1.5, 2)",
        "decimal(1, 02)",
        "decimal(1, -2)",
        "decimal(1, 2.0)",
        // A scale past an unsigned 32-bit integer (FR-091-AC-57's bound).
        "decimal(1, 4294967296)",
    ] {
        assert_eq!(
            refusal(body),
            Some((CompleteCode::InvalidSyntax, CompleteCause::InvalidToken)),
            "{body}"
        );
    }
}

#[trace("FR-091-AC-58", "TC-1232")]
#[test]
fn the_well_formed_constructors_are_admitted() {
    for body in [
        "decimal(125, 2)",
        "decimal(-5, 0)",
        "decimal(0, 3)",
        "decimal(1, 4294967295)",
    ] {
        assert_eq!(refusal(body), None, "{body}");
    }
}

/// FR-091-AC-58: `decimal` is reserved, so no declaration takes it as a name.
#[trace("FR-091-AC-58", "TC-1232")]
#[test]
fn the_name_decimal_is_reserved_at_a_declaration() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
                profile v = \"quire.value.complete/v1\";\n\
                function decimal using v(): Boolean pure { true }\n";
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit");
    let first = parsed.diagnostics().first().expect("a refusal");
    assert_eq!(first.code, CompleteCode::InvalidSyntax);
}
