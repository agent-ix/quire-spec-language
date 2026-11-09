// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-505 (TC-1300) through the spine: a bounded Rational value type is
//! admitted by model intake, assembled, and checked in clauses. Every test
//! enters through `admit_unit` -> `assemble` -> `check`; assertions compare
//! typed codes, causes and member paths, never rendered text.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{AssemblyRefusal, CheckedGraph};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use crate::model_operations::{
    admit_and_assemble_with_body, config_version_document, empty_frame, operation, wire_field,
    wire_type, PACKAGE_IDENTITY,
};
use qsl_foundation::IntakeLimits;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, ScalarValueDomain, ValueTypeRef,
};
use qsl_semantics::model::intake::{admit, meaning, package_input, read_domain_package};
use qsl_semantics::model::normalize::{
    normalize_with_meter, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};

fn identity(path: &str) -> String {
    format!("ix://{PACKAGE_IDENTITY}/{path}")
}

fn constraint(ratio: &str, keyword: &str, value: i64) -> Value {
    json!({
        "identity": format!("{ratio}/constraints/{keyword}"),
        "keyword": keyword,
        "operands": {"value": value},
        "appliesTo": ratio,
        "diagnosticCode": format!("rational.{keyword}"),
        "origin": {"generated": {
            "generatorIdentity": ratio,
            "generatorVersion": "1.0.0",
            "inputIdentities": [ratio],
        }},
    })
}

/// `Ratio` as TC-1300's R0 states it: `Rational[-3, 7; 1, 12]`.
fn ratio_constraints(ratio: &str) -> Vec<Value> {
    vec![
        constraint(ratio, "maxDenominator", 12),
        constraint(ratio, "minNumerator", -3),
        constraint(ratio, "minDenominator", 1),
        constraint(ratio, "maxNumerator", 7),
    ]
}

fn rational_type(name: &str, constraints: Vec<Value>, unit: Option<&str>) -> Value {
    let mut extra = json!({"scalar": "rational", "constraints": constraints});
    if let Some(unit) = unit {
        extra["unit"] = json!(unit);
    }
    wire_type(&identity(name), "value_type", extra)
}

/// R0: `ConfigVersion` plus the types `types` and the fields `fields`
/// (`(name, typeRef)`), each required with multiplicity one.
fn document(types: Vec<Value>, extra_constructs: Vec<Value>, fields: &[(&str, &str)]) -> Vec<u8> {
    let base = config_version_document(
        operation("unused", json!([]), None, empty_frame()),
        extra_constructs,
        types,
        json!([]),
    );
    let mut value: Value = serde_json::from_slice(&base).expect("fixture JSON");
    for (name, type_ref) in fields {
        let field = wire_field(
            &identity(&format!("ConfigVersion/{name}")),
            name,
            type_ref,
            "required",
            1,
        );
        let owner = value["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|ty| ty["identity"] == identity("ConfigVersion"))
            .expect("ConfigVersion");
        owner["fields"].as_array_mut().expect("fields").push(field);
    }
    serde_json::to_vec(&value).expect("fixture encoding")
}

fn assemble(document: &[u8], body: &str) -> Result<CheckedGraph, Vec<(Code, Option<String>)>> {
    let declarations = match admit_and_assemble_with_body(document, body) {
        Ok(declarations) => declarations,
        Err(AssemblyRefusal { errors, .. }) => {
            return Err(errors
                .iter()
                .map(|error| {
                    (
                        error.cause.code(),
                        Some(error.cause.catalog_code().cause().to_owned()),
                    )
                })
                .collect())
        }
    };
    declarations
        .check(
            CheckingLimits::default(),
            quire_semantic_value::checking::CheckMode::Linked,
        )
        .map_err(|refusals| {
            refusals
                .iter()
                .map(|refusal| {
                    (
                        refusal.cause.code(),
                        refusal.cause.cause().map(str::to_owned),
                    )
                })
                .collect()
        })
}

// ---- FR-505-AC-10: tuple positions and variant payloads (package_read) ----

const MODULE: &str = "test/ratio-positions";

fn generated(node: &str) -> Value {
    json!({"generated": {
        "generatorIdentity": identity(node),
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity(node)],
    }})
}

