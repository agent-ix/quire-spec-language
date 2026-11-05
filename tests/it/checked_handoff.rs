// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-517 and FR-518 (TC-1312, TC-1313): the checked-package handoff writer
//! and its adverse mutation corpus, over the ported IT-001 recipe.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quire_contract_model::{
    read_checked_package, CheckedNodeKind, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedPackageRefusal, CheckedPackageRefusalCause,
    CheckedPackageRefusalCode, CheckedPackageV2, ClockConfiguration, ClockExactNumber, JsonPointer,
    ProtocolForm, TemporalForm,
};
use quire_spec_language::checked_handoff::{
    evidence, write, write_with, CheckedHandoffRequest, Corpus, Environment, Error, ErrorCode,
    RealEnvironment, Selection, SelectionAxis, SelectionRefusal,
};
use quire_spec_language::emit_checked_v2::{self, ClockSelection, Request};
use serde_json::Value;

const DIR: &str = "examples/checked-handoff";

/// The three packages of recipe R, one per clock kind: the unit file and the
/// clock configuration of each of its temporal clauses.
fn recipe(kind: &str) -> (&'static str, Vec<(&'static str, ClockConfiguration)>) {
    match kind {
        "event_position" => (
            "unit-event-position.native",
            vec![
                (
                    "Full",
                    ClockConfiguration::EventPosition {
                        sequence_authority: "events-authority".into(),
                    },
                ),
                (
                    "Partial",
                    ClockConfiguration::EventPosition {
                        sequence_authority: "events-authority".into(),
                    },
                ),
                (
                    "Due",
                    ClockConfiguration::EventPosition {
                        sequence_authority: "events-authority".into(),
                    },
                ),
                (
                    "DueLate",
                    ClockConfiguration::EventPosition {
                        sequence_authority: "events-authority".into(),
                    },
                ),
            ],
        ),
        "fixed_sample" => (
            "unit-fixed-sample.native",
            vec![(
                "DueSample",
                ClockConfiguration::FixedSample {
                    epoch: ClockExactNumber::Integer {
                        decimal: "0".into(),
                    },
                    period: ClockExactNumber::Rational {
                        numerator: "1".into(),
                        denominator: "10".into(),
                    },
                    unit: "milliseconds".into(),
                },
            )],
        ),
        _ => (
            "unit-timestamped-event.native",
            vec![(
                "DueTimestamp",
                ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "rfc3339".into(),
                },
            )],
        ),
    }
}

pub(super) fn request(kind: &str, corpus: Corpus) -> CheckedHandoffRequest {
    let (unit, clocks) = recipe(kind);
    CheckedHandoffRequest {
        emit: Request {
            source: PathBuf::from(format!("{DIR}/{unit}")),
            package_name: "r".into(),
            models: vec![PathBuf::from(format!("{DIR}/model.semantic-ir.json"))],
            libraries: vec![(
                "agent-ix/checked-handoff-library".into(),
                PathBuf::from(format!("{DIR}/library.native")),
            )],
            execution_points: None,
            temporal_selections: clocks
                .into_iter()
                .map(|(clause, clock)| ClockSelection {
                    clause: clause.into(),
                    clock,
                })
                .collect(),
            out: None,
        },
        corpus,
    }
}

pub(super) const KINDS: [&str; 3] = ["event_position", "fixed_sample", "timestamped_event"];

/// Every regular file below `root`, keyed by its root-relative path.
pub(super) fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).expect("read directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let name = path
                    .strip_prefix(root)
                    .expect("below root")
                    .display()
                    .to_string();
                files.insert(name, std::fs::read(&path).expect("read file"));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

/// The refusal a reader double gives a package that is not the base.
fn other_refusal() -> CheckedPackageRefusal {
    CheckedPackageRefusal {
        code: CheckedPackageRefusalCode::InvalidPackage,
        loci: vec![],
        cause: None,
        locus: None,
        contract_version: None,
        document_pointer: None,
        extension_detail: None,
        protocol_detail: None,
        terminal_duplicate: None,
    }
}

/// The real stages with one injected difference.
#[derive(Default)]
struct Double {
    /// Give every read after the first the refusal `other_refusal`.
    wrong_case_refusal: bool,
    /// Give every read the refusal `other_refusal`.
    refuse_reads: bool,
    /// Read these bytes instead of the first package read: an admitted
    /// package that is not the one the unit compiled to.
    substitute_first_read: Option<Vec<u8>>,
    /// Fail the write of the file with this index.
    fail_write_at: Option<usize>,
    reads: Cell<usize>,
}

impl Environment for Double {
    fn read(
        &self,
        bytes: &[u8],
        limits: CheckedPackageReadLimits,
        evidence: &CheckedPackageEvidence,
    ) -> CheckedPackageDispatchResult {
        let first = self.reads.get() == 0;
        self.reads.set(self.reads.get() + 1);
        if self.refuse_reads || (self.wrong_case_refusal && !first) {
            return CheckedPackageDispatchResult::Refused(other_refusal());
        }
        if let (true, Some(substitute)) = (first, &self.substitute_first_read) {
            return RealEnvironment.read(substitute, limits, evidence);
        }
        RealEnvironment.read(bytes, limits, evidence)
    }

    fn check(
        &self,
        selection: &Selection,
        package: &CheckedPackageV2,
    ) -> Result<(), SelectionRefusal> {
        selection.check(package)
    }

    fn write_file(&self, index: usize, path: &Path, contents: &[u8]) -> std::io::Result<()> {
        if self.fail_write_at == Some(index) {
            return Err(std::io::Error::other("injected"));
        }
        std::fs::write(path, contents)
    }
}

/// The handoff of `kind` written with `double` into a fresh directory.
fn written(kind: &str, corpus: Corpus, double: &Double) -> (tempfile::TempDir, PathBuf) {
    let holder = tempfile::tempdir().expect("temporary directory");
    let out = holder.path().join("handoff");
    write_with(&request(kind, corpus), &out, double).expect("the handoff is written");
    (holder, out)
}

pub(super) fn admitted(bytes: &[u8], evidence: &CheckedPackageEvidence) -> Box<CheckedPackageV2> {
    match read_checked_package(bytes, CheckedPackageReadLimits::bounded(), evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => package,
        other => panic!("the reader refused the package: {other:?}"),
    }
}

const SOURCE_LABEL: &str = "sources/source:examples/checked-handoff/unit-event-position.native";

/// FR-517-AC-1, the paths the writer writes today: everything in the Outputs
/// table except `inventory.json`, and no `mutations/` for a `None` corpus. The
/// package bytes are those `emit_checked_v2::emit` returns.
#[trace("FR-517-AC-1", "FR-042-AC-15", "FR-048-AC-10", "TC-1312", "TC-135")]
#[test]
fn the_directory_holds_the_outputs_table_except_the_inventory() {
    let (_holder, out) = written("event_position", Corpus::None, &Double::default());
    let files = tree(&out);
    let paths: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "checked-package.json",
            "equality.json",
            "identities.json",
            "models/0.json",
            "selection.json",
            "sources/library:examples/checked-handoff/library.native",
            SOURCE_LABEL,
        ]
    );
    let request = request("event_position", Corpus::None);
    assert_eq!(
        files["checked-package.json"],
        emit_checked_v2::emit(&request.emit).unwrap()
    );
    assert_eq!(
        files[SOURCE_LABEL],
        std::fs::read(&request.emit.source).unwrap()
    );
    assert_eq!(
        files["models/0.json"],
        std::fs::read(&request.emit.models[0]).unwrap()
    );
}

/// FR-517-AC-11: the paths of the Outputs table with `inventory.json`
/// included (`qsl_replay::qsl_inventory` over the unit with the selected
/// formal identity, FR-514).
#[trace("FR-517-AC-11", "TC-1312")]
#[test]
#[ignore = "open: QSL-1689 (the writer): `write` does not yet run the replay spine over the unit nor write inventory.json in the format quire.checked-handoff.inventory/1 (FR-517-AC-11; reader AC-17)"]
fn each_directory_holds_the_inventory_with_the_outputs_table() {
    let (_holder, out) = written("event_position", Corpus::None, &Double::default());
    let paths: Vec<String> = tree(&out).into_keys().collect();
    assert_eq!(
        paths,
        [
            "checked-package.json",
            "inventory.json",
            "models/0.json",
            "selection.json",
            "sources/library:examples/checked-handoff/library.native",
            SOURCE_LABEL,
        ]
    );
}

