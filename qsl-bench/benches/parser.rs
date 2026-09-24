// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 parser benchmarks: wall time of one `qsl_cst::parse` call.
//!
//! - `parser/depth/<outcome>/<d>`: one function whose body nests `d`
//!   parenthesis pairs. At `89326999` depth 5 and above is refused with
//!   `resource_exhausted`; a refused parse is still timed, because the cost
//!   of reaching the refusal is part of the baseline.
//! - `parser/volume/<outcome>/<n>`: `n` one-line functions. 3,000 is
//!   refused at `89326999` (work budget).
//!
//! `<outcome>` (`admitted`, `recovered` or `refused`) is the input's parse
//! outcome, decided once at setup: when a parser change turns a refusal
//! into a parse (QSL-197), the benchmark gets a new id rather than being
//! compared against the time it took to refuse.
//!
//! Throughput is source bytes, so criterion also reports bytes per second.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qsl_bench::parse::{nested_source, outcome_id, parse, volume_source, ParseOutcome};
use qsl_bench::widen;
use std::hint::black_box;

/// Depths either side of today's refusal boundary (4 parses, 5 refuses).
const DEPTHS: [usize; 7] = [0, 1, 2, 3, 4, 5, 8];

/// Function counts, up to one past today's volume refusal.
const VOLUMES: [usize; 5] = [100, 500, 1_000, 2_000, 3_000];

fn depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser/depth");
    for depth in DEPTHS {
        let text = nested_source(depth);
        let id = outcome_id(&ParseOutcome::of(&parse(&text)), depth);
        group.throughput(Throughput::Bytes(widen(text.len())));
        group.bench_with_input(BenchmarkId::from_parameter(id), &text, |b, text| {
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
        let id = outcome_id(&ParseOutcome::of(&parse(&text)), functions);
        group.throughput(Throughput::Bytes(widen(text.len())));
        group.bench_with_input(BenchmarkId::from_parameter(id), &text, |b, text| {
            b.iter(|| parse(black_box(text)));
        });
    }
    group.finish();
}

criterion_group!(benches, depth, volume);
criterion_main!(benches);
