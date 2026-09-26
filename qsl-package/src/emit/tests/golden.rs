// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-416 step 7 (FR-093-AC-13): the emitter's application nodes against
//! QSpec's v2 positive fixtures `positive-operation-identities.json` and
//! `positive-control-operations.json`, read from `$QSPEC_DIR` at run time
//! under `make conformance`. Nothing of QSpec is copied into this repository.

use super::*;

/// The application nodes of one fixture, `(identity, mode value, node)`.
fn fixture_applications(path: &std::path::Path) -> Vec<Value> {
    let bytes =
        std::fs::read(path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let fixture: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()));
    fixture["semantic_graph"]["nodes"]
        .as_array()
        .unwrap_or_else(|| panic!("{} has no semantic_graph.nodes", path.display()))
        .iter()
        .filter(|node| node["body"]["term"] == "application")
        .cloned()
        .collect()
}

/// The FR-093-AC-13 comparison members of an application node, and only
/// those: `node_tag`, `semantic_form`, `operator`, `operation.identity`, law
/// roles in order, `mode`, member `kind` and `name`, each leaf's `path` and
/// `mode`, and each argument's term kind and binding name. Every other
/// member (ids, keys, `semantic_type`, dependencies, law definitions) is
/// placeholder in the fixtures and stays outside the comparison.
fn compared(node: &Value, with_mode: bool) -> Value {
    fn lowered(term: &Value) -> Value {
        match term["term"].as_str() {
            Some("binding") => json!(["binding", term["name"], lowered(&term["value"])]),
            other => json!([other]),
        }
    }
    let body = &node["body"];
    let operation = &body["operation"];
    let roles: Vec<&Value> = operation["laws"]
        .as_array()
        .unwrap()
        .iter()
        .map(|law| &law["role"])
        .collect();
    let leaves: Vec<Value> = operation["leaves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|leaf| json!({"path": leaf["path"], "mode": leaf["mode"]}))
        .collect();
    let member = if operation["member"].is_null() {
        Value::Null
    } else {
        json!({"kind": operation["member"]["kind"], "name": operation["member"]["name"]})
    };
    let arguments: Vec<Value> = body["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(lowered)
        .collect();
    json!({
        "node_tag": node["node_tag"],
        "semantic_form": node["semantic_form"],
        "operator": body["operator"],
        "identity": operation["identity"],
        "laws": roles,
        "mode": if with_mode { operation["mode"].clone() } else { Value::Null },
        "member": member,
        "leaves": leaves,
        "arguments": arguments,
    })
}

/// One function body QSL lowers to an application node with `identity`
/// (and, where the fixtures carry several nodes of one identity, `mode`).
struct Case {
    identity: &'static str,
    mode: Option<&'static str>,
    /// Whether `mode` is compared. `false` only where QSL cannot write the
    /// fixture's mode (`float64.add`, see the test's comment).
    with_mode: bool,
    package: CheckedPackage,
}

fn form(builtin: BuiltinType, bounds: &[&str]) -> TypeForm {
    TypeForm::builtin(builtin, SPAN).with_bounds(bounds.iter().map(|b| (*b).to_owned()).collect())
}

fn text(profile: &str) -> TypeForm {
    form(BuiltinType::Text, &["0", "8", profile])
}

fn typed(
    name: &str,
    parameters: &[(&str, TypeForm)],
    result: TypeForm,
    body: Expression,
) -> FunctionDeclaration {
    FunctionDeclaration::new(
        name,
        parameters
            .iter()
            .map(|(name, form)| ((*name).to_owned(), form.clone()))
            .collect(),
        result,
        None,
        body,
    )
}

fn binary(operator: BinaryOperator, left: &str, right: &str) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(name(left)),
        right: Box::new(name(right)),
    }
}

/// `record R { name: Text[0, 8; nfc]; }` (FR-093-AC-11's shape).
fn record_types() -> TypeEnvironment {
    let text =
        ValueType::Text(quire_exact::TextType::new(0, 8, quire_exact::TextProfile::Nfc).unwrap());
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            NodeKey::from_digest([7; 32]),
            "R",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "name",
                text,
                Presence::Required,
            )]),
        )],
        [],
    )
    .expect("FR-143 admits R")
}

/// The lock evidence of these packages: the catalog's text-profile
/// definition, which the emitted lock selects.
fn lock_evidence() -> qsl_semantics::check::LockEvidence {
    let entry = DefinitionLock::pinned()
        .entry(CatalogRole::TextProfile)
        .expect("the catalog names a text profile");
    qsl_semantics::check::LockEvidence::default().with_text_profile(
        qsl_semantics::value::DefinitionReference {
            authority: entry.authority.to_owned(),
            identity: entry.identity.to_owned(),
            revision: qsl_semantics::value::DefinitionRevision {
                namespace: entry.revision_namespace.to_owned(),
                value: entry.revision_value.to_owned(),
            },
            digest_domain: "quire.definition.bytes/v1".to_owned(),
            digest: entry.digest.to_owned(),
        },
    )
}

