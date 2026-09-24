// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 evaluator benchmark: cost per call frame.
//!
//! `evaluator/call_chain/<n>` times one `CheckedPackage::call` of `f0(5)`
//! on a checked, linked `n`-function call chain, which pushes `n` call
//! frames. Throughput is `n` elements, so criterion reports frames per
//! second; the per-frame cost is the slope between sizes, which removes
//! the fixed per-call overhead that `n = 1` isolates. Checking and linking
//! the chain happens once, outside the timing.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qsl_bench::check::{call_head, completed_with_five, linked_chain};
use qsl_semantics::model::object_environment::ObjectEnvironment;

const FRAMES: [usize; 4] = [1, 10, 100, 1_000];

fn call_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("evaluator/call_chain");
    let objects = ObjectEnvironment::default();
    for frames in FRAMES {
        let package = linked_chain(frames);
        assert!(
            completed_with_five(&call_head(&package, &objects)),
            "f0(5) on a {frames}-chain evaluates to 5"
        );
        group.throughput(Throughput::Elements(frames as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(frames),
            &package,
            |b, package| {
                b.iter(|| call_head(package, &objects));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, call_chain);
criterion_main!(benches);