fn sourced(line: u64) -> Value {
    json!({"source": {
        "sourceIdentity": identity("spec"),
        "path": "spec.qspec",
        "startLine": line,
        "startColumn": 3,
    }})
}

fn construct(name: &str, meaning: &str, shape: &str, members: Value) -> Value {
    json!({
        "kind": {"module": MODULE, "name": name},
        "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": shape, "members": members, "meaning": meaning},
    })
}

fn definition(name: &str, kind: &str, members: Value) -> Value {
    let mut definition = json!({
        "identity": identity(name), "displayName": name,
        "kind": {"module": MODULE, "name": kind},
        "roles": [], "origin": generated(name),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
    });
    definition
        .as_object_mut()
        .expect("definition is an object")
        .extend(members.as_object().expect("members are an object").clone());
    definition
}

/// TC-1300 row 24: `Ratio` (bounded, or with empty `constraints`), the tuple
/// type `P` whose position `p0` is typed `Ratio`, and the variant type `V`
/// whose case `c` has `Ratio` as its `payloadType`.
fn positions_document(bounded: bool, via_alias: bool) -> Value {
    let ratio = identity("Ratio");
    // The position and the payload name `Ratio` itself, or an alias of it.
    let named = if via_alias {
        identity("RatioAlias")
    } else {
        ratio.clone()
    };
    let constraints = if bounded {
        ratio_constraints(&ratio)
    } else {
        Vec::new()
    };
    let ratio_type = definition(
        "Ratio",
        "value_type",
        json!({"scalar": "rational", "constraints": constraints}),
    );
    let tuple = definition(
        "P",
        "tuple_type",
        json!({"positions": [{
            "identity": identity("P/p0"),
            "typeRef": named,
            "origin": sourced(5),
        }]}),
    );
    let variant = definition(
        "V",
        "variant_type",
        json!({"variants": [{
            "identity": identity("variant/V-c"),
            "name": "c",
            "origin": sourced(7),
            "payloadType": named,
        }]}),
    );
    let mut types = vec![ratio_type, tuple, variant];
    if via_alias {
        let mut alias = definition("RatioAlias", "unused", json!({"target": ratio}));
        alias["kind"] = json!("alias");
        types.push(alias);
    }
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": identity("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": PACKAGE_IDENTITY, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": []},
        "constructs": [
            construct("value_type", meaning::VALUE_TYPE, "record", json!({})),
            construct("tuple_type", meaning::TUPLE_VALUE_TYPE, "tuple", json!({"positions": "required"})),
            construct("variant_type", meaning::VARIANT_TYPE, "enumeration", json!({"variants": "required"})),
        ],
        "types": types,
        "occurrences": [], "extensions": [],
    })
}

fn read_positions(document: &Value) -> Result<DomainPackage, Vec<ModelRefusal>> {
    let bytes = serde_json::to_vec(document).expect("authored document serializes");
    let packages = package_input([bytes.as_slice()]).unwrap();
    let (selection, admitted) =
        admit(PACKAGE_IDENTITY, &packages, IntakeLimits::default()).expect("byte admission");
    read_domain_package(selection, &admitted)
}

fn key(node: &str) -> quire_checked::key::DeclarationKey {
    quire_checked::key::DeclarationKey {
        package: PACKAGE_IDENTITY.to_owned(),
        node: identity(node),
    }
}

