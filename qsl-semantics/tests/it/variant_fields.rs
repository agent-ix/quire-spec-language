// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-360 / TC-1006 to TC-1009: a field, parameter or result typed by a
//! variant type is admitted, assembled as the variant type's enum type,
//! checked nominally and invariantly, and compared by case.
//!
//! Evaluation and replay of the same fixture are in `qsl-eval` and
//! `qsl-replay`; every assertion here compares typed codes, causes, node
//! identities, member paths and types, never message text.

use ix_trace_rs::trace;
use qsl_forms::{build_unit, BinaryOperator, BinderQuery, Expression};
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations, VariantTypeDeclaration,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{DomainPackage, DomainPackageRecord, ValueTypeRef};
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{CollectionKind, IllTypedCause, Presence, ValueType};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use serde_json::Value;

#[path = "../support/variant_fields.rs"]
mod fixture;

use fixture::id;

fn declaration_key(node: &str) -> DeclarationKey {
    DeclarationKey {
        package: fixture::PACKAGE.to_owned(),
        node: id(node),
    }
}

/// `declarations` as a unit selecting `document` as model `alias`.
fn declarations_of(
    document: &Value,
    alias: &str,
    declarations: &str,
) -> (PackageDeclarations, Vec<DomainPackage>) {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model {alias} = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         function keep using v(): Boolean pure {{ true }}\n\
         {declarations}\n",
        package = fixture::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "variant-fields", "fixture", "fixture:1"),
        "variant-fields.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the source unit");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits and normalizes the fixture package");
    let packages = selected
        .iter()
        .map(|model| model.view.domain_package().clone())
        .collect();
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        selected,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"));
    (assembled, packages)
}

fn assemble_as(alias: &str, declarations: &str) -> PackageDeclarations {
    declarations_of(&fixture::document(), alias, declarations).0
}

fn variant<'a>(
    declarations: &'a PackageDeclarations,
    alias: &str,
    name: &str,
) -> &'a VariantTypeDeclaration {
    declarations
        .variant_types
        .iter()
        .find(|declared| declared.name() == format!("{alias}::{name}"))
        .unwrap_or_else(|| panic!("{alias}::{name} is declared"))
}

fn enum_of(declarations: &PackageDeclarations, alias: &str, name: &str) -> ValueType {
    ValueType::Enum(variant(declarations, alias, name).shape())
}

fn field_records<'a>(
    package: &'a DomainPackage,
    owner: &str,
    name: &str,
) -> Vec<&'a qsl_semantics::model::domain_package::FieldMemberRecord> {
    package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::FieldMember(field)
                if field.owner == declaration_key(owner)
                    && field.key.node == id(&format!("{owner}/{name}")) =>
            {
                Some(field)
            }
            _ => None,
        })
        .collect()
}

/// TC-1006 step 1 (FR-360-AC-1): a field of an object type, a record value
/// type, a state machine and a process typed by `Tier` is admitted with one
/// `field` export each and no `reference` export, and `Tier`'s exports equal
/// those of the same package with no member naming it.
#[trace("TC-1006", "FR-360-AC-1")]
#[test]
fn a_member_typed_by_a_variant_type_is_admitted_with_a_field_export_only() {
    let (_, packages) = declarations_of(&fixture::document(), "Config", "");
    let package = &packages[0];
    for owner in ["Account", "Settings", "Lifecycle", "Fulfilment"] {
        let found = field_records(package, owner, "tier");
        assert_eq!(found.len(), 1, "{owner}.tier is one field export");
        assert_eq!(
            found[0].value_type,
            ValueTypeRef::Package(declaration_key("Tier"))
        );
    }
    assert!(
        !package
            .records
            .iter()
            .any(|record| matches!(record, DomainPackageRecord::Relationship(_))),
        "a variant-typed field exports no reference"
    );

    let (_, bare) = declarations_of(&fixture::variants_only(), "Config", "");
    let exports = |package: &DomainPackage| -> Vec<DomainPackageRecord> {
        package
            .records
            .iter()
            .filter(|record| {
                matches!(
                    record,
                    DomainPackageRecord::Enum(_) | DomainPackageRecord::Variant(_)
                )
            })
            .cloned()
            .collect()
    };
    let named = exports(package);
    let alone = exports(&bare[0]);
    assert!(!alone.is_empty());
    assert_eq!(
        named.iter().filter(|record| alone.contains(record)).count(),
        alone.len(),
        "the enum and variant exports are unchanged by a member naming the type"
    );
}

/// The refusals I1 reports for `document`, selected as model `Config`.
fn intake_refusals(document: &Value) -> Vec<qsl_semantics::model::normalize::ModelRefusal> {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         function keep using v(): Boolean pure {{ true }}\n",
        package = fixture::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "variant-fields", "fixture", "fixture:1"),
        "variant-fields.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    let unit = build_unit(&parsed).expect("S2 builds the source unit");
    match admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(refusal) => match refusal.cause {
            qsl_semantics::model::intake::UnitIntakeCause::Refused(refusals) => {
                refusals.iter().cloned().collect()
            }
            other => panic!("a refusal of the package, not {other:?}"),
        },
    }
}

fn malformed_at(refusal: &qsl_semantics::model::normalize::ModelRefusal, node: &str, member: &str) {
    use qsl_semantics::model::normalize::ModelRefusalCause;
    assert_eq!(refusal.code, qsl_foundation::Code::InvalidModelBinding);
    let ModelRefusalCause::IntakeMalformedDeclaration {
        node: found,
        member: path,
        ..
    } = &refusal.cause
    else {
        panic!("a malformed declaration, not {refusal:?}");
    };
    assert_eq!(found, &id(node));
    assert_eq!(path.to_string(), member);
}

/// TC-1006 step 3 (FR-360-AC-1): an event whose non-occurrence field is typed
/// by `Tier` is admitted, and an event whose occurrence field is typed by
/// `Tier` refuses at the event type node, path `TypeDefinition.occurrenceField`.
#[trace("TC-1006", "FR-360-AC-1")]
#[test]
fn an_event_admits_a_variant_field_but_not_a_variant_occurrence() {
    let admitted = fixture::with_types(vec![
        fixture::instant(),
        fixture::event(
            "Shipped",
            "at",
            &id("Instant"),
            vec![fixture::field("Shipped", "tier", &id("Tier"))],
        ),
    ]);
    let found = intake_refusals(&admitted);
    assert!(found.is_empty(), "{found:?}");

    let refused = fixture::with_types(vec![fixture::event("Placed", "at", &id("Tier"), vec![])]);
    let found = intake_refusals(&refused);
    assert_eq!(found.len(), 1, "{found:?}");
    malformed_at(&found[0], "Placed", "TypeDefinition.occurrenceField");
}