/// The package's one admitted IEEE profile: the catalog's own definition.
fn ieee_profile() -> qsl_semantics::value::AdmittedIeeeProfile {
    let lock = DefinitionLock::pinned();
    let entry = lock
        .entry(CatalogRole::IeeeProfile)
        .expect("the catalog names an IEEE profile");
    lock.admit_ieee_profile(
        &[qsl_semantics::value::DefinitionReference {
            authority: entry.authority.to_owned(),
            identity: entry.identity.to_owned(),
            revision: qsl_semantics::value::DefinitionRevision {
                namespace: entry.revision_namespace.to_owned(),
                value: entry.revision_value.to_owned(),
            },
            digest_domain: "quire.definition.bytes/v1".to_owned(),
            digest: entry.digest.to_owned(),
        }],
        &[],
    )
    .expect("the catalog's IEEE profile is admitted")
}

fn linked(types: TypeEnvironment, functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    CheckedPackage::link(
        PackageDeclarations {
            types,
            functions,
            lock_evidence: lock_evidence(),
            ieee_profile: Some(ieee_profile()),
            ..PackageDeclarations::new(source())
        }
        .check(CheckingLimits::default())
        .expect("the fixture functions check"),
    )
}

fn package(functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    linked(TypeEnvironment::default(), functions)
}

fn with_record(functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    linked(record_types(), functions)
}

fn cases() -> Vec<Case> {
    let integer = || TypeForm::builtin(BuiltinType::Integer, SPAN);
    let small = || form(BuiltinType::Int, &["1", "9"]);
    let case = |identity, mode, package| Case {
        identity,
        mode,
        with_mode: true,
        package,
    };
    let record = || TypeForm::name("R", SPAN);
    let texts = || {
        TypeForm::collection(CollectionKind::Sequence, SPAN)
            .with_arguments(vec![text("nfc")])
            .with_bounds(vec!["0".into(), "5".into()])
    };
    vec![
        case("quire.op.control.if", None, package(vec![t()])),
        case(
            "quire.op.integer.add",
            None,
            package(vec![typed(
                "add",
                &[("x", integer()), ("y", integer())],
                integer(),
                binary(BinaryOperator::Add, "x", "y"),
            )]),
        ),
        case(
            "quire.op.integer.mul",
            None,
            package(vec![typed(
                "mul",
                &[("x", integer()), ("y", integer())],
                integer(),
                binary(BinaryOperator::Multiply, "x", "y"),
            )]),
        ),
        case(
            "quire.op.rational.div",
            None,
            package(vec![typed(
                "rdiv",
                &[("x", small()), ("y", small())],
                form(BuiltinType::Rational, &["-1000", "1000", "1", "1000"]),
                binary(BinaryOperator::Divide, "x", "y"),
            )]),
        ),
        case(
            "quire.op.text.eq",
            Some("nfc"),
            package(vec![typed(
                "teq",
                &[("p", text("nfc")), ("r", text("nfc"))],
                boolean(),
                binary(BinaryOperator::Equal, "p", "r"),
            )]),
        ),
        case(
            "quire.op.text.eq",
            Some("binary-utf8"),
            package(vec![typed(
                "beq",
                &[("p", text("binary-utf8")), ("r", text("binary-utf8"))],
                boolean(),
                binary(BinaryOperator::Equal, "p", "r"),
            )]),
        ),
        case(
            // `a = a`: IR's v2 reader refuses `a = b` over two distinct
            // record parameters (`ill_typed`/`operator-ineligible` at the
            // second argument), so the comparison uses one operand twice,
            // as the fixture's own node does.
            "quire.op.structural.eq",
            None,
            with_record(vec![typed(
                "seq",
                &[("a", record())],
                boolean(),
                binary(BinaryOperator::Equal, "a", "a"),
            )]),
        ),
        case(
            "quire.op.record.project",
            None,
            with_record(vec![typed(
                "proj",
                &[("a", record())],
                text("nfc"),
                Expression::Field {
                    operand: Box::new(name("a")),
                    field: "name".to_owned(),
                },
            )]),
        ),
        case(
            "quire.op.collection.contains",
            None,
            package(vec![typed(
                "has",
                &[("s", texts()), ("t", text("nfc"))],
                boolean(),
                Expression::Contains {
                    collection: Box::new(name("s")),
                    item: Box::new(name("t")),
                },
            )]),
        ),
        case(
            "quire.op.function.call",
            None,
            package(vec![
                typed("g", &[], boolean(), Expression::Boolean(true)),
                typed(
                    "c",
                    &[],
                    boolean(),
                    Expression::Call {
                        name: "g".to_owned(),
                        arguments: vec![],
                    },
                ),
            ]),
        ),
        // The fixtures' two `float64.add` nodes carry `toward-zero` and
        // `nearest-even`; QSL admits only the omitted `exact` spelling of a
        // float's rounding (`NodeKind::Ieee`), so `mode` cannot be compared.
        Case {
            identity: "quire.op.ieee.float64.add",
            mode: None,
            with_mode: false,
            package: package(vec![typed(
                "fadd",
                &[
                    ("f", TypeForm::builtin(BuiltinType::Float64, SPAN)),
                    ("g", TypeForm::builtin(BuiltinType::Float64, SPAN)),
                ],
                TypeForm::builtin(BuiltinType::Float64, SPAN),
                binary(BinaryOperator::Add, "f", "g"),
            )]),
        },
    ]
}

