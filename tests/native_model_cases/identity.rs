// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-042: bounded semantic mutation and permutation families over real models.

use super::*;
use ix_trace_rs::trace;
use quire_spec_language::{formal_source::FormalSource, Source, SourceIdentity};

#[test]
#[trace("TC-042", "FR-015-AC-3")]
fn tc_042_generated_semantic_mutations_bind_the_changed_payload() {
    let baseline = authored(|_| {}).model();
    let original: Value = serde_json::from_slice(baseline.artifact_bytes()).unwrap();
    let formal: Value = serde_json::from_str(original["declarations"].as_str().unwrap()).unwrap();
    let node = formal["value"]["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["declaration"]["name"] == "Node")
        .unwrap();
    let wide = node["declaration"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "wide")
        .unwrap();
    assert_eq!(
        wide["value_type"]["value"]["minimum"].as_i64(),
        Some(i64::MIN)
    );
    assert_eq!(
        wide["value_type"]["value"]["maximum"].as_i64(),
        Some(i64::MAX)
    );
    // Each case regenerates the real producer's source loci. Assert the affected
    // semantic payload as well as digest sensitivity, so changed source bytes
    // alone cannot disguise an omitted field in the artifact.
    let mutations: &[fn(&mut Value)] = &[
        |d| {
            d["scalars"][0]["name"] = json!("Edition");
            d["records"][0]["fields"][0]["type"]["name"] = json!("Edition");
            d["records"][0]["fields"][9]["type"]["value"]["name"] = json!("Edition");
        },
        |d| d["scalars"][4]["unit"] = json!("kilometre"),
        |d| d["objects"][0]["universe"] = json!("other_nodes"),
        |d| d["operations"][0]["anchor"] = json!("commit"),
        |d| d["operations"][0]["frame"]["created"] = json!(["Node"]),
        |d| d["scalars"][0]["maximum"] = json!(999),
        |d| {
            d["values"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"unused", "kind":"state", "type":{"kind":"boolean"}}))
        },
    ];
    for (case, mutate) in mutations.iter().enumerate() {
        let model = authored(mutate).model();
        let artifact: Value = serde_json::from_slice(model.artifact_bytes()).unwrap();
        assert_ne!(model.digest(), baseline.digest(), "case {case}");
        let declarations: Value =
            serde_json::from_str(artifact["declarations"].as_str().unwrap()).unwrap();
        match case {
            0 => assert!(artifact["roles"]["scalars"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["name"] == "Edition")),
            1 => assert!(artifact["roles"]["scalars"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["kind"]["unit"] == json!({"named":"kilometre"}))),
            2 => assert_eq!(artifact["roles"]["objects"][0]["universe"], "other_nodes"),
            3 => assert_eq!(artifact["roles"]["operations"][0]["anchor"], "commit"),
            4 => assert_eq!(
                artifact["roles"]["operations"][0]["frame"]["created"],
                json!(["Node"])
            ),
            5 => {
                assert_ne!(artifact["declarations"], original["declarations"]);
                let node = declarations["value"]["types"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["declaration"]["name"] == "Node")
                    .unwrap();
                let field = node["declaration"]["fields"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|f| f["name"] == "n")
                    .unwrap();
                assert_eq!(field["value_type"]["value"]["maximum"], 999);
            }
            6 => {
                assert_ne!(artifact["declarations"], original["declarations"]);
                let unused = declarations["value"]["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["name"] == "unused")
                    .unwrap();
                assert_eq!(unused["kind"], "state");
                assert_eq!(unused["value_type"], json!({"kind":"boolean"}));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
#[trace("TC-042", "FR-015-AC-3")]
fn tc_042_set_inventories_are_permutation_invariant_but_parameters_are_ordered() {
    let baseline = extra_inventory().model();
    for rotation in 0..6 {
        let mut input = extra_inventory();
        let mut types = input.environment.types().to_vec();
        let count = types.len();
        types.rotate_left(rotation % count);
        for ty in &mut types {
            *ty = match ty {
                ir::TypeDeclaration::Record { declaration } => {
                    let mut fields = declaration.fields().to_vec();
                    fields.reverse();
                    ir::TypeDeclaration::Record {
                        declaration: ir::RecordDeclaration::new(
                            declaration.name().clone(),
                            declaration.source().clone(),
                            fields,
                        )
                        .unwrap(),
                    }
                }
                ir::TypeDeclaration::Enum { declaration } => {
                    let mut variants = declaration.variants().to_vec();
                    variants.reverse();
                    ir::TypeDeclaration::Enum {
                        declaration: ir::EnumDeclaration::new(
                            declaration.name().clone(),
                            declaration.source().clone(),
                            variants,
                        )
                        .unwrap(),
                    }
                }
            };
        }
        let mut values = input.environment.values().to_vec();
        let count = values.len();
        values.rotate_left(rotation % count);
        input.environment = ir::DeclarationEnvironment::new(
            input.environment.owner().clone(),
            types,
            values,
            Vec::new(),
        )
        .unwrap();
        let count = input.roles.scalars.len();
        input.roles.scalars.rotate_left(rotation % count);
        input.roles.objects.reverse();
        input.roles.operations.reverse();
        for scalar in &mut input.roles.scalars {
            scalar.sites.reverse();
        }
        for operation in &mut input.roles.operations {
            operation.frame.fields.reverse();
            operation.frame.created.reverse();
            operation.frame.deleted.reverse();
        }
        assert_eq!(
            input.model().artifact_bytes(),
            baseline.artifact_bytes(),
            "rotation {rotation}"
        );
    }
    let mut reordered = extra_inventory();
    reordered.roles.operations[0].parameters.reverse();
    let reordered = reordered.model();
    assert_ne!(reordered.digest(), baseline.digest());
    let step = reordered
        .roles()
        .operations
        .iter()
        .find(|o| o.name.as_str() == "step")
        .unwrap();
    assert_eq!(step.parameters, vec![symbol("first"), symbol("second")]);
    let artifact: Value = serde_json::from_slice(reordered.artifact_bytes()).unwrap();
    let step = artifact["roles"]["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["name"] == "step")
        .unwrap();
    assert_eq!(step["parameters"], json!(["first", "second"]));
}

#[test]
#[trace("TC-042", "FR-015-AC-3")]
fn tc_042_native_source_labels_and_formal_owner_participate_in_identity() {
    let baseline = parts().model();
    for (identity, revision) in [
        ("test:other-model", "draft:1"),
        ("test:rule-model", "draft:2"),
    ] {
        let mut input = parts();
        let native = Source::read(
            SourceIdentity {
                identity: identity.into(),
                revision: revision.into(),
            },
            "native-rule-model.json",
            native_rule_model::FIXTURE.as_bytes(),
            1_048_576,
        )
        .unwrap();
        input.source = FormalSource::new(native, input.source.identity().clone());
        let model = input.model();
        assert_ne!(model.digest(), baseline.digest());
        let artifact: Value = serde_json::from_slice(model.artifact_bytes()).unwrap();
        assert_eq!(artifact["source"]["identity"], identity);
        assert_eq!(artifact["source"]["revision"], revision);
    }
    let mut input = parts();
    let owner = ir::RequirementRef::new(
        ir::PackageId::new("example/other").unwrap(),
        ir::RequirementId::new("OtherModel").unwrap(),
        ir::RequirementRevision::new(2).unwrap(),
    );
    input.environment = ir::DeclarationEnvironment::new(
        owner,
        input.environment.types().to_vec(),
        input.environment.values().to_vec(),
        Vec::new(),
    )
    .unwrap();
    assert_ne!(input.model().digest(), baseline.digest());
}
