// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 CST identity benchmarks: what building a CST, identities
//! included, costs against the source bytes it covers.
//!
//! Since QSL-200 a parse computes one identity digest, over the
//! document-revision labels, whatever its node count; node identities are
//! fixed-size values and reuse is decided on request. The former
//! `cst/sha256/<input>` rows timed the per-node digests QSL-200 removed and
//! are gone with them.
//!
//! - `cst/parse/<input>`: wall time of one `qsl_cst::parse` call.
//!   Throughput is the input's source bytes.
//!
//! Inputs: the nesting sources at depth 0..=4 (about 250 source bytes
//! each) and 1,000 one-line functions. `qsl-bench-probe cst` prints their
//! node counts.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qsl_bench::parse::{nested_source, parse, volume_source};
use std::hint::black_box;

fn inputs() -> Vec<(String, String)> {
    let mut inputs: Vec<(String, String)> = (0..=4)
        .map(|depth| (format!("nested-{depth}"), nested_source(depth)))
        .collect();
    inputs.push(("volume-1000".to_owned(), volume_source(1_000)));
    inputs
}

fn cst_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("cst");
    for (label, text) in inputs() {
        let parsed = parse(&text).expect("every CST bench input parses at 89326999");
        assert!(parsed.is_admissible(), "{label} is admissible");
        group.throughput(Throughput::Bytes(
            u64::try_from(text.len()).expect("source bytes fit in u64"),
        ));
        group.bench_with_input(BenchmarkId::new("parse", &label), &text, |b, text| {
            b.iter(|| parse(black_box(text)));
        });
    }
    group.finish();
}

criterion_group!(benches, cst_parse);
criterion_main!(benches);
