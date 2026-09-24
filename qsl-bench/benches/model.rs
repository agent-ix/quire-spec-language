// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 model-layer benchmarks over an N-type `DomainPackage` built
//! through FR-154 model intake (`qsl_bench::model` documents the shape).
//! Every stage is timed alone, on inputs prepared outside the timing, so
//! each stage's figure is its own cost:
//!
//! - `model/intake/{parse,admit,read_records}/<n>`: `PackageDocument::parse`
//!   (the one parse intake makes since QSL-201), `intake::admit` (which
//!   includes that parse) and `intake::read_records` (FCD's validator and
//!   the per-node reader, over the parsed document).
//! - `model/normalize/<n>`: FR-150 normalization under unlimited limits.
//! - `model/admit_binding/<n>` and `model/admit_invocation/<n>`: FR-153
//!   admission of a fixed 100-member population, directly and as one
//!   unchanged invocation (pre and post). Admission reads its object
//!   universe from the effective view and never normalizes (QSL-204), so
//!   neither figure scales with `model/normalize/<n>`.
//! - `model/conformance/resolve_redefinition_target/<n>`: one model
//!   conformance call, which builds `ConformanceIndex` over the whole
//!   package every time (QSL-202).
//! - `model/all_instances/{root,own}/<depth>`: `allInstances` over 1,000
//!   members whose type has `depth` proper ancestors, querying the root
//!   type (a full ancestor walk per member) or the members' own type (no
//!   walk) -- F6's A axis.
//! - `model/all_instances/members/<m>` and
//!   `model/query/evaluate_all_instances/<m>`: `allInstances<C0>` over `m`
//!   members of an 8-ancestor type, called directly and through the
//!   evaluator's bridge (`value::model_query`, which rebuilds its reverse
//!   catalog per query) -- F6's N axis and QSL-202's `reverse_catalog`.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use qsl_bench::model::{
    self, admit_offer, admit_population, admit_unchanged_invocation, admitted, chain_type, intake,
    offer, parse_document, population_document, query_all_instances_of_root, read,
    resolve_root_field_redefinition, view, ModelShape,
};
use qsl_bench::widen;
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

/// The `allInstances` depth sweep: members, package size, depths.
const QUERY_MEMBERS: usize = 1_000;
const QUERY_TYPES: usize = 1_000;
const QUERY_DEPTHS: [usize; 4] = [1, 8, 32, 120];

/// The `allInstances` member sweep: depth and member counts.
const MEMBER_DEPTH: usize = 8;
const MEMBER_COUNTS: [usize; 3] = [250, 1_000, 4_000];

fn shape(types: usize) -> ModelShape {
    ModelShape {
        types,
        depth: SWEEP_DEPTH,
    }
}

fn intake_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("model/intake");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(4));
    for types in TYPES {
        let document = model::document(shape(types));
        group.throughput(Throughput::Bytes(widen(document.len())));
        let offered = offer(document.clone());
        let (_, parsed) = admit_offer(&offered).expect("the generated document admits");
        assert!(read(&parsed).is_ok(), "{types} types read clean");
        group.bench_with_input(BenchmarkId::new("parse", types), &document, |b, d| {
            b.iter(|| black_box(parse_document(black_box(d))).is_ok());
        });
        group.bench_with_input(BenchmarkId::new("admit", types), &offered, |b, o| {
            b.iter(|| black_box(admit_offer(o)).is_ok());
        });
        group.bench_with_input(BenchmarkId::new("read_records", types), &parsed, |b, p| {
            b.iter(|| black_box(read(black_box(p))).is_ok());
        });
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
        let document = population_document(shape(types), ADMITTED_MEMBERS);
        assert!(matches!(
            admit_population(&domain_package, &effective, &document),
            AdmissionOutcome::Admitted(_)
        ));
        assert!(matches!(
            admit_unchanged_invocation(&domain_package, &effective, &document),
            AdmissionOutcome::Admitted(_)
        ));
        assert!(resolve_root_field_redefinition(&domain_package).is_ok());
        group.throughput(Throughput::Elements(widen(domain_package.records.len())));
        group.bench_with_input(
            BenchmarkId::new("normalize", types),
            &domain_package,
            |b, package| b.iter(|| view(black_box(package))),
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
        group.bench_with_input(
            BenchmarkId::new("conformance/resolve_redefinition_target", types),
            &domain_package,
            |b, package| {
                b.iter(|| black_box(resolve_root_field_redefinition(black_box(package))).is_ok());
            },
        );
    }
    group.finish();
}

fn all_instances_of(
    binding: &qsl_semantics::model::population::PopulationBinding,
    queried: &qsl_semantics::model::key::DeclarationKey,
) -> AllInstancesOutcome {
    let mut meter = Meter::new(qsl_bench::check::SCALAR_UNLIMITED);
    all_instances(binding, queried, &mut meter)
}

fn conformance_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("model/all_instances");
    group.throughput(Throughput::Elements(widen(QUERY_MEMBERS)));
    for depth in QUERY_DEPTHS {
        let (_, binding) = admitted(
            ModelShape {
                types: QUERY_TYPES,
                depth,
            },
            QUERY_MEMBERS,
        );
        for (target, type_name) in [("root", chain_type(0)), ("own", chain_type(depth))] {
            let queried = model::key(&type_name);
            match all_instances_of(&binding, &queried) {
                AllInstancesOutcome::Completed(set) => assert_eq!(set.len(), QUERY_MEMBERS),
                other => panic!("allInstances<{type_name}> completes: {other:?}"),
            }
            group.bench_with_input(BenchmarkId::new(target, depth), &queried, |b, queried| {
                b.iter(|| all_instances_of(&binding, queried));
            });
        }
    }
    group.finish();
}

fn conformance_members(c: &mut Criterion) {
    let mut group = c.benchmark_group("model");
    let root = model::key(&chain_type(0));
    for members in MEMBER_COUNTS {
        let (_, binding) = admitted(
            ModelShape {
                types: QUERY_TYPES,
                depth: MEMBER_DEPTH,
            },
            members,
        );
        match all_instances_of(&binding, &root) {
            AllInstancesOutcome::Completed(set) => assert_eq!(set.len(), members),
            other => panic!("allInstances<C0> completes: {other:?}"),
        }
        assert!(query_all_instances_of_root(&binding).is_ok());
        group.throughput(Throughput::Elements(widen(members)));
        group.bench_with_input(
            BenchmarkId::new("all_instances/members", members),
            &binding,
            |b, binding| b.iter(|| all_instances_of(binding, &root)),
        );
        group.bench_with_input(
            BenchmarkId::new("query/evaluate_all_instances", members),
            &binding,
            |b, binding| b.iter(|| black_box(query_all_instances_of_root(binding)).is_ok()),
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    intake_stages,
    normalization_and_admission,
    conformance_depth,
    conformance_members
);
criterion_main!(benches);