/// TC-1300 row 24 (FR-505-AC-10, record level): a tuple position and a
/// variant case payload typed by a bounded `Ratio` admit, and both name the
/// one `Ratio` declaration whose domain is `Rational[-3, 7; 1, 12]`. The
/// checked type and the `rational_range` node of the two are not asserted
/// here, so the test is not tagged with the criterion.
#[trace("TC-1300")]
#[test]
fn a_bounded_ratio_admits_as_a_tuple_position_and_a_variant_payload() {
    let package = read_positions(&positions_document(true, false)).expect("the package reads");
    let ratio = package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::ScalarType(scalar) if scalar.key == key("Ratio") => Some(scalar),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(ratio.len(), 1, "one original declaration for Ratio");
    let ScalarValueDomain::Rational {
        domain: Some(domain),
        unit: None,
    } = &ratio[0].domain
    else {
        panic!("Ratio is a bounded Rational: {:?}", ratio[0].domain);
    };
    assert_eq!(
        (
            domain.numerator().lower().to_string(),
            domain.numerator().upper().to_string(),
            domain.denominator().lower().to_string(),
            domain.denominator().upper().to_string(),
        ),
        (
            "-3".to_owned(),
            "7".to_owned(),
            "1".to_owned(),
            "12".to_owned()
        )
    );
    let named = ValueTypeRef::Package(key("Ratio"));
    assert!(package.records.iter().any(|record| matches!(
        record,
        DomainPackageRecord::TuplePosition(position)
            if position.key == key("P/p0") && position.value_type == named
    )));
    assert!(package.records.iter().any(|record| matches!(
        record,
        DomainPackageRecord::Variant(variant)
            if variant.key == key("variant/V-c") && variant.payload_type.as_ref() == Some(&named)
    )));
}

/// TC-1300 row 25 (FR-505-AC-10, refusal half): with `Ratio`'s `constraints`
/// empty, a tuple position and a variant payload that name `Ratio` or an
/// alias of it refuse at the package read: the position
/// `invalid_model_binding`/`malformed-declaration` at the position's
/// `typeRef`, the payload `unsupported_construct`/`declaration-form` at
/// `Variant.payloadType`. Both are intake-reader causes raised by
/// `read_domain_package`, before any assembly. The "field admits with no
/// checked type" clause is not carried by the spine, so the test is not
/// tagged with the criterion.
#[trace("TC-1300")]
#[test]
fn an_unbounded_ratio_refuses_as_a_tuple_position_and_a_variant_payload() {
    for via_alias in [false, true] {
        let package =
            read_positions(&positions_document(false, via_alias)).expect("the package reads");
        assert!(
            package.records.iter().all(|record| !matches!(
                record,
                DomainPackageRecord::TuplePosition(_) | DomainPackageRecord::Variant(_)
            )),
            "alias {via_alias}: neither the position nor the payload is admitted"
        );
        let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::default());
        let NormalizeOutcome::Refused(refusals) = outcome else {
            panic!("alias {via_alias}: the reader's refusals surface: {outcome:?}");
        };
        let refusals: Vec<ModelRefusal> = refusals.iter().cloned().collect();
        assert_eq!(refusals.len(), 2, "alias {via_alias}: {refusals:#?}");
        let position = refusals
            .iter()
            .find(|refusal| refusal.code == Code::InvalidModelBinding)
            .expect("the tuple position refuses malformed-declaration");
        assert!(
            matches!(&position.cause, ModelRefusalCause::IntakeMalformedDeclaration { node, member, .. }
                if *node == identity("P/p0")
                    && member.to_string() == "TypeDefinition.positions[0].typeRef"),
            "alias {via_alias}: {position:#?}"
        );
        let payload = refusals
            .iter()
            .find(|refusal| refusal.code == Code::UnsupportedConstruct)
            .expect("the variant payload refuses declaration-form");
        assert!(
            matches!(&payload.cause, ModelRefusalCause::UnsupportedDeclarationForm { node, member, .. }
                if *node == identity("variant/V-c") && member.to_string() == "Variant.payloadType"),
            "alias {via_alias}: {payload:#?}"
        );
    }
}

fn source_refusals(declarations: &str) -> Result<(), Vec<(Code, Option<String>)>> {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{declarations}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "rational-types"),
        "rational-types.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = match qsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    ) {
        Ok(declarations) => declarations,
        Err(refusal) => {
            return Err(refusal
                .errors
                .iter()
                .map(|error| {
                    (
                        error.cause.code(),
                        Some(error.cause.catalog_code().cause().to_owned()),
                    )
                })
                .collect())
        }
    };
    declarations
        .check(
            CheckingLimits::default(),
            quire_semantic_value::checking::CheckMode::Linked,
        )
        .map(|_| ())
        .map_err(|refusals| {
            refusals
                .iter()
                .map(|refusal| {
                    (
                        refusal.cause.code(),
                        refusal.cause.cause().map(str::to_owned),
                    )
                })
                .collect()
        })
}

