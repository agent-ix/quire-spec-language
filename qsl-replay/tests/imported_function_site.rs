// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-516 (FR-121-AC-34 fully; the library-name and package-bytes parts of
//! FR-121-AC-32, AC-33, AC-35 and AC-36): the code has no `package_id`, which
//! those ACs state their package identity in, so those tests are tagged
//! `TC-516` only and claim none of AC-32, AC-33, AC-35 or AC-36.
//!
//! `call_site` selects a function a
//! direct import of the unit declares through an [`ImportedFunctionName`],
//! and answers the declaring library, its v2 bytes and the function's
//! `FunctionSite` from the unit's own compile of its dependency input.

use ix_trace_rs::trace;
use qsl_replay::{
    call_site, replay, CallSite, CallSiteRefusal, Code, DeclineCode, DependencyInput, FunctionSite,
    Identifier, ImportedFunctionName, InputSettlement, LibraryName, ProofRefusalCause,
    QualifiedName, ReplayLimits, ReplayRequestWire, ReplayResult, ReplaySource, RequestTargetWire,
    SourceIdentity, StateEnvironment, SuppliedLibrary, TerminalValue, WitnessValue,
};
use qsl_replay::{CanonicalAssignment, EvaluatedValue};
use quire_exact::ScalarLimits;
use std::collections::BTreeMap;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";
const BIG: &str = "function big using v(x: Int[0, 9]): Boolean pure { x > 5 }\n";
const SMALL: &str = "function small using v(x: Int[0, 9]): Boolean pure { x < 2 }\n";
const BOTH: &str = "function both using v(a: Int[0, 9], b: Int[0, 9]): Boolean pure { a < b }\n";
const UNIT_BODY: &str = "function q using v(x: Int[0, 9]): Boolean pure { u::big(x) }\n";

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn id(name: &str) -> Identifier {
    Identifier::new(name).expect("an identifier")
}

fn lib(name: &str) -> LibraryName {
    LibraryName::new(name.to_owned()).expect("a library name")
}

fn function(name: &str) -> QualifiedName {
    QualifiedName::new(vec![id(name)]).expect("a qualified name")
}

fn imported(library: &str, name: &str) -> ImportedFunctionName {
    ImportedFunctionName {
        library: lib(library),
        function: id(name),
    }
}

fn supplied(identity: &str, label: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("a", label),
        path: format!("{label}.native"),
        bytes: format!("{HEADER}{body}").into_bytes(),
    }
}

fn units_source(body: &str) -> String {
    format!("{HEADER}{body}")
}

fn unit_source(import: &str, body: &str) -> String {
    format!("{HEADER}import \"{import}\" as u;\n{body}")
}

fn selected<S: qsl_replay::CallSiteSelection>(
    unit: &str,
    libraries: Vec<SuppliedLibrary>,
    selection: &S,
) -> Result<CallSite<S::Site>, Box<CallSiteRefusal>> {
    call_site(
        SourceIdentity::new("a", "u"),
        "fixture".to_owned(),
        "unit.native",
        unit.as_bytes(),
        [],
        &DependencyInput::new(libraries).expect("a dependency input"),
        selection,
    )
}

/// `test/units`' own source compiled alone, selecting `name` as a unit
/// function: the site FR-121-AC-32 requires the imported selection to equal.
fn standalone(body: &str, name: &str) -> CallSite<FunctionSite> {
    call_site(
        SourceIdentity::new("a", "units"),
        "test/units".to_owned(),
        "units.native",
        units_source(body).as_bytes(),
        [],
        &DependencyInput::default(),
        &function(name),
    )
    .expect("the library compiles alone")
}

fn units() -> SuppliedLibrary {
    supplied("test/units", "units", &format!("{BIG}{SMALL}{BOTH}"))
}

/// `key` as the package bytes spell a node id.
fn wire_node(key: &qsl_replay::NodeKey) -> serde_json::Value {
    serde_json::to_value(qsl_semantics::check::wire_id::declared_id(key).expect("declared node"))
        .expect("a wire id serializes")
}

/// The `package`/`node` of the one `dependency_reference` term in `bytes`.
fn dependency_reference(bytes: &[u8]) -> (String, serde_json::Value) {
    fn find(value: &serde_json::Value, out: &mut Vec<serde_json::Value>) {
        match value {
            serde_json::Value::Object(map) => {
                if map.get("term").and_then(|term| term.as_str()) == Some("dependency_reference") {
                    out.push(value.clone());
                }
                map.values().for_each(|inner| find(inner, out));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|inner| find(inner, out)),
            _ => {}
        }
    }
    let document: serde_json::Value = serde_json::from_slice(bytes).expect("v2 bytes are JSON");
    let mut terms = Vec::new();
    find(&document, &mut terms);
    assert_eq!(
        terms.len(),
        1,
        "q's call lowers to one reference: {terms:?}"
    );
    (
        terms[0]["package"].as_str().expect("package").to_owned(),
        terms[0]["node"].clone(),
    )
}

