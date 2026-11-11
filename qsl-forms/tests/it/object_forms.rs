// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569 "Surface syntax": S1 parses and S2 builds the source `object`
//! declaration of TC-1408 fixture S, and refuses the malformed variants.

use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode};
use qsl_forms::{
    build_unit, DeclarationForm, FrameEntryForm, ObjectForm, ObjectMemberForm, ParsedUnit,
};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"quire.state.core/v1\";\n";

/// TC-1408 fixture S's three object declarations.
const OBJECTS: &str = "\
object Order identity (total) {
  total: Int[0, 9];
  note: Text[0, 8; nfc]?;
  population orders;
  operation size(): Integer;
  operation count(): Integer;
  operation scaled(n: Integer): Integer;
  operation bump(): Boolean modifies self.total;
}
object Sub extends Order {
  operation size(): Integer redefines Order::size;
  operation scaled(m: Integer): Integer redefines Order::scaled;
}
abstract object Shape { population shapes; }
";

fn parse(declarations: &str) -> (String, qsl_cst::ParsedSource) {
    let text = format!("{HEADER}{declarations}");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    (text, parsed)
}

fn build(declarations: &str) -> (String, ParsedUnit) {
    let (text, parsed) = parse(declarations);
    assert!(
        parsed.is_admissible(),
        "{declarations}: {:?}",
        parsed.diagnostics()
    );
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{declarations}: {failure:?}"));
    (text, unit)
}

