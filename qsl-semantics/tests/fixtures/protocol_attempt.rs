// SPDX-License-Identifier: AGPL-3.0-or-later
// Original private FR114 model view and source generator, owned by its tests.
/// The original two protocols sharing an ordinary binder name.
pub fn shared_binder_protocols() -> String {
    let first =
        attempt_flow("First", "").replacen("as (tried: Boolean)", "as (shared: Boolean)", 1);
    let second =
        attempt_flow("Second", "").replacen("as (tried: Boolean)", "as (shared: Boolean)", 1);
    // Both protocols are made only of fully checked parts, so
    // the whole package checks: neither refuses `Shadow`.
    format!("{first}\n{second}")
}

use semantics::model::accounting::ModelNormalizationLimits;
use semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord, OperationEffect,
    OperationMemberRecord,
};
use semantics::model::intake::SelectedModel;
use semantics::model::key::DeclarationKey;
use semantics::model::normalize::{normalize, NormalizeOutcome};

/// The owning protocol test's original native header.
pub const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    model M = \"example/protocol-fixture\" version \"1\" digest \
    \"sha256-jcs:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\";\n";

/// `M::Actor` and its own `op` and `other` operations (no parameters,
/// no result, an empty frame), normalized to a [`SelectedModel`] aliased `M` --
/// the FR-114 fixture, so every `attempt ... on M::Actor::op` this
/// module's fixtures already write (FR-114's own assembler resolution,
/// which now runs for every protocol) resolves rather than refusing at
/// assembly. This module's FR-113 tests still exercise anchor
/// resolution and binder shadowing only; FR-114 touches neither.
pub fn m_actor_model() -> SelectedModel {
    let key = |name: &str| DeclarationKey {
        package: "example/protocol-fixture".to_owned(),
        node: format!("ix://example/protocol-fixture/{name}"),
    };
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: "example/protocol-fixture".to_owned(),
            version: "1".to_owned(),
            digest: [0_u8; 32],
        },
        vec![
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: key("Actor"),
                interface_features: None,
                abstract_type: false,
                supertypes: Vec::new(),
            }),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: key("Actor/op"),
                owner: key("Actor"),
                parameters: Vec::new(),
                result: None,
                effect: OperationEffect::default(),
                own_postcondition_clauses: Vec::new(),
                has_body: false,
                redefines: None,
            }),
            // A second operation, so FR-114-AC-3's "a `pre` clause of a
            // different operation" case has one to anchor at.
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: key("Actor/other"),
                owner: key("Actor"),
                parameters: Vec::new(),
                result: None,
                effect: OperationEffect::default(),
                own_postcondition_clauses: Vec::new(),
                has_body: false,
                redefines: None,
            }),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the M::Actor fixture normalizes");
    };
    SelectedModel {
        alias: "M".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
        view,
    }
}

/// A protocol `name` made only of parts the checker covers in
/// full (`super::content`): one role, a `run sequence` holding one
/// `attempt` of `M::Actor::op` with `contracts [contracts]`, and a
/// `finish`, every body the bare literal `true`.
pub fn attempt_flow(name: &str, contracts: &str) -> String {
    format!(
        "protocol {name} using v over (input: Boolean) on origin {{\n\
         role R on M::Actor;\n\
         run sequence Main {{\n\
         attempt Tried by R on M::Actor::op contracts [{contracts}] \
         as (tried: Boolean) {{ true }};\n\
         }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}
