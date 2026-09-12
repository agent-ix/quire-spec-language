// SPDX-License-Identifier: AGPL-3.0-only
//! Real source/model/definition inputs shared by the composed type controls.

use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::binding;
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::linking::composed::definitions::{Artifact, Inventory, RuleInput};
use quire_spec_language::linking::composed::models::ModelInput;
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, SourceInventory, WorkLimits,
};
use quire_spec_language::model_source::{self, ModelSourceLimits};
use quire_spec_language::native_model::{ModelLimits, NativeModel, NativeModelProfile};
use quire_spec_language::{ByteDigest, Limits, Source, SourceIdentity};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[allow(
    dead_code,
    reason = "Shared fixture module; each test binary selects its own constructor"
)]
pub fn model(name: &str) -> NativeModel {
    model_with_maximum(name, 5)
}

#[allow(
    dead_code,
    reason = "Shared fixture module; each test binary selects its own constructor"
)]
pub fn model_with_maximum(name: &str, maximum: u32) -> NativeModel {
    try_model_with_maximum(name, maximum).expect("admitted model fixture")
}

#[allow(
    dead_code,
    reason = "Shared fixture module; each test binary selects its own constructor"
)]
pub fn try_model_with_maximum(
    name: &str,
    maximum: u32,
) -> Result<NativeModel, Box<model_source::ModelSourceError>> {
    try_model_with(name, maximum, |_| {})
}

/// The shared fixture already declares the full signed-64 integer scalar `Wide`,
/// but every rational domain in it is narrow. This variant adds `Exact`, the
/// rational domain at the denominator ceiling `ir::RationalType::new` admits, so
/// a literal can reach both signed-64 endpoints and that ceiling.
#[allow(
    dead_code,
    reason = "Only numeric-boundary consumers need this shared fixture variant"
)]
pub fn model_with_signed64_domains(name: &str) -> NativeModel {
    try_model_with(name, 5, |document| {
        document["scalars"].as_array_mut().unwrap().push(
            json!({"name":"Exact", "kind":"rational", "numerator_minimum": i64::MIN, "numerator_maximum": i64::MAX, "maximum_denominator": i64::MAX}),
        );
        // Model admission requires a declaration site for every scalar role.
        document["records"][0]["fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"exact", "type":{"kind":"scalar", "name":"Exact"}}));
    })
    .expect("admitted signed-64 domain model fixture")
}

#[allow(
    dead_code,
    reason = "Only admitted state-evaluation fixtures need this compact query record"
)]
pub fn model_with_query_input(name: &str, maximum: u32) -> NativeModel {
    try_model_with(name, maximum, |document| {
        document["records"].as_array_mut().unwrap().push(json!({
            "name":"QueryInput",
            "fields":[{
                "name":"amounts",
                "type":{
                    "kind":"sequence",
                    "maximum":maximum,
                    "value":{"kind":"scalar", "name":"Amount"}
                }
            }]
        }));
    })
    .expect("admitted compact query-input model fixture")
}

#[allow(
    dead_code,
    reason = "Only admitted state-evaluation fixtures need this compact graph role"
)]
pub fn model_with_graph_input(name: &str) -> NativeModel {
    try_model_with(name, 5, |document| {
        document["records"].as_array_mut().unwrap().extend([
            json!({
                "name":"GraphNode",
                "fields":[{
                    "name":"links",
                    "type":{
                        "kind":"sequence",
                        "maximum":5,
                        "value":{"kind":"record", "name":"GraphNodeRef"}
                    }
                }]
            }),
            json!({
                "name":"GraphNodeRef",
                "fields":[{
                    "name":"id",
                    "type":{"kind":"scalar", "name":"ObjectId"}
                }]
            }),
        ]);
        document["objects"].as_array_mut().unwrap().push(json!({
            "record":"GraphNode",
            "reference":"GraphNodeRef",
            "identity_field":"id",
            "universe":"graph_nodes"
        }));
    })
    .expect("admitted compact graph model fixture")
}

