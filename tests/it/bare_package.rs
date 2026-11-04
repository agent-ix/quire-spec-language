// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 and FR-532 (TC-1331, TC-1332): the evaluation entry over a checked
//! package read from its bytes alone, `BarePackage` and
//! `EvidencePackage::from_package`, against the two routes that hold a source.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{EvidencePackage, Truth};
use qsl_package::BarePackage;
use quire_contract_model::CheckedPackageV2;
use serde_json::json;

use super::checked_handoff::{admitted, request, tree, KINDS};
use super::temporal_package_route::{
    clause_ids, event_clauses, recipe, request as evidence_request, run,
};
use quire_spec_language::checked_handoff::{evidence, write, Corpus};

/// The requests of the matrix: one per truth outcome, activation state and
/// `Incomplete` counter of FR-523, over `clause`.
fn matrix(
    package: &EvidencePackage<'_>,
    clause: &quire_contract_model::CheckedNodeId,
) -> Vec<(String, qsl_eval::temporal::evidence::EvidenceRequest)> {
    use qsl_eval::temporal::evidence::{EvidenceCounter, TemporalEvidenceLimits};
    use qsl_eval::temporal::Valuation;
    let base = evidence_request(package, clause, None, 3);
    let mut requests = vec![("true".to_owned(), base.clone())];
    let mut falsified = base.clone();
    for position in &mut falsified.positions {
        for valuation in position.valuations.values_mut() {
            *valuation = Valuation::False;
        }
    }
    requests.push(("false".to_owned(), falsified));
    let mut pending = base.clone();
    pending.decision_progress = Some(0);
    pending.decision_closure = Some(qsl_eval::temporal::ProgressClosure::Open);
    pending.surrounding_progress = Some(0);
    pending.surrounding_closure = Some(qsl_eval::temporal::ProgressClosure::Open);
    pending.positions.truncate(1);
    requests.push(("pending".to_owned(), pending));
    let mut inactive = base.clone();
    inactive.origins.clear();
    inactive.trigger = None;
    requests.push(("inactive".to_owned(), inactive));
    let mut unavailable = base.clone();
    unavailable.anchor = None;
    requests.push(("unavailable".to_owned(), unavailable));
    let usage = run(package, clause, &base)
        .0
        .expect("the base request evaluates")
        .usage;
    for (counter, value) in usage.entries() {
        if value == 0 {
            continue;
        }
        let mut limited = base.clone();
        limited.limits =
            TemporalEvidenceLimits::default().with_ceiling(counter, value.saturating_sub(1));
        requests.push((format!("incomplete:{counter:?}"), limited));
    }
    let _ = EvidenceCounter::FormulaNodes;
    requests
}