/// FR-505-AC-9 (TC-1300 row 20): a source `Rational[..]` whose numerator
/// bounds are inverted or whose denominator may be zero, as a function
/// parameter type and as the carrier of `Quantity<m, ..>`, refuses
/// `ill_typed` once at the type; the consistent bounds check.
#[trace("FR-505-AC-9", "TC-1300")]
#[test]
fn inconsistent_source_rational_bounds_refuse_ill_typed_at_the_type() {
    const UNITS: &str = "dimension Length;\nunit m : Length = rational(1, 1);\n";
    for (carrier, refuses) in [
        ("Rational[-3, 7; 1, 12]", false),
        ("Rational[8, 7; 1, 12]", true),
        ("Rational[0, 7; 0, 12]", true),
    ] {
        for (prefix, parameter) in [
            ("", carrier.to_owned()),
            (UNITS, format!("Quantity<m, {carrier}>")),
        ] {
            let outcome = source_refusals(&format!(
                "{prefix}function f using v(a: {parameter}): Boolean pure {{ true }}"
            ));
            if refuses {
                let found = outcome.expect_err(&parameter);
                assert_eq!(found.len(), 1, "{parameter}: {found:?}");
                assert_eq!(found[0].0, Code::IllTyped, "{parameter}");
            } else {
                assert_eq!(outcome, Ok(()), "{parameter}");
            }
        }
    }
}

/// The package's `Length` dimension with the units `m` and `cm`.
fn with_length_units(document: &[u8]) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(document).expect("fixture JSON");
    let origin = value["types"][0]["origin"].clone();
    value["dimensions"] = json!([{
        "identity": identity("dimension/Length"), "name": "Length",
        "base": true, "terms": [], "origin": origin
    }]);
    value["units"] = json!([
        {"identity": identity("unit/m"), "name": "m",
         "dimension": identity("dimension/Length"), "scale": "1", "offset": "0",
         "origin": origin},
        {"identity": identity("unit/cm"), "name": "cm",
         "dimension": identity("dimension/Length"), "target": identity("unit/m"),
         "scale": "1/100", "offset": "0", "origin": origin}
    ]);
    serde_json::to_vec(&value).expect("fixture encoding")
}

/// FR-505-AC-9 (TC-1300 row 19): over a unit-bearing `Ratio` whose unit `m` is
/// declared in the type's own model document, which also declares an object
/// type, two fields `a` and `b` of `Ratio`: the invariants
/// `self.a = self.b`, `self.a < self.b` and `self.a + self.b = self.a` each
/// refuse `ill_typed`/`type-mismatch`, while
/// `convert<Quantity<Config::m>>(self.a) = convert<Quantity<Config::m>>(self.b)` and the same
/// with `<` check. `m` is the document's unit.
#[trace("FR-505-AC-9", "TC-1300")]
#[test]
fn unit_bearing_rational_members_compare_only_through_convert() {
    let ratio = identity("Ratio");
    let document = with_length_units(&document(
        vec![rational_type(
            "Ratio",
            ratio_constraints(&ratio),
            Some("ix://quire/unit/m"),
        )],
        Vec::new(),
        &[("a", &ratio), ("b", &ratio)],
    ));
    for clause in [
        "self.a = self.b",
        "self.a < self.b",
        "self.a + self.b = self.a",
    ] {
        let body =
            format!("invariant I using v on Config::ConfigVersion at current {{ {clause} }}\n");
        assert_eq!(
            assemble(&document, &body).map(|_| ()),
            Err(vec![(Code::IllTyped, Some("type-mismatch".to_owned()))]),
            "{clause}"
        );
    }
    for clause in [
        "convert<Quantity<Config::m>>(self.a) = convert<Quantity<Config::m>>(self.b)",
        "convert<Quantity<Config::m>>(self.a) < convert<Quantity<Config::m>>(self.b)",
        "convert<Quantity<Config::m>>(self.a) = 1 Config::m",
        "convert<Quantity<Config::cm>>(self.a) = 100 Config::cm",
    ] {
        let body =
            format!("invariant I using v on Config::ConfigVersion at current {{ {clause} }}\n");
        assert_eq!(assemble(&document, &body).map(|_| ()), Ok(()), "{clause}");
    }
}

