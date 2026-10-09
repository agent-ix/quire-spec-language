// SPDX-License-Identifier: AGPL-3.0-or-later
//! Presence refinement through the public checker boundary.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::check_field_refinement_obligation;
use qsl_semantics::model::conformance::ConformanceOutcome;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    NativeValueType, OperationEffect, OperationMemberRecord, PostconditionClause, ValueTypeRef,
};
use qsl_semantics::model::index::ModelIndex;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::normalize::ModelRefusalCause;
use quire_exact::Presence;

fn key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: "test/presence".into(),
        node: format!("ix://test/presence/{name}"),
    }
}

fn index(parent: Presence, child: Presence, upper: u64, proves_presence: bool) -> ModelIndex {
    let field = |name: &str, owner: &str, presence| {
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: key(name),
            owner: key(owner),
            value_type: ValueTypeRef::Native(NativeValueType::Boolean),
            multiplicity: Multiplicity {
                lower: 1,
                upper: Some(upper),
                ordered: true,
                unique: false,
            },
            presence,
            subsets: vec![],
            redefines: (owner == "Child").then(|| key("Parent/x")),
        })
    };
    ModelIndex::build(DomainPackage::new(
        DomainPackageRef {
            identity: "test/presence".into(),
            version: "1".into(),
            digest: [0; 32],
        },
        vec![
            field("Parent/x", "Parent", parent),
            field("Child/x", "Child", child),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: key("Parent/write"),
                owner: key("Parent"),
                parameters: vec![],
                result: None,
                effect: OperationEffect {
                    modifies: vec![key("Parent/x")],
                    creates: vec![],
                    deletes: vec![],
                },
                own_postcondition_clauses: if proves_presence {
                    vec![PostconditionClause::Presence {
                        field: key("Parent/x"),
                    }]
                } else {
                    vec![]
                },
                has_body: false,
                redefines: None,
            }),
        ],
    ))
}

#[trace("FR-082-AC-8")]
#[test]
fn optional_to_required_needs_a_fact_independently_of_multiplicity() {
    for upper in [1, 3] {
        let result = check_field_refinement_obligation(
            &index(Presence::Optional, Presence::Required, upper, false),
            &key("Child/x"),
            &key("Parent/x"),
        )
        .expect("both fields exist");
        let ConformanceOutcome::Refused(failures) = result else {
            panic!("presence narrowing admitted without a fact: {result:?}")
        };
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].axis, "refinement");
        assert_eq!(failures[0].code, Code::UndefinedExpression);
        assert_eq!(failures[0].cause, ModelRefusalCause::UnprovedRefinement);
        assert_eq!(failures[0].detail, "ix://test/presence/Child/x narrows the presence of ix://test/presence/Parent/x with no establishing presence fact (obligation field-presence)");
        for (parent, child, fact) in [
            (Presence::Optional, Presence::Required, true),
            (Presence::Required, Presence::Required, false),
            (Presence::Optional, Presence::Optional, false),
        ] {
            assert_eq!(
                check_field_refinement_obligation(
                    &index(parent, child, upper, fact),
                    &key("Child/x"),
                    &key("Parent/x")
                )
                .unwrap(),
                ConformanceOutcome::Compatible
            );
        }
    }
}
