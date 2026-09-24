// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 checker benchmarks: wall time of one `PackageDeclarations::check`
//! over a whole package.
//!
//! - `checker/chain/<n>`: an `n`-function call chain, `f0 -> f1 -> ... ->
//!   f{n-1}` (QSL-203's shape: the termination check's closure-and-filter
//!   is quadratic in it).
//! - `checker/independent/<n>`: `n` functions with no calls (QSL-205's
//!   F7 shape without QSL-203's: no call edge for termination to close
//!   over).
//!
//! Building the declarations is excluded from the timing
//! (`iter_batched`). Chains of 4,000 and 8,000 take seconds per check, too
//! slow to sample here; `qsl-bench-probe check chain <n>` measures them
//! once each, with peak RSS.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use qsl_bench::check::{call_chain, check, independent};
use std::time::Duration;

const CHAIN: [usize; 3] = [250, 1_000, 2_000];
const INDEPENDENT: [usize; 2] = [1_000, 5_000];

fn chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("checker/chain");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(8));
    for functions in CHAIN {
        group.bench_with_input(
            BenchmarkId::from_parameter(functions),
            &functions,
            |b, &functions| {
                b.iter_batched(|| call_chain(functions), check, BatchSize::LargeInput);
            },
        );
    }
    group.finish();
}

fn independent_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("checker/independent");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    for functions in INDEPENDENT {
        group.bench_with_input(
            BenchmarkId::from_parameter(functions),
            &functions,
            |b, &functions| {
                b.iter_batched(|| independent(functions), check, BatchSize::LargeInput);
            },
        );
    }
    group.finish();
}

criterion_group!(benches, chain, independent_functions);
criterion_main!(benches);
