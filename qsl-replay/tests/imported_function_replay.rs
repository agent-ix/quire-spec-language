// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098-AC-32 and FR-098-AC-33 (TC-444 step 12), FR-071's `function_owner`
//! member: a `function` replay request that names the library declaring its
//! function executes against that library's checked package, resolved through
//! the unit's own dependency input, and refuses typed when the owner is not a
//! direct import or the function is not callable. The package-identity clauses
//! of FR-121 are not tagged: the code has no `package_id`.

use ix_trace_rs::trace;
use qsl_replay::{
    call_site, replay, CallSite, CanonicalAssignment, Code, DependencyEntryWire, DependencyInput,
    EnvironmentLabels, EvaluatedValue, FunctionSite, Identifier, ImportedFunctionName,
    ImportedFunctionSite, InputArmResult, InputSettlement, LibraryName, ObjectEnvironmentRef,
    QualifiedName, ReplayLimits, ReplayRefusal, ReplayRequestWire, ReplayResult, ReplaySource,
    RequestTargetWire, SourceIdentity, StateEnvironment, SuppliedLibrary, WitnessValue,
};
use quire_exact::ScalarLimits;
use std::collections::BTreeMap;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";
const BIG: &str = "function big using v(x: Int[0, 9]): Boolean pure { x > 5 }\n";
const BOTH: &str = "function both using v(a: Int[0, 9], b: Int[0, 9]): Boolean pure { a < b }\n";
const PICK: &str = "record R { a: Boolean; }\n\
    function pick using v(r: R): Boolean pure { true }\n";
const UNIT_BODY: &str = "function q using v(x: Int[0, 9]): Boolean pure { u::big(x) }\n";
const UNIT_OWN_BIG: &str = "function big using v(x: Int[0, 9]): Boolean pure { x < 2 }\n";

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

fn source_of(body: &str) -> String {
    format!("{HEADER}{body}")
}

fn unit_source(import: &str, alias: &str, body: &str) -> String {
    format!("{HEADER}import \"{import}\" as {alias};\n{body}")
}

fn library(identity: &str, label: &str, body: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("a", label),
        path: format!("{label}.native"),
        bytes: source_of(body).into_bytes(),
    }
}

fn units_body() -> String {
    format!("{BIG}{BOTH}{PICK}")
}

fn units() -> SuppliedLibrary {
    library("test/units", "units", &units_body())
}

/// The imported site of `name` in `test/units`, located from `unit`.
fn imported(
    unit: &str,
    libraries: Vec<SuppliedLibrary>,
    name: &str,
) -> CallSite<ImportedFunctionSite> {
    call_site(
        SourceIdentity::new("a", "u"),
        "fixture".to_owned(),
        "unit.native",
        unit.as_bytes(),
        [],
        &DependencyInput::new(libraries).expect("a dependency input"),
        &ImportedFunctionName {
            library: lib("test/units"),
            function: id(name),
        },
    )
    .unwrap_or_else(|refusal| panic!("{name}: {refusal}"))
}

/// The unit's own site of `name`.
fn own(unit: &str, libraries: Vec<SuppliedLibrary>, name: &str) -> CallSite<FunctionSite> {
    call_site(
        SourceIdentity::new("a", "u"),
        "fixture".to_owned(),
        "unit.native",
        unit.as_bytes(),
        [],
        &DependencyInput::new(libraries).expect("a dependency input"),
        &function(name),
    )
    .unwrap_or_else(|refusal| panic!("{name}: {refusal}"))
}

/// What a request needs from a located function.
struct Located {
    package: Vec<u8>,
    site: FunctionSite,
}

impl Located {
    fn assign(&self, values: &[i128]) -> Vec<CanonicalAssignment> {
        self.site
            .parameters
            .iter()
            .zip(values)
            .map(|((_, key), value)| CanonicalAssignment {
                parameter: key.clone(),
                value: WitnessValue::Integer(*value),
            })
            .collect()
    }
}

