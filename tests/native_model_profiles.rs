// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-120: explicit rational producer profiles through the public source/model APIs.

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::NativeType;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::binding_work::{Limits as BindingLimits, Work};
use quire_spec_language::linking::composed::models::{
    bind_models, ImportRefusal, ModelErrorKind, ModelInput,
};
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, SourceInventory, SyntaxNamespace, UnitId, WorkLimits,
};
use quire_spec_language::linking::DeclarationKey;
use quire_spec_language::model_source::{
    read, ModelDraft, ModelSourceCause, ModelSourceLimits, FORMAT, FORMAT_V2,
};
use quire_spec_language::native_model::{
    ModelLimits, NativeModel, NativeModelProfile, ScalarKind, ScalarSite, Unit,
};
use quire_spec_language::syntax::composed::{DeclarationKind, ParameterType, QualifiedName};
use quire_spec_language::{
    link_native, parse, ByteDigest, Code, Limits, LinkLimits, Source, SourceIdentity, Span, Spanned,
};
use serde_json::{json, Value};

const RATIONAL: &str = r#"{"kind":"rational","name":"Ratio","numerator_minimum":-1,"numerator_maximum":1,"maximum_denominator":2}"#;
const AMOUNT: &str = r#"{"name":"amount","kind":"state","type":{"kind":"scalar","name":"Ratio"}}"#;
const WRAPPED: &str = r#"{"name":"amount","kind":"state","type":{"kind":"option","value":{"kind":"sequence","maximum":3,"value":{"kind":"scalar","name":"Ratio"}}}}"#;

fn symbol(name: &str) -> ir::SymbolName {
    ir::SymbolName::new(name).unwrap()
}