/// TC-1006 step 4 (FR-360-AC-3): a variant type with no case refuses once, at
/// the variant type, path `TypeDefinition.variants`, and the field naming it
/// raises no refusal of its own.
#[trace("TC-1006", "FR-360-AC-3")]
#[test]
fn a_variant_type_with_no_case_refuses_once_and_its_field_raises_nothing() {
    let document = fixture::with_types(vec![
        fixture::variant_type("Hollow", &[]),
        fixture::event(
            "Cap",
            "at",
            "ix://quire/native/Timestamp",
            vec![fixture::field("Cap", "hollow", &id("Hollow"))],
        ),
    ]);
    let found = intake_refusals(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    malformed_at(&found[0], "Hollow", "TypeDefinition.variants");
}

/// TC-1006 step 2 (FR-360-AC-2): the assembler gives each variant-typed
/// member the variant type's enum type: an optional field `Option<_>` (the
/// declaration's own type plus its presence), a `[0, 3]` ordered field a
/// `Sequence`, and an operation parameter and result the type itself.
#[trace("TC-1006", "FR-360-AC-2")]
#[test]
fn the_assembler_types_variant_members_by_the_enum_type() {
    let declarations = assemble_as("Config", "");
    let tier = enum_of(&declarations, "Config", "Tier");
    let account = declarations
        .types
        .object_types()
        .find(|declared| declared.name() == "Config::Account")
        .expect("Account is declared");
    let field = |name: &str| {
        account
            .attributes()
            .iter()
            .find(|attribute| attribute.name() == name)
            .unwrap_or_else(|| panic!("Account.{name} is declared"))
    };
    assert_eq!(field("tier").value_type(), &tier);
    assert_eq!(field("tier").presence(), Presence::Required);
    assert_eq!(field("backup").value_type(), &tier);
    assert_eq!(field("backup").presence(), Presence::Optional);
    let ValueType::Collection(history) = field("history").value_type() else {
        panic!(
            "history is a collection: {:?}",
            field("history").value_type()
        );
    };
    assert_eq!(history.kind(), CollectionKind::Sequence);
    assert_eq!(history.element(), &tier);

    let promote = declarations
        .operations
        .declared_by(account.key())
        .iter()
        .find(|operation| operation.name() == "promote")
        .expect("promote is declared");
    assert_eq!(promote.parameters()[0].1, tier);
    assert_eq!(promote.result(), Some(&tier));
    assert!(
        !matches!(field("tier").value_type(), ValueType::Reference(_)),
        "a variant-typed field is never a Reference"
    );
}

/// TC-1006 step 2 (FR-360-AC-2): the type environment holds, for the type and
/// each case, the effective identity and the checked node key together, and a
/// unit that selects the package as another alias assigns the same keys.
#[trace("TC-1006", "FR-360-AC-2")]
#[test]
fn the_keys_of_a_variant_type_do_not_depend_on_the_alias() {
    let config = assemble_as("Config", "");
    let cfg = assemble_as("Cfg", "");
    for name in ["Tier", "Level", "Grade"] {
        let left = variant(&config, "Config", name);
        let right = variant(&cfg, "Cfg", name);
        assert_eq!(left.key(), right.key(), "{name} effective identity");
        assert_eq!(left.node(), right.node(), "{name} checked node key");
        assert_eq!(left.shape(), right.shape(), "{name} enum shape");
        let cases = |declared: &VariantTypeDeclaration| -> Vec<_> {
            declared
                .cases()
                .iter()
                .map(|case| (case.name().to_owned(), case.key(), case.variant()))
                .collect()
        };
        assert_eq!(cases(left), cases(right), "{name} cases");
    }
    assert_ne!(
        variant(&config, "Config", "Tier").node(),
        variant(&config, "Config", "Level").node()
    );
    let tier = variant(&config, "Config", "Tier");
    let level = variant(&config, "Config", "Level");
    assert_ne!(
        tier.cases()[0].variant(),
        level.cases()[0].variant(),
        "two variant types sharing a case name keep distinct case keys"
    );
}

/// The checked graph of `declarations` over the FR-360 package, and the
/// types a check below needs.
struct Checked {
    graph: CheckedGraph,
    tier: ValueType,
    level: ValueType,
    settings: ValueType,
}

fn checked() -> Checked {
    let declarations = assemble_as("Config", "");
    let tier = enum_of(&declarations, "Config", "Tier");
    let level = enum_of(&declarations, "Config", "Level");
    let settings = declarations
        .types
        .composites()
        .find(|declared| declared.name() == "Config::Settings")
        .map(|declared| ValueType::Composite(declared.key()))
        .expect("Settings is declared");
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    Checked {
        graph,
        tier,
        level,
        settings,
    }
}

impl Checked {
    fn check(
        &self,
        parameters: &[(&str, &ValueType)],
        expression: &Expression,
        expected: Option<&ValueType>,
    ) -> Result<ValueType, CheckRefusal> {
        self.graph
            .check_expression(
                parameters
                    .iter()
                    .map(|(name, value_type)| ((*name).to_owned(), (*value_type).clone()))
                    .collect(),
                expression,
                expected,
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .map(|checked| checked.value_type().clone())
    }
}

fn name(spelling: &str) -> Expression {
    Expression::name(spelling.to_owned())
}

fn mismatch() -> CheckCause {
    CheckCause::IllTyped(IllTypedCause::TypeMismatch)
}

/// TC-1007 step 1 (FR-360-AC-4): the case literal and every variant-producing
/// form check as `Config::Tier`.
#[trace("TC-1007", "FR-360-AC-4")]
#[test]
fn a_case_literal_and_every_variant_producing_form_check_as_the_variant_type() {
    let c = checked();
    let tier = &c.tier;
    let option = ValueType::option(tier.clone());
    let boolean = ValueType::Boolean;
    let collection = ValueType::collection(quire_exact::CollectionType::new(
        CollectionKind::Sequence,
        tier.clone(),
        None,
    ));
    let gold = || name("Config::Tier::Gold");
    type Form<'a> = (&'a str, Vec<(&'a str, &'a ValueType)>, Expression);
    let forms: Vec<Form<'_>> = vec![
        ("the literal", vec![], gold()),
        ("a parameter", vec![("t", tier)], name("t")),
        (
            "a field read",
            vec![("s", &c.settings)],
            Expression::field(name("s"), "tier"),
        ),
        (
            "a let binding",
            vec![],
            Expression::let_in("x", gold(), name("x")),
        ),
        (
            "an Option payload",
            vec![("o", &option)],
            Expression::if_then_else(
                Expression::present(name("o")),
                Expression::value(name("o")),
                gold(),
            ),
        ),
        (
            "a conditional",
            vec![("b", &boolean), ("s", &c.settings)],
            Expression::if_then_else(name("b"), gold(), Expression::field(name("s"), "tier")),
        ),
    ];
    for (label, parameters, expression) in forms {
        let found = c
            .check(&parameters, &expression, Some(tier))
            .unwrap_or_else(|refusal| panic!("{label} checks at a Tier slot: {refusal:?}"));
        assert_eq!(&found, tier, "{label} has the type Config::Tier");
    }
    // An element of a collection is a Tier: the binder compares it to a case.
    let element = Expression::query(
        BinderQuery::Exists,
        "x",
        name("c"),
        Expression::binary(BinaryOperator::Equal, name("x"), gold()),
    );
    assert_eq!(
        c.check(&[("c", &collection)], &element, None).unwrap(),
        ValueType::Boolean
    );
}

