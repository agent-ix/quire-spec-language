// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091's assembler over real complete-V1 source: S1 (`qsl_cst::parse`),
//! S2 (`qsl_forms::build_unit`), then [`PackageDeclarations::assemble`].

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::{SourceIdentity, Span};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, EffectiveId, IeeeWidth, Integer,
    IntegerInterval, Presence, Rational, RoundingMode, ValueType,
};

use super::{
    admit_types_with, model_field, model_operation, model_value_type, AssemblyCause, AssemblyError,
    AssemblyRefusal, TopologyFault, Unmapped,
};
use crate::check::{CheckCause, Owner, PackageDeclarations, SourceOwner, TypeFormFault};
use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::{
    AliasTypeRecord, ClauseRecord, DomainPackage, DomainPackageRecord, DomainPackageRef,
    EnumRecord, FieldMemberRecord, MapTypeRecord, Multiplicity, NamespaceRecord, NativeValueType,
    OperationEffect, OperationMemberRecord, OperationParameterRecord, OperationResult,
    OptionTypeRecord, RecordValueTypeRecord, ScalarTypeRecord, ScalarValueDomain,
    SequenceTypeRecord, StateMachineRecord, StateMachineStateRecord, StateMachineTransitionRecord,
    TuplePositionRecord, TupleValueTypeRecord, ValueTypeRef, VariantRecord,
};
use crate::model::intake::SelectedModel;
use crate::model::normalize::{normalize, NormalizeOutcome};
use crate::value::layer::{HeaderLayer, LayerForm};
use quire_checked::key::DeclarationKey;
use quire_exact::Cancel;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{
    CompositeShape, DeclarationCause, EnvironmentFailure, FieldDeclaration, RecursionEdges,
    StorageRequest, StorageUnit, TypeEnvironmentLimits,
};
use quire_semantic_value::location::{ExpressionLocation as Location, ExpressionOwner as Origin};

const PROFILE_V: &str = "profile v = \"quire.value.complete/v1\";\n";

/// The unit text: the header, `profiles`, then `declarations`.
fn text(profiles: &str, declarations: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\n{profiles}{declarations}\n")
}

/// S1, S2 and the assembler over `text`, under owner (`authority`,
/// `identity`).
fn assemble_as(
    authority: &str,
    identity: &str,
    text: &str,
) -> Result<PackageDeclarations, AssemblyRefusal> {
    let parsed = qsl_cst::parse(
        SourceIdentity::new(authority, identity),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "test/package".to_owned(),
        unit,
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
    )
}

fn assemble(declarations: &str) -> (String, Result<PackageDeclarations, AssemblyRefusal>) {
    let text = text(PROFILE_V, declarations);
    let assembled = assemble_as("a", "u", &text);
    (text, assembled)
}

fn errors(declarations: &str) -> (String, Vec<AssemblyError>) {
    let (text, assembled) = assemble(declarations);
    match assembled {
        Err(refusal) => (text, refusal.errors),
        Ok(_) => panic!("{declarations}: the assembler refuses"),
    }
}

#[test]
fn a_state_machine_meaning_is_emitted_as_an_object_type() {
    let machine = DeclarationKey::fixture("ix://test/orders/OrderLifecycle");
    let draft = DeclarationKey::fixture("ix://test/orders/OrderLifecycle/Draft");
    let placed = DeclarationKey::fixture("ix://test/orders/OrderLifecycle/Placed");
    let advance = DeclarationKey::fixture("ix://test/orders/OrderLifecycle/advance");
    let not_yet_placed = DeclarationKey::fixture("ix://test/orders/OrderLifecycle/NotYetPlaced");
    let consistent = DeclarationKey::fixture("ix://test/orders/OrderLifecycle/Consistent");
    let management = DeclarationKey::fixture("ix://test/orders/OrderManagement");
    let package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.tc235-state-machine-assembly"),
        vec![
            DomainPackageRecord::StateMachine(StateMachineRecord {
                key: machine.clone(),
                supertypes: Vec::new(),
                states: vec![
                    StateMachineStateRecord {
                        key: draft.clone(),
                        name: "Draft".to_owned(),
                    },
                    StateMachineStateRecord {
                        key: placed.clone(),
                        name: "Placed".to_owned(),
                    },
                ],
                transitions: vec![StateMachineTransitionRecord {
                    key: DeclarationKey::fixture("ix://test/orders/OrderLifecycle/place_order"),
                    from: draft,
                    to: placed,
                    trigger: advance.clone(),
                    guard: None,
                    emits: Vec::new(),
                    path: "/types/0/transitions/0".to_owned(),
                    origin: serde_json::Value::Null,
                }],
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: DeclarationKey::fixture("ix://test/orders/OrderLifecycle/status"),
                owner: machine.clone(),
                value_type: ValueTypeRef::Native(NativeValueType::Integer),
                multiplicity: Multiplicity {
                    lower: 1,
                    upper: Some(1),
                    ordered: false,
                    unique: false,
                },
                presence: Presence::Required,
                subsets: Vec::new(),
                redefines: None,
                position: 0,
            }),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: advance,
                owner: machine.clone(),
                parameters: Vec::new(),
                result: None,
                effect: OperationEffect::default(),
                requires: Vec::new(),
                ensures: Vec::new(),
                has_body: true,
                redefines: None,
            }),
            DomainPackageRecord::Clause(ClauseRecord {
                key: not_yet_placed,
                owner: machine.clone(),
                language: "quire".to_owned(),
                text: "not self.status = 1".to_owned(),
            }),
            DomainPackageRecord::Clause(ClauseRecord {
                key: consistent,
                owner: machine.clone(),
                language: "quire".to_owned(),
                text: "self.status = 1".to_owned(),
            }),
            DomainPackageRecord::Namespace(NamespaceRecord {
                key: management,
                members: vec![machine.clone()],
                vocabulary: Vec::new(),
            }),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::default())
    else {
        panic!("the state-machine domain package normalizes");
    };
    let selected = SelectedModel {
        alias: "M".to_owned(),
        span: Span { start: 0, end: 0 },
        view,
        operation_clauses: Vec::new(),
        type_invariants: Vec::new(),
    };
    let mut operations = super::OperationTable::default();
    let super::ModelTypes {
        objects: declarations,
        composites,
        ..
    } = super::model_object_types(
        "M",
        &selected,
        selected.span,
        &mut operations,
        &BTreeMap::new(),
        &[],
    )
    .expect("state-machine meaning is supported by the object-type seam");
    assert!(composites.is_empty());
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].name(), "M::OrderLifecycle");
    assert_eq!(
        declarations[0]
            .attributes()
            .iter()
            .map(FieldDeclaration::name)
            .collect::<Vec<_>>(),
        ["status"]
    );
    let identity = selected
        .view
        .type_identities()
        .get(&DeclarationKey::fixture("ix://test/orders/OrderLifecycle"))
        .expect("normalization indexed the state-machine identity");
    assert_eq!(
        operations
            .declared_by(identity.clone())
            .iter()
            .map(|operation| operation.name())
            .collect::<Vec<_>>(),
        ["advance"]
    );
}

/// QSpec FR-154-AC-33 and FR-120-AC-22: S3 preserves imported tuple
/// positions as an ordered tuple shape in the checked type environment.
#[trace("QSpec-FR-154-AC-33", "FR-120-AC-22")]
#[test]
fn model_tuple_positions_are_assembled_as_a_tuple_type() {
    let tuple = DeclarationKey::fixture("ix://test/orders/Pair");
    let package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.tc1270-tuple-assembly"),
        vec![
            DomainPackageRecord::TupleValueType(TupleValueTypeRecord { key: tuple.clone() }),
            DomainPackageRecord::TuplePosition(TuplePositionRecord {
                key: DeclarationKey::fixture("ix://test/orders/Pair/p0"),
                owner: tuple.clone(),
                ordinal: 0,
                value_type: ValueTypeRef::Native(NativeValueType::Boolean),
            }),
            DomainPackageRecord::TuplePosition(TuplePositionRecord {
                key: DeclarationKey::fixture("ix://test/orders/Pair/p1"),
                owner: tuple,
                ordinal: 1,
                value_type: ValueTypeRef::Native(NativeValueType::Boolean),
            }),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the tuple package normalizes")
    };
    let selected = SelectedModel {
        alias: "M".to_owned(),
        span: Span { start: 0, end: 0 },
        view,
        operation_clauses: Vec::new(),
        type_invariants: Vec::new(),
    };
    let mut operations = super::OperationTable::default();
    let model_types = super::model_object_types(
        "M",
        &selected,
        selected.span,
        &mut operations,
        &BTreeMap::new(),
        &[],
    )
    .expect("the tuple declaration assembles");
    assert_eq!(model_types.composites.len(), 1);
    assert!(matches!(
        model_types.composites[0].shape(),
        CompositeShape::Tuple(positions)
            if positions == &[ValueType::Boolean, ValueType::Boolean]
    ));
}

/// A model scalar type is a nameable type: `M::Total` carries its
/// `Int[lower, upper]` (FR-218-AC-24, FR-381-AC-13).
#[test]
fn a_model_scalar_is_named_with_its_integer_interval() {
    let total = DeclarationKey::fixture("ix://test/orders/Total");
    let package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.model-scalar-name"),
        vec![DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: total,
            domain: ScalarValueDomain::Integer {
                lower: 0,
                upper: 1000,
            },
        })],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::default())
    else {
        panic!("the scalar domain package normalizes");
    };
    let selected = SelectedModel {
        alias: "M".to_owned(),
        span: Span { start: 0, end: 0 },
        view,
        operation_clauses: Vec::new(),
        type_invariants: Vec::new(),
    };
    let mut operations = super::OperationTable::default();
    let types = super::model_object_types(
        "M",
        &selected,
        selected.span,
        &mut operations,
        &BTreeMap::new(),
        &[],
    )
    .expect("a scalar-only model is supported");
    assert_eq!(types.scalars, [("M::Total".to_owned(), int(0, 1000))]);
}

/// FR-056-AC-42: an unresolved unit typeRef identifies the model member path.
#[trace("TC-147", "FR-056-AC-42")]
#[test]
fn unresolved_model_unit_type_names_the_operation_parameter_path() {
    let object = DeclarationKey::fixture("ix://test/orders/Counter");
    let operation = DeclarationKey::fixture("ix://test/orders/Counter/set");
    let parameter = DeclarationKey::fixture("ix://test/orders/Counter/set/value");
    let package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.unresolved-model-unit-type"),
        vec![
            DomainPackageRecord::ObjectType(crate::model::domain_package::ObjectTypeRecord {
                key: object.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: Vec::new(),
            }),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: operation,
                owner: object,
                parameters: vec![OperationParameterRecord {
                    key: parameter,
                    value_type: ValueTypeRef::Unit("missing".to_owned()),
                    multiplicity: Multiplicity {
                        lower: 1,
                        upper: Some(1),
                        ordered: false,
                        unique: false,
                    },
                }],
                result: None,
                effect: OperationEffect::default(),
                requires: Vec::new(),
                ensures: Vec::new(),
                has_body: true,
                redefines: None,
            }),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::default())
    else {
        panic!("the unit-free domain package normalizes");
    };
    let selected = SelectedModel {
        alias: "M".to_owned(),
        span: Span { start: 10, end: 20 },
        view,
        operation_clauses: Vec::new(),
        type_invariants: Vec::new(),
    };
    let mut operations = super::OperationTable::default();
    let errors = match super::model_object_types(
        "M",
        &selected,
        selected.span,
        &mut operations,
        &BTreeMap::new(),
        &[],
    ) {
        Ok(_) => panic!("the unresolved unit typeRef refuses during model assembly"),
        Err(errors) => errors,
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].cause,
        AssemblyCause::UnresolvedModelQuantityType {
            name: "missing".to_owned(),
            member_identity: "ix://test/orders/Counter/set/value".to_owned(),
            member_path: "Operation.params[0].typeRef".to_owned(),
        }
    );
    assert_eq!(errors[0].span, selected.span);
    assert_eq!(
        errors[0].cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
}

/// The span of the last occurrence of `needle` in `text`.
fn last(text: &str, needle: &str) -> Span {
    let start = text.rfind(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// The span of the first occurrence of `needle` in `text` at or after
/// `from`.
fn after(text: &str, from: &str, needle: &str) -> Span {
    let from = text.find(from).expect("the anchor is in the unit");
    let start = from + text[from..].find(needle).expect("the needle follows");
    Span {
        start,
        end: start + needle.len(),
    }
}

fn int(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(lower.into(), upper.into()).expect("a nonempty interval"))
}

#[trace("FR-091-AC-12", "TC-399")]
#[test]
fn the_assembler_resolves_aliases_and_signatures() {
    let (_, assembled) = assemble(
        "type Digit = Int[0, 9];\n\
         function inc using v(x: Digit): Int[0, 10] pure { x + 1 }\n\
         function two using v(): Int[0, 10] pure { inc(1) }",
    );
    let package = assembled.expect("the unit assembles");
    assert_eq!(package.aliases, [("Digit".to_owned(), int(0, 9))]);
    let names: Vec<&str> = package
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(names, ["inc", "two"]);
    let (parameters, result) = package
        .resolved_signatures
        .get(0)
        .expect("inc's resolved signature");
    assert_eq!(parameters, &[("x".to_owned(), int(0, 9))]);
    assert_eq!(result, &int(0, 10));
    let (parameters, result) = package
        .resolved_signatures
        .get(1)
        .expect("two's resolved signature");
    assert!(parameters.is_empty());
    assert_eq!(result, &int(0, 10));
    package
        .check(CheckingLimits::default())
        .expect("the package checks");
}

#[trace(
    "FR-091-AC-14",
    "FR-091-AC-15",
    "FR-091-AC-16",
    "FR-091-AC-17",
    "TC-400"
)]
#[test]
fn the_assembler_reports_every_error() {
    let (text, found) = errors(
        "function f using v(x: Missing): Boolean pure { true }\n\
         function g using v(y: Absent): Boolean pure { true }",
    );
    assert_eq!(
        found,
        [
            AssemblyError {
                cause: AssemblyCause::UnresolvedTypeName {
                    name: "Missing".into()
                },
                span: last(&text, "Missing"),
            },
            AssemblyError {
                cause: AssemblyCause::UnresolvedTypeName {
                    name: "Absent".into()
                },
                span: last(&text, "Absent"),
            },
        ]
    );

    let (text, found) = errors(
        "type A = Int[0, 1];\ntype A = Int[0, 2];\n\
         function f using v(x: A): Boolean pure { true }",
    );
    let candidates = vec![
        after(&text, "type A = Int[0, 1]", "A"),
        after(&text, "type A = Int[0, 2]", "A"),
    ];
    assert_eq!(
        found,
        [
            AssemblyError {
                cause: AssemblyCause::AmbiguousTypeName {
                    name: "A".into(),
                    candidates: candidates.clone(),
                    native: false,
                },
                span: candidates[1],
            },
            AssemblyError {
                cause: AssemblyCause::AmbiguousTypeName {
                    name: "A".into(),
                    candidates,
                    native: false,
                },
                span: after(&text, "(x: A)", "A"),
            },
        ]
    );

    let (text, found) = errors(
        "function f using v(x: Int[9, 0]): Boolean pure { true }\n\
         function g using v(x: Rational[0, 1; 0, 5]): Boolean pure { true }\n\
         function h using v(x: Text[5, 1; nfc]): Boolean pure { true }",
    );
    assert_eq!(
        found,
        [
            AssemblyError {
                cause: AssemblyCause::IllFormedBounds(TypeFormFault::EmptyInterval),
                span: last(&text, "Int[9, 0]"),
            },
            AssemblyError {
                cause: AssemblyCause::IllFormedBounds(TypeFormFault::DenominatorBelowOne),
                span: last(&text, "Rational[0, 1; 0, 5]"),
            },
            AssemblyError {
                cause: AssemblyCause::IllFormedBounds(TypeFormFault::EmptyTextBounds),
                span: last(&text, "Text[5, 1; nfc]"),
            },
        ]
    );

    let (_, found) = errors("type A = B;\ntype B = A;");
    let [AssemblyError {
        cause: AssemblyCause::AliasCycle { edges },
        ..
    }] = found.as_slice()
    else {
        panic!("one alias cycle, not {found:?}");
    };
    assert_eq!(edges, &[("A".into(), "B".into()), ("B".into(), "A".into())]);
    assert_eq!(
        found[0].cause.catalog_code().to_string(),
        "invalid_package/definition-cycle"
    );
    let (_, found) = errors("type C = Option<C>;");
    assert_eq!(
        found
            .iter()
            .map(|error| error.cause.clone())
            .collect::<Vec<_>>(),
        [AssemblyCause::AliasCycle {
            edges: vec![("C".into(), "C".into())]
        }]
    );
}

/// The key `check` minted for the record or tuple named `name`.
fn composite_key(package: &PackageDeclarations, name: &str) -> quire_exact::NodeKey {
    package
        .types
        .composites()
        .find(|declaration| declaration.name() == name)
        .map(|declaration| declaration.key())
        .unwrap_or_else(|| panic!("{name} is declared"))
}

#[trace("FR-091-AC-18", "TC-401")]
#[test]
fn records_and_tuples_are_keyed_over_the_units_owner() {
    let text = text(
        PROFILE_V,
        "record Point { x: Int[0, 9]; y: Int[0, 9]; }\n\
         tuple Pair(Int[0, 9], Int[0, 9]);\n\
         function px using v(p: Point): Int[0, 9] pure { p.x }",
    );
    let keys = |identity: &str| {
        let package = assemble_as("a", identity, &text).expect("the unit assembles");
        let point = composite_key(&package, "Point");
        let pair = composite_key(&package, "Pair");
        assert_ne!(point, pair);
        let (parameters, _) = package
            .resolved_signatures
            .get(0)
            .expect("px's resolved signature");
        assert_eq!(
            parameters,
            &[("p".to_owned(), ValueType::Composite(point.clone()))]
        );
        let graph = package
            .check(CheckingLimits::default())
            .expect("the package checks");
        let px = graph.function_identity("px").expect("px is checked");
        (point, pair, px)
    };
    let first = keys("u");
    assert_eq!(keys("u"), first);
}

#[trace("FR-091-AC-19", "FR-091-AC-23", "TC-405")]
#[test]
fn floating_types_are_admitted_and_reference_types_are_refused() {
    let admitted = |parameter: &str| {
        let (_, assembled) = assemble(&format!(
            "function f using v(x: {parameter}): Boolean pure {{ true }}"
        ));
        assembled.expect("a floating type is admitted")
    };
    // Resolution to `ValueType::Float` is TC-405's type_form test; here the
    // assembler admits every spelling without a floating-type refusal.
    admitted("Float64[nearest-even]");
    admitted("Float32[toward-zero]");
    admitted("Float64");
    let (text, found) = errors("function g using v(r: Reference<M::T>): Boolean pure { true }");
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::UnresolvedTypeName {
                name: "M::T".into()
            },
            span: last(&text, "M::T"),
        }]
    );
}

/// FR-326: the assembler keeps each `temporal` clause's form for the
/// `TemporalTrace` check, and records the unit's temporal profile
/// selections; it refuses a clause whose `using` alias names no profile
/// selection at that alias.
#[trace("FR-326")]
#[test]
fn the_assembler_keeps_temporal_clauses_and_refuses_an_undeclared_alias() {
    let clause = |alias: &str| {
        format!(
            "temporal A using {alias} over (p: Boolean) clock \"s\" on origin {{ holds(true) }}"
        )
    };
    let profiles = format!("{PROFILE_V}profile t = \"quire.temporal.infinite-trace/v1\";\n");
    let text = text(&profiles, &clause("t"));
    let assembled = assemble_as("a", "u", &text).expect("the unit assembles");
    assert_eq!(assembled.temporal_clauses.len(), 1);
    assert_eq!(
        assembled
            .temporal_selections
            .iter()
            .map(|selection| selection.profile)
            .collect::<Vec<_>>(),
        [crate::check::TemporalProfile::InfiniteTrace]
    );

    let (text, found) = errors(&clause("missing"));
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::UndeclaredAlias {
                alias: "missing".into()
            },
            span: last(&text, "missing"),
        }]
    );
}

/// FR-124: a `terminal` declaration resolves its `using` alias and its model
/// alias; one on an alias that names no `model` declaration refuses
/// `missing_declaration`/`missing-name` at the alias.
#[trace("TC-1059", "FR-382-AC-6")]
#[test]
fn a_terminal_declaration_on_no_model_refuses_at_the_model_alias() {
    for declaration in [
        "terminal Done using v on M when { true };",
        "terminal Halt using v on M any;",
    ] {
        let (text, found) = errors(declaration);
        assert_eq!(
            found,
            [AssemblyError {
                cause: AssemblyCause::UnresolvedTerminalModel { alias: "M".into() },
                span: after(&text, "on", "M"),
            }],
            "{declaration}"
        );
        assert_eq!(found[0].cause.code(), Code::MissingDeclaration);
        assert_eq!(
            found[0].cause.catalog_code(),
            quire_semantic_value::catalog::CatalogCode::new("missing_declaration", "missing-name")
        );
    }
}

/// FR-124: a `terminal` declaration's `using` alias names a profile
/// selection of the unit.
#[test]
fn a_terminal_declaration_using_no_profile_refuses_at_the_alias() {
    let (text, found) = errors("terminal Halt using missing on M any;");
    assert!(
        found.contains(&AssemblyError {
            cause: AssemblyCause::UndeclaredAlias {
                alias: "missing".into()
            },
            span: last(&text, "missing"),
        }),
        "{found:?}"
    );
}

#[trace("FR-091-AC-8", "TC-396")]
#[test]
fn nested_constructs_of_other_families_refuse_with_their_own_causes() {
    let checked = |declarations: &str| {
        // `deref` is `state.graph`'s (FR-361 item 17), so the unit selects
        // the model profile, which holds it.
        let text = text(
            &PROFILE_V.replace("quire.value.complete", "quire.model.complete"),
            declarations,
        );
        let assembled = assemble_as("a", "u", &text);
        let refusals = assembled
            .expect("the unit assembles")
            .check(CheckingLimits::default())
            .expect_err("check refuses");
        assert!(
            refusals
                .iter()
                .all(|refusal| refusal.cause.code() != qsl_foundation::Code::UnsupportedConstruct),
            "no step-3 refusal is unsupported_construct: {refusals:?}"
        );
        refusals
    };
    for declarations in [
        "function f using v(x: Int[0, 9]): Boolean pure { pre(x) }",
        "function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }",
    ] {
        let refusals = checked(declarations);
        assert!(
            refusals.iter().any(|refusal| refusal.cause.code()
                == qsl_foundation::Code::WrongSnapshot
                && refusal.cause.cause() == Some("forbidden-pre-read")),
            "{declarations}: {refusals:?}"
        );
    }
    let refusals = checked("function f using v(x: Int[0, 9]): Boolean pure { deref(x) }");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
        )),
        "{refusals:?}"
    );
    let (_, found) =
        errors("function f using v(x: Int[0, 9]): Boolean pure { allInstances<M::T>(x) }");
    assert!(matches!(
        found.as_slice(),
        [AssemblyError {
            cause: AssemblyCause::UnresolvedTypeName { name },
            ..
        }] if name == "M::T"
    ));
    assert_eq!(
        found[0].cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
}

