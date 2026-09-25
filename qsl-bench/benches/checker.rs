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
//! - `checker/enum_members/<m>`: 1,000 functions over one enum of `m`
//!   members, each resolving a member by name and checking an enum
//!   equality (QSL-205: a per-function cost growing with `m` shows here).
//!
//! - `checker/object_chain/<n>`: admitting an `n`-type generalization
//!   chain, one field per type, as the checker's `TypeEnvironment` under
//!   the default limits (QSL-57: the flattened output is `n(n+1)/2` slots,
//!   and admission work grows with it, not faster). 5,000 types exceed the
//!   default `work_units` budget, so that row times the refusal, and
//!   `checker/object_chain_unbudgeted/5000` times the full admission.
//!
//! Building the declarations is excluded from the timing
//! (`iter_batched`). Chains of 4,000 and 8,000 take seconds per check, too
//! slow to sample here; `qsl-bench-probe check chain <n>` measures them
//! once each, with peak RSS.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use qsl_bench::check::{
    admit_object_types, call_chain, check, enum_binding, enum_members, independent, object_chain,
};
use qsl_semantics::value::declaration::DEFAULT_WORK_UNITS;
use std::time::Duration;

const CHAIN: [usize; 3] = [250, 1_000, 2_000];
const INDEPENDENT: [usize; 2] = [1_000, 5_000];
const ENUM_FUNCTIONS: usize = 1_000;
const ENUM_MEMBERS: [usize; 3] = [10, 100, 1_000];
const OBJECT_CHAIN: [usize; 3] = [500, 1_000, 5_000];

fn chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("checker/chain");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(8));
    for functions in CHAIN {
        assert!(
            check(call_chain(functions)).is_ok(),
            "{functions} functions check, so the timing is of a completed check"
        );
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
        assert!(
            check(independent(functions)).is_ok(),
            "{functions} functions check, so the timing is of a completed check"
        );
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

fn enum_member_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("checker/enum_members");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    for members in ENUM_MEMBERS {
        let binding = enum_binding(members);
        assert!(
            check(enum_members(ENUM_FUNCTIONS, &binding)).is_ok(),
            "{members} members check, so the timing is of a completed check"
        );
        group.bench_with_input(
            BenchmarkId::from_parameter(members),
            &binding,
            |b, binding| {
                b.iter_batched(
                    || enum_members(ENUM_FUNCTIONS, binding),
                    check,
                    BatchSize::LargeInput,
                );
            },
        );
    }
    group.finish();
}

fn object_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("checker/object_chain");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    for depth in OBJECT_CHAIN {
        group.bench_with_input(BenchmarkId::from_parameter(depth), &depth, |b, &depth| {
            b.iter_batched(
                || object_chain(depth),
                |types| admit_object_types(types, DEFAULT_WORK_UNITS),
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
    let mut group = c.benchmark_group("checker/object_chain_unbudgeted");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    let depth = 5_000;
    assert!(
        admit_object_types(object_chain(depth), u64::MAX).is_ok(),
        "an unbudgeted 5,000-type chain admits, so the timing is of a completed admission"
    );
    group.bench_with_input(BenchmarkId::from_parameter(depth), &depth, |b, &depth| {
        b.iter_batched(
            || object_chain(depth),
            |types| admit_object_types(types, u64::MAX),
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    chain,
    independent_functions,
    enum_member_counts,
    object_chains
);
criterion_main!(benches);
