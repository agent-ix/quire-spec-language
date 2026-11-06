// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 and FR-532 (TC-1331, TC-1332): the evaluation entry over a checked
//! package read from its bytes alone, `BarePackage` and
//! `EvidencePackage::from_package`, against the two routes that hold a source.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{ActivationState, EvidencePackage, Truth};
use qsl_package::{BarePackage, IdentityFileRefusal, IdentityMember, ReadIdentityRows};
use quire_contract_model::CheckedPackageV2;
use serde_json::{json, Value};

use super::checked_handoff::{admitted, request, tree, KINDS};
use super::temporal_package_route::{
    clause_ids, event_clauses, event_clauses_over, recipe, request as evidence_request, run,
};
use quire_spec_language::checked_handoff::{evidence, write, Corpus};

use super::bare_equality::tc1070 as population_fixture;

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

/// FR-531-AC-1, FR-531-AC-3, FR-531-AC-8: the package read from the handoff
/// directory gives a `BarePackage` and a temporal call that returns a result;
/// `identities.json` is the format's file, written deterministically.
#[trace("FR-531-AC-8", "FR-517-AC-1", "TC-1331")]
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
    assert_eq!(file["format"], "quire.checked-handoff.identities/2");
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

#[trace("TC-1331", "FR-531-AC-21", "FR-517-AC-23")]
#[test]
fn the_identity_handoff_retains_unused_populations_and_every_selected_root() {
    let holder = tempfile::tempdir().unwrap();
    let source = holder.path().join("unit.native");
    let model = holder.path().join("orders.json");
    let text = population_fixture::SOURCE.replace(
        "model M = \"test/orders\";",
        "model M = \"test/orders\";\nmodel Extra = \"agent-ix/checked-handoff\";",
    );
    std::fs::write(&source, &text).unwrap();
    std::fs::write(
        &model,
        serde_json::to_vec(&population_fixture::document_ep()).unwrap(),
    )
    .unwrap();
    let ordinary = request("event_position", Corpus::None);
    let asked = quire_spec_language::checked_handoff::CheckedHandoffRequest {
        emit: quire_spec_language::emit_checked_v2::Request {
            source,
            package_name: "population-handoff".into(),
            models: vec![model, ordinary.emit.models[0].clone()],
            libraries: Vec::new(),
            execution_points: None,
            temporal_selections: Vec::new(),
            out: None,
        },
        corpus: Corpus::None,
    };
    let out = holder.path().join("handoff");
    write(&asked, &out).expect("the genuine two-model handoff is written");
    let files = tree(&out);
    let package = admitted(&files["checked-package.json"], &evidence(&asked).unwrap());
    let rows = ReadIdentityRows::read(&files["identities.json"], &package)
        .expect("the required catalogs read");
    let catalogs = rows.selected_populations();
    assert_eq!(
        catalogs.len(),
        2,
        "even the unused selected model has a root catalog"
    );
    let extra = catalogs
        .iter()
        .find(|catalog| catalog.selected_document.as_ref() == "agent-ix/checked-handoff")
        .unwrap();
    assert!(extra.populations.is_empty());
    let orders = catalogs
        .iter()
        .find(|catalog| catalog.selected_document.as_ref() == "test/orders")
        .unwrap();
    assert_eq!(orders.populations.len(), 4);
    let unused = orders
        .populations
        .iter()
        .find(|population| population.original.node().ends_with("otherSpare"))
        .expect("an unused original population is retained");
    assert_eq!(unused.members.len(), 1);
    let member = &unused.members[0];
    assert!(member.original.node().ends_with("Other"));
    assert_eq!(
        member.effective_type.owner_chain(),
        vec![member.original.clone()]
    );
    for catalog in catalogs {
        assert!(package
            .graph()
            .nodes
            .iter()
            .any(|node| node.node_id == catalog.model_root));
        assert!(catalog
            .populations
            .windows(2)
            .all(|pair| pair[0].original < pair[1].original));
        for population in &catalog.populations {
            assert!(!population.members.is_empty());
            assert!(population.members.iter().all(|member| member
                .effective_type
                .owner_chain()
                .len()
                == 1));
        }
    }
    assert_eq!(rows.to_bytes(&package).unwrap(), files["identities.json"]);
    let second = holder.path().join("second");
    write(&asked, &second).unwrap();
    assert_eq!(tree(&second)["identities.json"], files["identities.json"]);
    assert!(
        !text.contains("otherSpare"),
        "the source does not select the unused population"
    );
}

