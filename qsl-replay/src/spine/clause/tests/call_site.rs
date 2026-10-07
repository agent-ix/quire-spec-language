// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-516 (FR-121): `crate::call_site`'s operation and clause selections
//! over FR-108's ConfigVersion unit, with the `test/config-version` domain
//! package as its package input. `VersionUnchanged` is the unit's one
//! clause naming `attemptUpdate`; `ParentOrder` and `NoCycle` are
//! invariants. The two-operation unit adds `probe` to the domain package, a
//! precondition on `attemptUpdate` declared before `VersionUnchanged`, and a
//! postcondition on `probe`.
//!
//! The expected identities are read by scanning the compiled graph's nodes
//! by semantic form, and the occurrence keys are built from the facade's
//! own `OccurrenceKey`, `Origin` and `Role` -- not by re-running
//! `resolve_operation`/`operation_frame`/`state_clause`, the lookups
//! `call_site` itself uses.

use super::frame::identifier;
use super::*;
use crate::spine::OperationName;
use crate::{
    call_site, CallSite, CallSiteRefusal, ClauseName, ClauseSite, OccurrenceKey, OperationSite,
    Origin, Role, WireNodeId,
};

fn wire(node: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*node.as_bytes())
}

fn operation(model: &str, object: &str, operation: &str) -> OperationName {
    OperationName {
        model: identifier(model),
        object: identifier(object),
        operation: identifier(operation),
    }
}

/// `call_site` over the ConfigVersion unit, its domain package supplied,
/// selecting `operation`.
fn operation_site(
    operation: OperationName,
) -> Result<CallSite<OperationSite>, Box<CallSiteRefusal>> {
    let (unit, _) = config_version_unit_and_packages();
    let document = config_version_domain_document();
    call_site(
        source(),
        "clause-run-config-version.native",
        unit.as_bytes(),
        [document.as_slice()],
        &DependencyInput::default(),
        &operation,
    )
}

/// The ConfigVersion unit over the domain package with `probe` added:
/// `ParentOrder` and `NoCycle`, then `AttemptPre` on `attemptUpdate`,
/// `VersionUnchanged` on `attemptUpdate` and `ProbeUnchanged` on `probe`,
/// in that order, and the function `sameIdentity`. With `clauses` false it
/// declares only the invariants, `VersionUnchanged` and `sameIdentity`, so
/// no clause names `probe`.
fn two_operation_unit(clauses: bool) -> (String, Vec<u8>) {
    let document = config_version_step3_domain_document();
    let packages = qsl_semantics::model::intake::package_input(
        [document.as_slice()],
    );
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = hex(digest);
    let (pre, probe_post) = if clauses {
        (
            "pre AttemptPre using v on Config::ConfigVersion::attemptUpdate { \
             not reaches(self, self, parent) }\n",
            "post ProbeUnchanged using v on Config::ConfigVersion::probe { \
             self.versionNumber = pre(self.versionNumber) }\n",
        )
    } else {
        ("", "")
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?} version \"1.0.0\" \
         digest \"sha256-jcs:{digest}\";\n\
         invariant ParentOrder using v on Config::ConfigVersion at current {{ \
         present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }}\n\
         invariant NoCycle using v on Config::ConfigVersion at current {{ \
         not reaches(self, self, parent) }}\n\
         {pre}\
         post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate {{ \
         self.versionNumber = pre(self.versionNumber) }}\n\
         {probe_post}\
         function sameIdentity using v(a: Config::ConfigVersion, b: Config::ConfigVersion): \
         Boolean pure {{ a = b }}\n"
    );
    (unit, document)
}

/// `call_site` over `unit` with `document` supplied, selecting `selection`.
fn locate<S: crate::CallSiteSelection>(
    unit: &str,
    document: &[u8],
    selection: &S,
) -> Result<CallSite<S::Site>, Box<CallSiteRefusal>> {
    call_site(
        source(),
        "call-site.native",
        unit.as_bytes(),
        [document],
        &DependencyInput::default(),
        selection,
    )
}