/// FR-532-AC-1 (TC-1332): over each of R's four clocked clauses of its three
/// packages, the result of every request of the matrix (each truth outcome,
/// activation state and `Incomplete` counter) is equal over the three routes,
/// field by field, with the charge log and `result_identity`.
#[trace("FR-532-AC-1", "FR-516-AC-18", "TC-1332")]
#[test]
fn the_bare_route_gives_the_results_and_charges_of_the_two_source_routes() {
    let mut clauses = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    for kind in KINDS {
        let built = recipe(kind);
        let read = built.read(&built.emission);
        let emitted = EvidencePackage::from_emission(&built.emission, &read).expect("emission");
        let back = EvidencePackage::from_read(&read).expect("read");
        let bare = BarePackage::new(read.admitted());
        let bytes = EvidencePackage::from_package(&bare).expect("bare");
        assert_eq!(emitted.rows(), bytes.rows());
        for clause in clause_ids(&emitted) {
            let subject = emitted.subject(&clause, None).expect("reads");
            let name = subject.facts.name().to_owned();
            if !matches!(
                name.as_str(),
                "Due" | "DueLate" | "DueSample" | "DueTimestamp"
            ) {
                continue;
            }
            clauses.insert(name);
            for (what, asked) in matrix(&emitted, &clause) {
                let (left, left_charges) = run(&emitted, &clause, &asked);
                let (middle, middle_charges) = run(&back, &clause, &asked);
                let (right, right_charges) = run(&bytes, &clause, &asked);
                let (left, middle, right) = (
                    left.unwrap_or_else(|failure| panic!("{what}: {failure:?}")),
                    middle.unwrap_or_else(|failure| panic!("{what}: {failure:?}")),
                    right.unwrap_or_else(|failure| panic!("{what}: {failure:?}")),
                );
                for other in [&middle, &right] {
                    assert_eq!(left.activation, other.activation, "{what}");
                    assert_eq!(left.truth, other.truth, "{what}");
                    assert_eq!(left.settlement, other.settlement, "{what}");
                    assert_eq!(left.non_value, other.non_value, "{what}");
                    assert_eq!(left.incomplete, other.incomplete, "{what}");
                    assert_eq!(left.support, other.support, "{what}");
                    assert_eq!(left.usage, other.usage, "{what}");
                    assert_eq!(left.result_identity, other.result_identity, "{what}");
                    assert_eq!(format!("{left:?}"), format!("{other:?}"), "{what}");
                }
                assert_eq!(left_charges, middle_charges, "{what}");
                assert_eq!(left_charges, right_charges, "{what}");
                seen.insert(format!(
                    "{what:.10}/{:?}/{:?}",
                    right.activation, right.truth
                ));
            }
            // The facts the evaluator reads are one value on every route.
            let one = emitted.subject(&clause, None).expect("reads");
            let two = back.subject(&clause, None).expect("reads");
            let three = bytes.subject(&clause, None).expect("reads");
            assert_eq!(one.facts, three.facts);
            assert_eq!(two.facts, three.facts);
            assert_eq!(one.instance_bound, three.instance_bound);
        }
    }
    assert_eq!(clauses.len(), 4, "{clauses:?}");
    for outcome in ["Some(True)", "Some(False)", "Some(Pending)"] {
        assert!(
            seen.iter().any(|row| row.ends_with(outcome)),
            "{outcome}: {seen:?}"
        );
    }
    for state in ["Active", "Inactive", "Unavailable"] {
        assert!(
            seen.iter().any(|row| row.contains(&format!("/{state}/"))),
            "{state}: {seen:?}"
        );
    }
    assert!(
        seen.iter().any(|row| row.starts_with("incomplete")),
        "{seen:?}"
    );
}

/// FR-532-AC-5: the package component of a progress binding is
/// `package.name()` on every route.
#[trace("FR-532-AC-5", "TC-1332")]
#[test]
fn the_progress_binding_names_the_package_and_no_source_label() {
    let built = recipe("event_position");
    let read = built.read(&built.emission);
    let emitted = EvidencePackage::from_emission(&built.emission, &read).expect("emission");
    let bare = BarePackage::new(read.admitted());
    let bytes = EvidencePackage::from_package(&bare).expect("bare");
    for clause in clause_ids(&bytes) {
        let left = emitted.subject(&clause, None).expect("reads");
        let right = bytes.subject(&clause, None).expect("reads");
        assert_eq!(right.progress.package, read.admitted().name());
        assert_eq!(left.progress, right.progress);
        assert_eq!(right.progress.clock, right.facts.clock());
        assert!(!right.progress.clock.is_empty());
    }
}

