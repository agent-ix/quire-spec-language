// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1012 to TC-1017 (FR-361): S3 refuses each model construct a unit
//! writes unless its header selects `quire.model.complete/v1`. Each case
//! compiles real source through the spine over the `test/config-version`
//! domain package: `ConfigVersion` with the operation `attemptUpdate` and the
//! closed population `config_history`.

use std::collections::BTreeSet;

use super::*;
use crate::spine::{CompileRefusal, SuppliedLibrary};
use qsl_semantics::check::{
    AbstractionKey, AbstractionRefusal, AssemblyCause, AssemblyError, AuthoredBinding, CheckCause,
    FrameBinding, MalformedEntry, ModelFamily, ObjectBinding, OperationKey, OwnerKind,
    PopulationBinding, ProfileCause, RustField, RustPath, RustReceiver,
};
use qsl_semantics::value::{CatalogRole, DefinitionLock, HeaderLayer, LayerForm};

const CORE: &str = "quire.state.core/v1";
const QUERIES: &str = "quire.state.queries/v1";
const GRAPH: &str = "quire.state.graph/v1";
const VALUE: &str = "quire.value.complete/v1";
const MODEL: &str = "quire.model.complete/v1";

const BODY: &str = "body ok using v on Config::ConfigVersion::size() { 1 }";
const CALL: &str = "pre c using v on Config::ConfigVersion::probe { self.size() > 0 }";
const NOOP: &str = "function noop using v(): Boolean pure { true }";
const ABSTRACTION: &str = "abstraction A using v {\n\
    type Config::ConfigVersion to \"store::Version\" { versionNumber to \"number\"; };\n\
    population Config::config_history to \"store::history\";\n\
    operation Config::ConfigVersion::attemptUpdate to \"store::update\" receiver \"self\" ();\n\
    }";
const LOOKUP: &str = "function q using v(p: Population<Config::ConfigVersion>[1], \
                      r: Config::ConfigVersion): Config::ConfigVersion pure \
                      { lookup<Config::ConfigVersion>(p, r) absent refused }";

/// The `size` operation: no parameters, returns an `Integer`.
fn size_operation() -> serde_json::Value {
    let identity = format!("{}/size", config_version_type());
    json!({
        "identity": identity,
        "name": "size",
        "params": [],
        "returns": {
            "typeRef": "ix://quire/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [],
        "post": [],
        "origin": {"source": {
            "sourceIdentity": format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/spec"),
            "path": "spec.qspec",
            "startLine": 1,
            "startColumn": 1,
        }},
        "frame": {"modifies": [], "creates": [], "deletes": []},
    })
}

/// The header of a unit selecting `profiles` (alias, identity) and the
/// `Config` model, with the packages that supply it.
fn header(profiles: &[(&str, &str)], imports: &str) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let document = config_version_domain_document_with_operations(vec![
        size_operation(),
        config_version_probe_operation(),
    ]);
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let profiles: String = profiles
        .iter()
        .map(|(alias, identity)| format!("profile {alias} = \"{identity}\";\n"))
        .collect();
    let header = format!(
        "language \"ix:native\" edition \"1-draft\";\n{profiles}{imports}\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?} version \"1.0.0\" \
         digest \"sha256-jcs:{}\";\n",
        hex(digest)
    );
    (header, packages)
}