/// FR-531-AC-12, FR-531-AC-13 (TC-1331): the identity file's refusals are
/// closed and name what they found; a forged row that passes the joins is
/// accepted.
#[trace("FR-531-AC-12", "FR-531-AC-13", "TC-1331")]
#[test]
fn the_identity_file_refuses_what_it_cannot_join() {
    let (_holder, files, package) = directory("event_position");
    let good: Value = serde_json::from_slice(&files["identities.json"]).expect("JSON");
    let read = |value: &Value| {
        ReadIdentityRows::read(&serde_json::to_vec(value).expect("bytes"), &package)
    };
    let rows = read(&good).expect("the genuine file reads");

    let malformed = |member: IdentityMember| Err(IdentityFileRefusal::Malformed { member });
    assert_eq!(
        ReadIdentityRows::read(b"not json", &package),
        malformed(IdentityMember::File)
    );
    let mut unknown = good.clone();
    unknown["extra"] = json!(1);
    assert_eq!(
        read(&unknown),
        malformed(IdentityMember::Unknown("extra".to_owned()))
    );
    let repeated = String::from_utf8(files["identities.json"].clone())
        .expect("UTF-8")
        .replacen("\"fields\"", "\"fields\": [], \"fields\"", 1);
    assert_eq!(
        ReadIdentityRows::read(repeated.as_bytes(), &package),
        malformed(IdentityMember::Repeated("fields"))
    );
    let mut format = good.clone();
    format["format"] = json!("quire.checked-handoff.identities/1");
    assert_eq!(
        read(&format),
        Err(IdentityFileRefusal::Format(
            "quire.checked-handoff.identities/1".to_owned()
        ))
    );

    let declarations = good["declarations"].as_array().expect("rows");
    assert!(!declarations.is_empty(), "R selects declarations");
    let unjoined = |index: usize| {
        Err(IdentityFileRefusal::Unjoined {
            member: IdentityMember::Declarations,
            index,
        })
    };
    let mut absent = good.clone();
    absent["declarations"][0]["structural_node"]["package"] = json!("another-package");
    assert_eq!(read(&absent), unjoined(0));

    // A form that does not match its identity.
    let first = &declarations[0];
    let mut mismatched = good.clone();
    mismatched["declarations"][0]["form"] = json!(if first["form"] == "union" {
        "object_type"
    } else {
        "union"
    });
    assert_eq!(read(&mismatched), unjoined(0));

    // A declared unit: another `kind` is malformed, an absent node and a
    // mismatched identity kind are unjoined.
    let unit_row = |value: Value, kind: &str| {
        let mut row = first.clone();
        row["form"] = json!("declared_unit");
        row["exact_identity"] = json!({ "kind": kind, "value": value });
        let mut file = good.clone();
        file["declarations"] = json!([row]);
        file
    };
    let node = first["structural_node"].clone();
    assert!(read(&unit_row(
        json!({"kind": "declared", "node": node}),
        "declared_unit"
    ))
    .is_ok());
    assert_eq!(
        read(&unit_row(
            json!({"kind": "compound", "node": node}),
            "declared_unit"
        )),
        malformed(IdentityMember::ExactIdentity)
    );
    let mut gone = node.clone();
    gone["package"] = json!("another-package");
    assert_eq!(
        read(&unit_row(
            json!({"kind": "declared", "node": gone}),
            "declared_unit"
        )),
        unjoined(0)
    );
    assert_eq!(read(&unit_row(node.clone(), "record")), unjoined(0));

    // A forged row that passes the joins is accepted: `read` makes no claim
    // of authority.
    let at = declarations
        .iter()
        .position(|row| {
            matches!(
                row["exact_identity"]["kind"].as_str(),
                Some("record" | "union")
            )
        })
        .expect("R selects a record or union declaration");
    let other = declarations
        .iter()
        .map(|row| &row["structural_node"])
        .find(|other| **other != declarations[at]["structural_node"])
        .expect("a second node of the package");
    let mut forged = good.clone();
    forged["declarations"][at]["exact_identity"]["value"] = other.clone();
    let forged_rows = read(&forged).expect("a joining forgery is accepted");
    assert_ne!(forged_rows, rows);
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

/// The names of the public methods of the `impl` of `type_name` (or the
/// methods of the trait `type_name`) in the Rust source `text`, read with
/// `syn`.
fn method_names(text: &str, type_name: &str) -> Vec<String> {
    let file = syn::parse_file(text).expect("the source parses");
    let mut names = Vec::new();
    for item in &file.items {
        match item {
            syn::Item::Impl(item) if item.trait_.is_none() => {
                if let syn::Type::Path(path) = &*item.self_ty {
                    if path
                        .path
                        .segments
                        .last()
                        .is_some_and(|s| s.ident == type_name)
                    {
                        for member in &item.items {
                            if let syn::ImplItem::Fn(method) = member {
                                if matches!(method.vis, syn::Visibility::Public(_)) {
                                    names.push(method.sig.ident.to_string());
                                }
                            }
                        }
                    }
                }
            }
            syn::Item::Trait(item) if item.ident == type_name => {
                for member in &item.items {
                    if let syn::TraitItem::Fn(method) = member {
                        names.push(method.sig.ident.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    names
}

/// FR-531-AC-9 (TC-1331): every public method of `EvidencePackage` and every
/// method of the `CheckedPackageEvaluation` trait, read from the crate source
/// with `syn`, has a row in the capability inventory; a method with no row
/// fails the test.
#[trace("FR-531-AC-9", "TC-1331")]
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
    let mut methods = method_names(
        &read_source("qsl-eval/src/temporal/evidence/package.rs"),
        "EvidencePackage",
    );
    methods.extend(method_names(
        &read_source("qsl-eval/src/value/expression/mod.rs"),
        "CheckedPackageEvaluation",
    ));
    assert!(methods.len() >= 12, "{methods:?}");
    for name in &methods {
        assert!(
            firsts.iter().any(|row| row.contains(&format!("{name}`"))),
            "`{name}` has no row in the capability inventory"
        );
    }
    // The probe is real: a method the inventory lacks is found missing.
    assert!(!firsts.iter().any(|row| row.contains("`not_a_method`")));
    let fake = method_names(
        "impl EvidencePackage { pub fn not_a_method(&self) {} }",
        "EvidencePackage",
    );
    assert_eq!(fake, ["not_a_method"]);
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
    let constructors = method_names(&bare, "BarePackage");
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

/// FR-531-AC-4 (temporal half): a node that is not a temporal clause, and a node the package does not
/// hold, refuse `clause-root-unresolved` over the bare package with an empty
/// charge log, and the same call over a clause evaluates.
#[trace("FR-531-AC-4", "TC-1331")]
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

/// FR-532-AC-12 (capture half): with the identity rows of the unit in the
/// supply, the bare route projects a record-typed capture to the type the
/// in-process clause declares.
#[test]
fn the_supplied_identity_rows_project_a_record_typed_capture_on_the_bare_route() {
    let built = record_capture();
    let read = built.read(&built.emission);
    let rows = built
        .emission
        .identity_rows(read.admitted())
        .expect("final identity rows");
    let graph = read.source().graph();
    let bare = BarePackage::new(read.admitted()).with_supply(qsl_package::BareSupply {
        identities: &rows,
        equality: None,
    });
    let package = EvidencePackage::from_package(&bare).expect("bare");
    let [clause] = clause_ids(&package).try_into().expect("one clause");
    let subject = package
        .subject(&clause, None)
        .expect("the capture projects");
    let held: Vec<_> = subject
        .facts
        .captures()
        .map(|(name, ty)| (name.to_owned(), ty.clone()))
        .collect();
    let source = graph
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "Due")
        .expect("Due");
    let declared: Vec<_> = source
        .clause()
        .captures()
        .map(|(name, ty)| (name.to_owned(), ty.clone()))
        .collect();
    assert_eq!(held, declared);
}

const LEAF: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.model.complete/v1\";\n\
    model Config = \"example/config-version\";\n\
    predicate Leaf using v (x: Boolean): Boolean { x }";

fn through(import: &str, name: &str, callee: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         {import}\nmodel Config = \"example/config-version\";\n\
         predicate {name} using v (x: Boolean): Boolean {{ {callee}(x) }}"
    )
}

const CLAUSE: &str =
    "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
     always[0,2] holds(%CALL%(c.versionNumber >= 1)) }";

/// IR-1325 AC3: the closure over a chain (root, mid, leaf) and a diamond
/// (root, left and right, one shared leaf) is read from the retained direct
/// dependencies, nearest first, each retained package once.
#[test]
fn the_dependency_closure_follows_a_chain_and_lists_a_diamond_once() {
    let names = |built: &super::temporal_package_route::Built| {
        let read = built.read(&built.emission);
        let direct: Vec<String> = read
            .admitted()
            .direct_dependencies()
            .map(|package| package.name().to_owned())
            .collect();
        let bare = BarePackage::new(read.admitted());
        let closure: Vec<String> = bare
            .dependencies()
            .iter()
            .map(|package| package.name().to_owned())
            .collect();
        (direct, closure)
    };
    let mid = through("import \"t/leaf\" as K;", "Mid", "K::Leaf");
    let chain = event_clauses_over(
        "import \"t/mid\" as M;\n",
        &[("t/leaf", LEAF), ("t/mid", mid.as_str())],
        &CLAUSE.replace("%CALL%", "M::Mid"),
        &[("Due", "authority")],
    );
    let (direct, closure) = names(&chain);
    assert_eq!(direct, ["t/mid"]);
    assert_eq!(
        closure,
        ["t/mid", "t/leaf"],
        "the chain's second hop is read"
    );

    let left = through("import \"t/leaf\" as K;", "Left", "K::Leaf");
    let right = through("import \"t/leaf\" as K;", "Right", "K::Leaf");
    let diamond = event_clauses_over(
        "import \"t/left\" as A;\nimport \"t/right\" as B;\n",
        &[
            ("t/leaf", LEAF),
            ("t/left", left.as_str()),
            ("t/right", right.as_str()),
        ],
        "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
         always[0,2] holds(A::Left(c.versionNumber >= 1) and B::Right(c.versionNumber >= 1)) }",
        &[("Due", "authority")],
    );
    let (direct, closure) = names(&diamond);
    assert_eq!(direct, ["t/left", "t/right"]);
    assert_eq!(
        closure,
        ["t/left", "t/right", "t/leaf"],
        "the shared leaf is listed once"
    );
}

const ONE_CLAUSE: &str =
    "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { \
     always[0,3] holds(c.versionNumber >= 1) }";

/// Rewrite the interval of the `always` node of `built`'s package.
fn with_interval(
    built: &super::temporal_package_route::Built,
    interval: serde_json::Value,
) -> qsl_package::Emission {
    forged(built, |wire| {
        for node in wire["semantic_graph"]["nodes"]
            .as_array_mut()
            .expect("nodes")
        {
            if node["body"]["operation"]["identity"] == "quire.op.temporal.always" {
                node["body"]["operation"]["member"]["interval"] = interval.clone();
            }
        }
    })
}

/// FR-532-AC-2 (clock row), FR-532-AC-7 (TC-1332): a package the reader
/// admits with one clock row replaced gives `from_package` the result of the
/// replaced row (a request that asserts the genuine row is refused
/// `binding-mismatch`), while `from_emission` refuses the construction with
/// `PackageMismatch::Bindings`. An interval replaced in a formula node is
/// refused by the reader (`OperationMemberMismatch`: the node's operation
/// member is part of its identity), so the unbounded-operator reason of
/// `clause-unreadable` cannot be met by an admitted package; its
/// `ClauseFormula` is unit-tested in `temporal_causes`.
#[trace("FR-532-AC-2", "TC-1332")]
#[test]
fn bytes_that_differ_from_the_source_drive_the_bare_result() {
    let built = event_clauses(ONE_CLAUSE, &[("Due", "authority")]);
    let genuine_read = built.read(&built.emission);
    let genuine_bare = BarePackage::new(genuine_read.admitted());
    let genuine = EvidencePackage::from_package(&genuine_bare).expect("bare");
    let [clause] = clause_ids(&genuine).try_into().expect("one clause");
    let asked = evidence_request(&genuine, &clause, None, 5);
    assert!(run(&genuine, &clause, &asked).0.is_ok());

    let renamed = forged(&built, |wire| {
        wire["temporal_bindings"][0]["clock"]["sequence_authority"] = json!("another-authority");
    });
    let read = built.read(&renamed);
    let bare = BarePackage::new(read.admitted());
    let package = EvidencePackage::from_package(&bare).expect("bare");
    let (refused, charges) = run(&package, &clause, &asked);
    assert_eq!(
        super::temporal_package_route::cause(&refused.expect_err("another clock row")),
        "binding-mismatch"
    );
    assert!(charges.is_empty());
    // A request that asserts the replaced row evaluates.
    let own = evidence_request(&package, &clause, None, 5);
    assert!(run(&package, &clause, &own).0.is_ok());
    // The route that holds the genuine emission refuses the construction.
    assert!(matches!(
        EvidencePackage::from_emission(&built.emission, &read),
        Err(qsl_eval::temporal::evidence::PackageMismatch::Bindings)
    ));

    // One finite interval replaced by another: the bytes drive the result,
    // and the routes that hold the genuine source refuse the difference.
    let longer = with_interval(&built, json!({"lower": "0", "upper": "4"}));
    let read = built.read(&longer);
    let bare = BarePackage::new(read.admitted());
    let package = EvidencePackage::from_package(&bare).expect("bare");
    let (baseline, _) = run(&genuine, &clause, &asked);
    let baseline = baseline.expect("the genuine package evaluates");
    let (replaced, _) = run(&package, &clause, &asked);
    let replaced = replaced.expect("the replaced interval evaluates");
    assert_ne!(
        format!("{:?}", replaced.usage),
        format!("{:?}", baseline.usage),
        "the replaced interval changes the work the call charges"
    );
    for source_route in [
        EvidencePackage::from_read(&read).expect("read"),
        EvidencePackage::from_emission(&longer, &read).expect("emission"),
    ] {
        let mut meter = super::temporal_package_route::unlimited();
        let refused = source_route.evaluate_temporal_evidence(&clause, &asked, &mut meter);
        let refused = refused.expect_err("the source differs from the bytes");
        assert_eq!(
            super::temporal_package_route::cause(&refused),
            "clause-unreadable"
        );
        assert_eq!(reason(&refused), "formula-differs");
        assert!(meter.admitted_charges().is_empty());
    }
    // An operator without a finite upper bound is a reader refusal.
    let unbounded = with_interval(&built, json!({"lower": "0", "upper": null}));
    assert!(built.try_read(&unbounded).is_err());
}

/// FR-531-AC-6 (TC-1331): Contract IR refuses a package whose clause has no
/// `temporal_bindings` row (`TimedNodeReference`) and a lock with no
/// `profile_selections` row, before any evaluation, so neither the absent
/// clock row's `Unavailable` result nor the `profile-selection` reason can be
/// met by an admitted package; the reader's refusals are the behavior.
#[trace("TC-1331")]
#[test]
fn the_reader_refuses_absent_clock_rows_and_a_missing_profile_row() {
    let built = event_clauses(ONE_CLAUSE, &[("Due", "authority")]);
    let no_clock = forged(&built, |wire| {
        wire.as_object_mut()
            .expect("object")
            .remove("temporal_bindings");
    });
    let refused = built.try_read(&no_clock).err().expect("the reader refuses");
    assert!(format!("{refused:?}").contains("TimedNodeReference"));

    let no_profile = forged(&built, |wire| {
        wire["lock"]["profile_selections"] = json!([]);
    });
    let refused = built
        .try_read(&no_profile)
        .err()
        .expect("the reader refuses");
    // The refusal names the lock or the nodes that select the removed row.
    assert!(!format!("{refused:?}").is_empty());
}

/// FR-532-AC-3 (TC-1332): one clause per operator of FR-337's temporal list
/// gives, over the three routes, the same result and charges for each of a
/// set of traces that distinguish the operator.
#[trace("FR-532-AC-3", "TC-1332")]
#[test]
fn every_temporal_operator_gives_the_same_result_over_the_three_routes() {
    use qsl_eval::temporal::evidence::EvidencePosition;
    use qsl_eval::temporal::Valuation;
    let p = "holds(c.versionNumber >= 1)";
    let q = "holds(c.versionNumber >= 2)";
    let bodies = [
        format!("eventually[0,2] {p}"),
        format!("always[0,2] {p}"),
        format!("once[0,2] {p}"),
        format!("historically[0,2] {p}"),
        format!("{p} until[0,2] {q}"),
        format!("{p} release[0,2] {q}"),
        format!("{p} since[0,2] {q}"),
        format!("{p} triggered[0,2] {q}"),
        format!("not {p}"),
        format!("{p} and {q}"),
        format!("{p} or {q}"),
        format!("{p} implies {q}"),
    ];
    use Valuation::{False as F, True as T};
    let traces: [[(Valuation, Valuation); 3]; 4] = [
        [(T, T), (T, T), (T, T)],
        [(F, F), (F, F), (F, F)],
        [(T, F), (F, T), (T, F)],
        [(F, T), (T, T), (F, F)],
    ];
    let mut outcomes = std::collections::BTreeSet::new();
    for body in &bodies {
        let built = event_clauses(
            &format!(
                "temporal Due using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin {{ {body} }}"
            ),
            &[("Due", "authority")],
        );
        let read = built.read(&built.emission);
        let emitted = EvidencePackage::from_emission(&built.emission, &read).expect("emission");
        let back = EvidencePackage::from_read(&read).expect("read");
        let bare = BarePackage::new(read.admitted());
        let bytes = EvidencePackage::from_package(&bare).expect("bare");
        let [clause] = clause_ids(&bytes).try_into().expect("one clause");
        for trace in &traces {
            let mut asked = evidence_request(&emitted, &clause, None, 3);
            asked.positions = trace
                .iter()
                .enumerate()
                .map(|(index, (first, second))| EvidencePosition {
                    observation: qsl_eval::temporal::evidence::EvidenceRef::named(format!(
                        "obs{index}"
                    )),
                    position: index as u64,
                    valuations: [(0, first.clone()), (1, second.clone())]
                        .into_iter()
                        .filter(|(atom, _)| {
                            *atom
                                < emitted
                                    .subject(&clause, None)
                                    .expect("reads")
                                    .facts
                                    .formula()
                                    .atom_count() as u64
                        })
                        .collect(),
                    ..EvidencePosition::default()
                })
                .collect();
            let (left, left_charges) = run(&emitted, &clause, &asked);
            let (middle, middle_charges) = run(&back, &clause, &asked);
            let (right, right_charges) = run(&bytes, &clause, &asked);
            let (left, middle, right) = (
                left.unwrap_or_else(|failure| panic!("{body}: {failure:?}")),
                middle.unwrap_or_else(|failure| panic!("{body}: {failure:?}")),
                right.unwrap_or_else(|failure| panic!("{body}: {failure:?}")),
            );
            assert_eq!(format!("{left:?}"), format!("{right:?}"), "{body}");
            assert_eq!(format!("{middle:?}"), format!("{right:?}"), "{body}");
            assert_eq!(left_charges, right_charges, "{body}");
            assert_eq!(middle_charges, right_charges, "{body}");
            outcomes.insert((
                body.clone(),
                format!("{:?}/{:?}", right.truth, right.settlement),
            ));
        }
    }
    let distinct: std::collections::BTreeSet<_> = outcomes
        .iter()
        .map(|(_, outcome)| outcome.clone())
        .collect();
    assert!(
        distinct
            .iter()
            .any(|outcome| outcome.starts_with("Some(True)"))
            && distinct
                .iter()
                .any(|outcome| outcome.starts_with("Some(False)")),
        "the traces distinguish the operators: {distinct:?}"
    );
}

/// FR-532-AC-5 (replay route): the replay route has no emitted package, and
/// binds progress under the identity string of the unit's source. Where an
/// emitted package for the same source exists, that string is its
/// `package.name()`, on all three routes.
#[trace("FR-532-AC-5", "TC-1332")]
#[test]
fn the_replay_binding_names_the_package_the_source_emits() {
    for kind in KINDS {
        let built = recipe(kind);
        let read = built.read(&built.emission);
        let bare = BarePackage::new(read.admitted());
        let package = EvidencePackage::from_package(&bare).expect("bare");
        for clause in clause_ids(&package) {
            let subject = package.subject(&clause, None).expect("reads");
            assert_eq!(
                subject.progress.package,
                built.replay_package_identity(),
                "{kind}"
            );
        }
        assert_eq!(read.admitted().name(), built.replay_package_identity());
    }
}
