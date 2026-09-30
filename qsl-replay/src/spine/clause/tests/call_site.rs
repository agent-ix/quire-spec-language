// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-516 (FR-121): `crate::call_site`'s `Operation` selection over FR-108's
//! ConfigVersion unit, with the `test/config-version` domain package as its
//! package input. `VersionUnchanged` is the unit's one clause naming
//! `attemptUpdate`; `ParentOrder` and `NoCycle` are invariants and name no
//! operation.
//!
//! The expected identities are read by scanning the compiled graph's nodes
//! by semantic form, and the frame's occurrence key is built from the
//! facade's own `OccurrenceKey`, `Origin` and `Role` -- not by re-running
//! `resolve_operation`/`operation_frame`, the lookup `call_site` itself uses.

use super::frame::identifier;
use super::*;
use crate::spine::OperationName;
use crate::{
    call_site, CallSite, CallSiteRefusal, OccurrenceKey, OperationSite, Origin, Role, WireNodeId,
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