/// FR-532-AC-10, FR-532-AC-4: one shared `holds` node used twice is one atom.
#[trace("FR-532-AC-10", "TC-1332")]
#[test]
fn a_shared_holds_node_is_one_atom_on_every_route() {
    let shared = event_clauses(
        "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
         always[0,3] (holds(c.versionNumber >= 1) and holds(c.versionNumber >= 1)) }",
        &[("Due", "authority")],
    );
    let single = event_clauses(
        "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
         always[0,3] holds(c.versionNumber >= 1) }",
        &[("Due", "authority")],
    );
    let usage = |built: &super::temporal_package_route::Built| {
        let read = built.read(&built.emission);
        let emitted = EvidencePackage::from_emission(&built.emission, &read).expect("emission");
        let back = EvidencePackage::from_read(&read).expect("read");
        let bare = BarePackage::new(read.admitted());
        let bytes = EvidencePackage::from_package(&bare).expect("bare");
        let [clause] = clause_ids(&bytes).try_into().expect("one clause");
        let mut outcomes = Vec::new();
        for package in [&emitted, &back, &bytes] {
            let subject = package.subject(&clause, None).expect("reads");
            assert_eq!(subject.facts.formula().atom_count(), 1, "one atom");
            let asked = evidence_request(package, &clause, None, 4);
            let (result, charges) = run(package, &clause, &asked);
            let result = result.expect("a result, no missing valuation of atom 0");
            assert_eq!(result.truth, Some(Truth::True));
            assert!(result.non_value.is_none());
            outcomes.push((result.usage, charges));
        }
        assert_eq!(outcomes[0], outcomes[1]);
        assert_eq!(outcomes[0], outcomes[2]);
        outcomes.remove(0)
    };
    let (shared_usage, _) = usage(&shared);
    let (single_usage, _) = usage(&single);
    // The shared clause has one more node, its `and`, which is one visit and
    // one formula node; a repeated operand adds no lookup, no valuation and
    // no step.
    let counter = |usage: &qsl_eval::temporal::evidence::EvidenceUsage, name: &str| {
        usage
            .entries()
            .iter()
            .find(|(counter, _)| format!("{counter:?}").to_lowercase().replace('_', "") == name)
            .map(|(_, value)| *value)
            .expect("a counter")
    };
    assert_eq!(counter(&shared_usage, "formulanodes"), 3);
    assert_eq!(counter(&single_usage, "formulanodes"), 2);
    for name in ["valuations", "support", "evaluationsteps", "positions"] {
        assert_eq!(
            counter(&shared_usage, name),
            counter(&single_usage, name),
            "{name}"
        );
    }
    assert_eq!(
        counter(&shared_usage, "visited"),
        counter(&single_usage, "visited") + 1
    );
}

/// The package of `kind`'s handoff directory, read from its bytes alone, with
/// the directory's files.
fn directory(
    kind: &str,
) -> (
    tempfile::TempDir,
    std::collections::BTreeMap<String, Vec<u8>>,
    Box<CheckedPackageV2>,
) {
    let holder = tempfile::tempdir().expect("temporary directory");
    let out = holder.path().join("handoff");
    let asked = request(kind, Corpus::None);
    write(&asked, &out).expect("the handoff is written");
    let files = tree(&out);
    let package = admitted(
        &files["checked-package.json"],
        &evidence(&asked).expect("evidence"),
    );
    (holder, files, package)
}

/// IR-1325 AC3: the bytes-only dependency closure (the slot call that
/// consumes it is not delivered, so no FR-531/FR-533 criterion is claimed): the closure is built
/// from the direct dependencies the reader retained in the package.
#[trace("TC-1331")]
#[test]
fn the_dependency_closure_is_read_from_the_retained_direct_dependencies() {
    let (_holder, _files, package) = directory("event_position");
    let direct: Vec<&str> = package.direct_dependencies().map(|d| d.name()).collect();
    assert!(!direct.is_empty(), "R imports a library");
    let bare = BarePackage::new(&package);
    let closure: Vec<&str> = bare.dependencies().iter().map(|d| d.name()).collect();
    assert_eq!(closure[..direct.len()], direct[..]);
    let mut once = closure.clone();
    once.sort_unstable();
    once.dedup();
    assert_eq!(once.len(), closure.len(), "every dependency once");
}