/// Fixture identities no row of FR-093's application table lowers in a
/// function body: IEEE `numeric_equal`, integer
/// division and remainder, a decimal `sum` (the `Sum` row writes
/// `sum.integer` only), `reaches`, and the four rows AC-3 excludes because
/// the `Value` family builds no such node in a function body.
const NOT_LOWERED: [&str; 8] = [
    "quire.op.ieee.numeric_equal",
    "quire.op.integer.div",
    "quire.op.integer.rem",
    "quire.op.collection.sum.decimal",
    "quire.op.model.reaches",
    "quire.op.model.dispatch_call",
    "quire.op.model.lookup",
    "quire.op.model.all_instances",
];

/// FR-093-AC-13 / TC-416 step 7: for each application node of QSpec's
/// `positive-operation-identities.json` and
/// `positive-control-operations.json` whose identity a row lowers, the node
/// QSL emits for a function holding that operation equals the fixture node
/// on the AC's members, and IR's v2 reader admits the emitted package.
///
/// Skipped (and passing) when `QSPEC_DIR` is unset; `make conformance`
/// requires it.
#[trace("FR-093-AC-13", "TC-416")]
#[test]
fn conformance_emitted_application_nodes_match_qspec_positive_fixtures() {
    let Some(qspec) = std::env::var_os("QSPEC_DIR") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let directory = std::path::Path::new(&qspec).join("proposals/checked-package-v2/fixtures");
    let cases = cases();
    let mut hit = vec![false; cases.len()];
    let mut emitted = Vec::new();
    for case in &cases {
        let emission = emit(&case.package);
        assert_eq!(
            emission.omitted,
            [],
            "{}: nothing is omitted",
            case.identity
        );
        let read = read_back(&emission);
        assert!(
            matches!(read, Read::Verified { .. }),
            "{}: IR's v2 reader must admit the emitted package, got {read:?}",
            case.identity
        );
        emitted.push(wire(&emission));
    }

    let mut compared_nodes = 0_usize;
    for file in [
        "positive-operation-identities.json",
        "positive-control-operations.json",
    ] {
        for fixture in fixture_applications(&directory.join(file)) {
            let identity = fixture["body"]["operation"]["identity"].as_str().unwrap();
            let mode = fixture["body"]["operation"]["mode"]["value"].as_str();
            if NOT_LOWERED.contains(&identity) {
                continue;
            }
            let (index, case) = cases
                .iter()
                .enumerate()
                .find(|(_, case)| {
                    case.identity == identity && (case.mode.is_none() || case.mode == mode)
                })
                .unwrap_or_else(|| panic!("{file}: no case writes {identity} (mode {mode:?})"));
            hit[index] = true;
            let ours: Vec<&Value> = nodes(&emitted[index])
                .iter()
                .filter(|node| node["body"]["operation"]["identity"] == identity)
                .collect();
            let [ours] = ours.as_slice() else {
                panic!("{identity}: expected one emitted node, got {}", ours.len());
            };
            assert_eq!(
                compared(ours, case.with_mode),
                compared(&fixture, case.with_mode),
                "{file}: {identity} (mode {mode:?})"
            );
            compared_nodes += 1;
        }
    }
    assert!(
        hit.iter().all(|hit| *hit),
        "a case matched no fixture node: {:?}",
        cases
            .iter()
            .zip(&hit)
            .filter(|(_, hit)| !**hit)
            .map(|(case, _)| case.identity)
            .collect::<Vec<_>>()
    );
    println!(
        "conformance: {compared_nodes} emitted application nodes match QSpec's positive fixtures"
    );
}
