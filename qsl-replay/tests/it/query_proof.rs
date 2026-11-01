// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-127 (FR-046-AC-5): the checked query-proof boundary classifies a
//! collection representation as unsupported without producing a value or
//! evaluation charge.

use ix_trace_rs::trace;
use qsl_replay::spine::{
    self, query_proof::QueryProofDisposition, CheckedUnit, DependencyInput, ParseRequest,
    SpineLimits,
};
use qsl_replay::SourceIdentity;
use quire_exact::{Cancel, ValueType};

const PACKAGE: &str = "example/config-version";
const MODEL: &[u8] = include_bytes!("../../../examples/config-version/model.semantic-ir.json");

fn checked_collection_query() -> CheckedUnit {
    let source = SourceIdentity::new("agent-ix", "query-proof", "git", "1");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Config = {PACKAGE:?};\n\
         function ints using v(xs: Sequence<Int[0, 1]>[0, 3]): Sequence<Int[0, 1]>[0, 3] pure {{ xs }}\n\
         invariant QueryCandidate using v on Config::ConfigVersion at current {{ \
         contains(map(item in ints(sequence[0, 1]): item), 0) }}\n"
    );
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "query-proof.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("query candidate parses")
    .into_value();
    let packages = qsl_semantics::model::intake::package_input([MODEL])
        .expect("the checked model fixture is valid");
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .expect("model package is admitted")
        .into_value();
    spine::check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect("query clause is checked")
    .into_value()
}

#[trace("TC-127", "FR-046-AC-5")]
#[test]
fn checked_map_collection_is_explicitly_unsupported_at_proof_transfer() {
    let checked = checked_collection_query();
    let result = spine::query_proof::select_query_proof(checked.package(), "QueryCandidate")
        .expect("the clause is selected from the checked package");
    let QueryProofDisposition::Unsupported(unsupported) = result else {
        panic!("checked map collection has no proof transfer: {result:?}");
    };
    assert_eq!(
        unsupported.cause,
        spine::query_proof::QueryProofCause::ValueRepresentation
    );
    assert_eq!(unsupported.query.visit, qsl_semantics::check::Visit::Map);
    assert!(matches!(
        unsupported.query.result_type,
        ValueType::Collection(_)
    ));
    // The disposition is only a typed unsupported cause plus the checked
    // query identity. It has no value, outcome, or partial meter to leak.
}