/// A replay request over `unit` (source `a`/`u`) with the supplied
/// `libraries` as its `dependencies`, naming `target` and replaying
/// `located` over `values`.
fn request(
    unit: &str,
    libraries: &[SuppliedLibrary],
    target: RequestTargetWire,
    located: &Located,
    values: &[i128],
) -> ReplayRequestWire {
    let mut dependencies: Vec<DependencyEntryWire> = libraries
        .iter()
        .map(|library| DependencyEntryWire {
            identity: library.identity.clone(),
            sources: vec![("a".to_owned(), library.source.identity.to_owned())],
            refinement_internals: None,
        })
        .collect();
    dependencies.sort_by(|left, right| left.identity.cmp(&right.identity));
    let mut byte_provision = vec![("a".to_owned(), "u".to_owned(), unit.as_bytes().to_vec())];
    byte_provision.extend(libraries.iter().map(|library| {
        (
            "a".to_owned(),
            library.source.identity.to_owned(),
            library.bytes.clone(),
        )
    }));
    ReplayRequestWire {
        profile_selections: vec![],
        package: qsl_replay::ReplayPackage::read(
            &located.package,
            ReplayLimits::default().input_bytes,
        )
        .unwrap(),
        dependencies,
        replay_target: target,
        source: ReplaySource::Input(located.assign(values)),
        obligation_identity: located.site.obligation_identity().clone(),
        backend: "kani-backend-1".to_owned(),
        state_environment: StateEnvironment::new(vec![]),
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents: vec![],
    }
}

fn input_arm(result: Result<ReplayResult, ReplayRefusal>) -> InputArmResult {
    match result.expect("the replay runs") {
        ReplayResult::Input(arm) => arm,
        _ => panic!("an input-sourced request settles on the input arm"),
    }
}

/// An owner-carrying request for `name` of `test/units`, keyed from `site`.
fn owned(site: &CallSite<ImportedFunctionSite>, name: &str) -> RequestTargetWire {
    RequestTargetWire::imported_function(
        site.site.library.clone(),
        function(name),
        site.site.function.declaration.clone(),
    )
}

fn refusal_of(wire: ReplayRequestWire) -> ReplayRefusal {
    replay(wire, ReplayLimits::default()).expect_err("the request refuses")
}

/// FR-098-AC-32: an owner-carrying request executes the library's function
/// against the dependency input and returns the value the function's body
/// gives, for distinct parameter values in both argument orders.
#[trace("FR-098-AC-32", "TC-444")]
#[test]
fn an_owner_carrying_request_executes_the_imported_function() {
    let unit = unit_source("test/units", "u", UNIT_BODY);
    let libraries = [units()];
    let big = imported(&unit, libraries.to_vec(), "big");
    let big_located = Located {
        package: big.package.clone(),
        site: big.site.function.clone(),
    };
    for (x, holds) in [(3, false), (7, true)] {
        let result = input_arm(replay(
            request(&unit, &libraries, owned(&big, "big"), &big_located, &[x]),
            ReplayLimits::default(),
        ));
        assert_eq!(
            result.value(),
            Some(EvaluatedValue::Boolean(holds)),
            "x = {x}"
        );
    }

    let both = imported(&unit, libraries.to_vec(), "both");
    let both_located = Located {
        package: both.package.clone(),
        site: both.site.function.clone(),
    };
    for (a, b, holds) in [(1, 5, true), (5, 1, false)] {
        let result = input_arm(replay(
            request(
                &unit,
                &libraries,
                owned(&both, "both"),
                &both_located,
                &[a, b],
            ),
            ReplayLimits::default(),
        ));
        assert_eq!(
            result.value(),
            Some(EvaluatedValue::Boolean(holds)),
            "a = {a}, b = {b}"
        );
        assert_eq!(
            result.settlement(),
            if holds {
                InputSettlement::Inconclusive
            } else {
                InputSettlement::ReproducedWithoutWitness
            },
            "a = {a}, b = {b}"
        );
    }
}

/// FR-098-AC-32: the unit's own `big` (`x < 2`) never answers an
/// owner-carrying request for the library's `big` (`x > 5`), and the library's
/// never answers the owner-less one.
#[trace("FR-098-AC-32", "TC-444")]
#[test]
fn a_same_named_unit_function_never_answers_the_owner() {
    let unit = unit_source("test/units", "u", &format!("{UNIT_BODY}{UNIT_OWN_BIG}"));
    let libraries = [units()];
    let theirs = imported(&unit, libraries.to_vec(), "big");
    let mine = own(&unit, libraries.to_vec(), "big");
    assert_ne!(mine.site.declaration, theirs.site.function.declaration);
    let theirs_located = Located {
        package: theirs.package.clone(),
        site: theirs.site.function.clone(),
    };
    let mine_located = Located {
        package: mine.package.clone(),
        site: mine.site.clone(),
    };

    let imported_value = input_arm(replay(
        request(
            &unit,
            &libraries,
            owned(&theirs, "big"),
            &theirs_located,
            &[1],
        ),
        ReplayLimits::default(),
    ))
    .value();
    assert_eq!(imported_value, Some(EvaluatedValue::Boolean(false)));

    let own_value = input_arm(replay(
        request(
            &unit,
            &libraries,
            RequestTargetWire::function(function("big"), mine.site.declaration.clone()),
            &mine_located,
            &[1],
        ),
        ReplayLimits::default(),
    ))
    .value();
    assert_eq!(own_value, Some(EvaluatedValue::Boolean(true)));
}