/// TC-1007 steps 2 to 7 (FR-360-AC-5): each mistyped use refuses
/// `ill_typed`/`type-mismatch`, and a name that resolves to no variant type
/// refuses `missing-name`.
#[trace("TC-1007", "FR-360-AC-5")]
#[test]
fn each_mistyped_use_of_a_variant_type_refuses_with_its_cause() {
    let c = checked();
    let tier = &c.tier;
    let level = &c.level;
    let sequence = |element: &ValueType| {
        ValueType::collection(quire_exact::CollectionType::new(
            CollectionKind::Sequence,
            element.clone(),
            None,
        ))
    };
    let option_tier = ValueType::option(tier.clone());
    let tiers = sequence(tier);
    let levels = sequence(level);
    let refuses = |parameters: &[(&str, &ValueType)],
                   expression: Expression,
                   expected: Option<&ValueType>,
                   cause: CheckCause,
                   label: &str| {
        let refusal = c.check(parameters, &expression, expected).expect_err(label);
        assert_eq!(refusal.cause, cause, "{label}: {refusal:?}");
    };
    refuses(
        &[],
        name("Config::Level::Gold"),
        Some(tier),
        mismatch(),
        "a case of another variant type at a Tier slot",
    );
    refuses(
        &[],
        Expression::integer(quire_exact::Integer::from(0_i64)),
        Some(tier),
        mismatch(),
        "an Integer literal at a Tier slot",
    );
    refuses(
        &[],
        name("Gold"),
        Some(tier),
        mismatch(),
        "a bare case name at a Tier slot",
    );
    refuses(
        &[],
        name("Standard"),
        Some(&ValueType::Integer),
        CheckCause::MissingName("Standard".to_owned()),
        "an unbound name that is a case name where no variant is expected",
    );
    refuses(
        &[("t", tier)],
        name("t"),
        Some(level),
        mismatch(),
        "a Tier at a Level slot",
    );
    refuses(
        &[("c", &tiers)],
        name("c"),
        Some(&levels),
        mismatch(),
        "a Sequence<Tier> at a Sequence<Level> slot",
    );
    refuses(
        &[("c", &tiers)],
        name("c"),
        Some(&option_tier),
        mismatch(),
        "a Sequence<Tier> at an Option<Tier> slot",
    );
    refuses(
        &[("o", &option_tier)],
        name("o"),
        Some(tier),
        mismatch(),
        "an Option<Tier> at a Tier slot",
    );
    refuses(
        &[],
        name("Config::Tier::Silver"),
        None,
        mismatch(),
        "an undeclared case",
    );
    refuses(
        &[],
        name("Config::Tier::gold"),
        None,
        mismatch(),
        "a letter-case variant of a case",
    );
    refuses(
        &[],
        name("Config::Nope::Gold"),
        None,
        CheckCause::MissingName("Config::Nope::Gold".to_owned()),
        "a case of no variant type",
    );
    for operator in [
        BinaryOperator::Less,
        BinaryOperator::LessOrEqual,
        BinaryOperator::Greater,
        BinaryOperator::GreaterOrEqual,
    ] {
        refuses(
            &[],
            Expression::binary(
                operator,
                name("Config::Tier::Gold"),
                name("Config::Tier::Standard"),
            ),
            None,
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
            "an ordering over a variant type",
        );
    }
}

/// TC-1007 step 6 (FR-360-AC-5): a variant type name in a type position that
/// resolves to no variant type refuses at the assembler as `missing-name`.
#[trace("TC-1007", "FR-360-AC-5")]
#[test]
fn an_unknown_variant_type_name_in_a_type_position_refuses_at_the_assembler() {
    let (_, _) = declarations_of(&fixture::document(), "Config", "");
    let source_error = |declarations: &str| {
        let document = fixture::document();
        let bytes = serde_json::to_vec(&document).unwrap();
        let packages = package_input([bytes.as_slice()]);
        let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
            panic!("one model package");
        };
        let digest = qsl_semantics::model::key::hex(digest);
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n\
             model Config = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
             {declarations}\n",
            package = fixture::PACKAGE,
        );
        let parsed = qsl_cst::parse(
            SourceIdentity::new("test", "variant-fields", "fixture", "fixture:1"),
            "variant-fields.native",
            source.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .unwrap();
        let unit = build_unit(&parsed).unwrap();
        let selected = admit_unit(
            &unit.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .unwrap();
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            selected,
            Vec::new(),
        )
        .expect_err("the unit refuses at the assembler")
    };
    let refusal = source_error("function f using v(t: Config::Nope): Boolean pure { true }");
    assert_eq!(refusal.errors.len(), 1, "{refusal:?}");
    assert_eq!(
        refusal.errors[0].cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
}

// ---------------------------------------------------------------------------
// TC-1010: replay admission of a variant value
// ---------------------------------------------------------------------------

use qsl_semantics::model::observation::{
    admit_current_snapshot, population_universe_for, AdmissionFailure, DocumentRef,
    ObservationLimits, PathStep,
};
use quire_exact::{EnumMember, FieldValue, Value as Datum};
use quire_semantic_value::declaration::FieldRef;
use serde_json::json;
use std::collections::BTreeMap;

fn document_ref(label: &str, bytes: &[u8]) -> DocumentRef {
    let document = quire_canonical::read(bytes, u64::MAX).expect("the document is JSON");
    let digest = *quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
        .expect("the document is canonical")
        .as_bytes();
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "1".to_owned(),
        digest,
    }
}

/// A current snapshot over the FR-360 package, admitted against the checked
/// graph of a unit that names no variant type.
struct Replay {
    declarations: PackageDeclarations,
    graph: CheckedGraph,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    model: serde_json::Value,
}

fn replay() -> Replay {
    replay_over(fixture::document())
}