fn compile(
    profiles: &[(&str, &str)],
    declarations: &str,
) -> (String, Result<ComposedUnit, Box<CompileRefusal>>) {
    let (header, packages) = header(profiles, "");
    let unit = format!("{header}{declarations}\n");
    let compiled = crate::spine::compose(
        source(),
        "gate.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    );
    (unit, compiled)
}

/// The assembler errors of an assembly refusal.
fn errors(refusal: CompileRefusal) -> Vec<AssemblyError> {
    match refusal {
        CompileRefusal::Assembly { refusal, .. } => refusal.errors,
        other => panic!("an assembly refusal, not {other:?}"),
    }
}

/// The unit's refusals under `profiles`, with the text each is located at.
fn refusals(profiles: &[(&str, &str)], declarations: &str) -> Vec<(AssemblyError, String)> {
    let (unit, compiled) = compile(profiles, declarations);
    errors(*compiled.expect_err("the unit refuses"))
        .into_iter()
        .map(|error| {
            let located = unit[error.span.start..error.span.end].to_owned();
            (error, located)
        })
        .collect()
}

/// The one model-gate refusal a unit holds, and the text it is located at.
fn gate(
    profiles: &[(&str, &str)],
    declarations: &str,
) -> (qsl_semantics::check::ModelGateRefusal, String) {
    let found = refusals(profiles, declarations);
    let [(error, located)] = found.as_slice() else {
        panic!("one refusal, not {found:?}");
    };
    let AssemblyCause::UnselectedModelConstruct(refusal) = error.cause.clone() else {
        panic!("a model-gate refusal, not {error:?}");
    };
    (refusal, located.clone())
}

/// FR-361-AC-1: a value-profile unit with a `model` declaration and one
/// `body` refuses once at the body declaration, retaining the family, the
/// owning declaration, the selected layers and the required layer; the same
/// unit under the model profile compiles and the operation has its body.
#[trace("FR-361-AC-1", "TC-1012")]
#[test]
fn an_operation_body_needs_the_model_profile() {
    let (refusal, located) = gate(&[("v", VALUE)], BODY);
    assert_eq!(located, BODY);
    assert_eq!(refusal.family, ModelFamily::OperationBody);
    assert_eq!(refusal.owner_kind, OwnerKind::OperationBody);
    assert_eq!(refusal.owner, "ok");
    assert_eq!(refusal.selected, [HeaderLayer::ValueComplete]);
    assert_eq!(refusal.required_layer(), HeaderLayer::ModelComplete);
    let error = &refusals(&[("v", VALUE)], BODY)[0].0;
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "unsupported_construct/declaration-form"
    );

    let with_call = format!("{BODY}\n{CALL}");
    let (_, compiled) = compile(&[("v", MODEL)], &with_call);
    let compiled = compiled.expect("the model profile admits the body");
    let tables = compiled.package.graph().dispatch_tables();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].candidate_count(), 1, "the operation has a body");
}

/// FR-361-AC-2: a value-profile unit that calls `receiver.operation(args)`
/// refuses once at the call with the family `dispatch-call`, and one that
/// holds `lookup<T>(p, r) absent m` refuses once at the expression with the
/// family `lookup`; under the model profile both compile.
#[trace("FR-361-AC-2", "TC-1013")]
#[test]
fn a_dispatched_call_and_a_lookup_need_the_model_profile() {
    let (call_refusal, located) = gate(&[("v", VALUE)], CALL);
    assert_eq!(located, "self.size()");
    assert_eq!(call_refusal.family, ModelFamily::DispatchCall);
    assert_eq!(call_refusal.owner_kind, OwnerKind::StateClause);
    assert_eq!(call_refusal.owner, "c");
    assert_eq!(
        refusals(&[("v", VALUE)], CALL)[0]
            .0
            .cause
            .catalog_code()
            .to_string(),
        "unsupported_construct/expression-form"
    );

    let (lookup_refusal, located) = gate(&[("v", VALUE)], LOOKUP);
    assert_eq!(
        located,
        "lookup<Config::ConfigVersion>(p, r) absent refused"
    );
    assert_eq!(lookup_refusal.family, ModelFamily::Lookup);
    assert_eq!(
        refusals(&[("v", VALUE)], LOOKUP)[0]
            .0
            .cause
            .catalog_code()
            .to_string(),
        "unsupported_construct/expression-form"
    );
    assert_eq!(lookup_refusal.owner_kind, OwnerKind::Function);
    assert_eq!(lookup_refusal.owner, "q");

    let (_, compiled) = compile(&[("v", MODEL)], &format!("{BODY}\n{CALL}"));
    let compiled = compiled.expect("the model profile admits the call");
    let tables = compiled.package.graph().dispatch_tables();
    assert_eq!(tables.len(), 1, "the call resolves to size's table");
    assert_eq!(tables[0].candidate_count(), 1);
    compile(&[("v", MODEL)], LOOKUP)
        .1
        .expect("the model profile admits the lookup");
}