#[trace("FR-091-AC-21", "TC-406")]
#[test]
fn each_assembler_cause_has_its_catalog_code() {
    let span = Span { start: 0, end: 0 };
    let cases = [
        (
            AssemblyCause::UnresolvedTypeName { name: "X".into() },
            "missing_declaration/missing-name",
        ),
        (
            AssemblyCause::AmbiguousTypeName {
                name: "X".into(),
                candidates: vec![span],
                native: false,
            },
            "ambiguous_declaration/ambiguous-name",
        ),
        (
            AssemblyCause::IllFormedBounds(TypeFormFault::EmptyInterval),
            "ill_typed/type-mismatch",
        ),
        (
            AssemblyCause::FloatingType {
                width: IeeeWidth::Binary32,
                rounding: RoundingMode::Exact,
                profile: None,
            },
            "unknown_required_feature/unsupported-feature",
        ),
        (
            AssemblyCause::AliasCycle { edges: Vec::new() },
            "invalid_package/definition-cycle",
        ),
        (
            AssemblyCause::UndeclaredAlias { alias: "w".into() },
            "missing_declaration/missing-selection",
        ),
        (
            AssemblyCause::DuplicateAlias {
                alias: "v".into(),
                spans: vec![span],
            },
            "ambiguous_declaration/ambiguous-name",
        ),
        (
            AssemblyCause::UnresolvedOperation {
                context: "M::T".into(),
                operation: "op".into(),
            },
            "missing_declaration/missing-name",
        ),
        (
            AssemblyCause::AmbiguousOperation {
                context: "M::T".into(),
                operation: "op".into(),
            },
            "ambiguous_declaration/ambiguous-name",
        ),
        (
            AssemblyCause::AmbiguousPopulation {
                context: "M::T".into(),
                populations: Vec::new(),
            },
            "ambiguous_declaration/ambiguous-name",
        ),
    ];
    for (cause, code) in cases {
        assert_eq!(cause.catalog_code().to_string(), code, "{cause:?}");
    }
}

/// FR-082-AC-10: assembler allocation refusals preserve the kernel's exact
/// request amount and unit, including element counts that are not bytes.
#[trace("FR-082-AC-10")]
#[test]
fn assembler_allocation_cause_preserves_native_request_unit() {
    for (request, expected_unit) in [
        (StorageRequest::new(6, StorageUnit::Bytes), "bytes"),
        (
            StorageRequest::new(3, StorageUnit::AdditionalElements),
            "additional-elements",
        ),
    ] {
        let refusal = admit_types_with(
            Vec::new(),
            &[],
            &BTreeMap::new(),
            TypeEnvironmentLimits::default(),
            &Cancel::new(),
            |_, _, _, _| -> Result<(), _> { Err(EnvironmentFailure::Allocation(request)) },
        )
        .expect_err("injected registry allocation failure must refuse admission");
        let [error] = refusal.errors.as_slice() else {
            panic!("registry allocation failure must produce one assembly error")
        };
        let cause = &error.cause;
        assert_eq!(
            cause.catalog_code().to_string(),
            "resource_exhausted/allocation-failed"
        );
        assert_eq!(
            cause.storage_request(),
            Some((request.amount(), request.unit()))
        );
        let fields =
            BTreeMap::from([("requested", format!("{expected_unit}:{}", request.amount()))]);
        assert_eq!(cause.catalog_fields(), Some(fields));
    }
}

/// FR-082-AC-11: reservation size overflow is distinct from allocation
/// denial and preserves the kernel's exact request amount and unit.
#[trace("FR-082-AC-11")]
#[test]
fn assembler_capacity_cause_preserves_native_request_unit() {
    for (request, expected_upstream) in [
        (StorageRequest::new(9, StorageUnit::Bytes), "bytes:9"),
        (
            StorageRequest::new(4, StorageUnit::AdditionalElements),
            "additional-elements:4",
        ),
    ] {
        let refusal = admit_types_with(
            Vec::new(),
            &[],
            &BTreeMap::new(),
            TypeEnvironmentLimits::default(),
            &Cancel::new(),
            |_, _, _, _| -> Result<(), _> { Err(EnvironmentFailure::Capacity(request)) },
        )
        .expect_err("injected reservation overflow must refuse admission");
        let [error] = refusal.errors.as_slice() else {
            panic!("reservation overflow must produce one assembly error")
        };
        let cause = &error.cause;
        assert_eq!(
            cause.catalog_code().to_string(),
            "resource_exhausted/reservation-size-overflow"
        );
        assert_eq!(
            cause.storage_request(),
            Some((request.amount(), request.unit()))
        );
        assert_eq!(
            cause.catalog_fields(),
            Some(BTreeMap::from([("upstream", expected_upstream.to_owned())]))
        );
    }
}

/// FR-113: the assembler keeps a `protocol` declaration's form
/// rather than refusing it -- resolving its scoped anchors is
/// `check::protocol_clause`'s job at S3, not the assembler's at E3, so a
/// protocol with an anchor naming no declaration (`effect Applied of
/// Missing`) still assembles. No `attempt` here: FR-114 resolves
/// an attempt's own operation at assembly time, so an attempt naming an
/// unadmitted model (this fixture declares no `model M = ...;`) would
/// refuse there instead -- `an_attempt_naming_an_unadmitted_model_refuses_
/// at_assembly` below covers that.
#[trace("TC-510", "FR-112")]
#[test]
fn a_protocol_declaration_assembles_leaving_anchor_resolution_to_s3() {
    let (_, assembled) = assemble(
        "protocol Flow using p over (input: M::Input) on origin {\n\
         role R on M::Actor;\n\
         run sequence Main {\n\
         effect Applied of Missing as (applied: M::Effect) { true };\n\
         }\n\
         finish End as (outcome: Boolean) { true };\n\
         }",
    );
    let package = assembled.unwrap_or_else(|refusal| panic!("{refusal:?}"));
    assert_eq!(package.protocols.len(), 1);
    assert_eq!(package.protocols[0].name.name, "Flow");
}

/// Unlike FR-113's own anchor resolution
/// (deferred to S3, above), an `attempt`'s `on M::T::op` operation is
/// resolved by the FR-091 assembler itself, mirroring a `pre`/`post`
/// clause's own resolution (FR-104) -- so a context naming no admitted
/// model refuses `missing_declaration`/`missing-name` at assembly, the same
/// way an unresolved state-clause context does.
#[trace("TC-513", "FR-114")]
#[test]
fn an_attempt_naming_an_unadmitted_model_refuses_at_assembly() {
    let (_, errors) = errors(
        "protocol Flow using p over (input: M::Input) on origin {\n\
         role R on M::Actor;\n\
         run sequence Main {\n\
         attempt Tried by R on M::Actor::act contracts [] as (tried: Boolean) { true };\n\
         }\n\
         finish End as (outcome: Boolean) { true };\n\
         }",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    match &errors[0].cause {
        AssemblyCause::UnresolvedTypeName { name } => assert_eq!(name, "M::Actor"),
        other => panic!("an UnresolvedTypeName cause, got {other:?}"),
    }
}

/// FR-104 "Resolution" (TC-460 row 1's shape, over a unit with no `model`
/// declaration): the assembler keeps a state clause and resolves its
/// context, so a context `M::T` naming no admitted object type refuses
/// `missing_declaration`/`missing-name` at the name -- never an
/// unsupported-feature refusal of the whole clause, and never a silent
/// drop. TC-459 to TC-461 check clauses over an admitted model
/// (`qsl-semantics/tests/it/state_clauses.rs`).
#[trace("TC-460", "FR-104-AC-3")]
#[test]
fn a_state_clause_context_resolves_or_refuses_at_its_name() {
    let clause = "invariant Foo using v on Config::ConfigVersion at current { true }";
    let (text, found) = errors(&format!(
        "{clause}\nfunction ok using v(): Boolean pure {{ true }}\n"
    ));
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::UnresolvedTypeName {
                name: "Config::ConfigVersion".into()
            },
            span: after(&text, "on ", "Config::ConfigVersion"),
        }]
    );
    assert_eq!(
        found[0].cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
}

#[trace("FR-091-AC-22", "TC-412")]
#[test]
fn using_aliases_resolve_to_the_units_profile_selections() {
    let (unit_text, assembled) = assemble("function f using v(): Boolean pure { true }");
    let package = assembled.expect("f's alias names the profile selection v");
    assert_eq!(
        package.functions[0]
            .using()
            .map(|using| using.alias.as_str()),
        Some("v")
    );
    let selection = package
        .function_selections
        .get(&0)
        .expect("f's alias resolved to a recorded selection");
    assert_eq!(selection.alias, "v");
    assert_eq!(selection.identity, "quire.value.complete/v1");
    assert_eq!(
        &unit_text[selection.span.start..selection.span.end],
        PROFILE_V.trim_end()
    );
    assert_eq!(
        &unit_text[selection.identity_span.start..selection.identity_span.end],
        "\"quire.value.complete/v1\""
    );
    let (text, found) = errors(
        "function f using v(): Boolean pure { true }\n\
         function g using w(): Boolean pure { true }",
    );
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::UndeclaredAlias { alias: "w".into() },
            span: after(&text, "function g using", "w"),
        }]
    );

    let second = PROFILE_V.replace("aaaa", "bbbb");
    let text = text_with(
        &[PROFILE_V, &second],
        "function f using v(): Boolean pure { true }",
    );
    match assemble_as("a", "u", &text) {
        Err(refusal) => {
            let [AssemblyError {
                cause: AssemblyCause::DuplicateAlias { alias, spans },
                ..
            }] = refusal.errors.as_slice()
            else {
                panic!("one duplicate-alias error, not {refusal:?}");
            };
            assert_eq!(alias, "v");
            let spelled: Vec<&str> = spans
                .iter()
                .map(|span| &text[span.start..span.end])
                .collect();
            assert_eq!(spelled, [PROFILE_V.trim_end(), second.trim_end()]);
            assert_eq!(
                refusal.errors[0].cause.catalog_code().to_string(),
                "ambiguous_declaration/ambiguous-name"
            );
        }
        Ok(_) => panic!("a duplicate alias refuses"),
    }
}

fn text_with(profiles: &[&str], declarations: &str) -> String {
    text(&profiles.concat(), declarations)
}

/// Every path segment and `use` name in shipped items that is `qsl_cst`.
#[derive(Default)]
struct CstEdges(usize);

impl<'ast> syn::visit::Visit<'ast> for CstEdges {
    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        if segment.ident == "qsl_cst" {
            self.0 += 1;
        }
        syn::visit::visit_path_segment(self, segment);
    }

    fn visit_use_path(&mut self, path: &'ast syn::UsePath) {
        if path.ident == "qsl_cst" {
            self.0 += 1;
        }
        syn::visit::visit_use_path(self, path);
    }

    fn visit_use_name(&mut self, name: &'ast syn::UseName) {
        if name.ident == "qsl_cst" {
            self.0 += 1;
        }
    }
}

#[trace("FR-091-AC-20", "TC-402")]
#[test]
fn the_assembler_reads_no_cst() {
    use syn::visit::Visit as _;
    assert!(module_path!().starts_with("qsl_semantics::check::assemble"));
    for (name, source) in [
        ("assemble.rs", include_str!("../assemble.rs")),
        ("assemble/units.rs", include_str!("units.rs")),
    ] {
        let file = syn::parse_file(source).expect("an assembler file parses");
        let mut edges = CstEdges::default();
        edges.visit_file(&file);
        assert_eq!(
            edges.0, 0,
            "{name}: the assembler's shipped code names qsl_cst"
        );
    }
}

/// Whether a `use` tree names `qsl_cst` anywhere.
fn use_tree_names_cst(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Path(path) => path.ident == "qsl_cst" || use_tree_names_cst(&path.tree),
        syn::UseTree::Name(name) => name.ident == "qsl_cst",
        syn::UseTree::Rename(rename) => rename.ident == "qsl_cst",
        syn::UseTree::Glob(_) => false,
        syn::UseTree::Group(group) => group.items.iter().any(use_tree_names_cst),
    }
}

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory reads") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// The `qsl_cst` re-exports in `dir`: a non-private `use` of a path rooted at
/// `qsl_cst`, or a non-private `type` alias whose type names a `qsl_cst`
/// path. Returns `path:line` findings.
fn cst_re_exports(dir: &std::path::Path) -> Vec<String> {
    /// Whether a type names a path rooted at `qsl_cst`.
    struct NamesCst(bool);
    impl<'ast> syn::visit::Visit<'ast> for NamesCst {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            if path
                .segments
                .first()
                .is_some_and(|first| first.ident == "qsl_cst")
            {
                self.0 = true;
            }
            syn::visit::visit_path(self, path);
        }
    }
    struct ReExports(Vec<usize>);
    impl<'ast> syn::visit::Visit<'ast> for ReExports {
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            if !matches!(item.vis, syn::Visibility::Inherited) && use_tree_names_cst(&item.tree) {
                self.0.push(item.use_token.span.start().line);
            }
        }
        fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
            let mut names = NamesCst(false);
            syn::visit::Visit::visit_type(&mut names, &item.ty);
            if !matches!(item.vis, syn::Visibility::Inherited) && names.0 {
                self.0.push(item.type_token.span.start().line);
            }
        }
    }
    use syn::visit::Visit as _;
    let mut files = Vec::new();
    rust_files(dir, &mut files);
    assert!(files.len() > 3, "the scan reads {}", dir.display());
    let mut found = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path).expect("a source file reads");
        let file = syn::parse_file(&source).expect("a source file parses");
        let mut visitor = ReExports(Vec::new());
        visitor.visit_file(&file);
        found.extend(
            visitor
                .0
                .iter()
                .map(|line| format!("{}:{line}", path.display())),
        );
    }
    found
}

/// TC-402 step 2, re-exported types: neither `qsl-forms` (through which the
/// assembler reaches the forms types) nor `qsl-semantics` re-exports a
/// `qsl_cst` item, by `pub use` or by a `pub type` alias. With no such
/// re-export in either crate, the assembler has no path to a CST type
/// except `qsl_cst` itself, which `the_assembler_reads_no_cst` refuses.
#[trace("FR-091-AC-20", "TC-402")]
#[test]
fn no_qsl_cst_type_is_re_exported_to_the_assembler() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for dir in [manifest.join("src"), manifest.join("../qsl-forms/src")] {
        found.extend(cst_re_exports(&dir));
    }
    assert!(found.is_empty(), "qsl_cst is re-exported: {found:?}");
}

/// TC-402 step 3: the assembler's test code reaches `qsl_cst` only to run
/// S1 (`parse`, `parse_source`) and to build the `Limits` argument of that
/// call.
#[trace("FR-091-AC-20", "TC-402")]
#[test]
fn the_assembler_tests_reach_qsl_cst_only_to_run_s1() {
    struct Reach(Vec<String>, usize);
    impl Reach {
        fn judge(&mut self, names: &[String]) {
            if names.first().is_some_and(|first| first == "qsl_cst") {
                self.1 += 1;
                match names.get(1).map(String::as_str) {
                    Some("parse" | "parse_source" | "Limits") => {}
                    _ => self.0.push(names.join("::")),
                }
            }
        }
        fn use_tree(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
            match tree {
                syn::UseTree::Path(path) => {
                    prefix.push(path.ident.to_string());
                    self.use_tree(&path.tree, prefix);
                    prefix.pop();
                }
                syn::UseTree::Name(name) => {
                    let mut full = prefix.clone();
                    full.push(name.ident.to_string());
                    self.judge(&full);
                }
                syn::UseTree::Rename(rename) => {
                    let mut full = prefix.clone();
                    full.push(rename.ident.to_string());
                    self.judge(&full);
                }
                syn::UseTree::Glob(_) => self.judge(prefix),
                syn::UseTree::Group(group) => {
                    for item in &group.items {
                        self.use_tree(item, prefix);
                    }
                }
            }
        }
        fn tokens(&mut self, stream: proc_macro2::TokenStream) {
            use proc_macro2::TokenTree;
            let tokens: Vec<TokenTree> = stream.into_iter().collect();
            let mut i = 0;
            while i < tokens.len() {
                match &tokens[i] {
                    TokenTree::Group(group) => self.tokens(group.stream()),
                    TokenTree::Ident(root) => {
                        let mut names = vec![root.to_string()];
                        let mut j = i + 1;
                        while let (
                            Some(TokenTree::Punct(a)),
                            Some(TokenTree::Punct(b)),
                            Some(TokenTree::Ident(next)),
                        ) = (tokens.get(j), tokens.get(j + 1), tokens.get(j + 2))
                        {
                            if a.as_char() != ':' || b.as_char() != ':' {
                                break;
                            }
                            names.push(next.to_string());
                            j += 3;
                        }
                        self.judge(&names);
                        i = j;
                        continue;
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }
    impl<'ast> syn::visit::Visit<'ast> for Reach {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            self.judge(&names);
            syn::visit::visit_path(self, path);
        }
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            self.use_tree(&item.tree, &mut Vec::new());
        }
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            self.tokens(mac.tokens.clone());
            syn::visit::visit_macro(self, mac);
        }
    }
    use syn::visit::Visit as _;
    let file = syn::parse_file(include_str!("tests.rs")).expect("tests.rs parses");
    let mut reach = Reach(Vec::new(), 0);
    reach.visit_file(&file);
    assert!(reach.1 > 0, "the scan sees the S1 call");
    assert!(
        reach.0.is_empty(),
        "a test reaches qsl_cst beyond S1: {:?}",
        reach.0
    );
}

/// FR-096: a record read from the unit places its `declaration`
/// occurrence at its declared name.
#[trace("FR-096-AC-1", "FR-091-AC-18")]
#[test]
fn a_declared_type_resolves_to_its_names_region() {
    let text = text(
        PROFILE_V,
        "record Point { x: Int[0, 9]; }\n\
         function px using v(p: Point): Int[0, 9] pure { p.x }",
    );
    let package = assemble_as("a", "u", &text).expect("the unit assembles");
    let location = Location::root(Origin::TypeDeclaration {
        name: "Point".into(),
    });
    let region = package.region(&location).expect("Point has a region");
    let span = last(&text, "Point {");
    assert_eq!(
        (region.start(), region.end()),
        (span.start as u64, span.start as u64 + 5)
    );
    let graph = package
        .check(CheckingLimits::default())
        .expect("the package checks");
    assert_eq!(graph.region(&location), Some(region));
}

/// A declared type name binds one declaration whichever kind comes first:
/// every declaration after the first is refused, in either order.
#[trace("FR-091-AC-15", "TC-400")]
#[test]
fn a_duplicate_type_name_is_refused_in_either_order() {
    for declarations in [
        "record P { x: Integer; }\ntype P = Boolean;",
        "type P = Boolean;\nrecord P { x: Integer; }",
    ] {
        let (text, found) = errors(declarations);
        let first = after(&text, declarations.lines().next().unwrap(), "P");
        let second = after(&text, declarations.lines().nth(1).unwrap(), "P");
        assert_eq!(
            found,
            [AssemblyError {
                cause: AssemblyCause::AmbiguousTypeName {
                    name: "P".into(),
                    candidates: vec![first, second],
                    native: false,
                },
                span: second,
            }],
            "{declarations}"
        );
    }
}

/// FR-317-AC-3 (TC-820 step 3): one unit cannot declare two types named `q`
/// -- two unions, or a union and an enum -- so the member resolver never sees
/// them. The assembler refuses at the second declaration.
#[trace("FR-317-AC-3", "TC-820")]
#[test]
fn two_types_named_q_are_refused_at_the_second_declaration() {
    for declarations in [
        "union q { m }\nunion q { m }",
        "union q { m }\nenum q { m }",
    ] {
        let (text, found) = errors(declarations);
        // The first and the last `q {` of the unit, each at its name.
        let first = after(&text, "union q {", "q");
        let second = Span {
            start: text.rfind("q {").expect("the second declaration"),
            end: text.rfind("q {").expect("the second declaration") + 1,
        };
        assert_eq!(
            found,
            [AssemblyError {
                cause: AssemblyCause::AmbiguousTypeName {
                    name: "q".into(),
                    candidates: vec![first, second],
                    native: false,
                },
                span: second,
            }],
            "{declarations}"
        );
    }
}

/// A type form inside a body resolves at the assembler, so its ill-formed
/// bounds refuse there as a signature's do.
#[trace("FR-091-AC-16", "TC-400")]
#[test]
fn a_body_type_forms_bounds_refuse_at_the_assembler() {
    let (text, found) =
        errors("function f using v(x: Int[0, 9]): Boolean pure { convert<Int[9, 0]>(x) = x }");
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::IllFormedBounds(TypeFormFault::EmptyInterval),
            span: last(&text, "Int[9, 0]"),
        }]
    );
}

/// The named types of `fold`, `count` and `sum` are refused at their own
/// names' spans, measure before body, in source order.
#[trace("FR-091-AC-14", "TC-400")]
#[test]
fn accumulator_type_names_are_refused_at_their_own_spans_in_source_order() {
    let (text, found) = errors(
        "function f using v(c: Integer): Integer pure decreases(sum<Early>(x in c: x)) \
         { count<Late>(x in c: true) + fold<Missing>(acc, y in c: acc, identity: 0) }",
    );
    let unresolved = |name: &str| AssemblyError {
        cause: AssemblyCause::UnresolvedTypeName { name: name.into() },
        span: last(&text, name),
    };
    assert_eq!(
        found,
        [
            unresolved("Early"),
            unresolved("Late"),
            unresolved("Missing")
        ]
    );
    assert_eq!(&text[found[2].span.start..found[2].span.end], "Missing");
}

/// Every FR-143 refusal is reported, not only the first.
#[trace("FR-091-AC-18", "TC-401")]
#[test]
fn every_record_refusal_is_reported() {
    let (text, found) = errors(
        "record P { x: Integer; x: Integer; }\n\
         record Q { y: Integer; y: Integer; }",
    );
    let names: Vec<&str> = found
        .iter()
        .map(|error| &text[error.span.start..error.span.end])
        .collect();
    assert_eq!(names, ["P", "Q"]);
    for error in &found {
        assert!(
            matches!(
                &error.cause,
                AssemblyCause::InvalidTypeDeclaration(invalid)
                    if matches!(invalid.cause, quire_semantic_value::declaration::DeclarationCause::DuplicateMember(_))
            ),
            "{error:?}"
        );
    }
}

// `model_field` follows QSpec's own Presence row
// (`model-complete.md`:158) and FR-322's "Model-owned members" step 4:
// multiplicity alone gives the value type, and `presence` alone -- never a
// lower bound of `0` -- gives `Option`.

/// A native-`Integer` field member at `multiplicity`/`presence`, with no
/// redefinition. `model_field` never consults `records`/`identities` for a
/// native value type, so [`model_field_of`] passes empty maps.
fn presence_field(multiplicity: Multiplicity, presence: Presence) -> FieldMemberRecord {
    FieldMemberRecord {
        key: DeclarationKey::fixture("model.A/f"),
        owner: DeclarationKey::fixture("model.A"),
        value_type: ValueTypeRef::Native(NativeValueType::Integer),
        multiplicity,
        presence,
        subsets: Vec::new(),
        redefines: None,
        position: 0,
    }
}

fn model_field_of(
    multiplicity: Multiplicity,
    presence: Presence,
) -> Result<FieldDeclaration, Unmapped> {
    let field = presence_field(multiplicity, presence);
    let records: BTreeMap<&DeclarationKey, &DomainPackageRecord> = BTreeMap::new();
    let identities: BTreeMap<DeclarationKey, EffectiveId> = BTreeMap::new();
    let meaning_handles = BTreeMap::new();
    model_field(
        &field,
        &records,
        &identities,
        &meaning_handles,
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
}

fn mult(lower: u64, upper: Option<u64>, ordered: bool, unique: bool) -> Multiplicity {
    Multiplicity {
        lower,
        upper,
        ordered,
        unique,
    }
}

fn bounded(kind: CollectionKind, lower: u64, upper: u64) -> ValueType {
    ValueType::collection(CollectionType::new(
        kind,
        ValueType::Integer,
        Some(CardinalityBound::new(lower, upper).unwrap()),
    ))
}

/// FR-322's table: `[1, 1]` gives the element type `E` outright, whatever
/// `ordered`/`unique` says.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn one_one_required_gives_the_element_type() {
    let declaration = model_field_of(mult(1, Some(1), false, true), Presence::Required).unwrap();
    assert_eq!(declaration.value_type(), &ValueType::Integer);
    assert_eq!(declaration.presence(), Presence::Required);
}