/// FR-121-AC-32: the site's library, bytes and function equal the
/// standalone compile of the library, and its `library` and `function` equal
/// the `dependency_reference` term of the unit's call; the unit's own
/// `package` differs from the library's.
#[trace("TC-516")]
#[test]
fn an_imported_function_site_equals_the_standalone_library_site() {
    let unit = unit_source("test/units", UNIT_BODY);
    let unit_site = selected(&unit, vec![units()], &function("q")).expect("q");
    let (term_package, term_node) = dependency_reference(&unit_site.package);

    for name in ["big", "small", "both"] {
        let site = selected(&unit, vec![units()], &imported("test/units", name))
            .unwrap_or_else(|refusal| panic!("{name}: {refusal}"));
        let alone = standalone(&format!("{BIG}{SMALL}{BOTH}"), name);
        assert_eq!(site.site.function, alone.site, "{name}");
        assert_eq!(site.site.package, alone.package, "{name}");
        assert_ne!(site.site.package, site.package, "{name}");
        assert_eq!(site.site.library, lib("test/units"));
        let names: Vec<&str> = site
            .site
            .function
            .parameters
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(
            names,
            if name == "both" {
                vec!["a", "b"]
            } else {
                vec!["x"]
            }
        );
        if name == "big" {
            assert_eq!(site.site.library.as_str(), term_package);
            let node = wire_node(&site.site.function.function);
            assert_eq!(node, term_node);
        }
    }
}

/// A request replaying `name` of `test/units` with `assignments`, keyed from
/// the imported `site` alone.
fn library_replay(
    site: &qsl_replay::ImportedFunctionSite,
    name: &str,
    assignments: Vec<CanonicalAssignment>,
) -> qsl_replay::InputArmResult {
    let source = ("a".to_owned(), "units".to_owned());
    let wire = ReplayRequestWire {
        profile_selections: vec![],
        package: qsl_replay::ReplayPackage::descriptor_only(
            &site.library.as_str().to_owned(),
            &vec![source.clone()],
            None,
        ),
        dependencies: Vec::new(),
        replay_target: RequestTargetWire::function(
            function(name),
            site.function.declaration.clone(),
        ),
        source: ReplaySource::Input(assignments),
        obligation_identity: site.function.obligation_identity().clone(),
        backend: "kani-backend-1".to_owned(),
        state_environment: StateEnvironment::new(vec![]),
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![(
            source.0,
            source.1,
            units_source(&format!("{BIG}{SMALL}{BOTH}")).into_bytes(),
        )],
        documents: vec![],
    };
    let ReplayResult::Input(result) = replay(wire, ReplayLimits::default()).expect("replays")
    else {
        panic!("an input-sourced request settles on the input arm");
    };
    result
}

/// FR-121-AC-33: from the imported site alone a request keyed by the
/// library's package name and the site's parameter replays `big(3)` to `false`.
#[trace("TC-516")]
#[test]
fn the_imported_site_alone_keys_a_replay_of_the_library_function() {
    let unit = unit_source("test/units", UNIT_BODY);
    let site = selected(&unit, vec![units()], &imported("test/units", "big")).expect("big");
    let result = library_replay(
        &site.site,
        "big",
        vec![CanonicalAssignment {
            parameter: site.site.function.parameters[0].1.clone(),
            value: WitnessValue::Integer(3),
        }],
    );
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
}

/// FR-121-AC-32, FR-121-AC-33: the ordered parameter keys of a two-parameter
/// function bind the argument of their own name. `both` is `a < b`; with
/// `a = 1` and `b = 5` it is `true`, and with the values on the swapped keys
/// it is `false`, so a site that listed the keys in the wrong order, or
/// paired a name with the other's key, fails here independently of the
/// standalone-compile equality.
#[trace("TC-516")]
#[test]
fn imported_parameter_keys_bind_their_own_arguments_in_declared_order() {
    let unit = unit_source("test/units", UNIT_BODY);
    let site = selected(&unit, vec![units()], &imported("test/units", "both")).expect("both");
    let [(a_name, a_key), (b_name, b_key)] = &site.site.function.parameters[..] else {
        panic!("both declares two parameters");
    };
    assert_eq!((a_name.as_str(), b_name.as_str()), ("a", "b"));
    assert_ne!(a_key, b_key);
    let assign = |a: i128, b: i128| {
        vec![
            CanonicalAssignment {
                parameter: a_key.clone(),
                value: WitnessValue::Integer(a),
            },
            CanonicalAssignment {
                parameter: b_key.clone(),
                value: WitnessValue::Integer(b),
            },
        ]
    };
    for (a, b, holds) in [(1, 5, true), (5, 1, false)] {
        let result = library_replay(&site.site, "both", assign(a, b));
        assert_eq!(
            result.value(),
            Some(EvaluatedValue::Boolean(holds)),
            "a = {a}, b = {b}"
        );
    }
}

