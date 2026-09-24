// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 parser benchmarks: wall time of one `qsl_cst::parse` call.
//!
//! - `parser/depth/<d>`: one function whose body nests `d` parenthesis
//!   pairs. At `89326999` depth 5 and above is refused with
//!   `resource_exhausted`; a refused parse is still timed, because the cost
//!   of reaching the refusal is part of the baseline. `qsl-bench-probe
//!   parse` records which depths parse.
//! - `parser/volume/<n>`: `n` one-line functions. 3,000 is refused at
//!   `89326999` (work budget); the probe records it.
//!
//! Throughput is source bytes, so criterion also reports bytes per second.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qsl_bench::parse::{nested_source, parse, volume_source};
use std::hint::black_box;

/// Depths either side of today's refusal boundary (4 parses, 5 refuses).
const DEPTHS: [usize; 7] = [0, 1, 2, 3, 4, 5, 8];

/// Function counts, up to one past today's volume refusal.
const VOLUMES: [usize; 5] = [100, 500, 1_000, 2_000, 3_000];

fn depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser/depth");
    for depth in DEPTHS {
        let text = nested_source(depth);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(depth), &text, |b, text| {
            b.iter(|| parse(black_box(text)));
        });
    }
    group.finish();
}

fn volume(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser/volume");
    group.sample_size(20);
    for functions in VOLUMES {
        let text = volume_source(functions);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(functions), &text, |b, text| {
            b.iter(|| parse(black_box(text)));
        });
    }
    group.finish();
}

criterion_group!(benches, depth, volume);
criterion_main!(benches);