#[trace("TC-897", "FR-056-AC-17")]
#[test]
fn timestamp_alias_field_assembles_with_its_distinct_kernel_type() {
    let mut field = presence_field(mult(1, Some(1), false, true), Presence::Required);
    let alias_key = DeclarationKey::fixture("ix://test/orders/Instant");
    field.value_type = ValueTypeRef::Package(alias_key.clone());
    let alias = DomainPackageRecord::ScalarType(ScalarTypeRecord {
        key: alias_key.clone(),
        domain: ScalarValueDomain::Native(NativeValueType::Timestamp),
    });
    let records: BTreeMap<&DeclarationKey, &DomainPackageRecord> =
        BTreeMap::from([(&alias_key, &alias)]);
    let identities: BTreeMap<DeclarationKey, EffectiveId> = BTreeMap::new();
    let meaning_handles = BTreeMap::new();

    let declaration = model_field(
        &field,
        &records,
        &identities,
        &meaning_handles,
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .expect("native Timestamp is supported by the shared value kernel");

    assert_eq!(*declaration.value_type(), ValueType::Timestamp);
}

/// Native `String` is a distinct checked leaf, not profile-bounded `Text`.
#[trace("FR-056-AC-11")]
#[test]
fn native_string_field_assembles_as_a_distinct_kernel_type() {
    let mut field = presence_field(mult(1, Some(1), false, true), Presence::Required);
    field.value_type = ValueTypeRef::Native(NativeValueType::String);
    let records: BTreeMap<&DeclarationKey, &DomainPackageRecord> = BTreeMap::new();
    let identities: BTreeMap<DeclarationKey, EffectiveId> = BTreeMap::new();
    let meaning_handles = BTreeMap::new();

    let declaration = model_field(
        &field,
        &records,
        &identities,
        &meaning_handles,
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .expect("native String is supported by the shared value kernel");

    assert_eq!(*declaration.value_type(), ValueType::String);
    assert_ne!(
        *declaration.value_type(),
        ValueType::Text(
            quire_exact::TextType::new(0, 1, quire_exact::TextProfile::UnicodeScalars,)
                .expect("valid Text bounds")
        )
    );
}

/// A lower bound of `0` makes an empty collection legal; it never makes a
/// field optional (QSpec's Presence row). `[0, 1]` required is the bounded
/// collection `K<E>[0, 1]`, never `Option<E>`.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn zero_one_required_gives_the_bounded_collection_not_option() {
    let declaration = model_field_of(mult(0, Some(1), false, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 0, 1)
    );
    assert_eq!(declaration.presence(), Presence::Required);
}

/// FR-016-AC-10's `present(x.f)` clause, retired with the root domain checker:
/// a required `[0, 1]` field is a bounded collection (the test above), not an
/// `Option`, so `present` over a value of that type refuses
/// `ill_typed`/`type-mismatch` while `present` over an `Option` checks.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn present_over_a_zero_one_collection_refuses_ill_typed() {
    let check = |declarations: &str| {
        let text = text(
            &PROFILE_V.replace("quire.value.complete", "quire.model.complete"),
            declarations,
        );
        assemble_as("a", "u", &text)
            .expect("the unit assembles")
            .check(CheckingLimits::default())
    };
    let refusals =
        check("function f using v(x: Sequence<Integer>[0, 1]): Boolean pure { present(x) }")
            .expect_err("a bounded collection has no presence");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
        )),
        "{refusals:?}"
    );
    check("function g using v(x: Option<Integer>): Boolean pure { present(x) }")
        .expect("`present` over an Option checks");
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn zero_five_required_gives_the_bounded_collection() {
    let declaration = model_field_of(mult(0, Some(5), false, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 0, 5)
    );
    assert_eq!(declaration.presence(), Presence::Required);
}

/// `[0, unbounded]` gives the unbounded collection `K<E>`, with no
/// `collection_bounds` node -- distinct from an unbounded upper with a
/// lower bound above `0`, which has no kernel type at all.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn zero_unbounded_gives_the_unbounded_collection() {
    let declaration = model_field_of(mult(0, None, false, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &ValueType::collection(CollectionType::new(
            CollectionKind::Set,
            ValueType::Integer,
            None
        ))
    );
    assert_eq!(declaration.presence(), Presence::Required);
}

/// An unbounded upper bound with a lower bound above `0` has no kernel
/// type at all (FR-322's table); the assembler refuses it. QSpec's own
/// merged member-type vectors (`model-member-type-vectors.json` MA-07) pin
/// this same refusal at `[1, unbounded]`.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn one_unbounded_refuses() {
    let outcome = model_field_of(mult(1, None, false, true), Presence::Required);
    assert!(matches!(outcome, Err(Unmapped::Unsupported)), "{outcome:?}");
}

/// Optional presence over `[1, 1]` leaves `model_field`'s own value type as
/// the plain element `E` -- `check`'s `attribute`/`field` readers wrap an
/// `Optional`-presence declaration's value type in `Option` from
/// `presence()` alone (`check/check.rs`'s `attribute`/`field`), exactly as
/// they already do for every other `FieldDeclaration`, giving FR-322's
/// table's `Option<E>` without `model_field` baking `Option` in itself.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn optional_presence_over_one_one_is_optional_not_multiplicity_driven() {
    let declaration = model_field_of(mult(1, Some(1), false, true), Presence::Optional).unwrap();
    assert_eq!(declaration.value_type(), &ValueType::Integer);
    assert_eq!(declaration.presence(), Presence::Optional);
}

/// An option value type's type-level wrapper is projected independently of
/// the field Presence property (QSpec FR-154-AC-39 / FR-322-AC-298).
#[trace("QSpec-FR-154-AC-39", "QSpec-FR-322-AC-298")]
#[test]
fn option_value_type_projects_option_and_preserves_field_presence() {
    let option_key = DeclarationKey::fixture("model.OptInt");
    let option = DomainPackageRecord::OptionType(OptionTypeRecord {
        key: option_key.clone(),
        inner: ValueTypeRef::Native(NativeValueType::Integer),
    });
    let records = BTreeMap::from([(&option_key, &option)]);
    let identities = BTreeMap::new();
    let meaning_handles = BTreeMap::new();
    let variants = BTreeMap::new();
    let units = BTreeMap::new();
    let field = FieldMemberRecord {
        key: DeclarationKey::fixture("model.Holder/a"),
        owner: DeclarationKey::fixture("model.Holder"),
        value_type: ValueTypeRef::Package(option_key.clone()),
        multiplicity: mult(1, Some(1), false, true),
        presence: Presence::Required,
        subsets: Vec::new(),
        redefines: None,
        position: 0,
    };

    let declaration = model_field(
        &field,
        &records,
        &identities,
        &meaning_handles,
        &variants,
        &units,
    )
    .unwrap();
    assert_eq!(
        declaration.value_type(),
        &ValueType::option(ValueType::Integer)
    );
    assert_eq!(declaration.presence(), Presence::Required);

    let mut optional_native_field =
        presence_field(mult(1, Some(1), false, true), Presence::Optional);
    optional_native_field.value_type = ValueTypeRef::Native(NativeValueType::Integer);
    let no_records = BTreeMap::new();
    let optional_native = model_field(
        &optional_native_field,
        &no_records,
        &identities,
        &meaning_handles,
        &variants,
        &units,
    )
    .unwrap();
    assert_eq!(optional_native.value_type(), &ValueType::Integer);
    assert_eq!(optional_native.presence(), Presence::Optional);
}

/// Multiplicity wraps the option element itself (FR-154-AC-39 / FR-322-AC-298).
#[trace("QSpec-FR-154-AC-39", "QSpec-FR-322-AC-298")]
#[test]
fn option_field_multiplicity_wraps_the_option_element() {
    let option_key = DeclarationKey::fixture("model.OptInt");
    let option = DomainPackageRecord::OptionType(OptionTypeRecord {
        key: option_key.clone(),
        inner: ValueTypeRef::Native(NativeValueType::Integer),
    });
    let records = BTreeMap::from([(&option_key, &option)]);
    let identities = BTreeMap::new();
    let meaning_handles = BTreeMap::new();
    let variants = BTreeMap::new();
    let units = BTreeMap::new();
    let field = FieldMemberRecord {
        key: DeclarationKey::fixture("model.Holder/d"),
        owner: DeclarationKey::fixture("model.Holder"),
        value_type: ValueTypeRef::Package(option_key.clone()),
        multiplicity: mult(0, Some(3), true, false),
        presence: Presence::Required,
        subsets: Vec::new(),
        redefines: None,
        position: 0,
    };
    let declaration = model_field(
        &field,
        &records,
        &identities,
        &meaning_handles,
        &variants,
        &units,
    )
    .unwrap();
    assert_eq!(
        declaration.value_type(),
        &ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            ValueType::option(ValueType::Integer),
            Some(CardinalityBound::new(0, 3).unwrap()),
        ))
    );
    assert_eq!(declaration.presence(), Presence::Required);
}

/// FR-505-AC-1/7: model Rational bounds become a bounded exact value type, and
/// an authored unit becomes the same carrier identity FR-497 consumes.
#[trace("FR-505-AC-1", "FR-505-AC-7", "TC-1300")]
#[test]
fn rational_value_types_project_with_bounds_and_nominal_unit() {
    let key = DeclarationKey::fixture("model.Ratio");
    let bounds = quire_exact::RationalDomain::new(
        IntegerInterval::spanning(Integer::from(-3_i64), Integer::from(7_i64)),
        IntegerInterval::spanning(Integer::one(), Integer::from(12_i64)),
    )
    .expect("valid rational bounds");
    let bounded = DomainPackageRecord::ScalarType(ScalarTypeRecord {
        key: key.clone(),
        domain: ScalarValueDomain::Rational {
            domain: Some(bounds.clone()),
            unit: None,
        },
    });
    let unbounded_key = DeclarationKey::fixture("model.UnboundedRatio");
    let unbounded = DomainPackageRecord::ScalarType(ScalarTypeRecord {
        key: unbounded_key.clone(),
        domain: ScalarValueDomain::Rational {
            domain: None,
            unit: None,
        },
    });
    let unit_key = DeclarationKey::fixture("model.RatioInMeters");
    let unit_bearing = DomainPackageRecord::ScalarType(ScalarTypeRecord {
        key: unit_key.clone(),
        domain: ScalarValueDomain::Rational {
            domain: Some(bounds.clone()),
            unit: Some("m".to_owned()),
        },
    });
    let records = [&bounded, &unbounded, &unit_bearing]
        .into_iter()
        .map(|record| (record.key(), record))
        .collect();
    let identities = BTreeMap::new();
    let handles = BTreeMap::new();
    let variants = BTreeMap::new();
    let metre = quire_exact::UnitId::declared(
        quire_exact::NodeKey::new("test", vec![quire_exact::PathSegment::Text("m".into())], 0)
            .expect("unit key"),
    );
    let units = BTreeMap::from([("m".to_owned(), metre.clone())]);
    let single = mult(1, Some(1), false, true);

    assert_eq!(
        model_value_type(
            &ValueTypeRef::Package(key),
            single,
            &records,
            &identities,
            &handles,
            &variants,
            &units,
        )
        .unwrap(),
        ValueType::Rational(bounds.clone())
    );
    assert!(matches!(
        model_value_type(
            &ValueTypeRef::Package(unbounded_key),
            single,
            &records,
            &identities,
            &handles,
            &variants,
            &units,
        ),
        Err(Unmapped::Untyped(_))
    ));
    assert_eq!(
        model_value_type(
            &ValueTypeRef::Package(unit_key),
            single,
            &records,
            &identities,
            &handles,
            &variants,
            &units,
        )
        .unwrap(),
        ValueType::carried_quantity(quire_exact::CarriedQuantityType::new(
            metre,
            quire_exact::QuantityCarrier::Rational(bounds),
        ))
    );
}

/// QSpec FR-154 admits Option<T> at the same declared value-type positions
/// as T, and each existing projection path must retain the Option wrapper.
#[trace("QSpec-FR-154-AC-39", "QSpec-FR-322-AC-298")]
#[test]
fn option_type_projects_through_signature_alias_sequence_map_tuple_variant_unit_and_record() {
    let option_key = DeclarationKey::fixture("model.OptInt");
    let alias_key = DeclarationKey::fixture("model.OptAlias");
    let sequence_key = DeclarationKey::fixture("model.OptSequence");
    let map_key = DeclarationKey::fixture("model.OptMap");
    let tuple_key = DeclarationKey::fixture("model.Pair");
    let tuple_option_key = DeclarationKey::fixture("model.OptPair");
    let variant_key = DeclarationKey::fixture("model.Result");
    let variant_option_key = DeclarationKey::fixture("model.OptResult");
    let record_key = DeclarationKey::fixture("model.Row");
    let record_option_key = DeclarationKey::fixture("model.OptRow");
    let unit_option_key = DeclarationKey::fixture("model.OptMs");
    let operation_key = DeclarationKey::fixture("model.Holder/read");
    let parameter_key = DeclarationKey::fixture("model.Holder/read/value");

    let records = vec![
        DomainPackageRecord::OptionType(OptionTypeRecord {
            key: option_key.clone(),
            inner: ValueTypeRef::Native(NativeValueType::Integer),
        }),
        DomainPackageRecord::AliasType(AliasTypeRecord {
            key: alias_key.clone(),
            target: ValueTypeRef::Package(option_key.clone()),
        }),
        DomainPackageRecord::SequenceType(SequenceTypeRecord {
            key: sequence_key.clone(),
            items: ValueTypeRef::Package(option_key.clone()),
        }),
        DomainPackageRecord::MapType(MapTypeRecord {
            key: map_key.clone(),
            keys: ValueTypeRef::Native(NativeValueType::String),
            values: ValueTypeRef::Package(option_key.clone()),
            lower: 0,
            upper: None,
        }),
        DomainPackageRecord::TupleValueType(TupleValueTypeRecord {
            key: tuple_key.clone(),
        }),
        DomainPackageRecord::TuplePosition(TuplePositionRecord {
            key: DeclarationKey::fixture("model.Pair/p0"),
            owner: tuple_key.clone(),
            ordinal: 0,
            value_type: ValueTypeRef::Package(option_key.clone()),
        }),
        DomainPackageRecord::Enum(EnumRecord {
            key: variant_key.clone(),
            name: "Result".to_owned(),
            members: vec!["Ok".to_owned(), "Error".to_owned()],
        }),
        DomainPackageRecord::Variant(VariantRecord {
            key: DeclarationKey::fixture("model.Result-Ok"),
            owner: variant_key.clone(),
            case: "Ok".to_owned(),
            payload_type: Some(ValueTypeRef::Package(option_key.clone())),
        }),
        DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
            key: record_key.clone(),
            identity_fields: Vec::new(),
            supertypes: Vec::new(),
        }),
        DomainPackageRecord::OptionType(OptionTypeRecord {
            key: tuple_option_key.clone(),
            inner: ValueTypeRef::Package(tuple_key.clone()),
        }),
        DomainPackageRecord::OptionType(OptionTypeRecord {
            key: variant_option_key.clone(),
            inner: ValueTypeRef::Package(variant_key.clone()),
        }),
        DomainPackageRecord::OptionType(OptionTypeRecord {
            key: record_option_key.clone(),
            inner: ValueTypeRef::Package(record_key.clone()),
        }),
        DomainPackageRecord::OptionType(OptionTypeRecord {
            key: unit_option_key.clone(),
            inner: ValueTypeRef::Unit("ms".to_owned()),
        }),
        DomainPackageRecord::OperationMember(OperationMemberRecord {
            key: operation_key.clone(),
            owner: DeclarationKey::fixture("model.Holder"),
            parameters: vec![OperationParameterRecord {
                key: parameter_key,
                value_type: ValueTypeRef::Package(option_key.clone()),
                multiplicity: mult(1, Some(1), false, true),
            }],
            result: Some(OperationResult {
                value_type: ValueTypeRef::Package(option_key.clone()),
                multiplicity: mult(1, Some(1), false, true),
            }),
            effect: OperationEffect::default(),
            requires: Vec::new(),
            ensures: Vec::new(),
            has_body: false,
            redefines: None,
        }),
    ];
    let by_key: BTreeMap<_, _> = records
        .iter()
        .map(|record| (record.key(), record))
        .collect();
    let node_key = |name: &str| {
        quire_exact::NodeKey::new(
            "test",
            vec![quire_exact::PathSegment::Text(name.to_owned())],
            0,
        )
        .expect("fixture node key")
    };
    let tuple_handle = node_key("tuple");
    let variant_handle = node_key("variant");
    let record_handle = node_key("record");
    let handles = BTreeMap::from([
        (tuple_key.clone(), tuple_handle.clone()),
        (variant_key.clone(), variant_handle.clone()),
        (record_key.clone(), record_handle.clone()),
    ]);
    let unit_key = node_key("milliseconds");
    let units = BTreeMap::from([("ms".to_owned(), quire_exact::UnitId::declared(unit_key))]);
    let identities = BTreeMap::new();
    let variants = BTreeMap::new();
    let single = mult(1, Some(1), false, true);
    let project = |value_type: &ValueTypeRef| {
        model_value_type(
            value_type,
            single,
            &by_key,
            &identities,
            &handles,
            &variants,
            &units,
        )
    };

    assert_eq!(
        project(&ValueTypeRef::Package(alias_key)).unwrap(),
        ValueType::option(ValueType::Integer)
    );
    assert_eq!(
        project(&ValueTypeRef::Package(sequence_key)).unwrap(),
        ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            ValueType::option(ValueType::Integer),
            None,
        ))
    );
    assert_eq!(
        project(&ValueTypeRef::Package(map_key)).unwrap(),
        ValueType::Map(std::sync::Arc::new(
            quire_exact::MapType::new(
                ValueType::String,
                ValueType::option(ValueType::Integer),
                None,
            )
            .expect("a leaf key"),
        ))
    );
    assert_eq!(
        project(&ValueTypeRef::Package(tuple_option_key)).unwrap(),
        ValueType::option(ValueType::Composite(tuple_handle))
    );
    assert_eq!(
        project(&ValueTypeRef::Package(variant_option_key)).unwrap(),
        ValueType::option(ValueType::Composite(variant_handle))
    );
    assert_eq!(
        project(&ValueTypeRef::Package(record_option_key)).unwrap(),
        ValueType::option(ValueType::Composite(record_handle))
    );
    assert_eq!(
        project(&ValueTypeRef::Package(unit_option_key)).unwrap(),
        ValueType::option(ValueType::Quantity(quire_exact::UnitId::declared(
            node_key("milliseconds")
        )))
    );

    let tuple_position = by_key
        .values()
        .find_map(|record| match record {
            DomainPackageRecord::TuplePosition(position) => Some(position),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        project(&tuple_position.value_type).unwrap(),
        ValueType::option(ValueType::Integer)
    );
    let variant_payload = by_key
        .values()
        .find_map(|record| match record {
            DomainPackageRecord::Variant(variant) => variant.payload_type.as_ref(),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        project(variant_payload).unwrap(),
        ValueType::option(ValueType::Integer)
    );

    let operation = by_key
        .values()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) => Some(operation),
            _ => None,
        })
        .unwrap();
    let declaration = model_operation(operation, &by_key, &identities, &handles, &variants, &units)
        .unwrap()
        .0;
    assert_eq!(
        declaration.parameters(),
        &[("value".to_owned(), ValueType::option(ValueType::Integer))]
    );
    assert_eq!(
        declaration.result(),
        Some(&ValueType::option(ValueType::Integer))
    );
}

/// Projection refuses values that would produce an unsupported nested option
/// or optional clock place (FR-154-AC-40).
#[trace("QSpec-FR-154-AC-40")]
#[test]
fn option_projection_rejects_nested_option_and_clock_inner_types() {
    let option_key = DeclarationKey::fixture("model.OptInt");
    for inner in [
        ValueTypeRef::Package(option_key.clone()),
        ValueTypeRef::Clock,
    ] {
        let option = DomainPackageRecord::OptionType(OptionTypeRecord {
            key: option_key.clone(),
            inner,
        });
        let records = BTreeMap::from([(&option_key, &option)]);
        let field = FieldMemberRecord {
            key: DeclarationKey::fixture("model.Holder/a"),
            owner: DeclarationKey::fixture("model.Holder"),
            value_type: ValueTypeRef::Package(option_key.clone()),
            multiplicity: mult(1, Some(1), false, true),
            presence: Presence::Required,
            subsets: Vec::new(),
            redefines: None,
            position: 0,
        };
        let outcome = model_field(
            &field,
            &records,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert!(matches!(outcome, Err(Unmapped::Unsupported)), "{outcome:?}");
    }
}

/// Optional presence composes with a bounded collection exactly the same
/// way: the collection type is unaffected, and only `presence()` carries
/// the `Option` wrapping `check`'s readers apply at read time.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn optional_presence_over_zero_three_wraps_the_collection_not_the_bound() {
    let declaration = model_field_of(mult(0, Some(3), false, true), Presence::Optional).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 0, 3)
    );
    assert_eq!(declaration.presence(), Presence::Optional);
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn optional_presence_over_zero_one_wraps_the_collection_not_the_bound() {
    let declaration = model_field_of(mult(0, Some(1), false, true), Presence::Optional).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 0, 1)
    );
    assert_eq!(declaration.presence(), Presence::Optional);
}

/// Each `ordered`/`unique` combination names its own collection kind
/// (FR-322's table), over a finite multiplicity distinct from the
/// `[1, 1]`/`[0, 1]` special cases above.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn ordered_and_unique_selects_ordered_set() {
    let declaration = model_field_of(mult(2, Some(4), true, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::OrderedSet, 2, 4)
    );
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn ordered_and_not_unique_selects_sequence() {
    let declaration = model_field_of(mult(2, Some(4), true, false), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Sequence, 2, 4)
    );
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn unordered_and_unique_selects_set() {
    let declaration = model_field_of(mult(2, Some(4), false, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 2, 4)
    );
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn unordered_and_not_unique_selects_bag() {
    let declaration = model_field_of(mult(2, Some(4), false, false), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Bag, 2, 4)
    );
}

// ----------------------------------------------------------------------
// Enums and predicates (TC-481)
// ----------------------------------------------------------------------

const STATUS: &str = "ordered enum Status { READY, DONE }\n";

fn binding<'a>(package: &'a PackageDeclarations, name: &str) -> &'a crate::check::EnumBinding {
    package
        .enums
        .iter()
        .find(|binding| binding.name == name)
        .unwrap_or_else(|| panic!("{name} is declared"))
}

fn cases(binding: &crate::check::EnumBinding) -> Vec<&str> {
    binding.members.iter().map(|member| member.case()).collect()
}

#[trace("FR-091-AC-27", "TC-481")]
#[test]
fn enums_are_admitted_with_nominal_keys_over_the_units_owner() {
    let (text, assembled) =
        assemble("ordered enum Status { READY, DONE }\nenum Color { RED, BLUE = \"Blue\" }");
    let package = assembled.expect("the unit assembles");
    let names: Vec<&str> = package.enums.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Status", "Color"]);
    let status = binding(&package, "Status");
    assert_eq!(cases(status), ["READY", "DONE"]);
    assert_ne!(status.members[0].member(), status.members[1].member());
    let color = binding(&package, "Color");
    assert_ne!(status.declaration.key(), color.declaration.key());
    assert_eq!(cases(color), ["BLUE", "RED"]);
    assert_eq!(
        package.declared_type_spans.get("Status"),
        Some(&last(&text, "Status"))
    );
    assert_eq!(
        package.declared_type_spans.get("Color"),
        Some(&last(&text, "Color"))
    );

    // Neither a display string nor the order of an unordered enum's cases
    // enters its key.
    let (_, other) = assemble("enum Color { RED, BLUE }");
    let other = other.expect("the unit assembles");
    assert_eq!(
        binding(&other, "Color").declaration.key(),
        color.declaration.key()
    );

    // The owner is the node's `owner`, not part of its structural key
    // (package, declared path, ordinal).
    let moved =
        assemble_as("a", "w", &text_with(&[PROFILE_V], STATUS)).expect("the unit assembles");
    assert_eq!(
        binding(&moved, "Status").declaration.key(),
        status.declaration.key()
    );
}