/// FR-361-AC-5: a unit with a `model` declaration and `pre`, `post` and
/// `invariant` clauses against its alias compiles under each of the four
/// layers that are not the model layer.
#[trace("FR-361-AC-5", "TC-1016")]
#[test]
fn state_clauses_need_no_model_profile() {
    let clauses = "invariant I using v on Config::ConfigVersion at current { true }\n\
                   pre P using v on Config::ConfigVersion::attemptUpdate { true }\n\
                   post Q using v on Config::ConfigVersion::attemptUpdate { true }";
    for identity in [CORE, QUERIES, GRAPH, VALUE] {
        compile(&[("v", identity)], clauses)
            .1
            .unwrap_or_else(|refusal| panic!("{identity} refuses a state clause: {refusal:?}"));
    }
}

/// FR-361-AC-6: a value-profile unit holding a construct of every family
/// refuses once per construct, in source order, each retaining the selected
/// layers and the required layer, though nothing uses a declaration.
#[trace("TC-1017")]
#[test]
fn every_construct_is_refused_once_in_source_order() {
    let declarations = format!("{LOOKUP}\n{ABSTRACTION}\n{BODY}\n{CALL}");
    let found = refusals(&[("v", VALUE), ("c", CORE)], &declarations);
    let seen: Vec<_> = found
        .iter()
        .map(|(error, located)| {
            let AssemblyCause::UnselectedModelConstruct(refusal) = &error.cause else {
                panic!("a model-gate refusal, not {error:?}");
            };
            (refusal.family, located.as_str(), refusal.selected.clone())
        })
        .collect();
    let layers = vec![HeaderLayer::ValueComplete, HeaderLayer::StateCore];
    assert_eq!(
        seen,
        [
            (
                ModelFamily::Lookup,
                "lookup<Config::ConfigVersion>(p, r) absent refused",
                layers.clone()
            ),
            (ModelFamily::Abstraction, ABSTRACTION, layers.clone()),
            (ModelFamily::OperationBody, BODY, layers.clone()),
            (ModelFamily::DispatchCall, "self.size()", layers),
        ]
    );
}

/// The importing unit's text, importing `test/lib` as `l`.
fn library(profiles: &[(&str, &str)], declarations: &str) -> SuppliedLibrary {
    let (header, _) = header(profiles, "");
    SuppliedLibrary {
        identity: "test/lib".to_owned(),
        source: qsl_foundation::SourceIdentity::new("agent-ix", "gate-lib", "git", "1"),
        path: "lib.native".to_owned(),
        bytes: format!("{header}{declarations}\n").into_bytes(),
    }
}

fn compile_importing(
    unit_profiles: &[(&str, &str)],
    unit_declarations: &str,
    library: SuppliedLibrary,
) -> (String, Result<ComposedUnit, Box<CompileRefusal>>) {
    let (header, packages) = header(unit_profiles, "import \"test/lib\" as l;\n");
    let unit = format!("{header}{unit_declarations}\n");
    let compiled = crate::spine::compose(
        source(),
        "gate.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::new(vec![library]).expect("an admissible dependency input"),
        SpineLimits::default(),
    );
    (unit, compiled)
}

/// FR-361-AC-6: the test is per unit. A library that selects the model
/// profile does not give its importer the selection, and the reverse. The
/// importer's forms against the library's import alias are not tested
/// (FR-361-AC-6 is not bound).
#[trace("TC-1017")]
#[test]
fn the_selection_belongs_to_the_unit() {
    let (unit, compiled) = compile_importing(
        &[("v", VALUE)],
        BODY,
        library(&[("v", MODEL)], &format!("{NOOP}\n{BODY}")),
    );
    let found = errors(*compiled.expect_err("the importer lacks the selection"));
    let [error] = found.as_slice() else {
        panic!("one refusal, not {found:?}");
    };
    assert!(matches!(
        error.cause,
        AssemblyCause::UnselectedModelConstruct(_)
    ));
    assert_eq!(&unit[error.span.start..error.span.end], BODY);

    let (_, compiled) = compile_importing(
        &[("v", MODEL)],
        BODY,
        library(&[("v", VALUE)], &format!("{NOOP}\n{BODY}")),
    );
    match *compiled.expect_err("the library lacks the selection") {
        CompileRefusal::Dependency { refusal, .. } => match *refusal {
            CompileRefusal::Assembly { refusal, .. } => {
                assert!(matches!(
                    refusal.errors[0].cause,
                    AssemblyCause::UnselectedModelConstruct(_)
                ));
            }
            other => panic!("an assembly refusal, not {other:?}"),
        },
        other => panic!("a dependency refusal, not {other:?}"),
    }
}

