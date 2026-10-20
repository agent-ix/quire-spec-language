// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-450 "Binding forms", QSL FR-304: S2 builds the abstraction
//! relation declaration -- `abstraction Name using p { type ...; population
//! ...; operation ...; }` -- run on real complete-V1 source through
//! `qsl_cst::parse` and `build_unit` (TC-811, the parse half). The compile
//! half, FR-304-AC-8, is claimed in `qsl-replay`'s model-gate tests.

use qsl_forms::{
    build_unit, AbstractionBindingForm, AbstractionDeclarationForm, DeclarationForm, ParsedUnit,
};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile t = \"quire.value.complete/v1\";\n\
     model Config = \"example/config-version\" version \"1\" digest \
     \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

fn build(declarations: &str) -> (String, ParsedUnit) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"));
    (text, unit)
}

fn sole_abstraction(unit: &ParsedUnit) -> &AbstractionDeclarationForm {
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Abstraction(declaration) = form.form() else {
        panic!("an abstraction declaration: {:?}", form.form());
    };
    declaration
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// TC-811 step 1, the S2 half: a declaration of an object type with two
/// fields, a population and an operation with two parameters builds its name,
/// `using` alias and three bindings in source order, each name and Rust
/// spelling with a span that slices to its text.
#[test]
fn a_declaration_builds_its_three_binding_forms() {
    let declaration = r#"abstraction Link using t {
        type Config::ConfigVersion to "store::ConfigVersion" { version to "version"; note to "0"; };
        population Config::configs to "state::configs";
        operation Config::ConfigVersion::attemptUpdate to "update" receiver "self" (n to "count", m to "limit");
    }"#;
    let (text, unit) = build(declaration);
    let form = sole_abstraction(&unit);

    assert_eq!(form.name.name, "Link");
    assert_eq!(slice(&text, form.name.span), "Link");
    assert_eq!(form.profile.alias, "t");
    assert_eq!(slice(&text, form.span), declaration);

    let [AbstractionBindingForm::Object(object), AbstractionBindingForm::Population(population), AbstractionBindingForm::Frame(frame)] =
        &form.bindings[..]
    else {
        panic!(
            "an object, a population and a frame binding: {:?}",
            form.bindings
        );
    };

    assert_eq!(object.model_type.name, "Config::ConfigVersion");
    assert_eq!(
        slice(&text, object.model_type.span),
        "Config::ConfigVersion"
    );
    assert_eq!(object.rust_type.text, "store::ConfigVersion");
    assert_eq!(
        slice(&text, object.rust_type.span),
        "\"store::ConfigVersion\""
    );
    let fields: Vec<(&str, &str)> = object
        .fields
        .iter()
        .map(|field| (field.field.name.as_str(), field.rust_field.text.as_str()))
        .collect();
    assert_eq!(fields, [("version", "version"), ("note", "0")]);
    assert_eq!(
        slice(&text, object.fields[0].span),
        "version to \"version\";"
    );

    assert_eq!(population.population.name, "Config::configs");
    assert_eq!(population.collection.text, "state::configs");
    assert_eq!(
        slice(&text, population.span),
        "population Config::configs to \"state::configs\";"
    );

    assert_eq!(frame.operation.name, "Config::ConfigVersion::attemptUpdate");
    assert_eq!(frame.function.text, "update");
    assert_eq!(frame.receiver.text, "self");
    let parameters: Vec<(&str, &str)> = frame
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.parameter.name.as_str(),
                parameter.rust_parameter.text.as_str(),
            )
        })
        .collect();
    assert_eq!(parameters, [("n", "count"), ("m", "limit")]);
    assert_eq!(slice(&text, frame.parameters[1].span), "m to \"limit\"");
}

/// TC-811 (FR-450 "Binding forms"): an object binding with no braces binds no
/// field, and an operation with no parameters has an empty parameter list.
#[test]
fn an_object_binding_without_braces_has_no_fields_and_a_frame_may_have_no_parameters() {
    let (_, unit) = build(
        r#"abstraction Link using t {
            type Config::ConfigVersion to "ConfigVersion";
            operation Config::ConfigVersion::reset to "reset" receiver "store" ();
        }"#,
    );
    let form = sole_abstraction(&unit);
    let [AbstractionBindingForm::Object(object), AbstractionBindingForm::Frame(frame)] =
        &form.bindings[..]
    else {
        panic!("an object and a frame binding: {:?}", form.bindings);
    };
    assert!(object.fields.is_empty());
    assert_eq!(frame.receiver.text, "store");
    assert!(frame.parameters.is_empty());
}

/// FR-450 "Rust spellings": a Rust string is the source bytes between its
/// quotes. S2 does not decode an escape, so S3 can refuse a backslash.
#[test]
fn a_rust_spelling_is_kept_as_the_bytes_between_its_quotes() {
    let (_, unit) = build(
        r#"abstraction Link using t {
            type Config::ConfigVersion to "aA" { version to "r#type"; };
        }"#,
    );
    let form = sole_abstraction(&unit);
    let [AbstractionBindingForm::Object(object)] = &form.bindings[..] else {
        panic!("one object binding: {:?}", form.bindings);
    };
    assert_eq!(object.rust_type.text, r"aA");
    assert_eq!(object.fields[0].rust_field.text, "r#type");
}