#[trace("FR-091-AC-28", "TC-481")]
#[test]
fn enum_types_and_members_check_and_a_case_the_enum_lacks_refuses() {
    let checked = |declarations: &str| {
        let (_, assembled) = assemble(declarations);
        assembled
            .expect("the unit assembles")
            .check(CheckingLimits::default())
    };
    let package = assemble(&format!(
        "{STATUS}\
         function isReady using v(s: Status): Boolean pure {{ s = Status::READY }}\n\
         function ok using v(): Boolean pure {{ isReady(Status::READY) }}\n\
         function later using v(): Boolean pure {{ Status::READY < Status::DONE }}"
    ))
    .1
    .expect("the unit assembles");
    let shape = binding(&package, "Status").shape();
    let (parameters, _) = package
        .resolved_signatures
        .get(0)
        .expect("isReady's resolved signature");
    assert_eq!(parameters, &[("s".to_owned(), ValueType::Enum(shape))]);
    package
        .check(CheckingLimits::default())
        .expect("the package checks");

    let refusals = checked(
        "enum Color { RED, BLUE }\n\
         function bad using v(): Boolean pure { Color::RED < Color::BLUE }",
    )
    .expect_err("an unordered enum has no order");
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.cause.code() == qsl_foundation::Code::IllTyped),
        "{refusals:?}"
    );

    let refusals = checked(&format!(
        "{STATUS}function gone using v(): Boolean pure {{ Status::GONE = Status::READY }}"
    ))
    .expect_err("Status has no GONE");
    assert!(
        refusals.iter().any(|refusal| {
            refusal.cause.code() == qsl_foundation::Code::MissingDeclaration
                && refusal.cause.cause() == Some("missing-name")
                && matches!(
                    &refusal.cause,
                    CheckCause::MissingQualifiedName(name) if name.name == "Status::GONE"
                )
        }),
        "{refusals:?}"
    );
}

#[trace("FR-091-AC-29", "TC-481")]
#[test]
fn enum_and_type_name_errors_are_all_reported() {
    let (text, found) = errors(
        "enum E { A, B, A }\n\
         enum F { X }\n\
         record F { y: Boolean; }\n\
         function f using v(p: F): Boolean pure { true }\n\
         function g using v(p: Shade): Boolean pure { true }",
    );
    let first_a = after(&text, "enum E", "A");
    let second_a = last(&text, "A }");
    let second_a = Span {
        start: second_a.start,
        end: second_a.start + 1,
    };
    let duplicate = found
        .iter()
        .find(|error| matches!(error.cause, AssemblyCause::DuplicateEnumMember { .. }))
        .expect("a duplicate-enum-member error");
    assert_eq!(
        duplicate.cause,
        AssemblyCause::DuplicateEnumMember {
            enumeration: "E".to_owned(),
            case: "A".to_owned(),
            spans: vec![first_a, second_a],
        }
    );
    assert_eq!(
        duplicate.cause.catalog_code().to_string(),
        "ambiguous_declaration/ambiguous-name"
    );
    let enum_f = after(&text, "enum F", "F");
    let record_f = after(&text, "record F", "F");
    let type_form = after(&text, "function f", "F)");
    let type_form = Span {
        start: type_form.start,
        end: type_form.start + 1,
    };
    assert!(
        found.iter().any(|error| error.span == type_form
            && error.cause
                == AssemblyCause::AmbiguousTypeName {
                    name: "F".to_owned(),
                    candidates: vec![enum_f, record_f],
                    native: false,
                }),
        "{found:?}"
    );
    assert!(
        found.iter().any(|error| matches!(
            &error.cause,
            AssemblyCause::UnresolvedTypeName { name } if name == "Shade"
        )),
        "{found:?}"
    );
}

#[trace("FR-091-AC-21", "TC-406")]
#[test]
fn the_enum_causes_have_their_catalog_codes() {
    let span = Span { start: 0, end: 0 };
    assert_eq!(
        AssemblyCause::DuplicateEnumMember {
            enumeration: "E".into(),
            case: "A".into(),
            spans: vec![span],
        }
        .catalog_code()
        .to_string(),
        "ambiguous_declaration/ambiguous-name"
    );
    let fault = quire_semantic_value::semantic_node::InvalidSemanticGraph {
        cause: quire_semantic_value::semantic_node::SemanticGraphCause::StaleKey,
    };
    assert_eq!(
        AssemblyCause::NominalAdmission(fault)
            .catalog_code()
            .to_string(),
        "runtime_invariant/established-invariant-broken"
    );
}

#[trace("FR-091-AC-30", "TC-481")]
#[test]
fn a_predicate_assembles_as_a_function_of_kind_predicate() {
    let package = assemble(
        "predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }\n\
         function three using v(): Boolean pure { Positive(3) }",
    )
    .1
    .expect("the unit assembles");
    let kinds: Vec<(&str, qsl_forms::DeclarationKind)> = package
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function.kind()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("Positive", qsl_forms::DeclarationKind::Predicate),
            ("three", qsl_forms::DeclarationKind::Function)
        ]
    );
    let (parameters, result) = package
        .resolved_signatures
        .get(0)
        .expect("Positive's resolved signature");
    assert_eq!(parameters, &[("x".to_owned(), int(0, 9))]);
    assert_eq!(result, &ValueType::Boolean);
    assert_eq!(package.function_selections[&0].alias, "v");
    package
        .check(CheckingLimits::default())
        .expect("the package checks");

    let (text, found) = errors("predicate Q using w(x: Boolean): Boolean { x }");
    assert!(matches!(
        found.as_slice(),
        [AssemblyError {
            cause: AssemblyCause::UndeclaredAlias { alias },
            span,
        }] if alias == "w" && *span == after(&text, "predicate Q", "w")
    ));

    let refusals = assemble("predicate R using v(x: Boolean): Boolean { R(x) }")
        .1
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("a recursive predicate has no measure");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::UnprovedDecrease {
                obligation: crate::check::MeasureObligation::MissingMeasure,
                ..
            }
        )),
        "{refusals:?}"
    );
}

#[trace("FR-092-AC-13", "TC-481")]
#[test]
fn predicate_and_enum_nodes_lower_to_their_fr_092_forms() {
    use crate::check::{NodeTag, Owner, SourceOwner};

    let predicate = assemble("predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }")
        .1
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let function = assemble("function Positive using v(x: Int[0, 9]): Boolean pure { x > 0 }")
        .1
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let node_of = |graph: &crate::check::CheckedGraph| {
        let key = graph
            .function_identity("Positive")
            .expect("Positive is checked");
        graph
            .semantic_graph()
            .node(key)
            .expect("the node is lowered")
            .clone()
    };
    let (predicate, function) = (node_of(&predicate), node_of(&function));
    assert_eq!(predicate.node_tag(), NodeTag::Function);
    assert_eq!(predicate.semantic_form(), "predicate");
    let names: Vec<&str> = predicate
        .declaration()
        .expect("a declaration")
        .iter()
        .map(|segment| segment.as_str())
        .collect();
    assert_eq!(names, ["Positive"]);
    assert_eq!(
        predicate.owner(),
        Some(&Owner::Source(
            SourceOwner::new("a", "u").expect("an owner")
        ))
    );
    assert_eq!(function.semantic_form(), "pure_function");
    // A declared node's key is its package and declared path: the form
    // (`predicate`, `pure_function`) is the node's own, not part of its key.
    assert_eq!(predicate.key(), function.key());

    let package = assemble(STATUS).1.expect("the unit assembles");
    let status = binding(&package, "Status");
    let (declaration_key, member_key) = (status.declaration.key(), status.members[0].member());
    let graph = package
        .check(CheckingLimits::default())
        .expect("the package checks");
    let by_key = |key: &quire_exact::NodeKey| {
        graph
            .semantic_graph()
            .nodes()
            .find(|node| node.key() == *key)
            .unwrap_or_else(|| panic!("the checked graph holds {key}"))
    };
    let declaration = by_key(&declaration_key);
    assert_eq!(declaration.node_tag(), NodeTag::ScalarType);
    assert_eq!(declaration.semantic_form(), "enum");
    let member = by_key(&member_key);
    assert_eq!(member.node_tag(), NodeTag::Value);
    assert_eq!(member.semantic_form(), "enum_value");
}

#[trace("FR-091-AC-27", "TC-481")]
#[test]
fn the_preimage_constructors_apply_the_reader_checks() {
    use crate::value::enumeration::{EnumDeclarationPreimage, EnumMemberPreimage};
    use crate::value::semantic_node::{NodeOwner, OwnerSubject};

    let owner = || {
        NodeOwner::Source(OwnerSubject {
            authority: "a".to_owned(),
            identity: "u".to_owned(),
            package_name: "p".to_owned(),
        })
    };
    let name = || vec!["E".to_owned()];
    let cases = |cases: &[&str]| cases.iter().map(|case| (*case).to_owned()).collect();
    assert!(EnumDeclarationPreimage::new(owner(), name(), false, cases(&["A", "B"])).is_ok());
    for refused in [
        EnumDeclarationPreimage::new(owner(), name(), false, cases(&[])),
        EnumDeclarationPreimage::new(owner(), name(), false, cases(&["A", "A"])),
        EnumDeclarationPreimage::new(owner(), name(), false, cases(&["1x"])),
        EnumDeclarationPreimage::new(owner(), Vec::new(), false, cases(&["A"])),
    ] {
        assert!(refused.is_err());
    }
    let key = quire_exact::NodeKey::new(
        "test",
        vec![quire_exact::PathSegment::Text("enum".to_owned())],
        0,
    )
    .expect("fixture node key");
    assert!(EnumMemberPreimage::new(key.clone(), "A").is_ok());
    assert!(EnumMemberPreimage::new(key, "not an identifier").is_err());
}

// ----------------------------------------------------------------------
// Dimensions and units (TC-483)
// ----------------------------------------------------------------------

const Q_SOURCES: &str = "dimension Length;\n\
    dimension Time;\n\
    dimension Speed = Length / Time;\n\
    dimension Accel = Speed / Time;\n\
    dimension Temperature;\n\
    unit m : Length = rational(1, 1);\n\
    unit km : Length = rational(2000, 2) * m;\n\
    unit cm : Length = decimal(1, 2) * m;\n\
    unit K : Temperature = rational(1, 1);\n\
    unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);";

/// The key of each dimension and unit `package` declares, by declared name.
fn named_keys(package: &PackageDeclarations, text: &str) -> BTreeMap<String, quire_exact::NodeKey> {
    package
        .nominal_spans
        .iter()
        .map(|(key, span)| (text[span.start..span.end].to_owned(), key.clone()))
        .collect()
}

/// `Q_SOURCES` with its lines reversed: every declaration comes before
/// the ones it depends on.
fn reversed_sources() -> String {
    Q_SOURCES.split('\n').rev().collect::<Vec<_>>().join("\n")
}

#[trace("FR-091-AC-32", "TC-483")]
#[test]
fn dependencies_are_keyed_before_their_dependents_whatever_the_source_order() {
    let (text, assembled) = assemble(&reversed_sources());
    let package = assembled.expect("the reversed unit assembles");
    let graph = &package.units;
    let keys = named_keys(&package, &text);
    assert_eq!(keys.len(), 10);
    for name in ["m", "km", "cm", "K", "C"] {
        graph
            .unit(keys[name].clone())
            .unwrap_or_else(|| panic!("the graph holds unit {name}"));
    }
    let km = graph.unit(keys["km"].clone()).expect("km");
    assert_eq!(km.root(), keys["m"]);
}

#[trace("FR-091-AC-32", "TC-483")]
#[test]
fn dimensions_and_units_are_admitted_under_their_declared_names() {
    let (text, assembled) = assemble(Q_SOURCES);
    let package = assembled.expect("the unit assembles");
    let graph = &package.units;
    let keys = named_keys(&package, &text);
    let names: Vec<&str> = keys.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "Accel",
            "C",
            "K",
            "Length",
            "Speed",
            "Temperature",
            "Time",
            "cm",
            "km",
            "m"
        ]
    );
    let km = graph.unit(keys["km"].clone()).expect("km");
    assert_eq!(km.canonical().scale().to_string(), "1000/1");
    let cm = graph.unit(keys["cm"].clone()).expect("cm");
    assert_eq!(cm.canonical().scale().to_string(), "1/100");
    let celsius = graph.unit(keys["C"].clone()).expect("C");
    assert_eq!(celsius.canonical().offset().to_string(), "5463/20");
    assert!(celsius.is_affine());
    assert!(!km.is_affine());
    let metre = graph.unit(keys["m"].clone()).expect("m");
    assert_eq!(km.root(), metre.key());
    graph.unit(keys["K"].clone()).expect("K");
    // Each dimension is one the units name, and the derived ones normalize
    // to base dimensions.
    let by_key: BTreeMap<quire_exact::NodeKey, &str> = keys
        .iter()
        .map(|(name, key)| (key.clone(), name.as_str()))
        .collect();
    let exponents = |name: &str| -> Vec<(String, String)> {
        let dimension = graph
            .dimension(keys[name].clone())
            .unwrap_or_else(|| panic!("the graph holds dimension {name}"));
        let mut terms: Vec<(String, String)> = dimension
            .exponents()
            .map(|(base, exponent)| (by_key[&base.clone()].to_owned(), exponent.to_string()))
            .collect();
        terms.sort();
        terms
    };
    let term = |name: &str, exponent: &str| (name.to_owned(), exponent.to_owned());
    assert_eq!(exponents("Length"), [term("Length", "1")]);
    assert_eq!(exponents("Time"), [term("Time", "1")]);
    assert_eq!(
        exponents("Speed"),
        [term("Length", "1"), term("Time", "-1")]
    );
    assert_eq!(
        exponents("Accel"),
        [term("Length", "1"), term("Time", "-2")]
    );
    assert_eq!(exponents("Temperature"), [term("Temperature", "1")]);
    // A unit with no dimension or unit form has the empty graph.
    let empty = assemble("type Digit = Int[0, 9];").1.expect("assembles");
    assert_eq!(empty.units.units().count(), 0);
    assert!(empty.nominal_spans.is_empty());
}

#[trace("FR-091-AC-32", "TC-483")]
#[trace("TC-720", "FR-255-AC-1")]
#[test]
fn the_decimal_scale_bound_refuses_with_a_work_budget_limit() {
    let assemble_with = |scale: u64| {
        let text = text(
            PROFILE_V,
            &format!(
                "dimension Length; unit m : Length = rational(1, 1);\n\
                 unit c : Length = decimal(1, {scale}) * m;"
            ),
        );
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        let unit = build_unit(&parsed).expect("S2 builds the unit");
        let assembled = PackageDeclarations::assemble_with_limits(
            parsed.source().reference().clone(),
            "test/package".to_owned(),
            unit,
            crate::model::intake::SelectedModels::default(),
            Vec::new(),
            crate::check::AssemblyLimits {
                decimal_scale: 4,
                ..crate::check::AssemblyLimits::default()
            },
        );
        (text, assembled)
    };
    let (text, assembled) = assemble_with(5);
    let refusal = assembled.expect_err("scale 5 is above the bound 4");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error, not {:?}", refusal.errors)
    };
    let AssemblyCause::DecimalScaleLimit(limit) = &error.cause else {
        panic!("a decimal-scale limit, not {:?}", error.cause)
    };
    assert_eq!(limit.configured_bound(), 4);
    assert_eq!(limit.actual(), 5);
    assert_eq!(limit.setting(), qsl_foundation::Setting::S3DecimalScale);
    assert_eq!(error.span, last(&text, "decimal(1, 5)"));
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "stage_limit_exceeded/work-budget-exceeded"
    );
    assert!(assemble_with(4).1.is_ok());
}

#[trace("FR-091-AC-33", "TC-483")]
#[test]
fn a_unit_name_is_not_a_type() {
    let (_, found) = errors(
        "dimension Length;\n\
         unit m : Length = rational(1, 1);\n\
         function f using v(x: m): Boolean pure { true }",
    );
    assert!(matches!(
        found.as_slice(),
        [AssemblyError {
            cause: AssemblyCause::UnresolvedTypeName { name },
            ..
        }] if name == "m"
    ));
}

#[trace("FR-091-AC-34", "TC-483")]
#[test]
fn name_and_number_errors_are_reported_and_the_topology_does_not_run() {
    let (text, found) = errors(
        "dimension Length;\n\
         dimension Mass;\n\
         dimension Mass;\n\
         dimension Area = Width^2;\n\
         dimension P = Q;\n\
         dimension Q = P;\n\
         unit a : Length = rational(1, 0);\n\
         unit b : Length = rational(2, 1) * c;\n\
         unit c : Length = rational(1, 2) * b;\n\
         unit d : Ghost = rational(1, 1);",
    );
    let mass_span = |start: usize| Span {
        start: start + "dimension ".len(),
        end: start + "dimension Mass".len(),
    };
    let first = text.find("dimension Mass").expect("first Mass");
    let second = text.rfind("dimension Mass").expect("second Mass");
    let mut reported: Vec<(String, AssemblyCause)> = found
        .iter()
        .map(|error| (error.cause.catalog_code().to_string(), error.cause.clone()))
        .collect();
    reported.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        reported,
        vec![
            (
                "ambiguous_declaration/ambiguous-name".to_owned(),
                AssemblyCause::DuplicateQuantityName {
                    name: "Mass".to_owned(),
                    spans: vec![mass_span(first), mass_span(second)],
                }
            ),
            (
                "missing_declaration/missing-name".to_owned(),
                AssemblyCause::UnresolvedQuantityName {
                    name: "Ghost".to_owned()
                }
            ),
            (
                "undefined_expression/unproved-nonzero".to_owned(),
                AssemblyCause::ZeroDenominator
            ),
        ]
    );
    assert!(found
        .iter()
        .any(|error| error.cause == AssemblyCause::ZeroDenominator
            && error.span == last(&text, "rational(1, 0)")));

    // Without those three, the topology runs and reports the first defect
    // alone: `Length`'s turn, before `Area`'s `Width` and the `P`/`Q` cycle.
    let (_, found) = errors(
        "dimension Length;\n\
         dimension Mass;\n\
         dimension Area = Width^2;\n\
         dimension P = Q;\n\
         dimension Q = P;\n\
         unit b : Length = rational(2, 1) * c;\n\
         unit c : Length = rational(1, 2) * b;",
    );
    let [only] = found.as_slice() else {
        panic!("one first defect, not {found:?}")
    };
    assert_eq!(
        only.cause,
        AssemblyCause::UnitGraphTopology {
            fault: super::TopologyFault::TargetCycle,
            retained: vec![
                super::TopologyRetained::Edge {
                    from: "b".to_owned(),
                    to: "c".to_owned(),
                },
                super::TopologyRetained::Edge {
                    from: "c".to_owned(),
                    to: "b".to_owned(),
                },
            ],
        }
    );
    assert_eq!(
        only.cause.catalog_code().to_string(),
        "invalid_package/definition-cycle"
    );
}

#[trace("FR-091-AC-35", "FR-091-AC-71", "TC-483")]
#[test]
fn each_unit_graph_topology_error_refuses_alone() {
    use super::TopologyFault as Fault;
    use super::TopologyRetained as Keep;
    for (source, fault, retained, code) in [
        (
            "dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * r;",
            Fault::ZeroScale,
            vec![
                Keep::Unit {
                    name: "z".into(),
                    key: None,
                },
                Keep::Scale(Rational::from_integer(Integer::zero())),
            ],
            "invalid_package/invalid-value",
        ),
        (
            "dimension L; unit r : L = rational(2, 1);",
            Fault::NonIdentityRoot,
            vec![
                Keep::Unit {
                    name: "r".into(),
                    key: None,
                },
                Keep::Scale(Rational::from_integer(Integer::from(2_i64))),
                Keep::Offset(Rational::from_integer(Integer::zero())),
            ],
            "invalid_package/invalid-value",
        ),
        (
            "dimension L; dimension T; unit r : L = rational(1, 1); \
             unit s : T = rational(1, 1) * r;",
            Fault::CrossDimensionTarget,
            vec![
                Keep::Unit {
                    name: "s".into(),
                    key: None,
                },
                Keep::TargetName("r".into()),
                Keep::Dimension {
                    name: "T".into(),
                    key: None,
                },
                Keep::Dimension {
                    name: "L".into(),
                    key: None,
                },
            ],
            "invalid_package/incompatible-definition",
        ),
        (
            "dimension L; unit r : L = rational(1, 1); unit q : L = rational(1, 1);",
            Fault::TwoRoots,
            vec![
                Keep::Dimension {
                    name: "L".into(),
                    key: None,
                },
                Keep::Unit {
                    name: "r".into(),
                    key: None,
                },
                Keep::Unit {
                    name: "q".into(),
                    key: None,
                },
            ],
            "invalid_package/conflicting-definition",
        ),
    ] {
        let (_, found) = errors(source);
        let [error] = found.as_slice() else {
            panic!("{source}: one error, not {found:?}")
        };
        assert_eq!(
            error.cause,
            AssemblyCause::UnitGraphTopology { fault, retained },
            "{source}"
        );
        assert_eq!(error.cause.catalog_code().to_string(), code, "{source}");
    }
    let (_, found) =
        errors("dimension L; unit r : L = rational(1, 1); unit u : L = rational(1, 1) * ghost;");
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnitGraphTopology {
            fault: Fault::UnknownTarget,
            retained: vec![
                Keep::Unit {
                    name: "u".into(),
                    key: None
                },
                Keep::TargetName("ghost".into()),
            ],
        }
    );
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
}

/// The one error `source` refuses with: its code and cause, and the text at
/// its span.
fn first_defect(source: &str) -> (String, String) {
    let (text, found) = errors(source);
    let [error] = found.as_slice() else {
        panic!("{source}: exactly one first defect, not {found:?}")
    };
    (
        error.cause.catalog_code().to_string(),
        text[error.span.start..error.span.end].to_owned(),
    )
}

/// FR-091-AC-46 and QSpec FR-142-AC-15 (TC-187 U11h to U11k): the first
/// defect is the earliest row of the earliest unit of the earliest node.
#[trace("FR-091-AC-46", "TC-483", "QSpec-TC-187", "QSpec-FR-142-AC-15")]
#[test]
fn the_first_defect_is_the_earliest_row_unit_and_node() {
    let found = first_defect;
    let pair = |code: &str, at: &str| (code.to_owned(), at.to_owned());
    let edge = |from: &str, to: &str| (from.to_owned(), to.to_owned());
    // U11h: row 1 before row 2 within one unit.
    assert_eq!(
        found(
            "dimension L; unit r : L = rational(1, 1); \
             unit z : L = rational(0, 1) * ghost;"
        ),
        pair("invalid_package/invalid-value", "z")
    );
    // U11i: the earlier unit's row 2 before the later unit's row 1.
    assert_eq!(
        found(
            "dimension L; unit r : L = rational(1, 1); \
             unit u : L = rational(1, 1) * ghost; unit z : L = rational(0, 1) * r;"
        ),
        pair("missing_declaration/missing-name", "ghost")
    );
    // U11j: row 4 before row 6.
    assert_eq!(
        found(
            "dimension L; unit r : L = rational(1, 1); unit q : L = rational(1, 1); \
             unit s : L = rational(1, 1) * s;"
        ),
        pair("invalid_package/conflicting-definition", "q")
    );
    // U11k: `L` takes its turn before `T`, so row 5 at `l`, not row 1 at `t`.
    assert_eq!(
        found(
            "dimension L; dimension T; unit t : T = rational(0, 1) * tr; \
             unit tr : T = rational(1, 1); unit l : L = rational(2, 1);"
        ),
        pair("invalid_package/invalid-value", "l")
    );
    // A dimension cycle comes before any unit row of a later dimension.
    assert_eq!(
        found(
            "dimension A = B; dimension B = A; dimension L; \
             unit l : L = rational(2, 1);"
        ),
        pair("invalid_package/definition-cycle", "A")
    );
    let (_, found) =
        errors("dimension A = B; dimension B = A; dimension L; unit l : L = rational(2, 1);");
    assert_eq!(
        found.first().map(|error| &error.cause),
        Some(&AssemblyCause::QuantityCycle {
            edges: vec![edge("A", "B"), edge("B", "A")]
        })
    );
}

