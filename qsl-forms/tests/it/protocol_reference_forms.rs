// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-308's protocol reference expression at S2: `protocol <qualified-name>`
//! builds one `ExprNode::ProtocolReference` per occurrence, with the written
//! target and the span of the whole expression, and resolves nothing.

use ix_trace_rs::trace;
use qsl_forms::{build_unit, DeclarationForm, ExprNode, FunctionDeclaration};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

/// The unit text and the functions its S2 forms hold, in source order.
fn functions(declarations: &str) -> (String, Vec<FunctionDeclaration>) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let functions = unit
        .forms()
        .iter()
        .filter_map(|form| match form.form() {
            DeclarationForm::Function(function) => Some((**function).clone()),
            _ => None,
        })
        .collect();
    (text, functions)
}

fn span_of(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// FR-308-AC-6: `protocol Fill` builds one reference with target `Fill` and
/// the span of the two tokens; `protocol M::Fill` builds the target
/// `M::Fill`; two references build two forms in source order.
#[trace("FR-308-AC-6", "TC-1021")]
#[test]
fn a_protocol_reference_builds_its_written_target_and_whole_span() {
    let (text, functions) = functions(
        "function a using v(): Boolean pure { protocol Fill }\n\
         function b using v(): Boolean pure { protocol M::Fill }",
    );
    let [a, b] = functions.as_slice() else {
        panic!("two functions: {functions:?}");
    };
    let root = a.body.root_node();
    assert_eq!(
        root,
        &ExprNode::ProtocolReference {
            target: "Fill".to_owned()
        }
    );
    assert_eq!(
        a.spans().expect("spans").body.at(&[]),
        Some(span_of(&text, "protocol Fill"))
    );
    assert_eq!(
        b.body.root_node(),
        &ExprNode::ProtocolReference {
            target: "M::Fill".to_owned()
        }
    );
    assert_eq!(
        b.spans().expect("spans").body.at(&[]),
        Some(span_of(&text, "protocol M::Fill"))
    );
}

/// FR-308-AC-6: two references in one body build two forms in source order,
/// and the target is only written, never resolved.
#[trace("FR-308-AC-6", "TC-1021")]
#[test]
fn two_references_in_one_unit_build_two_forms_in_source_order() {
    let (_, functions) =
        functions("function a using v(): Boolean pure { protocol Fill and protocol Drain }");
    let [a] = functions.as_slice() else {
        panic!("one function");
    };
    let targets: Vec<&str> = a
        .body
        .root()
        .children()
        .into_iter()
        .filter_map(|expression| match expression.node() {
            ExprNode::ProtocolReference { target } => Some(target.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(targets, ["Fill", "Drain"]);
}

/// FR-308-AC-6: a unit that holds a `protocol-clause` declaration and a
/// reference builds both, and the declaration parses as before.
#[trace("FR-308-AC-6", "TC-1021")]
#[test]
fn a_protocol_declaration_and_a_reference_build_side_by_side() {
    let text = format!(
        "{HEADER}function a using v(): Boolean pure {{ protocol Flow }}\n\
         protocol Flow using v over (input: Boolean) on origin {{\n\
         role R on M::Actor;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let kinds: Vec<&'static str> = unit
        .forms()
        .iter()
        .map(|form| match form.form() {
            DeclarationForm::Function(_) => "function",
            DeclarationForm::Protocol(_) => "protocol",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["function", "protocol"]);
}

/// FR-308: a target that is not a `qualified-name` is a parse refusal, not an
/// S2 form.
#[trace("FR-308-AC-6", "TC-1021")]
#[test]
fn a_target_that_is_no_qualified_name_is_not_admissible() {
    for body in ["protocol", "protocol 3", "protocol (Fill)"] {
        let text = format!("{HEADER}function a using v(): Boolean pure {{ {body} }}\n");
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(!parsed.is_admissible(), "{body}: admitted");
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == qsl_foundation::Code::InvalidSyntax),
            "{body}: {:?}",
            parsed.diagnostics()
        );
    }
}

/// FR-308: a reference formats and parses back to the same unit, so the
/// formatter keeps `protocol <target>` as written.
#[trace("FR-308-AC-6", "TC-1021")]
#[test]
fn a_protocol_reference_formats_and_reparses_to_the_same_bytes() {
    let text = format!("{HEADER}function a using v(): Boolean pure {{ protocol M::Fill }}\n");
    let parse = |text: &str| {
        qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit")
    };
    let formatted =
        qsl_cst::format::format_with_limit(&parse(&text), usize::MAX).expect("the unit formats");
    assert!(formatted.contains("protocol M::Fill"), "{formatted}");
    let again = qsl_cst::format::format_with_limit(&parse(&formatted), usize::MAX)
        .expect("the formatted unit formats");
    assert_eq!(again, formatted);
}
