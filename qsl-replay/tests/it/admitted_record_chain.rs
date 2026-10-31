// SPDX-License-Identifier: AGPL-3.0-or-later
//! A long chain of declared records compiles through the admitted facade:
//! FR-096's anchor walk visits each declared record once and the FR-092
//! type build checks an open composite by lookup, so the chain's length is
//! bounded only by the limits the caller raises.

use ix_trace_rs::trace;
use qsl_replay::{
    compile_admitted, CompileUnit, DependencyInput, ReplayLimits, SourceIdentity, StageLimits,
};

/// `records` records `record ChainI { next: ChainI+1; }`, the last one's
/// field `Boolean`, and `record Shallow { next: Boolean?; }`.
fn record_chain(records: usize) -> String {
    let mut source = String::from(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n",
    );
    for index in 0..records {
        let payload = if index + 1 == records {
            "Boolean".to_owned()
        } else {
            format!("Chain{}", index + 1)
        };
        source.push_str(&format!("record Chain{index} {{ next: {payload}; }}\n"));
    }
    source.push_str("record Shallow { next: Boolean?; }\n");
    source
}

/// The chain length: CG's TC-029 size in release, a quarter of it in a
/// debug build.
const RECORDS: usize = if cfg!(debug_assertions) {
    16_384
} else {
    65_536
};

/// TC-413 (FR-092-AC-7): a chain of [`RECORDS`] declared records, each one
/// a type-declaration root reaching every later record, compiles admitted
/// under s1 limits raised to fit its source.
#[trace("FR-092-AC-7", "TC-413")]
#[test]
fn a_long_admitted_record_chain_compiles() {
    let source = record_chain(RECORDS);
    let bytes = u64::try_from(source.len()).expect("the source length fits u64");
    let records = u64::try_from(RECORDS).expect("the record count fits u64");
    let stages = StageLimits::default()
        .with_named("s1.input_bytes", bytes)
        .and_then(|limits| limits.with_named("s1.tokens", bytes + 1))
        .and_then(|limits| limits.with_named("s1.nodes", records * 8 + 32))
        .expect("the s1 limits are named");
    let admitted = compile_admitted(
        CompileUnit {
            source: SourceIdentity::new("test", "chain.native", "git", "fixture"),
            package_name: "chain".to_owned(),
            path: "chain.native",
            bytes: source.as_bytes(),
        },
        [],
        &DependencyInput::default(),
        stages,
        ReplayLimits::default().with_input_bytes(bytes),
    );
    if let Err(error) = admitted {
        panic!("the {RECORDS}-record chain compiles: {error:?}");
    }
}