/// FR-091-AC-47 and QSpec FR-142-AC-15 (TC-187 U11b, U11f, U11l): the cycle
/// is retained from its least unit by source position.
#[trace(
    "FR-091-AC-47",
    "FR-091-AC-71",
    "TC-483",
    "QSpec-TC-187",
    "QSpec-FR-142-AC-15"
)]
#[test]
fn a_unit_cycle_is_retained_from_its_least_unit_by_source_position() {
    let edge = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    for (source, expected) in [
        (
            "dimension L; unit r : L = rational(1, 1); \
             unit y : L = rational(1, 1) * x; unit x : L = rational(1, 1) * y;",
            vec![edge("y", "x"), edge("x", "y")],
        ),
        (
            "dimension L; unit m : L = rational(1, 1) * cm; \
             unit cm : L = rational(1, 100) * m;",
            vec![edge("m", "cm"), edge("cm", "m")],
        ),
        (
            "dimension L; unit u_self : L = rational(1, 1) * u_self;",
            vec![edge("u_self", "u_self")],
        ),
    ] {
        let (_, found) = errors(source);
        let [error] = found.as_slice() else {
            panic!("{source}: one error, not {found:?}")
        };
        assert_eq!(
            error.cause,
            AssemblyCause::UnitGraphTopology {
                fault: super::TopologyFault::TargetCycle,
                retained: expected
                    .into_iter()
                    .map(|(from, to)| super::TopologyRetained::Edge { from, to })
                    .collect(),
            },
            "{source}"
        );
    }
}

/// QSpec FR-142-AC-15, TC-187 U11j: two targetless units beside a unit that
/// targets itself refuse row 4, `invalid_package`/`conflicting-definition`,
/// not row 6, and retain the dimension and both roots.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-15")]
#[test]
fn u11j_two_roots_refuse_before_a_self_cycle() {
    let (text, found) = errors(
        "dimension L; unit m : L = rational(1, 1); unit m_alt : L = rational(1, 1); \
         unit u_self : L = rational(1, 1) * u_self;",
    );
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "invalid_package/conflicting-definition"
    );
    assert_eq!(
        error.cause,
        AssemblyCause::UnitGraphTopology {
            fault: super::TopologyFault::TwoRoots,
            retained: vec![
                super::TopologyRetained::Dimension {
                    name: "L".into(),
                    key: None
                },
                super::TopologyRetained::Unit {
                    name: "m".into(),
                    key: None
                },
                super::TopologyRetained::Unit {
                    name: "m_alt".into(),
                    key: None
                },
            ],
        }
    );
    assert_eq!(&text[error.span.start..error.span.end], "m_alt");
}

/// QSpec FR-142-AC-16, TC-187 U11m: with `Area` declared before `L`, `Area`'s
/// turn reports the unit cycle and `L` is not examined; with `L` first, `L`'s
/// turn refuses the base dimension no unit anchors.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-16")]
#[test]
fn u11m_the_base_dimension_rule_runs_at_the_base_dimensions_own_turn() {
    let units = "unit a : Area = rational(1, 1) * b; unit b : Area = rational(1, 1) * a;";
    let (_, found) = errors(&format!("dimension Area = L ^ 2; dimension L; {units}"));
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnitGraphTopology {
            fault: super::TopologyFault::TargetCycle,
            retained: vec![
                super::TopologyRetained::Edge {
                    from: "a".to_owned(),
                    to: "b".to_owned(),
                },
                super::TopologyRetained::Edge {
                    from: "b".to_owned(),
                    to: "a".to_owned(),
                },
            ],
        }
    );
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "invalid_package/definition-cycle"
    );

    let (text, found) = errors(&format!("dimension L; dimension Area = L ^ 2; {units}"));
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnanchoredBaseDimension {
            dimension: "L".to_owned(),
            unit: "a".to_owned(),
        }
    );
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
    assert_eq!(&text[error.span.start..error.span.end], "L");
}

/// QSpec FR-142-AC-17, TC-187 U11n, U11o, U11p, U11w: a dimension's own terms
/// are checked at its turn before its units, an unknown term before a
/// cycle. An unknown term retains its owning dimension and the term spelling.
#[trace("FR-091-AC-71", "QSpec-TC-187", "QSpec-FR-142-AC-17")]
#[test]
fn step_minus_one_checks_a_dimensions_own_terms_before_its_units() {
    let edge = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    // U11n: a cycle of two, from `A`, declared first; `a` is not examined.
    let (text, found) =
        errors("dimension A = B; dimension B = A ^ 2; unit a : A = rational(1, 1);");
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::QuantityCycle {
            edges: vec![edge("A", "B"), edge("B", "A")]
        }
    );
    assert_eq!(&text[error.span.start..error.span.end], "A");
    // U11o: a self-term; `L` has a unit and is admitted first.
    let (_, found) = errors("dimension L; dimension D = D * L; unit m : L = rational(1, 1);");
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::QuantityCycle {
            edges: vec![edge("D", "D")]
        }
    );
    // U11p: `L` is admitted at its turn, then `D`'s unknown term `K`.
    let (text, found) = errors("dimension L; dimension D = L * K; unit m : L = rational(1, 1);");
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnresolvedDimensionTerm {
            dimension: "D".to_owned(),
            term: "K".to_owned(),
        }
    );
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
    assert_eq!(&text[error.span.start..error.span.end], "K");
    // U11w: the unknown term before the cycle, and nothing else.
    let (text, found) = errors("dimension A = B * K; dimension B = A;");
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnresolvedDimensionTerm {
            dimension: "A".to_owned(),
            term: "K".to_owned(),
        }
    );
    assert_eq!(&text[error.span.start..error.span.end], "K");
}

/// QSpec FR-142-AC-17, TC-187 U11x and U11z3: a dimension that only reaches
/// an unknown term or a cycle through another dimension is not refused for
/// it, has no normalized map, and its unit rows run at its own turn.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-17")]
#[test]
fn a_dimension_reaching_a_defect_runs_its_own_unit_rows_first() {
    let base = "dimension L; unit m : L = rational(1, 1); \
                dimension E = D; dimension D = L * K;";
    // U11x1: `E` does not look through `D`; `D`'s turn reports `K`.
    let (text, found) = errors(base);
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnresolvedDimensionTerm {
            dimension: "D".to_owned(),
            term: "K".to_owned(),
        }
    );
    assert_eq!(&text[error.span.start..error.span.end], "K");
    // U11x2: `E`'s turn comes first and its unit row 1 runs.
    assert_eq!(
        first_defect(&format!("{base} unit e : E = rational(0, 1);")),
        ("invalid_package/invalid-value".to_owned(), "e".to_owned())
    );
    // U11z3a and U11z3b.
    let cycle = "dimension E = A; dimension A = B; dimension B = A;";
    assert_eq!(
        first_defect(
            "dimension E = A; unit e : E = rational(0, 1); \
             dimension A = B; dimension B = A;"
        ),
        ("invalid_package/invalid-value".to_owned(), "e".to_owned())
    );
    let (text, found) = errors(cycle);
    let edge = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::QuantityCycle {
            edges: vec![edge("A", "B"), edge("B", "A")]
        }
    );
    assert_eq!(&text[error.span.start..error.span.end], "A");
}

/// QSpec FR-142-AC-16: a dimension with no normalized map, here `E` reaching
/// `D`'s unknown term, carries no base dimension, so `L`'s turn admits it
/// and `D`'s turn reports `K`; with `E` mapped, `L` is refused first.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-16")]
#[test]
fn a_dimension_without_a_map_anchors_no_base_dimension_obligation() {
    let (text, found) = errors(
        "dimension L; dimension D = L * K; dimension E = D; \
         unit e : E = rational(1, 1);",
    );
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnresolvedDimensionTerm {
            dimension: "D".to_owned(),
            term: "K".to_owned(),
        }
    );
    assert_eq!(&text[error.span.start..error.span.end], "K");
    let (_, found) =
        errors("dimension L; dimension D = L; dimension E = D; unit e : E = rational(1, 1);");
    assert!(
        matches!(
            found.as_slice(),
            [AssemblyError {
                cause: AssemblyCause::UnanchoredBaseDimension { dimension, unit },
                ..
            }] if dimension == "L" && unit == "e"
        ),
        "{found:?}"
    );
}

/// QSpec FR-142-AC-17, TC-187 U11q, FR-091-AC-46 and FR-480-AC-6: aggregate
/// source order is the caller's document list, then the byte offset, so a
/// document whose declarations sit at larger offsets still takes its turn
/// first when it appears first in the list.
#[trace("FR-091-AC-46", "FR-480-AC-6", "QSpec-TC-187", "QSpec-FR-142-AC-17")]
#[test]
fn u11q_the_first_document_takes_the_first_turn_whatever_its_offsets() {
    let padding = " ".repeat(400);
    let time = text(
        "",
        &format!("{padding}dimension T; unit z : T = rational(0, 1);"),
    );
    let length = text(
        "",
        "dimension L; unit u_self : L = rational(1, 1) * u_self;",
    );
    let run = |first: (&str, &str), second: (&str, &str)| {
        let (source_a, unit_a) = parsed_unit_as("repo", first.0, first.1);
        let (source_b, unit_b) = parsed_unit_as("repo", second.0, second.1);
        let refusal = PackageDeclarations::assemble_units(
            "test/package".to_owned(),
            vec![(source_a, unit_a), (source_b, unit_b)],
            crate::model::intake::SelectedModels::default(),
            Vec::new(),
            super::AssemblyLimits::default(),
            &Cancel::new(),
        )
        .expect_err("a defect in either document");
        let [error] = refusal.errors.as_slice() else {
            panic!("one first defect, not {:?}", refusal.errors)
        };
        (error.cause.catalog_code().to_string(), error.cause.clone())
    };
    assert_eq!(
        run(("1", &time), ("2", &length)).0,
        "invalid_package/invalid-value".to_owned()
    );
    assert_eq!(
        run(("2", &length), ("1", &time)),
        (
            "invalid_package/definition-cycle".to_owned(),
            AssemblyCause::UnitGraphTopology {
                fault: super::TopologyFault::TargetCycle,
                retained: vec![super::TopologyRetained::Edge {
                    from: "u_self".to_owned(),
                    to: "u_self".to_owned(),
                }],
            }
        )
    );
}

/// FR-091-AC-45 (TC-483): the base-dimension rule refuses `L`, unitless
/// and carried by `m2`'s dimension, at `L`, naming `L` and `m2`; a unit of
/// `L` anchors it; a unitless base dimension no unit carries, a derived
/// dimension without a unit, and the dimensionless `N = L / L` with its
/// own unit all assemble.
#[trace("FR-091-AC-45", "TC-483")]
#[test]
fn the_base_dimension_rule_refuses_an_unanchored_base_dimension() {
    let area = "dimension L; dimension A = L ^ 2; unit m2 : A = rational(1, 1);";
    let (text, found) = errors(area);
    let [error] = found.as_slice() else {
        panic!("one error, not {found:?}")
    };
    assert_eq!(
        (
            &error.cause,
            error.cause.catalog_code().to_string(),
            &text[error.span.start..error.span.end]
        ),
        (
            &AssemblyCause::UnanchoredBaseDimension {
                dimension: "L".to_owned(),
                unit: "m2".to_owned(),
            },
            "missing_declaration/missing-name".to_owned(),
            "L"
        )
    );
    for source in [
        "dimension L; unit m : L = rational(1, 1); dimension A = L ^ 2; \
         unit m2 : A = rational(1, 1);",
        "dimension L; dimension A = L ^ 2;",
        "dimension L; dimension N = L / L; unit one : N = rational(1, 1);",
    ] {
        let (_, assembled) = assemble(source);
        assert!(assembled.is_ok(), "{source}: {:?}", assembled.err());
    }
}

const TIME_UNITS: &str = "dimension Time;\n\
     unit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\n";

/// FR-091-AC-44 (TC-483 step 5): `Quantity<ms>` resolves to the type of the
/// unit node `ms` and introduces no declaration; a dimension or an unknown
/// name refuses `missing_declaration`/`missing-name` at the name.
#[trace("FR-091-AC-44", "TC-483", "QSpec-TC-187", "QSpec-FR-142-AC-11")]
#[test]
fn a_quantity_type_form_resolves_to_its_unit_and_refuses_a_non_unit_name() {
    let accepted =
        format!("{TIME_UNITS}function f using v(d: Quantity<ms>): Boolean pure {{ true }}");
    let package = assemble(&accepted).1.expect("Quantity<ms> assembles");
    let (parameters, _) = package
        .resolved_signatures
        .get(0)
        .expect("f's resolved signature");
    let ms = match parameters.as_slice() {
        [(name, ValueType::Quantity(unit))] if name == "d" => unit.clone(),
        other => panic!("d is a quantity: {other:?}"),
    };
    let declared = package
        .units
        .units()
        .find(|unit| unit.id() == ms)
        .expect("the type names a unit of the admitted graph");
    assert_ne!(
        declared.root(),
        declared.key(),
        "ms is the derived unit, not the root s"
    );
    package
        .check(CheckingLimits::default())
        .expect("the package checks");

    for name in ["Time", "nothing"] {
        let source =
            format!("{TIME_UNITS}function f using v(d: Quantity<{name}>): Boolean pure {{ true }}");
        let (text, found) = errors(&source);
        let [error] = found.as_slice() else {
            panic!("{name}: one error, not {found:?}")
        };
        assert!(
            matches!(&error.cause, AssemblyCause::UnresolvedUnitName { name: found, packages } if found == name && packages.is_empty()),
            "{name}: {error:?}"
        );
        assert_eq!(
            error.cause.catalog_code().to_string(),
            "missing_declaration/missing-name"
        );
        assert_eq!(error.span, last(&text, name));
    }
}

/// FR-091-AC-44: `f(3 s)` where `f` takes `Quantity<ms>` refuses
/// `ill_typed`/`type-mismatch`.
#[trace(
    "FR-091-AC-44",
    "FR-093-AC-34",
    "TC-483",
    "QSpec-TC-187",
    "QSpec-FR-142-AC-11"
)]
#[test]
fn a_quantity_argument_of_another_unit_is_a_type_mismatch() {
    let source = format!(
        "{TIME_UNITS}function f using v(d: Quantity<ms>): Boolean pure {{ true }}\n\
         function g using v(): Boolean pure {{ f(3 s) }}"
    );
    let (text, assembled) = assemble(&source);
    let package = assembled.expect("the unit assembles");
    let regions = assemble(&source).1.expect("the unit assembles");
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a Quantity<s> argument is not a Quantity<ms>");
    let argument = last(&text, "3 s");
    let located = |refusal: &crate::check::CheckRefusal| {
        regions.region(&refusal.location).is_some_and(|region| {
            (region.start(), region.end()) == (argument.start as u64, argument.end as u64)
        })
    };
    assert!(
        refusals.iter().any(|refusal| located(refusal)
            && refusal
                .cause
                .catalog_code()
                .is_some_and(|code| code.to_string() == "ill_typed/type-mismatch")),
        "the mismatch is located at the argument: {refusals:?}"
    );
}

/// QSpec FR-142-AC-12: `f(d: Quantity<ms>): Quantity<ms>` returning `d`
/// checks; returning `convert<Quantity<s>>(d)` refuses `ill_typed`/
/// `type-mismatch` at the returned expression.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-12")]
#[test]
fn a_quantity_result_checks_and_a_converted_result_of_another_unit_is_a_mismatch() {
    let returning = |body: &str| {
        format!("{TIME_UNITS}function f using v(d: Quantity<ms>): Quantity<ms> pure {{ {body} }}")
    };
    assemble(&returning("d"))
        .1
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("returning d checks");

    let source = returning("convert<Quantity<s>>(d)");
    let (text, assembled) = assemble(&source);
    let package = assembled.expect("the unit assembles");
    let regions = assemble(&source).1.expect("the unit assembles");
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a Quantity<s> result is not a Quantity<ms>");
    let returned = last(&text, "convert<Quantity<s>>(d)");
    let [refusal] = refusals.as_slice() else {
        panic!("exactly the one result mismatch, not {refusals:?}")
    };
    let region = regions
        .region(&refusal.location)
        .expect("the refusal names a source region");
    assert_eq!(
        (
            region.start(),
            region.end(),
            refusal.cause.catalog_code().map(|code| code.to_string())
        ),
        (
            returned.start as u64,
            returned.end as u64,
            Some("ill_typed/type-mismatch".to_owned())
        ),
        "the mismatch is located at the returned expression: {refusals:?}"
    );
}

/// The catalog code of each refusal `check` gives `declarations` (after the
/// shared units and `f`) under `limits`, and whether it is located at the
/// source text `at`.
fn quantity_refusals(declarations: &str, limits: CheckingLimits, at: &str) -> Vec<(String, bool)> {
    let source = format!("{TIME_UNITS}{QUANTITY_F}{declarations}");
    let (text, assembled) = assemble(&source);
    let package = assembled.expect("the unit assembles");
    let regions = package.regions();
    let span = last(&text, at);
    package
        .check(limits)
        .expect_err("the quantity literal refuses")
        .iter()
        .map(|refusal| {
            let located = regions.refusal_region(refusal).is_some_and(|region| {
                (region.start(), region.end()) == (span.start as u64, span.end as u64)
            });
            (
                refusal
                    .cause
                    .catalog_code()
                    .map_or_else(|| format!("{:?}", refusal.cause), |code| code.to_string()),
                located,
            )
        })
        .collect()
}

const QUANTITY_F: &str = "function f using v(d: Quantity<ms>): Boolean pure { true }\n";

/// FR-093-AC-33: every admitted quantity literal checks (the lowering and
/// the strict reader are exercised in `qsl-package`).
#[trace("FR-093-AC-33", "TC-483")]
#[test]
fn admitted_quantity_literals_check() {
    for body in [
        "function h using v(): Boolean pure { f(3 ms) }",
        "function h2 using v(): Boolean pure { f(0.5 ms) }",
        "function h3 using v(): Boolean pure { f(rational(2, 4) ms) }",
        "function n using v(): Boolean pure { f(rational(-3, 1) ms) }",
        "function k using v(d: Quantity<s>): Boolean pure { d = rational(1, 1000) s }",
        "function c using v(d: Quantity<ms>): Boolean pure { convert<Quantity<s>>(d) = 3 s }",
    ] {
        let source = format!("{TIME_UNITS}{QUANTITY_F}{body}");
        assemble(&source)
            .1
            .expect("the unit assembles")
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"));
    }
}

/// FR-093-AC-34: each refusal, with its code and location. A refused
/// literal yields no graph, so no literal node.
#[trace("FR-093-AC-34", "TC-483")]
#[test]
fn quantity_literal_refusals_carry_their_codes_and_locations() {
    let ceiling = CheckingLimits::default().with_rational_bits(16);
    let default = CheckingLimits::default();
    let cases: [(&str, CheckingLimits, &str, &str); 9] = [
        (
            "function e using v(d: Quantity<ms>): Boolean pure { d < 3 s }",
            default,
            "d < 3 s",
            "ill_typed/type-mismatch",
        ),
        (
            "function g2 using v(x: Integer): Boolean pure { x = 3 ms }",
            default,
            "x = 3 ms",
            "ill_typed/type-mismatch",
        ),
        (
            "function m using v(b: Boolean): Boolean pure { -b }",
            default,
            "b",
            "ill_typed/type-mismatch",
        ),
        (
            "function g using v(): Boolean pure { f(3 ghost) }",
            default,
            "3 ghost",
            "missing_declaration/missing-name",
        ),
        (
            "function g using v(): Boolean pure { f(3 Time) }",
            default,
            "3 Time",
            "missing_declaration/missing-name",
        ),
        (
            "function g using v(): Boolean pure { f(rational(1, 0) ms) }",
            default,
            "rational(1, 0) ms",
            "undefined_expression/unproved-nonzero",
        ),
        (
            "function q using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure { d / 0 ms = e / 0 ms }",
            default,
            "d / 0 ms",
            "undefined_expression/unproved-nonzero",
        ),
        (
            "function g using v(): Boolean pure { f(rational(1, 100000) ms) }",
            ceiling,
            "rational(1, 100000) ms",
            "stage_limit_exceeded/rational-bits-exceeded",
        ),
        (
            "function g using v(): Boolean pure { f(rational(1, 0) ghost) }",
            default,
            "rational(1, 0) ghost",
            "undefined_expression/unproved-nonzero",
        ),
    ];
    for (declarations, limits, at, code) in cases {
        let refusals = quantity_refusals(declarations, limits, at);
        assert!(
            refusals
                .iter()
                .any(|(found, located)| found == code && *located),
            "{declarations}: expected {code} at {at:?}: {refusals:?}"
        );
        assert_eq!(refusals.len(), 1, "{declarations}: {refusals:?}");
    }
}

/// QSpec FR-146 "Accepted proof forms" (TC-191, the literal-divisor clauses
/// of FR-146-AC-9): a quantity division by a quantity literal of nonzero
/// magnitude, written as an integer, a decimal or a `rational(n,d)` (a
/// negative one included), is admitted with no range obligation; a
/// zero-magnitude literal and every non-literal divisor (a parameter, a
/// `let` name bound to a literal, a product) are `unproved-nonzero` at the
/// division whatever guard encloses them.
#[trace("QSpec-TC-191")]
#[test]
fn only_a_nonzero_quantity_literal_divisor_proves_a_quantity_division_defined() {
    let check = |body: &str| {
        let source = format!("{TIME_UNITS}{QUANTITY_F}{body}");
        assemble(&source)
            .1
            .expect("the unit assembles")
            .check(CheckingLimits::default())
    };
    for divisor in [
        "3 ms",
        "0.5 s",
        "1.50 s",
        "rational(3, 2) s",
        "rational(-3, 2) s",
    ] {
        let body = format!(
            "function q using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure \
             {{ d / {divisor} = e / {divisor} }}"
        );
        check(&body).unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"));
    }
    for (body, at) in [
        (
            "function q using v(d: Quantity<ms>): Boolean pure { d / 0 s = d / 3 s }",
            "d / 0 s",
        ),
        (
            "function q using v(d: Quantity<ms>): Boolean pure \
             { d / rational(0, 1) s = d / 3 s }",
            "d / rational(0, 1) s",
        ),
        (
            "function q using v(d: Quantity<ms>): Boolean pure { d / 0.0 s = d / 3 s }",
            "d / 0.0 s",
        ),
        (
            "function q using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure \
             { e != 0 ms and d / e = d / 3 ms }",
            "d / e",
        ),
        (
            "function q using v(d: Quantity<ms>): Boolean pure \
             { let x = 3 ms in d / x = d / 3 ms }",
            "d / x",
        ),
    ] {
        let refusals = quantity_refusals(body, CheckingLimits::default(), at);
        assert_eq!(
            refusals,
            [("undefined_expression/unproved-nonzero".to_owned(), true)],
            "{body}"
        );
    }
}