/// The node keys of `graph`'s state clauses of `kind`, in graph order.
fn clause_nodes_of(graph: &qsl_semantics::check::CheckedGraph, kind: &str) -> Vec<WireNodeId> {
    state_clause_nodes(graph)
        .into_iter()
        .filter(|(_, found)| *found == kind)
        .map(|(node, _)| wire(node.key()))
        .collect()
}

fn claim(node: WireNodeId) -> OccurrenceKey {
    OccurrenceKey::new(node, Origin::new(Role::new("claim"), 0))
}

/// FR-121-AC-3: `attemptUpdate`'s selection returns the compiled package's
/// `package_id`, the one `operation_anchor` and `frame` node, the frame's
/// `generated` occurrence at ordinal 0 -- equal to the key a client builds
/// through the facade's own `OccurrenceKey::new` and `Origin::new` -- and
/// exactly the one postcondition naming the operation, at its `claim`
/// occurrence.
#[trace("TC-516", "FR-121-AC-3")]
#[test]
fn call_site_locates_an_operations_anchor_frame_and_clauses() {
    let site = operation_site(operation("Config", "ConfigVersion", "attemptUpdate"))
        .expect("attemptUpdate is named by VersionUnchanged");
    let compiled = config_version_compiled();
    assert_eq!(site.package_id, compiled.emitted.package_id().record());

    let graph = compiled.package.graph();
    let operation = &site.site;
    let frame = wire(single_state_node_key(graph, "frame"));
    assert_eq!(
        operation.anchor,
        wire(single_state_node_key(graph, "operation_anchor"))
    );
    assert_eq!(operation.frame, frame);
    assert_eq!(
        operation.frame_occurrence,
        OccurrenceKey::new(frame, Origin::new(Role::new("generated"), 0))
    );

    let posts: Vec<WireNodeId> = state_clause_nodes(graph)
        .into_iter()
        .filter(|(_, kind)| *kind == "postcondition")
        .map(|(node, _)| wire(node.key()))
        .collect();
    let [post] = posts[..] else {
        panic!("the unit declares one postcondition: {posts:?}");
    };
    let [clause] = &operation.clauses[..] else {
        panic!(
            "only VersionUnchanged names attemptUpdate: {:?}",
            operation.clauses
        );
    };
    assert_eq!(clause.name.as_str(), "VersionUnchanged");
    assert_eq!(clause.node, post);
    assert_eq!(
        clause.occurrence,
        OccurrenceKey::new(post, Origin::new(Role::new("claim"), 0))
    );
}

/// FR-121-AC-5: an operation whose model alias, object type or operation
/// does not resolve, or that no clause or attempt names, refuses
/// `UnknownOperation`, paired with the compiled package's own `package_id`.
#[trace("TC-516", "FR-121-AC-5")]
#[test]
fn call_site_refuses_an_unknown_operation_paired_with_its_package() {
    let package = config_version_compiled().emitted.package_id().record();
    for selection in [
        operation("Nope", "ConfigVersion", "attemptUpdate"),
        operation("Config", "Nope", "attemptUpdate"),
        operation("Config", "ConfigVersion", "nope"),
    ] {
        let refusal =
            operation_site(selection.clone()).expect_err("the selection names no operation frame");
        match *refusal {
            CallSiteRefusal::UnknownOperation {
                selection: refused,
                package: refused_in,
            } => {
                assert_eq!(refused, selection);
                assert_eq!(refused_in, package);
            }
            other => panic!("expected UnknownOperation for {selection}, got {other:?}"),
        }
    }
}

