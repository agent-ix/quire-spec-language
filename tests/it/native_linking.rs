// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-015: native model selection and explicit reference/operation correspondence.

use crate::support::native_rule_model;

use ix_trace_rs::trace;
use native_rule_model::{parts, symbol, Parts};
use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::{DeclarationKey, ResolutionTarget};
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::{
    link, link_native, parse, ByteDigest, Code, Limits, LinkLimits, ParsedUnit, Phase, Source,
    SourceIdentity,
};

fn read(text: &str) -> ParsedUnit {
    parse(
        SourceIdentity {
            identity: "test:native-linking".into(),
            revision: "draft:1".into(),
        },
        "native-linking.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("valid native syntax before testing linkage")
}

fn document(model: &NativeModel, digest: &str, clauses: &str) -> String {
    format!(
        "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = {} version \"{}\" digest {};\n{clauses}\n",
        serde_json::to_string(model.environment().owner().package().as_str()).unwrap(),
        model.environment().owner().revision().get(),
        serde_json::to_string(digest).unwrap(),
    )
}

fn unit(model: &NativeModel, clauses: &str) -> ParsedUnit {
    read(&document(model, &model.digest().to_string(), clauses))
}

fn invariant(expression: &str) -> String {
    format!("invariant Rule on M::Node at current {{ {expression} }}")
}

fn reowner(input: &mut Parts, requirement: &str) {
    let old = &input.environment;
    input.environment = ir::DeclarationEnvironment::new(
        ir::RequirementRef::new(
            old.owner().package().clone(),
            ir::RequirementId::new(requirement).unwrap(),
            old.owner().revision(),
        ),
        old.types().to_vec(),
        old.values().to_vec(),
        old.functions().to_vec(),
    )
    .unwrap();
}

fn authored_model(edit: impl FnOnce(&mut serde_json::Value)) -> NativeModel {
    let mut authored: serde_json::Value = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    edit(&mut authored);
    let text = serde_json::to_string(&authored).unwrap();
    native_rule_model::from_text(&text, "native-link-variant.json", "draft:1")
        .unwrap()
        .model()
}

#[test]
#[trace("TC-044", "FR-015-AC-5")]
fn tc_044_native_reference_fields_and_operations_keep_declared_targets() {
    let models = [parts().model()];
    let clauses = "invariant Parent on M::Node at current { let p = self.parent in present(p) implies deref(value(p)).n <= self.n and reaches(self, other, parent) }\npre Before on M::Node::step { self.n >= 0 }\npost After on M::Node::step { result and self.n = pre(self.n) }";
    let text = document(&models[0], &models[0].digest().to_string(), clauses);
    let linked = link_native(read(&text), &models, LinkLimits::default()).unwrap();
    assert_eq!(linked.binding_profile(), "native-state-model/1");
    assert_eq!(linked.unit().source().text(), text);
    assert_eq!(linked.models()[0].digest(), models[0].digest());
    assert!(std::ptr::eq(
        linked.models()[0].native_model().unwrap(),
        &models[0]
    ));
    assert_eq!(linked.clauses().len(), 3);
    assert!(linked.clauses()[0].operation().is_none());
    let role = &models[0].roles().operations[0];
    for clause in &linked.clauses()[1..] {
        let operation = clause.operation().unwrap();
        assert_eq!(operation.identity.owner, *models[0].environment().owner());
        assert_eq!(operation.source, role.source);
        assert_eq!(
            operation.identity.key,
            DeclarationKey::Operation {
                context: symbol("Node"),
                name: symbol("step"),
            }
        );
        assert!(clause.occurrences().iter().any(|occurrence| {
            linked.unit().source().slice(occurrence.span) == Some("step")
                && occurrence.target == ResolutionTarget::Formal(operation.clone())
        }));
    }
    let parent = &linked.clauses()[0];
    let dereference_fields: Vec<_> = parent
        .occurrences()
        .iter()
        .filter(|occurrence| {
            occurrence.expression.is_some_and(|id| {
                match &linked.unit().expression(id).unwrap().kind {
                    quire_spec_language::syntax::ExprKind::Field { base, .. } => matches!(
                        linked.unit().expression(*base).unwrap().kind,
                        quire_spec_language::syntax::ExprKind::Call {
                            builtin: quire_spec_language::syntax::Builtin::Deref,
                            ..
                        }
                    ),
                    _ => false,
                }
            })
        })
        .collect();
    assert_eq!(dereference_fields.len(), 1);
    let ResolutionTarget::Formal(dereferenced) = &dereference_fields[0].target else {
        panic!("formal dereference field");
    };
    assert_eq!(
        dereferenced.identity.key,
        DeclarationKey::Field {
            record: symbol("Node"),
            field: symbol("n")
        }
    );
    let node = models[0]
        .environment()
        .types()
        .iter()
        .find(|declaration| declaration.name().as_str() == "Node")
        .unwrap();
    let ir::TypeDeclaration::Record { declaration: node } = node else {
        panic!("Node record");
    };
    assert_eq!(
        &dereferenced.source,
        node.fields()
            .iter()
            .find(|field| field.name().as_str() == "n")
            .unwrap()
            .source()
    );
    assert!(parent.occurrences().iter().any(|occurrence| {
        matches!(&occurrence.target, ResolutionTarget::Formal(location)
            if location.identity.key == DeclarationKey::Field { record: symbol("Node"), field: symbol("n") }
            && linked.unit().source().slice(occurrence.span) == Some("n"))
    }));
    let root = linked.unit().clauses()[0].expression;
    assert!(parent.occurrences().iter().any(|occurrence| {
        occurrence.expression.is_some_and(|id| id != root)
            && matches!(&occurrence.target, ResolutionTarget::Formal(location)
                if location.identity.key == DeclarationKey::Field { record: symbol("Node"), field: symbol("parent") })
            && occurrence.expression.is_some_and(|id| matches!(linked.unit().expression(id).unwrap().kind, quire_spec_language::syntax::ExprKind::Reaches { .. }))
    }));
    assert!(linked.clauses()[2].occurrences().iter().any(|occurrence| {
        matches!(&occurrence.target, ResolutionTarget::Formal(location)
            if location.identity.key == DeclarationKey::Value(symbol("step_result")))
            && linked.unit().source().slice(occurrence.span) == Some("result")
    }));
}

#[test]
#[trace("TC-044", "FR-015-AC-5")]
fn tc_044_unmapped_native_paths_refuse_and_legacy_profile_is_preserved() {
    let models = [parts().model()];
    for (clause, code) in [
        (
            "post Rule on M::Node::missing { true }".into(),
            Code::MissingDeclaration,
        ),
        (
            invariant("deref(self.peer).missing"),
            Code::MissingDeclaration,
        ),
        (invariant("self.peer.id"), Code::IllTyped),
        (invariant("value(self.parent).id"), Code::IllTyped),
        (invariant("deref(self)"), Code::IllTyped),
        (invariant("result"), Code::WrongSnapshot),
        (
            "post Rule on M::Node::step { step_result }".into(),
            Code::WrongSnapshot,
        ),
        (
            invariant("reaches(self, other, missing)"),
            Code::MissingDeclaration,
        ),
        (invariant("reaches(self, other, n)"), Code::IllTyped),
    ] {
        let error =
            link_native(unit(&models[0], &clause), &models, LinkLimits::default()).unwrap_err();
        assert_eq!(error.diagnostic.code, code, "{clause}: {error}");
        assert_eq!(error.diagnostic.phase, Phase::Link);
        assert!(!error.diagnostic.is_incomplete());
        assert_eq!(
            link_native(
                unit(&models[0], &invariant("true")),
                &models,
                LinkLimits::default()
            )
            .unwrap()
            .clauses()
            .len(),
            1
        );
    }
    let environments = [models[0].environment().clone()];
    let canonical = environments[0]
        .canonical_declaration(ir::CanonicalProfile::V1)
        .unwrap();
    let digest = ByteDigest::of(canonical.bytes().as_slice());
    let legacy = link(
        read(&document(
            &models[0],
            &digest.to_string(),
            &invariant("self.n"),
        )),
        &environments,
        LinkLimits::default(),
    )
    .unwrap();
    assert_eq!(legacy.binding_profile(), "native-formal-environment/1");
    assert!(legacy.models()[0].native_model().is_none());
    assert_eq!(legacy.models()[0].digest(), digest);
    assert_ne!(digest, models[0].digest());
    for (clause, code) in [
        (invariant("deref(self.peer).n"), Code::InvalidModelBinding),
        (
            invariant("reaches(self, other, parent)"),
            Code::InvalidModelBinding,
        ),
        (
            "post Rule on M::Node::step { true }".into(),
            Code::InvalidModelBinding,
        ),
    ] {
        assert_eq!(
            link(
                read(&document(&models[0], &digest.to_string(), &clause)),
                &environments,
                LinkLimits::default()
            )
            .unwrap_err()
            .diagnostic
            .code,
            code,
            "{clause}"
        );
    }
}

#[test]
#[trace("TC-042", "FR-015-AC-3")]
fn tc_042_native_selection_requires_its_own_exact_artifact_and_revision() {
    let models = [parts().model()];
    let canonical = models[0]
        .environment()
        .canonical_declaration(ir::CanonicalProfile::V1)
        .unwrap();
    let mut foreign = parts();
    foreign.roles.objects[0].universe = symbol("otherUniverse");
    for digest in [
        ByteDigest::of(canonical.bytes().as_slice()),
        foreign.model().digest(),
        ByteDigest::of(b"foreign"),
    ] {
        let error = link_native(
            read(&document(
                &models[0],
                &digest.to_string(),
                &invariant("true"),
            )),
            &models,
            LinkLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.diagnostic.code, Code::StaleDependency);
    }
    for revision in ["01", "0", "2"] {
        let text = document(
            &models[0],
            &models[0].digest().to_string(),
            &invariant("true"),
        )
        .replace("version \"1\"", &format!("version \"{revision}\""));
        assert_eq!(
            link_native(read(&text), &models, LinkLimits::default())
                .unwrap_err()
                .diagnostic
                .code,
            Code::StaleDependency
        );
    }
    let semantic_digest = canonical.digest().to_string();
    for (digest, expected) in [
        (semantic_digest.clone(), Code::InvalidModelBinding),
        (format!("sha256:{semantic_digest}"), Code::StaleDependency),
    ] {
        assert_eq!(
            link_native(
                read(&document(&models[0], &digest, &invariant("true"))),
                &models,
                LinkLimits::default()
            )
            .unwrap_err()
            .diagnostic
            .code,
            expected
        );
    }
}

#[test]
#[trace("TC-044", "FR-015-AC-5")]
fn tc_044_enum_identity_is_not_a_reference_and_parameters_are_operation_scoped() {
    let model =
        authored_model(|authored| {
            authored["enums"] = serde_json::json!([{ "name": "Flag", "variants": ["On", "Off"] }]);
            authored["values"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "name": "mode", "kind": "state", "type": { "kind": "enum", "name": "Flag" }
                }));
            authored["values"].as_array_mut().unwrap().push(serde_json::json!({
            "name": "request", "kind": "input", "type": { "kind": "record", "name": "NodeRef" }
        }));
            authored["operations"][0]["parameters"] = serde_json::json!(["request"]);
        });
    let models = [model];
    let linked = link_native(
        unit(
            &models[0],
            "pre Rule on M::Node::step { deref(request).n >= 0 and M::Flag::On = M::Flag::Off }",
        ),
        &models,
        LinkLimits::default(),
    )
    .unwrap();
    assert!(linked.clauses()[0]
        .occurrences()
        .iter()
        .any(|occurrence| matches!(
            &occurrence.target, ResolutionTarget::Formal(location)
                if location.identity.key == DeclarationKey::Value(symbol("request"))
        )));
    for (expression, code) in [
        ("deref(M::Flag::On)", Code::IllTyped),
        ("deref(mode)", Code::IllTyped),
        ("request", Code::MissingDeclaration),
    ] {
        assert_eq!(
            link_native(
                unit(&models[0], &invariant(expression)),
                &models,
                LinkLimits::default()
            )
            .unwrap_err()
            .diagnostic
            .code,
            code
        );
    }
    let context_only = authored_model(|authored| {
        authored["values"]
            .as_array_mut()
            .unwrap()
            .retain(|value| value["name"] != "self");
    });
    let contexts = [context_only];
    let linked = link_native(
        unit(&contexts[0], &invariant("self.n >= 0")),
        &contexts,
        LinkLimits::default(),
    )
    .unwrap();
    assert_eq!(
        linked.clauses()[0].context().identity.key,
        DeclarationKey::Type(symbol("Node"))
    );
}