/// FR-093-AC-34: the refusals name the unit name and the limit.
#[trace("FR-093-AC-34", "TC-483")]
#[test]
fn quantity_literal_refusals_name_the_name_and_the_limit() {
    let refuse = |declarations: &str, limits| {
        let source = format!("{TIME_UNITS}{QUANTITY_F}{declarations}");
        let mut refusals = assemble(&source)
            .1
            .expect("the unit assembles")
            .check(limits)
            .expect_err("the quantity literal refuses");
        assert_eq!(refusals.len(), 1, "{declarations}: {refusals:?}");
        refusals.remove(0).cause
    };
    for name in ["ghost", "Time"] {
        let cause = refuse(
            &format!("function g using v(): Boolean pure {{ f(3 {name}) }}"),
            CheckingLimits::default(),
        );
        assert!(
            matches!(&cause, CheckCause::UnresolvedUnitName { name: found, packages } if found == name && packages.is_empty()),
            "{name}: {cause:?}"
        );
    }
    let cause = refuse(
        "function g using v(): Boolean pure { f(rational(1, 100000) ms) }",
        CheckingLimits::default().with_rational_bits(16),
    );
    let CheckCause::ResourceExhausted(limit) = &cause else {
        panic!("a limit refusal: {cause:?}");
    };
    assert_eq!(
        (limit.kind.setting().name(), limit.limit),
        ("s3.rational_bits", 16)
    );
}

/// FR-093-AC-34 order: a literal that violates several conditions gets the
/// first in order (ceiling, zero denominator, unit name, position type).
#[trace("FR-093-AC-34", "TC-483")]
#[test]
fn a_quantity_literal_refuses_with_the_first_violated_condition_in_order() {
    let ceiling = CheckingLimits::default().with_rational_bits(16);
    let default = CheckingLimits::default();
    let only = |declarations: &str, limits, at: &str| -> String {
        let refusals = quantity_refusals(declarations, limits, at);
        assert_eq!(refusals.len(), 1, "{declarations}: {refusals:?}");
        assert!(refusals[0].1, "{declarations}: located at {at:?}");
        refusals[0].0.clone()
    };
    // Over the ceiling, zero denominator, unknown unit and wrong position.
    assert_eq!(
        only(
            "function g using v(x: Integer): Boolean pure { x = rational(100000, 0) ghost }",
            ceiling,
            "rational(100000, 0) ghost"
        ),
        "stage_limit_exceeded/rational-bits-exceeded"
    );
    // Zero denominator, unknown unit and wrong position.
    assert_eq!(
        only(
            "function g using v(x: Integer): Boolean pure { x = rational(1, 0) ghost }",
            default,
            "rational(1, 0) ghost"
        ),
        "undefined_expression/unproved-nonzero"
    );
    // Unknown unit and wrong position.
    assert_eq!(
        only(
            "function g using v(x: Integer): Boolean pure { x = 3 ghost }",
            default,
            "3 ghost"
        ),
        "missing_declaration/missing-name"
    );
}

/// FR-091-AC-44: the unit namespace is its own: a unit and a record both
/// named `ms` check, and two units named `ms` refuse at both declarations.
/// The strict checked-package reader refuses the emitted package of a unit
/// and a record that share a name (`AmbiguousDeclaration`); that is
/// Remaining work: QSL-950, so this test asserts `check` only.
#[trace("FR-091-AC-44", "TC-483", "QSpec-TC-187", "QSpec-FR-142-AC-13")]
#[test]
fn a_unit_and_a_record_share_a_name_and_two_units_do_not() {
    let both = format!(
        "{TIME_UNITS}record ms {{ x: Boolean; }}\n\
         function f using v(a: Quantity<ms>, r: ms): Boolean pure {{ r.x }}"
    );
    let package = assemble(&both)
        .1
        .expect("a unit and a record named ms assemble");
    package
        .check(CheckingLimits::default())
        .expect("Quantity<ms> means the unit and ms the record");

    let (text, found) = errors(
        "dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1);",
    );
    let at = |start: usize| Span {
        start,
        end: start + "ms".len(),
    };
    let first = text.find("unit ms").expect("first") + "unit ".len();
    let second = text.rfind("unit ms").expect("second") + "unit ".len();
    assert!(found.iter().any(|error| matches!(
        &error.cause,
        AssemblyCause::DuplicateQuantityName { name, spans }
            if name == "ms" && *spans == vec![at(first), at(second)]
    ) && error.cause.catalog_code().to_string()
        == "ambiguous_declaration/ambiguous-name"));
}

/// FR-091-AC-44 (QSpec FR-142-AC-13): a declaration named `Quantity` is
/// refused at S1.
#[trace("FR-091-AC-44", "TC-483", "QSpec-TC-187", "QSpec-FR-142-AC-13")]
#[test]
fn a_declaration_named_quantity_is_invalid_syntax() {
    let source = text(PROFILE_V, "record Quantity { x: Boolean; }");
    let admissible = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .is_ok_and(|parsed| parsed.is_admissible());
    assert!(!admissible, "`Quantity` is reserved");
}

#[trace("FR-091-AC-21", "TC-406")]
#[test]
fn the_dimension_and_unit_causes_have_their_catalog_codes() {
    let span = Span { start: 0, end: 0 };
    let cases = [
        (
            AssemblyCause::DuplicateQuantityName {
                name: "m".into(),
                spans: vec![span],
            },
            "ambiguous_declaration/ambiguous-name",
        ),
        (
            AssemblyCause::UnresolvedQuantityName { name: "m".into() },
            "missing_declaration/missing-name",
        ),
        (
            AssemblyCause::QuantityCycle { edges: Vec::new() },
            "invalid_package/definition-cycle",
        ),
        (
            AssemblyCause::ZeroDenominator,
            "undefined_expression/unproved-nonzero",
        ),
        (
            AssemblyCause::UnitGraphTopology {
                fault: TopologyFault::TwoRoots,
                retained: Vec::new(),
            },
            "invalid_package/conflicting-definition",
        ),
        (
            AssemblyCause::DecimalScaleLimit(qsl_foundation::diagnostic::LimitExceeded::new(
                qsl_foundation::Setting::S3DecimalScale,
                4,
                5,
            )),
            "stage_limit_exceeded/work-budget-exceeded",
        ),
    ];
    for (cause, code) in cases {
        assert_eq!(cause.catalog_code().to_string(), code, "{cause:?}");
    }
}

fn composite_node(label: &str) -> quire_exact::NodeKey {
    quire_exact::NodeKey::new(
        "test",
        vec![quire_exact::PathSegment::Text(label.to_owned())],
        0,
    )
    .expect("fixture node key")
}

fn empty_record(
    label: &str,
    supertypes: &[&str],
) -> quire_semantic_value::declaration::CompositeDeclaration {
    quire_semantic_value::declaration::CompositeDeclaration::new(
        composite_node(label),
        label.to_owned(),
        quire_semantic_value::declaration::CompositeShape::Record(Vec::new()),
    )
    .with_supertypes(
        supertypes
            .iter()
            .map(|general| composite_node(general))
            .collect(),
    )
}

fn admit_composites(
    declarations: Vec<quire_semantic_value::declaration::CompositeDeclaration>,
) -> Result<quire_semantic_value::declaration::TypeEnvironment, AssemblyRefusal> {
    super::admit_types(
        declarations,
        &[],
        &BTreeMap::new(),
        TypeEnvironmentLimits::default(),
        &Cancel::new(),
    )
    .map(|admitted| admitted.types)
}

/// QSL-732: a composite generalization cycle refuses with the typed cause,
/// `ill_typed`, and the `type-mismatch` catalog cause beside the object-type
/// cycle; it is not reported as a broken invariant.
#[trace("FR-143-AC-9", "TC-188")]
#[test]
fn a_composite_supertype_cycle_refuses_as_ill_typed_type_mismatch() {
    let refusal = admit_composites(vec![empty_record("A", &["B"]), empty_record("B", &["A"])])
        .expect_err("a composite cycle refuses");
    assert!(!refusal.errors.is_empty());
    for error in &refusal.errors {
        let AssemblyCause::InvalidTypeDeclaration(invalid) = &error.cause else {
            panic!("expected the typed declaration refusal: {error:?}");
        };
        assert!(
            matches!(
                invalid.cause,
                quire_semantic_value::declaration::DeclarationCause::CompositeGeneralizationCycle { .. }
            ),
            "{invalid:?}"
        );
        assert_eq!(
            error.cause.code(),
            qsl_foundation::diagnostic::Code::IllTyped
        );
        assert_eq!(
            error.cause.catalog_code().to_string(),
            "ill_typed/type-mismatch"
        );
    }
}

/// QSL-732: a composite supertype naming no declaration that any round set
/// aside is its own refusal (`UnknownDeclaration`, a broken invariant), not a
/// silently dropped child.
#[trace("FR-143-AC-9", "TC-188")]
#[test]
fn a_dangling_composite_supertype_is_refused_not_dropped() {
    let refusal = admit_composites(vec![empty_record("Child", &["Missing"])])
        .expect_err("a dangling composite supertype refuses");
    let [error] = refusal.errors.as_slice() else {
        panic!("one refusal expected: {:?}", refusal.errors);
    };
    let AssemblyCause::InvalidTypeDeclaration(invalid) = &error.cause else {
        panic!("expected the typed declaration refusal: {error:?}");
    };
    assert_eq!(invalid.declaration, "Child");
    assert_eq!(
        invalid.cause,
        quire_semantic_value::declaration::DeclarationCause::UnknownDeclaration(composite_node(
            "Missing"
        ))
    );
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "runtime_invariant/established-invariant-broken"
    );
}

/// QSL-732: a child whose composite parent is itself refused is a
/// consequence of the parent's refusal and is set aside unreported.
#[trace("FR-143-AC-9", "TC-188")]
#[test]
fn a_child_of_a_refused_composite_parent_is_a_consequence() {
    use quire_semantic_value::declaration::{
        CompositeDeclaration, CompositeShape, FieldDeclaration,
    };
    let duplicate = || FieldDeclaration::new("x", ValueType::Integer, Presence::Required);
    let parent = CompositeDeclaration::new(
        composite_node("Parent"),
        "Parent".to_owned(),
        CompositeShape::Record(vec![duplicate(), duplicate()]),
    );
    let refusal = admit_composites(vec![parent, empty_record("Child", &["Parent"])])
        .expect_err("the parent refuses");
    let [error] = refusal.errors.as_slice() else {
        panic!("only the parent is reported: {:?}", refusal.errors);
    };
    let AssemblyCause::InvalidTypeDeclaration(invalid) = &error.cause else {
        panic!("expected the typed declaration refusal: {error:?}");
    };
    assert_eq!(invalid.declaration, "Parent");
}

/// Relaxing a widened redefinition retypes only the field that redefines the
/// widened member (or a member it redefines), never a sibling field of the
/// same type that redefines something else.
#[test]
fn relaxing_a_widening_leaves_a_sibling_redefinition_alone() {
    use quire_semantic_value::declaration::{FieldDeclaration, FieldRef, ObjectTypeDeclaration};
    let id = |byte: u8| {
        EffectiveId::new(
            None,
            quire_exact::OriginalDeclarationKey::new("test", byte.to_string()),
        )
    };
    let (a, b) = (id(1), id(2));
    let int = |upper: i64| {
        ValueType::Int(
            IntegerInterval::new(
                quire_exact::Integer::from(0i64),
                quire_exact::Integer::from(upper),
            )
            .expect("interval"),
        )
    };
    let parent = ObjectTypeDeclaration::new(
        a.clone(),
        "M::A",
        vec![
            FieldDeclaration::new("x", int(5), Presence::Required),
            FieldDeclaration::new("y", int(9), Presence::Required),
        ],
    );
    let child = ObjectTypeDeclaration::new(
        b.clone(),
        "M::B",
        vec![
            FieldDeclaration::new("x", int(7), Presence::Required)
                .with_redefines(FieldRef::new(a.clone(), "x")),
            FieldDeclaration::new("y", int(3), Presence::Required)
                .with_redefines(FieldRef::new(a.clone(), "y")),
        ],
    )
    .with_supertypes(vec![a.clone()]);
    let mut types = vec![parent, child];
    assert!(super::relax_widening(
        &mut types,
        "M::B",
        &FieldRef::new(a, "x")
    ));
    let fields = types[1].attributes();
    assert_eq!(
        fields[0].value_type(),
        &int(5),
        "the widened field is relaxed"
    );
    assert_eq!(
        fields[1].value_type(),
        &int(3),
        "the sibling keeps its own type"
    );
}

/// The composite declaration the unit declares under `name`.
fn composite_named<'p>(
    package: &'p PackageDeclarations,
    name: &str,
) -> &'p quire_semantic_value::declaration::CompositeDeclaration {
    package
        .types
        .composites()
        .find(|declaration| declaration.name() == name)
        .unwrap_or_else(|| panic!("the unit declares {name}"))
}

/// A union's members as (name, payload types), in declared order.
fn members_of(
    declaration: &quire_semantic_value::declaration::CompositeDeclaration,
) -> Vec<(String, Vec<ValueType>)> {
    let CompositeShape::Union(members) = declaration.shape() else {
        panic!("{} is a union", declaration.name())
    };
    members
        .iter()
        .map(|member| (member.identifier().to_owned(), member.positions().to_vec()))
        .collect()
}

#[trace("FR-316-AC-1", "TC-817")]
#[test]
fn a_union_is_its_own_composite_in_declared_order_beside_an_enum() {
    let (_, assembled) = assemble(
        "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
         enum Kind { Empty, Circle }",
    );
    let assembled = assembled.expect("the unit assembles");
    let shape = composite_named(&assembled, "Shape");
    assert_eq!(
        members_of(shape),
        [
            ("Empty".to_owned(), vec![]),
            ("Circle".to_owned(), vec![ValueType::Integer]),
            (
                "Rect".to_owned(),
                vec![ValueType::Integer, ValueType::Integer]
            ),
        ]
    );
    let [kind] = assembled.enums.as_slice() else {
        panic!("one enum")
    };
    assert_ne!(
        ValueType::Composite(shape.key()),
        ValueType::Enum(kind.shape()),
        "a union and an enum with the same member names are different types"
    );
}

/// The refusals of `declarations`, each with its recursion cycle when the
/// cause is the recursion rule.
fn recursion_refusals(
    declarations: &str,
) -> (String, Vec<(AssemblyError, RecursionEdges, Vec<String>)>) {
    let (text, found) = errors(declarations);
    let refusals = found
        .into_iter()
        .map(|error| {
            let AssemblyCause::InvalidTypeDeclaration(invalid) = &error.cause else {
                panic!("a type declaration refusal, not {:?}", error.cause)
            };
            let DeclarationCause::Recursion { edges, cycle } = &invalid.cause else {
                panic!("the recursion rule, not {:?}", invalid.cause)
            };
            let (edges, cycle) = (*edges, cycle.clone());
            (error, edges, cycle)
        })
        .collect();
    (text, refusals)
}

#[trace(
    "FR-316-AC-2",
    "TC-817",
    "TC-818",
    "QSpec-TC-263",
    "QSpec-FR-143-AC-13"
)]
#[test]
fn a_union_recurses_only_through_an_escaping_edge() {
    let (_, tree) = assemble(
        "union Tree { Leaf, Node(Integer, Option<Tree>, Option<Tree>) }\n\
         union L { Nil, Cons(Integer, Option<L>) }",
    );
    tree.expect("an Option payload escapes the recursion rule");

    let (text, refusals) = recursion_refusals("union L { Nil, Cons(Integer, L) }");
    let [(error, edges, cycle)] = refusals.as_slice() else {
        panic!("one refusal, not {refusals:?}")
    };
    assert_eq!(*edges, RecursionEdges::NonEscaping);
    assert_eq!(cycle, &["L", "L"]);
    assert_eq!(error.span, after(&text, "union L", "L"));
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "ill_typed/type-mismatch"
    );
}

#[trace("FR-316-AC-2", "TC-818", "QSpec-TC-263", "QSpec-FR-143-AC-13")]
#[test]
fn a_cycle_through_a_record_and_a_union_is_refused_at_both_declarations() {
    let (text, refusals) = recursion_refusals("record Box { inner: U; }\nunion U { Wrap(Box) }");
    assert_eq!(refusals.len(), 2, "{refusals:?}");
    let mut spans: Vec<Span> = refusals.iter().map(|(error, ..)| error.span).collect();
    spans.sort_by_key(|span| span.start);
    assert_eq!(
        spans,
        [
            after(&text, "record Box", "Box"),
            after(&text, "union U", "U")
        ]
    );
    for (_, edges, cycle) in &refusals {
        assert_eq!(*edges, RecursionEdges::NonEscaping);
        let mut named: Vec<&str> = cycle.iter().map(String::as_str).collect();
        named.sort_unstable();
        named.dedup();
        assert_eq!(named, ["Box", "U"], "the cycle runs through both");
        assert_eq!(cycle.first(), cycle.last());
    }
}

#[trace("FR-316-AC-3", "TC-818")]
#[test]
fn a_union_refuses_a_repeated_member_and_an_unresolved_payload_type() {
    let (text, found) = errors("union U { A, A(Integer) }");
    let [error] = found.as_slice() else {
        panic!("one refusal, not {found:?}")
    };
    let AssemblyCause::InvalidTypeDeclaration(invalid) = &error.cause else {
        panic!("a type declaration refusal, not {:?}", error.cause)
    };
    assert_eq!(invalid.declaration, "U");
    assert_eq!(invalid.cause, DeclarationCause::DuplicateMember("A".into()));
    assert_eq!(error.span, after(&text, "union U", "U"));
    assert_eq!(error.cause.code(), Code::AmbiguousDeclaration);
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "ambiguous_declaration/ambiguous-name"
    );

    let (text, found) = errors("union U { A(Missing) }");
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::UnresolvedTypeName {
                name: "Missing".into()
            },
            span: last(&text, "Missing"),
        }]
    );
}

#[trace("FR-316-AC-4", "TC-818")]
#[test]
fn a_union_is_bounded_only_by_the_configured_ceilings() {
    let members: Vec<String> = (0..5_000).map(|member| format!("M{member}")).collect();
    let text = text(
        PROFILE_V,
        &format!("union Big {{ {} }}", members.join(", ")),
    );
    let assemble_under = |work_units: u64| {
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        let unit = build_unit(&parsed).expect("S2 builds the unit");
        PackageDeclarations::assemble_with_limits(
            parsed.source().reference().clone(),
            "test/package".to_owned(),
            unit,
            crate::model::intake::SelectedModels::default(),
            Vec::new(),
            crate::check::AssemblyLimits {
                environment: TypeEnvironmentLimits {
                    work_units,
                    ..crate::check::AssemblyLimits::default().environment
                },
                ..crate::check::AssemblyLimits::default()
            },
        )
    };
    let default = crate::check::AssemblyLimits::default()
        .environment
        .work_units;
    let admitted = assemble_under(default).expect("5,000 members are admitted by default");
    assert_eq!(members_of(composite_named(&admitted, "Big")).len(), 5_000);
    let needed = admitted.types.spent();
    let refusal = assemble_under(needed - 1).expect_err("one unit short of the work needed");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error, not {:?}", refusal.errors)
    };
    let AssemblyCause::TypeLimit(limit) = &error.cause else {
        panic!("a type-environment limit, not {:?}", error.cause)
    };
    assert_eq!(limit.configured_bound(), needed - 1);
    assert_eq!(
        limit.setting(),
        qsl_foundation::Setting::EnvironmentWorkUnits
    );
    assert!(
        assemble_under(needed).is_ok(),
        "the field raised by one admits it"
    );

    let nested = format!(
        "union Deep {{ Wrap({}Integer{}) }}",
        "Option<".repeat(1_000),
        ">".repeat(1_000)
    );
    assemble(&nested)
        .1
        .expect("1,000 nested payload levels are admitted");
}

/// FR-313 builds `case` and FR-318 checks it, so a unit holding one
/// assembles; the union that declares the scrutinee type is registered.
#[trace("FR-313-AC-2", "TC-815")]
#[test]
fn a_case_expression_assembles_over_a_registered_union() {
    let (_, assembled) = assemble(
        "union Shape { Empty, Circle(Integer) }\n\
         function f using v(s: Shape): Integer pure { case s { Circle(r): r; Empty: 0; } }",
    );
    assembled.expect("the unit assembles");
}

const LAYER_PROFILES: &str = "profile c = \"quire.state.core/v1\";\n\
                              profile q = \"quire.state.queries/v1\";\n";

/// The unit holding the layer profiles `c` (state core) and `q` (state
/// queries) and `declarations`, assembled.
fn assemble_layers(declarations: &str) -> (String, Result<PackageDeclarations, AssemblyRefusal>) {
    let text = text(LAYER_PROFILES, declarations);
    let assembled = assemble_as("a", "u", &text);
    (text, assembled)
}

/// The one error the assembler gives for `declarations`, which names layer
/// `layer`, the form outside its set and the located text.
fn layer_refusal(declarations: &str, located: &str) -> (HeaderLayer, LayerForm, String) {
    let (text, assembled) = assemble_layers(declarations);
    let refusal = assembled.expect_err("the layer refuses the form");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error, not {refusal:?}");
    };
    let AssemblyCause::UnsupportedLayerForm { layer, form } = error.cause else {
        panic!("a layer refusal, not {error:?}");
    };
    assert_eq!(&text[error.span.start..error.span.end], located);
    assert_eq!(
        error.cause.catalog_code().to_string(),
        format!("unsupported_construct/{}", form.refusal_cause())
    );
    (layer, form, text)
}

/// FR-110-AC-8: a named predicate declared under state core refuses
/// `unsupported_construct`/`declaration-form` at the declaration, naming the
/// layer; the same declaration under state queries assembles.
#[trace("TC-490", "FR-110-AC-8")]
#[test]
fn a_named_predicate_declaration_is_outside_state_core_and_inside_state_queries() {
    let declaration = "predicate Positive using c(x: Int[0, 9]): Boolean { x > 0 }";
    let (layer, form, _) = layer_refusal(declaration, declaration);
    assert_eq!(layer, HeaderLayer::StateCore);
    assert_eq!(form, LayerForm::PredicateDeclaration);
    assemble_layers(&declaration.replace("using c", "using q"))
        .1
        .expect("state queries admits a named predicate declaration");
}

/// FR-110-AC-8: an expression form outside the closure refuses
/// `unsupported_construct`/`expression-form` at the expression, and a form of
/// a wider layer is never admitted: state queries admits `filter` over a
/// predicate but not integer `div`, which only the value layer adds.
#[trace("TC-490", "FR-110-AC-8")]
#[test]
fn an_expression_form_of_a_wider_layer_is_refused_at_its_span() {
    let (layer, form, _) = layer_refusal(
        "predicate Half using q(x: Int[0, 9]): Boolean { x div 2 = 1 }",
        "x div 2",
    );
    assert_eq!(layer, HeaderLayer::StateQueries);
    assert_eq!(form, LayerForm::ValueConstruct);
    // The first offending form in source order is the one refused.
    let (_, form, _) = layer_refusal(
        "predicate Twice using q(x: Int[0, 9]): Boolean { x rem 2 = 0 and x div 2 = 1 }",
        "x rem 2",
    );
    assert_eq!(form, LayerForm::ValueConstruct);
}

/// FR-110-AC-8: a named predicate call is a state-queries form. A function
/// declaration is the value layer's, so a function using state queries
/// refuses as a declaration, not as the call in its body.
#[trace("TC-490", "FR-110-AC-8")]
#[test]
fn a_function_declaration_is_outside_state_queries() {
    let declarations = "predicate Positive using q(x: Int[0, 9]): Boolean { x > 0 }\n\
                        function f using q(x: Int[0, 9]): Boolean pure { Positive(x) }";
    let located = "function f using q(x: Int[0, 9]): Boolean pure { Positive(x) }";
    let (layer, form, _) = layer_refusal(declarations, located);
    assert_eq!(layer, HeaderLayer::StateQueries);
    assert_eq!(form, LayerForm::FunctionDeclaration);
}

