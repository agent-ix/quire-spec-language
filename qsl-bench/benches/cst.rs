// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 CST identity-hashing benchmarks: what building a CST's node
//! identities costs against the source bytes it covers.
//!
//! `qsl_cst::parse` builds every node's `StableNodeId` from a SHA-256
//! preimage that carries the node's whole source slice and its ancestor
//! production names (`LosslessCst::new`), so the bytes hashed per source
//! byte grow with nesting. Two measurements, over the same inputs:
//!
//! - `cst/parse/<input>`: wall time of one `qsl_cst::parse` call (hashing
//!   included). Throughput is the input's **hashed** bytes.
//! - `cst/sha256/<input>`: wall time of SHA-256 alone over a buffer of
//!   that input's hashed-byte count, cut into one message per CST node.
//!   The ratio of the two is the hashing share of the parse.
//!
//! Inputs: the nesting sources at depth 0..=4 (about 250 source bytes
//! each, the deepest that parse at `89326999`) and 1,000 one-line
//! functions. `qsl-bench-probe cst` prints the byte counts.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qsl_bench::parse::{nested_source, parse, volume_source};
use sha2::{Digest, Sha256};
use std::hint::black_box;

fn inputs() -> Vec<(String, String)> {
    let mut inputs: Vec<(String, String)> = (0..=4)
        .map(|depth| (format!("nested-{depth}"), nested_source(depth)))
        .collect();
    inputs.push(("volume-1000".to_owned(), volume_source(1_000)));
    inputs
}

fn identity_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("cst");
    for (label, text) in inputs() {
        let parsed = parse(&text).expect("every CST bench input parses at 89326999");
        assert!(parsed.is_admissible(), "{label} is admissible");
        // Since QSL-200 a parse hashes only the document-revision preimage,
        // once, whatever the node count.
        let hashed = parsed.cst().identity_hashed_bytes();
        group.throughput(Throughput::Bytes(
            u64::try_from(hashed).expect("hashed bytes fit in u64"),
        ));
        group.bench_with_input(BenchmarkId::new("parse", &label), &text, |b, text| {
            b.iter(|| parse(black_box(text)));
        });
        let buffer = vec![0x5a_u8; hashed];
        group.bench_with_input(BenchmarkId::new("sha256", &label), &buffer, |b, buffer| {
            b.iter(|| black_box(Sha256::digest(black_box(buffer))));
        });
    }
    group.finish();
}

criterion_group!(benches, identity_hashing);
criterion_main!(benches);