/// FR-361-AC-7: the selection is the header and nothing else. A unit holding
/// a `model` declaration, a supplied domain package and a model construct
/// whose header does not name the model identity refuses, whatever it names
/// its alias.
#[trace("FR-361-AC-7", "TC-1017")]
#[test]
fn the_selection_is_the_header_and_nothing_else() {
    for alias in ["v", "mc", "complete"] {
        let body = format!("body ok using {alias} on Config::ConfigVersion::size() {{ 1 }}");
        let (refusal, _) = gate(&[(alias, VALUE)], &body);
        assert_eq!(refusal.family, ModelFamily::OperationBody);
    }
    assert!(!refusals(&[("v", VALUE)], BODY).is_empty());
}

/// FR-361-AC-8: a header profile naming the model identity resolves, and the
/// emitted lock's `definition_selections` hold the model layer and the layers
/// of its `requires` closure, the complete-value lock's package-selection
/// rows and the complete-model lock's `always` rows, and no more.
#[trace("FR-361-AC-8", "TC-1017")]
#[test]
fn the_model_layer_selects_its_closure_and_both_locks_rows() {
    let (_, compiled) = compile(&[("v", MODEL)], NOOP);
    let compiled = compiled.expect("the model profile compiles");
    let wire: serde_json::Value = serde_json::from_slice(compiled.emitted.bytes()).unwrap();
    let selected: BTreeSet<String> = wire["lock"]["definition_selections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["identity"].as_str().unwrap().to_owned())
        .collect();
    let lock = DefinitionLock::pinned();
    let mut expected: BTreeSet<String> = HeaderLayer::ALL
        .map(|layer| layer.identity().to_owned())
        .into();
    expected.extend(
        lock.always_roles()
            .iter()
            .filter(|role| **role != CatalogRole::Edition)
            .map(|role| lock.entry(*role).identity.to_owned()),
    );
    expected.extend(
        lock.model_catalog()
            .iter()
            .map(|entry| entry.identity.to_owned()),
    );
    assert_eq!(selected, expected);
}

/// FR-361-AC-9: in a unit that holds model constructs, a header profile
/// naming a `qualification_catalog` row refuses
/// `unknown_profile`/`wrong-selection-role`, and one held by no catalog
/// `unknown_profile`/`unsupported-selection`; the E3 refusal is the only one.
#[trace("FR-361-AC-9", "TC-1017")]
#[test]
fn an_e3_refusal_is_the_only_one_reported() {
    for (identity, cause) in [
        (
            "quire.value.ieee754-2019-default/v1",
            ProfileCause::WrongSelectionRole,
        ),
        ("test:unknown-profile", ProfileCause::UnsupportedSelection),
    ] {
        let (_, compiled) = compile(&[("v", VALUE), ("p", identity)], BODY);
        match *compiled.expect_err("E3 refuses") {
            CompileRefusal::Profile { refusals, .. } => {
                let [refusal] = refusals.as_slice() else {
                    panic!("one E3 refusal, not {refusals:?}");
                };
                assert_eq!(refusal.cause, cause, "{identity}");
            }
            other => panic!("an E3 profile refusal, not {other:?}"),
        }
    }
}

/// FR-361-AC-11: in a value-profile unit S2 builds the forms of every family
/// and reports no refusal; the refusals are S3's. Verifies three families
/// (`abstraction` has no form), so FR-361-AC-11 is not bound.
#[trace("TC-1017")]
#[test]
fn the_refusals_are_s3s_not_s2s() {
    let declarations = format!("{LOOKUP}\n{BODY}\n{CALL}");
    let (_, compiled) = compile(&[("v", VALUE)], &declarations);
    assert!(matches!(
        *compiled.expect_err("S3 refuses"),
        CompileRefusal::Assembly { .. }
    ));
}

/// FR-361-AC-12: a `body ... using V` where `V` resolves to
/// `quire.value.complete/v1` in a unit that selects only that profile
/// receives the one family refusal; the same declaration in a unit that also
/// selects the model profile through another alias receives FR-110's layer
/// refusal alone.
#[trace("FR-361-AC-12", "TC-1017")]
#[test]
fn one_refusal_per_construct() {
    let found = refusals(&[("v", VALUE)], BODY);
    assert_eq!(found.len(), 1);
    assert!(matches!(
        found[0].0.cause,
        AssemblyCause::UnselectedModelConstruct(_)
    ));

    let found = refusals(&[("v", VALUE), ("m", MODEL)], BODY);
    let [(error, located)] = found.as_slice() else {
        panic!("one refusal, not {found:?}");
    };
    assert_eq!(located, BODY);
    assert_eq!(
        error.cause,
        AssemblyCause::UnsupportedLayerForm {
            layer: HeaderLayer::ValueComplete,
            form: LayerForm::OperationBody,
        }
    );
}

/// FR-361 item 7: `allInstances`, field navigation and a `Population<T>`
/// parameter are not model constructs. A value-profile unit holding them
/// raises no model-gate refusal and compiles; so does the same unit under the
/// model profile.
#[test]
fn forms_no_gate_covers_stay_ungated() {
    let declarations = "\
        function total using v(p: Population<Config::ConfigVersion>[1]): Integer pure \
        { size(allInstances<Config::ConfigVersion>(p)) }\n\
        invariant Navigates using v on Config::ConfigVersion at current { self.versionNumber >= 0 }\n\
        pre P using v on Config::ConfigVersion::probe { self.versionNumber >= 0 }";
    for identity in [VALUE, MODEL] {
        compile(&[("v", identity)], declarations)
            .1
            .unwrap_or_else(|refusal| panic!("{identity} refuses an ungated form: {refusal:?}"));
    }
}

/// FR-361-AC-13: `deref` and `reaches` are `state.graph`'s. Under an alias
/// that resolves to `quire.value.complete/v1` each is refused once by
/// FR-110's layer rule naming that layer and retaining no model family, also
/// in a unit whose header selects `quire.state.graph/v1` through a second
/// alias; under a graph or model alias each compiles.
#[trace("FR-361-AC-13", "TC-1015")]
#[test]
fn object_navigation_is_not_a_value_layer_form() {
    let deref = "invariant D using a on Config::ConfigVersion at current \
                 { present(self.parent) implies deref(value(self.parent)).versionNumber >= 0 }";
    let reaches = "invariant R using a on Config::ConfigVersion at current \
                   { not reaches(self, self, parent) }";
    for declaration in [deref, reaches] {
        for profiles in [vec![("a", VALUE)], vec![("a", VALUE), ("g", GRAPH)]] {
            let found = refusals(&profiles, declaration);
            let [(error, _)] = found.as_slice() else {
                panic!("one refusal, not {found:?}");
            };
            assert_eq!(
                error.cause,
                AssemblyCause::UnsupportedLayerForm {
                    layer: HeaderLayer::ValueComplete,
                    form: LayerForm::ObjectNavigation,
                }
            );
        }
        for identity in [GRAPH, MODEL] {
            compile(&[("a", identity)], declaration)
                .1
                .unwrap_or_else(|refusal| panic!("{identity} refuses navigation: {refusal:?}"));
        }
    }
}

/// FR-361-AC-3, FR-304-AC-8: a value-profile unit holding an `abstraction`
/// declaration refuses once at the declaration, `unsupported_construct`/
/// `declaration-form`, family `abstraction`; under the model profile the same
/// source compiles to the package's relation, the one `check_abstraction`
/// gives the same bindings (FR-304-AC-1), located at the declaration.
#[trace("FR-361-AC-3", "FR-304-AC-8", "TC-1014", "TC-811")]
#[test]
fn an_abstraction_declaration_needs_the_model_profile() {
    let (refusal, located) = gate(&[("v", VALUE)], ABSTRACTION);
    assert_eq!(located, ABSTRACTION);
    assert_eq!(refusal.family, ModelFamily::Abstraction);
    assert_eq!(refusal.owner_kind, OwnerKind::Abstraction);
    assert_eq!(refusal.owner, "A");
    assert_eq!(
        refusals(&[("v", VALUE)], ABSTRACTION)[0]
            .0
            .cause
            .catalog_code()
            .to_string(),
        "unsupported_construct/declaration-form"
    );

    let (_, compiled) = compile(&[("v", MODEL)], ABSTRACTION);
    let compiled = compiled.expect("the model profile admits the declaration");
    let relation = compiled
        .package
        .abstraction()
        .expect("the package carries the relation");
    assert_eq!(relation.len(), 3);
    assert_eq!(relation.sites().len(), 1);

    // The relation FR-304-AC-1 gives: the keys written out from the names the
    // source writes (`Config::ConfigVersion`, `Config::config_history`,
    // `Config::ConfigVersion::attemptUpdate`) in the domain package, not read
    // back from the compiled relation.
    let object = qsl_semantics::model::key::DeclarationKey {
        package: CONFIG_VERSION_PACKAGE_IDENTITY.to_owned(),
        node: config_version_type(),
    };
    let population = qsl_semantics::model::key::DeclarationKey {
        package: CONFIG_VERSION_PACKAGE_IDENTITY.to_owned(),
        node: config_version_population_identity(),
    };
    let ident = |name: &str| quire_exact::Identifier::new(name).expect("an identifier");
    let authored = [
        AuthoredBinding::Object {
            key: object.clone(),
            value: ObjectBinding {
                rust_type: RustPath::new("store::Version"),
                fields: BTreeMap::from([(ident("versionNumber"), RustField::new("number"))]),
            },
        },
        AuthoredBinding::Population {
            key: population,
            value: PopulationBinding {
                collection: RustPath::new("store::history"),
            },
        },
        AuthoredBinding::Frame {
            key: OperationKey {
                declaring: object,
                operation: ident("attemptUpdate"),
            },
            value: FrameBinding {
                function: RustPath::new("store::update"),
                receiver: RustReceiver::SelfValue,
                parameters: BTreeMap::new(),
            },
        },
    ];
    let expected = compiled
        .package
        .graph()
        .check_abstraction(&authored)
        .expect("the bindings check")
        .with_sites(relation.sites().to_vec());
    assert_eq!(*relation, expected);
    assert_eq!(
        relation
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>(),
        authored
            .iter()
            .map(AuthoredBinding::key)
            .collect::<Vec<_>>()
    );
}

/// The abstraction refusals of a model-profile unit, in order, with the text
/// the first is located at.
fn abstraction_refusals(declarations: &str) -> (Vec<AbstractionRefusal>, String) {
    let (unit, compiled) = compile(&[("v", MODEL)], declarations);
    let CompileRefusal::Check { refusals, region } = *compiled.expect_err("the unit refuses")
    else {
        panic!("a check refusal");
    };
    let causes = refusals
        .iter()
        .map(|refusal| match &refusal.cause {
            CheckCause::Abstraction(cause) => (**cause).clone(),
            other => panic!("an abstraction refusal, not {other:?}"),
        })
        .collect();
    let region = region.expect("the first refusal is located");
    let (start, end) = (
        usize::try_from(region.start()).expect("a start offset"),
        usize::try_from(region.end()).expect("an end offset"),
    );
    (causes, unit[start..end].to_owned())
}

const OBJECT_A: &str =
    "abstraction A using v { type Config::ConfigVersion to \"store::Version\"; }";

/// FR-304-AC-7: two declarations of one unit check into one relation with a
/// site for each; a second binding of one key refuses `invalid_model_binding`/
/// `conflicting-binding` whether its value is equal or different, naming both
/// bindings and the key, at the second declaration.
#[trace("FR-304-AC-7", "TC-810")]
#[test]
fn two_abstraction_declarations_make_one_relation() {
    let population = "abstraction B using v { population Config::config_history to \"store::h\"; }";
    let (_, compiled) = compile(&[("v", MODEL)], &format!("{OBJECT_A}\n{population}"));
    let compiled = compiled.expect("two declarations check");
    let relation = compiled.package.abstraction().expect("one relation");
    assert_eq!(relation.len(), 2);
    assert_eq!(relation.sites().len(), 2);

    for (value, label) in [("store::Version", "equal"), ("store::Other", "different")] {
        let again =
            format!("abstraction B using v {{ type Config::ConfigVersion to \"{value}\"; }}");
        let (found, located) = abstraction_refusals(&format!("{OBJECT_A}\n{again}"));
        let [AbstractionRefusal::ConflictingBinding { key, first, second }] = &found[..] else {
            panic!("{label}: one conflicting binding: {found:?}");
        };
        assert!(matches!(key, AbstractionKey::Object(_)), "{label}");
        let rust_type = |binding: &AuthoredBinding| match binding {
            AuthoredBinding::Object { value, .. } => value.rust_type.as_str().to_owned(),
            other => panic!("an object binding, not {other:?}"),
        };
        assert_eq!(rust_type(first), "store::Version", "{label}");
        assert_eq!(rust_type(second), value, "{label}");
        assert_eq!(first.key(), *key, "{label}");
        assert_eq!(second.key(), *key, "{label}");
        assert_eq!(found[0].code().as_str(), "invalid_model_binding", "{label}");
        assert_eq!(found[0].cause(), "conflicting-binding", "{label}");
        assert_eq!(located, "B", "{label}: the second declaration");
    }
}

/// FR-304-AC-3, QSpec FR-450 rule 1: a name that resolves to no declaration
/// of its model refuses `missing_declaration`/`missing-name` at the name,
/// naming the owning domain package; an operation its type does not declare
/// refuses the same.
#[trace("FR-304-AC-3")]
#[test]
fn an_abstraction_binding_naming_nothing_refuses_at_the_name() {
    let declaration = "abstraction A using v { type Config::Absent to \"store::Version\"; }";
    let (unit, compiled) = compile(&[("v", MODEL)], declaration);
    let CompileRefusal::Check { refusals, region } = *compiled.expect_err("the name is absent")
    else {
        panic!("a check refusal");
    };
    assert_eq!(
        refusals[0]
            .cause
            .catalog_code()
            .expect("a catalog pair")
            .to_string(),
        "missing_declaration/missing-name"
    );
    let CheckCause::Abstraction(cause) = &refusals[0].cause else {
        panic!("an abstraction refusal");
    };
    let AbstractionRefusal::UnresolvedName { package, name, .. } = &**cause else {
        panic!("an unresolved name: {cause:?}");
    };
    assert_eq!(package.as_deref(), Some(CONFIG_VERSION_PACKAGE_IDENTITY));
    assert_eq!(name, "Config::Absent");
    let region = region.expect("the refusal is located");
    let (start, end) = (
        usize::try_from(region.start()).expect("a start offset"),
        usize::try_from(region.end()).expect("an end offset"),
    );
    assert_eq!(&unit[start..end], "Config::Absent");

    let (found, _) = abstraction_refusals(
        "abstraction A using v { operation Config::ConfigVersion::nope to \"f\" receiver \"self\" (); }",
    );
    assert!(
        matches!(&found[..], [AbstractionRefusal::MissingDeclaration { .. }]),
        "{found:?}"
    );
    assert_eq!(found[0].cause(), "missing-name");
}

/// QSpec FR-450 rule 2: a name that resolves to a declaration of another kind
/// refuses `invalid_model_binding`/`malformed-declaration`, not
/// `missing-name`.
#[test]
fn a_name_of_another_kind_refuses_malformed_declaration() {
    for (declaration, entry) in [
        (
            "abstraction A using v { population Config::ConfigVersion to \"store::h\"; }",
            MalformedEntry::NotAPopulation,
        ),
        (
            "abstraction A using v { type Config::config_history to \"store::T\"; }",
            MalformedEntry::NotAnObjectType,
        ),
    ] {
        let (found, _) = abstraction_refusals(declaration);
        let [AbstractionRefusal::MalformedDeclaration {
            entry: found_entry, ..
        }] = &found[..]
        else {
            panic!("{declaration}: one malformed declaration: {found:?}");
        };
        assert_eq!(*found_entry, entry, "{declaration}");
        assert_eq!(found[0].code().as_str(), "invalid_model_binding");
        assert_eq!(found[0].cause(), "malformed-declaration");
    }
}

/// QSpec FR-450 rule 4: a field bound twice refuses `invalid_model_binding`/
/// `conflicting-binding`, naming both field bindings and the field; a
/// parameter bound twice is `malformed-declaration`.
#[test]
fn a_field_or_parameter_bound_twice_refuses() {
    let (found, located) = abstraction_refusals(
        "abstraction A using v { type Config::ConfigVersion to \"store::T\" \
         { versionNumber to \"a\"; versionNumber to \"b\"; }; }",
    );
    let [AbstractionRefusal::ConflictingField {
        field,
        first,
        second,
        first_span,
        second_span,
        ..
    }] = &found[..]
    else {
        panic!("one conflicting field: {found:?}");
    };
    assert_eq!(field.as_str(), "versionNumber");
    assert_eq!((first.as_str(), second.as_str()), ("a", "b"));
    assert!(first_span.start < second_span.start);
    assert_eq!(found[0].code().as_str(), "invalid_model_binding");
    assert_eq!(found[0].cause(), "conflicting-binding");
    assert_eq!(located, "A");

    let (found, _) = abstraction_refusals(
        "abstraction A using v { operation Config::ConfigVersion::attemptUpdate to \"f\" \
         receiver \"self\" (n to \"a\", n to \"b\"); }",
    );
    let [AbstractionRefusal::MalformedDeclaration {
        entry: MalformedEntry::DuplicateEntry(name),
        ..
    }] = &found[..]
    else {
        panic!("one duplicate parameter: {found:?}");
    };
    assert_eq!(name.as_str(), "n");
    assert_eq!(found[0].cause(), "malformed-declaration");
}

/// FR-304 "Refusals": refusals come in source order of the bindings across the
/// unit's declarations, each located at its own declaration.
#[test]
fn abstraction_refusals_come_in_source_order() {
    let first = "abstraction A using v { type Config::config_history to \"store::T\"; }";
    let second = "abstraction B using v { type Config::Absent to \"store::T\"; }";
    let (found, located) = abstraction_refusals(&format!("{second}\n{first}"));
    assert!(
        matches!(
            &found[..],
            [
                AbstractionRefusal::UnresolvedName { .. },
                AbstractionRefusal::MalformedDeclaration { .. }
            ]
        ),
        "{found:?}"
    );
    assert_eq!(located, "Config::Absent");
    let (found, _) = abstraction_refusals(&format!("{first}\n{second}"));
    assert!(
        matches!(
            &found[..],
            [
                AbstractionRefusal::MalformedDeclaration { .. },
                AbstractionRefusal::UnresolvedName { .. }
            ]
        ),
        "{found:?}"
    );
    assert_eq!(located_declaration(&format!("{first}\n{second}")), "A");
}

/// The text the first refusal of `declarations` is located at, when that
/// refusal names no span of its own.
fn located_declaration(declarations: &str) -> String {
    abstraction_refusals(declarations).1
}

/// FR-304: an `abstraction` declaration's `using` alias names one of the
/// unit's profile selections.
#[test]
fn an_abstraction_declaration_using_no_profile_refuses_at_the_alias() {
    let declaration = "abstraction A using missing { type Config::ConfigVersion to \"store::T\"; }";
    let found = refusals(&[("v", MODEL)], declaration);
    assert!(
        found.iter().any(|(error, located)| {
            matches!(&error.cause, AssemblyCause::UndeclaredAlias { alias } if alias == "missing")
                && located == "missing"
        }),
        "{found:?}"
    );
}

/// FR-304-AC-8, TC-811 steps 3 and 4: a declaration with no binding, and a
/// binding with an unquoted Rust path, refuse `invalid_syntax` at the
/// offending token, under either profile.
#[trace("FR-304-AC-8", "TC-811")]
#[test]
fn a_malformed_abstraction_declaration_refuses_invalid_syntax_at_the_token() {
    for (declaration, offending) in [
        ("abstraction A using v { }", "}"),
        (
            "abstraction A using v { type Config::ConfigVersion to store::T; }",
            "store",
        ),
    ] {
        for profile in [VALUE, MODEL] {
            let (unit, compiled) = compile(&[("v", profile)], declaration);
            let CompileRefusal::Source(diagnostic) = *compiled.expect_err("the unit refuses")
            else {
                panic!("{declaration}: a source refusal");
            };
            assert_eq!(diagnostic.code, qsl_foundation::Code::InvalidSyntax);
            let region = diagnostic.region.expect("the refusal is located");
            let at = unit
                .find(declaration)
                .expect("the declaration is in the unit")
                + declaration.find(offending).expect("the token is in it");
            assert_eq!(
                usize::try_from(region.start()).expect("an offset"),
                at,
                "{declaration} under {profile}"
            );
        }
    }
}
