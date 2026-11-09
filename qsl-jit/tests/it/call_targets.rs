// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561-AC-3: the call targets of every generated block, from the code
//! generator's relocation records.

use ix_trace_rs::trace;
use qsl_eval::execution::{ExecutionBackend, PrepareLimits};
use qsl_jit::{
    check_call_targets, CallTarget, IndirectSource, Jit, RelocationKind, StrayCall, StrayCallKind,
};
use quire_exact::Cancel;

use crate::common::{package_of, select};

/// Every block the JIT prepares over the 10,000 generated units calls a
/// `KernelTable` entry or a block of the same prepared value, and at least
/// one block calls a table entry.
#[trace("TC-1383", "FR-561-AC-3")]
#[test]
fn every_generated_block_calls_only_kernel_table_entries_and_its_own_blocks() {
    let units = crate::common::generated_units();
    assert_eq!(units.len(), 10_000);
    let mut calling_the_table = 0;
    let mut blocks = 0;
    for (unit, package) in units {
        let functions: Vec<&str> = unit
            .functions
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        let prepared = Jit
            .prepare(
                package,
                &select(&functions),
                &PrepareLimits::default(),
                &Cancel::new(),
            )
            .unwrap_or_else(|failure| panic!("unit {} prepares: {failure:?}", unit.index));
        let records = prepared.call_targets_for_test();
        blocks += records.len();
        calling_the_table += check_call_targets(records)
            .unwrap_or_else(|stray| panic!("unit {}: {stray:?}", unit.index));
        for block in records {
            for record in &block.direct {
                match (&record.target, record.kind) {
                    (CallTarget::KernelTable(_), RelocationKind::Absolute)
                    | (CallTarget::Block(_), RelocationKind::Relative) => {}
                    other => panic!("unit {} {}: {other:?}", unit.index, block.block),
                }
            }
            assert!(block
                .indirect
                .iter()
                .all(|source| *source == IndirectSource::TableSlot));
        }
    }
    assert!(blocks > 0);
    assert!(
        calling_the_table > 0,
        "{blocks} blocks, none call the table"
    );
}

const SOURCE: &str = "\
 function twice using v(a: Int[0, 9]): Integer pure { a + a }\n\
 function wide using v(a: Integer, b: Integer): Integer pure { a * b }\n";

fn stray_of(kind: StrayCallKind) -> StrayCall {
    let package = package_of(SOURCE);
    let prepared = Jit
        .prepare_with_stray_call_for_test(
            &package,
            &select(&["twice", "wide"]),
            &PrepareLimits::default(),
            &Cancel::new(),
            kind,
        )
        .expect("the test code generator's unit prepares");
    check_call_targets(prepared.call_targets_for_test())
        .expect_err("the check fails on a stray call")
}

/// A test code generator that emits one direct call to an address outside
/// the table and the blocks, and one that emits an indirect call through a
/// register, each make the check fail, naming the block and the target.
#[trace("TC-1383", "FR-561-AC-3")]
#[test]
fn a_stray_direct_or_indirect_call_fails_the_check_naming_block_and_target() {
    let direct = stray_of(StrayCallKind::Direct);
    assert!(direct.block.starts_with("qsl_f"), "{direct:?}");
    assert_eq!(direct.target, "qsl_jit_stray");
    let indirect = stray_of(StrayCallKind::Indirect);
    assert!(indirect.block.starts_with("qsl_f"), "{indirect:?}");
    assert!(indirect.target.contains("register"), "{indirect:?}");
}