fn replay_over(document: serde_json::Value) -> Replay {
    let bytes = serde_json::to_vec(&document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let model = json!({
        "identity": fixture::PACKAGE, "version": "1",
        "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(digest)),
    });
    let declarations = declarations_of(&document, "Config", "").0;
    let graph = declarations_of(&document, "Config", "")
        .0
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    Replay {
        declarations,
        graph,
        packages,
        model,
    }
}

impl Replay {
    /// The type identity string of the variant type `name`.
    fn type_id(name: &str) -> String {
        id(name)
    }

    /// `account_fields` as the fields of the one `Account` object `a1`, in a
    /// current snapshot admitted against the checked graph.
    fn admit(
        &self,
        account_fields: serde_json::Value,
    ) -> Result<quire_semantic_value::object_closure::ObjectClosure, AdmissionFailure> {
        let mut fields = json!({
            "account_id": {"integer": "1"},
            "tier": variant_json("Tier", "Gold"),
            "backup": {"absent": {}},
            "history": {"sequence": []},
        });
        fields
            .as_object_mut()
            .unwrap()
            .extend(account_fields.as_object().unwrap().clone());
        let bytes = serde_json::to_vec(&json!({
            "format": "quire.state.snapshot/v1",
            "identity": {
                "authority": "test", "identity": "current",
                "revision_namespace": "fixture", "revision": "1",
            },
            "observation": "current",
            "model": self.model,
            "populations": [{
                "population": id("accounts"), "complete": true,
                "objects": [{"key": "a1", "type": id("Account"), "fields": fields}],
            }],
        }))
        .expect("the snapshot serializes");
        let reference = document_ref("current", &bytes);
        let provision = BTreeMap::from([(reference.digest, bytes)]);
        admit_current_snapshot(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &self.packages,
            ModelNormalizationLimits::default(),
            &provision,
            &reference,
            ObservationLimits::default(),
        )
        .map(|(environment, _)| environment.objects().clone())
    }

    /// The `tier` value `Account.a1` was admitted with.
    fn slot(
        &self,
        closure: &quire_semantic_value::object_closure::ObjectClosure,
        field: &str,
    ) -> FieldValue {
        let account = self
            .declarations
            .types
            .object_types()
            .find(|declared| declared.name() == "Config::Account")
            .expect("Account is declared")
            .key();
        let universe = population_universe_for(
            self.graph.model_selections(),
            &self.packages,
            ModelNormalizationLimits::default(),
            account,
        )
        .expect("the Account universe");
        let reference = closure.find(universe, "a1").expect("the admitted object");
        closure
            .attribute(
                self.graph.scope().types(),
                reference,
                &FieldRef::new(account, field),
            )
            .cloned()
            .expect("the field slot")
    }

    /// The value the case `name` of the variant type `type_name` has.
    fn case(&self, type_name: &str, name: &str) -> Datum {
        let declared = self
            .graph
            .scope()
            .variant_types()
            .iter()
            .find(|declared| declared.name() == format!("Config::{type_name}"))
            .expect("the variant type is declared");
        let rank = declared
            .cases()
            .iter()
            .position(|case| case.name() == name)
            .expect("the case is declared");
        Datum::Enum(EnumMember::new(
            declared.cases()[rank].variant(),
            u32::try_from(rank).unwrap(),
        ))
    }
}

/// Whether two values are one value by the kernel's canonical key order.
fn same_value(left: &Datum, right: &Datum) -> bool {
    quire_exact::compare_keys(left, right) == Some(std::cmp::Ordering::Equal)
}

fn variant_json(type_node: &str, case_name: &str) -> serde_json::Value {
    json!({"variant": {"type": Replay::type_id(type_node), "case": case_name}})
}

/// TC-1010 step 1 (FR-360-AC-9): a snapshot whose `Account.tier` holds the
/// variant form admits, so does the same value as the payload of `present` and
/// as an element of a `sequence`, and the admitted value equals the case the
/// literal `Config::Tier::Gold` has.
#[trace("TC-1010", "FR-360-AC-9")]
#[test]
fn a_variant_form_is_admitted_at_a_field_a_payload_and_an_element() {
    let replay = replay();
    let gold = variant_json("Tier", "Gold");
    let closure = replay
        .admit(json!({
            "tier": gold,
            "backup": {"present": variant_json("Tier", "Standard")},
            "history": {"sequence": [gold, variant_json("Tier", "Standard")]},
        }))
        .unwrap_or_else(|failure| panic!("the snapshot admits: {failure:?}"));
    let FieldValue::Present(tier) = replay.slot(&closure, "tier") else {
        panic!("the required field is present");
    };
    assert!(same_value(&tier, &replay.case("Tier", "Gold")));
    let FieldValue::Present(backup) = replay.slot(&closure, "backup") else {
        panic!("the optional field is present");
    };
    assert!(same_value(&backup, &replay.case("Tier", "Standard")));
    let FieldValue::Present(Datum::Collection(history)) = replay.slot(&closure, "history") else {
        panic!("a sequence field holds a collection");
    };
    let elements = history.elements();
    assert_eq!(elements.len(), 2);
    assert!(same_value(&elements[0], &replay.case("Tier", "Gold")));
    assert!(same_value(&elements[1], &replay.case("Tier", "Standard")));
}

/// TC-1010 step 1 (FR-360-AC-9, FR-360-AC-11): the observation form names a
/// case that is not a source identifier by its exact bytes. The value admits,
/// equals the case the type declares, and encodes as the same form again; a
/// letter-case variant of the name and a case of another variant type refuse
/// at the nested value.
#[trace("TC-1010", "FR-360-AC-9", "FR-360-AC-10", "FR-360-AC-11")]
#[test]
fn a_variant_form_names_a_non_identifier_case_by_its_exact_bytes() {
    let replay = replay_over(fixture::with_odd_grade(&ODD_CASES));
    let encoder = ValueEncoder::over(
        replay.graph.model_selections(),
        replay.graph.scope().types(),
        &replay.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the encoder reads the selected package");
    for case in ODD_CASES {
        let form = variant_json("Odd", case);
        let closure = replay
            .admit(json!({"grade": {"present": form}}))
            .unwrap_or_else(|failure| panic!("{case} admits: {failure:?}"));
        let FieldValue::Present(grade) = replay.slot(&closure, "grade") else {
            panic!("{case}: the optional field is present");
        };
        assert!(same_value(&grade, &replay.case("Odd", case)), "{case}");
        assert_eq!(
            encoder
                .encode(&grade)
                .expect("a variant value encodes")
                .to_json(),
            form,
            "{case}"
        );
    }
    let refused = |grade: serde_json::Value| match replay.admit(json!({"grade": grade})) {
        Err(AdmissionFailure::Refused(record)) => (record.code, record.cause),
        other => panic!("a refusal, not {other:?}"),
    };
    assert_eq!(
        refused(json!({"present": variant_json("Odd", "gold plus")})),
        (qsl_foundation::Code::InvalidRuntimeInput, "invalid-value"),
        "a letter-case variant of a case"
    );
    assert_eq!(
        refused(json!({"present": variant_json("Tier", "Gold")})),
        (
            qsl_foundation::Code::InvalidRuntimeInput,
            "wrong-value-kind"
        ),
        "a case of another variant type"
    );
}

/// The refusal `fields` is admitted with, as its code, cause and nested path.
fn refusal_of(
    replay: &Replay,
    tier: serde_json::Value,
) -> (
    qsl_foundation::Code,
    &'static str,
    Vec<PathStep>,
    Option<String>,
) {
    match replay.admit(json!({"tier": tier})) {
        Err(AdmissionFailure::Refused(record)) => (
            record.code,
            record.cause,
            record.path,
            record.fields.get("member").cloned(),
        ),
        other => panic!("a refusal, not {other:?}"),
    }
}

/// TC-1010 steps 2 and 3 (FR-360-AC-10): each malformed form refuses
/// `invalid_runtime_input` with its cause, and a value with several defects
/// reports the first in FR-360 item 9's order.
#[trace("TC-1010", "FR-360-AC-10")]
#[test]
fn each_malformed_variant_form_refuses_with_its_cause_in_item_9_order() {
    use qsl_foundation::Code;
    let replay = replay();
    let tier = Replay::type_id("Tier");
    let level = Replay::type_id("Level");
    let invalid = Code::InvalidRuntimeInput;
    for (label, value, cause, expected_member) in [
        (
            "a missing case",
            json!({"variant": {"type": tier}}),
            "missing-member",
            Some("case"),
        ),
        (
            "no member at all reports type first",
            json!({"variant": {}}),
            "missing-member",
            Some("type"),
        ),
        (
            "a missing type",
            json!({"variant": {"case": "Gold"}}),
            "missing-member",
            Some("type"),
        ),
        (
            "an extra member",
            json!({"variant": {"type": tier, "case": "Gold", "x": 1}}),
            "unknown-member",
            None,
        ),
        (
            "a type naming another variant type",
            json!({"variant": {"type": level, "case": "Gold"}}),
            "wrong-value-kind",
            None,
        ),
        (
            "a type naming a record",
            json!({"variant": {"type": id("Settings"), "case": "Gold"}}),
            "wrong-value-kind",
            None,
        ),
        (
            "a type naming an object",
            json!({"variant": {"type": id("Account"), "case": "Gold"}}),
            "wrong-value-kind",
            None,
        ),
        (
            "a type naming no declaration",
            json!({"variant": {"type": id("Nope"), "case": "Gold"}}),
            "wrong-value-kind",
            None,
        ),
        (
            "a Text value at the slot",
            json!({"string": "Gold"}),
            "wrong-value-kind",
            None,
        ),
        (
            "an Integer value at the slot",
            json!({"integer": "0"}),
            "wrong-value-kind",
            None,
        ),
        (
            "a payload that is not an object",
            json!({"variant": "Gold"}),
            "wrong-value-kind",
            None,
        ),
        (
            "an absent value at a required field",
            json!({"absent": {}}),
            "wrong-value-kind",
            None,
        ),
        (
            "an undeclared case",
            json!({"variant": {"type": tier, "case": "Silver"}}),
            "invalid-value",
            None,
        ),
        (
            "a letter-case variant of a case",
            json!({"variant": {"type": tier, "case": "gold"}}),
            "invalid-value",
            None,
        ),
        (
            "a missing case beats a wrong type",
            json!({"variant": {"type": level}}),
            "missing-member",
            Some("case"),
        ),
        (
            "an extra member beats an undeclared case",
            json!({"variant": {"type": tier, "case": "Silver", "x": 1}}),
            "unknown-member",
            None,
        ),
        (
            "a wrong type beats an undeclared case",
            json!({"variant": {"type": level, "case": "Silver"}}),
            "wrong-value-kind",
            None,
        ),
    ] {
        let (code, found, path, member) = refusal_of(&replay, value);
        assert_eq!((code, found), (invalid, cause), "{label}");
        assert_eq!(member.as_deref(), expected_member, "{label}");
        assert!(path.is_empty(), "{label}: the value is the field itself");
    }
}

/// TC-1010 step 3 (FR-360-AC-10): a defect inside a nested value is refused at
/// the nested path: the `present` payload and the sequence element.
#[trace("TC-1010", "FR-360-AC-10")]
#[test]
fn a_defect_in_a_nested_variant_value_is_refused_at_its_path() {
    let replay = replay();
    let outcome = replay.admit(json!({
        "tier": variant_json("Tier", "Gold"),
        "backup": {"present": variant_json("Tier", "Silver")},
    }));
    let Err(AdmissionFailure::Refused(record)) = outcome else {
        panic!("a refusal, not {outcome:?}");
    };
    assert_eq!(record.cause, "invalid-value");
    assert_eq!(record.path, [PathStep::OptionValue]);

    let outcome = replay.admit(json!({
        "tier": variant_json("Tier", "Gold"),
        "history": {"sequence": [variant_json("Tier", "Gold"), variant_json("Level", "Gold")]},
    }));
    let Err(AdmissionFailure::Refused(record)) = outcome else {
        panic!("a refusal, not {outcome:?}");
    };
    assert_eq!(record.cause, "wrong-value-kind");
    assert_eq!(record.path, [PathStep::Index(1)]);
}

// ---------------------------------------------------------------------------
// TC-1009: a function and an operation signature typed by a variant type
// ---------------------------------------------------------------------------

/// The refusals `declarations` is checked with.
fn check_refusals(declarations: &str) -> Vec<CheckRefusal> {
    assemble_as("Config", declarations)
        .check(CheckingLimits::default())
        .map(|_| Vec::new())
        .unwrap_or_else(|refusals| refusals)
}

/// Whether some refusal is `cause` in the body of `function` at child-index
/// `path` from its root expression.
fn refused_in_body(
    refusals: &[CheckRefusal],
    function: &str,
    path: &[usize],
    cause: &CheckCause,
) -> bool {
    refusals.iter().any(|refusal| {
        &refusal.cause == cause
            && matches!(&refusal.location.origin,
                quire_semantic_value::location::Origin::Body { function: found, .. }
                    if found == function)
            && refusal.location.path() == path
    })
}

/// TC-1009 step 2 (FR-360-AC-8): a call that passes a `Config::Level` value
/// at a `Config::Tier` parameter refuses `ill_typed`/`type-mismatch` at the
/// argument, and one that passes a `Config::Tier` does not.
#[trace("TC-1009", "FR-360-AC-8")]
#[test]
fn a_call_with_a_value_of_another_variant_type_refuses_at_the_argument() {
    let source = "function needs using v(t: Config::Tier): Boolean pure { true }\n\
                  function good using v(t: Config::Tier): Boolean pure { needs(t) }\n\
                  function bad using v(l: Config::Level): Boolean pure { needs(l) }";
    let refusals = check_refusals(source);
    assert!(
        refused_in_body(&refusals, "bad", &[0], &mismatch()),
        "bad refuses at its argument: {refusals:?}"
    );
    assert!(
        !refusals.iter().any(|refusal| matches!(
            &refusal.location.origin,
            quire_semantic_value::location::Origin::Body { function, .. } if function == "good"
        )),
        "good takes a Tier: {refusals:?}"
    );
}

/// TC-1009 step 3 and FR-360 item 12 (FR-360-AC-8): a unit whose function
/// takes, returns or passes a variant type checks and lowers, with no
/// refusal at the member's, parameter's or result's type node.
#[trace("TC-1009", "FR-360-AC-8")]
#[test]
fn a_variant_typed_parameter_and_result_check_and_lower() {
    let refusals = check_refusals(
        "function pick using v(t: Config::Tier): Config::Tier pure { t }\n\
         function isGold using v(t: Config::Tier): Boolean pure { t = Config::Tier::Gold }\n\
         function tierOf using v(s: Config::Settings): Config::Tier pure { s.tier }",
    );
    assert!(refusals.is_empty(), "{refusals:?}");
}

// ---------------------------------------------------------------------------
// TC-1010: an operation parameter and result holding a variant value
// ---------------------------------------------------------------------------

use qsl_semantics::model::observation::{
    admit_observations, AdmittedObservations, ClauseFacts, ClauseSelection, ClauseSelectionInput,
    OperationFacts, Provisions,
};

fn identity_json(reference: &DocumentRef) -> serde_json::Value {
    json!({
        "authority": reference.authority, "identity": reference.identity,
        "revision_namespace": reference.revision_namespace, "revision": reference.revision,
    })
}

impl Replay {
    /// The facts of a clause over `Account.promote(t: Tier): Tier`.
    fn promote_facts(&self, kind: qsl_forms::StateClauseKind) -> ClauseFacts {
        let account = self
            .declarations
            .types
            .object_types()
            .find(|declared| declared.name() == "Config::Account")
            .expect("Account is declared")
            .key();
        let declaration = self
            .declarations
            .operations
            .declared_by(account)
            .iter()
            .find(|operation| operation.name() == "promote")
            .expect("promote is declared")
            .clone();
        ClauseFacts {
            identity: quire_exact::NodeKey::from_digest([7; 32]),
            kind,
            context: account,
            operation: Some(OperationFacts {
                declaring: account,
                declaration,
            }),
            operations: Default::default(),
        }
    }

    fn snapshot(&self, label: &str, observation: &str) -> (DocumentRef, Vec<u8>) {
        let reference = DocumentRef {
            authority: "test".to_owned(),
            identity: label.to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "1".to_owned(),
            digest: [0; 32],
        };
        let bytes = serde_json::to_vec(&json!({
            "format": "quire.state.snapshot/v1",
            "identity": identity_json(&reference),
            "observation": observation,
            "model": self.model,
            "populations": [{
                "population": id("accounts"), "complete": true,
                "objects": [{"key": "a1", "type": id("Account"), "fields": {
                    "account_id": {"integer": "1"},
                    "tier": variant_json("Tier", "Gold"),
                    "backup": {"absent": {}},
                    "history": {"sequence": []},
                }}],
            }],
        }))
        .expect("the snapshot serializes");
        (document_ref(label, &bytes), bytes)
    }

    /// `promote` admitted over an invocation whose parameter `t` and result
    /// are the given values.
    fn invoke(
        &self,
        parameter: serde_json::Value,
        result: serde_json::Value,
    ) -> Result<AdmittedObservations, AdmissionFailure> {
        let (pre, pre_bytes) = self.snapshot("pre", "pre");
        let (post, post_bytes) = self.snapshot("post", "post");
        let invocation_identity = DocumentRef {
            authority: "test".to_owned(),
            identity: "call".to_owned(),
            revision_namespace: "fixture".to_owned(),
            revision: "1".to_owned(),
            digest: [0; 32],
        };
        let reference = |document: &DocumentRef| {
            json!({
                "identity": identity_json(document),
                "digest": format!(
                    "sha256-jcs:{}",
                    qsl_semantics::model::key::hex(&document.digest)
                ),
            })
        };
        let invocation_bytes = serde_json::to_vec(&json!({
            "format": "quire.state.invocation/v1",
            "identity": identity_json(&invocation_identity),
            "model": self.model,
            "context": id("Account"),
            "operation": "promote",
            "self": {"population": id("accounts"), "key": "a1"},
            "pre": reference(&pre),
            "post": reference(&post),
            "parameters": {"t": parameter},
            "result": result,
            "created": [],
            "deleted": [],
        }))
        .expect("the invocation serializes");
        let invocation = document_ref("call", &invocation_bytes);
        let snapshots = BTreeMap::from([(pre.digest, pre_bytes), (post.digest, post_bytes)]);
        let invocations = BTreeMap::from([(invocation.digest, invocation_bytes)]);
        admit_observations(
            self.graph.model_selections(),
            self.graph.scope().types(),
            &self.promote_facts(qsl_forms::StateClauseKind::Postcondition),
            &self.packages,
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &snapshots,
                invocations: &invocations,
            },
            &ClauseSelection {
                name: "Promoted".to_owned(),
                input: ClauseSelectionInput::Invocation { invocation },
            },
            ObservationLimits::default(),
        )
    }
}

/// TC-1010 step 1 (FR-360-AC-9): an operation parameter and result holding
/// the variant form are admitted and equal the case the literal has.
#[trace("TC-1010", "FR-360-AC-9")]
#[test]
fn a_variant_form_is_admitted_as_an_operation_parameter_and_result() {
    let replay = replay();
    let admitted = replay
        .invoke(
            variant_json("Tier", "Gold"),
            variant_json("Tier", "Standard"),
        )
        .unwrap_or_else(|failure| panic!("the invocation admits: {failure:?}"));
    let [(name, parameter)] = &admitted.parameters[..] else {
        panic!("one parameter");
    };
    assert_eq!(name, "t");
    assert!(same_value(parameter, &replay.case("Tier", "Gold")));
    let result = admitted.result.as_ref().expect("a result");
    assert!(same_value(result, &replay.case("Tier", "Standard")));
}

/// TC-1010 step 3 (FR-360-AC-10): a malformed variant form at the parameter
/// or the result refuses `invalid_runtime_input`.
#[trace("TC-1010", "FR-360-AC-10")]
#[test]
fn a_malformed_variant_form_as_a_parameter_or_result_refuses() {
    let replay = replay();
    let gold = variant_json("Tier", "Gold");
    for (parameter, result, cause) in [
        (
            json!({"variant": {"type": id("Level"), "case": "Gold"}}),
            gold.clone(),
            "wrong-value-kind",
        ),
        (
            gold.clone(),
            json!({"variant": {"type": id("Tier"), "case": "Silver"}}),
            "invalid-value",
        ),
    ] {
        let Err(AdmissionFailure::Refused(record)) = replay.invoke(parameter, result) else {
            panic!("a refusal expected for {cause}");
        };
        assert_eq!(record.code, qsl_foundation::Code::InvalidRuntimeInput);
        assert_eq!(record.cause, cause);
    }
}

/// A unit selecting the FR-360 package as `Config` whose function `f` has the
/// body `body`, parsed by S1.
fn parsed_with_body(body: &str) -> (String, qsl_cst::ParsedSource) {
    let bytes = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         function f using v(t: Config::Tier): Boolean pure {{ {body} }}\n",
        package = fixture::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "variant-fields", "fixture", "fixture:1"),
        "variant-fields.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source");
    (source, parsed)
}