/// FR-121-AC-6: with two operations, `attemptUpdate`'s clauses are exactly
/// its precondition and its postcondition, in declaration order;
/// `probe`'s postcondition and the invariants are absent, and `probe`'s
/// own selection returns only its postcondition.
#[trace("TC-516", "FR-121-AC-6")]
#[test]
fn call_site_returns_only_the_selected_operations_clauses_in_declaration_order() {
    let (unit, document) = two_operation_unit(true);
    let attempt = locate(
        &unit,
        &document,
        &operation("Config", "ConfigVersion", "attemptUpdate"),
    )
    .expect("attemptUpdate is named by AttemptPre and VersionUnchanged");
    let names: Vec<&str> = attempt
        .site
        .clauses
        .iter()
        .map(|clause| clause.name.as_str())
        .collect();
    assert_eq!(names, ["AttemptPre", "VersionUnchanged"]);
    let probe = locate(
        &unit,
        &document,
        &operation("Config", "ConfigVersion", "probe"),
    )
    .expect("probe is named by ProbeUnchanged");
    let names: Vec<&str> = probe
        .site
        .clauses
        .iter()
        .map(|clause| clause.name.as_str())
        .collect();
    assert_eq!(names, ["ProbeUnchanged"]);
    assert_ne!(attempt.site.frame, probe.site.frame);
}

/// FR-121-AC-7: `probe` is declared in the supplied domain package, but no
/// clause or attempt of the unit names it, so the package holds no frame
/// for it: the selection refuses `UnknownOperation`, paired with the
/// package.
#[trace("TC-516", "FR-121-AC-7")]
#[test]
fn call_site_refuses_an_operation_no_clause_names() {
    let (unit, document) = two_operation_unit(false);
    let package = locate(
        &unit,
        &document,
        &ClauseName(identifier("VersionUnchanged")),
    )
    .expect("VersionUnchanged is declared")
    .package_id;
    let selection = operation("Config", "ConfigVersion", "probe");
    let refusal = locate(&unit, &document, &selection)
        .expect_err("no clause names probe, so it has no frame");
    match *refusal {
        CallSiteRefusal::UnknownOperation {
            selection: refused,
            package: refused_in,
        } => {
            assert_eq!(refused, selection);
            assert_eq!(refused_in, package);
        }
        other => panic!("expected UnknownOperation, got {other:?}"),
    }
}

/// FR-121-AC-8: a clause selection returns an invariant's, a
/// precondition's and a postcondition's own `state_clause` node, of the
/// clause's own kind, at its `claim` occurrence at ordinal 0, with the
/// compiled package's `package_id`; a postcondition's answer is the one the
/// operation selection lists. A name no state clause declares -- including
/// a function's -- refuses `UnknownClause`, paired with the package.
#[trace("TC-516", "FR-121-AC-8")]
#[test]
fn call_site_locates_a_state_clause_by_name() {
    let (unit, document) = two_operation_unit(true);
    let packages = qsl_semantics::model::intake::package_input(
        [document.as_slice()],
    );
    let compiled = compose(
        source(),
        "call-site.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the two-operation unit compiles");
    let graph = compiled.package.graph();
    let clause = |name: &str| -> CallSite<ClauseSite> {
        locate(&unit, &document, &ClauseName(identifier(name)))
            .unwrap_or_else(|refusal| panic!("{name} is declared: {refusal:?}"))
    };

    let parent_order = clause("ParentOrder");
    let no_cycle = clause("NoCycle");
    assert_eq!(
        parent_order.package_id,
        compiled.emitted.package_id().record()
    );
    let mut invariants = vec![parent_order.site.node, no_cycle.site.node];
    invariants.sort_unstable();
    let mut expected = clause_nodes_of(graph, "invariant");
    expected.sort_unstable();
    assert_eq!(invariants, expected, "the two invariants' own nodes");
    assert_eq!(parent_order.site.name.as_str(), "ParentOrder");
    assert_eq!(parent_order.site.occurrence, claim(parent_order.site.node));

    let pre = clause("AttemptPre");
    assert_eq!(vec![pre.site.node], clause_nodes_of(graph, "precondition"));
    assert_eq!(pre.site.occurrence, claim(pre.site.node));

    let post = clause("VersionUnchanged");
    assert!(clause_nodes_of(graph, "postcondition").contains(&post.site.node));
    assert_eq!(post.site.occurrence, claim(post.site.node));
    let attempt = locate(
        &unit,
        &document,
        &operation("Config", "ConfigVersion", "attemptUpdate"),
    )
    .expect("attemptUpdate is named");
    assert_eq!(attempt.site.clauses[1], post.site);

    let function = crate::QualifiedName::new(vec![identifier("sameIdentity")]).unwrap();
    locate(&unit, &document, &function).expect("sameIdentity is a declared function");
    for name in ["Absent", "sameIdentity"] {
        let selection = ClauseName(identifier(name));
        let refusal =
            locate(&unit, &document, &selection).expect_err("the name declares no state clause");
        match *refusal {
            CallSiteRefusal::UnknownClause {
                selection: refused,
                package,
            } => {
                assert_eq!(refused, selection);
                assert_eq!(package, parent_order.package_id);
            }
            other => panic!("expected UnknownClause for {name}, got {other:?}"),
        }
    }
}