/// FR-098-AC-33: an owner the unit does not import directly -- absent from
/// the lock, or present in it only through another library -- refuses
/// `UnknownOwner` before any call.
#[trace("FR-098-AC-33", "TC-444")]
#[test]
fn an_owner_the_unit_does_not_import_refuses_typed() {
    let unit = unit_source("test/units", "u", UNIT_BODY);
    let libraries = [units()];
    let big = imported(&unit, libraries.to_vec(), "big");
    let located = Located {
        package: big.package.clone(),
        site: big.site.function.clone(),
    };

    let absent = RequestTargetWire::imported_function(
        lib("test/absent"),
        function("big"),
        big.site.function.declaration.clone(),
    );
    let refusal = refusal_of(request(&unit, &libraries, absent, &located, &[3]));
    let ReplayRefusal::UnknownOwner { owner, package } = &refusal else {
        panic!("{refusal:?}");
    };
    assert_eq!(owner.as_str(), "test/absent");
    assert_eq!(package.as_str(), "fixture");
    assert_eq!(refusal.code(), Code::MissingImport);
    assert_eq!(refusal.cause(), Some("missing-selection"));

    // A unit importing only `test/wrap`, which imports `test/units`: the lock
    // carries `test/units`, but the unit does not import it.
    let wrap = library(
        "test/wrap",
        "wrap",
        "import \"test/units\" as w;\n\
         function w1 using v(x: Int[0, 9]): Boolean pure { w::big(x) }\n",
    );
    let transitive = unit_source(
        "test/wrap",
        "u",
        "function t using v(): Boolean pure { true }\n",
    );
    let transitive_libraries = [units(), wrap];
    let mut wire = request(
        &transitive,
        &transitive_libraries,
        owned(&big, "big"),
        &located,
        &[3],
    );
    let transitive_package = own(&transitive, transitive_libraries.to_vec(), "t");
    wire.package = qsl_replay::ReplayPackage::read(
        &transitive_package.package,
        ReplayLimits::default().input_bytes,
    )
    .unwrap();
    let refusal = refusal_of(wire);
    assert!(
        matches!(&refusal, ReplayRefusal::UnknownOwner { owner, .. } if owner.as_str() == "test/units"),
        "{refusal:?}"
    );
}

/// FR-098-AC-33: a name the owner declares no function for, and a function
/// whose signature is not package-independent, refuse typed paired with the
/// owner; the owner-less request for the same name is the unit's, unaffected.
#[trace("FR-098-AC-33", "TC-444")]
#[test]
fn a_function_the_owner_cannot_call_refuses_typed() {
    let unit = unit_source("test/units", "u", UNIT_BODY);
    let libraries = [units()];
    let big = imported(&unit, libraries.to_vec(), "big");
    let located = Located {
        package: big.package.clone(),
        site: big.site.function.clone(),
    };

    let refusal = refusal_of(request(
        &unit,
        &libraries,
        owned(&big, "nope"),
        &located,
        &[3],
    ));
    let ReplayRefusal::UnknownFunction { selection, package } = &refusal else {
        panic!("{refusal:?}");
    };
    assert_eq!(selection, &function("nope"));
    assert_eq!(package.as_str(), "test/units");
    assert_eq!(refusal.code(), Code::MissingDeclaration);

    let refusal = refusal_of(request(
        &unit,
        &libraries,
        owned(&big, "pick"),
        &located,
        &[3],
    ));
    let ReplayRefusal::NotImportable { selection, package } = &refusal else {
        panic!("{refusal:?}");
    };
    assert_eq!(selection, &function("pick"));
    assert_eq!(package.as_str(), "test/units");
    assert_eq!(refusal.code(), Code::IllTyped);
    assert_eq!(refusal.cause(), Some("operator-ineligible"));
}