/// FR-360-AC-5: a Text literal and `none` at a variant slot are admitted by S1
/// and refused by S2 as `unsupported_construct`, cause `UnrepresentedConstruct`
/// of the `Primary` production, at the literal's span: `qsl-forms` has no
/// expression form for either, so no checker row exists for them.
#[trace("TC-1007", "FR-360-AC-5")]
#[test]
fn a_text_literal_and_none_are_refused_by_s2_as_unrepresented() {
    for literal in ["\"Gold\"", "none"] {
        let (source, parsed) = parsed_with_body(&format!("t = {literal}"));
        assert!(
            parsed.is_admissible(),
            "{literal}: {:?}",
            parsed.diagnostics()
        );
        let refusal = build_unit(&parsed).expect_err("S2 refuses the literal");
        assert!(
            matches!(
                refusal.cause,
                qsl_forms::FormsCause::UnrepresentedConstruct {
                    production: qsl_cst::Production::Primary
                }
            ),
            "{literal}: {refusal:?}"
        );
        assert_eq!(
            refusal.catalog_code(),
            qsl_foundation::Code::UnsupportedConstruct
        );
        let start = source.find(&format!("t = {literal}")).expect("in source") + "t = ".len();
        let span = refusal.span.expect("the refusal names the literal");
        assert_eq!(
            (span.start, span.end),
            (start, start + literal.len()),
            "{literal}"
        );
    }
}