fn read_source(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The names of the `fn` items declared between the line that holds `start`
/// and the next line that closes the block at the same indentation.
fn public_fns(text: &str, start: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut inside = false;
    let mut indent = 0;
    for line in text.lines() {
        if !inside {
            if line.contains(start) {
                inside = true;
                indent = line.len() - line.trim_start().len();
            }
            continue;
        }
        let here = line.len() - line.trim_start().len();
        if line.trim() == "}" && here == indent {
            break;
        }
        let trimmed = line.trim_start();
        let trimmed = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
        if let Some(rest) = trimmed.strip_prefix("fn ") {
            if here == indent + 4 {
                names.push(rest.split(['(', '<']).next().unwrap_or("").to_owned());
            }
        }
    }
    names
}

/// FR-531-AC-9 (TC-1331): every public method of `EvidencePackage` and every
/// method of `CheckedPackageEvaluation` has a row in the capability
/// inventory.
#[trace("TC-1331")]
#[test]
fn every_evaluation_method_has_an_inventory_row() {
    let spec = read_source(
        "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
    );
    let table = spec
        .split("### Capability inventory")
        .nth(1)
        .and_then(|rest| rest.split("### The identity file").next())
        .expect("the inventory");
    let firsts: Vec<&str> = table
        .lines()
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| line.split('|').nth(1))
        .collect();
    let package = read_source("qsl-eval/src/temporal/evidence/package.rs");
    let mut methods = public_fns(&package, "impl<'p> EvidencePackage<'p> {");
    methods.retain(|name| {
        !matches!(
            name.as_str(),
            "with_rows" | "from_emission" | "from_read" | "from_package"
        )
    });
    let evaluation = read_source("qsl-eval/src/value/expression/mod.rs");
    methods.extend(public_fns(
        &evaluation,
        "pub trait CheckedPackageEvaluation: family::sealed::Sealed {",
    ));
    assert!(methods.len() >= 10, "{methods:?}");
    for name in &methods {
        assert!(
            firsts.iter().any(|row| row.contains(&format!("`{name}`"))),
            "`{name}` has no row in the capability inventory"
        );
    }
    for constructor in ["from_emission", "from_read"] {
        assert!(firsts.iter().any(|row| row.contains(constructor)));
    }
}

/// FR-531-AC-2 (TC-1331): no public signature of the bare entry names the
/// source side.
#[trace("FR-531-AC-2", "TC-1331")]
#[test]
fn no_bare_signature_names_the_source_side() {
    let bare = read_source("qsl-package/src/bare.rs");
    let package = read_source("qsl-eval/src/temporal/evidence/package.rs");
    let signatures = bare
        .lines()
        .filter(|line| line.trim_start().starts_with("pub "))
        .map(str::to_owned)
        .chain(
            package
                .lines()
                .filter(|line| line.contains("from_package"))
                .map(str::to_owned),
        );
    for signature in signatures {
        for source in [
            "CheckedPackage ",
            "CheckedPackage,",
            "Emission",
            "AdmittedPackageRead",
        ] {
            assert!(!signature.contains(source), "{signature}");
        }
    }
    // `BarePackage` has the one constructor.
    let constructors = public_fns(&bare, "impl<'p> BarePackage<'p> {");
    assert_eq!(constructors.iter().filter(|name| *name == "new").count(), 1);
    assert!(!constructors.iter().any(|name| name.starts_with("from_")));
}

/// FR-532-AC-6 (TC-1332): `from_package` returns `AmbiguousName` and no other
/// variant.
#[trace("TC-1332")]
#[test]
fn from_package_returns_only_the_ambiguous_name_mismatch() {
    let package = read_source("qsl-eval/src/temporal/evidence/package.rs");
    let body = package
        .split("pub fn from_package")
        .nth(1)
        .and_then(|rest| rest.split("fn with_rows").next())
        .expect("from_package");
    for variant in ["ClauseRow", "ProfileSelections", "Bindings"] {
        assert!(!body.contains(variant), "{variant}");
    }
    let rows = package
        .split("pub fn of_package")
        .nth(1)
        .and_then(|rest| rest.split("pub fn names").next())
        .expect("of_package");
    assert!(rows.contains("PackageMismatch::AmbiguousName"));
    for variant in ["ClauseRow", "ProfileSelections", "Bindings"] {
        assert!(
            !rows.contains(&format!("PackageMismatch::{variant}")),
            "{variant}"
        );
    }
}

