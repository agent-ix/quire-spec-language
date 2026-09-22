// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-015: complete native admission through public IR and source-bound models.

#[path = "../native_model_cases/admission.rs"]
mod admission;
#[path = "../native_model_cases/identity.rs"]
mod identity;
#[path = "../native_model_cases/limits.rs"]
mod limits;
use crate::support::native_rule_model;
#[path = "../native_model_cases/provenance.rs"]
mod provenance;

use native_rule_model::{parts, symbol, Parts};
use quire_contract_ir as ir;
use quire_spec_language::native_model::{
    ModelLimits, NativeModel, ScalarKind, ScalarRole, ScalarSite, Unit,
};
use quire_spec_language::{Code, Diagnostic, Phase};
use serde_json::{json, Value};

fn authored(change: impl FnOnce(&mut Value)) -> Parts {
    let mut data = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    change(&mut data);
    let text = serde_json::to_string_pretty(&data).unwrap();
    native_rule_model::from_text(&text, "qualification.json", "draft:1")
        .expect("source-derived IR setup must succeed before the native judgment")
}

fn scalar<'a>(input: &'a mut Parts, name: &str) -> &'a mut ScalarRole {
    input
        .roles
        .scalars
        .iter_mut()
        .find(|s| s.name.as_str() == name)
        .unwrap()
}

fn replace_record(
    input: &mut Parts,
    name: &str,
    change: impl FnOnce(&ir::RecordDeclaration) -> ir::RecordDeclaration,
) {
    let mut types = input.environment.types().to_vec();
    let index = types
        .iter()
        .position(|d| d.name().as_str() == name)
        .unwrap();
    let ir::TypeDeclaration::Record { declaration } = &types[index] else {
        panic!("record setup")
    };
    types[index] = ir::TypeDeclaration::Record {
        declaration: change(declaration),
    };
    input.environment = ir::DeclarationEnvironment::new(
        input.environment.owner().clone(),
        types,
        input.environment.values().to_vec(),
        input.environment.functions().to_vec(),
    )
    .expect("mutated IR record must be valid before native admission");
}

fn replace_field_type(input: &mut Parts, record: &str, field: &str, ty: ir::ValueType) {
    replace_record(input, record, |record| {
        let fields = record
            .fields()
            .iter()
            .map(|f| {
                if f.name().as_str() == field {
                    ir::RecordFieldDeclaration::new(
                        f.name().clone(),
                        ty.clone(),
                        f.source().clone(),
                    )
                } else {
                    f.clone()
                }
            })
            .collect();
        ir::RecordDeclaration::new(record.name().clone(), record.source().clone(), fields).unwrap()
    });
}

fn add_value(input: &mut Parts, name: &str, kind: ir::ValueDeclarationKind, ty: ir::ValueType) {
    let mut values = input.environment.values().to_vec();
    values.push(ir::ValueDeclaration::new(
        symbol(name),
        kind,
        ty,
        values[0].source().clone(),
    ));
    input.environment = ir::DeclarationEnvironment::new(
        input.environment.owner().clone(),
        input.environment.types().to_vec(),
        values,
        input.environment.functions().to_vec(),
    )
    .expect("added IR value must be valid before native admission");
}

fn refuse(input: Parts, code: Code) -> Box<Diagnostic> {
    let native = input.source.source().clone();
    let error = NativeModel::new(
        input.source,
        input.environment,
        input.roles,
        ModelLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, code, "{error:?}");
    assert_eq!(error.phase, Phase::Link);
    assert_eq!(error.is_incomplete(), code == Code::ResourceExhausted);
    assert_eq!(error.source, *native.identity());
    assert_eq!(error.path, native.path());
    assert_eq!((error.span.start.byte, error.span.end.byte), (0, 0));
    // Admission consumes only its argument; an adverse call cannot poison reuse.
    assert_eq!(
        parts().model().environment().owner().requirement().as_str(),
        "RuleModel"
    );
    error
}

fn field_site(record: &str, field: &str) -> ScalarSite {
    ScalarSite::Field {
        record: symbol(record),
        field: symbol(field),
    }
}

fn integer(min: i64, max: i64) -> ir::ValueType {
    ir::ValueType::integer(
        ir::IntegerType::new(
            ir::IntegerDomain::Signed,
            min,
            max,
            ir::OverflowPolicy::Reject,
        )
        .unwrap(),
    )
}

fn extra_inventory() -> Parts {
    authored(|data| {
        data["enums"] = json!([{ "name": "Flag", "variants": ["Off", "On"] }]);
        data["records"].as_array_mut().unwrap().extend([
            json!({"name":"Aux", "fields":[{"name":"enabled", "type":{"kind":"boolean"}}]}),
            json!({"name":"AuxRef", "fields":[{"name":"key", "type":{"kind":"scalar", "name":"ObjectId"}}]}),
            json!({"name":"Unused", "fields":[{"name":"spare", "type":{"kind":"boolean"}}]}),
        ]);
        data["objects"].as_array_mut().unwrap().push(json!({"record":"Aux", "reference":"AuxRef", "identity_field":"key", "universe":"auxiliary"}));
        data["values"].as_array_mut().unwrap().extend([
            json!({"name":"first", "kind":"input", "type":{"kind":"boolean"}}),
            json!({"name":"second", "kind":"input", "type":{"kind":"boolean"}}),
            json!({"name":"unused", "kind":"state", "type":{"kind":"enum", "name":"Flag"}}),
        ]);
        data["operations"][0]["parameters"] = json!(["second", "first"]);
        data["operations"][0]["frame"] = json!({"fields":[["Node","n"],["Aux","enabled"]], "created":["Node","Aux"], "deleted":["Aux","Node"]});
        data["operations"].as_array_mut().unwrap().push(json!({"name":"reset", "context":"Aux", "anchor":"reset", "parameters":[], "result":null, "frame":{"fields":[], "created":[], "deleted":[]}}));
    })
}