/// FR-360-AC-5: the `case <ident> when <block> <control>` control form is not
/// an expression. In expression position `case` opens FR-313's arm list
/// (`case <scrutinee> { ... }`), so S1 refuses `when` where the `{` belongs as
/// `invalid_syntax`, cause `UnexpectedToken`, and no scrutinee check exists at
/// this ref.
#[trace("TC-1007", "FR-360-AC-5")]
#[test]
fn a_case_in_expression_position_is_refused_by_s1_as_unexpected() {
    let (source, parsed) = parsed_with_body("case t when { true } true");
    assert!(!parsed.is_admissible());
    let [diagnostic] = parsed.diagnostics() else {
        panic!("one diagnostic: {:?}", parsed.diagnostics());
    };
    assert_eq!(diagnostic.code, qsl_foundation::Code::InvalidSyntax);
    assert_eq!(diagnostic.cause, qsl_cst::CompleteCause::UnexpectedToken);
    let at = source.find("when").expect("in source");
    let region = diagnostic.region.as_ref().expect("a region");
    let at = u64::try_from(at).unwrap();
    assert_eq!((region.start(), region.end()), (at, at + 4));
}

/// FR-360-AC-5 (item 5): since FR-313 a well-formed `case` over a variant
/// parses and reaches the checker, and since FR-318 the checker refuses it
/// `ill_typed`/`type-mismatch` at the scrutinee: `case` ranges over a declared
/// union only, and a variant type is an enum type, not a union.
#[trace("TC-1007", "FR-360-AC-5", "FR-318-AC-4")]
#[test]
fn a_well_formed_case_over_a_variant_is_refused_at_its_scrutinee() {
    let refusals = check_refusals(
        "function f using v(t: Config::Tier): Boolean pure { case t { Gold: true; } }",
    );
    assert!(
        refused_in_body(&refusals, "f", &[0], &mismatch()),
        "{refusals:?}"
    );
}