fn named_source(text: &str, native: &str, formal: &str) -> FormalSource {
    FormalSource::new(
        Source::read(
            SourceIdentity {
                identity: native.into(),
                revision: "1".into(),
            },
            "model.json",
            text.as_bytes(),
            1_048_576,
        )
        .unwrap(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new(formal).unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    )
}

fn source(text: &str) -> FormalSource {
    named_source(text, "test:rule-model", "RuleModelSource")
}

fn document(scalars: &str, records: &str, values: &str) -> String {
    format!(
        r#"{{"license":"AGPL-3.0-or-later","package":"test/rational","requirement":"RationalModel","revision":1,"scalars":[{scalars}],"records":[{records}],"values":[{values}],"objects":[],"operations":[]}}"#
    )
}

fn draft(text: &str) -> ModelDraft {
    read(source(text), FORMAT_V2, ModelSourceLimits::default()).unwrap()
}

fn model(text: &str) -> NativeModel {
    draft(text).admit(ModelLimits::default()).unwrap()
}

fn rational(ty: &ir::ValueType) -> &ir::RationalType {
    match ty {
        ir::ValueType::Rational { value } => value,
        _ => panic!("expected actual IR rational representation: {ty:?}"),
    }
}

fn assert_bounds(ty: &ir::ValueType, expected: (i64, i64, u64)) {
    let ty = rational(ty);
    assert_eq!(
        (
            ty.numerator_minimum(),
            ty.numerator_maximum(),
            ty.maximum_denominator()
        ),
        expected
    );
}

fn assert_occurrence(source: &FormalSource, span: &ir::SourceSpan, original: &str) {
    let start = source.source().text().find(original).unwrap();
    assert_eq!(
        source.to_native(span).unwrap(),
        Span {
            start,
            end: start + original.len()
        }
    );
    assert_eq!(
        source.source().slice(Span {
            start,
            end: start + original.len()
        }),
        Some(original)
    );
}

#[test]
#[trace("TC-120", "TC-131", "FR-041-AC-1", "FR-041-AC-6", "FR-047-AC-8")]
fn explicit_profiles_preserve_the_frozen_historical_artifact() {
    let text = include_str!("fixtures/native-package/model-source.json");
    let old = read(source(text), FORMAT, ModelSourceLimits::default()).unwrap();
    assert_eq!(old.profile(), NativeModelProfile::V1);
    let direct = NativeModel::new(
        old.source.clone(),
        old.environment.clone(),
        old.roles.clone(),
        ModelLimits::default(),
    )
    .unwrap();
    let v1 = old.admit(ModelLimits::default()).unwrap();
    let frozen: Value = serde_json::from_slice(include_bytes!(
        "fixtures/native-package/minimal.package.json"
    ))
    .unwrap();
    assert_eq!(
        v1.artifact_bytes(),
        frozen["models"][0]["artifact"].as_str().unwrap().as_bytes()
    );
    assert_eq!(direct.artifact_bytes(), v1.artifact_bytes());
    let selected = read(source(text), FORMAT_V2, ModelSourceLimits::default()).unwrap();
    assert_eq!(selected.profile(), NativeModelProfile::V2);
    let v2 = selected.admit(ModelLimits::default()).unwrap();
    assert_eq!(v2.profile(), NativeModelProfile::V2);
    let mut expected: Value = serde_json::from_slice(v1.artifact_bytes()).unwrap();
    expected["profile"] = json!("native-state-model/2");
    assert_eq!(
        serde_json::from_slice::<Value>(v2.artifact_bytes()).unwrap(),
        expected
    );
    assert_ne!(v1.digest(), v2.digest());
    assert_eq!(v1.environment(), v2.environment());
    for (name, expected) in [
        ("native-state-model/1", NativeModelProfile::V1),
        ("native-state-model/2", NativeModelProfile::V2),
    ] {
        assert_eq!(name.parse::<NativeModelProfile>(), Ok(expected));
        assert_eq!(NativeModelProfile::try_from(name), Ok(expected));
        assert_eq!(expected.as_str(), name);
    }
    for unknown in ["", "native-state-model/3", "native-rule-model/2"] {
        assert!(unknown.parse::<NativeModelProfile>().is_err());
        assert!(NativeModelProfile::try_from(unknown).is_err());
    }
    assert_eq!(
        read(
            source(text),
            "native-rule-model/3",
            ModelSourceLimits::default()
        )
        .unwrap_err()
        .code(),
        Code::UnknownWire
    );
}

#[test]
#[trace("TC-120", "FR-041-AC-1", "FR-041-AC-4", "FR-041-AC-6")]
fn integer_and_text_source_meanings_agree_across_explicit_profiles() {
    let text = include_str!("fixtures/native-rule-model.json");
    let old = read(source(text), FORMAT, ModelSourceLimits::default()).unwrap();
    let selected = read(source(text), FORMAT_V2, ModelSourceLimits::default()).unwrap();
    assert_eq!(old.profile(), NativeModelProfile::V1);
    assert_eq!(selected.profile(), NativeModelProfile::V2);
    assert_eq!(old.environment, selected.environment);
    assert_eq!(old.roles, selected.roles);
    assert!(old.roles.scalars.iter().any(|role| matches!(
        role.kind,
        ScalarKind::Integer {
            unit: Unit::Dimensionless
        }
    )));
    assert!(old.roles.scalars.iter().any(|role| matches!(
        role.kind,
        ScalarKind::Integer {
            unit: Unit::Named(_)
        }
    )));
    assert!(old
        .roles
        .scalars
        .iter()
        .any(|role| matches!(role.kind, ScalarKind::Text { max_scalars: 256 })));

    let old = old.admit(ModelLimits::default()).unwrap();
    let selected = selected.admit(ModelLimits::default()).unwrap();
    assert_eq!(old.environment(), selected.environment());
    assert_eq!(old.roles(), selected.roles());
    assert_eq!(
        old.source().source().digest(),
        selected.source().source().digest()
    );
    let mut expected: Value = serde_json::from_slice(old.artifact_bytes()).unwrap();
    expected["profile"] = json!("native-state-model/2");
    assert_eq!(
        serde_json::from_slice::<Value>(selected.artifact_bytes()).unwrap(),
        expected
    );
    assert_ne!(old.artifact_bytes(), selected.artifact_bytes());
    assert_ne!(old.digest(), selected.digest());
}

#[test]
#[trace("TC-120", "FR-041-AC-1", "FR-041-AC-6")]
fn replacing_public_draft_payloads_cannot_retag_the_selected_profile() {
    let legacy = include_str!("fixtures/native-rule-model.json");
    let rational_text = document(RATIONAL, "", AMOUNT);
    let mut old = read(source(legacy), FORMAT, ModelSourceLimits::default()).unwrap();
    let mut selected = draft(&rational_text);
    let explicit_v2 = read(source(legacy), FORMAT_V2, ModelSourceLimits::default())
        .unwrap()
        .admit(ModelLimits::default())
        .unwrap();

    // These payload fields are public. Swap their exact source correspondence
    // together, leaving each draft's original source-format selection intact.
    std::mem::swap(&mut old.source, &mut selected.source);
    std::mem::swap(&mut old.environment, &mut selected.environment);
    std::mem::swap(&mut old.roles, &mut selected.roles);
    assert_eq!(old.profile(), NativeModelProfile::V1);
    assert_eq!(selected.profile(), NativeModelProfile::V2);

    let error = old.admit(ModelLimits::default()).unwrap_err();
    let ModelSourceCause::Admission(cause) = &error.cause else {
        panic!("historical admission must refuse rational payload: {error:?}")
    };
    assert_eq!(cause.code, Code::UnsupportedConstruct);
    assert_eq!(error.source().source().text(), rational_text);

    let admitted = selected.admit(ModelLimits::default()).unwrap();
    assert_eq!(admitted.profile(), NativeModelProfile::V2);
    assert_eq!(admitted.source().source().text(), legacy);
    assert_eq!(admitted.environment(), explicit_v2.environment());
    assert_eq!(admitted.roles(), explicit_v2.roles());
    assert_eq!(admitted.artifact_bytes(), explicit_v2.artifact_bytes());
}

#[test]
#[trace("TC-120", "TC-131", "FR-041-AC-1", "FR-047-AC-8")]
fn historical_entrypoints_refuse_rational_declarations_and_roles() {
    let text = document(RATIONAL, "", AMOUNT);
    let error = read(source(&text), FORMAT, ModelSourceLimits::default()).unwrap_err();
    assert!(matches!(error.cause, ModelSourceCause::Decode(_)));
    assert_eq!(error.source().source().text(), text);
    let input = draft(&text);
    let error = NativeModel::new(
        input.source,
        input.environment,
        input.roles,
        ModelLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, Code::UnsupportedConstruct);
    let mut input = draft(&text);
    input.roles.scalars.clear();
    let error = NativeModel::new(
        input.source,
        input.environment,
        input.roles,
        ModelLimits::default(),
    )
    .unwrap_err();
    assert_eq!(
        error.code,
        Code::UnsupportedConstruct,
        "unmapped rational declarations cannot enter /1 either"
    );
    let legacy = include_str!("fixtures/native-package/model-source.json");
    let mut input = read(source(legacy), FORMAT, ModelSourceLimits::default()).unwrap();
    input.roles.scalars[0].kind = ScalarKind::Rational {
        unit: Unit::Dimensionless,
    };
    assert_eq!(
        input.admit(ModelLimits::default()).unwrap_err().code(),
        Code::UnsupportedConstruct
    );
}

#[test]
#[trace("TC-120", "FR-041-AC-3", "FR-041-AC-4")]
fn rational_integer_components_and_units_are_exact() {
    for (minimum, maximum) in [
        (i64::MIN, i64::MAX),
        (-1, 1),
        (9_007_199_254_740_991, 9_007_199_254_740_991),
        (9_007_199_254_740_992, 9_007_199_254_740_992),
        (9_007_199_254_740_993, 9_007_199_254_740_993),
    ] {
        for denominator in [1, 2, 9_223_372_036_854_775_807] {
            let scalar = format!(
                r#"{{"kind":"rational","name":"Ratio","numerator_minimum":{minimum},"numerator_maximum":{maximum},"maximum_denominator":{denominator}}}"#
            );
            let admitted = model(&document(&scalar, "", AMOUNT));
            assert_bounds(
                admitted.environment().values()[0].value_type(),
                (minimum, maximum, denominator),
            );
        }
    }
    for (suffix, unit) in [
        ("", Unit::Dimensionless),
        (",\"unit\":null", Unit::Dimensionless),
        (",\"unit\":\"metre\"", Unit::Named(symbol("metre"))),
    ] {
        let scalar = format!("{}{suffix}}}", &RATIONAL[..RATIONAL.len() - 1]);
        let admitted = model(&document(&scalar, "", AMOUNT));
        assert_eq!(
            admitted.roles().scalars[0].kind,
            ScalarKind::Rational { unit }
        );
    }
}

#[test]
#[trace("TC-120", "FR-041-AC-3")]
fn rational_wire_variant_is_closed_and_never_rounds_numbers() {
    let fields = [
        ("kind", "\"rational\""),
        ("name", "\"Ratio\""),
        ("numerator_minimum", "-1"),
        ("numerator_maximum", "1"),
        ("maximum_denominator", "2"),
    ];
    let mut invalid = vec![
        RATIONAL.replace("\"rational\"", "\"ratio\""),
        RATIONAL.replace("\"name\":\"Ratio\"", "\"name\":false"),
        RATIONAL.replace("\"name\":\"Ratio\"", "\"name\":null"),
        RATIONAL.replacen('{', "{\"unknown\":0,", 1),
        format!(
            "{},\"unit\":\"m\",\"unit\":\"m\"}}",
            &RATIONAL[..RATIONAL.len() - 1]
        ),
        "[]".into(),
        "null".into(),
    ];
    for (name, token) in fields {
        let member = format!("\"{name}\":{token}");
        invalid.push(RATIONAL.replace(&member, &format!("{member},{member}")));
        let removed = RATIONAL
            .replace(&format!("{member},"), "")
            .replace(&format!(",{member}"), "");
        assert_ne!(removed, RATIONAL);
        invalid.push(removed);
    }
    for name in [
        "numerator_minimum",
        "numerator_maximum",
        "maximum_denominator",
    ] {
        let token = match name {
            "numerator_minimum" => "-1",
            "numerator_maximum" => "1",
            _ => "2",
        };
        for wrong in [
            "null",
            "\"1\"",
            "true",
            "[]",
            "{}",
            "1.0",
            "1e0",
            "1E+2",
            "18446744073709551616",
            "-9223372036854775809",
        ] {
            invalid.push(RATIONAL.replace(
                &format!("\"{name}\":{token}"),
                &format!("\"{name}\":{wrong}"),
            ));
        }
    }
    invalid.push(RATIONAL.replace("\"maximum_denominator\":2", "\"maximum_denominator\":-1"));
    invalid.push(RATIONAL.replace(
        "\"numerator_maximum\":1",
        "\"numerator_maximum\":9223372036854775808",
    ));
    invalid.push(RATIONAL.replace(
        "\"numerator_minimum\":-1",
        "\"numerator_minimum\":9223372036854775808",
    ));
    for scalar in invalid {
        let text = document(&scalar, "", AMOUNT);
        let error = read(source(&text), FORMAT_V2, ModelSourceLimits::default()).unwrap_err();
        assert!(
            matches!(error.cause, ModelSourceCause::Decode(_)),
            "{scalar}: {error:?}"
        );
        assert_eq!(error.code(), Code::InvalidModelBinding);
        assert_eq!(error.source().source().text(), text);
        assert_eq!(
            error.source().source().digest(),
            ByteDigest::of(text.as_bytes())
        );
        assert!(!error.is_incomplete());
    }
}

#[test]
#[trace("TC-120", "TC-131", "FR-041-AC-3", "FR-041-AC-4", "FR-047-AC-8")]
fn invalid_ir_bounds_keep_the_original_scalar_occurrence() {
    for scalar in [
        RATIONAL.replace("\"maximum_denominator\":2", "\"maximum_denominator\":0"),
        RATIONAL.replace(
            "\"maximum_denominator\":2",
            "\"maximum_denominator\":9223372036854775808",
        ),
        RATIONAL.replace(
            "\"maximum_denominator\":2",
            "\"maximum_denominator\":18446744073709551615",
        ),
        RATIONAL.replace("\"numerator_minimum\":-1", "\"numerator_minimum\":2"),
    ] {
        let text = document(&scalar, "", AMOUNT).replace("\"scalars\":[", "\"scalars\":[\r\n  ");
        let error = read(source(&text), FORMAT_V2, ModelSourceLimits::default()).unwrap_err();
        let ModelSourceCause::Formal(errors) = &error.cause else {
            panic!("actual IR constructor error: {error:?}")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, ir::DiagnosticCode::InvalidNumericBounds);
        assert_occurrence(error.source(), errors[0].span.as_ref().unwrap(), &scalar);
        assert_eq!(error.source().source().text(), text);
    }
}

#[test]
#[trace("TC-120", "FR-041-AC-2", "FR-041-AC-4")]
fn wrapped_rational_sites_keep_nominal_units_and_original_loci() {
    let scalar = RATIONAL.replace("Ratio", "Ra\\u0074io").replace(
        "\"maximum_denominator\":2",
        "\"maximum_denominator\":2,\"unit\":\"metre\"",
    );
    let first =
        r#"{"name":"amount","type":{"kind":"option","value":{"kind":"scalar","name":"Ratio"}}}"#;
    let second = r#"{"name":"amount","type":{"kind":"sequence","maximum":3,"value":{"kind":"scalar","name":"Ratio"}}}"#;
    let records =
        format!(r#"{{"name":"First","fields":[{first}]}},{{"name":"Second","fields":[{second}]}}"#);
    let text = document(&scalar, &records, WRAPPED).replace("\"records\":[", "\"records\":[\r\n  ");
    let admitted = model(&text);
    let role = &admitted.roles().scalars[0];
    assert_eq!(role.name, symbol("Ratio"));
    assert_eq!(
        role.kind,
        ScalarKind::Rational {
            unit: Unit::Named(symbol("metre"))
        }
    );
    assert_eq!(
        role.sites,
        vec![
            ScalarSite::Value {
                name: symbol("amount")
            },
            ScalarSite::Field {
                record: symbol("First"),
                field: symbol("amount")
            },
            ScalarSite::Field {
                record: symbol("Second"),
                field: symbol("amount")
            }
        ]
    );
    assert_occurrence(admitted.source(), &role.source, &scalar);
    for (name, expected) in [("First", first), ("Second", second)] {
        let ir::TypeDeclaration::Record { declaration } = admitted
            .environment()
            .types()
            .iter()
            .find(|item| item.name().as_str() == name)
            .unwrap()
        else {
            panic!("record")
        };
        let field = &declaration.fields()[0];
        assert_occurrence(admitted.source(), field.source(), expected);
        match field.value_type() {
            ir::ValueType::Option { value } => assert_bounds(value, (-1, 1, 2)),
            ir::ValueType::Collection { value } => {
                assert_eq!(value.maximum_items(), 3);
                assert_bounds(value.element(), (-1, 1, 2));
            }
            _ => panic!("retained authored wrapper"),
        }
    }
    let value = &admitted.environment().values()[0];
    assert_occurrence(admitted.source(), value.source(), WRAPPED);
    let ir::ValueType::Option { value } = value.value_type() else {
        panic!("outer option")
    };
    let ir::ValueType::Collection { value } = value.as_ref() else {
        panic!("inner sequence")
    };
    assert_eq!(value.maximum_items(), 3);
    assert_bounds(value.element(), (-1, 1, 2));
}

#[test]
#[trace("TC-120", "FR-041-AC-2")]
fn every_rational_site_requires_one_consistent_role() {
    let text = document(
        RATIONAL,
        "",
        &format!("{AMOUNT},{}", AMOUNT.replace("amount", "other")),
    );
    let foreign = named_source(&text, "foreign:native", "ForeignSource");
    let foreign_span = foreign
        .to_ir(foreign.source(), Span { start: 0, end: 1 })
        .unwrap();
    for (mutation, description) in [
        "missing scalar role",
        "empty role alongside completely mapped sites",
        "duplicate scalar identity with disjoint sites",
        "repeated site within one role",
        "absent extra site alongside completely mapped sites",
        "integer role for rational sites",
        "text role for rational sites",
        "same sites assigned to different scalar identities",
        "inconsistent numerator minimum",
        "inconsistent numerator maximum",
        "inconsistent maximum denominator",
        "foreign scalar declaration locus",
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = draft(&text);
        let owner = input.environment.owner().clone();
        match mutation {
            0 => input.roles.scalars.clear(),
            1 => {
                let mut empty = input.roles.scalars[0].clone();
                empty.name = symbol("EmptyRatio");
                empty.sites.clear();
                input.roles.scalars.push(empty);
            }
            2 => {
                let mut duplicate = input.roles.scalars[0].clone();
                duplicate.sites = input.roles.scalars[0].sites.split_off(1);
                input.roles.scalars.push(duplicate);
            }
            3 => {
                let site = input.roles.scalars[0].sites[0].clone();
                input.roles.scalars[0].sites.push(site);
            }
            4 => {
                input.roles.scalars[0].sites.push(ScalarSite::Value {
                    name: symbol("absent"),
                });
            }
            5 => {
                input.roles.scalars[0].kind = ScalarKind::Integer {
                    unit: Unit::Dimensionless,
                }
            }
            6 => input.roles.scalars[0].kind = ScalarKind::Text { max_scalars: 2 },
            7 => {
                let mut role = input.roles.scalars[0].clone();
                role.name = symbol("OtherRatio");
                input.roles.scalars.push(role);
            }
            8..=10 => {
                let (minimum, maximum, denominator) = match mutation {
                    8 => (-2, 1, 2),
                    9 => (-1, 2, 2),
                    10 => (-1, 1, 3),
                    _ => unreachable!(),
                };
                let mut values = input.environment.values().to_vec();
                let original = &values[1];
                values[1] = ir::ValueDeclaration::new(
                    original.name().clone(),
                    original.kind(),
                    ir::ValueType::rational(
                        ir::RationalType::new(minimum, maximum, denominator).unwrap(),
                    ),
                    original.source().clone(),
                );
                input.environment = ir::DeclarationEnvironment::new(
                    input.environment.owner().clone(),
                    Vec::new(),
                    values,
                    Vec::new(),
                )
                .unwrap();
            }
            11 => {
                input.roles.scalars[0].source = foreign_span.clone();
            }
            _ => unreachable!(),
        }
        let error = input.admit(ModelLimits::default()).unwrap_err();
        assert_eq!(error.code(), Code::InvalidModelBinding, "{description}");
        let ModelSourceCause::Admission(cause) = &error.cause else {
            panic!("{description}: expected native admission refusal, got {error:?}")
        };
        assert_eq!(
            &cause.source,
            error.source().source().identity(),
            "{description}"
        );
        if mutation == 11 {
            let [related] = cause.related.as_slice() else {
                panic!("foreign scalar refusal must retain exactly its locus: {cause:?}")
            };
            assert_eq!(related.identity.owner, owner);
            assert_eq!(
                related.identity.key,
                DeclarationKey::Scalar(symbol("Ratio"))
            );
            assert_eq!(related.source, foreign_span);
        }
        assert_eq!(error.source().source().text(), text);
    }
    assert_eq!(model(&text).roles().scalars[0].sites.len(), 2);
}

#[test]
#[trace("TC-120", "FR-041-AC-4", "FR-041-AC-6")]
fn mixed_existing_roles_and_rational_artifact_identity_are_preserved() {
    let mut document: Value =
        serde_json::from_str(include_str!("fixtures/native-rule-model.json")).unwrap();
    document["scalars"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::from_str(RATIONAL).unwrap());
    document["values"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::from_str(AMOUNT).unwrap());
    document["enums"] = json!([{"name":"Status","variants":["Ready","Stopped"]}]);
    let text = document.to_string();
    let admitted = model(&text);
    assert_eq!(admitted.roles().scalars.len(), 8);
    assert_eq!(admitted.roles().objects[0].reference, symbol("NodeRef"));
    assert_eq!(
        admitted.roles().operations[0].result,
        Some(symbol("step_result"))
    );
    assert_eq!(
        admitted.roles().operations[0].frame.fields,
        vec![(symbol("Node"), symbol("n"))]
    );
    assert!(admitted.environment().types().iter().any(|declaration| matches!(declaration, ir::TypeDeclaration::Enum { declaration } if declaration.name().as_str() == "Status")));
    for (name, kind) in [
        ("ObjectId", ScalarKind::Text { max_scalars: 256 }),
        (
            "Distance",
            ScalarKind::Integer {
                unit: Unit::Named(symbol("metre")),
            },
        ),
    ] {
        assert_eq!(
            admitted
                .roles()
                .scalars
                .iter()
                .find(|role| role.name.as_str() == name)
                .unwrap()
                .kind,
            kind
        );
    }
    let mut reordered = draft(&text);
    reordered.roles.scalars.reverse();
    for role in &mut reordered.roles.scalars {
        role.sites.reverse();
    }
    assert_eq!(
        admitted.artifact_bytes(),
        reordered
            .admit(ModelLimits::default())
            .unwrap()
            .artifact_bytes()
    );
    let simple = document_source();
    let baseline = model(&simple);
    for changed in [
        simple.replace("\"numerator_minimum\":-1", "\"numerator_minimum\":-2"),
        simple.replace("\"maximum_denominator\":2", "\"maximum_denominator\":3"),
        simple.replace(
            "\"maximum_denominator\":2",
            "\"maximum_denominator\":2,\"unit\":\"metre\"",
        ),
        format!("{simple}\n"),
    ] {
        assert_ne!(baseline.digest(), model(&changed).digest());
    }
}

fn document_source() -> String {
    document(RATIONAL, "", AMOUNT)
}

fn historical(model: &NativeModel) -> quire_spec_language::ParsedUnit {
    let text = format!("language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"{}\" version \"1\" digest \"{}\";\ninvariant Rule on M::Node at current {{ true }}", model.environment().owner().package().as_str(), model.digest());
    parse(
        SourceIdentity {
            identity: "native:historical".into(),
            revision: "1".into(),
        },
        "historical.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap()
}

#[test]
#[trace("TC-120", "TC-131", "FR-041-AC-5", "FR-047-AC-8")]
fn historical_linking_refuses_only_selected_v2_artifacts() {
    let legacy = include_str!("fixtures/native-package/model-source.json");
    let v1 = read(source(legacy), FORMAT, ModelSourceLimits::default())
        .unwrap()
        .admit(ModelLimits::default())
        .unwrap();
    let v2_legacy = model(legacy);
    let mut extended: Value = serde_json::from_str(legacy).unwrap();
    extended["package"] = json!("test/unused");
    extended["requirement"] = json!("UnusedModel");
    extended["scalars"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::from_str(RATIONAL).unwrap());
    extended["values"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::from_str(AMOUNT).unwrap());
    let v2_rational = read(
        named_source(&extended.to_string(), "test:unused", "UnusedSource"),
        FORMAT_V2,
        ModelSourceLimits::default(),
    )
    .unwrap()
    .admit(ModelLimits::default())
    .unwrap();
    for selected in [&v2_legacy, &v2_rational] {
        let unit = historical(selected);
        let span = unit.imports()[0].span;
        let error =
            link_native(unit, std::slice::from_ref(selected), LinkLimits::default()).unwrap_err();
        assert_eq!(error.code, Code::UnsupportedConstruct);
        assert_eq!(
            (error.span.start.byte, error.span.end.byte),
            (span.start, span.end)
        );
    }
    let unit = historical(&v1);
    let supplied = [v1, v2_rational];
    let linked = link_native(unit, &supplied, LinkLimits::default()).unwrap();
    assert_eq!(linked.models().len(), 1);
    assert_eq!(
        linked.models()[0].native_model().unwrap().profile(),
        NativeModelProfile::V1
    );
}

fn with_namespace(text: &str, inspect: impl FnOnce(&SyntaxNamespace)) {
    let sources = [Source::read(
        SourceIdentity {
            identity: "native:composed".into(),
            revision: "1".into(),
        },
        "composed.native",
        text.as_bytes(),
        1_048_576,
    )
    .unwrap()];
    let inventory = SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: vec![ExpectedSource {
            authority: "native:composed".into(),
            identity: sources[0].identity().clone(),
            digest: sources[0].digest(),
        }],
    };
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(report.issues().is_empty(), "{:?}", report.issues());
    inspect(report.namespace().unwrap());
}

fn parameter(namespace: &SyntaxNamespace, index: usize) -> (UnitId, &QualifiedName) {
    let id = namespace.lookup("Rule")[0];
    let DeclarationKind::Predicate { parameters, .. } = &namespace.syntax(id).unwrap().kind else {
        panic!("predicate")
    };
    let ParameterType::Model(name) = &parameters[index].ty else {
        panic!("model parameter")
    };
    (namespace.declaration(id).unwrap().unit(), name)
}

#[test]
#[trace("TC-120", "FR-041-AC-5", "FR-041-AC-6")]
fn composed_exports_retain_rational_roles_and_require_exact_native_artifacts() {
    let record = r#"{"name":"Measure","fields":[{"name":"amount","type":{"kind":"scalar","name":"Ratio"}}]}"#;
    let admitted = model(&document(RATIONAL, record, AMOUNT));
    let program = format!("language \"ix:native\" edition \"1-draft\";\nprofile S = \"test:unresolved\" version \"1\" digest \"unresolved\";\nmodel M = \"test/rational\" version \"1\" digest \"{}\";\npredicate Rule using S (amount: M::Ratio, item: M::Measure): Boolean {{ true }}", admitted.digest());
    let inputs = [ModelInput::Native(&admitted)];
    with_namespace(&program, |namespace| {
        let mut work = Work::new(BindingLimits::default());
        let bindings = bind_models(namespace, &inputs, &mut work);
        assert!(bindings.complete());
        let (unit, name) = parameter(namespace, 0);
        let scalar = bindings.resolve_type(unit, name, &mut work).unwrap();
        assert_eq!(scalar.model().profile(), NativeModelProfile::V2);
        assert_eq!(
            scalar.location().identity.owner,
            *admitted.environment().owner()
        );
        assert_eq!(
            scalar.location().identity.key,
            DeclarationKey::Scalar(symbol("Ratio"))
        );
        assert_eq!(scalar.location().source, admitted.roles().scalars[0].source);
        let NativeType::Scalar {
            representation,
            role,
            ..
        } = scalar.native()
        else {
            panic!("nominal scalar")
        };
        assert_bounds(representation, (-1, 1, 2));
        assert_eq!(
            role.kind,
            ScalarKind::Rational {
                unit: Unit::Dimensionless
            }
        );
        let (_, name) = parameter(namespace, 1);
        let record = bindings.resolve_type(unit, name, &mut work).unwrap();
        let field = bindings
            .field(
                unit,
                &record,
                &Spanned {
                    value: "amount".into(),
                    span: name.name.span,
                },
                &mut work,
            )
            .unwrap();
        assert_eq!(field.native(), scalar.native());
        assert_eq!(
            field.location().identity.key,
            DeclarationKey::Field {
                record: symbol("Measure"),
                field: symbol("amount")
            }
        );
    });
    let canonical = admitted
        .environment()
        .canonical_declaration(ir::CanonicalProfile::V1)
        .unwrap();
    for digest in [
        ByteDigest::of(b"stale artifact"),
        admitted.source().source().digest(),
        ByteDigest::of(canonical.bytes().as_slice()),
    ] {
        assert_ne!(digest, admitted.digest());
        with_namespace(
            &program.replace(&admitted.digest().to_string(), &digest.to_string()),
            |namespace| {
                let bindings =
                    bind_models(namespace, &inputs, &mut Work::new(BindingLimits::default()));
                assert_eq!(
                    bindings.imports()[0].selection,
                    Err(ImportRefusal::StaleSelection)
                );
            },
        );
    }
    with_namespace(
        &program.replace("test/rational", "test/foreign"),
        |namespace| {
            let bindings =
                bind_models(namespace, &inputs, &mut Work::new(BindingLimits::default()));
            assert_eq!(
                bindings.imports()[0].selection,
                Err(ImportRefusal::MissingPackage)
            );
        },
    );
    with_namespace(&program.replace("M::Ratio", "M::Missing"), |namespace| {
        let mut work = Work::new(BindingLimits::default());
        let bindings = bind_models(namespace, &inputs, &mut work);
        let (unit, name) = parameter(namespace, 0);
        assert_eq!(
            bindings
                .resolve_type(unit, name, &mut work)
                .unwrap_err()
                .kind,
            ModelErrorKind::MissingExport
        );
    });
}

#[test]
#[trace("TC-120", "FR-041-AC-7")]
fn rational_frontend_limits_count_authored_entries_and_nested_levels() {
    let text = document(RATIONAL, "", WRAPPED);
    // One scalar + one value entry; Option -> Sequence -> Scalar is depth three.
    let exact = ModelSourceLimits {
        source_bytes: text.len(),
        entries: 2,
        type_depth: 3,
    };
    for limits in [
        ModelSourceLimits {
            source_bytes: 0,
            ..exact
        },
        ModelSourceLimits {
            source_bytes: text.len() - 1,
            ..exact
        },
        ModelSourceLimits {
            entries: 0,
            ..exact
        },
        ModelSourceLimits {
            entries: 1,
            ..exact
        },
        ModelSourceLimits {
            type_depth: 0,
            ..exact
        },
        ModelSourceLimits {
            type_depth: 2,
            ..exact
        },
    ] {
        let error = read(source(&text), FORMAT_V2, limits).unwrap_err();
        assert_eq!(error.code(), Code::ResourceExhausted);
        assert!(error.is_incomplete());
        assert_eq!(error.source_limits(), limits);
        assert_eq!(error.source().source().text(), text);
    }
    let exact_draft = read(source(&text), FORMAT_V2, exact).unwrap();
    assert_eq!(exact_draft.profile(), NativeModelProfile::V2);
    exact_draft.admit(ModelLimits::default()).unwrap();
    let above = ModelSourceLimits {
        source_bytes: usize::MAX,
        entries: usize::MAX,
        type_depth: usize::MAX,
    };
    assert_eq!(
        read(source(&text), FORMAT_V2, above).unwrap().source_limits,
        ModelSourceLimits::default()
    );
}

#[test]
#[trace("TC-120", "FR-041-AC-2", "FR-041-AC-4", "FR-041-AC-5")]
fn equal_rational_representations_preserve_distinct_names_and_owners() {
    let text = document(
        &format!("{RATIONAL},{}", RATIONAL.replace("Ratio", "OtherRatio")),
        "",
        &format!(
            "{AMOUNT},{}",
            AMOUNT
                .replace("amount", "other")
                .replace("Ratio", "OtherRatio")
        ),
    );
    let first = model(&text);
    let second_text = text
        .replace("test/rational", "test/second")
        .replace("RationalModel", "SecondModel");
    let second = read(
        named_source(&second_text, "test:second", "SecondSource"),
        FORMAT_V2,
        ModelSourceLimits::default(),
    )
    .unwrap()
    .admit(ModelLimits::default())
    .unwrap();
    let program = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile S = \"test:unresolved\" version \"1\" digest \"unresolved\";\nmodel M = \"test/rational\" version \"1\" digest \"{}\";\nmodel N = \"test/second\" version \"1\" digest \"{}\";\npredicate Rule using S (firstValue: M::Ratio, other: M::OtherRatio, secondValue: N::Ratio): Boolean {{ true }}", first.digest(), second.digest()
    );
    let inputs = [ModelInput::Native(&first), ModelInput::Native(&second)];
    with_namespace(&program, |namespace| {
        let mut work = Work::new(BindingLimits::default());
        let bindings = bind_models(namespace, &inputs, &mut work);
        let resolved: Vec<_> = (0..3)
            .map(|index| {
                let (unit, name) = parameter(namespace, index);
                bindings.resolve_type(unit, name, &mut work).unwrap()
            })
            .collect();
        for value in &resolved {
            let NativeType::Scalar { representation, .. } = value.native() else {
                panic!("rational scalar")
            };
            assert_bounds(representation, (-1, 1, 2));
        }
        assert_ne!(resolved[0].native(), resolved[1].native());
        assert_ne!(resolved[0].native(), resolved[2].native());
        assert_eq!(
            resolved[0].location().identity.owner,
            resolved[1].location().identity.owner
        );
        assert_ne!(
            resolved[0].location().identity.owner,
            resolved[2].location().identity.owner
        );
        assert_ne!(
            resolved[0].location().source.source(),
            resolved[2].location().source.source()
        );
    });
}

#[test]
#[trace("TC-120", "FR-041-AC-7")]
fn rational_admission_limits_charge_roles_sites_and_ir_wrapper_nodes() {
    let text = document(RATIONAL, "", WRAPPED);
    // One value declaration plus Option, Collection and Rational nodes.
    let exact = ModelLimits {
        roles: 1,
        entries: 1,
        nodes: 4,
        depth: 3,
        ..ModelLimits::default()
    };
    for limits in [
        ModelLimits { roles: 0, ..exact },
        ModelLimits {
            entries: 0,
            ..exact
        },
        ModelLimits { nodes: 0, ..exact },
        ModelLimits { nodes: 3, ..exact },
        ModelLimits { depth: 0, ..exact },
        ModelLimits { depth: 2, ..exact },
    ] {
        let error = draft(&text).admit(limits).unwrap_err();
        assert_eq!(error.code(), Code::ResourceExhausted);
        assert!(error.is_incomplete());
        assert!(matches!(error.cause, ModelSourceCause::Admission(_)));
    }
    assert_eq!(
        draft(&text).admit(exact).unwrap().profile(),
        NativeModelProfile::V2
    );
    for roles in [true, false] {
        let mut input = draft(&text);
        if roles {
            input.roles.scalars = vec![input.roles.scalars[0].clone(); 10_001];
        } else {
            input.roles.scalars[0].sites = vec![input.roles.scalars[0].sites[0].clone(); 10_001];
        }
        let error = input
            .admit(ModelLimits {
                roles: usize::MAX,
                entries: usize::MAX,
                ..ModelLimits::default()
            })
            .unwrap_err();
        assert_eq!(
            error.code(),
            Code::ResourceExhausted,
            "hard ceiling precedes duplicate checking"
        );
    }
}

#[test]
#[trace("TC-120", "FR-041-AC-6", "FR-041-AC-7")]
fn rational_output_limits_use_independently_assembled_artifact_content() {
    let text = document_source();
    let input = draft(&text);
    let canonical = input
        .environment
        .canonical_declaration(ir::CanonicalProfile::V1)
        .unwrap();
    let declarations = std::str::from_utf8(canonical.bytes().as_slice()).unwrap();
    // This oracle names every expected wire member; it never asks NativeModel
    // for its output length or serializes NativeRoles through the producer.
    let expected = json!({
        "profile":"native-state-model/2", "declarations": declarations,
        "source": {"identity":"test:rule-model", "revision":"1", "digest":ByteDigest::of(text.as_bytes()).to_string(), "formal":input.source.identity()},
        "loci":[{"key":{"kind":"value","name":"amount"},"source":input.environment.values()[0].source()}],
        "roles":{"scalars":[{"name":"Ratio","source":input.roles.scalars[0].source,"kind":{"kind":"rational","unit":"dimensionless"},"sites":[{"kind":"value","name":"amount"}]}],"objects":[],"operations":[]}
    });
    let expected_bytes = serde_json::to_vec(&expected).unwrap().len();
    for maximum in [
        0,
        canonical.bytes().as_slice().len() - 1,
        expected_bytes - 1,
        expected_bytes,
    ] {
        let result = draft(&text).admit(ModelLimits {
            artifact_bytes: maximum,
            ..ModelLimits::default()
        });
        if maximum < expected_bytes {
            let error = result.unwrap_err();
            assert_eq!(error.code(), Code::ResourceExhausted);
            assert!(error.is_incomplete());
            if maximum == canonical.bytes().as_slice().len() - 1 {
                let ModelSourceCause::Admission(cause) = &error.cause else {
                    panic!("admission budget")
                };
                assert_eq!(
                    cause.upstream.as_ref().unwrap().code,
                    ir::DiagnosticCode::CanonicalizationResourceExhausted
                );
            }
        } else {
            let admitted = result.unwrap();
            assert_eq!(admitted.artifact_bytes().len(), expected_bytes);
            assert_eq!(
                serde_json::from_slice::<Value>(admitted.artifact_bytes()).unwrap(),
                expected
            );
        }
    }
}