/// FR-110-AC-8: the value layer admits what it adds to state core, and the
/// header layers are recorded in the assembled package, ascending.
#[trace("TC-490", "FR-110-AC-7")]
#[test]
fn the_value_layer_admits_functions_and_the_package_records_its_layers() {
    let package = assemble(
        "function f using v(x: Int[0, 9]): Boolean pure { x div 2 = 1 }\n\
         predicate P using v(x: Int[0, 9]): Boolean { f(x) }",
    )
    .1
    .expect("the value layer admits functions, predicates and integer division");
    assert_eq!(package.header_layers, [HeaderLayer::ValueComplete]);
}

fn parsed_unit_as(
    authority: &str,
    identity: &str,
    text: &str,
) -> (RawSourceRef, qsl_forms::ParsedUnit) {
    let parsed = qsl_cst::parse(
        SourceIdentity::new(authority, identity),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let source = parsed.source().reference().clone();
    (
        source,
        build_unit(&parsed).expect("S2 builds the source unit"),
    )
}

fn admitted_config_version_aliases(
    text: &str,
) -> (
    RawSourceRef,
    qsl_forms::ParsedUnit,
    crate::model::intake::SelectedModels,
) {
    use crate::model::intake::{admit_unit, package_input};

    let document = include_bytes!("../../../../examples/config-version/model.semantic-ir.json");
    let packages = package_input([document.as_slice()]).expect("one package document admits");
    let (source, unit) = parsed_unit_as("repo", "two-model-aliases", text);
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the same package identity admits under both aliases");
    (source, unit, selected)
}

/// FR-513-AC-2: one package identity selected under two aliases is admitted
/// once by intake and assembled while retaining both model-alias records.
/// This does not assert member-qualified type lookup or pin a package version.
#[trace("TC-1308", "FR-513-AC-2")]
#[test]
fn one_package_identity_assembles_under_two_aliases() {
    let source_text = text(
        "profile v = \"quire.value.complete/v1\";\n",
        "model Config = \"example/config-version\";\n\
         model History = \"example/config-version\";\n\
         predicate Ready using v(): Boolean { true }",
    );
    let (source, unit, selected) = admitted_config_version_aliases(&source_text);
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].alias, "Config");
    assert_eq!(selected[1].alias, "History");
    assert_eq!(selected[0].view, selected[1].view);

    let declarations = PackageDeclarations::assemble(
        source,
        "test/two-model-aliases".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("the shared package root contributes once");
    let models = &declarations
        .model_inputs
        .as_ref()
        .expect("admitted models")
        .0;
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].alias, "Config");
    assert_eq!(models[1].alias, "History");
    declarations
        .check(CheckingLimits::default())
        .expect("the unit with two aliases checks");
}

/// Repeated aliases do not project the same package population twice when a
/// state invariant resolves its model type.
#[trace("TC-1308", "FR-513-AC-2")]
#[test]
fn model_population_projection_is_unique_across_package_aliases() {
    let source_text = text(
        "profile s = \"quire.state.graph/v1\";\n",
        "model Config = \"example/config-version\";\n\
         model History = \"example/config-version\";\n\
         invariant ConfigNonNegative using s on Config::ConfigVersion at current { self.versionNumber >= 0 }",
    );
    let (source, unit, selected) = admitted_config_version_aliases(&source_text);
    let checked = PackageDeclarations::assemble(
        source,
        "test/two-model-aliases".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("the population is projected once for this package identity")
    .check(CheckingLimits::default())
    .expect("the Config alias invariant checks");
    assert_eq!(checked.state_clauses().len(), 1);
    assert_eq!(checked.state_clauses()[0].name(), "ConfigNonNegative");
}

/// `declarations` in a unit selecting `example/config-version` as `Config`
/// and then as `History`, under the model profile `s`, assembled and
/// checked.
fn check_two_aliases(declarations: &str) -> crate::check::CheckedGraph {
    let source_text = text(
        "profile s = \"quire.model.complete/v1\";\n",
        &format!(
            "model Config = \"example/config-version\";\n\
             model History = \"example/config-version\";\n{declarations}"
        ),
    );
    let (source, unit, selected) = admitted_config_version_aliases(&source_text);
    PackageDeclarations::assemble(
        source,
        "test/two-model-aliases".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{declarations}: assembles: {refusal:?}"))
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{declarations}: checks: {refusals:?}"))
}

/// FR-480-AC-9 (QSL-1578): the second alias of one admitted package names its
/// object type, scalar type, operation and population in every position the
/// first alias does -- a state clause's context, a `pre`/`post` operation, an
/// `allInstances` or `lookup` type argument and population, a function
/// signature and a population bound -- and each spelling resolves to the one package
/// declaration, so a `History::ConfigVersion` and a `Config::ConfigVersion`
/// are one type.
#[trace("FR-480-AC-9")]
#[test]
fn a_second_alias_names_every_package_type_as_the_first_alias_does() {
    for declarations in [
        "invariant A using s on History::ConfigVersion at current { self.versionNumber >= 0 }",
        "pre P using s on History::ConfigVersion::attemptUpdate { true }",
        "post P using s on History::ConfigVersion::attemptUpdate { self.versionNumber >= 0 }",
        "invariant A using s on History::ConfigVersion at current { \
         forall(o in allInstances<History::ConfigVersion>(History::config_history) : \
         o.versionNumber >= 0) }",
        "invariant A using s on Config::ConfigVersion at current { \
         forall(o in allInstances<History::ConfigVersion>(Config::config_history) : \
         o.versionNumber >= 0) }",
        "invariant A using s on History::ConfigVersion at current { \
         present(self.parent) implies deref(value(self.parent)).versionNumber >= 0 }",
        "bound Cap on History::config_history [3]\n\
         invariant A using s on History::ConfigVersion at current { self.versionNumber >= 0 }",
        "function q using s(p: Population<History::ConfigVersion>[1], \
         r: Config::ConfigVersion): History::ConfigVersion pure \
         { lookup<History::ConfigVersion>(p, r) absent refused }",
    ] {
        check_two_aliases(declarations);
    }

    let checked = check_two_aliases(
        "function same using s(c: History::ConfigVersion, d: Config::ConfigVersion): Boolean \
         pure { c = d }\n\
         function level using s(h: History::VersionNumber, c: Config::VersionNumber): Boolean \
         pure { h = c }",
    );
    let parameters = |name: &str| -> Vec<ValueType> {
        checked
            .callable(name)
            .unwrap_or_else(|| panic!("{name} is checked"))
            .parameters
            .iter()
            .map(|(_, value_type)| value_type.clone())
            .collect()
    };
    let same = parameters("same");
    assert!(matches!(&same[0], ValueType::Reference(_)), "{same:?}");
    assert_eq!(same[0], same[1], "both spellings name one object type");
    let level = parameters("level");
    assert!(matches!(&level[0], ValueType::Int(_)), "{level:?}");
    assert_eq!(level[0], level[1], "both spellings name one scalar type");
    let object_types: Vec<_> = checked
        .scope()
        .types()
        .object_types()
        .map(|declared| declared.name().to_owned())
        .collect();
    assert_eq!(
        object_types,
        ["Config::ConfigVersion"],
        "the package's object type is declared once, under its first alias"
    );
}

/// FR-480-AC-10 (QSL-1578): selecting the package under a second alias adds no
/// package-owned projection: the checked graph has the same model nodes, the
/// same one model root and the same operation-effect rows as the unit that
/// selects it once.
#[trace("FR-480-AC-10")]
#[test]
fn a_second_alias_projects_no_package_fact_twice() {
    let clauses = "post P using s on Config::ConfigVersion::attemptUpdate { \
                   self.versionNumber >= 0 }\n\
                   pre Q using s on History::ConfigVersion::attemptUpdate { true }";
    let check_with = |aliases: &str| {
        let source_text = text(
            "profile s = \"quire.model.complete/v1\";\n",
            &format!("{aliases}{}", clauses.replace("History::", "Config::")),
        );
        let (source, unit, selected) = admitted_config_version_aliases(&source_text);
        PackageDeclarations::assemble(
            source,
            "test/two-model-aliases".to_owned(),
            unit,
            selected,
            Vec::new(),
        )
        .expect("assembles")
        .check(CheckingLimits::default())
        .expect("checks")
    };
    let once = check_with("model Config = \"example/config-version\";\n");
    let twice = check_two_aliases(clauses);
    let model_nodes = |checked: &crate::check::CheckedGraph| -> Vec<quire_exact::NodeKey> {
        let mut nodes: Vec<_> = checked
            .semantic_graph()
            .nodes()
            .filter(|node| {
                matches!(
                    node.node_tag(),
                    crate::check::NodeTag::Model | crate::check::NodeTag::Relation
                )
            })
            .map(|node| node.key())
            .collect();
        nodes.sort();
        nodes
    };
    assert!(!model_nodes(&once).is_empty());
    assert_eq!(model_nodes(&once), model_nodes(&twice));
    assert_eq!(once.model_roots().count(), 1);
    assert_eq!(twice.model_roots().count(), 1);
    assert!(
        once.operation_effects()
            .is_some_and(|rows| !rows.is_empty()),
        "attemptUpdate has an effect row"
    );
    assert_eq!(once.operation_effects(), twice.operation_effects());
    assert_eq!(
        once.scope().operations(),
        twice.scope().operations(),
        "one operation table entry per package operation"
    );
}

/// FR-480-AC-12 (QSL-1578): a second alias is still a unit alias. Spelled like
/// a profile alias it refuses as a duplicate alias.
#[trace("FR-480-AC-12")]
#[test]
fn a_second_alias_spelled_like_a_profile_alias_refuses_as_a_duplicate() {
    let source_text = text(
        "profile s = \"quire.model.complete/v1\";\nprofile History = \"quire.value.complete/v1\";\n",
        "model Config = \"example/config-version\";\n\
         model History = \"example/config-version\";\n\
         invariant A using s on History::ConfigVersion at current { self.versionNumber >= 0 }",
    );
    let (source, unit, selected) = admitted_config_version_aliases(&source_text);
    let refusal = PackageDeclarations::assemble(
        source,
        "test/two-model-aliases".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect_err("a profile and a model alias share the spelling History");
    let duplicate = refusal
        .errors
        .iter()
        .find(|error| {
            matches!(
                &error.cause,
                AssemblyCause::DuplicateAlias { alias, .. } if alias == "History"
            )
        })
        .unwrap_or_else(|| panic!("History refuses as a duplicate alias: {refusal:?}"));
    assert_eq!(duplicate.cause.code(), Code::AmbiguousDeclaration);
    assert_eq!(
        duplicate.cause.catalog_code().to_string(),
        "ambiguous_declaration/ambiguous-name"
    );
}

/// FR-480-AC-13 (QSL-1578): a unit declaration that shares a package type's
/// artifact name keeps its own bare name, apart from both alias spellings of
/// the package type.
#[trace("FR-480-AC-13")]
#[test]
fn a_unit_record_keeps_its_bare_name_apart_from_both_alias_spellings() {
    let checked = check_two_aliases(
        "record ConfigVersion { versionNumber: Integer; }\n\
         function local using s(r: ConfigVersion, c: History::ConfigVersion, \
         d: Config::ConfigVersion): Boolean pure { r.versionNumber >= 0 and c = d }",
    );
    let parameters: Vec<ValueType> = checked
        .callable("local")
        .expect("local is checked")
        .parameters
        .iter()
        .map(|(_, value_type)| value_type.clone())
        .collect();
    assert!(
        matches!(&parameters[0], ValueType::Composite(_)),
        "{parameters:?}"
    );
    assert!(
        matches!(&parameters[1], ValueType::Reference(_)),
        "{parameters:?}"
    );
    assert_eq!(parameters[1], parameters[2]);
}

/// `example/config-version` and a copy of it under
/// `example/alternate-config-version` that declares the same type names.
fn config_version_and_alternate() -> crate::model::intake::PackageInput {
    let document = include_bytes!("../../../../examples/config-version/model.semantic-ir.json");
    let alternate = String::from_utf8(document.to_vec())
        .expect("the model document is UTF-8")
        .replace("example/config-version", "example/alternate-config-version")
        .into_bytes();
    crate::model::intake::package_input([document.as_slice(), alternate.as_slice()])
        .expect("both package documents admit")
}

/// The units `(identity, declarations)`, each under the model profile `s`,
/// with their model selections admitted together and assembled as one
/// package.
fn assemble_model_units(units: &[(&str, &str)]) -> Result<PackageDeclarations, AssemblyRefusal> {
    assemble_model_units_with("profile s = \"quire.model.complete/v1\";\n", units)
}

/// [`assemble_model_units`] with the header `profiles`.
fn assemble_model_units_with(
    profiles: &str,
    units: &[(&str, &str)],
) -> Result<PackageDeclarations, AssemblyRefusal> {
    let parsed: Vec<_> = units
        .iter()
        .map(|(identity, declarations)| {
            parsed_unit_as("repo", identity, &text(profiles, declarations))
        })
        .collect();
    let selections: Vec<_> = parsed
        .iter()
        .map(|(_, unit)| unit.selections().models.as_slice())
        .collect();
    let selected = crate::model::intake::admit_units_with_cancel(
        &selections,
        &config_version_and_alternate(),
        ModelNormalizationLimits::default(),
        &Cancel::new(),
    )
    .expect("each unit's model selections admit");
    PackageDeclarations::assemble_units(
        "test/model-units".to_owned(),
        parsed,
        selected,
        Vec::new(),
        super::AssemblyLimits::default(),
        &Cancel::new(),
    )
}

/// FR-480-AC-9 (QSL-1578): two units selecting one package under two
/// aliases each name its object and scalar types through their own alias,
/// and both spellings resolve to the one declaration: a function of one
/// unit accepts the other unit's values.
#[trace("FR-480-AC-9")]
#[test]
fn two_units_name_one_package_through_their_own_aliases() {
    let checked = assemble_model_units(&[
        (
            "a",
            "model Config = \"example/config-version\";\n\
             function f using s(c: Config::ConfigVersion, v: Config::VersionNumber): Boolean \
             pure { c = c and v >= 0 }",
        ),
        (
            "b",
            "model History = \"example/config-version\";\n\
             function g using s(c: History::ConfigVersion, v: History::VersionNumber): Boolean \
             pure { f(c, v) }\n\
             invariant A using s on History::ConfigVersion at current { self.versionNumber >= 0 }",
        ),
    ])
    .unwrap_or_else(|refusal| panic!("both units assemble: {refusal:?}"))
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("both units check: {refusals:?}"));
    let parameters = |name: &str| -> Vec<ValueType> {
        checked
            .callable(name)
            .unwrap_or_else(|| panic!("{name} is checked"))
            .parameters
            .iter()
            .map(|(_, value_type)| value_type.clone())
            .collect()
    };
    let (f, g) = (parameters("f"), parameters("g"));
    assert!(matches!(&f[0], ValueType::Reference(_)), "{f:?}");
    assert!(matches!(&f[1], ValueType::Int(_)), "{f:?}");
    assert_eq!(
        f, g,
        "each unit's alias names the same object and scalar types"
    );
    let object_types: Vec<_> = checked
        .scope()
        .types()
        .object_types()
        .map(|declared| declared.name().to_owned())
        .collect();
    assert_eq!(object_types, ["Config::ConfigVersion"]);
    assert_eq!(checked.model_roots().count(), 1);
}

/// The object type `function`'s first parameter names, in `checked`.
fn first_parameter(checked: &crate::check::CheckedGraph, function: &str) -> ValueType {
    checked
        .callable(function)
        .unwrap_or_else(|| panic!("{function} is checked"))
        .parameters[0]
        .1
        .clone()
}

/// The object type the invariant `clause` is over, in `checked`.
fn clause_context(checked: &crate::check::CheckedGraph, clause: &str) -> ValueType {
    ValueType::Reference(
        checked
            .state_clauses()
            .iter()
            .find(|checked| checked.name() == clause)
            .unwrap_or_else(|| panic!("{clause} is checked"))
            .context(),
    )
}

/// FR-480-AC-18 (QSL-1613): two units that each select one package under
/// the same alias `M` both name its types through `M`. Whichever unit is
/// canonically later, and in either request order, its `M::ConfigVersion`
/// resolves to the type the other unit's `M::ConfigVersion` names.
#[trace("FR-480-AC-18")]
#[test]
fn two_units_selecting_one_package_under_one_alias_both_name_its_types() {
    let unit = |function: &str| {
        format!(
            "model M = \"example/config-version\";\n\
             function {function} using s(c: M::ConfigVersion): Boolean pure {{ true }}\n\
             invariant {function}_inv using s on M::ConfigVersion at current \
             {{ self.versionNumber >= 0 }}"
        )
    };
    let (a, b) = (unit("fa"), unit("fb"));
    for units in [
        [("a", a.as_str()), ("b", b.as_str())],
        [("b", b.as_str()), ("a", a.as_str())],
    ] {
        let checked = assemble_model_units(&units)
            .unwrap_or_else(|refusal| panic!("both units' M::ConfigVersion resolve: {refusal:?}"))
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("both units check: {refusals:?}"));
        let fa = first_parameter(&checked, "fa");
        assert!(matches!(&fa, ValueType::Reference(_)), "{fa:?}");
        assert_eq!(fa, first_parameter(&checked, "fb"));
        assert_eq!(clause_context(&checked, "fa_inv"), fa);
        assert_eq!(clause_context(&checked, "fb_inv"), fa);
    }
}

/// FR-480-AC-14 (QSL-1613): model aliases are local to their unit. Unit `a`
/// selects one package as `M` and `N`, and unit `b` binds `N` to another
/// package that declares the same type names. Each unit's
/// `N::ConfigVersion`, as a state clause's context or a parameter type,
/// names its own alias's package: `a`'s the type `M::ConfigVersion` names,
/// `b`'s the other package's.
#[trace("FR-480-AC-14")]
#[test]
fn one_alias_two_units_bind_to_two_packages_names_each_units_own_package() {
    let checked = assemble_model_units(&[
        (
            "a",
            "model M = \"example/config-version\";\n\
             model N = \"example/config-version\";\n\
             function fa using s(c: N::ConfigVersion, d: M::ConfigVersion): Boolean \
             pure { c = d }\n\
             invariant A using s on N::ConfigVersion at current { self.versionNumber >= 0 }",
        ),
        (
            "b",
            "model N = \"example/alternate-config-version\";\n\
             function fb using s(c: N::ConfigVersion): Boolean pure { true }\n\
             invariant B using s on N::ConfigVersion at current { self.versionNumber >= 0 }",
        ),
    ])
    .unwrap_or_else(|refusal| panic!("each unit's N::ConfigVersion resolves: {refusal:?}"))
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("both units check: {refusals:?}"));
    let a = first_parameter(&checked, "fa");
    let b = first_parameter(&checked, "fb");
    let m = checked.callable("fa").expect("fa is checked").parameters[1]
        .1
        .clone();
    assert!(matches!(&a, ValueType::Reference(_)), "{a:?}");
    assert_eq!(
        a, m,
        "a's N::ConfigVersion is the type M::ConfigVersion names"
    );
    assert_ne!(a, b, "b's N::ConfigVersion is the other package's type");
    assert_eq!(clause_context(&checked, "A"), a);
    assert_eq!(clause_context(&checked, "B"), b);
}

/// FR-480-AC-15 (QSL-1613): two units each select a different package under
/// the first alias `N`, both packages declaring `ConfigVersion`, so both
/// project `N::ConfigVersion`. Each unit's state clause context and
/// parameter type resolve to its own package's type, never to the other
/// unit's.
#[trace("FR-480-AC-15")]
#[test]
fn one_first_alias_of_two_packages_names_each_units_own_package() {
    let checked = assemble_model_units(&[
        (
            "a",
            "model N = \"example/config-version\";\n\
             function fa using s(c: N::ConfigVersion): Boolean pure { true }\n\
             invariant A using s on N::ConfigVersion at current { self.versionNumber >= 0 }",
        ),
        (
            "b",
            "model N = \"example/alternate-config-version\";\n\
             function fb using s(c: N::ConfigVersion): Boolean pure { true }\n\
             invariant B using s on N::ConfigVersion at current { self.versionNumber >= 0 }",
        ),
    ])
    .unwrap_or_else(|refusal| panic!("each unit's N::ConfigVersion resolves: {refusal:?}"))
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("both units check: {refusals:?}"));
    let a = first_parameter(&checked, "fa");
    let b = first_parameter(&checked, "fb");
    assert!(matches!(&a, ValueType::Reference(_)), "{a:?}");
    assert_ne!(a, b, "each unit's N names its own package");
    assert_eq!(clause_context(&checked, "A"), a);
    assert_eq!(clause_context(&checked, "B"), b);
}

/// Unit `a` selecting `example/config-version` as `N` with `a`, and unit
/// `b` selecting the alternate package as `N` with `b`.
fn one_alias_two_packages(a: &str, b: &str) -> [(&'static str, String); 2] {
    [
        ("a", format!("model N = \"example/config-version\";\n{a}")),
        (
            "b",
            format!("model N = \"example/alternate-config-version\";\n{b}"),
        ),
    ]
}

/// The admitted model of `declarations` selecting `identity`.
fn admitted_model<'d>(
    declarations: &'d PackageDeclarations,
    identity: &str,
) -> &'d crate::check::lowering::AdmittedModel {
    declarations
        .models
        .iter()
        .find(|model| model.selection().identity == identity)
        .unwrap_or_else(|| panic!("{identity} is admitted"))
}

/// FR-480 "Selections and imports" (QSL-1638): a `terminal` declaration's
/// model alias names the package its own unit binds it to. Unit `b`, the
/// later unit, binds `N` to the alternate package: its `terminal ... on N`
/// is on that package, its `self` that package's state object, though unit
/// `a`'s `N` is admitted first.
#[trace("FR-480")]
#[test]
fn a_terminal_on_a_shared_alias_is_on_its_own_units_package() {
    let units = one_alias_two_packages(
        "type Pad = Integer;",
        "terminal Done using s on N when { self.versionNumber = 1000 };",
    );
    let units: Vec<_> = units
        .iter()
        .map(|(id, text)| (*id, text.as_str()))
        .collect();
    let declarations = assemble_model_units(&units)
        .unwrap_or_else(|refusal| panic!("b's terminal resolves: {refusal:?}"));
    let [terminal] = declarations.terminals.as_slice() else {
        panic!("one terminal: {:?}", declarations.terminals);
    };
    assert_eq!(terminal.package, "example/alternate-config-version");
    let crate::check::TerminalClauseKind::When { context, .. } = &terminal.kind else {
        panic!("a `when` terminal: {:?}", terminal.kind);
    };
    let alternate = admitted_model(&declarations, "example/alternate-config-version");
    let original = admitted_model(&declarations, "example/config-version");
    assert!(
        alternate.population_objects().contains(context),
        "{context:?}"
    );
    assert!(
        !original.population_objects().contains(context),
        "{context:?}"
    );
    declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("b's terminal checks: {refusals:?}"));
}

/// FR-480 "Selections and imports" (QSL-1638): a `bound` declaration's model
/// alias names the package its own unit binds it to. Unit `b`'s bound on
/// `N::config_history` bounds the alternate package's population, and unit
/// `a`'s package keeps none.
#[trace("FR-480")]
#[test]
fn a_population_bound_on_a_shared_alias_bounds_its_own_units_package() {
    let units = one_alias_two_packages(
        "type Pad = Integer;",
        "bound Cap on N::config_history [3]\n\
         invariant B using s on N::ConfigVersion at current { self.versionNumber >= 0 }",
    );
    let units: Vec<_> = units
        .iter()
        .map(|(id, text)| (*id, text.as_str()))
        .collect();
    let declarations = assemble_model_units(&units)
        .unwrap_or_else(|refusal| panic!("b's bound resolves: {refusal:?}"));
    let bounds = |identity: &str| -> Vec<(String, String)> {
        admitted_model(&declarations, identity)
            .population_bounds()
            .map(|bound| (bound.label.clone(), bound.population.package.clone()))
            .collect()
    };
    assert_eq!(
        bounds("example/alternate-config-version"),
        [(
            "Cap".to_owned(),
            "example/alternate-config-version".to_owned()
        )]
    );
    assert_eq!(bounds("example/config-version"), []);
    declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("b's bound checks: {refusals:?}"));
}

