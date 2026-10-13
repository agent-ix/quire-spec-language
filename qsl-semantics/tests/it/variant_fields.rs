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
    ValueType::Enum(
        variant(declarations, alias, name)
            .shape()
            .expect("the variant type has an enum shape"),
    )
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
    let document = fixture::document();
    let bytes = serde_json::to_vec(&document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let model = json!({
        "identity": fixture::PACKAGE, "version": "1",
        "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(digest)),
    });
    let declarations = assemble_as("Config", "");
    let graph = declarations
        .clone()
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

/// The refusal `fields` is admitted with, as its code, cause and nested path.
fn refusal_of(
    replay: &Replay,
    tier: serde_json::Value,
) -> (qsl_foundation::Code, &'static str, Vec<PathStep>) {
    match replay.admit(json!({"tier": tier})) {
        Err(AdmissionFailure::Refused(record)) => (record.code, record.cause, record.path),
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
    for (label, value, cause) in [
        (
            "a missing case",
            json!({"variant": {"type": tier}}),
            "missing-member",
        ),
        (
            "a missing type",
            json!({"variant": {"case": "Gold"}}),
            "missing-member",
        ),
        (
            "an extra member",
            json!({"variant": {"type": tier, "case": "Gold", "x": 1}}),
            "unknown-member",
        ),
        (
            "a type naming another variant type",
            json!({"variant": {"type": level, "case": "Gold"}}),
            "wrong-value-kind",
        ),
        (
            "a type naming a record",
            json!({"variant": {"type": id("Settings"), "case": "Gold"}}),
            "wrong-value-kind",
        ),
        (
            "a type naming an object",
            json!({"variant": {"type": id("Account"), "case": "Gold"}}),
            "wrong-value-kind",
        ),
        (
            "a type naming no declaration",
            json!({"variant": {"type": id("Nope"), "case": "Gold"}}),
            "wrong-value-kind",
        ),
        (
            "a Text value at the slot",
            json!({"string": "Gold"}),
            "wrong-value-kind",
        ),
        (
            "an Integer value at the slot",
            json!({"integer": "0"}),
            "wrong-value-kind",
        ),
        (
            "a payload that is not an object",
            json!({"variant": "Gold"}),
            "wrong-value-kind",
        ),
        (
            "an absent value at a required field",
            json!({"absent": {}}),
            "wrong-value-kind",
        ),
        (
            "an undeclared case",
            json!({"variant": {"type": tier, "case": "Silver"}}),
            "invalid-value",
        ),
        (
            "a letter-case variant of a case",
            json!({"variant": {"type": tier, "case": "gold"}}),
            "invalid-value",
        ),
        (
            "a missing case beats a wrong type",
            json!({"variant": {"type": level}}),
            "missing-member",
        ),
        (
            "an extra member beats an undeclared case",
            json!({"variant": {"type": tier, "case": "Silver", "x": 1}}),
            "unknown-member",
        ),
        (
            "a wrong type beats an undeclared case",
            json!({"variant": {"type": level, "case": "Silver"}}),
            "wrong-value-kind",
        ),
    ] {
        let (code, found, path) = refusal_of(&replay, value);
        assert_eq!((code, found), (invalid, cause), "{label}");
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

/// TC-1009 step 3 and FR-360 item 12 (FR-360-AC-8): until QSL-748
/// requirement 1 holds in the emitter, a unit whose function takes a variant
/// type refuses at emission with `ill_typed`/`type-mismatch` at the
/// parameter's type node, and checks nothing else.
#[trace("TC-1009", "FR-360-AC-8")]
#[test]
fn the_emitter_refuses_a_variant_typed_parameter_at_its_type_node() {
    let refusals = check_refusals("function pick using v(t: Config::Tier): Boolean pure { true }");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].cause, mismatch(), "{refusals:?}");
}