/// FR-505-AC-9 (TC-1300 row 19), wrong-unit controls: the converted sides must
/// agree on the unit, so a different unit, a different dimension or a bare
/// carried member refuses `ill_typed`/`type-mismatch`.
#[trace("TC-1300")]
#[test]
fn unit_bearing_rational_comparisons_refuse_a_wrong_unit() {
    let ratio = identity("Ratio");
    let document = with_length_units(&document(
        vec![rational_type(
            "Ratio",
            ratio_constraints(&ratio),
            Some("ix://quire/unit/m"),
        )],
        Vec::new(),
        &[("a", &ratio), ("b", &ratio)],
    ));
    for clause in [
        "convert<Quantity<Config::m>>(self.a) = convert<Quantity<Config::cm>>(self.b)",
        "convert<Quantity<Config::m>>(self.a) = 1 Config::cm",
        "convert<Quantity<Config::m>>(self.a) = self.b",
    ] {
        let body =
            format!("invariant I using v on Config::ConfigVersion at current {{ {clause} }}\n");
        assert_eq!(
            assemble(&document, &body).map(|_| ()),
            Err(vec![(Code::IllTyped, Some("type-mismatch".to_owned()))]),
            "{clause}"
        );
    }
}

/// FR-505-AC-9 (TC-1300 row 19): over the same document, the bare
/// `convert<Quantity<m>>(self.a) = convert<Quantity<m>>(self.b)` names a unit
/// the compiling unit does not declare, so it refuses `missing_declaration`/
/// `missing-name` with `UnresolvedUnitName` naming the document's package,
/// while `Config::m` names the document's unit.
#[trace("FR-505-AC-9", "TC-1300")]
#[test]
fn a_bare_unit_name_never_reaches_the_document_unit() {
    use qsl_semantics::check::AssemblyCause;
    let ratio = identity("Ratio");
    let document = with_length_units(&document(
        vec![rational_type(
            "Ratio",
            ratio_constraints(&ratio),
            Some("ix://quire/unit/m"),
        )],
        Vec::new(),
        &[("a", &ratio), ("b", &ratio)],
    ));
    let body = "invariant I using v on Config::ConfigVersion at current { \
        convert<Quantity<m>>(self.a) = convert<Quantity<m>>(self.b) }\n";
    let expected = AssemblyCause::UnresolvedUnitName {
        name: "m".to_owned(),
        packages: vec![PACKAGE_IDENTITY.to_owned()],
    };
    let refusal = admit_and_assemble_with_body(&document, body)
        .map(|_| ())
        .expect_err("the bare unit name refuses at assembly");
    let causes: Vec<_> = refusal.errors.iter().map(|error| &error.cause).collect();
    assert_eq!(causes, vec![&expected, &expected]);
    assert_eq!(expected.code(), Code::MissingDeclaration);
    assert_eq!(expected.catalog_code().cause(), "missing-name");
    let first = &refusal.errors[0].span;
    let (text, _) = crate::model_operations::config_unit_with_profiles(&document, "", body);
    assert_eq!(&text[first.start..first.end], "m");
    assert_eq!(
        first.start,
        text.find("Quantity<m>").unwrap() + "Quantity<".len()
    );
}

/// A source unit `m` of another dimension takes the name from the model
/// document's `m` (the compiling unit's own unit wins), so the document's
/// `Ratio` carries the source `m` (Time): `convert<Quantity<Config::cm>>(self.a)`,
/// a conversion to the document's Length unit, checks without the source `m`
/// and is refused `ill_typed`/`type-mismatch` with it.
#[trace("TC-1300")]
#[test]
fn a_source_unit_named_like_the_document_unit_wins_the_name() {
    let ratio = identity("Ratio");
    let document = with_length_units(&document(
        vec![rational_type(
            "Ratio",
            ratio_constraints(&ratio),
            Some("ix://quire/unit/m"),
        )],
        Vec::new(),
        &[("a", &ratio), ("b", &ratio)],
    ));
    let clause = "invariant I using v on Config::ConfigVersion at current { \
        convert<Quantity<Config::cm>>(self.a) = 100 Config::cm }\n";
    assert_eq!(assemble(&document, clause).map(|_| ()), Ok(()));
    let shadowed = format!("dimension Time;\nunit m : Time = rational(1, 1);\n{clause}");
    assert_eq!(
        assemble(&document, &shadowed).map(|_| ()),
        Err(vec![(Code::IllTyped, Some("type-mismatch".to_owned()))])
    );
}

