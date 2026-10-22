// SPDX-License-Identifier: AGPL-3.0-or-later
// The model fixture of the protocol obligation tests (FR-381, TC-1057,
// TC-1058), owned by those tests.
use semantics::model::accounting::ModelNormalizationLimits;
use semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, EventTypeRecord, FieldMemberRecord,
    EnumRecord, Multiplicity, NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    RecordValueTypeRecord, ScalarTypeRecord, ScalarValueDomain, ValueTypeRef, VariantRecord,
};
use semantics::model::intake::SelectedModel;
use semantics::model::key::DeclarationKey;
use semantics::model::normalize::{normalize, NormalizeOutcome};

/// The native header of the obligation tests' units: model alias `M`,
/// profile alias `v` and temporal profile alias `t`, which a `compensate`
/// template and an `await` need (QSpec FR-464).
pub const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    profile t = \"quire.temporal.infinite-trace/v1\";\n\
    model M = \"example/protocol-obligations\" version \"1\" digest \
    \"sha256-jcs:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\";\n";

fn key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: "example/protocol-obligations".to_owned(),
        node: format!("ix://example/protocol-obligations/{name}"),
    }
}

fn scalar(name: &str, lower: i128, upper: i128) -> DomainPackageRecord {
    DomainPackageRecord::ScalarType(ScalarTypeRecord {
        key: key(name),
        domain: ScalarValueDomain::Integer { lower, upper },
    })
}

fn field(owner: &str, name: &str, value_type: ValueTypeRef) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        value_type,
        multiplicity: Multiplicity {
            lower: 1,
            upper: Some(1),
            ordered: false,
            unique: true,
        },
        presence: quire_exact::Presence::Required,
        subsets: Vec::new(),
        redefines: None,
    })
}

fn record(name: &str) -> DomainPackageRecord {
    DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
        key: key(name),
        identity_fields: Vec::new(),
        supertypes: Vec::new(),
    })
}

fn package(name: &str) -> ValueTypeRef {
    ValueTypeRef::Package(key(name))
}

fn native(native: NativeValueType) -> ValueTypeRef {
    ValueTypeRef::Native(native)
}

/// The TC-1057 fixture R: the scalar types `Total` (`0..1000`), `Tally`
/// (`0..3`) and `Digit` (`0..9`); the variant type `Mode`; the object type `Node{total, tally, n}`
/// with the operation `step`; the record value types `Receipt{total, tally,
/// n}`, `Attempt{tally}`, `Flag{ready}`, `Tick{ready, count}` and
/// `Hop{ready, node}`; and the event type `Pulse{ready, occurred}`; all
/// normalized to a [`SelectedModel`] aliased `M`.
// Unused by the includers that build their own populated model.
#[allow(dead_code)]
pub fn obligation_model() -> SelectedModel {
    obligation_model_with(Vec::new())
}

/// Fixture R's records followed by `extra`, normalized to a
/// [`SelectedModel`] aliased `M`.
pub fn obligation_model_with(extra: Vec<DomainPackageRecord>) -> SelectedModel {
    let mut records = vec![
        scalar("Total", 0, 1000),
        scalar("Tally", 0, 3),
        scalar("Digit", 0, 9),
        DomainPackageRecord::Enum(EnumRecord {
            key: key("Mode"),
            name: "Mode".to_owned(),
            members: vec!["Off".to_owned(), "On".to_owned()],
        }),
        DomainPackageRecord::Variant(VariantRecord {
            key: key("Mode/Off"),
            owner: key("Mode"),
            case: "Off".to_owned(),
        }),
        DomainPackageRecord::Variant(VariantRecord {
            key: key("Mode/On"),
            owner: key("Mode"),
            case: "On".to_owned(),
        }),
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: key("Node"),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        }),
        field("Node", "total", package("Total")),
        field("Node", "tally", package("Tally")),
        field("Node", "n", package("Digit")),
        DomainPackageRecord::OperationMember(OperationMemberRecord {
            key: key("Node/step"),
            owner: key("Node"),
            parameters: Vec::new(),
            result: None,
            effect: OperationEffect::default(),
            requires: Vec::new(),
            ensures: Vec::new(),
            has_body: false,
            redefines: None,
        }),
        // A second operation with a frame of its own, for a template that
        // names another operation than the protocol's attempt (FR-381-AC-5).
        DomainPackageRecord::OperationMember(OperationMemberRecord {
            key: key("Node/undo"),
            owner: key("Node"),
            parameters: Vec::new(),
            result: None,
            effect: OperationEffect {
                receiver_modifies: vec![key("Node/total")],
                ..OperationEffect::default()
            },
            requires: Vec::new(),
            ensures: Vec::new(),
            has_body: false,
            redefines: None,
        }),
        record("Receipt"),
        field("Receipt", "total", package("Total")),
        field("Receipt", "tally", package("Tally")),
        field("Receipt", "n", package("Digit")),
        record("Attempt"),
        field("Attempt", "tally", package("Tally")),
        record("Flag"),
        field("Flag", "ready", native(NativeValueType::Boolean)),
        record("Tick"),
        field("Tick", "ready", native(NativeValueType::Boolean)),
        field("Tick", "count", package("Digit")),
        record("Hop"),
        field("Hop", "ready", native(NativeValueType::Boolean)),
        field("Hop", "node", package("Node")),
        DomainPackageRecord::EventType(EventTypeRecord {
            key: key("Pulse"),
            identity_fields: Vec::new(),
            occurrence_field: Some(key("Pulse/occurred")),
            supertypes: Vec::new(),
        }),
        field("Pulse", "ready", native(NativeValueType::Boolean)),
        field("Pulse", "occurred", native(NativeValueType::Timestamp)),
    ];
    records.extend(extra);
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: "example/protocol-obligations".to_owned(),
            version: "1".to_owned(),
            digest: [0_u8; 32],
        },
        records,
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the obligation fixture normalizes");
    };
    SelectedModel {
        alias: "M".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
        view,
        operation_clauses: Vec::new(),
        type_invariants: Vec::new(),
    }
}
