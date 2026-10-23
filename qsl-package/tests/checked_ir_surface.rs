// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1111, NFR-015-AC-5: the `qsl-package` surface that exists with `std`
//! disabled. This file builds in both configurations; the `std`-only exports
//! are checked as compile-fail cases in `qsl_package`'s crate docs.

use ix_trace_rs::trace;
use qsl_package::{
    export_bindings, BoundItem, CheckedPackage, ExportRefusal, ResolvedDependency, UnboundElement,
};
use qsl_semantics::check::{CheckedGraph, PackageDeclarations};
use quire_semantic_value::checking::CheckingLimits;

fn empty_graph() -> CheckedGraph {
    let source = qsl_foundation::Source::read(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "u".to_owned(),
        b"",
        0,
    )
    .expect("an empty source reads")
    .reference()
    .clone();
    PackageDeclarations::new(source, qsl_foundation::IdentityLimits::default())
        .check(CheckingLimits::default())
        .expect("an empty package checks")
}

#[trace("TC-1111", "NFR-015-AC-5")]
#[test]
fn link_builds_a_package_with_no_dependencies_or_selections() {
    let package = CheckedPackage::link(empty_graph());
    assert!(package.dependencies().is_empty());
    assert!(package.dependency_selections().is_empty());
    assert!(package.abstraction().is_none());
    let _: &CheckedGraph = package.graph();
    let _shared: std::sync::Arc<CheckedGraph> = package.shared_graph();
    let _: Option<&ResolvedDependency> = package.dependency_selections().values().next();
}

#[trace("TC-1111", "NFR-015-AC-5")]
#[test]
fn bind_abstraction_with_no_bindings_gives_the_package_a_relation() {
    let mut package = CheckedPackage::link(empty_graph());
    package
        .bind_abstraction(&[])
        .expect("no bindings check against an empty package");
    assert!(package.abstraction().is_some());
    package
        .bind_abstraction_at(&[], Vec::new())
        .expect("no bindings check against an empty package");
    assert!(package.abstraction().is_some());
}

#[trace("TC-1111", "NFR-015-AC-5")]
#[test]
fn export_surface_resolves_without_std() {
    let package = CheckedPackage::link(empty_graph());
    let none: Vec<Result<BoundItem, ExportRefusal>> = export_bindings(&package, &[]);
    assert!(none.is_empty());
    let _: Option<UnboundElement> = None; // the type resolves
}