/// The unit selecting `packages` with the source `body` after a source `Time`,
/// assembled and then checked: the assembly causes, else the check causes.
#[cfg(feature = "test-support")]
fn unit_name_causes(
    packages: &[(&str, &str, Vec<u8>)],
    body: &str,
) -> Result<
    (),
    (
        Vec<qsl_semantics::check::AssemblyCause>,
        Vec<qsl_semantics::check::CheckCause>,
    ),
> {
    use crate::timed_declarations::assemble_selected_as;
    let assembled = assemble_selected_as(("test", "fr-505-two-units"), packages, body)
        .1
        .map_err(|refusal| {
            (
                refusal
                    .errors
                    .into_iter()
                    .map(|error| error.cause)
                    .collect::<Vec<_>>(),
                Vec::new(),
            )
        })?;
    assembled
        .check(
            CheckingLimits::default(),
            quire_semantic_value::checking::CheckMode::Linked,
        )
        .map(|_| ())
        .map_err(|refusals| {
            (
                Vec::new(),
                refusals.into_iter().map(|refusal| refusal.cause).collect(),
            )
        })
}

/// FR-505-AC-12 (TC-1300 row 28, FR-091-AC-21): two selected models that each
/// declare a unit `x` leave the bare name `x` unresolved: it refuses
/// `missing_declaration`/`missing-name` once through `UnresolvedUnitName`
/// naming both packages in selection order, and no package is picked. `A::x`
/// and `B::x` resolve. With `acme/b` declaring `y`, the cause names `acme/a`
/// alone, and a name no package declares names none.
#[cfg(feature = "test-support")]
#[trace("FR-505-AC-12", "TC-1300", "FR-091-AC-21")]
#[test]
fn a_bare_unit_name_declared_by_two_models_is_refused() {
    use qsl_semantics::check::AssemblyCause;
    let package = |alias, identity, unit| {
        (
            alias,
            identity,
            crate::timed_declarations::time_package(identity, false, &[(unit, None, "1")]),
        )
    };
    let both = [package("A", "acme/a", "x"), package("B", "acme/b", "x")];
    let one = [package("A", "acme/a", "x"), package("B", "acme/b", "y")];
    let refusal = |packages: &[(&str, &str, Vec<u8>)], unit: &str| {
        unit_name_causes(
            packages,
            &format!("function f using v(d: Quantity<{unit}>): Boolean pure {{ d = d }}"),
        )
    };
    let unresolved = |name: &str, packages: &[&str]| {
        Err((
            vec![AssemblyCause::UnresolvedUnitName {
                name: name.to_owned(),
                packages: packages
                    .iter()
                    .map(|package| (*package).to_owned())
                    .collect(),
            }],
            Vec::new(),
        ))
    };
    assert_eq!(refusal(&both, "x"), unresolved("x", &["acme/a", "acme/b"]));
    for unit in ["A::x", "B::x"] {
        assert_eq!(refusal(&both, unit), Ok(()), "{unit}");
    }
    // The cause follows the order the `model` declarations select the
    // packages in, not alias or identity order.
    let reversed = [package("B", "acme/b", "x"), package("A", "acme/a", "x")];
    assert_eq!(
        refusal(&reversed, "x"),
        unresolved("x", &["acme/b", "acme/a"])
    );
    assert_eq!(refusal(&one, "x"), unresolved("x", &["acme/a"]));
    assert_eq!(refusal(&one, "nosuch"), unresolved("nosuch", &[]));
    assert_eq!(refusal(&one, "y"), unresolved("y", &["acme/b"]));
    let cause = AssemblyCause::UnresolvedUnitName {
        name: "x".to_owned(),
        packages: vec!["acme/a".to_owned()],
    };
    assert_eq!(cause.code(), Code::MissingDeclaration);
    assert_eq!(cause.catalog_code().cause(), "missing-name");
}

