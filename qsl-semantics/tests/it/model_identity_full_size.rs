// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1025 and TC-195 N19 (FR-081-AC-12, FR-081-AC-13): the full 10,000-edge
//! single-inheritance chain, normalized at the NFR-012 defaults in an
//! isolated child process, on a thread with a 524,288-byte stack. The counters
//! are asserted exactly; the child's peak resident set (`VmHWM`) and wall time
//! are measured and checked against the published ceilings.

use std::process::Command;
use std::time::{Duration, Instant};

use ix_trace_rs::trace;
use qsl_semantics::model::accounting::{LimitKind, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::normalize::{normalize_with_meter, NormalizeOutcome};

/// Set in the child process only.
const CHILD_ENV: &str = "QSL_670_FULL_SIZE_CHILD";
const TEST_PATH: &str = "model_identity_full_size::ten_thousand_edge_chain";
const STACK_BYTES: usize = 524_288;
const EDGES: usize = 10_000;
const MAX_RESIDENT_BYTES: u64 = 21_474_836_480;
const MAX_WALL: Duration = Duration::from_secs(30);

fn key(at: usize) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/T{at}"))
}

/// `T0` to `T10000`, each after `T0` naming only the one before.
fn chain() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("n01"),
        (0..=EDGES)
            .map(|at| {
                DomainPackageRecord::ObjectType(ObjectTypeRecord {
                    key: key(at),
                    interface_features: None,
                    abstract_type: false,
                    supertypes: if at == 0 { Vec::new() } else { vec![key(at - 1)] },
                })
            })
            .collect(),
    )
}

/// `VmHWM` of this process, in bytes.
fn peak_resident_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status reads");
    let kib: u64 = status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
        .expect("VmHWM is reported");
    kib * 1024
}

/// The child: normalize on a 512 KiB-stack thread and print the measurements.
fn child() {
    let package = chain();
    let (elapsed, declarations, facts, hashed, work, outcome) = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK_BYTES)
            .spawn_scoped(scope, || {
                let started = Instant::now();
                let (outcome, meter) =
                    normalize_with_meter(&package, ModelNormalizationLimits::default());
                let elapsed = started.elapsed();
                (
                    elapsed,
                    meter.consumed(LimitKind::EffectiveDeclarations),
                    meter.consumed(LimitKind::DerivationFacts),
                    meter.consumed(LimitKind::HashedBytes),
                    meter.consumed(LimitKind::WorkUnits),
                    outcome,
                )
            })
            .expect("the 512 KiB thread spawns")
            .join()
            .expect("normalization does not overflow a 512 KiB stack")
    });
    let NormalizeOutcome::Completed(view) = &outcome else {
        panic!("the chain completes at the defaults, got {outcome:?}");
    };
    assert_eq!(view.declarations().len(), EDGES + 1);
    println!("DECLARATIONS={declarations}");
    println!("FACTS={facts}");
    println!("HASHED={hashed}");
    println!("WORK={work}");
    println!("WALL_MS={}", elapsed.as_millis());
    println!("VMHWM_BYTES={}", peak_resident_bytes());
}

fn measured(output: &str, name: &str) -> u64 {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=")))
        .unwrap_or_else(|| panic!("the child reports {name}: {output}"))
        .trim()
        .parse()
        .unwrap_or_else(|error| panic!("{name} is a number: {error}"))
}

#[trace("QSpec-TC-195", "QSpec-FR-081-AC-12", "QSpec-FR-081-AC-13")]
#[test]
fn ten_thousand_edge_chain() {
    if std::env::var_os(CHILD_ENV).is_some() {
        child();
        return;
    }
    let output = Command::new(std::env::current_exe().expect("the test binary path"))
        .args(["--exact", TEST_PATH, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("the child process runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "the child failed: {}\n{stdout}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(measured(&stdout, "DECLARATIONS"), 10_001);
    assert_eq!(measured(&stdout, "FACTS"), 20_001);
    assert_eq!(measured(&stdout, "HASHED"), 15_104_845);
    assert_eq!(measured(&stdout, "WORK"), 60_006);
    let wall_ms = measured(&stdout, "WALL_MS");
    let resident = measured(&stdout, "VMHWM_BYTES");
    println!("N19 wall_ms={wall_ms} vmhwm_bytes={resident}");
    assert!(
        Duration::from_millis(wall_ms) <= MAX_WALL,
        "wall time {wall_ms} ms"
    );
    assert!(resident <= MAX_RESIDENT_BYTES, "peak resident {resident} bytes");
}