/// FR-480 "Selections and imports" (QSL-1638): a static protocol role on a
/// whole model alias binds the package its own unit binds the alias to.
/// Each unit's `role w on N` selects its own package, whichever unit is
/// later.
#[trace("FR-480")]
#[test]
fn a_whole_model_role_on_a_shared_alias_selects_its_own_units_package() {
    let protocol = |name: &str| {
        format!(
            "protocol {name} using p over (c: N::ConfigVersion) on origin {{\n\
             role w on N;\n\
             run sequence Main {{ }}\n\
             finish Done as (d: Boolean) {{ true }};\n\
             }}"
        )
    };
    let (a, b) = (protocol("Pa"), protocol("Pb"));
    let units = one_alias_two_packages(&a, &b);
    let units: Vec<_> = units
        .iter()
        .map(|(id, text)| (*id, text.as_str()))
        .collect();
    let checked = assemble_model_units_with(
        "profile s = \"quire.model.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n",
        &units,
    )
    .unwrap_or_else(|refusal| panic!("both protocols assemble: {refusal:?}"))
    .with_unwritten_protocols()
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("both protocols check: {refusals:?}"));
    let selection = |name: &str| {
        let protocol = checked
            .protocols()
            .iter()
            .find(|protocol| protocol.name == name)
            .unwrap_or_else(|| panic!("{name} is checked"));
        let [role] = protocol.roles.as_slice() else {
            panic!("{name} has one role: {:?}", protocol.roles);
        };
        role.model_selection.clone()
    };
    assert_eq!(selection("Pa").as_deref(), Some("example/config-version"));
    assert_eq!(
        selection("Pb").as_deref(),
        Some("example/alternate-config-version")
    );
}

/// FR-480-AC-16 (QSL-1613): a unit never reaches a package through another
/// unit's model alias: unit `b`, which binds no `M`, refuses
/// `M::ConfigVersion` with `missing_declaration`/`missing-name` in every
/// position a type name is written -- a state clause's context, a parameter
/// type, an `allInstances` and a `lookup` type argument.
#[trace("FR-480-AC-16")]
#[test]
fn a_unit_reaches_no_package_through_another_units_alias() {
    for declaration in [
        "invariant B using s on M::ConfigVersion at current { self.versionNumber >= 0 }",
        "function fb using s(c: M::ConfigVersion): Boolean pure { true }",
        "invariant B using s on N::ConfigVersion at current { \
         forall(o in allInstances<M::ConfigVersion>(N::config_history) : \
         o.versionNumber >= 0) }",
        "function q using s(p: Population<N::ConfigVersion>[1], r: N::ConfigVersion): \
         N::ConfigVersion pure { lookup<M::ConfigVersion>(p, r) absent refused }",
    ] {
        let assembled = assemble_model_units(&[
            (
                "a",
                "model M = \"example/config-version\";\ntype Pad = Integer;",
            ),
            (
                "b",
                &format!("model N = \"example/alternate-config-version\";\n{declaration}"),
            ),
        ]);
        let codes: Vec<String> = match assembled {
            Err(refusal) => refusal
                .errors
                .iter()
                .map(|error| error.cause.catalog_code().to_string())
                .collect(),
            Ok(declarations) => declarations
                .check(CheckingLimits::default())
                .err()
                .unwrap_or_else(|| panic!("b binds no M: {declaration}"))
                .iter()
                .filter_map(|refusal| refusal.cause.catalog_code())
                .map(|code| code.to_string())
                .collect(),
        };
        assert!(
            codes
                .iter()
                .any(|code| code.ends_with("missing_declaration/missing-name")),
            "{declaration}: {codes:?}"
        );
    }
}

/// FR-480-AC-2, AC-5, AC-8: the aggregate assembler resolves a function
/// signature against another unit, keeps the declaring unit on the type
/// node and its source region, and assigns stable identities in canonical
/// source order independent of request order.
#[trace("FR-480-AC-2", "FR-480-AC-5", "FR-480-AC-8")]
#[test]
fn aggregate_units_resolve_types_and_retain_the_declaring_source() {
    let a_text = text("", "record Point { x: Int[0, 9]; }");
    let b_text = text(
        PROFILE_V,
        "function px using v(p: Point): Int[0, 9] pure { p.x }",
    );
    let (source_a, unit_a) = parsed_unit_as("repo", "a", &a_text);
    let (source_b, unit_b) = parsed_unit_as("repo", "b", &b_text);
    let assemble = |units| {
        PackageDeclarations::assemble_units(
            "test/package".to_owned(),
            units,
            crate::model::intake::SelectedModels::default(),
            Vec::new(),
            super::AssemblyLimits::default(),
            &Cancel::new(),
        )
        .expect("the aggregate namespace resolves Point")
        .check(CheckingLimits::default())
        .expect("the cross-unit function checks")
    };
    let first = assemble(vec![
        (source_b.clone(), unit_b.clone()),
        (source_a.clone(), unit_a.clone()),
    ]);
    let second = assemble(vec![(source_a.clone(), unit_a), (source_b.clone(), unit_b)]);

    let callable = first.callable("px").expect("px is checked");
    assert!(matches!(&callable.parameters[0].1, ValueType::Composite(_)));
    let region = first
        .region(&Location::root(Origin::TypeDeclaration {
            name: "Point".to_owned(),
        }))
        .expect("Point resolves to its own source region");
    assert_eq!(region.source(), &source_a);
    assert_eq!(first.source(), &source_a);
    assert_eq!(first.sources(), vec![&source_a, &source_b]);
    let body_region = first
        .region(&Location::root(Origin::Body {
            function: "px".to_owned(),
            index: 0,
        }))
        .expect("px resolves to its own source region");
    assert_eq!(body_region.source(), &source_b);
    assert_eq!(
        first.function_identity("px"),
        second.function_identity("px"),
        "request order does not change function identity"
    );
    let first_point = first.checked_type_nodes().next().unwrap().node();
    let second_point = second.checked_type_nodes().next().unwrap().node();
    assert_eq!(first_point, second_point);

    let moved_text = text(
        PROFILE_V,
        "record Point { x: Int[0, 9]; }\nfunction px using v(p: Point): Int[0, 9] pure { p.x }",
    );
    let (source_b, moved_unit) = parsed_unit_as("repo", "b", &moved_text);
    let moved = assemble(vec![(source_b.clone(), moved_unit)]);
    let moved_point = moved.checked_type_nodes().next().unwrap().node();
    assert_eq!(
        first_point, moved_point,
        "the structural node identity remains stable when ownership moves"
    );
    let point_owner = |graph: &crate::check::CheckedGraph| {
        graph
            .semantic_graph()
            .nodes()
            .find(|node| {
                node.declaration().is_some_and(|segments| {
                    segments
                        .last()
                        .is_some_and(|segment| segment.as_str() == "Point")
                })
            })
            .and_then(|node| node.owner())
            .cloned()
            .expect("Point's checked node retains its source owner")
    };
    assert_eq!(
        point_owner(&first),
        Owner::Source(SourceOwner::from(&source_a))
    );
    assert_eq!(
        point_owner(&moved),
        Owner::Source(SourceOwner::from(&source_b))
    );
    let px_owner = first
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.declaration().is_some_and(|segments| {
                segments
                    .last()
                    .is_some_and(|segment| segment.as_str() == "px")
            })
        })
        .and_then(|node| node.owner())
        .cloned()
        .expect("px's checked node retains its source owner");
    assert_eq!(px_owner, Owner::Source(SourceOwner::from(&source_b)));
    let moved_region = moved
        .region(&Location::root(Origin::TypeDeclaration {
            name: "Point".to_owned(),
        }))
        .expect("moved Point resolves to its new source region");
    assert_eq!(moved_region.source(), &source_b);
}

/// FR-480-AC-2: a duplicate declaration is still ambiguous after the
/// aggregate assembler has merged all units into one namespace.
#[trace("FR-480-AC-2")]
#[test]
fn aggregate_units_refuse_duplicate_type_declarations() {
    let (source_a, unit_a) =
        parsed_unit_as("repo", "a", &text("", "record Point { x: Int[0, 9]; }"));
    let (source_b, unit_b) =
        parsed_unit_as("repo", "b", &text("", "record Point { y: Int[0, 9]; }"));
    let refusal = PackageDeclarations::assemble_units(
        "test/package".to_owned(),
        vec![(source_a, unit_a), (source_b, unit_b)],
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
        super::AssemblyLimits::default(),
        &Cancel::new(),
    )
    .expect_err("two Point declarations are ambiguous");
    assert!(refusal.errors.iter().any(|error| matches!(
        &error.cause,
        AssemblyCause::AmbiguousTypeName { name, .. } if name == "Point"
    )));
}

/// FR-480-AC-5: a dimension equivalence declared in a later unit keeps that
/// unit's owner on its checked node and resolves its declaration region to
/// that unit's source.
#[trace("FR-480-AC-5")]
#[test]
fn aggregate_dimension_equivalence_keeps_its_declaring_source_owner() {
    let a_text = text(
        "",
        "dimension Length; dimension Time; unit m : Length = rational(1, 1); unit s : Time = rational(1, 1);",
    );
    let b_text = text(
        PROFILE_V,
        "dimension Distance; dimension Duration; unit dm : Distance = rational(1, 1); unit ms : Duration = rational(1, 1); dimension Distance ~ Length by 1 dm = 1 m;\nfunction later using v(x: Quantity<dm>, y: Quantity<ms>): Boolean pure { true }",
    );
    let (source_a, unit_a) = parsed_unit_as("repo", "a", &a_text);
    let (source_b, unit_b) = parsed_unit_as("repo", "b", &b_text);
    let package = PackageDeclarations::assemble_units(
        "test/package".to_owned(),
        vec![(source_b.clone(), unit_b), (source_a, unit_a)],
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
        super::AssemblyLimits::default(),
        &Cancel::new(),
    )
    .expect("the cross-unit dimensions and units resolve");
    for name in ["Distance", "Duration", "dm", "ms"] {
        assert_eq!(
            package.source_owners.get(name),
            Some(&source_b),
            "the aggregate owner map records {name} under its bare declaration name"
        );
    }
    for (kind, name) in [
        (super::super::lowering::QuantityKind::Dimension, "Distance"),
        (super::super::lowering::QuantityKind::Dimension, "Duration"),
        (super::super::lowering::QuantityKind::Unit, "dm"),
        (super::super::lowering::QuantityKind::Unit, "ms"),
    ] {
        let origin = super::super::lowering::quantity_origin(kind, name);
        assert_eq!(
            package.source_owners.get(&origin),
            Some(&source_b),
            "the aggregate owner map records the quantity origin for {name}"
        );
    }
    let checked = package
        .check(CheckingLimits::default())
        .expect("the package checks");
    let node = checked
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "dimension_equivalence")
        .expect("the equivalence has a checked node");
    assert_eq!(
        node.owner(),
        Some(&Owner::Source(SourceOwner::from(&source_b)))
    );
    for name in ["Distance", "Duration", "dm", "ms"] {
        let owned = checked
            .semantic_graph()
            .nodes()
            .find(|candidate| {
                candidate.declaration().is_some_and(|segments| {
                    segments
                        .last()
                        .is_some_and(|segment| segment.as_str() == name)
                })
            })
            .unwrap_or_else(|| panic!("{name} has a checked node"));
        assert_eq!(
            owned.owner(),
            Some(&Owner::Source(SourceOwner::from(&source_b))),
            "{name} retains the source that declared it"
        );
    }
    let region = checked
        .region(&Location::root(Origin::TypeDeclaration {
            name: super::super::lowering::equivalence_origin(0),
        }))
        .expect("the equivalence declaration resolves to its source");
    assert_eq!(region.source(), &source_b);
    for (kind, name) in [
        (super::super::lowering::QuantityKind::Dimension, "Distance"),
        (super::super::lowering::QuantityKind::Dimension, "Duration"),
        (super::super::lowering::QuantityKind::Unit, "dm"),
        (super::super::lowering::QuantityKind::Unit, "ms"),
    ] {
        let origin = super::super::lowering::quantity_origin(kind, name);
        let region = checked
            .region(&Location::root(Origin::TypeDeclaration { name: origin }))
            .unwrap_or_else(|| panic!("{name}'s type declaration resolves to a region"));
        assert_eq!(
            region.source(),
            &source_b,
            "{name}'s type declaration region uses its declaring source"
        );
    }
}

const COMPOUND_TYPE_UNITS: &str = "dimension Mass; dimension Length; dimension Time;
unit kg : Mass = rational(1, 1); unit m : Length = rational(1, 1); unit s : Time = rational(1, 1);
dimension Energy = Mass * Length^2 * Time^-2; unit J : Energy = rational(1, 1);";

#[trace("TC-483", "FR-091-AC-74")]
#[test]
fn compound_quantity_types_keep_exact_canonical_identity_and_held_unit_metadata() {
    use quire_semantic_value::quantity::QuantityUnit;
    let source = format!("{COMPOUND_TYPE_UNITS}
        function u25 using v(q: Quantity<kg*m^2*s^-2>): Quantity<kg*m^2*s^-2> pure {{ q }}
        function reordered using v(q: Quantity<s^-2*kg*m*m>): Quantity<kg*m^2*s^-2> pure {{ q }}
        function converted using v(q: Quantity<J>): Quantity<kg*m^2*s^-2> pure {{ convert<Quantity<kg*m^2*s^-2>>(q) }}
        function bare using v(q: Quantity<kg>): Quantity<kg> pure {{ q }}
        function power using v(q: Quantity<kg^1>): Quantity<kg*m^0> pure {{ q }}
        function empty using v(q: Quantity<m*m^-1>): Quantity<m^0> pure {{ q }}
        function carried using v(q: Quantity<kg, Int[0, 10]>): Quantity<kg, Int[0, 10]> pure {{ q }}");
    let package = assemble(&source).1.expect("compound type assembly");
    let roots: BTreeMap<_, _> = package
        .units
        .units()
        .map(|unit| {
            let name = package
                .types
                .units()
                .nominal_node(&unit.key())
                .unwrap()
                .declaration
                .qualified_declaration[0]
                .clone();
            (name, unit.key())
        })
        .collect();
    let signature_unit = |index| match &package.resolved_signatures.get(index).unwrap().0[0].1 {
        ValueType::Quantity(id) => id.clone(),
        other => panic!("quantity: {other:?}"),
    };
    let u25 = signature_unit(0);
    assert_eq!(u25, signature_unit(1));
    assert_ne!(u25, signature_unit(2));
    let Some(QuantityUnit::Compound(unit)) = package.types.units().get(u25.clone()) else {
        panic!("held compound metadata")
    };
    let actual: BTreeMap<_, _> = unit
        .terms()
        .map(|(root, exponent)| (root, exponent.to_string()))
        .collect();
    assert_eq!(
        actual,
        [
            (roots["kg"].clone(), "1".to_owned()),
            (roots["m"].clone(), "2".to_owned()),
            (roots["s"].clone(), "-2".to_owned())
        ]
        .into()
    );
    let bare = signature_unit(3);
    let power = signature_unit(4);
    assert_ne!(bare, power);
    assert!(matches!(
        package.types.units().get(bare),
        Some(QuantityUnit::Declared(_))
    ));
    let Some(QuantityUnit::Compound(unit)) = package.types.units().get(power) else {
        panic!("explicit power stays compound")
    };
    assert_eq!(
        unit.terms()
            .map(|(root, n)| (root, n.to_string()))
            .collect::<Vec<_>>(),
        [(roots["kg"].clone(), "1".to_owned())]
    );
    let Some(QuantityUnit::Compound(unit)) = package.types.units().get(signature_unit(5)) else {
        panic!("empty compound")
    };
    assert_eq!(unit.terms().count(), 0);
    let carried = &package.resolved_signatures.get(6).unwrap().0[0].1;
    assert!(matches!(carried, ValueType::CarriedQuantity(_)));
    let checked = package
        .check(CheckingLimits::default())
        .expect("compound types check and key");
    assert_eq!(
        first_parameter(&checked, "u25"),
        ValueType::Quantity(u25.clone())
    );
    let empty = quire_semantic_value::quantity::UnitTable::default();
    let mut retained = quire_semantic_value::quantity::UnitScope::new(&empty);
    checked.evaluation_units.seed(&mut retained);
    assert!(matches!(retained.get(u25), Some(QuantityUnit::Compound(_))));
}

#[trace("TC-483", "FR-091-AC-75")]
#[test]
fn compound_quantity_type_unknown_units_refuse_at_the_factor_even_at_zero_power() {
    for spelling in ["kg*ghost^2", "ghost^0", "Mass^0"] {
        let source = format!("{COMPOUND_TYPE_UNITS} function f using v(q: Quantity<{spelling}>): Boolean pure {{ true }}");
        let (text, found) = errors(&source);
        let [error] = found.as_slice() else {
            panic!("one missing factor: {found:?}")
        };
        let factor = if spelling.contains("ghost") {
            "ghost"
        } else {
            "Mass"
        };
        assert!(
            matches!(&error.cause, AssemblyCause::UnresolvedUnitName { name, packages } if name == factor && packages.is_empty())
        );
        assert_eq!(
            error.cause.catalog_code().to_string(),
            "missing_declaration/missing-name"
        );
        assert_eq!(error.span, last(&text, factor));
    }
}

#[trace("TC-483", "FR-091-AC-75")]
#[test]
fn compound_quantity_type_exponents_obey_i128_without_narrowing() {
    for exponent in [
        i128::MIN.to_string(),
        i128::MAX.to_string(),
        "18446744073709551616".to_owned(),
    ] {
        let source = format!("{COMPOUND_TYPE_UNITS} function f using v(q: Quantity<m^{exponent}>): Quantity<m^{exponent}> pure {{ q }}");
        let package = assemble(&source).1.expect("i128 exponent assembles");
        let ValueType::Quantity(id) = &package.resolved_signatures.get(0).unwrap().0[0].1 else {
            panic!("quantity")
        };
        let Some(quire_semantic_value::quantity::QuantityUnit::Compound(unit)) =
            package.types.units().get(id.clone())
        else {
            panic!("compound")
        };
        assert_eq!(unit.terms().next().unwrap().1.to_string(), exponent);
        package
            .check(CheckingLimits::default())
            .expect("wide exponent checks");
    }
    for exponent in [
        "170141183460469231731687303715884105728",
        "-170141183460469231731687303715884105729",
    ] {
        let source = format!("{COMPOUND_TYPE_UNITS} function f using v(q: Quantity<m^{exponent}>): Boolean pure {{ true }}");
        let (_, found) = errors(&source);
        assert!(
            matches!(&found[0].cause, AssemblyCause::IllFormedBounds(TypeFormFault::IntegerOutsideI128(outside)) if outside.site == crate::check::IntegerSite::UnitExponent && outside.value == exponent)
        );
    }
}

#[trace("TC-483", "FR-091-AC-75")]
#[test]
fn compound_quantity_type_factors_reuse_the_quantity_cores_affine_refusal() {
    let units = "dimension Temperature; unit K : Temperature = rational(1, 1); unit C : Temperature = rational(1, 1) * K + rational(27315, 100);";
    for expression in ["C^1", "C^0", "K*C"] {
        let (_, found) = errors(&format!(
            "{units} function f using v(q: Quantity<{expression}>): Boolean pure {{ true }}"
        ));
        let [error] = found.as_slice() else {
            panic!("one affine refusal: {found:?}")
        };
        assert!(matches!(
            error.cause,
            AssemblyCause::IllFormedBounds(TypeFormFault::UnitArithmetic(
                quire_exact::IllTypedCause::AffineUnitArithmetic
            ))
        ));
        assert_eq!(
            error.cause.catalog_code().to_string(),
            "ill_typed/operator-ineligible"
        );
    }
}

/// FR-056-AC-43, FR-505-AC-6 (TC-897 step 3, TC-1300): a bare native `Text`,
/// `Rational` or `Decimal` has no checked type -- the member is declared with
/// no kernel type and refused where used. Read below the semantic-IR
/// validator, whose pinned native set names no `Rational`.
#[trace("TC-897")]
#[test]
fn a_bare_native_text_rational_or_decimal_maps_to_no_checked_type() {
    let single = Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    };
    let none = BTreeMap::new();
    for native in [
        NativeValueType::Text,
        NativeValueType::Rational,
        NativeValueType::Decimal,
    ] {
        let mapped = model_value_type(
            &ValueTypeRef::Native(native),
            single,
            &none,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert!(
            matches!(mapped, Err(super::Unmapped::Untyped(_))),
            "{native:?}"
        );
    }
}

/// QSpec FR-154 "Map types" (FR-154-AC-48): a package map type assembles
/// with its declared key type and bound. A native `Text` key, bare or through
/// an alias, is keyed as a `String` is; a `Float32` or `Float64` key draws the
/// floating-type refusal a floating member does; a map with a bound carries it.
#[trace("QSpec-FR-154-AC-48", "FR-230-AC-19")]
#[test]
fn a_package_map_type_assembles_with_its_declared_key_and_bound() {
    let map = |name: &str, keys: ValueTypeRef, upper: Option<u64>| {
        DomainPackageRecord::MapType(MapTypeRecord {
            key: DeclarationKey::fixture(name),
            keys,
            values: ValueTypeRef::Native(NativeValueType::Integer),
            lower: 0,
            upper,
        })
    };
    let text_alias = DeclarationKey::fixture("model.Name");
    let records = [
        map(
            "model.ByText",
            ValueTypeRef::Native(NativeValueType::Text),
            Some(2),
        ),
        map(
            "model.ByAlias",
            ValueTypeRef::Package(text_alias.clone()),
            None,
        ),
        DomainPackageRecord::AliasType(AliasTypeRecord {
            key: text_alias,
            target: ValueTypeRef::Native(NativeValueType::Text),
        }),
        map(
            "model.ByInt",
            ValueTypeRef::Native(NativeValueType::Integer),
            None,
        ),
        map(
            "model.ByFloat",
            ValueTypeRef::Native(NativeValueType::Float32),
            None,
        ),
    ];
    let by_key: BTreeMap<_, _> = records
        .iter()
        .map(|record| (record.key(), record))
        .collect();
    let (identities, handles, variants, units) = (
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let project = |name: &str| {
        model_value_type(
            &ValueTypeRef::Package(DeclarationKey::fixture(name)),
            mult(1, Some(1), false, true),
            &by_key,
            &identities,
            &handles,
            &variants,
            &units,
        )
    };
    let expected = |keys: ValueType, bound: Option<quire_exact::CardinalityBound>| {
        ValueType::Map(std::sync::Arc::new(
            quire_exact::MapType::new(keys, ValueType::Integer, bound).expect("a leaf key"),
        ))
    };
    assert_eq!(
        project("model.ByText").expect("a Text key assembles"),
        expected(
            ValueType::String,
            Some(quire_exact::CardinalityBound::new(0, 2).expect("ordered"))
        )
    );
    assert_eq!(
        project("model.ByAlias").expect("an alias of Text assembles"),
        expected(ValueType::String, None)
    );
    assert_eq!(
        project("model.ByInt").expect("an Integer key assembles"),
        expected(ValueType::Integer, None)
    );
    assert!(matches!(
        project("model.ByFloat"),
        Err(Unmapped::Float(IeeeWidth::Binary32))
    ));
}
