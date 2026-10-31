// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1358 (QSL-381): a source map read from the wire formats for debug in
//! time and output linear in its size. The wire writes every entry's node id
//! as a tree of its own, so the keys a read builds from it share nothing
//! unless the read interns them; unshared keys make every comparison and
//! lookup walk the whole nested chain.

use std::time::{Duration, Instant};

use qsl_foundation::source::provenance::{OccurrenceKey, PackageSourceMap};
use quire_exact::Origin;

use super::*;
use crate::emit::{admitted_key, admitted_key_in, node_id, KeyInterner};

/// The key of the sum `x + x + ...` of `depth` additions: each key nests the
/// previous, every nested node one shared `Arc`.
fn chain(depth: u64) -> Vec<NodeKey> {
    let mut keys = vec![NodeKey::new("pkg", vec![PathSegment::Text("x".into())], 0).unwrap()];
    for level in 0..depth {
        let next = NodeKey::new(
            "pkg",
            vec![
                PathSegment::Text("add".into()),
                PathSegment::Node(keys[keys.len() - 1].clone()),
            ],
            level,
        )
        .unwrap();
        keys.push(next);
    }
    keys
}

fn map_of(keys: impl IntoIterator<Item = NodeKey>) -> PackageSourceMap {
    let source = qsl_foundation::source::provenance::RawSourceRef::new("a", "unit").unwrap();
    PackageSourceMap::from_entries(keys.into_iter().enumerate().map(|(at, key)| {
        let region = SourceRegion::new(source.clone(), at as u64, at as u64 + 1).unwrap();
        (
            OccurrenceKey::new(key, Origin::new(Role::new("expression"), 0)),
            vec![region],
        )
    }))
    .unwrap()
}

/// The fastest of three runs of `map`'s debug formatting.
fn debug_time(map: &PackageSourceMap) -> (Duration, usize) {
    let mut best = Duration::MAX;
    let mut bytes = 0;
    for _ in 0..3 {
        let start = Instant::now();
        bytes = format!("{map:?}").len();
        best = best.min(start.elapsed());
    }
    (best, bytes)
}

/// The keys of `wire` read the way the I2 read builds them, sharing through
/// `keys`, or each from a fresh interner when `keys` is `None`.
fn read_keys(wire: &[CheckedNodeId], mut keys: Option<&mut KeyInterner>) -> Vec<NodeKey> {
    wire.iter()
        .map(|id| match keys.as_deref_mut() {
            Some(keys) => admitted_key_in(keys, id).unwrap(),
            None => admitted_key(id).unwrap(),
        })
        .collect()
}

/// Entries whose node ids arrive as separate trees format in time within a
/// small factor of the same keys built as one shared chain, and in the same
/// output. Keys read without a shared interner are the case the comparison
/// lookups made quadratic.
#[test]
fn a_read_source_map_formats_in_linear_time_though_the_wire_ids_share_nothing() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            for depth in [500, 1_000, 2_000] {
                let shared = chain(depth);
                let wire: Vec<CheckedNodeId> = shared.iter().map(node_id).collect();
                let mut interner = KeyInterner::default();
                let read = read_keys(&wire, Some(&mut interner));
                assert_eq!(read, shared, "the read keys are the keys the ids name");
                let (shared_time, shared_bytes) = debug_time(&map_of(shared));
                let (read_time, read_bytes) = debug_time(&map_of(read));
                assert_eq!(read_bytes, shared_bytes, "depth {depth}: same output");
                assert!(
                    read_time < shared_time * 20 + Duration::from_millis(20),
                    "depth {depth}: read keys {read_time:?} against shared {shared_time:?}"
                );
                if std::env::var_os("QSL_1358_MEASURE").is_some() {
                    let unshared = read_keys(&wire, None);
                    let (unshared_time, _) = debug_time(&map_of(unshared));
                    eprintln!(
                        "MEASURE depth {depth}: shared {shared_time:?} interned {read_time:?} \
                         unshared {unshared_time:?} bytes {read_bytes}"
                    );
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Counts complete debug output without retaining its bytes.
struct ByteCount(usize);

impl std::fmt::Write for ByteCount {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0 += text.len();
        Ok(())
    }
}

/// The checked package's debug output (its graph's occurrence map,
/// requirement map, semantic graph and every other node-keyed field) grows
/// linearly with the depth of a sum, not with its square.
#[test]
fn the_checked_package_debug_output_grows_linearly_with_depth() {
    use qsl_semantics::check::depth_forms::{declarations, Form};
    use std::fmt::Write;
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let sizes: Vec<usize> = [100, 200, 400]
                .into_iter()
                .map(|levels| {
                    let graph = declarations(Form::Add, levels)
                        .check(CheckingLimits::default())
                        .unwrap_or_else(|refusals| panic!("checks: {refusals:?}"));
                    let package = CheckedPackage::link(graph);
                    let mut count = ByteCount(0);
                    write!(&mut count, "{package:?}").unwrap();
                    count.0
                })
                .collect();
            for pair in sizes.windows(2) {
                assert!(
                    pair[1] < pair[0] * 5 / 2,
                    "each doubling of the depth at most doubles the output: {sizes:?}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
