// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042-AC-2: literals typed by full signed-64 and exact-rational domains
//! reach native emission and the independent reader without loss or an
//! alternate spelling. These tests inspect emitted numbers, not evaluation.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{
    native, wire as w, ExactInteger, ExactRational, Limits, ProtocolNumber,
};
use setup::{Inputs, Unit};

/// Adjacent magnitudes are coprime. These pairs exercise both numerator
/// endpoints and the admitted denominator ceiling without reduction hiding it.
const RATIONAL_BOUNDARIES: [(i64, i64); 2] = [(i64::MAX, i64::MAX - 1), (i64::MIN, i64::MAX)];

const BOUNDED: &str = "predicate Bounded using G (wide: M::Wide, exact: M::Exact): Boolean {
    wide >= -9223372036854775808
    and wide <= 9223372036854775807
    and exact <= rational(9223372036854775807, 9223372036854775806)
    and exact >= rational(-9223372036854775808, 9223372036854775807)
}";

const FLOW: &str = "protocol Flow using P over (view: M::Node) on origin {
    role Service on M::Node;
    run check Ready using G { Bounded(view.wide, view.exact) };
    finish Closed as (closed: M::Node) { true };
}";

/// Select the authored predicate by name; its values are its own arena.
fn bounded(package: &w::Package) -> &w::Declaration {
    package
        .declarations
        .iter()
        .find(|declaration| declaration.name == "Bounded")
        .expect("authored signed-64 predicate")
}

fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for declaration in report.declarations() {
        assert_eq!(
            report.types().disposition(declaration.declaration()),
            Some(TypeDisposition::Typed),
            "declaration {:?}: {:?}",
            declaration.declaration(),
            report
                .types()
                .declaration(declaration.declaration())
                .map(|typed| typed.causes()),
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
    }
}

/// Collect every emitted number of one declaration from its own value arena.
/// Handles from other arenas are never mixed into this selection.
fn numbers(declaration: &w::Declaration) -> Vec<ProtocolNumber> {
    declaration
        .values
        .iter()
        .filter_map(|value| match &value.operation {
            w::ValueOperation::Number { value } => {
                Some(value.checked().expect("canonical emitted number"))
            }
            _ => None,
        })
        .collect()
}

#[test]
#[trace("TC-121", "FR-042-AC-2")]
fn signed64_extrema_and_ceiling_rational_survive_native_emission_and_independent_read() {
    let inputs = Inputs::with_signed64_domains(&[
        Unit {
            name: "numeric-domains",
            body: BOUNDED,
            declarations: &["Bounded"],
        },
        Unit {
            name: "numeric-protocol",
            body: FLOW,
            declarations: &["Flow"],
        },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let admitted = native::admit(proofs, selected, Limits::default())
                .into_result()
                .expect("full signed-64 and exact-rational literal domains");
            let package = admitted.package();
            let declaration = bounded(package);
            assert_eq!(
                numbers(declaration),
                [
                    ProtocolNumber::Integer(ExactInteger::new(i64::MIN)),
                    ProtocolNumber::Integer(ExactInteger::new(i64::MAX)),
                    ProtocolNumber::Rational(
                        ExactRational::new(RATIONAL_BOUNDARIES[0].0, RATIONAL_BOUNDARIES[0].1)
                            .unwrap()
                    ),
                    ProtocolNumber::Rational(
                        ExactRational::new(RATIONAL_BOUNDARIES[1].0, RATIONAL_BOUNDARIES[1].1)
                            .unwrap()
                    ),
                ],
                "the authored endpoints reach the wire unnarrowed"
            );

            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .expect("canonical encoding of signed-64 endpoints");
            // The selected profile spells every number as a tagged decimal
            // string, so no endpoint can degrade to a bare JSON number.
            let bytes = std::str::from_utf8(emitted.bytes()).expect("canonical UTF-8 output");
            for spelling in [
                "\"decimal\":\"-9223372036854775808\"",
                "\"decimal\":\"9223372036854775807\"",
                "\"numerator\":\"9223372036854775807\",\"denominator\":\"9223372036854775806\"",
                "\"numerator\":\"-9223372036854775808\",\"denominator\":\"9223372036854775807\"",
            ] {
                assert!(
                    bytes.contains(spelling),
                    "missing exact spelling {spelling}"
                );
            }

            let read = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independent reader admits the full signed-64 domains");
            assert_eq!(read.package(), package);
            assert_eq!(read.digest(), emitted.digest());
            assert_eq!(numbers(bounded(read.package())), numbers(declaration));
        },
    );
}