/// FR-098-AC-33: an owner-carrying request with an object environment is
/// checked against the owner's types, not the unit's. The owner's function is
/// importable only with package-independent types, so it holds no reference
/// parameter and the environment is `UnusedObjectEnvironment`; a function
/// with a reference parameter is `NotImportable` before any environment is
/// admitted. The unit's own `big` takes the same environment as an unused one
/// too, so the refusal is not the unit's package answering.
#[trace("FR-098-AC-33", "TC-444")]
#[test]
fn an_owner_carrying_request_checks_the_environment_against_the_owner() {
    let unit = unit_source("test/units", "u", UNIT_BODY);
    let libraries = [units()];
    let big = imported(&unit, libraries.to_vec(), "big");
    let located = Located {
        package: big.package.clone(),
        site: big.site.function.clone(),
    };
    let environment = || {
        Some(ObjectEnvironmentRef::new(EnvironmentLabels {
            authority: "a".to_owned(),
            identity: "env".to_owned(),
        }))
    };
    let mut wire = request(&unit, &libraries, owned(&big, "big"), &located, &[3]);
    wire.object_environment = environment();
    let refusal = refusal_of(wire);
    assert!(
        matches!(refusal, ReplayRefusal::UnusedObjectEnvironment),
        "{refusal:?}"
    );

    let library = library(
        "test/units",
        "units",
        &format!("{BIG}record R {{ a: Boolean; }}\nfunction pick using v(r: R): Boolean pure {{ true }}\n"),
    );
    let mut wire = request(&unit, &[library], owned(&big, "pick"), &located, &[3]);
    wire.object_environment = environment();
    let refusal = refusal_of(wire);
    assert!(
        matches!(refusal, ReplayRefusal::NotImportable { .. }),
        "{refusal:?}"
    );
}

/// FR-071: no deserializer exists for a replay request, its target or its
/// target members in this repository, so no JSON route can accept
/// `function_owner` until QSpec carries it.
#[trace("FR-071-AC-1")]
#[test]
fn no_serialized_route_accepts_the_function_owner() {
    use std::marker::PhantomData;
    trait Fallback {
        const DESERIALIZES: bool = false;
    }
    struct Probe<T>(PhantomData<T>);
    impl<T> Fallback for Probe<T> {}
    impl<T: serde::de::DeserializeOwned> Probe<T> {
        const DESERIALIZES: bool = true;
    }
    let found = [
        Probe::<RequestTargetWire>::DESERIALIZES,
        Probe::<qsl_replay::ReplayTargetMembers>::DESERIALIZES,
        Probe::<ReplayRequestWire>::DESERIALIZES,
        // Control: the probe does detect a deserializer.
        Probe::<String>::DESERIALIZES,
    ];
    assert_eq!(found, [false, false, false, true]);
}

#[test]
#[trace("TC-444", "FR-098-AC-20")]
#[trace("TC-1325", "FR-350-AC-16")]
fn supplied_dependency_internal_lists_reach_the_library_semantic_owner() {
    let unit = format!("{HEADER}import \"test/a\" as a;\nimport \"test/units\" as u;\n{UNIT_BODY}");
    let libraries = [
        library(
            "test/a",
            "earlier",
            "function unused using v(): Boolean pure { true }",
        ),
        units(),
    ];
    let big = imported(&unit, libraries.to_vec(), "big");
    let located = Located {
        package: big.package.clone(),
        site: big.site.function.clone(),
    };
    let mut wire = request(&unit, &libraries, owned(&big, "big"), &located, &[3]);
    assert_eq!(wire.dependencies[0].identity, "test/a");
    assert_eq!(wire.dependencies[1].identity, "test/units");
    wire.dependencies[1].refinement_internals = Some(
        serde_json::from_value(
            serde_json::json!([{"refinement":"NoSuchRefinement","internal":[]}]),
        )
        .unwrap(),
    );
    let error = refusal_of(wire);
    assert_eq!(
        error.pointer().as_deref(),
        Some("/dependencies/1/refinement_internals/0/refinement")
    );
    let ReplayRefusal::RefinementInternals {
        refusal,
        protocol_results,
        ..
    } = error
    else {
        panic!("the dependency list must reach semantic checking: {error:?}")
    };
    assert!(
        protocol_results.is_none(),
        "no protocol results are synthesized"
    );
    let qsl_replay::CompileRefusal::Dependency { path, refusal } = *refusal else {
        panic!("the refusal names its library owner: {refusal:?}")
    };
    assert_eq!(path[0].as_str(), "test/units");
    let qsl_replay::CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("the library semantic checker refuses: {refusal:?}")
    };
    let qsl_semantics::check::CheckCause::Refinement(refusal) = &refusals[0].cause else {
        panic!("typed refinement list refusal: {refusals:?}")
    };
    let qsl_semantics::check::RefinementCause::InternalList(defect) = &refusal.cause else {
        panic!("typed internal-list defect: {refusal:?}")
    };
    assert_eq!(defect.pointer, "/0/refinement");
}