fn try_model_with(
    name: &str,
    maximum: u32,
    extend: impl FnOnce(&mut Value),
) -> Result<NativeModel, Box<model_source::ModelSourceError>> {
    let mut document: Value =
        serde_json::from_str(include_str!("../../fixtures/native-rule-model.json")).unwrap();
    document["package"] = json!(format!("test/{name}"));
    document["scalars"].as_array_mut().unwrap().extend([
        json!({"name":"Q", "kind":"rational", "numerator_minimum":-1, "numerator_maximum":1, "maximum_denominator":2}),
        json!({"name":"Measured", "kind":"rational", "numerator_minimum":-1, "numerator_maximum":1, "maximum_denominator":2, "unit":"metre"}),
        json!({"name":"Twin", "kind":"rational", "numerator_minimum":-1, "numerator_maximum":1, "maximum_denominator":2}),
        json!({"name":"Amount", "kind":"integer", "minimum":1, "maximum":20, "unit":"U"}),
        json!({"name":"Total", "kind":"integer", "minimum":0, "maximum":100, "unit":"U"}),
        json!({"name":"Tally", "kind":"integer", "minimum":0, "maximum":5}),
        json!({"name":"Label", "kind":"text", "max_scalars":3}),
    ]);
    document["enums"] = json!([{"name":"Mode", "variants":["Ready","Paused"]}]);
    document["records"][0]["fields"].as_array_mut().unwrap().extend([
        json!({"name":"fraction", "type":{"kind":"scalar", "name":"Q"}}),
        json!({"name":"measured", "type":{"kind":"scalar", "name":"Measured"}}),
        json!({"name":"twin", "type":{"kind":"scalar", "name":"Twin"}}),
        json!({"name":"amount", "type":{"kind":"scalar", "name":"Amount"}}),
        json!({"name":"amounts", "type":{"kind":"sequence", "maximum":maximum, "value":{"kind":"scalar", "name":"Amount"}}}),
        json!({"name":"total", "type":{"kind":"scalar", "name":"Total"}}),
        json!({"name":"tally", "type":{"kind":"scalar", "name":"Tally"}}),
        json!({"name":"label", "type":{"kind":"scalar", "name":"Label"}}),
        json!({"name":"optionalQ", "type":{"kind":"option", "value":{"kind":"scalar", "name":"Q"}}}),
        json!({"name":"mode", "type":{"kind":"enum", "name":"Mode"}}),
        json!({"name":"plain", "type":{"kind":"record", "name":"Plain"}}),
    ]);
    document["records"].as_array_mut().unwrap().push(json!({
        "name":"Plain", "fields":[{"name":"ready", "type":{"kind":"boolean"}}]
    }));
    document["values"].as_array_mut().unwrap().push(json!({
        "name":"delta", "kind":"input", "type":{"kind":"scalar", "name":"Signed"}
    }));
    document["operations"][0]["parameters"] = json!(["delta"]);
    extend(&mut document);
    let text = serde_json::to_string_pretty(&document).unwrap();
    let source = Source::read(
        SourceIdentity {
            identity: format!("model:{name}"),
            revision: "authored".into(),
        },
        format!("{name}.json"),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .unwrap();
    let formal = FormalSource::new(
        source,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new(format!("{name}Model")).unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    let draft = model_source::read(
        formal,
        model_source::FORMAT_V2,
        ModelSourceLimits::default(),
    )?;
    let model = draft.admit(ModelLimits::default())?;
    assert_eq!(model.profile(), NativeModelProfile::V2);
    Ok(model)
}

pub fn source(name: &str, model: &NativeModel, body: &str) -> Source {
    let mut text = "language \"ix:native\" edition \"1-draft\";\n".to_owned();
    for (alias, profile) in [
        ("C", R::StateCore),
        ("S", R::StateQueries),
        ("G", R::StateGraph),
        ("T", R::EventPosition),
        ("F", R::FixedSample),
        ("W", R::TimestampedWindow),
        ("P", R::Protocol),
    ] {
        let selection = profile.selection();
        text.push_str(&format!(
            "profile {alias} = \"{}\" version \"{}\" digest \"{}\";\n",
            selection.identity, selection.revision, selection.digest
        ));
    }
    text.push_str(&format!(
        "model M = \"{}\" version \"{}\" digest \"{}\";\n{body}",
        model.environment().owner().package().as_str(),
        model.environment().owner().revision().get(),
        model.digest()
    ));
    Source::read(
        SourceIdentity {
            identity: format!("unit:{name}"),
            revision: "authored".into(),
        },
        format!("{name}.native"),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .unwrap()
}

pub fn formal_sources(sources: &[Source]) -> Vec<FormalSource> {
    sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            FormalSource::new(
                source.clone(),
                ir::SourceIdentity::new(
                    ir::SourceDocumentId::new(format!("NativeTypes{index}")).unwrap(),
                    ir::SourceRevision::new(1).unwrap(),
                ),
            )
        })
        .collect()
}

pub fn with_binding(
    sources: &[Source],
    models: &[&NativeModel],
    limits: BindingLimits,
    test: impl FnOnce(&binding::Report<'_>),
) {
    let selected_sources = SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: sources
            .iter()
            .map(|source| ExpectedSource {
                authority: source.identity().identity.clone(),
                identity: source.identity().clone(),
                digest: source.digest(),
            })
            .collect(),
    };
    let admitted = admit_namespace(
        &selected_sources,
        sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(admitted.issues().is_empty(), "{:?}", admitted.issues());
    assert!(admitted.exhaustion().is_none());
    let definitions: Vec<_> = R::all()
        .iter()
        .map(|definition| Artifact {
            selection: definition.selection(),
            bytes: definition.bytes(),
        })
        .collect();
    let rules: Vec<_> = R::all()
        .iter()
        .flat_map(|definition| definition.rules())
        .map(|rule| {
            (
                rule.path,
                RuleInput {
                    path: rule.path,
                    digest: ByteDigest::of(rule.bytes),
                    bytes: rule.bytes,
                },
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect();
    let definitions = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let inputs: Vec<_> = models
        .iter()
        .map(|model| ModelInput::Native(model))
        .collect();
    let report = binding::bind(admitted.namespace().unwrap(), &definitions, &inputs, limits);
    test(&report);
}
