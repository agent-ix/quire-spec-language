// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-860, TC-864, TC-865 and TC-866 (FR-340, FR-343, FR-344): the
//! spec-versioning refinement gate over corpora built from FR-108's
//! ConfigVersion unit, package and snapshots.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::source::provenance::RawSourceRef;
use serde_json::{json, Value};
use xtask::refinement::compare::CaseResult;
use xtask::refinement::report::{CaseEntry, CaseName, Report, SelectionName, SideReport, Verdict};
use xtask::refinement::versioning;

#[path = "../../examples/config-version/spine_unit.rs"]
mod config_version;

const AUTHORITY: &str = "agent-ix";
const IDENTITY: &str = "ix://example/config-version/spine/unit";
/// The ConfigVersion domain package, named in place by its repository path.
const PACKAGE: &str = "repo:examples/config-version/model.semantic-ir.json";
const POPULATION: &str = "ix://example/config-version/config_history";
const PARENT_ORDER: &str = "deref(value(self.parent)).versionNumber < self.versionNumber";

fn model_digest_hex() -> String {
    let packages =
        qsl_semantics::model::intake::package_input([config_version::DOMAIN_PACKAGE.as_bytes()]);
    let digest = packages.keys().next().expect("one supplied package");
    qsl_semantics::model::key::hex(digest)
}

fn unit() -> String {
    config_version::unit_text(&model_digest_hex())
}

/// The unit with `ParentOrder` strengthened so the healthy-parent snapshot
/// violates it (the parent must now be newer than the child).
fn tightened() -> String {
    let tight = unit().replace(
        PARENT_ORDER,
        "deref(value(self.parent)).versionNumber > self.versionNumber",
    );
    assert_ne!(tight, unit(), "the clause is strengthened");
    tight
}

/// The unit with `ParentOrder` and `sameIdentity` removed.
fn dropped() -> String {
    let kept: Vec<String> = unit()
        .lines()
        .filter(|line| {
            !line.starts_with("invariant ParentOrder") && !line.starts_with("function sameIdentity")
        })
        .map(str::to_owned)
        .collect();
    assert_eq!(kept.len() + 2, unit().lines().count());
    kept.join("\n") + "\n"
}

/// The unit with an ill-typed `sameIdentity` body.
fn broken() -> String {
    let broken = unit().replace("Boolean pure { a = b }", "Boolean pure { 1 }");
    assert_ne!(broken, unit());
    broken
}

/// The unit with a protocol whose attempt body the checker does not read yet
/// (`unsupported_construct`/`not-yet-implemented`).
fn unsupported() -> String {
    unit()
        + "protocol Flow using v over (input: Boolean) on origin {\n\
           role R on Config::ConfigVersion;\n\
           run sequence Main {\n\
           attempt Tried by R on Config::ConfigVersion::attemptUpdate contracts [] \
           as (tried: Boolean) { 1 + true };\n\
           }\n\
           finish End as (outcome: Boolean) { true };\n\
           }\n"
}

/// The unit with more work in `ParentOrder` and the same meaning.
fn heavier() -> String {
    let heavy = unit().replace(
        "present(self.parent) implies",
        "not not present(self.parent) implies",
    );
    assert_ne!(heavy, unit());
    heavy
}

fn snapshot(name: &str, rows: &[(&str, i64, Option<&str>)]) -> Value {
    let objects: Vec<Value> = rows
        .iter()
        .map(|(key, version, parent)| {
            let parent = match parent {
                Some(parent) => {
                    json!({"present": {"reference": {"population": POPULATION, "key": parent}}})
                }
                None => json!({"absent": {}}),
            };
            json!({
                "key": key,
                "type": "ix://example/config-version/ConfigVersion",
                "fields": {"versionNumber": {"integer": version.to_string()}, "parent": parent},
            })
        })
        .collect();
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": "agent-ix",
            "identity": format!("ix://example/config-version/spine/current/{name}"),
            "revision_namespace": "example",
            "revision": "1",
        },
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {
            "identity": config_version::PACKAGE_IDENTITY,
            "version": config_version::PACKAGE_VERSION,
            "digest": format!("sha256-jcs:{}", model_digest_hex()),
        },
        "populations": [{"population": POPULATION, "complete": true, "objects": objects}],
    })
}