/// The node keys of `graph`'s `function` nodes whose `declaration` is
/// `name`, read by scanning the graph rather than through `callable`, the
/// lookup `call_site` itself uses.
fn function_nodes_declaring(
    graph: &qsl_semantics::check::CheckedGraph,
    name: &str,
) -> Vec<WireNodeId> {
    graph
        .semantic_graph()
        .nodes()
        .filter(|node| {
            node.node_tag() == qsl_semantics::check::NodeTag::Function
                && node.declaration().is_some_and(|declaration| {
                    declaration.len() == 1 && declaration[0].as_str() == name
                })
        })
        .map(|node| wire(node.key()))
        .collect()
}

/// FR-121-AC-15 (TC-516 step 15): `p` and `q` share one parameter node and
/// differ in `function`; each `function` is the graph's `function` node
/// declaring it and `declaration` is the occurrence a client builds through
/// the facade; a comment and blank lines before `p` change neither; and
/// `sameIdentity`'s `function` is no clause node of the AC-6 unit.
#[trace("TC-516", "FR-121-AC-15")]
#[test]
fn function_site_names_its_function_node_and_declaration_occurrence() {
    const PQ: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        function p using v(x: Int[0, 9]): Boolean pure { x < 5 }\n\
        function q using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
    let site = |unit: &str, name: &str| {
        call_site(
            source(),
            "pq.native",
            unit.as_bytes(),
            [],
            &DependencyInput::default(),
            &crate::QualifiedName::new(vec![identifier(name)]).unwrap(),
        )
        .unwrap_or_else(|refusal| panic!("{name} is declared: {refusal:?}"))
        .site
    };
    let compiled = compose(
        source(),
        "pq.native",
        PQ.as_bytes(),
        &qsl_semantics::model::intake::package_input([]),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the unit compiles");
    let graph = compiled.package.graph();

    let p = site(PQ, "p");
    let q = site(PQ, "q");
    assert_eq!(vec![p.function], function_nodes_declaring(graph, "p"));
    assert_eq!(vec![q.function], function_nodes_declaring(graph, "q"));
    assert_eq!(
        p.declaration,
        OccurrenceKey::new(p.function, Origin::new(Role::new("declaration"), 0))
    );
    assert_eq!(
        q.declaration,
        OccurrenceKey::new(q.function, Origin::new(Role::new("declaration"), 0))
    );
    assert_eq!(p.parameters, q.parameters, "one shared parameter node");
    assert_ne!(p.function, q.function);
    assert_ne!(p.declaration, q.declaration);

    let shifted = PQ.replacen("function p", "// a comment\n\n\nfunction p", 1);
    let moved = site(&shifted, "p");
    assert_eq!(moved.function, p.function);
    assert_eq!(moved.declaration, p.declaration);

    let (unit, document) = two_operation_unit(true);
    let same_identity = locate(
        &unit,
        &document,
        &crate::QualifiedName::new(vec![identifier("sameIdentity")]).unwrap(),
    )
    .expect("sameIdentity is declared")
    .site;
    let mut clause_nodes = Vec::new();
    for name in [
        "ParentOrder",
        "NoCycle",
        "AttemptPre",
        "VersionUnchanged",
        "ProbeUnchanged",
    ] {
        clause_nodes.push(
            locate(&unit, &document, &ClauseName(identifier(name)))
                .expect("the clause is declared")
                .site
                .node,
        );
    }
    for operation_name in ["attemptUpdate", "probe"] {
        let operation_site = locate(
            &unit,
            &document,
            &operation("Config", "ConfigVersion", operation_name),
        )
        .expect("the operation is named");
        clause_nodes.extend(operation_site.site.clauses.iter().map(|clause| clause.node));
    }
    assert!(!clause_nodes.contains(&same_identity.function));
}

/// FR-121-AC-9: a unit whose `model` declaration selects a domain package
/// the supplied documents do not hold refuses `ModelIntake`, naming the
/// declaration's alias.
#[trace("TC-516", "FR-121-AC-9")]
#[test]
fn call_site_refuses_an_unsupplied_domain_package_as_model_intake() {
    let (unit, _) = two_operation_unit(true);
    let refusal = call_site(
        source(),
        "call-site.native",
        unit.as_bytes(),
        [],
        &DependencyInput::default(),
        &ClauseName(identifier("ParentOrder")),
    )
    .expect_err("no domain package is supplied");
    let CallSiteRefusal::ModelIntake { alias, .. } = *refusal else {
        panic!("expected ModelIntake, got {refusal:?}");
    };
    assert_eq!(alias, "Config");
}

/// FR-121-AC-14 (TC-516 step 14): AC-5's `UnknownOperation` and AC-8's
/// `UnknownClause` give `missing_declaration`, and AC-9's `ModelIntake`
/// gives the code `replay` gives the same unit with no domain package.
#[trace("TC-516", "FR-121-AC-14")]
#[test]
fn call_site_refusal_codes_for_operations_clauses_and_intake() {
    let refusal = operation_site(operation("Config", "ConfigVersion", "nope"))
        .expect_err("the selection names no operation frame");
    assert!(
        matches!(*refusal, CallSiteRefusal::UnknownOperation { .. }),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);

    let (unit, document) = two_operation_unit(true);
    let refusal = locate(&unit, &document, &ClauseName(identifier("Absent")))
        .expect_err("Absent declares no state clause");
    assert!(
        matches!(*refusal, CallSiteRefusal::UnknownClause { .. }),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);

    let refusal = call_site(
        source(),
        "call-site.native",
        unit.as_bytes(),
        [],
        &DependencyInput::default(),
        &ClauseName(identifier("ParentOrder")),
    )
    .expect_err("no domain package is supplied");
    assert!(
        matches!(*refusal, CallSiteRefusal::ModelIntake { .. }),
        "{refusal:?}"
    );
    let mut wire = super::frame_replay::request(
        unit.as_bytes(),
        &document,
        qsl_foundation::digest::DigestRecord::mint(
            qsl_foundation::digest::DigestDomain::PackageSemanticV2,
            [4; 32],
        ),
        &[],
    );
    wire.byte_provision.retain(|(domain, _, _)| {
        domain.as_deref() != Some(qsl_foundation::digest::DigestDomain::Sha256Jcs.as_str())
    });
    let replayed = crate::replay(wire, crate::ReplayLimits::default())
        .expect_err("no domain package is provided");
    assert!(
        matches!(replayed, crate::ReplayRefusal::Recompile(_)),
        "{replayed:?}"
    );
    assert_eq!(refusal.code(), replayed.code());
}

/// [`config_version_domain_document_with_sub`] with `Sub` declaring two
/// fields of its own, `zeta` then `alpha` -- out of name order, as
/// `ConfigVersion`'s own `versionNumber` then `parent` are -- each of type
/// `VersionNumber`.
fn sub_with_fields_document() -> Vec<u8> {
    let sub = format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/Sub");
    let field = |name: &str| {
        let identity = format!("{sub}/{name}");
        json!({
            "identity": identity,
            "name": name,
            "typeRef": version_number_type(),
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": {
                "generated": {
                    "generatorIdentity": identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity],
                }
            },
        })
    };
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&config_version_domain_document_with_sub())
            .expect("the document is JSON");
    let types = envelope["types"].as_array_mut().expect("types is an array");
    let entry = types
        .iter_mut()
        .find(|entry| entry["identity"] == sub.as_str())
        .expect("the document declares Sub");
    entry["fields"] = json!([field("zeta"), field("alpha")]);
    envelope.to_string().into_bytes()
}