/// FR-517-AC-2: the package read with evidence from the written files is
/// admitted, `Selection::check` passes, and the sidecar rows name the
/// clauses and clock rows of the package.
#[trace("FR-517-AC-2", "FR-048-AC-10", "TC-1312", "TC-135")]
#[test]
fn the_written_package_is_admitted_and_matches_its_sidecar() {
    for kind in KINDS {
        let (_holder, out) = written(kind, Corpus::None, &Double::default());
        let files = tree(&out);
        let mut held = evidence(&request(kind, Corpus::None)).expect("evidence");
        // An equal document is accepted; a different one would conflict.
        held.insert_domain_package_document(
            "agent-ix/checked-handoff",
            files["models/0.json"].as_slice(),
        )
        .expect("models/0.json is the document the evidence holds");
        let package = admitted(&files["checked-package.json"], &held);
        let selection = Selection::read(&files["selection.json"]).expect("sidecar parses");
        // FR-048-AC-10: the lock keeps every selection the request makes, as
        // exactly this role list (the protocol profile only where the unit
        // has a protocol clause), the selected library, and the domain
        // package whose document the evidence holds.
        let temporal = match kind {
            "event_position" => "quire.temporal.event-position.false-extension/v1",
            "fixed_sample" => "quire.temporal.fixed-sample.false-extension/v1",
            _ => "quire.temporal.timestamped-event.finite-window/v1",
        };
        let mut expected_roles = Vec::new();
        if kind == "event_position" {
            expected_roles.push(("protocol_profile", "quire.protocol.finite-global/v1"));
        }
        expected_roles.push(("temporal_profile", temporal));
        let lock = serde_json::to_value(package.lock()).unwrap();
        let roles: Vec<(&str, &str)> = lock["profile_selections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|selected| {
                (
                    selected["role"].as_str().unwrap(),
                    selected["definition"]["identity"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(roles, expected_roles, "{kind}: the lock's role selections");
        assert_eq!(
            lock["dependency_selections"],
            serde_json::json!([{"identity": "agent-ix/checked-handoff-library"}]),
            "{kind}"
        );
        assert_eq!(selection.check(&package), Ok(()));
        let sidecar: Value = serde_json::from_slice(&files["selection.json"]).unwrap();
        let rows = sidecar["clauses"].as_array().unwrap();
        let bindings = package.temporal_bindings().unwrap();
        // A binding row for each clause and each compensation template.
        let compensations = sidecar["compensations"].as_array().unwrap();
        assert_eq!(rows.len() + compensations.len(), bindings.len());
        let names: Vec<&str> = rows
            .iter()
            .map(|row| row["name"].as_str().unwrap())
            .collect();
        let authored: &[&str] = match kind {
            "event_position" => &["Due", "DueLate"],
            "fixed_sample" => &["DueSample"],
            _ => &["DueTimestamp"],
        };
        let mut sorted = names.clone();
        sorted.sort_unstable();
        let mut want = authored.to_vec();
        want.sort_unstable();
        assert_eq!(sorted, want, "the sidecar names the authored clauses");
        let protocols: Vec<&CheckedNodeKind> = package
            .node_kinds()
            .iter()
            .filter(|k| **k == CheckedNodeKind::Protocol(ProtocolForm::ProtocolClause))
            .collect();
        for row in rows {
            assert_eq!(row["activation"]["form"], "origin");
            assert_eq!(row["profile"]["authority"], "agent-ix");
            let lock = serde_json::to_value(&package.lock().profile_selections).unwrap();
            let selected = lock
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["role"] == "temporal_profile")
                .unwrap();
            let identity = selected["definition"]["identity"].as_str().unwrap();
            assert_eq!(
                format!(
                    "{}/{}",
                    row["profile"]["identity"].as_str().unwrap(),
                    row["profile"]["revision"].as_str().unwrap()
                ),
                identity
            );
            assert_eq!(
                selected["definition"]["authority"],
                row["profile"]["authority"]
            );
            let requirements = row["requirements"].as_array().unwrap();
            // Only the clause `Due` of the event-position unit is required.
            assert_eq!(requirements.len(), usize::from(row["name"] == "Due"));
            assert_eq!(protocols.len(), usize::from(kind == "event_position"));
        }
        for row in rows {
            let binding = bindings
                .iter()
                .find(|binding| serde_json::to_value(&binding.declaration).unwrap() == row["node"])
                .expect("each sidecar clause has its binding row");
            assert_eq!(
                serde_json::to_value(&binding.clock).unwrap(),
                row["binding"]["clock"]
            );
            assert_eq!(
                binding.clock_name.as_ref(),
                row["binding"]["clock_name"].as_str().unwrap()
            );
        }
    }
}

/// FR-517-AC-3: one `protocol_clause`, two compensations with different keys
/// pairing with one effect, and a binding row per clause in ascending order
/// with the kinds of the three clock kinds across the three packages.
#[trace("FR-517-AC-3", "FR-042-AC-10", "TC-1312")]
#[test]
fn the_recipe_emits_the_protocol_the_compensations_and_the_three_clock_kinds() {
    let mut kinds = Vec::new();
    for kind in KINDS {
        let (_holder, out) = written(kind, Corpus::None, &Double::default());
        let files = tree(&out);
        let held = evidence(&request(kind, Corpus::None)).unwrap();
        let package = admitted(&files["checked-package.json"], &held);
        let count = |wanted: CheckedNodeKind| {
            package
                .node_kinds()
                .iter()
                .filter(|k| **k == wanted)
                .count()
        };
        let clauses = count(CheckedNodeKind::Temporal(TemporalForm::TemporalClause));
        assert_eq!(clauses, if kind == "event_position" { 2 } else { 1 });
        let compensations = if kind == "event_position" { 2 } else { 0 };
        assert_eq!(
            package.temporal_bindings().unwrap().len(),
            clauses + compensations
        );
        let ids: Vec<_> = package
            .temporal_bindings()
            .unwrap()
            .iter()
            .map(|b| b.declaration.clone())
            .collect();
        assert!(
            ids.windows(2).all(|pair| pair[0] < pair[1]),
            "ascending NodeRef order"
        );
        for binding in package.temporal_bindings().unwrap() {
            kinds.push(
                serde_json::to_value(&binding.clock).unwrap()["kind"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        if kind == "event_position" {
            assert_eq!(
                count(CheckedNodeKind::Protocol(ProtocolForm::ProtocolClause)),
                1
            );
            assert_eq!(
                count(CheckedNodeKind::Protocol(ProtocolForm::Compensation)),
                2
            );
            let compensations: Vec<_> = package
                .node_kinds()
                .iter()
                .zip(&package.graph().nodes)
                .filter(|(k, _)| **k == CheckedNodeKind::Protocol(ProtocolForm::Compensation))
                .map(|(_, node)| node)
                .collect();
            let keys: Vec<String> = compensations
                .iter()
                .map(|node| serde_json::to_string(&node.node_id).unwrap())
                .collect();
            assert_ne!(keys[0], keys[1]);
            assert!(
                keys.iter().any(|key| key.contains("\"Full\""))
                    && keys.iter().any(|key| key.contains("\"Partial\"")),
                "{keys:?}"
            );
            let shared: Vec<_> = compensations[0]
                .dependencies
                .iter()
                .filter(|dependency| compensations[1].dependencies.contains(dependency))
                .collect();
            assert!(!shared.is_empty(), "both pair with the one effect node");
            let effects: Vec<String> =
                shared
                    .iter()
                    .filter(|id| {
                        package.graph().nodes.iter().any(|node| {
                            &node.node_id == **id && node.node_tag.as_ref() == "protocol"
                        })
                    })
                    .map(|id| serde_json::to_string(id).unwrap())
                    .filter(|id| id.contains("\"run\""))
                    .collect();
            assert_eq!(effects.len(), 1, "one shared run node: {effects:?}");
            let effect = &effects[0];
            assert!(
                effect.contains("\"Applied\""),
                "the shared node is the Main::Applied effect: {effect}"
            );
        }
    }
    assert_eq!(
        kinds,
        [
            "event_position",
            "event_position",
            "event_position",
            "event_position",
            "fixed_sample",
            "timestamped_event"
        ]
    );
}

/// FR-517-AC-4: the signed-64 extrema are values of the package, with the
/// declared bounds not widened.
#[trace("FR-517-AC-4", "FR-042-AC-10", "TC-1312")]
#[test]
fn the_signed_64_extrema_are_package_values_within_their_declared_bounds() {
    let (_holder, out) = written("event_position", Corpus::None, &Double::default());
    let text = String::from_utf8(tree(&out)["checked-package.json"].clone()).unwrap();
    for extremum in ["-9223372036854775808", "9223372036854775807"] {
        assert!(
            text.contains(&format!(
                "\"value\":\"{extremum}\",\"value_kind\":\"integer\""
            )),
            "{extremum} is an integer literal value of the package"
        );
    }
    let model: Value =
        serde_json::from_slice(&std::fs::read(format!("{DIR}/model.semantic-ir.json")).unwrap())
            .unwrap();
    let wide = model["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["displayName"].as_str().unwrap().ends_with("/Wide"))
        .unwrap();
    let bounds: Vec<&str> = wide["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["operands"]["value"].as_str().unwrap())
        .collect();
    assert_eq!(bounds, ["-9223372036854775808", "9223372036854775807"]);
}

/// FR-517-AC-12: `Exact` carries the extreme rationals
/// `9223372036854775807/9223372036854775806` and
/// `-9223372036854775808/9223372036854775807`. The spine admits no bounded
/// Rational value type yet (FR-505, QSL-1522), so R has none.
#[trace("FR-517-AC-12", "TC-1312")]
#[test]
#[ignore = "open: QSL-1693 (R fixture): FR-505 admits a unitless Rational, but R's model and sources declare no Ratio or Exact at all"]
fn the_extreme_rationals_are_package_values() {
    let (_holder, out) = written("event_position", Corpus::None, &Double::default());
    let text = String::from_utf8(tree(&out)["checked-package.json"].clone()).unwrap();
    for rational in [
        "9223372036854775807/9223372036854775806",
        "-9223372036854775808/9223372036854775807",
    ] {
        assert!(text.contains(rational), "{rational}");
    }
}

/// FR-517-AC-5: two writes of equal requests give byte-identical trees.
#[trace("FR-517-AC-5", "FR-042-AC-15", "TC-1312")]
#[test]
fn equal_requests_write_identical_trees() {
    for kind in KINDS {
        let (_a, first) = written(kind, Corpus::None, &Double::default());
        let (_b, second) = written(kind, Corpus::None, &Double::default());
        assert_eq!(tree(&first), tree(&second));
    }
}

/// The command line of `request`'s emission, with its clock selections in
/// `clocks` (a file this writes) passed as `--clock-selections`.
fn command_line(request: &Request, clocks: &Path) -> Vec<std::ffi::OsString> {
    let mut line: Vec<std::ffi::OsString> =
        vec!["--package-name".into(), request.package_name.clone().into()];
    for model in &request.models {
        line.extend(["--model".into(), model.clone().into_os_string()]);
    }
    for (identity, file) in &request.libraries {
        line.extend([
            "--library".into(),
            format!("{identity}={}", file.display()).into(),
        ]);
    }
    line.extend(["--clock-selections".into(), clocks.as_os_str().to_owned()]);
    line.push(request.source.clone().into_os_string());
    line
}

/// FR-517-AC-15: `--clock-selections` carries each package's clock
/// configurations, so the command line emits the bytes `emit` returns for the
/// equal `Request`; a malformed file is a refusal naming it and writes no
/// bytes.
#[trace("FR-517-AC-15", "TC-1312")]
#[test]
fn the_clock_selections_flag_emits_the_requests_bytes_and_bad_files_refuse() {
    let directory = tempfile::tempdir().unwrap();
    for kind in KINDS {
        let expected = request(kind, Corpus::None).emit;
        // Written by hand, in selection order: a `serde_json::Map` would sort.
        let entries: Vec<String> = expected
            .temporal_selections
            .iter()
            .map(|selection| {
                format!(
                    "{}:{}",
                    serde_json::to_string(&selection.clause).unwrap(),
                    serde_json::to_string(&selection.clock).unwrap()
                )
            })
            .collect();
        let clocks = directory.path().join(format!("{kind}.json"));
        std::fs::write(&clocks, format!("{{{}}}", entries.join(","))).unwrap();
        let line = command_line(&expected, &clocks);
        assert_eq!(Request::parse(&line).unwrap(), expected, "{kind}");
        let mut stdout = Vec::new();
        emit_checked_v2::run(&line, &mut stdout).expect(kind);
        assert_eq!(stdout, emit_checked_v2::emit(&expected).unwrap(), "{kind}");
    }

    // File order is kept: keys written out of sorted order are parsed in it.
    let ordered = request("event_position", Corpus::None).emit;
    let clock = r#"{"kind": "event_position", "sequence_authority": "events-authority"}"#;
    let clocks = directory.path().join("out-of-order.json");
    std::fs::write(
        &clocks,
        format!(r#"{{"Partial": {clock}, "Due": {clock}, "Full": {clock}, "DueLate": {clock}}}"#),
    )
    .unwrap();
    let parsed = Request::parse(&command_line(&ordered, &clocks)).unwrap();
    let names: Vec<&str> = parsed
        .temporal_selections
        .iter()
        .map(|selection| selection.clause.as_str())
        .collect();
    assert_eq!(names, ["Partial", "Due", "Full", "DueLate"]);

    let base = request("timestamped_event", Corpus::None).emit;
    for (name, text) in [
        (
            "missing",
            r#"{"DueTimestamp": {"kind": "timestamped_event"}}"#,
        ),
        ("empty", "{}"),
        (
            "repeated",
            r#"{"DueTimestamp": {"kind": "timestamped_event", "timestamp_unit": "rfc3339"}, "DueTimestamp": {"kind": "timestamped_event", "timestamp_unit": "rfc3339"}}"#,
        ),
        ("kind", r#"{"DueTimestamp": {"kind": "sundial"}}"#),
        (
            "member",
            r#"{"DueTimestamp": {"kind": "timestamped_event", "timestamp_unit": "rfc3339", "extra": 1}}"#,
        ),
    ] {
        let clocks = directory.path().join(format!("bad-{name}.json"));
        std::fs::write(&clocks, text).unwrap();
        let mut stdout = Vec::new();
        let refusal =
            emit_checked_v2::run(&command_line(&base, &clocks), &mut stdout).expect_err(name);
        assert!(
            matches!(&refusal, emit_checked_v2::Error::Clocks { path, .. } if path.ends_with(&format!("bad-{name}.json"))),
            "{name}: {refusal:?}"
        );
        assert_eq!(refusal.exit_code(), 20, "{name}");
        assert!(stdout.is_empty(), "{name}");
    }

    // A well-formed file naming no clause or template is refused before any
    // bytes are emitted.
    let clocks = directory.path().join("unknown-clause.json");
    std::fs::write(
        &clocks,
        r#"{"NoSuchClause": {"kind": "timestamped_event", "timestamp_unit": "rfc3339"}}"#,
    )
    .unwrap();
    let mut stdout = Vec::new();
    let refusal = emit_checked_v2::run(&command_line(&base, &clocks), &mut stdout).unwrap_err();
    assert!(
        matches!(&refusal, emit_checked_v2::Error::UnknownClause { clause } if clause == "NoSuchClause"),
        "{refusal:?}"
    );
    assert!(stdout.is_empty());
}

/// The names beside `holder`'s handoff directory.
fn entries(holder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(holder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// FR-517-AC-6: an existing directory is refused and unchanged; a failure
/// after the temporary tree is built leaves nothing at or beside the path.
#[trace("FR-517-AC-6", "FR-042-AC-15", "TC-1312")]
#[test]
fn an_existing_directory_is_untouched_and_a_failure_leaves_nothing() {
    let (holder, out) = written("event_position", Corpus::None, &Double::default());
    let before = tree(&out);
    let error = write(&request("event_position", Corpus::None), &out).unwrap_err();
    assert_eq!(error.code(), ErrorCode::Exists);
    assert_eq!(tree(&out), before);
    assert_eq!(entries(holder.path()), ["handoff"]);

    let fresh = tempfile::tempdir().unwrap();
    let target = fresh.path().join("handoff");
    let double = Double {
        fail_write_at: Some(2),
        ..Double::default()
    };
    let error = write_with(&request("event_position", Corpus::None), &target, &double).unwrap_err();
    assert_eq!(error.code(), ErrorCode::Io);
    assert_eq!(entries(fresh.path()), Vec::<String>::new());
}

/// FR-517-AC-7: a refused source, a refused package and an injected
/// read-back difference each write nothing.
#[trace("FR-517-AC-7", "FR-042-AC-15", "TC-1312")]
#[test]
fn a_refusal_or_difference_writes_nothing() {
    let fresh = tempfile::tempdir().unwrap();
    let target = fresh.path().join("handoff");
    let mut bad = request("event_position", Corpus::None);
    let unit = fresh.path().join("bad.native");
    std::fs::write(&unit, "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\nmodel M = \"agent-ix/checked-handoff\";\nfunction f using v(x: Boolean): Boolean pure { 1 }\n").unwrap();
    bad.emit.source = unit;
    bad.emit.temporal_selections.clear();
    let error = write(&bad, &target).unwrap_err();
    assert_eq!(error.code(), ErrorCode::Compile);
    assert_eq!(entries(fresh.path()), ["bad.native"]);

    let refuses = Double {
        refuse_reads: true,
        ..Double::default()
    };
    let error =
        write_with(&request("event_position", Corpus::None), &target, &refuses).unwrap_err();
    assert_eq!(error.code(), ErrorCode::Reader);
    // The read-back comparison fails for real: the reader admits a package
    // whose sequence authority is not the one the unit compiled to.
    let (_holder, corpus) = written("event_position", Corpus::Standard, &Double::default());
    let other = tree(&corpus)["mutations/clock-sequence-authority.json"].clone();
    let differs = Double {
        substitute_first_read: Some(other),
        ..Double::default()
    };
    let error =
        write_with(&request("event_position", Corpus::None), &target, &differs).unwrap_err();
    assert!(
        matches!(&error, Error::ReadBackDiffers { detail } if detail.contains("clock.sequence_authority")),
        "{error:?}"
    );
    assert_eq!(entries(fresh.path()), ["bad.native"]);
}

/// FR-517-AC-8: the module is behind `checked-handoff`, which names no
/// dependency, and the module's sources use no dev-dependency.
#[trace("FR-517-AC-8", "FR-042-AC-15", "TC-1312")]
#[test]
fn the_feature_adds_no_dependency() {
    let manifest: toml_free::Manifest = toml_free::read("Cargo.toml");
    assert_eq!(
        manifest.feature_line("checked-handoff"),
        Some("[]".to_owned())
    );
    let lock = std::fs::read_to_string("Cargo.lock").unwrap();
    assert!(!lock.contains("checked-handoff"));
    for file in [
        "src/checked_handoff.rs",
        "src/checked_handoff/corpus.rs",
        "src/checked_handoff/selection.rs",
        "src/checked_handoff/tree.rs",
    ] {
        let text = std::fs::read_to_string(file).unwrap();
        for dev in [
            "tempfile",
            "jsonschema",
            "serde_json_canonicalizer",
            "syn::",
            "xtask",
        ] {
            assert!(!text.contains(dev), "{file} names dev-dependency {dev}");
        }
    }
    let lib = std::fs::read_to_string("src/lib.rs").unwrap();
    assert!(lib.contains("#[cfg(feature = \"checked-handoff\")]\npub mod checked_handoff;"));
}

/// A minimal view of `Cargo.toml`'s `[features]` lines.
mod toml_free {
    pub struct Manifest(String);

    pub fn read(path: &str) -> Manifest {
        Manifest(std::fs::read_to_string(path).unwrap())
    }

    impl Manifest {
        pub fn feature_line(&self, name: &str) -> Option<String> {
            let features = self.0.split("[features]").nth(1)?.split("\n[").next()?;
            features
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{name} = ")))
                .map(|rest| rest.trim().to_owned())
        }
    }
}

/// FR-517-AC-9: a package with one clock value changed is admitted by the
/// reader and refused by `Selection::check` naming its axis; the module has
/// no reader of its own.
#[trace("FR-517-AC-9", "TC-1312")]
#[test]
fn a_changed_clock_value_is_admitted_by_the_reader_and_refused_by_the_sidecar() {
    let (_holder, out) = written("event_position", Corpus::None, &Double::default());
    let files = tree(&out);
    let mut package: Value = serde_json::from_slice(&files["checked-package.json"]).unwrap();
    package["temporal_bindings"][0]["clock"]["sequence_authority"] = "another-authority".into();
    let held = evidence(&request("event_position", Corpus::None)).unwrap();
    let changed = admitted(&serde_json::to_vec(&package).unwrap(), &held);
    let selection = Selection::read(&files["selection.json"]).unwrap();
    let refusal = selection.check(&changed).unwrap_err();
    assert_eq!(refusal.axis, SelectionAxis::ClockSequenceAuthority);
    for file in [
        "src/checked_handoff.rs",
        "src/checked_handoff/corpus.rs",
        "src/checked_handoff/selection.rs",
        "src/checked_handoff/tree.rs",
    ] {
        let text = std::fs::read_to_string(file).unwrap();
        assert!(!text.contains("CheckedPackageV2::read"), "{file}");
        assert!(!text.contains("read_checked_package_v2"), "{file}");
        // The two JSON parses of package bytes: the corpus mutates them, and
        // the wire-id join reads the node ids; neither admits a package.
        let parses = text.matches("serde_json::from_slice").count()
            + text.matches("of_package_bytes").count();
        let allowed =
            usize::from(file.ends_with("corpus.rs")) + usize::from(file.ends_with("selection.rs"));
        assert_eq!(
            parses,
            allowed + usize::from(file.ends_with("selection.rs")),
            "{file}"
        );
    }
}

/// FR-517-AC-10: the legacy writer and its items are present.
#[trace("FR-517-AC-10", "TC-1312")]
#[test]
fn the_legacy_writer_is_still_present() {
    let handoff = std::fs::read_to_string("src/protocol_artifact/handoff.rs").unwrap();
    for constant in [
        "PUBLISHED_V1_OFFER_FILE",
        "PUBLISHED_V1_SELECTION_FILE",
        "PUBLISHED_OFFER_FILE",
        "PUBLISHED_SELECTION_FILE",
        "PUBLISHED_MUTATION_MANIFEST_FILE",
    ] {
        assert!(handoff.contains(constant), "{constant}");
    }
    let writer = std::fs::read_to_string("src/protocol_artifact/handoff/writer.rs").unwrap();
    assert!(writer.contains("pub fn write_v1") && writer.contains("pub fn write_v2"));
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap();
    assert!(manifest.contains("handoff-writer = []"));
    for example in [
        "native_protocol_handoff.rs",
        "native_protocol_v2_handoff.rs",
    ] {
        assert!(Path::new("examples").join(example).exists());
    }
    assert!(Path::new("examples/protocol-handoff/model.json").exists());
}

/// One row of FR-518's table as amended by the writer self-check: identity,
/// package kind, stage, and the declared refusal (a reader code, cause and
/// pointer, where `*` stands for a node index, or a selection axis).
struct Row {
    identity: &'static str,
    kind: &'static str,
    stage: &'static str,
    code: &'static str,
    cause: Option<&'static str>,
    pointer: &'static str,
}

const fn reader(
    identity: &'static str,
    kind: &'static str,
    code: &'static str,
    cause: Option<&'static str>,
    pointer: &'static str,
) -> Row {
    Row {
        identity,
        kind,
        stage: "reader",
        code,
        cause,
        pointer,
    }
}

const fn selection(identity: &'static str, kind: &'static str, axis: &'static str) -> Row {
    Row {
        identity,
        kind,
        stage: "selection",
        code: axis,
        cause: None,
        pointer: "",
    }
}

const TIMED: Option<&str> = Some("timed-node-reference");
const EP: &str = "event_position";
const FS: &str = "fixed_sample";
const TS: &str = "timestamped_event";
const LAW: &str = "/semantic_graph/nodes/*/body/operation/laws/0/definition";

/// FR-518's table, written by hand in table order (30 cases).
fn table() -> Vec<Row> {
    vec![
        reader(
            "header-wire-version-substitution",
            EP,
            "unknown_contract_version",
            None,
            "/contract_version",
        ),
        reader(
            "binding-missing",
            EP,
            "invalid_package",
            TIMED,
            "/semantic_graph/nodes/*/body",
        ),
        reader(
            "binding-surplus",
            EP,
            "invalid_package",
            TIMED,
            "/temporal_bindings/4/declaration",
        ),
        reader(
            "binding-duplicate",
            EP,
            "invalid_package",
            Some("conflicting-definition"),
            "/temporal_bindings/4",
        ),
        reader(
            "binding-reordered",
            EP,
            "invalid_package",
            TIMED,
            "/temporal_bindings/1/declaration",
        ),
        reader(
            "declaration-index",
            EP,
            "invalid_package",
            TIMED,
            "/temporal_bindings/0/declaration",
        ),
        reader(
            "definition-index",
            EP,
            "missing_declaration",
            Some("missing-selection"),
            "/semantic_graph/nodes/7",
        ),
        reader(
            "declaration-name",
            EP,
            "invalid_semantic_graph",
            None,
            "/semantic_graph/nodes/*/declaration",
        ),
        selection("declaration-requirement", EP, "clause.requirement"),
        selection("declaration-clause", EP, "clause.node"),
        selection("declaration-execution", EP, "clause.activation"),
        selection("definition-identity", EP, "definition.identity"),
        reader(
            "definition-revision",
            EP,
            "unknown_profile",
            Some("unsupported-selection"),
            LAW,
        ),
        reader(
            "definition-artifact-digest-closed-shape",
            EP,
            "unknown_member",
            None,
            "/lock/definition_selections/0/digest",
        ),
        reader(
            "definition-artifact-digest-domain-closed-shape",
            EP,
            "unknown_member",
            None,
            "/lock/definition_selections/0/digest_domain",
        ),
        reader(
            "clock-alternative",
            EP,
            "invalid_package",
            TIMED,
            "/temporal_bindings/0/clock/kind",
        ),
        selection("clock-sequence-authority", EP, "clock.sequence_authority"),
        selection("clock-epoch", FS, "clock.epoch"),
        selection("clock-period", FS, "clock.period"),
        selection("clock-unit", FS, "clock.unit"),
        selection("clock-timestamp-unit", TS, "clock.timestamp_unit"),
        reader(
            "clock-missing-sequence-authority",
            EP,
            "malformed_wire",
            None,
            "/temporal_bindings/0/clock",
        ),
        reader(
            "clock-renamed-sequence-authority",
            EP,
            "malformed_wire",
            None,
            "/temporal_bindings/0/clock",
        ),
        reader(
            "clock-missing-period",
            FS,
            "malformed_wire",
            None,
            "/temporal_bindings/0/clock",
        ),
        reader(
            "clock-renamed-period",
            FS,
            "malformed_wire",
            None,
            "/temporal_bindings/0/clock",
        ),
        reader(
            "clock-missing-timestamp-unit",
            TS,
            "malformed_wire",
            None,
            "/temporal_bindings/0/clock",
        ),
        reader(
            "clock-renamed-timestamp-unit",
            TS,
            "malformed_wire",
            None,
            "/temporal_bindings/0/clock",
        ),
        selection(
            "definition-artifact-wire-identity",
            EP,
            "definition.authority",
        ),
        reader(
            "clock-name",
            EP,
            "invalid_package",
            TIMED,
            "/temporal_bindings/0/clock_name",
        ),
        reader(
            "clock-name-empty",
            EP,
            "malformed_wire",
            None,
            "/temporal_bindings/0/clock_name",
        ),
    ]
}

fn pointer_matches(declared: &str, actual: &str) -> bool {
    match declared.split_once('*') {
        Some((head, tail)) => {
            actual.starts_with(head)
                && actual.ends_with(tail)
                && actual[head.len()..actual.len() - tail.len()]
                    .parse::<u64>()
                    .is_ok()
        }
        None => declared == actual,
    }
}

fn manifest_of(out: &Path) -> Value {
    serde_json::from_slice(&tree(out)["mutations/manifest.json"]).expect("manifest is JSON")
}

/// FR-518-AC-1 over the readers of today: the manifest has the format, the
/// table order, unique identities, stages and non-empty files, and the three
/// packages together hold the 30 cases including 14b and 15b.
#[trace("FR-518-AC-1", "TC-1313")]
#[test]
fn the_manifest_lists_every_case_in_table_order() {
    let table = table();
    assert_eq!(table.len(), 30);
    let mut listed = Vec::new();
    for kind in KINDS {
        let (_holder, out) = written(kind, Corpus::Standard, &Double::default());
        let manifest = manifest_of(&out);
        assert_eq!(manifest["format"], "quire.checked-handoff.mutations/1");
        assert_eq!(manifest["base_package"], "checked-package.json");
        assert_eq!(manifest["selection"], "selection.json");
        let files = tree(&out);
        let order: Vec<&str> = table
            .iter()
            .filter(|row| row.kind == kind)
            .map(|row| row.identity)
            .collect();
        let cases: Vec<&Value> = manifest["cases"].as_array().unwrap().iter().collect();
        let identities: Vec<&str> = cases
            .iter()
            .map(|case| case["identity"].as_str().unwrap())
            .collect();
        assert_eq!(identities, order);
        for case in cases {
            assert!(!files[case["input"]["file"].as_str().unwrap()].is_empty());
            let row = table
                .iter()
                .find(|row| row.identity == case["identity"])
                .unwrap();
            assert_eq!(case["stage"], row.stage);
        }
        listed.extend(identities.into_iter().map(str::to_owned));
    }
    let mut unique = listed.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 30);
    for identity in [
        "definition-artifact-digest-closed-shape",
        "definition-artifact-digest-domain-closed-shape",
    ] {
        assert!(listed.iter().any(|listed| listed == identity));
    }
}

/// FR-518-AC-2 and AC-3: each `reader` case is refused by the reader with
/// its row's code, cause and pointer; each `selection` case is admitted by
/// the reader and refused with its axis; the unmutated package passes both.
#[trace("FR-518-AC-2", "FR-518-AC-3", "TC-1313")]
#[test]
fn every_case_gets_the_refusal_of_its_row() {
    let table = table();
    let mut reader_cases = 0;
    let mut selection_cases = 0;
    for kind in KINDS {
        let (_holder, out) = written(kind, Corpus::Standard, &Double::default());
        let files = tree(&out);
        let held = evidence(&request(kind, Corpus::Standard)).unwrap();
        let selection = Selection::read(&files["selection.json"]).unwrap();
        assert_eq!(
            selection.check(&admitted(&files["checked-package.json"], &held)),
            Ok(())
        );
        for case in manifest_of(&out)["cases"].as_array().unwrap() {
            let identity = case["identity"].as_str().unwrap();
            let row = table.iter().find(|row| row.identity == identity).unwrap();
            let bytes = &files[case["input"]["file"].as_str().unwrap()];
            let result = read_checked_package(bytes, CheckedPackageReadLimits::bounded(), &held);
            match (row.stage, result) {
                ("reader", CheckedPackageDispatchResult::Refused(refusal)) => {
                    reader_cases += 1;
                    let code = serde_json::to_value(refusal.code).unwrap();
                    assert_eq!(code, row.code, "{identity}");
                    let cause = refusal
                        .cause
                        .map(|cause| serde_json::to_value(cause).unwrap());
                    assert_eq!(cause, row.cause.map(Value::from), "{identity}");
                    let pointer = refusal.loci[0].path.as_ref().unwrap().to_string();
                    assert!(
                        pointer_matches(row.pointer, &pointer),
                        "{identity}: {pointer}"
                    );
                }
                ("selection", CheckedPackageDispatchResult::AdmittedV2(package)) => {
                    selection_cases += 1;
                    let refusal = selection.check(&package).unwrap_err();
                    assert_eq!(refusal.axis.name(), row.code, "{identity}");
                }
                (stage, other) => panic!("{identity}: stage {stage} got {other:?}"),
            }
        }
    }
    // 30 cases: 20 stage `reader` and 10 stage `selection`.
    assert_eq!((reader_cases, selection_cases), (20, 10));
}

/// TC-1308 row 23: Contract IR admits the emitted protocol package directly,
/// then refuses substitutions to its source, selected model, result node
/// identity, and canonical wire bytes.
#[trace("TC-1308", "TC-135", "FR-513-AC-13", "FR-048-AC-9")]
#[test]
fn protocol_package_is_independently_admitted_and_identity_mutations_refuse() {
    let (_holder, out) = written("event_position", Corpus::None, &Double::default());
    let files = tree(&out);
    let bytes = &files["checked-package.json"];
    let held = evidence(&request("event_position", Corpus::None)).unwrap();
    admitted(bytes, &held);

    let refusal = |package: Value| {
        let bytes = serde_json::to_vec(&package).expect("mutated package is JSON");
        match read_checked_package(&bytes, CheckedPackageReadLimits::bounded(), &held) {
            CheckedPackageDispatchResult::Refused(refusal) => refusal,
            other => panic!("mutated package was admitted: {other:?}"),
        }
    };

    let round_trip = serde_json::to_vec(&serde_json::from_slice::<Value>(bytes).unwrap())
        .expect("unmutated package round-trips as canonical JSON");
    admitted(&round_trip, &held);

    let mut source = serde_json::from_slice::<Value>(bytes).unwrap();
    source["lock"]["sources"][0]["identity"] = serde_json::json!("source:foreign");
    let source_refusal = refusal(source);
    assert_eq!(
        source_refusal.code,
        CheckedPackageRefusalCode::InvalidSourceMap
    );
    assert_eq!(source_refusal.cause, None, "{source_refusal:?}");
    assert_eq!(
        source_refusal.loci[0]
            .path
            .as_ref()
            .map(|path| path.as_str()),
        Some("/source_map/0/regions/0/source")
    );

    let mut model = serde_json::from_slice::<Value>(bytes).unwrap();
    let nodes = model["semantic_graph"]["nodes"].as_array_mut().unwrap();
    let root_index = nodes
        .iter()
        .position(|node| node["semantic_form"] == "model_root")
        .expect("the protocol package carries its selected model root");
    let root = nodes
        .get_mut(root_index)
        .expect("the selected model root index is in range");
    root["owner"]["identity"] = serde_json::json!("foreign/model");
    let model_refusal = refusal(model);
    assert_eq!(model_refusal.code, CheckedPackageRefusalCode::MissingImport);
    assert_eq!(
        model_refusal.cause,
        Some(CheckedPackageRefusalCause::MissingSelection),
        "{model_refusal:?}"
    );
    assert_eq!(
        model_refusal.loci[0]
            .path
            .as_ref()
            .map(|path| path.as_str()),
        Some(format!("/semantic_graph/nodes/{root_index}/owner/identity").as_str())
    );

    let mut result = serde_json::from_slice::<Value>(bytes).unwrap();
    let node = result["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .first_mut()
        .expect("the protocol package has a result node");
    let ordinal = node["node_id"]["ordinal"]
        .as_u64()
        .expect("the result identity ordinal is an unsigned integer");
    node["node_id"]["ordinal"] = serde_json::json!(if ordinal == 0 { 1 } else { 0 });
    let result_refusal = refusal(result);
    assert_eq!(
        result_refusal.code,
        CheckedPackageRefusalCode::InvalidPackage,
        "{result_refusal:?}"
    );
    assert_eq!(
        result_refusal.cause,
        Some(CheckedPackageRefusalCause::InvalidDerivedShape),
        "{result_refusal:?}"
    );
    assert_eq!(
        result_refusal.loci[0]
            .path
            .as_ref()
            .map(|path| path.as_str()),
        Some("/semantic_graph/nodes/0/node_id")
    );

    let noncanonical = serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(bytes).unwrap())
        .expect("the same package can be reserialized with whitespace");
    let noncanonical_refusal =
        match read_checked_package(&noncanonical, CheckedPackageReadLimits::bounded(), &held) {
            CheckedPackageDispatchResult::Refused(refusal) => refusal,
            other => panic!("noncanonical package bytes were admitted: {other:?}"),
        };
    assert_eq!(
        noncanonical_refusal.code,
        CheckedPackageRefusalCode::NoncanonicalWire
    );
    assert_eq!(noncanonical_refusal.cause, None, "{noncanonical_refusal:?}");
    assert!(noncanonical_refusal.loci[0].path.is_none());
}

/// FR-518-AC-4: the case of row 12 (the clauses, the lock and the binding rows
/// move to another registered profile) and the case of row 29 are admitted by
/// the reader and refused with `definition.identity` and `definition.authority`.
/// Row 13 is a reader refusal: Contract IR registers one revision per profile.
#[trace("FR-518-AC-4", "TC-1313")]
#[test]
fn the_definition_cases_are_refused_on_identity_and_authority() {
    let (_holder, out) = written("event_position", Corpus::Standard, &Double::default());
    let files = tree(&out);
    let held = evidence(&request("event_position", Corpus::Standard)).unwrap();
    let selection = Selection::read(&files["selection.json"]).unwrap();
    for (case, axis) in [
        ("definition-identity", SelectionAxis::DefinitionIdentity),
        (
            "definition-artifact-wire-identity",
            SelectionAxis::DefinitionAuthority,
        ),
    ] {
        let package = admitted(&files[&format!("mutations/{case}.json")], &held);
        assert_eq!(selection.check(&package).unwrap_err().axis, axis, "{case}");
    }
    let revision = read_checked_package(
        &files["mutations/definition-revision.json"],
        CheckedPackageReadLimits::bounded(),
        &held,
    );
    let CheckedPackageDispatchResult::Refused(refusal) = revision else {
        panic!("admitted")
    };
    assert_eq!(
        serde_json::to_value(refusal.code).unwrap(),
        "unknown_profile"
    );
}

/// FR-518-AC-5: a stage that gives another refusal makes `write` return
/// `CaseMismatch` and write nothing.
#[trace("FR-518-AC-5", "TC-1313")]
#[test]
fn a_wrong_refusal_is_a_mismatch() {
    let fresh = tempfile::tempdir().unwrap();
    let target = fresh.path().join("handoff");
    let wrong = Double {
        wrong_case_refusal: true,
        ..Double::default()
    };
    let error = write_with(
        &request("event_position", Corpus::Standard),
        &target,
        &wrong,
    )
    .unwrap_err();
    let Error::CaseMismatch { identity } = error else {
        panic!("{error:?}")
    };
    assert_eq!(identity, "header-wire-version-substitution");
    assert_eq!(entries(fresh.path()), Vec::<String>::new());
}

/// FR-518-AC-6: two writes give byte-identical manifests and case files.
#[trace("FR-518-AC-6", "TC-1313")]
#[test]
fn the_corpus_is_deterministic() {
    for kind in KINDS {
        let (_a, first) = written(kind, Corpus::Standard, &Double::default());
        let (_b, second) = written(kind, Corpus::Standard, &Double::default());
        assert_eq!(tree(&first), tree(&second));
    }
}

/// FR-518-AC-7: no `expected` names a digest, a seal or a `v2.*` string, and a
/// `None` corpus has no `mutations/`.
#[trace("FR-518-AC-7", "TC-1313")]
#[test]
fn expected_values_name_no_digest_seal_or_legacy_string() {
    for kind in KINDS {
        let (_holder, out) = written(kind, Corpus::Standard, &Double::default());
        for case in manifest_of(&out)["cases"].as_array().unwrap() {
            // The pointer of the closed-shape cases names the member they add.
            let expected = format!(
                "{} {} {}",
                case["expected"]["code"], case["expected"]["cause"], case["expected"]["axis"]
            );
            for forbidden in ["digest", "seal", "v2."] {
                assert!(!expected.contains(forbidden), "{expected}");
            }
        }
        let (_holder, none) = written(kind, Corpus::None, &Double::default());
        assert!(!none.join("mutations").exists());
    }
}

/// FR-518-AC-8: each case file is named by exactly one case and each case
/// names an existing file; the module exports no case list constant.
#[trace("FR-518-AC-8", "TC-1313")]
#[test]
fn files_and_cases_correspond_one_to_one() {
    for kind in KINDS {
        let (_holder, out) = written(kind, Corpus::Standard, &Double::default());
        let files = tree(&out);
        let manifest = manifest_of(&out);
        let named: Vec<&str> = manifest["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["input"]["file"].as_str().unwrap())
            .collect();
        let mut present: Vec<&str> = files
            .keys()
            .map(String::as_str)
            .filter(|p| p.starts_with("mutations/") && *p != "mutations/manifest.json")
            .collect();
        present.sort_unstable();
        let mut sorted = named.clone();
        sorted.sort_unstable();
        assert_eq!(present, sorted);
        sorted.dedup();
        assert_eq!(sorted.len(), named.len());
    }
    let module = std::fs::read_to_string("src/checked_handoff.rs").unwrap();
    let constants: Vec<&str> = module
        .lines()
        .filter(|line| line.starts_with("pub const"))
        .collect();
    assert_eq!(constants.len(), 3, "{constants:?}");
    assert!(constants
        .iter()
        .all(|line| !line.contains("CASE") && !line.contains("IDENT")));
}

/// The package and sidecar of the event-position handoff, with the sidecar as
/// JSON, for tests that edit the sidecar.
fn sidecar_fixture() -> (Value, Box<CheckedPackageV2>) {
    let (_holder, out) = written("event_position", Corpus::None, &Double::default());
    let files = tree(&out);
    let held = evidence(&request("event_position", Corpus::None)).unwrap();
    (
        serde_json::from_slice(&files["selection.json"]).unwrap(),
        admitted(&files["checked-package.json"], &held),
    )
}

fn axis_of(sidecar: &Value, package: &CheckedPackageV2) -> Option<SelectionAxis> {
    let selection = Selection::read(&serde_json::to_vec(sidecar).unwrap())
        .expect("an edited sidecar still parses");
    selection.check(package).err().map(|refusal| refusal.axis)
}

/// Selection::check compares every sidecar value with the package; each edit
/// of one sidecar value, against the unchanged admitted package, is refused
/// on that value's own axis. Deleting any comparison turns one row red.
#[trace("FR-517-AC-2", "FR-517-AC-9", "FR-518-AC-12", "TC-1312", "TC-1313")]
#[test]
fn each_sidecar_value_is_compared_on_its_own_axis() {
    let (sidecar, package) = sidecar_fixture();
    assert_eq!(
        axis_of(&sidecar, &package),
        None,
        "the unedited sidecar passes"
    );
    let edit = |change: &dyn Fn(&mut Value)| {
        let mut edited = sidecar.clone();
        change(&mut edited);
        axis_of(&edited, &package)
    };
    use SelectionAxis as A;
    assert_eq!(
        edit(&|v| v["clauses"][0]["profile"]["identity"] = "other.profile".into()),
        Some(A::DefinitionIdentity)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["profile"]["revision"] = "v9".into()),
        Some(A::DefinitionRevision)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["profile"]["authority"] = "other".into()),
        Some(A::DefinitionAuthority)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["node"]["ordinal"] = 7.into()),
        Some(A::ClauseNode)
    );
    assert_eq!(
        edit(
            &|v| v["clauses"][1]["requirements"] = serde_json::json!([{"protocol": "Flow", "node": v["clauses"][0]["requirements"][0]["node"].clone()}])
        ),
        Some(A::ClauseRequirement)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["requirements"] = serde_json::json!([])),
        Some(A::ClauseRequirement)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["activation"]["form"] = "each".into()),
        Some(A::ClauseActivation)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["activation"]["guarded"] = true.into()),
        Some(A::ClauseActivation)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["binding"]["clock_name"] = "other".into()),
        Some(A::ClockName)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["binding"]["clock"] =
            serde_json::json!({"kind": "timestamped_event", "timestamp_unit": "u"})),
        Some(A::ClockKind)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["binding"]["clock"]["sequence_authority"] = "other".into()),
        Some(A::ClockSequenceAuthority)
    );
    assert_eq!(
        edit(&|v| v["clauses"][0]["binding"] = Value::Null),
        Some(A::ClauseBinding)
    );
    assert_eq!(
        edit(&|v| v["package"]["node_counts"]["protocol/compensation"] = 9.into()),
        Some(A::PackageNodes)
    );
    assert_eq!(
        edit(&|v| {
            v["clauses"].as_array_mut().unwrap().pop();
        }),
        Some(A::ClauseBinding)
    );
    // A `compensations` value of the sidecar is compared on its own axis too.
    assert_eq!(
        edit(&|v| v["compensations"][0]["binding"]["clock_name"] = "other".into()),
        Some(A::ClockName)
    );
    assert_eq!(
        edit(&|v| v["compensations"][1]["binding"]["clock"]["sequence_authority"] = "other".into()),
        Some(A::ClockSequenceAuthority)
    );
    assert_eq!(
        edit(&|v| v["compensations"][0]["node"]["ordinal"] = 7.into()),
        Some(A::ClauseBinding)
    );
    assert_eq!(
        edit(&|v| v["compensations"][0]["binding"] = Value::Null),
        Some(A::ClauseBinding)
    );
    assert_eq!(
        edit(&|v| v["clauses"][1]["node"]["ordinal"] = 7.into()),
        Some(A::ClauseNode)
    );
}

/// The fixed-sample and timestamped-event sidecars compare their own clock
/// members on their own axes.
#[trace("FR-517-AC-9", "TC-1312")]
#[test]
fn the_other_clock_kinds_are_compared_member_by_member() {
    for (kind, member, axis) in [
        ("fixed_sample", "epoch", SelectionAxis::ClockEpoch),
        ("fixed_sample", "period", SelectionAxis::ClockPeriod),
        ("fixed_sample", "unit", SelectionAxis::ClockUnit),
        (
            "timestamped_event",
            "timestamp_unit",
            SelectionAxis::ClockTimestampUnit,
        ),
    ] {
        let (_holder, out) = written(kind, Corpus::None, &Double::default());
        let files = tree(&out);
        let held = evidence(&request(kind, Corpus::None)).unwrap();
        let package = admitted(&files["checked-package.json"], &held);
        let mut sidecar: Value = serde_json::from_slice(&files["selection.json"]).unwrap();
        sidecar["clauses"][0]["binding"]["clock"][member] =
            serde_json::json!({"kind": "integer", "decimal": "99"});
        if member == "unit" || member == "timestamp_unit" {
            sidecar["clauses"][0]["binding"]["clock"][member] = "other-unit".into();
        }
        assert_eq!(axis_of(&sidecar, &package), Some(axis), "{kind} {member}");
    }
}

/// The package carries no declared clause name (a `temporal_clause` node has no
/// `declaration`), so the sidecar's name has nothing to compare with: an edited
/// name passes. `clause.name` fires only for a node that carries a declaration,
/// and the reader refuses that member on this node.
#[test]
fn the_clause_name_has_no_package_side_value() {
    let (mut sidecar, package) = sidecar_fixture();
    sidecar["clauses"][0]["name"] = "Renamed".into();
    assert_eq!(axis_of(&sidecar, &package), None);
    assert!(
        package
            .graph()
            .nodes
            .iter()
            .all(|node| node.declaration.is_none()
                || node.semantic_form.as_ref() != "temporal_clause")
    );
}

/// Selection::read refuses bytes that are not a sidecar of this format.
#[test]
fn a_sidecar_that_does_not_parse_is_refused() {
    use quire_spec_language::checked_handoff::SelectionReadError as E;
    assert!(matches!(Selection::read(b"not json"), Err(E::Malformed(_))));
    assert!(matches!(Selection::read(b"{}"), Err(E::Malformed(_))));
    let (mut sidecar, _package) = sidecar_fixture();
    sidecar["format"] = "quire.checked-handoff.selection/2".into();
    let error = Selection::read(&serde_json::to_vec(&sidecar).unwrap()).unwrap_err();
    assert!(matches!(&error, E::Format(format) if format == "quire.checked-handoff.selection/2"));
    sidecar["format"] = "quire.checked-handoff.selection/1".into();
    sidecar["extra"] = 1.into();
    assert!(matches!(
        Selection::read(&serde_json::to_vec(&sidecar).unwrap()),
        Err(E::Malformed(_))
    ));
}

/// The error variants a request can reach: a clock selection naming no
/// clause or naming one twice, a label that leaves the directory, and an
/// existing empty directory.
#[test]
fn bad_requests_get_their_own_error() {
    let fresh = tempfile::tempdir().unwrap();
    let target = fresh.path().join("handoff");
    let mut unknown = request("event_position", Corpus::None);
    unknown.emit.temporal_selections[0].clause = "Missing".into();
    let error = write(&unknown, &target).unwrap_err();
    assert!(
        matches!(&error, Error::Compile(emit_checked_v2::Error::UnknownClause { clause }) if clause == "Missing"),
        "{error:?}"
    );
    let mut twice = request("event_position", Corpus::None);
    twice.emit.temporal_selections[1].clause = "Due".into();
    let error = write(&twice, &target).unwrap_err();
    assert!(
        matches!(&error, Error::Compile(emit_checked_v2::Error::DuplicateClause { clause }) if clause == "Due"),
        "{error:?}"
    );

    let mut label = request("event_position", Corpus::None);
    let moved = fresh.path().join("unit.native");
    std::fs::copy(&label.emit.source, &moved).unwrap();
    label.emit.source = fresh.path().join("sub/../unit.native");
    std::fs::create_dir(fresh.path().join("sub")).unwrap();
    let error = write(&label, &target).unwrap_err();
    assert_eq!(error.code(), ErrorCode::Label, "{error:?}");
    assert!(!target.exists());

    std::fs::create_dir(&target).unwrap();
    let error = write(&request("event_position", Corpus::None), &target).unwrap_err();
    assert_eq!(error.code(), ErrorCode::Exists);
    assert_eq!(
        std::fs::read_dir(&target).unwrap().count(),
        0,
        "the empty directory is left as it was"
    );
}

/// Whether a stale temporary tree from a crashed write, or an empty
/// directory already at the path, blocks a write: the stale tree does not
/// (the writer picks another sibling name), the existing directory does.
#[test]
fn crash_residue_does_not_block_a_write_and_leaves_no_claim() {
    let fresh = tempfile::tempdir().unwrap();
    let target = fresh.path().join("handoff");
    let stale = format!(".handoff.writing-{}-0", std::process::id());
    std::fs::create_dir(fresh.path().join(&stale)).unwrap();
    std::fs::write(fresh.path().join(&stale).join("half"), b"x").unwrap();
    write(&request("event_position", Corpus::None), &target)
        .expect("a stale sibling does not block");
    assert!(target.join("checked-package.json").exists());
    assert_eq!(entries(fresh.path()), [stale.as_str(), "handoff"]);
    // A failed write leaves no empty claim at the path.
    let other = fresh.path().join("second");
    let double = Double {
        fail_write_at: Some(1),
        ..Double::default()
    };
    write_with(&request("event_position", Corpus::None), &other, &double).unwrap_err();
    assert!(!other.exists());
    write(&request("event_position", Corpus::None), &other).expect("a later write is not refused");
}

/// FR-518-AC-11: with a reader that admits a declared name on a temporal
/// clause, a package whose clause declares a name other than the sidecar's is
/// admitted by the reader and refused by `Selection::check` with
/// `clause.name`.
#[trace("FR-518-AC-11", "TC-1313")]
#[test]
#[ignore = "gated on IR-1278: the reader refuses a declaration member on a temporal clause node"]
fn a_differing_declared_clause_name_is_refused_on_clause_name() {
    let (_holder, out) = written("event_position", Corpus::None, &Double::default());
    let files = tree(&out);
    let mut package: Value = serde_json::from_slice(&files["checked-package.json"]).unwrap();
    let clause = package["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["semantic_form"] == "temporal_clause")
        .unwrap();
    clause["declaration"] = serde_json::json!({"qualified_name": ["Renamed"]});
    let held = evidence(&request("event_position", Corpus::None)).unwrap();
    let package = admitted(&serde_json::to_vec(&package).unwrap(), &held);
    let selection = Selection::read(&files["selection.json"]).unwrap();
    assert_eq!(
        selection.check(&package).unwrap_err().axis,
        SelectionAxis::ClauseName
    );
}

/// FR-517-AC-13: with the model admitting a unit on a bounded scalar, the
/// `Amount` and `Total` scalars of R carry the unit `U`: each is an alias of
/// `ix://quire/unit/U` whose `min` and `max` name the alias (QSpec FR-154
/// "Unit bounds"), the form [`unit_model`] writes.
#[trace("FR-517-AC-13", "TC-1312")]
#[test]
#[ignore = "open: QSL-1694 via QSL-1690/1691/1692 (spine items FR-517-AC-18..20): R's Amount and Total cannot yet be unit aliases"]
fn amount_and_total_carry_the_unit_u() {
    let model: Value =
        serde_json::from_slice(&std::fs::read(format!("{DIR}/model.semantic-ir.json")).unwrap())
            .unwrap();
    for scalar in ["Amount", "Total"] {
        let declared = declared_type(&model, scalar);
        assert_eq!(declared["kind"], "alias", "{scalar}");
        assert_eq!(declared["target"], "ix://quire/unit/U", "{scalar}");
    }
}

/// The type of R's model whose identity ends in `/name`.
fn declared_type<'m>(model: &'m Value, name: &str) -> &'m Value {
    model["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|declared| {
            declared["identity"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/{name}"))
        })
        .unwrap()
}

/// R's model with `Amount` and `Total` as aliases of the unit `U`, their
/// bounds kept and naming the alias, written into `dir`.
fn unit_model(dir: &Path) -> PathBuf {
    let mut model: Value =
        serde_json::from_slice(&std::fs::read(format!("{DIR}/model.semantic-ir.json")).unwrap())
            .unwrap();
    for name in ["Amount", "Total"] {
        let identity = declared_type(&model, name)["identity"].clone();
        let declared = model["types"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|declared| declared["identity"] == identity)
            .unwrap();
        let object = declared.as_object_mut().unwrap();
        object.remove("scalar");
        object.insert("kind".into(), "alias".into());
        object.insert("target".into(), "ix://quire/unit/U".into());
        for constraint in object["constraints"].as_array_mut().unwrap() {
            constraint["appliesTo"] = identity.clone();
        }
    }
    let path = dir.join("model.semantic-ir.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&model).unwrap()).unwrap();
    path
}

/// A handoff request for one unit over [`unit_model`] that declares the
/// unit `U` and then `body`; no library, no temporal clause.
fn unit_request(dir: &Path, body: &str) -> CheckedHandoffRequest {
    let source = dir.join("unit.native");
    std::fs::write(
        &source,
        format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.model.complete/v1\";\n\
             model M = \"agent-ix/checked-handoff\";\n\
             dimension Money;\nunit U : Money = rational(1, 1);\n{body}\n"
        ),
    )
    .unwrap();
    CheckedHandoffRequest {
        emit: Request {
            source,
            package_name: "r".into(),
            models: vec![unit_model(dir)],
            libraries: Vec::new(),
            execution_points: None,
            temporal_selections: Vec::new(),
            out: None,
        },
        corpus: Corpus::None,
    }
}

/// [`unit_request`] written into a new directory beside its inputs.
fn write_unit(body: &str) -> Result<(), Error> {
    let dir = tempfile::tempdir().unwrap();
    write(&unit_request(dir.path(), body), &dir.path().join("handoff"))
}

/// FR-517-AC-16 (QSL-1558): the refusals that keep the unit `U` off R's
/// `Amount` and `Total`. A unit-typed operation parameter compared with a
/// unit literal is written and admitted; a unit that no expression of the
/// unit reaches is not emitted, though the selected model's types name it,
/// so Contract IR's reader refuses the model `typeRef` that reaches it (R's
/// library and its fixed-sample and timestamped-event units read no
/// unit-typed member); the model alias is not a source type name; and `sum`
/// refuses a quantity summand.
#[trace("FR-517-AC-16", "TC-1312")]
#[test]
fn the_unit_on_amount_and_total_is_refused_where_the_spine_stops() {
    write_unit("pre BeforeApply using v on M::Workflow::apply { amount >= 1 U }")
        .expect("a unit-typed parameter against a unit literal is written");

    let error =
        write_unit("invariant Ready using v on M::Workflow at current { self.ready }").unwrap_err();
    let Error::ReaderRefused(refusal) = &error else {
        panic!("the reader refuses a unit no expression reaches: {error:?}");
    };
    assert_eq!(refusal.code, CheckedPackageRefusalCode::MissingDeclaration);
    assert_eq!(refusal.cause, Some(CheckedPackageRefusalCause::MissingName));
    // `Recovery.receipts`, typed `Amount`, the first `typeRef` to reach `U`.
    assert_eq!(
        refusal.document_pointer,
        JsonPointer::parse("/types/7/fields/0/typeRef")
    );

    let error = write_unit(
        "invariant Bounded using v on M::Workflow at current {\n    \
         sum<M::Total>(receipt in self.receipts: receipt) <= self.total\n}",
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::Compile);
    let detail = format!("{error:?}");
    assert!(
        detail.contains("UnresolvedTypeName { name: \"M::Total\" }"),
        "{detail}"
    );

    let error = write_unit(
        "type Cash = Quantity<U>;\n\
         invariant Bounded using v on M::Workflow at current {\n    \
         sum<Cash>(receipt in self.receipts: receipt) <= self.total\n}",
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::Compile);
    let detail = format!("{error:?}");
    assert!(
        detail.contains("StateClause { clause: \"Bounded\"")
            && detail.contains("IllTyped(TypeMismatch)"),
        "{detail}"
    );
}

/// FR-517-AC-14: the `recover` condition of `Full` compares the receipt sum
/// with the captured target; that of `Partial` requires a positive sum below
/// it.
#[trace("FR-517-AC-14", "TC-1312")]
#[test]
#[ignore = "gated on IR-1311 (the Contract IR reader): a `sum` binder bound inside `recover` is refused `BinderScope` (QSL emits it since QSL-1555)"]
fn the_recover_conditions_compare_the_receipt_sum_with_the_target() {
    let unit = std::fs::read_to_string(format!("{DIR}/unit-event-position.native")).unwrap();
    let recover = |name: &str| {
        let from = unit.find(&format!("recover (recovered{name}")).unwrap();
        unit[from..].split("};").next().unwrap().to_owned()
    };
    let full = recover("Full");
    assert!(
        full.contains("sum<M::Total>(") && full.contains("= targetFull"),
        "{full}"
    );
    let partial = recover("Partial");
    assert!(
        partial.contains("sum<M::Total>(")
            && partial.contains("> 0")
            && partial.contains("< targetPartial"),
        "{partial}"
    );
}
