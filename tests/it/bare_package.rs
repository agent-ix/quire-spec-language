// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 and FR-532 (TC-1331, TC-1332): the evaluation entry over a checked
//! package read from its bytes alone, `BarePackage` and
//! `EvidencePackage::from_package`, against the two routes that hold a source.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{ActivationState, EvidencePackage, Truth};
use qsl_package::{BarePackage, IdentityFileRefusal, ReadIdentityRows};
use quire_contract_model::CheckedPackageV2;
use serde_json::{json, Value};

use super::checked_handoff::{admitted, request, tree, KINDS};
use super::temporal_package_route::{
    clause_ids, event_clauses, recipe, request as evidence_request, run,
};
use quire_spec_language::checked_handoff::{evidence, write, Corpus};

/// The three routes' results of `ticks` positions over each of R's clocked
/// clauses: equal, field for field, with equal charges.
#[trace("FR-532-AC-1", "FR-516-AC-18", "FR-531-AC-10", "TC-1332", "TC-1331")]
#[test]
fn the_bare_route_gives_the_results_and_charges_of_the_two_source_routes() {
    let mut compared = 0;
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
            if !matches!(subject.facts.name(), "Due" | "DueSample" | "DueTimestamp") {
                continue;
            }
            for ticks in [1_u64, 3] {
                let asked = evidence_request(&emitted, &clause, None, ticks);
                let (left, left_charges) = run(&emitted, &clause, &asked);
                let (middle, middle_charges) = run(&back, &clause, &asked);
                let (right, right_charges) = run(&bytes, &clause, &asked);
                let (left, middle, right) = (
                    left.expect("result"),
                    middle.expect("result"),
                    right.expect("result"),
                );
                assert_eq!(left.activation, ActivationState::Active);
                assert_eq!(format!("{left:?}"), format!("{middle:?}"));
                assert_eq!(format!("{left:?}"), format!("{right:?}"));
                assert_eq!(left.result_identity, right.result_identity);
                assert_eq!(left_charges, right_charges);
                assert_eq!(middle_charges, right_charges);
                assert!(!right_charges.is_empty());
                compared += 1;
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
    assert!(compared >= 6);
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
#[trace("FR-532-AC-10", "FR-532-AC-4", "TC-1332")]
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
            outcomes.push((result.usage.clone(), charges));
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

/// FR-531-AC-1, FR-531-AC-3, FR-531-AC-8: the package read from the handoff
/// directory gives a `BarePackage` and a temporal call that returns a result;
/// `identities.json` is the format's file, written deterministically.
#[trace("FR-531-AC-1", "FR-531-AC-3", "FR-531-AC-8", "FR-517-AC-1", "TC-1331")]
#[test]
fn a_handoff_directory_gives_a_bare_package_and_an_identity_file() {
    let (_holder, files, package) = directory("event_position");
    let bare = BarePackage::new(&package);
    let evaluation = EvidencePackage::from_package(&bare).expect("bare");
    let due = clause_ids(&evaluation)
        .into_iter()
        .find(|clause| {
            evaluation
                .subject(clause, None)
                .is_ok_and(|subject| subject.facts.name() == "Due")
        })
        .expect("Due");
    let asked = evidence_request(&evaluation, &due, None, 3);
    let (result, charges) = run(&evaluation, &due, &asked);
    let result = result.expect("a result from the bytes alone");
    assert_eq!(result.activation, ActivationState::Active);
    assert_eq!(result.truth, Some(Truth::True));
    assert!(!charges.is_empty());

    let file: Value = serde_json::from_slice(&files["identities.json"]).expect("JSON");
    assert_eq!(file["format"], "quire.checked-handoff.identities/1");
    assert!(files["identities.json"].ends_with(b"}\n"));
    let rows = ReadIdentityRows::read(&files["identities.json"], &package).expect("reads");
    assert_eq!(rows.package_name(), package.name());
    assert_eq!(
        file["declarations"].as_array().map(Vec::len),
        Some(rows.declarations().len())
    );
    assert_eq!(
        rows.to_bytes(&package).expect("encodes"),
        files["identities.json"],
        "the file is the encoding of the rows it holds"
    );
    let (_again, second, _) = directory("event_position");
    assert_eq!(files["identities.json"], second["identities.json"]);
}

/// FR-531-AC-12, FR-531-AC-13: the identity file's refusals are closed and
/// name what they found.
#[trace("FR-531-AC-12", "FR-531-AC-13", "TC-1331")]
#[test]
fn the_identity_file_refuses_what_it_cannot_join() {
    let (_holder, files, package) = directory("event_position");
    let good: Value = serde_json::from_slice(&files["identities.json"]).expect("JSON");
    let read = |value: &Value| {
        ReadIdentityRows::read(&serde_json::to_vec(value).expect("bytes"), &package)
    };
    assert!(read(&good).is_ok());

    assert!(matches!(
        ReadIdentityRows::read(b"not json", &package),
        Err(IdentityFileRefusal::Malformed { .. })
    ));
    let mut unknown = good.clone();
    unknown["extra"] = json!(1);
    assert_eq!(
        read(&unknown),
        Err(IdentityFileRefusal::Malformed {
            member: "extra".to_owned()
        })
    );
    let repeated = String::from_utf8(files["identities.json"].clone())
        .expect("UTF-8")
        .replacen(
            "\"declarations\"",
            "\"declarations\": [], \"declarations\"",
            1,
        );
    assert!(matches!(
        ReadIdentityRows::read(repeated.as_bytes(), &package),
        Err(IdentityFileRefusal::Malformed { .. })
    ));
    let mut format = good.clone();
    format["format"] = json!("quire.checked-handoff.identities/2");
    assert_eq!(
        read(&format),
        Err(IdentityFileRefusal::Format(
            "quire.checked-handoff.identities/2".to_owned()
        ))
    );
    let declarations = good["declarations"].as_array().expect("rows");
    assert!(!declarations.is_empty(), "R selects declarations");
    let mut absent = good.clone();
    absent["declarations"][0]["structural_node"]["package"] = json!("another-package");
    assert_eq!(
        read(&absent),
        Err(IdentityFileRefusal::Unjoined {
            member: "declarations",
            index: 0
        })
    );
    let mut mismatched = good.clone();
    let form = declarations[0]["form"].as_str().expect("form");
    mismatched["declarations"][0]["form"] = json!(if form == "union" {
        "object_type"
    } else {
        "union"
    });
    assert!(read(&mismatched).is_err());
}

/// FR-531-AC-5 (bytes-only dependency closure), IR-1325: the closure is built
/// from the direct dependencies the reader retained in the package.
#[trace("FR-531-AC-5", "FR-533-AC-1", "TC-1331")]
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