#[test]
#[trace("TC-043", "FR-015-AC-4")]
fn tc_043_inventory_conflicts_refuse_before_import_selection() {
    let original = parts().model();
    for mutation in 0..4 {
        let mut input = parts();
        reowner(&mut input, "OtherRule");
        let old = input.source.source();
        let mut identity = old.identity().clone();
        let mut text = old.text().to_owned();
        match mutation {
            0 => identity.identity.push_str(":foreign"),
            1 => identity.revision.push_str(":foreign"),
            2 => text.push('\n'),
            3 => {
                reowner(&mut input, "RuleModel");
                input.roles.operations[0].frame.fields.clear();
            }
            _ => unreachable!(),
        }
        let native =
            Source::read(identity, "inventory-model.json", text.as_bytes(), 1_048_576).unwrap();
        input.source = FormalSource::new(native, input.source.identity().clone());
        let inventory = [original.clone(), input.model()];
        let error = link_native(
            unit(&inventory[0], &invariant("true")),
            &inventory,
            LinkLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            error.diagnostic.code,
            Code::InvalidModelBinding,
            "mutation {mutation}"
        );
        assert_eq!(error.diagnostic.phase, Phase::Link);
        assert_eq!(
            error.diagnostic.source,
            *inventory[1].source().source().identity()
        );
        assert_eq!(error.diagnostic.span.start.byte, 0);
        assert!(!error.related.is_empty());
    }
    let duplicates = [original.clone(), original.clone()];
    let error = link_native(
        unit(&duplicates[0], &invariant("true")),
        &duplicates,
        LinkLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.diagnostic.code, Code::AmbiguousDeclaration);
    assert!(!error.related.is_empty());
    let mut independent = parts();
    reowner(&mut independent, "IndependentRule");
    let inventory = [original, independent.model()];
    assert_eq!(
        link_native(
            unit(&inventory[0], &invariant("true")),
            &inventory,
            LinkLimits::default()
        )
        .unwrap()
        .models()[0]
            .environment()
            .owner()
            .requirement()
            .as_str(),
        "RuleModel"
    );
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_native_link_limits_are_effective_and_inclusive() {
    let models = [parts().model()];
    let clause = invariant("self.n >= 0");
    let bytes = models[0].artifact_bytes().len();
    let exact = LinkLimits {
        models: 1,
        imports: 1,
        clauses: 1,
        nodes: 4,
        depth: 3,
        model_bytes: bytes,
        total_model_bytes: bytes,
    };
    assert_eq!(
        link_native(unit(&models[0], &clause), &models, exact)
            .unwrap()
            .clauses()
            .len(),
        1
    );
    for dimension in 0..7 {
        for zero in [false, true] {
            let mut limits = exact;
            let selected = match dimension {
                0 => &mut limits.models,
                1 => &mut limits.imports,
                2 => &mut limits.clauses,
                3 => &mut limits.nodes,
                4 => &mut limits.depth,
                5 => &mut limits.model_bytes,
                6 => &mut limits.total_model_bytes,
                _ => unreachable!(),
            };
            *selected = if zero { 0 } else { *selected - 1 };
            let error = link_native(unit(&models[0], &clause), &models, limits).unwrap_err();
            assert_eq!(
                error.diagnostic.code,
                Code::ResourceExhausted,
                "dimension {dimension}"
            );
            assert!(error.diagnostic.is_incomplete());
        }
    }
}

fn balanced_conjunction(leaves: usize) -> String {
    if leaves == 1 {
        return "true".into();
    }
    format!(
        "({} and {})",
        balanced_conjunction(leaves / 2),
        balanced_conjunction(leaves - leaves / 2)
    )
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_native_link_hard_inventory_syntax_and_depth_limits_cannot_be_raised() {
    let model = parts().model();
    let permissive = LinkLimits {
        models: usize::MAX,
        imports: usize::MAX,
        clauses: usize::MAX,
        nodes: usize::MAX,
        depth: usize::MAX,
        model_bytes: usize::MAX,
        total_model_bytes: usize::MAX,
    };
    let models = [model.clone()];
    let excessive_inventory = vec![model.clone(); 65];
    assert_eq!(
        link_native(
            unit(&model, &invariant("true")),
            &excessive_inventory,
            permissive
        )
        .unwrap_err()
        .diagnostic
        .code,
        Code::ResourceExhausted
    );
    for count in [64, 65] {
        let imports: String = (1..count)
            .map(|index| {
                format!(
                    "model Alias{index} = \"example/rule-tests\" version \"1\" digest \"{}\";\n",
                    model.digest()
                )
            })
            .collect();
        let text = document(
            &model,
            &model.digest().to_string(),
            &(imports + &invariant("true")),
        );
        let result = link_native(read(&text), &models, permissive);
        if count == 64 {
            assert_eq!(result.unwrap().models().len(), 64);
        } else {
            assert_eq!(result.unwrap_err().diagnostic.code, Code::ResourceExhausted);
        }
    }
    for count in [256, 257] {
        let clauses: String = (0..count)
            .map(|index| format!("invariant Rule{index} on M::Node at current {{ true }}\n"))
            .collect();
        let result = link_native(unit(&model, &clauses), &models, permissive);
        if count == 256 {
            assert_eq!(result.unwrap().clauses().len(), 256);
        } else {
            assert_eq!(result.unwrap_err().diagnostic.code, Code::ResourceExhausted);
        }
    }
    for leaves in [3334, 3335] {
        let input = unit(&model, &invariant(&balanced_conjunction(leaves)));
        assert_eq!(input.expressions().len(), 3 * leaves - 2);
        let result = link_native(input, &models, permissive);
        if leaves == 3334 {
            assert_eq!(result.unwrap().unit().expressions().len(), 10_000);
        } else {
            assert_eq!(result.unwrap_err().diagnostic.code, Code::ResourceExhausted);
        }
    }
    for depth in [64, 65] {
        let expression = std::iter::repeat_n("true", depth)
            .collect::<Vec<_>>()
            .join(" and ");
        let result = link_native(unit(&model, &invariant(&expression)), &models, permissive);
        if depth == 64 {
            assert_eq!(result.unwrap().clauses().len(), 1);
        } else {
            assert_eq!(result.unwrap_err().diagnostic.code, Code::ResourceExhausted);
        }
    }
}

fn model_at_artifact_ceiling(requirement: &str) -> NativeModel {
    let mut input = parts();
    reowner(&mut input, requirement);
    let base_length = input.model().artifact_bytes().len();
    let mut input = parts();
    reowner(&mut input, requirement);
    let old = input.source.source();
    let mut identity = old.identity().clone();
    identity
        .identity
        .push_str(&"x".repeat(1_048_576 - base_length));
    let native = Source::read(identity, old.path(), old.text().as_bytes(), 1_048_576).unwrap();
    input.source = FormalSource::new(native, input.source.identity().clone());
    let model = input.model();
    assert_eq!(model.artifact_bytes().len(), 1_048_576);
    model
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_native_artifact_and_aggregate_hard_byte_ceilings() {
    // Each input has a large native label, with small formal declaration names
    // and loci. Nine models keep this boundary test's memory in tens of MiB.
    let models: Vec<_> = (0..9)
        .map(|index| model_at_artifact_ceiling(&format!("Rule{index:05}")))
        .collect();
    let limits = LinkLimits {
        model_bytes: usize::MAX,
        total_model_bytes: usize::MAX,
        ..LinkLimits::default()
    };
    let clauses = invariant("true");
    let linked = link_native(unit(&models[0], &clauses), &models[..8], limits).unwrap();
    assert_eq!(
        linked.models()[0]
            .native_model()
            .unwrap()
            .artifact_bytes()
            .len(),
        1_048_576
    );
    assert_eq!(
        link_native(unit(&models[0], &clauses), &models, limits)
            .unwrap_err()
            .diagnostic
            .code,
        Code::ResourceExhausted
    );
    let exact_total = models[0].artifact_bytes().len() + models[1].artifact_bytes().len();
    for total in [0, exact_total - 1, exact_total] {
        let result = link_native(
            unit(&models[0], &clauses),
            &models[..2],
            LinkLimits {
                total_model_bytes: total,
                ..LinkLimits::default()
            },
        );
        if total == exact_total {
            assert_eq!(result.unwrap().models().len(), 1);
        } else {
            assert_eq!(result.unwrap_err().diagnostic.code, Code::ResourceExhausted);
        }
    }
    // NativeModel admission already prevents constructing an artifact beyond
    // the link profile's per-model hard ceiling; raising the link option cannot
    // bypass that producer boundary.
    let model = &models[0];
    let mut identity = model.source().source().identity().clone();
    identity.identity.push('x');
    let source = Source::read(
        identity,
        "one-byte-over.json",
        model.source().source().text().as_bytes(),
        1_048_576,
    )
    .unwrap();
    let error = NativeModel::new(
        FormalSource::new(source, model.source().identity().clone()),
        model.environment().clone(),
        model.roles().clone(),
        quire_spec_language::native_model::ModelLimits {
            artifact_bytes: usize::MAX,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.diagnostic.code, Code::ResourceExhausted);
}
