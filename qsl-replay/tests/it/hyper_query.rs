// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Hyper self-composition queries": the scripts and refusals of
//! `encode_hyper_query` over ADR-023 section 8's vault.

use ix_trace_rs::trace;
use qsl_replay::{
    encode_hyper_query, HyperQuery, HyperQueryRequest, SmtEncodingLimits, SmtEncodingRefusal,
    SmtQueryKind,
};
use quire_exact::Meter;

use crate::vault_world::{with_body, World, MONITORED, SECURE, UNLIMITED};

fn instance(key: &str) -> Vec<(String, String)> {
    vec![("v".to_owned(), key.to_owned())]
}

/// `encode_hyper_query` over `world`'s clause and subject.
fn query(
    world: &World,
    key: &str,
    kind: SmtQueryKind,
    meter: &mut Meter,
    encoding: SmtEncodingLimits,
) -> Result<HyperQuery, SmtEncodingRefusal> {
    let subjects = [world.subject()];
    let instance = instance(key);
    encode_hyper_query(
        HyperQueryRequest {
            clause: world.clause(),
            subjects: &subjects,
            instance: &instance,
            meter,
            encoding,
        },
        kind,
    )
}

fn script(world: &World, kind: SmtQueryKind) -> String {
    let mut meter = Meter::new(UNLIMITED);
    let query = query(world, "v", kind, &mut meter, SmtEncodingLimits::default())
        .expect("the request encodes");
    String::from_utf8(query.script).expect("a script is UTF-8")
}

/// The golden script `name`, recorded from the encoder of this requirement.
fn golden(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/golden/hyper_monitor/{name}.smt2",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|_| panic!("the golden file {path}"))
}

/// FR-315-AC-43: a body that is not `always` over a state predicate is read
/// through a monitor, and its scripts are byte-equal to the golden scripts
/// recorded from the encoder.
#[trace("TC-1264", "FR-315-AC-43")]
#[test]
fn a_monitored_body_gives_the_recorded_golden_scripts() {
    let world = World::build(&["v"], false, &with_body("", "", MONITORED), &SECURE);
    for (name, kind) in [
        ("secure-base-1", SmtQueryKind::InductionBase { depth: 1 }),
        ("secure-step-1", SmtQueryKind::InductionStep { depth: 1 }),
    ] {
        let first = script(&world, kind);
        assert_eq!(first, script(&world, kind), "{name}: byte-equal twice");
        assert_eq!(first.as_bytes(), golden(name).as_slice(), "{name}");
    }
}