fn field(model: &str, object: &str, field: &str) -> crate::FieldName {
    crate::FieldName {
        model: identifier(model),
        object: identifier(object),
        field: identifier(field),
    }
}

/// FR-121-AC-18 (ADR-012 §15.4): a field selection keys each state field
/// under the `model`/`object_type` node of the type that declares it, at
/// its ordinal among that type's own fields in ascending name order, not
/// declaration order: `ConfigVersion` declares `versionNumber` then
/// `parent`, keyed `[1]` and `[0]`; `Sub` declares `zeta` then `alpha`,
/// keyed `[1]` and `[0]`. `versionNumber` named through `Sub` is keyed
/// under `ConfigVersion`, its declaring type. Each node is the one the
/// compiled graph's own lowering minted for that type -- `Sub`'s because
/// an invariant on `Config::Sub` takes it as its context. A field the type
/// does not have refuses `UnknownField`, paired with the package.
#[trace("TC-516", "FR-121-AC-18")]
#[test]
fn call_site_keys_a_state_field_under_its_declaring_type_by_name_ordinal() {
    let (unit, packages) = config_version_unit_and_packages_for(sub_with_fields_document());
    let unit = insert_clause_after(
        &unit,
        "VersionUnchanged",
        "invariant SubAlpha using v on Config::Sub at current { self.alpha <= self.zeta }",
    );
    let compiled = compile_config_version_unit(&unit, &packages);
    let graph = compiled.package.graph();
    let config_version = wire(config_version_node_key(graph));
    let sub_declaration = qsl_semantics::model::key::DeclarationKey {
        package: CONFIG_VERSION_PACKAGE_IDENTITY.to_owned(),
        node: format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/Sub"),
    };
    let sub = graph
        .semantic_graph()
        .nodes()
        .find(|node| graph.resolve_declaration(node.key()) == Some(&sub_declaration))
        .map(|node| wire(node.key()))
        .expect("SubAlpha's context mints Sub's own object_type node");
    assert_ne!(sub, config_version);

    let [(_, document)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let key = |object: &str, name: &str| {
        locate(&unit, document, &field("Config", object, name))
            .unwrap_or_else(|refusal| panic!("{object}.{name} resolves: {refusal:?}"))
            .site
            .domain
    };
    assert_eq!(
        key("ConfigVersion", "parent"),
        crate::DomainKey::Node {
            node: config_version,
            path: vec![0]
        }
    );
    assert_eq!(
        key("ConfigVersion", "versionNumber"),
        crate::DomainKey::Node {
            node: config_version,
            path: vec![1]
        }
    );
    assert_eq!(
        key("Sub", "versionNumber"),
        crate::DomainKey::Node {
            node: config_version,
            path: vec![1]
        },
        "an inherited field is keyed under its declaring type"
    );
    assert_eq!(
        key("Sub", "alpha"),
        crate::DomainKey::Node {
            node: sub,
            path: vec![0]
        }
    );
    assert_eq!(
        key("Sub", "zeta"),
        crate::DomainKey::Node {
            node: sub,
            path: vec![1]
        }
    );

    let package = compiled.emitted.package_id().record();
    for selection in [
        field("Config", "Sub", "nope"),
        field("Config", "ConfigVersion", "alpha"),
        field("Config", "Nope", "parent"),
        field("Nope", "Sub", "alpha"),
    ] {
        let refusal = locate(&unit, document, &selection)
            .expect_err("the selection names no field of the package");
        assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
        match *refusal {
            CallSiteRefusal::UnknownField {
                selection: refused,
                package: refused_in,
            } => {
                assert_eq!(refused, selection);
                assert_eq!(refused_in, package);
            }
            other => panic!("expected UnknownField for {selection}, got {other:?}"),
        }
    }
}

/// FR-121-AC-19: a field's domain key does not depend on whether the
/// package's lowering minted its declaring type's node. With no clause on
/// `Sub`, nothing in the graph names `Sub`'s node, yet `Sub.alpha` is
/// keyed under the same node the AC-18 unit, whose invariant mints it,
/// keys it under.
#[trace("TC-516", "FR-121-AC-19")]
#[test]
fn call_site_keys_a_field_of_a_type_no_clause_names() {
    let (unit, packages) = config_version_unit_and_packages_for(sub_with_fields_document());
    let [(_, document)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let with_clause = insert_clause_after(
        &unit,
        "VersionUnchanged",
        "invariant SubAlpha using v on Config::Sub at current { self.alpha <= self.zeta }",
    );
    let unminted = locate(&unit, document, &field("Config", "Sub", "alpha"))
        .expect("Sub.alpha resolves")
        .site
        .domain;
    let minted = locate(&with_clause, document, &field("Config", "Sub", "alpha"))
        .expect("Sub.alpha resolves")
        .site
        .domain;
    let crate::DomainKey::Node { node: sub, .. } = unminted else {
        panic!("a field key is a node key: {unminted:?}");
    };
    let compiled = compile_config_version_unit(&unit, &packages);
    assert!(
        !compiled
            .package
            .graph()
            .semantic_graph()
            .nodes()
            .any(|node| wire(node.key()) == sub),
        "no clause names Sub, so its node is not in the graph"
    );
    assert_eq!(unminted, minted);
}

/// FR-121-AC-20 (ADR-012 §15.4, §15.7): `parent` is `ConfigVersion`'s field
/// at name ordinal 0, and `config_history` is the package's population at
/// ordinal 0 whose member type is `ConfigVersion`: one node, one ordinal,
/// two distinct keys, because each names its subject.
#[trace("TC-516", "FR-121-AC-20")]
#[test]
fn a_field_key_and_a_population_key_with_one_ordinal_are_distinct() {
    let compiled = config_version_compiled();
    let graph = compiled.package.graph();
    let config_version = wire(config_version_node_key(graph));
    let population = crate::DomainKey::Population {
        member_type: config_version,
        ordinal: 0,
    };
    let recorded: Vec<&crate::DomainKey> = graph
        .requirements()
        .values()
        .filter_map(|record| match record.requirements().extent() {
            qsl_semantics::family::ClaimExtent::Unbounded(domains) => Some(domains),
            qsl_semantics::family::ClaimExtent::Bounded => None,
        })
        .flat_map(|domains| domains.iter().map(|(key, _)| key))
        .collect();
    assert!(
        !recorded.is_empty() && recorded.iter().all(|key| **key == population),
        "every recorded domain is config_history's population key: {recorded:?}"
    );

    let document = config_version_domain_document();
    let (unit, _) = config_version_unit_and_packages();
    let parent = locate(
        &unit,
        &document,
        &field("Config", "ConfigVersion", "parent"),
    )
    .expect("parent resolves")
    .site
    .domain;
    assert_eq!(
        parent,
        crate::DomainKey::Node {
            node: config_version,
            path: vec![0]
        }
    );
    assert_ne!(parent, population);
}

/// [`sub_with_fields_document`] with a second population, `aaa_subs` of
/// `Sub`, declared after `config_history` in source but ahead of it in
/// ascending `DeclarationKey` order: `aaa_subs` is ordinal 0 and
/// `config_history` ordinal 1.
fn two_population_document() -> Vec<u8> {
    let population = format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/aaa_subs");
    let sub = format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/Sub");
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&sub_with_fields_document()).expect("the document is JSON");
    envelope["populations"]
        .as_array_mut()
        .expect("populations is an array")
        .push(json!({
            "identity": population,
            "displayName": population,
            "kind": {"module": CONFIG_VERSION_PACKAGE_IDENTITY, "name": "population"},
            "members": [sub],
            "extent": "closed",
            "origin": {
                "generated": {
                    "generatorIdentity": population,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [population],
                }
            },
        }));
    envelope.to_string().into_bytes()
}

fn population(model: &str, population: &str) -> crate::PopulationName {
    crate::PopulationName {
        model: identifier(model),
        population: identifier(population),
    }
}

/// FR-121-AC-21 (ADR-012 §15.7): over two populations whose declaration-key
/// order is not their source order, a population selection returns
/// `config_history`'s key at ordinal 1 on `ConfigVersion`'s node -- exactly
/// the key every requirement record of the unit's `ConfigVersion` clauses
/// carries -- and `aaa_subs`'s at ordinal 0 on `Sub`'s node, the node a
/// `Sub` field is keyed under.
#[trace("TC-516", "FR-121-AC-21")]
#[test]
fn call_site_keys_a_population_as_its_requirement_records_do() {
    let (unit, packages) = config_version_unit_and_packages_for(two_population_document());
    let [(_, document)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let compiled = compile_config_version_unit(&unit, &packages);
    let graph = compiled.package.graph();
    let recorded: BTreeSet<&crate::DomainKey> = graph
        .requirements()
        .values()
        .filter_map(|record| match record.requirements().extent() {
            qsl_semantics::family::ClaimExtent::Unbounded(domains) => Some(domains),
            qsl_semantics::family::ClaimExtent::Bounded => None,
        })
        .flat_map(|domains| domains.iter().map(|(key, _)| key))
        .collect();
    let history = locate(&unit, document, &population("Config", "config_history"))
        .expect("config_history resolves")
        .site
        .domain;
    assert_eq!(recorded, BTreeSet::from([&history]));
    let config_version = wire(config_version_node_key(graph));
    assert_eq!(
        history,
        crate::DomainKey::Population {
            member_type: config_version,
            ordinal: 1
        }
    );

    let subs = locate(&unit, document, &population("Config", "aaa_subs"))
        .expect("aaa_subs resolves")
        .site
        .domain;
    let crate::DomainKey::Node { node: sub, .. } =
        locate(&unit, document, &field("Config", "Sub", "alpha"))
            .expect("Sub.alpha resolves")
            .site
            .domain
    else {
        panic!("a field key is a node key");
    };
    assert_eq!(
        subs,
        crate::DomainKey::Population {
            member_type: sub,
            ordinal: 0
        }
    );
}

/// FR-121-AC-22: a population selection whose alias or population does not
/// resolve refuses `UnknownPopulation`, paired with the compiled package's
/// own `package_id`.
#[trace("TC-516", "FR-121-AC-22")]
#[test]
fn call_site_refuses_an_unknown_population_paired_with_its_package() {
    let (unit, packages) = config_version_unit_and_packages_for(two_population_document());
    let [(_, document)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let package = compile_config_version_unit(&unit, &packages)
        .emitted
        .package_id()
        .record();
    for selection in [
        population("Config", "nope"),
        population("Nope", "config_history"),
        population("Config", "ConfigVersion"),
    ] {
        let refusal = locate(&unit, document, &selection)
            .expect_err("the selection names no population of the package");
        assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
        match *refusal {
            CallSiteRefusal::UnknownPopulation {
                selection: refused,
                package: refused_in,
            } => {
                assert_eq!(refused, selection);
                assert_eq!(refused_in, package);
            }
            other => panic!("expected UnknownPopulation for {selection}, got {other:?}"),
        }
    }
}