// ---------------------------------------------------------------------------
// FR-360 item 10: the encoder
// ---------------------------------------------------------------------------

use qsl_semantics::model::observation::ValueEncoder;

/// FR-360-AC-11 (TC-1010): the variant values an admitted snapshot holds
/// encode as the item 9 form, naming their own variant type and case; a
/// document whose members are written in the other order has the same
/// digest, and the encoded value decodes to the same value again.
#[trace("TC-1010", "FR-360-AC-10", "FR-360-AC-11")]
#[test]
fn an_admitted_variant_value_encodes_as_its_form_and_decodes_to_itself() {
    let replay = replay();
    let encoder = ValueEncoder::over(
        replay.graph.model_selections(),
        replay.graph.scope().types(),
        &replay.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the encoder reads the selected package");
    let standard = json!({"variant": {"case": "Standard", "type": id("Tier")}});
    let closure = replay
        .admit(json!({
            "tier": variant_json("Tier", "Gold"),
            "backup": {"present": standard},
            "history": {"sequence": [variant_json("Tier", "Gold"), variant_json("Tier", "Standard")]},
        }))
        .expect("the snapshot admits");

    let FieldValue::Present(tier) = replay.slot(&closure, "tier") else {
        panic!("present");
    };
    let encoded = encoder.encode(&tier).expect("a variant value encodes");
    assert_eq!(encoded.to_json(), variant_json("Tier", "Gold"));

    let FieldValue::Present(backup) = replay.slot(&closure, "backup") else {
        panic!("present");
    };
    let tier_shape = replay
        .graph
        .scope()
        .variant_types()
        .iter()
        .find(|declared| declared.name() == "Config::Tier")
        .map(VariantTypeDeclaration::shape)
        .expect("Tier is declared");
    let present = encoder
        .encode(&quire_exact::OptionValue::from_admitted(
            ValueType::Enum(tier_shape),
            Some(backup.clone()),
        ))
        .expect("an Option of a variant encodes");
    assert_eq!(
        present.to_json(),
        json!({"present": variant_json("Tier", "Standard")})
    );

    let FieldValue::Present(history) = replay.slot(&closure, "history") else {
        panic!("present");
    };
    let written_history = encoder
        .encode(&history)
        .expect("a sequence encodes")
        .to_json();
    assert_eq!(
        written_history,
        json!({"sequence": [variant_json("Tier", "Gold"), variant_json("Tier", "Standard")]})
    );
    // Decoding what was written gives the same value again.
    let again = replay
        .admit(json!({"history": written_history}))
        .expect("the written form admits");
    let FieldValue::Present(Datum::Collection(again)) = replay.slot(&again, "history") else {
        panic!("a sequence");
    };
    let FieldValue::Present(Datum::Collection(first)) = replay.slot(&closure, "history") else {
        panic!("a sequence");
    };
    assert_eq!(again.elements().len(), first.elements().len());
    for (left, right) in again.elements().iter().zip(first.elements()) {
        assert!(same_value(left, right));
    }

    // Member order of the written document changes neither the typed value nor
    // the digest: the encoder's output and the input spelled with its members
    // in the other order are one document.
    let tier = id("Tier");
    let input_order = format!(
        r#"{{"sequence":[{{"variant":{{"type":"{tier}","case":"Gold"}}}},{{"variant":{{"type":"{tier}","case":"Standard"}}}}]}}"#
    )
    .into_bytes();
    let encoded_bytes = serde_json::to_vec(&written_history).unwrap();
    assert_ne!(input_order, encoded_bytes, "the spellings differ");
    assert_eq!(
        document_ref("a", &input_order).digest,
        document_ref("a", &encoded_bytes).digest
    );

    // A record holding a variant encodes as its form, fields by name.
    let settings_type = replay
        .declarations
        .types
        .composites()
        .find(|declared| declared.name() == "Config::Settings")
        .expect("Settings is declared")
        .key();
    let settings = replay
        .graph
        .scope()
        .types()
        .record(
            settings_type,
            vec![
                ("tier", FieldValue::Present(replay.case("Tier", "Standard"))),
                (
                    "size",
                    FieldValue::Present(Datum::Integer(quire_exact::Integer::from(3_i64))),
                ),
            ],
        )
        .expect("Settings constructs");
    assert_eq!(
        encoder
            .encode(&settings)
            .expect("a record encodes")
            .to_json(),
        json!({"record": {"type": id("Settings"), "fields": {
            "size": {"integer": "3"},
            "tier": variant_json("Tier", "Standard"),
        }}})
    );

    // A value of no variant type does not encode.
    assert!(encoder
        .encode(&Datum::Enum(EnumMember::new(
            quire_exact::VariantId::from_digest([9; 32]),
            0
        )))
        .is_none());
}

/// FR-360-AC-7: a tuple naming a variant type checks and lowers, so its
/// position is a variant-typed member like any other.
#[trace("TC-1008", "FR-360-AC-7")]
#[test]
fn a_tuple_naming_a_variant_type_checks() {
    let refusals = check_refusals("tuple Pair(Config::Tier, Config::Tier);");
    assert!(refusals.is_empty(), "{refusals:?}");
}

// ---------------------------------------------------------------------------
// Case names that are not source identifiers
// ---------------------------------------------------------------------------

/// The cases of `Odd`, in ascending UTF-8 byte order of their names.
const ODD_CASES: [&str; 3] = ["2x", "Café", "Gold Plus"];

/// The FR-360 package with the variant types `Odd` (cases that are not
/// identifiers) and `Mixed` (`Gold` beside `Gold Plus`).
fn odd_document() -> Value {
    fixture::with_types(vec![
        fixture::variant_type("Odd", &ODD_CASES),
        fixture::variant_type("Mixed", &["Gold Plus", "Gold"]),
    ])
}

fn odd_refusals(declarations: &str) -> Vec<CheckRefusal> {
    declarations_of(&odd_document(), "Config", declarations)
        .0
        .check(CheckingLimits::default())
        .map(|_| Vec::new())
        .unwrap_or_else(|refusals| refusals)
}

/// FR-360-AC-4, FR-056 "Variant types": a variant type whose case names are
/// not identifiers (`2x`, `Café`, `Gold Plus`) has an enum shape whose members
/// are the FR-141 identities over its node and the exact name, in ascending
/// UTF-8 byte order of the names.
#[trace("FR-360-AC-4", "TC-1007")]
#[test]
fn a_variant_type_with_non_identifier_cases_has_an_enum_shape_over_the_exact_names() {
    let (declarations, _) = declarations_of(&odd_document(), "Config", "");
    let odd = variant(&declarations, "Config", "Odd");
    let names: Vec<&str> = odd
        .cases()
        .iter()
        .map(qsl_semantics::check::VariantCaseDeclaration::name)
        .collect();
    assert_eq!(names, ODD_CASES);
    for case in odd.cases() {
        assert_eq!(
            case.variant(),
            qsl_semantics::value::enumeration::mint_variant_id(odd.node(), case.name()),
            "{}",
            case.name()
        );
    }
    let shape = odd.shape();
    assert!(!shape.is_ordered());
    let members: Vec<_> = shape.variants().collect();
    let expected: Vec<_> = odd
        .cases()
        .iter()
        .map(qsl_semantics::check::VariantCaseDeclaration::variant)
        .collect();
    assert_eq!(members, expected);
}

/// FR-360-AC-4: a unit names a type with non-identifier cases in a parameter,
/// result and field read, and `Config::Mixed::Gold` checks beside the case
/// `Gold Plus`.
#[trace("FR-360-AC-4", "TC-1007")]
#[test]
fn a_type_with_non_identifier_cases_is_named_and_its_identifier_case_checks() {
    let refusals = odd_refusals(
        "function same_odd using v(o: Config::Odd): Config::Odd pure { o }\n\
         function gold using v(): Config::Mixed pure { Config::Mixed::Gold }",
    );
    assert!(refusals.is_empty(), "{refusals:?}");
}

/// FR-360-AC-6, FR-141-AC-5: `=` and `!=` over a type with non-identifier
/// cases check, and every ordering operator refuses
/// `ill_typed`/`operator-ineligible`.
#[trace("FR-360-AC-6", "TC-1008")]
#[test]
fn equality_is_allowed_and_ordering_is_refused_over_non_identifier_cases() {
    let allowed = odd_refusals(
        "function same using v(a: Config::Odd, b: Config::Odd): Boolean pure { a = b }\n\
         function differs using v(a: Config::Odd, b: Config::Odd): Boolean pure { a != b }",
    );
    assert!(allowed.is_empty(), "{allowed:?}");
    for operator in ["<", "<=", ">", ">="] {
        let refused = odd_refusals(&format!(
            "function ranks using v(a: Config::Odd, b: Config::Odd): Boolean pure {{ a {operator} b }}"
        ));
        assert_eq!(refused.len(), 1, "{operator}: {refused:?}");
        assert_eq!(
            refused[0].cause,
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
            "{operator}"
        );
    }
}
