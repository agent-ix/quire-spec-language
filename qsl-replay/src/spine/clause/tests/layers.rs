// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-490 (FR-110-AC-1, AC-7, AC-8) and FR-341-AC-6: a header profile selects
//! any header-selectable layer, the lock holds the layer's `requires`
//! closure, and S3 admits a declaration under exactly its layer's
//! admitted-form set. Each case compiles real source through the spine over
//! the `test/nodes` domain package.

use std::collections::BTreeSet;

use super::*;
use crate::spine::{classify_refusal, CompileRefusal, RefinementClass};
use qsl_foundation::Code;
use qsl_semantics::check::{AssemblyCause, AssemblyError};
use qsl_semantics::value::{DefinitionLock, HeaderLayer, LayerForm};

const CORE: &str = "quire.state.core/v1";
const QUERIES: &str = "quire.state.queries/v1";
const GRAPH: &str = "quire.state.graph/v1";
const VALUE: &str = "quire.value.complete/v1";
const MODEL: &str = "quire.model.complete/v1";

const PREDICATE: &str = "predicate Id using v(b: Boolean): Boolean { b }\n";

/// A unit selecting the `test/nodes` model as `M` and `profiles` (alias,
/// identity), holding `declarations`.
fn unit(profiles: &[(&str, &str)], declarations: &str) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let document = domain_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let headers: String = profiles
        .iter()
        .map(|(alias, identity)| format!("profile {alias} = \"{identity}\";\n"))
        .collect();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n{headers}\
         model M = {PACKAGE_IDENTITY:?} version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         {declarations}",
        hex(digest)
    );
    (unit, packages)
}