/// FR-121-AC-34: a unit function of the same name never answers an imported
/// selection, and the imported one never answers the unit's.
#[trace("TC-516", "FR-121-AC-34")]
#[test]
fn a_same_named_unit_function_is_never_the_imported_site() {
    let body = format!("{UNIT_BODY}function big using v(x: Int[0, 9]): Boolean pure {{ x < 2 }}\n");
    let unit = unit_source("test/units", &body);
    let own = selected(&unit, vec![units()], &function("big")).expect("own big");
    let theirs = selected(&unit, vec![units()], &imported("test/units", "big")).expect("lib big");
    assert_ne!(own.site.function, theirs.site.function.function);
    assert_ne!(own.site.declaration, theirs.site.function.declaration);
    let (_, term_node) = dependency_reference(
        &selected(&unit, vec![units()], &function("q"))
            .expect("q")
            .package,
    );
    assert_eq!(wire_node(&theirs.site.function.function), term_node);
    let alone = standalone(&format!("{BIG}{SMALL}{BOTH}"), "big");
    assert_eq!(theirs.site.function, alone.site);
}

/// FR-121-AC-35: a library the unit does not import directly -- supplied but
/// unimported, absent, or reached only through another library -- refuses
/// `UnknownImport` paired with the unit's package.
#[trace("TC-516")]
#[test]
fn a_library_without_a_direct_import_refuses_unknown_import() {
    let other = supplied("test/other", "other", BIG);
    let unit = unit_source("test/units", UNIT_BODY);
    let wrap = supplied(
        "test/wrap",
        "wrap",
        "import \"test/units\" as w;\nfunction w1 using v(x: Int[0, 9]): Boolean pure { w::big(x) }\n",
    );
    let transitive_unit = unit_source("test/wrap", "function t using v(): Boolean pure { true }\n");
    let cases = [
        (
            unit.clone(),
            vec![units(), other],
            imported("test/other", "big"),
        ),
        (unit, vec![units()], imported("test/absent", "big")),
        (
            transitive_unit,
            vec![units(), wrap],
            imported("test/units", "big"),
        ),
    ];
    for (unit, libraries, selection) in cases {
        let refusal = selected(&unit, libraries, &selection).expect_err("refused");
        let CallSiteRefusal::UnknownImport {
            selection: named,
            package,
        } = &*refusal
        else {
            panic!("{selection}: {refusal:?}");
        };
        assert_eq!(named, &selection);
        assert_eq!(package.as_str(), "fixture");
        assert_eq!(refusal.code(), Code::MissingImport);
        assert!(refusal
            .to_string()
            .starts_with("missing_import/missing-selection"));
        assert_eq!(
            TerminalValue::from_call_site_refusal(&refusal),
            TerminalValue::Declined {
                cause: ProofRefusalCause::InvalidInput,
                code: DeclineCode::Qsl(Code::MissingImport),
            }
        );
    }
}

/// FR-121-AC-36: a function the library does not declare, and one whose
/// signature is not package-independent, refuse paired with the library.
#[trace("TC-516")]
#[test]
fn an_undeclared_or_unimportable_function_refuses_paired_with_the_library() {
    let library = supplied(
        "test/units",
        "units",
        &format!(
            "{BIG}record R {{ a: Boolean; }}\n\
             function pick using v(r: R): Boolean pure {{ true }}\n"
        ),
    );
    let unit = unit_source("test/units", UNIT_BODY);

    let absent = selected(
        &unit,
        vec![library.clone()],
        &imported("test/units", "nope"),
    )
    .expect_err("refused");
    let CallSiteRefusal::UnknownImportedFunction { package, .. } = &*absent else {
        panic!("{absent:?}");
    };
    assert_eq!(package.as_str(), "test/units");
    assert_eq!(absent.code(), Code::MissingDeclaration);
    assert!(absent
        .to_string()
        .starts_with("missing_declaration/missing-name"));

    let unimportable =
        selected(&unit, vec![library], &imported("test/units", "pick")).expect_err("refused");
    let CallSiteRefusal::NotImportable { package, .. } = &*unimportable else {
        panic!("{unimportable:?}");
    };
    assert_eq!(package.as_str(), "test/units");
    assert_eq!(unimportable.code(), Code::IllTyped);
    assert!(unimportable
        .to_string()
        .starts_with("ill_typed/operator-ineligible"));
    for refusal in [&absent, &unimportable] {
        assert!(matches!(
            TerminalValue::from_call_site_refusal(refusal),
            TerminalValue::Declined {
                cause: ProofRefusalCause::InvalidInput,
                ..
            }
        ));
    }
}
