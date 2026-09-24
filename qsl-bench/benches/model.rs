// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 model-layer benchmarks over an N-type `DomainPackage` built
//! through FR-154 model intake (`qsl_bench::model` documents the shape).
//! Every stage is timed alone, on inputs prepared outside the timing, so
//! each stage's figure is its own cost:
//!
//! - `model/intake/{admit,read_records}/<n>`: the two intake calls.
//!   `model/parse/{serde_json,semantic_ir}/<n>` time one bare parse of the
//!   same bytes by each of the two JSON parsers intake runs -- F13's
//!   comparison point (`admit` parses once with `serde_json`,
//!   `read_records` once with `agent-ix-semantic-ir` and once more with
//!   `serde_json`).
//! - `model/normalize/<n>`: FR-150 normalization under unlimited limits.
//! - `model/object_universe_of/<n>`: the unmetered rebuild F5 names
//!   (`normalize::build` under `UNLIMITED`), called alone.
//! - `model/admit_binding/<n>` and `model/admit_invocation/<n>`: FR-153
//!   admission of a fixed 100-member population, directly and as one
//!   unchanged invocation (pre and post). F5's comparison: admission
//!   against `object_universe_of`, and invocation against binding.
//! - `model/all_instances/<target>/<depth>`: `allInstances` over 1,000
//!   members whose type has `depth` proper ancestors, querying the root
//!   type (`target = root`, a full ancestor walk per member) or the
//!   members' own type (`target = own`, no walk) -- F6's O(N x A).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qsl_bench::model::{
    self, admit_population, admit_unchanged_invocation, chain_type, intake, offer,
    population_document, view, ModelShape, PACKAGE,
};
use qsl_semantics::model::intake::{admit, read_records};
use qsl_semantics::model::key::SHA256_JCS_DIGEST_DOMAIN;
use qsl_semantics::model::normalize::object_universe_of;
use qsl_semantics::model::population::{all_instances, AdmissionOutcome, AllInstancesOutcome};
use quire_exact::Meter;
use std::hint::black_box;
use std::time::Duration;

/// Object types per package (each also declares one field record).
const TYPES: [usize; 3] = [250, 1_000, 4_000];

/// Supertype-chain depth for the size sweep (F6 sweeps depth separately).
const SWEEP_DEPTH: usize = 4;

/// Population members admitted in the admission benchmarks.
const ADMITTED_MEMBERS: usize = 100;

/// Population members queried by `allInstances`, and the package size.
const QUERY_MEMBERS: usize = 1_000;
const QUERY_TYPES: usize = 1_000;
const QUERY_DEPTHS: [usize; 4] = [1, 8, 32, 120];

fn shape(types: usize) -> ModelShape {
    ModelShape {
        types,
        depth: SWEEP_DEPTH,
    }
}

fn intake_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("model");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(4));
    for types in TYPES {
        let document = model::document(shape(types));
        group.throughput(Throughput::Bytes(document.len() as u64));
        let offered = offer(document.clone());
        group.bench_with_input(BenchmarkId::new("intake/admit", types), &offered, |b, o| {
            b.iter(|| {
                black_box(admit(
                    &o.selection,
                    SHA256_JCS_DIGEST_DOMAIN,
                    &o.bytes_by_digest,
                ))
                .is_ok()
            });
        });
        group.bench_with_input(
            BenchmarkId::new("intake/read_records", types),
            &document,
            |b, document| b.iter(|| read_records(PACKAGE, black_box(document))),
        );
        group.bench_with_input(
            BenchmarkId::new("parse/serde_json", types),
            &document,
            |b, document| {
                b.iter(|| serde_json::from_slice::<serde_json::Value>(black_box(document)));
            },
        );
        let text = String::from_utf8(document).expect("serde_json writes UTF-8");
        group.bench_with_input(
            BenchmarkId::new("parse/semantic_ir", types),
            &text,
            |b, text| b.iter(|| agent_ix_semantic_ir::json::parse(black_box(text))),
        );
    }
    group.finish();
}

fn normalization_and_admission(c: &mut Criterion) {
    let mut group = c.benchmark_group("model");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(4));
    for types in TYPES {
        let domain_package = intake(&offer(model::document(shape(types))))
            .expect("the generated document passes intake");
        let effective = view(&domain_package);
        let root = model::key(&chain_type(0));
        let document = population_document(shape(types), ADMITTED_MEMBERS);
        assert!(matches!(
            admit_population(&domain_package, &effective, &document),
            AdmissionOutcome::Admitted(_)
        ));
        assert!(matches!(
            admit_unchanged_invocation(&domain_package, &effective, &document),
            AdmissionOutcome::Admitted(_)
        ));
        group.throughput(Throughput::Elements(domain_package.records.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("normalize", types),
            &domain_package,
            |b, package| b.iter(|| view(black_box(package))),
        );
        group.bench_with_input(
            BenchmarkId::new("object_universe_of", types),
            &domain_package,
            |b, package| {
                b.iter(|| black_box(object_universe_of(black_box(package), &root)).is_ok())
            },
        );
        group.bench_with_input(
            BenchmarkId::new("admit_binding", types),
            &domain_package,
            |b, package| b.iter(|| admit_population(package, &effective, &document)),
        );
        group.bench_with_input(
            BenchmarkId::new("admit_invocation", types),
            &domain_package,
            |b, package| b.iter(|| admit_unchanged_invocation(package, &effective, &document)),
        );
    }
    group.finish();
}

fn conformance_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("model/all_instances");
    group.throughput(Throughput::Elements(QUERY_MEMBERS as u64));
    for depth in QUERY_DEPTHS {
        let shape = ModelShape {
            types: QUERY_TYPES,
            depth,
        };
        let domain_package =
            intake(&offer(model::document(shape))).expect("the generated document passes intake");
        let effective = view(&domain_package);
        let binding = match admit_population(
            &domain_package,
            &effective,
            &population_document(shape, QUERY_MEMBERS),
        ) {
            AdmissionOutcome::Admitted(binding) => binding,
            other => panic!("the generated population admits: {other:?}"),
        };
        for (target, type_name) in [("root", chain_type(0)), ("own", chain_type(depth))] {
            let queried = model::key(&type_name);
            let mut meter = Meter::new(qsl_bench::check::SCALAR_UNLIMITED);
            match all_instances(&binding, &queried, &mut meter) {
                AllInstancesOutcome::Completed(set) => assert_eq!(set.len(), QUERY_MEMBERS),
                other => panic!("allInstances<{type_name}> completes: {other:?}"),
            }
            group.bench_with_input(BenchmarkId::new(target, depth), &queried, |b, queried| {
                b.iter(|| {
                    let mut meter = Meter::new(qsl_bench::check::SCALAR_UNLIMITED);
                    all_instances(&binding, queried, &mut meter)
                });
            });
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    intake_stages,
    normalization_and_admission,
    conformance_queries
);
criterion_main!(benches);