/// FR-532-AC-9 (TC-1332): the evaluator reads the activation, profile and
/// clock only through `ClauseFacts`, and a source identity is no member of a
/// ledger binding.
#[trace("FR-532-AC-9", "TC-1332")]
#[test]
fn the_evaluator_reads_the_clause_only_through_the_facts() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("qsl-eval/src/temporal/evidence");
    for entry in std::fs::read_dir(&directory).expect("the evidence sources") {
        let path = entry.expect("entry").path();
        let text = std::fs::read_to_string(&path).expect("source");
        for forbidden in ["SourceIdentity", "CheckedTemporalClause"] {
            let code: String = text
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!code.contains(forbidden), "{}: {forbidden}", path.display());
        }
    }
    let ledger = read_source("qsl-eval/src/temporal/ledger.rs");
    let binding = ledger
        .split("pub struct LedgerBinding")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("LedgerBinding");
    assert!(!binding.contains("SourceIdentity"));
}

/// FR-531-AC-11 (TC-1331): FR-515 no longer calls the bytes-only evaluator
/// out of scope, and FR-515, FR-516 and FR-520 each carry a "Consumer entry"
/// section naming FR-531.
#[trace("FR-531-AC-11", "TC-1331")]
#[test]
fn the_consumer_entry_sections_name_the_bare_entry() {
    for (prefix, name) in [("FR-515", "515"), ("FR-516", "516"), ("FR-520", "520")] {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/functional");
        let path = std::fs::read_dir(&directory)
            .expect("spec")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|file| file.starts_with(&format!("{prefix}-")))
            })
            .unwrap_or_else(|| panic!("FR-{name}"));
        let text = std::fs::read_to_string(&path).expect("requirement");
        let section = text
            .split("Consumer entry")
            .nth(1)
            .unwrap_or_else(|| panic!("{prefix} has a Consumer entry section"));
        assert!(section.contains("FR-531"), "{prefix}");
        if prefix == "FR-515" {
            assert!(!text.contains("separate future capability"));
        }
    }
}

/// A node that is not a temporal clause, and a node the package does not
/// hold, refuse `clause-root-unresolved` over the bare package with an empty
/// charge log, and the same call over a clause evaluates.
#[test]
fn a_node_that_is_no_temporal_clause_is_unresolved_over_the_bare_package() {
    let (_holder, _files, package) = directory("event_position");
    let bare = BarePackage::new(&package);
    let evaluation = EvidencePackage::from_package(&bare).expect("bare");
    let due = clause_ids(&evaluation)
        .into_iter()
        .next()
        .expect("a clause");
    let asked = evidence_request(&evaluation, &due, None, 3);
    let other = package
        .graph()
        .nodes
        .iter()
        .map(|node| node.node_id.clone())
        .find(|id| !clause_ids(&evaluation).contains(id))
        .expect("a node that is no clause");
    let mut absent = due.clone();
    absent.ordinal += 1_000;
    for id in [other, absent] {
        let mut meter = super::temporal_package_route::unlimited();
        let refused = evaluation.evaluate_temporal_evidence(&id, &asked, &mut meter);
        assert_eq!(
            super::temporal_package_route::cause(&refused.expect_err("refused")),
            "clause-root-unresolved"
        );
        assert!(meter.admitted_charges().is_empty());
    }
    assert!(run(&evaluation, &due, &asked).0.is_ok());
}