fn objects(unit: &ParsedUnit) -> Vec<&ObjectForm> {
    unit.forms()
        .iter()
        .map(|form| match form.form() {
            DeclarationForm::Object(object) => object.as_ref(),
            other => panic!("an object form, not {other:?}"),
        })
        .collect()
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// Fixture S parses to one form per object declaration, each part in source
/// order with its span.
#[trace("FR-569-AC-1", "TC-1408")]
#[test]
fn fixture_s_builds_one_form_per_object_with_every_part_spanned() {
    let (text, unit) = build(OBJECTS);
    let [order, sub, shape] = objects(&unit)[..] else {
        panic!("three objects build three forms");
    };

    assert_eq!(order.name.name, "Order");
    assert_eq!(slice(&text, order.name.span), "Order");
    assert!(order.abstract_span.is_none());
    assert!(order.extends.is_empty());
    let identity: Vec<_> = order.identity.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(identity, ["total"]);
    let kinds: Vec<&str> = order
        .members
        .iter()
        .map(|member| match member {
            ObjectMemberForm::Field(field) => {
                if field.optional {
                    "optional field"
                } else {
                    "field"
                }
            }
            ObjectMemberForm::Population(_) => "population",
            ObjectMemberForm::Operation(_) => "operation",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "field",
            "optional field",
            "population",
            "operation",
            "operation",
            "operation",
            "operation"
        ]
    );
    let names: Vec<&str> = order
        .members
        .iter()
        .map(|member| match member {
            ObjectMemberForm::Field(field) => field.name.name.as_str(),
            ObjectMemberForm::Population(population) => population.name.name.as_str(),
            ObjectMemberForm::Operation(operation) => operation.name.name.as_str(),
        })
        .collect();
    assert_eq!(
        names,
        ["total", "note", "orders", "size", "count", "scaled", "bump"]
    );
    let ObjectMemberForm::Operation(scaled) = &order.members[5] else {
        panic!("scaled is an operation");
    };
    assert_eq!(scaled.parameters.len(), 1);
    assert_eq!(scaled.parameters[0].name.name, "n");
    assert!(scaled.result.is_some());
    let ObjectMemberForm::Operation(bump) = &order.members[6] else {
        panic!("bump is an operation");
    };
    let [FrameEntryForm::ModifiesReceiver { field }] = &bump.frame[..] else {
        panic!("bump has one receiver entry");
    };
    assert_eq!(slice(&text, field.span), "total");
    assert_eq!(
        slice(&text, bump.span),
        "operation bump(): Boolean modifies self.total;"
    );
    let ObjectMemberForm::Operation(count) = &order.members[4] else {
        panic!("count is an operation");
    };
    assert!(count.frame.is_empty() && count.redefines.is_none());

    let extends: Vec<_> = sub.extends.iter().map(|name| name.name.as_str()).collect();
    assert_eq!(extends, ["Order"]);
    assert_eq!(slice(&text, sub.extends[0].span), "Order");
    let ObjectMemberForm::Operation(size) = &sub.members[0] else {
        panic!("size is an operation");
    };
    let redefines = size.redefines.as_ref().expect("size redefines");
    assert_eq!(
        (
            redefines.declaring.name.as_str(),
            redefines.member.name.as_str()
        ),
        ("Order", "size")
    );
    assert_eq!(slice(&text, redefines.member.span), "size");

    assert_eq!(shape.name.name, "Shape");
    assert_eq!(
        shape.abstract_span.map(|span| slice(&text, span)),
        Some("abstract")
    );
    assert!(matches!(&shape.members[..], [ObjectMemberForm::Population(p)] if !p.symmetric));
    assert_eq!(
        slice(&text, shape.span),
        "abstract object Shape { population shapes; }"
    );
}

/// `symmetric` marks a population symmetric, and the frame entry kinds each
/// build.
#[trace("FR-569-AC-1", "TC-1408")]
#[test]
fn a_symmetric_population_and_every_frame_entry_kind_build() {
    let (_, unit) = build(
        "object Node {\n\
           total: Integer;\n\
           symmetric population nodes;\n\
           operation touch(p: Reference<Node>) modifies self.total, total, Node.total\n\
             creates Node deletes self, p, Node;\n\
         }\n",
    );
    let [node] = objects(&unit)[..] else {
        panic!("one object");
    };
    assert!(matches!(&node.members[1], ObjectMemberForm::Population(p) if p.symmetric));
    let ObjectMemberForm::Operation(touch) = &node.members[2] else {
        panic!("touch is an operation");
    };
    assert!(matches!(
        &touch.frame[..],
        [
            FrameEntryForm::ModifiesReceiver { .. },
            FrameEntryForm::ModifiesField { .. },
            FrameEntryForm::ModifiesTypeField { .. },
            FrameEntryForm::Creates(_),
            FrameEntryForm::DeletesReceiver(_),
            FrameEntryForm::Deletes(_),
            FrameEntryForm::Deletes(_),
        ]
    ));
}

/// A malformed object declaration is one `invalid_syntax` diagnostic at the
/// offending token and builds no `ObjectForm`.
#[trace("FR-569-AC-1", "TC-1408")]
#[test]
fn a_malformed_object_is_one_syntax_diagnostic_at_the_offending_token() {
    for (declaration, at) in [
        ("object Node { operation step() }", "}"),
        (
            "object Node { operation step(): Integer modifies self.; }",
            ";",
        ),
        ("object Node extends {}", "{"),
    ] {
        let (text, parsed) = parse(declaration);
        assert!(!parsed.is_admissible(), "{declaration}");
        let diagnostics = parsed.diagnostics();
        assert_eq!(diagnostics.len(), 1, "{declaration}: {diagnostics:?}");
        let span = diagnostics[0].byte_span().expect("a located diagnostic");
        assert_eq!(&text[span.start..span.end], at, "{declaration}");
        assert!(build_unit(&parsed).is_err(), "{declaration}");
    }
}

/// A parameter named `extends` or `modifies` is refused at the word, since
/// every grammar word of an object declaration is reserved.
#[trace("FR-569-AC-1", "TC-1408")]
#[test]
fn a_reserved_word_parameter_is_refused_at_the_word() {
    for word in [
        "extends",
        "modifies",
        "symmetric",
        "redefines",
        "creates",
        "deletes",
    ] {
        let declaration = format!("object Node {{ operation step({word}: Integer); }}");
        let (text, parsed) = parse(&declaration);
        assert!(!parsed.is_admissible(), "{word}");
        let diagnostics = parsed.diagnostics();
        assert_eq!(diagnostics.len(), 1, "{word}: {diagnostics:?}");
        assert_eq!(diagnostics[0].code, CompleteCode::InvalidSyntax, "{word}");
        assert_eq!(
            diagnostics[0].cause,
            CompleteCause::UnexpectedToken,
            "{word}"
        );
        let span = diagnostics[0].byte_span().expect("a located diagnostic");
        assert_eq!(&text[span.start..span.end], word);
    }
}