fn healthy() -> Value {
    snapshot(
        "healthy-parent",
        &[("root", 1, None), ("child", 2, Some("root"))],
    )
}

fn violating() -> Value {
    snapshot(
        "violating-parent",
        &[("root", 3, None), ("child", 2, Some("root"))],
    )
}

/// A `ParentOrder` case over the snapshot file `<observation>.json`.
fn clause_case(observation: &str) -> Value {
    json!({
        "selection": {"clause": {
            "name": "ParentOrder",
            "observation": observation,
            "self": {"population": POPULATION, "key": "child"},
            "anchor": {"kind": "handler", "name": "validate"},
        }},
        "observations": {observation: format!("{observation}.json")},
    })
}

/// A `sameIdentity(child, child)` case over the healthy snapshot.
fn function_case() -> Value {
    let child = json!({"reference": {"population": POPULATION, "key": "child"}});
    json!({
        "selection": {"function": {
            "name": "sameIdentity",
            "arguments": [
                {"parameter": "a", "value": child},
                {"parameter": "b", "value": child},
            ],
            "snapshot": "healthy-parent",
        }},
        "observations": {"healthy-parent": "healthy-parent.json"},
    })
}

fn with(mut case: Value, member: &str, value: Value) -> Value {
    case[member] = value;
    case
}

/// One pair of a corpus: both units, the labels they share and the cases.
struct Pair<'a> {
    prior: String,
    superseding: String,
    identity: &'a str,
    cases: Vec<Value>,
}

impl<'a> Pair<'a> {
    fn new(prior: String, superseding: String, cases: Vec<Value>) -> Self {
        Self {
            prior,
            superseding,
            identity: IDENTITY,
            cases,
        }
    }

    fn json(&self) -> Value {
        let source =
            |path: &str| json!({"path": path, "authority": AUTHORITY, "identity": self.identity});
        json!({
            "prior": source("prior.native"),
            "superseding": source("superseding.native"),
            "packages": [PACKAGE],
            "dependencies": [],
            "cases": self.cases,
        })
    }

    fn write(&self, directory: &Path) {
        fs::create_dir_all(directory).unwrap();
        fs::write(directory.join("prior.native"), &self.prior).unwrap();
        fs::write(directory.join("superseding.native"), &self.superseding).unwrap();
        fs::write(directory.join("healthy-parent.json"), healthy().to_string()).unwrap();
        fs::write(
            directory.join("violating-parent.json"),
            violating().to_string(),
        )
        .unwrap();
        fs::write(directory.join("pair.json"), self.json().to_string()).unwrap();
    }
}

fn corpus(pairs: &[(&str, Pair<'_>)]) -> tempfile::TempDir {
    let corpus = tempfile::tempdir().unwrap();
    for (name, pair) in pairs {
        pair.write(&corpus.path().join(name));
    }
    corpus
}

fn identity_pair() -> Pair<'static> {
    Pair::new(
        unit(),
        unit(),
        vec![
            clause_case("healthy-parent"),
            clause_case("violating-parent"),
        ],
    )
}

fn seed_pair() -> Pair<'static> {
    Pair::new(
        unit(),
        tightened(),
        vec![
            clause_case("healthy-parent"),
            clause_case("violating-parent"),
        ],
    )
}

fn json_of(report: &Report) -> Value {
    serde_json::from_str(&report.to_json().unwrap()).unwrap()
}

fn listed(report: &Report) -> Vec<Value> {
    json_of(report)["cases"].as_array().unwrap().clone()
}

/// The observation (or snapshot) key a case entry's selection names.
fn key_of(entry: &Value) -> String {
    let selection = &entry["name"]["selection"];
    let member = selection
        .get("clause")
        .map(|clause| &clause["observation"])
        .or_else(|| {
            selection
                .get("function")
                .map(|function| &function["snapshot"])
        })
        .expect("a clause or function selection");
    member.as_str().unwrap().to_owned()
}