fn compile(
    profiles: &[(&str, &str)],
    declarations: &str,
) -> (String, Result<ComposedUnit, Box<CompileRefusal>>) {
    let (unit, packages) = unit(profiles, declarations);
    let compiled = crate::spine::compose(
        source(),
        "layers.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    );
    (unit, compiled)
}

/// The identities the compiled lock's `definition_selections` hold.
fn selected(compiled: &ComposedUnit) -> BTreeSet<String> {
    let wire: serde_json::Value = serde_json::from_slice(compiled.emitted.bytes()).unwrap();
    wire["lock"]["definition_selections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["identity"].as_str().unwrap().to_owned())
        .collect()
}

/// The unit's one assembler error, and the source text it is located at.
fn layer_refusal(
    profiles: &[(&str, &str)],
    declarations: &str,
) -> (CompileRefusal, AssemblyError, String) {
    let (unit, compiled) = compile(profiles, declarations);
    let refusal = *compiled.expect_err("the layer refuses the form");
    let CompileRefusal::Assembly { refusal: found, .. } = &refusal else {
        panic!("an assembly refusal, not {refusal:?}");
    };
    let [error] = found.errors.as_slice() else {
        panic!("one error, not {found:?}");
    };
    let error = error.clone();
    let located = unit[error.span.start..error.span.end].to_owned();
    (refusal, error, located)
}

/// FR-110-AC-1: a unit whose one header profile names `quire.value.complete/v1`
/// compiles; its lock holds that layer, `quire.state.core/v1` and the
/// qualification rows the package-selection rules select, and no other layer.
#[trace("TC-490", "FR-110-AC-1")]
#[test]
fn the_value_layer_selects_state_core_and_the_qualification_rows() {
    let (_, compiled) = compile(
        &[("v", VALUE)],
        "invariant Any using v on M::Node at current { true }\n",
    );
    let selected = selected(&compiled.expect("the value layer compiles"));
    let lock = DefinitionLock::pinned();
    let mut expected: BTreeSet<String> = [CORE, VALUE].map(str::to_owned).into();
    expected.extend(
        lock.always_roles()
            .iter()
            .filter(|role| **role != qsl_semantics::value::CatalogRole::Edition)
            .map(|role| lock.entry(*role).identity.to_owned()),
    );
    assert_eq!(selected, expected);
}

/// FR-110-AC-7: for each of the five layers, a unit whose one header profile
/// names the layer and whose declarations use only that layer's forms
/// compiles, and its lock holds the layer and every layer its `requires`
/// closure names and no layer that requires it.
#[trace("TC-490", "FR-110-AC-7")]
#[test]
fn each_layer_compiles_its_own_forms_and_locks_its_closure() {
    let cases: [(&str, &str); 5] = [
        (CORE, "invariant Any using v on M::Node at current { true }\n"),
        (
            QUERIES,
            "predicate Id using v(b: Boolean): Boolean { b }\n\
             invariant Any using v on M::Node at current { Id(true) }\n",
        ),
        (
            GRAPH,
            "predicate Id using v(b: Boolean): Boolean { b }\n\
             invariant NoCycle using v on M::Node at current { Id(not reaches(self, self, next)) }\n",
        ),
        (
            VALUE,
            "function f using v(n: Integer): Boolean pure { n > 0 }\n\
             invariant Any using v on M::Node at current { true }\n",
        ),
        (
            MODEL,
            "function f using v(n: Integer): Boolean pure { n > 0 }\n\
             invariant NoCycle using v on M::Node at current { not reaches(self, self, next) }\n",
        ),
    ];
    let lock = DefinitionLock::pinned();
    for (identity, declarations) in cases {
        let layer = HeaderLayer::from_identity(identity).expect("a layer");
        let (_, compiled) = compile(&[("v", identity)], declarations);
        let selected = selected(
            &compiled
                .unwrap_or_else(|refusal| panic!("{identity} refuses its own forms: {refusal:?}")),
        );
        let closure = lock.closure(&[layer]);
        for candidate in HeaderLayer::ALL {
            assert_eq!(
                selected.contains(candidate.identity()),
                closure.contains(&candidate),
                "{identity}: the lock's {}",
                candidate.identity()
            );
        }
    }
}

/// FR-110-AC-8: a unit naming `quire.state.core/v1` whose declaration calls a
/// named predicate refuses `unsupported_construct`/`expression-form` at the
/// call's span naming the layer; the same unit naming `quire.state.queries/v1`
/// compiles.
#[trace("TC-490", "FR-110-AC-8")]
#[test]
fn a_named_predicate_call_is_outside_state_core() {
    let call = "invariant Calls using c on M::Node at current { Id(true) }\n";
    let profiles = |layer| [("c", layer), ("v", VALUE)];
    let (refusal, error, located) = layer_refusal(&profiles(CORE), &format!("{PREDICATE}{call}"));
    assert_eq!(located, "Id(true)");
    assert_eq!(
        error.cause,
        AssemblyCause::UnsupportedLayerForm {
            layer: HeaderLayer::StateCore,
            form: LayerForm::NamedPredicateCall,
        }
    );
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "unsupported_construct/expression-form"
    );
    assert_eq!(refusal.code(), Code::UnsupportedConstruct);
    let (_, compiled) = compile(&profiles(QUERIES), &format!("{PREDICATE}{call}"));
    compiled.expect("state queries admits a named predicate call");
}

/// FR-110-AC-8: a unit naming `quire.state.core/v1` that declares a named
/// predicate refuses `unsupported_construct`/`declaration-form` at the
/// declaration's span; the same unit naming `quire.state.queries/v1`
/// compiles.
#[trace("TC-490", "FR-110-AC-8")]
#[test]
fn a_named_predicate_declaration_is_outside_state_core() {
    let (refusal, error, located) = layer_refusal(&[("v", CORE)], PREDICATE);
    assert_eq!(located, PREDICATE.trim_end());
    assert_eq!(
        error.cause,
        AssemblyCause::UnsupportedLayerForm {
            layer: HeaderLayer::StateCore,
            form: LayerForm::PredicateDeclaration,
        }
    );
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "unsupported_construct/declaration-form"
    );
    assert_eq!(refusal.code(), Code::UnsupportedConstruct);
    let (_, compiled) = compile(&[("v", QUERIES)], PREDICATE);
    compiled.expect("state queries admits a named predicate declaration");
}

/// FR-110-AC-8: object navigation is the graph layer's, so a state clause
/// under state queries that writes `reaches` refuses at that expression, and
/// the graph layer admits it.
#[trace("TC-490", "FR-110-AC-8")]
#[test]
fn reachability_is_outside_state_queries() {
    let clause =
        "invariant NoCycle using v on M::Node at current { not reaches(self, self, next) }\n";
    let (_, error, located) = layer_refusal(&[("v", QUERIES)], clause);
    assert_eq!(located, "reaches(self, self, next)");
    assert_eq!(
        error.cause,
        AssemblyCause::UnsupportedLayerForm {
            layer: HeaderLayer::StateQueries,
            form: LayerForm::ObjectNavigation,
        }
    );
    compile(&[("v", GRAPH)], clause)
        .1
        .expect("the graph layer admits reaches");
}

/// FR-341-AC-6: a layer refusal is a profile-prohibited form, so the compile
/// result classifies `prohibited` with the one `unsupported_construct` cause.
#[trace("TC-861", "FR-341-AC-6")]
#[test]
fn a_layer_refusal_is_prohibited() {
    let (refusal, _, _) = layer_refusal(&[("v", CORE)], PREDICATE);
    let classified = classify_refusal(&refusal);
    assert_eq!(classified.class, RefinementClass::Prohibited);
    assert_eq!(
        classified.causes,
        [crate::spine::RefusalCause {
            code: Code::UnsupportedConstruct,
            cause: None,
            limit: None,
        }]
    );
}
