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
use qsl_semantics::model::normalize::{normalize_with_meter, NormalizeOutcome};
use quire_checked::key::DeclarationKey;

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
                    supertypes: if at == 0 {
                        Vec::new()
                    } else {
                        vec![key(at - 1)]
                    },
                })
            })
            .collect(),
    )
}

/// `VmHWM` of this process, in bytes. Linux only: macOS has no `/proc`, and
/// the crate forbids `unsafe`, so the parent reads the child's peak from
/// `/usr/bin/time -l` there (see `run_child`).
#[cfg(target_os = "linux")]
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
    #[cfg(target_os = "linux")]
    let baseline = peak_resident_bytes();
    let (elapsed, declarations, facts, work, outcome) = std::thread::scope(|scope| {
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
    println!("WORK={work}");
    println!("WALL_MS={}", elapsed.as_millis());
    #[cfg(target_os = "linux")]
    {
        println!("VMHWM_BASELINE_BYTES={baseline}");
        println!("VMHWM_BYTES={}", peak_resident_bytes());
    }
}

/// Run this test binary as the measured child.
#[cfg(not(target_os = "macos"))]
fn run_child() -> std::process::Output {
    Command::new(std::env::current_exe().expect("the test binary path"))
        .args(["--exact", TEST_PATH, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("the child process runs")
}

/// Run this test binary as the measured child under `/usr/bin/time -l`,
/// which reports the child's peak resident set in bytes on stderr.
#[cfg(target_os = "macos")]
fn run_child() -> std::process::Output {
    Command::new("/usr/bin/time")
        .arg("-l")
        .arg(std::env::current_exe().expect("the test binary path"))
        .args(["--exact", TEST_PATH, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("the child process runs under /usr/bin/time")
}

/// The child's peak resident set in bytes.
#[cfg(target_os = "linux")]
fn child_peak_bytes(stdout: &str, _stderr: &str) -> u64 {
    measured(stdout, "VMHWM_BYTES")
}

/// No peak-resident measurement exists for this target: the test fails
/// rather than passing without checking the ceiling.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn child_peak_bytes(_stdout: &str, _stderr: &str) -> u64 {
    panic!("peak resident set is measured only on Linux (VmHWM) and macOS (/usr/bin/time -l)")
}

/// Read the child's peak resident set from `/usr/bin/time -l` when available.
/// The exact sysctl permission failure is the only unavailable-metric case.
#[cfg(any(target_os = "macos", test))]
fn macos_peak_bytes(child_succeeded: bool, stderr: &str) -> Result<Option<u64>, &'static str> {
    if !child_succeeded {
        return Err("the child process failed");
    }
    let Some(value) = stderr
        .lines()
        .find_map(|line| line.trim().strip_suffix("maximum resident set size"))
    else {
        if macos_sysctl_denial(stderr) {
            return Ok(None);
        }
        return Err("/usr/bin/time -l did not report the peak resident set");
    };
    let peak = value
        .trim()
        .parse()
        .map_err(|_| "/usr/bin/time -l reported a non-numeric peak resident set")?;
    Ok(Some(peak))
}

/// A non-zero `/usr/bin/time` result is acceptable only when it is the
/// specific denied sysctl and the isolated child test reports success.
#[cfg(any(target_os = "macos", test))]
fn macos_child_succeeded(wrapper_succeeded: bool, stdout: &str, stderr: &str) -> bool {
    let child_test_passed = stdout.lines().any(|line| {
        line.trim_start()
            .starts_with("test result: ok. 1 passed; 0 failed;")
    });
    child_test_passed && (wrapper_succeeded || macos_sysctl_denial(stderr))
}

/// Whether `/usr/bin/time -l` reported only the expected metric denial.
#[cfg(any(target_os = "macos", test))]
fn macos_sysctl_denial(stderr: &str) -> bool {
    stderr
        .lines()
        .any(|line| line.trim() == "time: sysctl kern.clockrate: Operation not permitted")
}

/// The number the child printed after `name=`. The harness may put its own
/// `test ... ` prefix on the first printed line, so the key is searched for
/// within each line.
fn measured(output: &str, name: &str) -> u64 {
    let key = format!("{name}=");
    output
        .lines()
        .find_map(|line| line.find(&key).map(|at| &line[at + key.len()..]))
        .unwrap_or_else(|| panic!("the child reports {name}: {output}"))
        .trim()
        .parse()
        .unwrap_or_else(|error| panic!("{name} is a number: {error}"))
}

#[trace(
    "TC-1025",
    "FR-081-AC-12",
    "FR-081-AC-13",
    "QSpec-TC-195",
    "QSpec-FR-150-AC-15",
    "QSpec-FR-150-AC-16"
)]
#[test]
fn ten_thousand_edge_chain() {
    if std::env::var_os(CHILD_ENV).is_some() {
        child();
        return;
    }
    let output = run_child();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    #[cfg(target_os = "macos")]
    let child_succeeded = macos_child_succeeded(output.status.success(), &stdout, &stderr);
    #[cfg(not(target_os = "macos"))]
    let child_succeeded = output.status.success();
    assert!(child_succeeded, "the child failed: {stderr}\n{stdout}");
    assert_eq!(measured(&stdout, "DECLARATIONS"), 10_001);
    assert_eq!(measured(&stdout, "FACTS"), 20_001);
    assert_eq!(measured(&stdout, "WORK"), 50_003);
    let wall_ms = measured(&stdout, "WALL_MS");
    let stderr = String::from_utf8_lossy(&output.stderr);
    #[cfg(target_os = "linux")]
    let resident = Some(child_peak_bytes(&stdout, &stderr));
    #[cfg(target_os = "macos")]
    let resident = macos_peak_bytes(child_succeeded, &stderr).unwrap_or_else(|error| {
        panic!("could not read the child peak resident set: {error}; {stderr}")
    });
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let resident = Some(child_peak_bytes(&stdout, &stderr));
    match resident {
        Some(bytes) => println!("N19 wall_ms={wall_ms} peak_resident_bytes={bytes}"),
        None => println!(
            "N19 wall_ms={wall_ms} peak_resident_bytes=unavailable (sysctl permission denied)"
        ),
    }
    #[cfg(target_os = "linux")]
    println!(
        "N19 vmhwm_before_normalization_bytes={}",
        measured(&stdout, "VMHWM_BASELINE_BYTES")
    );
    assert!(
        Duration::from_millis(wall_ms) <= MAX_WALL,
        "wall time {wall_ms} ms"
    );
    if let Some(resident) = resident {
        assert!(
            resident <= MAX_RESIDENT_BYTES,
            "peak resident {resident} bytes"
        );
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::{macos_child_succeeded, macos_peak_bytes};

    /// Trace: TC-1025, FR-081-AC-12, FR-081-AC-13
    #[test]
    fn macos_sysctl_denial_skips_only_a_successful_childs_peak_metric() {
        let denial = "time: sysctl kern.clockrate: Operation not permitted";
        assert_eq!(macos_peak_bytes(true, denial), Ok(None));
        assert_eq!(
            macos_peak_bytes(false, denial),
            Err("the child process failed")
        );
        assert_eq!(
            macos_peak_bytes(true, "unexpected /usr/bin/time output"),
            Err("/usr/bin/time -l did not report the peak resident set")
        );
        assert_eq!(
            macos_peak_bytes(true, "not-a-number maximum resident set size"),
            Err("/usr/bin/time -l reported a non-numeric peak resident set")
        );
        assert_eq!(
            macos_peak_bytes(true, "42 maximum resident set size"),
            Ok(Some(42))
        );
    }

    /// Trace: TC-1025, FR-081-AC-12, FR-081-AC-13
    #[trace("TC-1025", "FR-081-AC-12", "FR-081-AC-13")]
    #[test]
    fn macos_time_denial_is_accepted_only_after_the_child_test_passes() {
        let passed =
            "running 1 test\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1301 filtered out; finished in 0.20s\n";
        let failed =
            "running 1 test\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1301 filtered out; finished in 0.20s\n";
        let denial = "time: sysctl kern.clockrate: Operation not permitted";

        assert!(macos_child_succeeded(false, &passed, denial));
        assert!(!macos_child_succeeded(false, failed, denial));
        assert!(!macos_child_succeeded(false, "", denial));
        assert!(!macos_child_succeeded(
            false,
            "test result: ok. passed; failed;\n",
            denial
        ));
        assert!(!macos_child_succeeded(
            false,
            &passed,
            "time: another operation not permitted"
        ));
        assert!(!macos_child_succeeded(
            false,
            &passed,
            "sysctl kern.clockrate: Operation not permitted"
        ));
        assert!(macos_child_succeeded(true, &passed, ""));
    }
}