/// `(selection kind, key, result)` of every case entry, in report order.
fn results(report: &Report) -> Vec<(String, String, String)> {
    listed(report)
        .iter()
        .filter(|entry| entry.get("name").is_some())
        .map(|entry| {
            let selection = entry["name"]["selection"].as_object().unwrap();
            let kind = selection.keys().next().unwrap().clone();
            (
                kind,
                key_of(entry),
                entry["result"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn triple(kind: &str, key: &str, result: &str) -> (String, String, String) {
    (kind.to_owned(), key.to_owned(), result.to_owned())
}

fn copy_directory(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_directory(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn real_corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/refinement/versioning")
}

/// Run the built binary as `make refinement-versioning` does and return its
/// exit code.
fn exit_code(corpus: &Path) -> i32 {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["refinement", "versioning"])
        .arg(corpus)
        .output()
        .unwrap()
        .status
        .code()
        .expect("the gate exits rather than being killed")
}

/// FR-340-AC-1: a well-formed pair with two `Clause` cases runs each against
/// both revisions, and the report holds exactly two case results naming the
/// pair's two source references and the selection.
#[test]
#[trace("TC-860", "FR-340-AC-1")]
fn a_well_formed_pair_reports_one_result_per_case() {
    let corpus = corpus(&[("pair", identity_pair())]);
    let report = versioning::run(corpus.path(), &repository());
    let entries = listed(&report);
    assert_eq!(entries.len(), 2);
    let names: Vec<&Value> = entries.iter().map(|entry| &entry["name"]).collect();
    for name in &names {
        for side in ["prior", "superseding"] {
            assert_eq!(name[side]["authority"], AUTHORITY);
            assert_eq!(name[side]["identity"], IDENTITY);
            assert_eq!(name[side]["digest_domain"], "quire.source.bytes/v1");
            assert_eq!(name[side]["digest"].as_str().unwrap().len(), 64);
        }
        assert_eq!(name["selection"]["clause"]["name"], "ParentOrder");
    }
    assert_eq!(
        results(&report),
        [
            triple("clause", "healthy-parent", "holds"),
            triple("clause", "violating-parent", "not applicable"),
        ]
    );
    // Both runs compiled and evaluated: no run was given a package identity
    // to disagree with.
    assert_eq!(entries[0]["prior"]["class"], "admitted");
    assert_eq!(entries[0]["prior"]["stage"], "evaluate");
    assert_eq!(entries[0]["superseding"]["class"], "admitted");
    assert_eq!(report.verdict, Verdict::Success);
    assert_eq!(report.exit, 0);
}

/// FR-340-AC-2: each of six malformed pairs is one tool-failure result naming
/// its `pair.json` and defect, none of its cases runs, and the well-formed
/// pair still runs; a corpus with no pair is one tool failure naming the
/// corpus path; both exit 30.
#[test]
#[trace("TC-860", "FR-340-AC-2")]
fn each_malformed_pair_is_one_tool_failure_and_the_rest_still_run() {
    let corpus = corpus(&[("good", identity_pair())]);
    let root = corpus.path();
    let write_pair = |name: &str, text: &str| {
        fs::create_dir_all(root.join(name)).unwrap();
        fs::write(root.join(name).join("pair.json"), text).unwrap();
    };
    let base = identity_pair().json();
    let without = |member: &str| {
        let mut pair = base.clone();
        pair.as_object_mut().unwrap().remove(member);
        pair.to_string()
    };
    write_pair("not-json", "{");
    write_pair("missing-superseding", &without("superseding"));
    let mut extra = base.clone();
    extra["expected"] = json!("anything");
    write_pair("extra-member", &extra.to_string());
    let mut outside = base.clone();
    outside["prior"]["path"] = json!("../outside.qsl");
    write_pair("outside", &outside.to_string());
    let mut absolute = base.clone();
    absolute["prior"]["path"] = json!("/outside.qsl");
    write_pair("absolute", &absolute.to_string());
    let mut mismatch = base.clone();
    mismatch["superseding"]["identity"] = json!("ix://example/another-spec");
    write_pair("identity", &mismatch.to_string());
    let mut repository_escape = base.clone();
    repository_escape["packages"] = json!(["repo:../outside"]);
    identity_pair().write(&root.join("repo-escape"));
    write_pair("repo-escape", &repository_escape.to_string());
    let mut repository_absolute = base.clone();
    repository_absolute["packages"] = json!(["repo:/etc/hostname"]);
    identity_pair().write(&root.join("repo-absolute"));
    write_pair("repo-absolute", &repository_absolute.to_string());

    let report = versioning::run(root, &repository());
    let entries = listed(&report);
    let failures: Vec<(&str, &str)> = entries
        .iter()
        .filter_map(|entry| Some((entry["pair"].as_str()?, entry["defect"].as_str()?)))
        .collect();
    let defect_of = |pair: &str| {
        let matching: Vec<&str> = failures
            .iter()
            .filter(|(name, _)| *name == pair)
            .map(|(_, defect)| *defect)
            .collect();
        assert_eq!(matching.len(), 1, "exactly one result for {pair}");
        matching[0]
    };
    assert_eq!(failures.len(), 8);
    assert!(defect_of("not-json/pair.json").contains("line 1"));
    assert!(defect_of("missing-superseding/pair.json").contains("missing field `superseding`"));
    assert!(defect_of("extra-member/pair.json").contains("unknown field `expected`"));
    assert!(defect_of("outside/pair.json")
        .contains("../outside.qsl resolves outside the pair directory"));
    assert!(defect_of("absolute/pair.json").contains("/outside.qsl is an absolute path"));
    assert!(defect_of("repo-escape/pair.json").contains("repo:../outside resolves outside"));
    assert!(defect_of("repo-absolute/pair.json").contains("repo:/etc/hostname is an absolute path"));
    let mismatch = defect_of("identity/pair.json");
    assert!(mismatch.contains(IDENTITY) && mismatch.contains("ix://example/another-spec"));
    assert!(mismatch.contains(AUTHORITY));
    // Tool-failure results come first, then the good pair's cases.
    assert!(entries[..8].iter().all(|entry| entry.get("pair").is_some()));
    assert_eq!(
        results(&report),
        [
            triple("clause", "healthy-parent", "holds"),
            triple("clause", "violating-parent", "not applicable"),
        ]
    );
    assert_eq!(report.verdict, Verdict::ToolFailure);
    assert_eq!(report.exit, 30);

    let empty = tempfile::tempdir().unwrap();
    let report = versioning::run(empty.path(), &repository());
    let entries = listed(&report);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["corpus"], empty.path().display().to_string());
    assert_eq!(entries[0]["result"], "tool failure");
    assert_eq!(report.exit, 30);
    assert_eq!(exit_code(empty.path()), 30);
}

/// FR-340-AC-3: the pair's direction is the one its members name.
#[test]
#[trace("TC-860", "FR-340-AC-3")]
fn the_pair_is_compared_in_the_direction_its_members_name() {
    let forward = corpus(&[(
        "pair",
        Pair::new(unit(), tightened(), vec![clause_case("healthy-parent")]),
    )]);
    let report = versioning::run(forward.path(), &repository());
    assert_eq!(
        results(&report),
        [triple("clause", "healthy-parent", "regression")]
    );
    let entry = &listed(&report)[0];
    assert_eq!(entry["prior"]["class"], "admitted");
    assert_eq!(entry["superseding"]["class"], "refused");

    // The same two units named the other way round: the prior unit now
    // refuses the case, so it does not apply.
    let reverse = corpus(&[(
        "pair",
        Pair::new(tightened(), unit(), vec![clause_case("healthy-parent")]),
    )]);
    let report = versioning::run(reverse.path(), &repository());
    assert_eq!(
        results(&report),
        [triple("clause", "healthy-parent", "not applicable")]
    );
}

/// FR-340-AC-4: a case stating no limit sets runs with the published
/// defaults; stating `accounting` with one work unit runs both revisions
/// with that budget.
#[test]
#[trace("TC-860", "FR-340-AC-4")]
fn a_stated_budget_reaches_both_runs_and_a_stated_nothing_uses_defaults() {
    let corpus_default = corpus(&[(
        "pair",
        Pair::new(unit(), unit(), vec![clause_case("healthy-parent")]),
    )]);
    let report = versioning::run(corpus_default.path(), &repository());
    assert_eq!(
        results(&report),
        [triple("clause", "healthy-parent", "holds")]
    );

    let budget = with(
        clause_case("healthy-parent"),
        "accounting",
        json!({"work_units": 1}),
    );
    let corpus_budget = corpus(&[("pair", Pair::new(unit(), unit(), vec![budget]))]);
    let report = versioning::run(corpus_budget.path(), &repository());
    assert_eq!(
        results(&report),
        [triple(
            "clause",
            "healthy-parent",
            "unresolved (incomplete)"
        )]
    );
    let entry = &listed(&report)[0];
    assert_eq!(entry["prior"]["class"], "incomplete");
    assert_eq!(entry["superseding"]["class"], "incomplete");
    assert_eq!(
        entry["limits"],
        json!([
            {"side": "prior", "name": "work_units", "bound": 1, "member": "accounting"},
            {"side": "superseding", "name": "work_units", "bound": 1, "member": "accounting"},
        ])
    );
    assert_eq!(report.verdict, Verdict::Incomplete);
    assert_eq!(report.exit, 22);
}

/// FR-340-AC-5: a copy of the corpus elsewhere gives a byte-equal report.
#[test]
#[trace("TC-860", "FR-340-AC-5")]
fn a_copied_corpus_gives_a_byte_equal_report() {
    let original = corpus(&[("pair", seed_pair())]);
    let copy = tempfile::tempdir().unwrap();
    copy_directory(original.path(), copy.path());
    let first = versioning::run(original.path(), &repository())
        .to_json()
        .unwrap();
    let second = versioning::run(copy.path(), &repository())
        .to_json()
        .unwrap();
    assert_eq!(first, second);
}

/// FR-343-AC-2, FR-344-AC-5 and FR-340-AC-6 (seeded half): the seeded pair
/// tightens a clause. Its healthy-parent case is the one regression; the
/// violating-parent case, which the prior revision refuses, does not apply;
/// the identity pair holds. The gate exits 10.
#[test]
#[trace("TC-864", "FR-343-AC-2", "FR-344-AC-5", "FR-340-AC-6")]
fn a_seeded_regression_fails_the_gate_naming_exactly_that_case() {
    let corpus = corpus(&[("identity", identity_pair()), ("seed", seed_pair())]);
    let report = versioning::run(corpus.path(), &repository());
    assert_eq!(report.verdict, Verdict::Violation);
    assert_eq!(report.exit, 10);
    let entries = listed(&report);
    let regressions: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry["result"] == "regression")
        .collect();
    assert_eq!(regressions.len(), 1);
    let seeded = regressions[0];
    assert_eq!(key_of(seeded), "healthy-parent");
    assert_eq!(seeded["prior"]["class"], "admitted");
    assert_eq!(seeded["superseding"]["class"], "refused");
    assert_eq!(seeded["superseding"]["stage"], "evaluate");
    assert_eq!(seeded["superseding"]["codes"], json!([]));
    // The tightened pair's two sources differ in digest, so its name differs
    // from the identity pair's, whose two sources are the same bytes.
    assert_ne!(seeded["name"]["prior"], seeded["name"]["superseding"]);
    let identity_cases = entries
        .iter()
        .filter(|entry| entry["name"]["prior"] == entry["name"]["superseding"])
        .count();
    assert_eq!(identity_cases, 2);
    let others: Vec<String> = entries
        .iter()
        .filter(|entry| *entry != seeded)
        .map(|entry| entry["result"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(others.iter().filter(|result| *result == "holds").count(), 1);
    assert_eq!(
        others
            .iter()
            .filter(|result| *result == "not applicable")
            .count(),
        2
    );
    assert_eq!(exit_code(corpus.path()), 10);
}

/// FR-343-AC-3: a removed clause holds, a removed function regresses.
#[test]
#[trace("TC-864", "FR-343-AC-3")]
fn a_dropped_clause_holds_and_a_dropped_function_regresses() {
    let corpus = corpus(&[(
        "dropped",
        Pair::new(
            unit(),
            dropped(),
            vec![clause_case("healthy-parent"), function_case()],
        ),
    )]);
    let report = versioning::run(corpus.path(), &repository());
    assert_eq!(
        results(&report),
        [
            triple("clause", "healthy-parent", "holds"),
            triple("function", "healthy-parent", "regression"),
        ]
    );
    let function = &listed(&report)[1];
    assert_eq!(function["prior"]["class"], "admitted");
    assert_eq!(function["superseding"]["class"], "absent");
    assert_eq!(function["superseding"]["stage"], "select");
}

/// FR-343-AC-4: a typed compile refusal of the superseding revision
/// regresses every case the prior admits and does not apply to the others.
#[test]
#[trace("TC-864", "FR-343-AC-4")]
fn a_superseding_compile_refusal_regresses_every_admitted_case() {
    let corpus = corpus(&[(
        "broken",
        Pair::new(
            unit(),
            broken(),
            vec![
                clause_case("healthy-parent"),
                clause_case("violating-parent"),
            ],
        ),
    )]);
    let report = versioning::run(corpus.path(), &repository());
    assert_eq!(
        results(&report),
        [
            triple("clause", "healthy-parent", "regression"),
            triple("clause", "violating-parent", "not applicable"),
        ]
    );
    let healthy = &listed(&report)[0];
    assert_eq!(healthy["superseding"]["class"], "refused");
    assert_eq!(healthy["superseding"]["stage"], "compile");
    assert_eq!(healthy["superseding"]["codes"][0]["code"], "ill_typed");
}

/// FR-340-AC-6: the real corpus passes, and adding the seeded pair makes the
/// gate fail.
#[test]
#[trace("TC-864", "FR-340-AC-6")]
fn the_real_corpus_passes_and_the_seeded_copy_fails() {
    let real = real_corpus();
    let report = versioning::run(&real, &repository());
    assert_eq!(
        report.verdict,
        Verdict::Success,
        "{}",
        report.to_json().unwrap()
    );
    assert_eq!(exit_code(&real), 0);

    let seeded = tempfile::tempdir().unwrap();
    copy_directory(&real, seeded.path());
    seed_pair().write(&seeded.path().join("seed"));
    assert_eq!(exit_code(seeded.path()), 10);
}

/// FR-343-AC-5 (unsupported half): an unsupported superseding compile and an
/// unsupported prior compile are each `unresolved (unsupported)`, exit 21.
#[test]
#[trace("TC-865", "FR-343-AC-5")]
fn an_unsupported_compile_on_either_side_is_unresolved() {
    for (prior, superseding) in [(unit(), unsupported()), (unsupported(), unit())] {
        let corpus = corpus(&[(
            "pair",
            Pair::new(prior, superseding, vec![clause_case("healthy-parent")]),
        )]);
        let report = versioning::run(corpus.path(), &repository());
        assert_eq!(
            results(&report),
            [triple(
                "clause",
                "healthy-parent",
                "unresolved (unsupported)"
            )]
        );
        assert_eq!(report.verdict, Verdict::Unsupported);
        assert_eq!(report.exit, 21);
    }
}

/// FR-343-AC-5 (incomplete half) and FR-344-AC-4: a budget the prior run fits
/// and the superseding run exceeds is `unresolved (incomplete)` naming the
/// superseding run's `work_units`; a budget the prior run exceeds is
/// `unresolved (incomplete)`, not `not applicable`.
#[test]
#[trace("TC-865", "FR-343-AC-5", "FR-344-AC-4")]
fn an_exhausted_budget_is_unresolved_and_names_its_limit() {
    let run = |budget: u64| {
        let case = with(
            clause_case("healthy-parent"),
            "accounting",
            json!({"work_units": budget}),
        );
        let corpus = corpus(&[("pair", Pair::new(unit(), heavier(), vec![case]))]);
        let report = versioning::run(corpus.path(), &repository());
        listed(&report).remove(0)
    };
    // The smallest budget the prior run fits in.
    let (mut low, mut high) = (1_u64, 1_u64 << 20);
    while low < high {
        let middle = low + (high - low) / 2;
        if run(middle)["prior"]["class"] == "admitted" {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    let fits = low;
    let at_charge = run(fits);
    assert_eq!(at_charge["prior"]["class"], "admitted");
    assert_eq!(at_charge["superseding"]["class"], "incomplete");
    assert_eq!(at_charge["result"], "unresolved (incomplete)");
    assert_eq!(
        at_charge["limits"],
        json!([{"side": "superseding", "name": "work_units", "bound": fits, "member": "accounting"}])
    );
    let below = run(fits - 1);
    assert_eq!(below["prior"]["class"], "incomplete");
    assert_eq!(below["result"], "unresolved (incomplete)");
}

/// FR-340-AC-2 (case half): a case whose inputs cannot be read is one
/// tool-failure case result naming the case and the defect; nothing of it runs
/// and the pair's other cases still run.
#[test]
#[trace("TC-860", "FR-340-AC-2")]
fn an_unreadable_case_is_a_tool_failure_of_that_case_alone() {
    let broken = json!({
        "selection": {"clause": {
            "name": "ParentOrder",
            "observation": "missing",
            "self": {"population": POPULATION, "key": "child"},
            "anchor": {"kind": "handler", "name": "validate"},
        }},
        "observations": {"missing": "missing.json"},
    });
    let pair = Pair::new(unit(), unit(), vec![broken, clause_case("healthy-parent")]);
    let corpus = corpus(&[("pair", pair)]);
    let report = versioning::run(corpus.path(), &repository());
    assert_eq!(
        results(&report),
        [
            triple("clause", "healthy-parent", "holds"),
            triple("clause", "missing", "tool failure"),
        ]
    );
    let failed = &listed(&report)[1];
    assert!(failed["prior"].is_null() && failed["superseding"].is_null());
    assert!(failed["defect"].as_str().unwrap().contains("missing.json"));
    assert_eq!(report.verdict, Verdict::ToolFailure);
    assert_eq!(report.exit, 30);
}

/// FR-340 (case naming): two cases of one pair that differ only in the
/// selected self object are named apart.
#[test]
#[trace("TC-860", "FR-340-AC-1")]
fn cases_differing_only_in_their_self_object_have_distinct_names() {
    let at_root = {
        let mut case = clause_case("healthy-parent");
        case["selection"]["clause"]["self"]["key"] = json!("root");
        case
    };
    let pair = Pair::new(unit(), unit(), vec![clause_case("healthy-parent"), at_root]);
    let corpus = corpus(&[("pair", pair)]);
    let report = versioning::run(corpus.path(), &repository());
    let entries = listed(&report);
    assert_eq!(entries.len(), 2);
    assert_ne!(entries[0]["name"], entries[1]["name"]);
    let keys: Vec<&Value> = entries
        .iter()
        .map(|entry| &entry["name"]["selection"]["clause"]["self"]["key"])
        .collect();
    assert_eq!(keys, [&json!("child"), &json!("root")]);
}

fn dummy_reference(identity: &str) -> RawSourceRef {
    RawSourceRef::new(
        AUTHORITY,
        identity,
        DigestRecord::mint(DigestDomain::SourceBytesV1, [7; 32]),
    )
    .unwrap()
}

fn side(class: &'static str, stage: &'static str) -> Option<SideReport> {
    Some(SideReport {
        class,
        stage,
        codes: Vec::new(),
    })
}

fn entry(identity: &str, result: CaseResult) -> CaseEntry {
    CaseEntry {
        name: CaseName {
            prior: dummy_reference(identity),
            superseding: dummy_reference(identity),
            selection: SelectionName::Clause {
                name: "ParentOrder".to_owned(),
                observation: "healthy-parent".to_owned(),
                self_object: None,
                anchor: None,
                parameters: std::collections::BTreeMap::new(),
            },
        },
        prior: side("admitted", "evaluate"),
        superseding: side("refused", "evaluate"),
        limits: Vec::new(),
        defect: None,
        result,
    }
}

/// FR-344-AC-1 and FR-344-AC-2: the verdict is the highest-severity result
/// present and a regression is listed whatever the verdict.
#[test]
#[trace("TC-866", "FR-344-AC-1", "FR-344-AC-2")]
fn the_verdict_is_the_highest_severity_result_and_a_regression_is_always_listed() {
    let sets: [(&[CaseResult], Verdict, u8); 5] = [
        (&[CaseResult::Holds], Verdict::Success, 0),
        (
            &[CaseResult::Holds, CaseResult::Regression],
            Verdict::Violation,
            10,
        ),
        (
            &[CaseResult::Regression, CaseResult::UnresolvedIncomplete],
            Verdict::Incomplete,
            22,
        ),
        (
            &[
                CaseResult::Regression,
                CaseResult::UnresolvedIncomplete,
                CaseResult::UnresolvedUnsupported,
            ],
            Verdict::Unsupported,
            21,
        ),
        (
            &[
                CaseResult::Regression,
                CaseResult::UnresolvedIncomplete,
                CaseResult::UnresolvedUnsupported,
                CaseResult::ToolFailure,
            ],
            Verdict::ToolFailure,
            30,
        ),
    ];
    for (results, verdict, exit) in sets {
        let cases = results
            .iter()
            .enumerate()
            .map(|(index, result)| entry(&format!("spec-{index}"), *result))
            .collect();
        let report = Report::new(Vec::new(), Vec::new(), cases);
        assert_eq!(report.verdict, verdict);
        assert_eq!(report.exit, exit);
        let regressions = listed(&report)
            .iter()
            .filter(|entry| entry["result"] == "regression")
            .count();
        assert_eq!(
            regressions,
            results
                .iter()
                .filter(|result| **result == CaseResult::Regression)
                .count()
        );
    }
    let unsupported = Report::new(
        Vec::new(),
        Vec::new(),
        vec![
            entry("spec-a", CaseResult::Regression),
            entry("spec-b", CaseResult::UnresolvedUnsupported),
        ],
    );
    let regression = &listed(&unsupported)[0];
    assert_eq!(regression["result"], "regression");
    assert_eq!(regression["prior"]["class"], "admitted");
    assert_eq!(regression["superseding"]["class"], "refused");
    assert_eq!(regression["superseding"]["codes"], json!([]));
    assert_eq!(unsupported.verdict, Verdict::Unsupported);
}

/// FR-344-AC-3: three pairs created in an order other than their names'
/// order give a report ordered by case name, and two runs give equal bytes.
#[test]
#[trace("TC-866", "FR-344-AC-3")]
fn cases_are_ordered_by_name_whatever_the_creation_order() {
    let corpus = tempfile::tempdir().unwrap();
    for (directory, identity) in [
        ("3", "ix://spec/a"),
        ("2", "ix://spec/m"),
        ("1", "ix://spec/z"),
    ] {
        let mut pair = identity_pair();
        pair.identity = identity;
        pair.cases = vec![clause_case("healthy-parent")];
        pair.write(&corpus.path().join(directory));
    }
    let first = versioning::run(corpus.path(), &repository());
    let identities: Vec<String> = listed(&first)
        .iter()
        .map(|entry| {
            entry["name"]["prior"]["identity"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(identities, ["ix://spec/a", "ix://spec/m", "ix://spec/z"]);
    assert_eq!(
        first.to_json().unwrap(),
        versioning::run(corpus.path(), &repository())
            .to_json()
            .unwrap()
    );
}

/// FR-344-AC-7: a violation run exits 10 and a success run exits 0, the codes
/// a make target fails and passes on.
#[test]
#[trace("TC-866", "FR-344-AC-7")]
fn a_violation_exits_10_and_a_success_exits_0() {
    let violation = corpus(&[("seed", seed_pair())]);
    assert_eq!(exit_code(violation.path()), 10);
    let success = corpus(&[("identity", identity_pair())]);
    assert_eq!(exit_code(success.path()), 0);
}
