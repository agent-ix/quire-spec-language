// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-215 benchmark: wall time of one `PackageDeclarations::check` over a
//! Text-reachable recursive cluster at the default checking limits.
//!
//! - `checker/text_cluster/<outcome>/<n>`: `n` records, each with a text
//!   field and an optional field of every other record, and one structural
//!   equality over the first ([`qsl_bench::text_cluster`]). FR-093's leaf
//!   list grows as about `(n - 1)!`.
//!
//! `<outcome>` (`checked` or `refused`) is decided once at setup, as the
//! parser bench does: at the default node ceiling (NFR-011, 100,000) the
//! clusters up to 8 records check and those from 9 refuse on the node
//! ceiling, so a refused size times how long the refusal takes.
//!
//! - `checker/deep_wide/<outcome>/<b>`: the QSL-214 review's long-path
//!   shape ([`qsl_bench::text_cluster::deep_wide`]): a 46-record chain into
//!   a 17-level binary tree, 65,536 text leaves, every field name `b` bytes
//!   long. The leaf count fits the node ceiling; the leaves' key bytes,
//!   charged to the work budget, do not.
//!
//! Building the declarations is excluded from the timing
//! (`iter_batched`).

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use qsl_bench::check::check;
use qsl_bench::text_cluster::{deep_wide, text_cluster, DEEP_WIDE_CHAIN, DEEP_WIDE_LEVELS};
use std::time::Duration;

/// Cluster sizes: QSL-215's 3 to 12.
const RECORDS: std::ops::RangeInclusive<usize> = 3..=12;

fn cluster(c: &mut Criterion) {
    let mut group = c.benchmark_group("checker/text_cluster");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    for records in RECORDS {
        let outcome = if check(text_cluster(records)).is_ok() {
            "checked"
        } else {
            "refused"
        };
        group.bench_with_input(
            BenchmarkId::new(outcome, records),
            &records,
            |b, &records| {
                b.iter_batched(|| text_cluster(records), check, BatchSize::LargeInput);
            },
        );
    }
    group.finish();
}

/// Field-name lengths for the long-path shape, in bytes.
const NAME_BYTES: [usize; 3] = [1, 64, 256];

fn long_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("checker/deep_wide");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    let package = |bytes| deep_wide(DEEP_WIDE_CHAIN, DEEP_WIDE_LEVELS, bytes);
    for bytes in NAME_BYTES {
        let outcome = if check(package(bytes)).is_ok() {
            "checked"
        } else {
            "refused"
        };
        group.bench_with_input(BenchmarkId::new(outcome, bytes), &bytes, |b, &bytes| {
            b.iter_batched(|| package(bytes), check, BatchSize::LargeInput);
        });
    }
    group.finish();
}

criterion_group!(benches, cluster, long_paths);
criterion_main!(benches);
