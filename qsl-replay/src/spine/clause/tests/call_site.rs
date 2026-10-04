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
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
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
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
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
    let replayed = crate::replay(wire).expect_err("no domain package is provided");
    assert!(
        matches!(replayed, crate::ReplayRefusal::Recompile(_)),
        "{replayed:?}"
    );
    assert_eq!(refusal.code(), replayed.code());
}