/// FR-505-AC-12 (TC-1300 row 28): the quantity literal `1 x` and the target of
/// `convert<Quantity<x>>(1 A::x)` refuse the same way at `x`, the literal as the
/// check phase's `UnresolvedUnitName` and the target as the assembler's; the qualified spellings check.
#[cfg(feature = "test-support")]
#[trace("FR-505-AC-12", "TC-1300")]
#[test]
fn a_bare_unit_name_in_a_literal_or_a_convert_target_is_refused() {
    use qsl_semantics::check::CheckCause;
    let package = |alias, identity| {
        (
            alias,
            identity,
            crate::timed_declarations::time_package(identity, false, &[("x", None, "1")]),
        )
    };
    let both = [package("A", "acme/a"), package("B", "acme/b")];
    let names = vec!["acme/a".to_owned(), "acme/b".to_owned()];
    assert_eq!(
        unit_name_causes(
            &both,
            "function f using v(d: Quantity<A::x>): Boolean pure { d = 1 x }"
        ),
        Err((
            Vec::new(),
            vec![CheckCause::UnresolvedUnitName {
                name: "x".to_owned(),
                packages: names.clone()
            }]
        ))
    );
    assert_eq!(
        unit_name_causes(
            &both,
            "function f using v(d: Quantity<A::x>): Boolean pure { convert<Quantity<x>>(1 A::x) = d }"
        ),
        Err((
            vec![qsl_semantics::check::AssemblyCause::UnresolvedUnitName {
                name: "x".to_owned(),
                packages: names
            }],
            Vec::new()
        ))
    );
    for body in [
        "function f using v(d: Quantity<A::x>): Boolean pure { d = 1 A::x }",
        "function f using v(d: Quantity<A::x>): Boolean pure { convert<Quantity<A::x>>(1 A::x) = d }",
    ] {
        assert_eq!(unit_name_causes(&both, body), Ok(()), "{body}");
    }
}

/// FR-505-AC-12 (TC-1300 row 28): a unit the compiling unit declares takes the
/// bare name, whatever its dimension, with no refusal; and a value type of
/// `acme/a` over `ix://quire/unit/x`, beside `acme/b`'s `x`, types its members
/// over `acme/a`'s own `x`.
#[cfg(feature = "test-support")]
#[trace("FR-505-AC-12", "TC-1300")]
#[test]
fn a_source_unit_takes_the_bare_name_and_a_document_keeps_its_own() {
    let package = |alias, identity| {
        (
            alias,
            identity,
            crate::timed_declarations::time_package(identity, false, &[("x", None, "1")]),
        )
    };
    let both = [package("A", "acme/a"), package("B", "acme/b")];
    let source = "dimension L;\nunit x : L = rational(1, 1);\n\
        function f using v(d: Quantity<x>): Boolean pure { d = 1 x }";
    assert_eq!(unit_name_causes(&both, source), Ok(()));

    let mut a = serde_json::from_slice::<Value>(&crate::timed_declarations::time_package(
        "acme/a",
        false,
        &[("x", None, "1")],
    ))
    .expect("fixture JSON");
    let seconds: Value = serde_json::from_str(
        &serde_json::to_string(&wire_type(
            "ix://acme/a/Seconds",
            "value_type",
            json!({"scalar": "rational", "unit": "ix://quire/unit/x"}),
        ))
        .expect("fixture encoding")
        .replace("example/config-version", "acme/a"),
    )
    .expect("fixture JSON");
    a["types"].as_array_mut().expect("types").push(seconds);
    let selected = [
        (
            "A",
            "acme/a",
            serde_json::to_vec(&a).expect("fixture encoding"),
        ),
        package("B", "acme/b"),
    ];
    assert_eq!(
        unit_name_causes(
            &selected,
            "function g using v(d: Quantity<A::x>): Boolean pure { d = d }"
        ),
        Ok(())
    );
}