/// The `reason` a `clause-unreadable` refusal names.
fn reason(failure: &quire_eval::CallFailure) -> String {
    match failure {
        quire_eval::CallFailure::Admission(failure) => match &**failure {
            quire_eval::CallAdmissionFailure::Refused(record) => record
                .record
                .fields
                .get("reason")
                .cloned()
                .expect("a reason"),
            other => panic!("a refusal, not {other:?}"),
        },
        other => panic!("an admission refusal, not {other:?}"),
    }
}

/// A unit whose one clause captures a record (`c` itself).
fn record_capture() -> super::temporal_package_route::Built {
    event_clauses(
        "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
         capture r: Config::ConfigVersion = c; always[0,2] holds(c.versionNumber >= 1) }",
        &[("Due", "authority")],
    )
}

/// FR-532-AC-12 (capture half, TC-1332): a record-typed capture is projected
/// with the source unit's selected identity rows on the two source routes,
/// and its type equals the in-process clause's; the bare route holds no
/// identity rows and refuses `clause-unreadable` with reason `capture-type`.
#[test]
fn a_record_typed_capture_projects_with_the_source_identities() {
    let built = record_capture();
    let read = built.read(&built.emission);
    let emitted = EvidencePackage::from_emission(&built.emission, &read).expect("emission");
    let back = EvidencePackage::from_read(&read).expect("read");
    let [clause] = clause_ids(&emitted).try_into().expect("one clause");
    let source = read
        .source()
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "Due")
        .expect("Due");
    let declared: Vec<_> = source
        .clause()
        .captures()
        .map(|(name, ty)| (name.to_owned(), ty.clone()))
        .collect();
    assert_eq!(declared.len(), 1);
    for package in [&emitted, &back] {
        let subject = package
            .subject(&clause, None)
            .expect("the capture projects");
        let held: Vec<_> = subject
            .facts
            .captures()
            .map(|(name, ty)| (name.to_owned(), ty.clone()))
            .collect();
        assert_eq!(held, declared);
    }
    let bare = BarePackage::new(read.admitted());
    let bytes = EvidencePackage::from_package(&bare).expect("bare");
    let refused = bytes
        .subject(&clause, None)
        .err()
        .expect("no identity rows");
    assert_eq!(
        super::temporal_package_route::cause(&refused),
        "clause-unreadable"
    );
    assert_eq!(reason(&refused), "capture-type");
}

/// Rewrite the emitted bytes of `built` with `edit` and read them back.
fn forged(
    built: &super::temporal_package_route::Built,
    edit: impl Fn(&mut serde_json::Value),
) -> qsl_package::Emission {
    let mut wire: serde_json::Value =
        serde_json::from_slice(built.emission.package().bytes()).expect("JSON");
    edit(&mut wire);
    built
        .emission
        .clone()
        .with_package_bytes(serde_json::to_vec(&wire).expect("encodes"))
}

/// FR-532-AC-12 (name half) and FR-531-AC-6 (profile half): the reasons
/// `clause-name` and the bare `not-finite-window-profile` need a package the
/// reader admits with no `temporal_clause_names` row, or with an
/// infinite-trace profile row. Contract IR refuses both, so the refusals are
/// defensive and cannot be met by an admitted package: the reader's refusal
/// is the observable behavior.
#[test]
fn the_reader_refuses_the_packages_that_would_meet_the_defensive_reasons() {
    let built = event_clauses(
        "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
         always[0,2] holds(c.versionNumber >= 1) }",
        &[("Due", "authority")],
    );
    let nameless = forged(&built, |wire| {
        wire["temporal_clause_names"] = json!([]);
    });
    let refused = built.try_read(&nameless).err().expect("refused");
    assert!(format!("{refused:?}").contains("/temporal_clause_names"));
    let infinite = forged(&built, |wire| {
        let text = wire.to_string().replace(
            "quire.temporal.event-position.false-extension/v1",
            "quire.temporal.infinite-trace/v1",
        );
        *wire = serde_json::from_str(&text).expect("JSON");
    });
    let refused = built.try_read(&infinite).err().expect("refused");
    assert!(format!("{refused:?}").contains("TimedNodeReference"));
}
