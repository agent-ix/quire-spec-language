// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-345 (TC-868) over the real layering corpus, generated at run time by
//! `xtask::refinement::layering_corpus::generate`: every unit is compiled by
//! the running build through the FR-278 composition. Compile outcomes are
//! substituted only through `compare`'s compile seam and witness
//! dispositions only through a test negotiation function, both defined here.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use qsl_foundation::bound::DomainKind;
use qsl_foundation::digest::ByteDigest;
use qsl_foundation::BackendId;
use qsl_replay::spine::RefinementClass;
use qsl_route::routing::Disposition;
use qsl_route::{BackendDescriptor, Candidate, ManifestDigest, Mode, ProviderOrigin};
use qsl_semantics::check::{Capability, CheckedStateClause};
use qsl_semantics::value::{HeaderLayer, WitnessKind, WitnessRequest};
use xtask::refinement::compare::CaseResult;
use xtask::refinement::layering::{
    compare, compile_unit, Compilation, EntryResult, RegistrySnapshot, RegistrySnapshotBuilder,
    UnitInput,
};
use xtask::refinement::layering_corpus::generate;
use xtask::refinement::report::Verdict;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the xtask manifest has a workspace root")
        .to_path_buf()
}

/// The real corpus as `make refinement-layering` generates it.
fn real_corpus() -> tempfile::TempDir {
    let out = tempfile::tempdir().expect("a temporary directory");
    generate(&repository(), out.path()).expect("the corpus generates");
    out
}

/// A backend named `name` advertising `capabilities` in `bounded` mode.
fn backend(name: &str, capabilities: &[Capability]) -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(BackendId::new(name)),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest(ByteDigest::of(name.as_bytes()).as_bytes()),
        capabilities
            .iter()
            .map(|capability| (*capability, Mode::Bounded)),
        Some(BTreeSet::from([DomainKind::Collection])),
        None,
    )
    .expect("a well-formed test backend")
}

fn registry(backends: &[BackendDescriptor]) -> RegistrySnapshot {
    let mut builder = RegistrySnapshotBuilder::new();
    for descriptor in backends {
        builder
            .register(descriptor.clone())
            .expect("the test backend registers");
    }
    builder.build()
}

const BOTH: &[Capability] = &[Capability::OperationContract, Capability::ValueValidity];

/// The unit's identity label's entry directory and side, such as
/// `e1-refused/child`.
fn unit_name(input: &UnitInput) -> String {
    let identity = &input.identity.identity;
    let mut parts = identity.rsplit('/');
    let side = parts.next().unwrap_or_default();
    let entry = parts.next().unwrap_or_default();
    format!("{entry}/{side}")
}

/// The case results by the parent unit's entry directory.
fn cases(results: &[EntryResult]) -> Vec<(String, CaseResult)> {
    let mut cases: Vec<_> = results
        .iter()
        .filter_map(|result| match result {
            EntryResult::Case { name, result, .. } => {
                let identity = name.parent.identity();
                let entry = identity.rsplit('/').nth(1).unwrap_or_default().to_owned();
                Some((entry, *result))
            }
            _ => None,
        })
        .collect();
    cases.sort_by(|left, right| left.0.cmp(&right.0));
    cases
}

fn edges(results: &[EntryResult]) -> Vec<(String, CaseResult)> {
    results
        .iter()
        .filter_map(|result| match result {
            EntryResult::Edge { edge, result, .. } => Some((edge.clone(), *result)),
            _ => None,
        })
        .collect()
}

fn layer<'r>(results: &'r [EntryResult], identity: &str) -> &'r EntryResult {
    results
        .iter()
        .find(|result| matches!(result, EntryResult::Layer { layer, .. } if layer == identity))
        .unwrap_or_else(|| panic!("a layer result for {identity}"))
}

fn tool_failures(results: &[EntryResult]) -> Vec<String> {
    results
        .iter()
        .filter_map(|result| match result {
            EntryResult::ToolFailure { entry, .. } => Some(entry.clone()),
            _ => None,
        })
        .collect()
}

/// Every case of the real corpus as the running build compiles it: each
/// edge's both-refuse case holds and its distinguishing-form case is not
/// applicable.
fn expected_cases() -> Vec<(String, CaseResult)> {
    (1..=6)
        .flat_map(|edge| {
            [
                (format!("e{edge}-form"), CaseResult::NotApplicable),
                (format!("e{edge}-refused"), CaseResult::Holds),
            ]
        })
        .collect()
}

const ALL_EDGES: [&str; 6] = ["E1", "E2", "E3", "E4", "E5", "E6"];

/// The six layers, each of whose witness units the running build admits:
/// the temporal witness's reset, guard and time invariant are written by S4
/// (FR-230), and its request is `operation-contract`/`bounded`
/// (FR-345-AC-6).
const LAYERS: [HeaderLayer; 6] = HeaderLayer::ALL;

/// FR-345-AC-4 and AC-7 over the real corpus, with no negotiation function:
/// on each of E1 to E6 the both-refuse case holds reporting both sides'
/// typed refusals, the distinguishing-form case is not applicable, and the
/// edge holds with the parent prohibiting the form by
/// `unsupported_construct` and the child admitting it. Each layer's
/// witness unit compiles `admitted`.
#[trace("TC-868", "FR-345-AC-4", "FR-345-AC-7")]
#[test]
fn the_generated_corpus_compares_every_edge() {
    let corpus = real_corpus();
    let report = compare(
        corpus.path(),
        compile_unit,
        &registry(&[backend("backend-a", BOTH)]),
        None,
    );
    assert_eq!(cases(&report.results), expected_cases());
    for result in &report.results {
        if let EntryResult::Case {
            name,
            parent,
            child,
            result: CaseResult::Holds,
        } = result
        {
            for side in [parent, child] {
                assert_eq!(side.class, "refused", "{name:?}");
                assert!(!side.codes.is_empty(), "{name:?} reports its codes");
            }
        }
    }
    assert_eq!(
        edges(&report.results),
        ALL_EDGES.map(|edge| (edge.to_owned(), CaseResult::Holds))
    );
    for result in &report.results {
        if let EntryResult::Edge { comparisons, .. } = result {
            let [comparison] = &comparisons[..] else {
                panic!("one distinguishing entry per edge");
            };
            assert_eq!(comparison.parent.class, "prohibited");
            assert!(comparison
                .parent
                .codes
                .iter()
                .all(|code| code.code == "unsupported_construct"));
            assert_eq!(comparison.child.class, "admitted");
        }
    }
    for each in LAYERS {
        let EntryResult::Layer {
            witnesses, result, ..
        } = layer(&report.results, each.identity())
        else {
            unreachable!("layer() finds layer results");
        };
        let [witness] = &witnesses[..] else {
            panic!("one witness for {}", each.identity());
        };
        assert_eq!(witness.class, "admitted", "{}", each.identity());
        assert_eq!(witness.reason.as_deref(), Some("not-negotiated"));
        assert_eq!(*result, CaseResult::UnresolvedUnsupported);
    }
    assert!(tool_failures(&report.results).is_empty());
}

/// FR-345-AC-4 (TC-868 step 3): with a test negotiation function settling
/// every request `supported`, a compile wrapper that admits the child of
/// each edge's both-refuse case gives exactly six regressions, one per edge,
/// each naming its case's two sources, its edge's layers and the parent's
/// codes; the verdict is violation, exit 10.
#[trace("TC-868", "FR-345-AC-4")]
#[test]
fn a_seeded_regression_on_each_edge_fails_naming_its_case() {
    let corpus = real_corpus();
    let mut supporting_all = |_: WitnessRequest, candidate: &Candidate, _: &CheckedStateClause| {
        Disposition::Supported(candidate.clone())
    };
    let wrapper = |input: &UnitInput| {
        let name = unit_name(input);
        if name.ends_with("-refused/child") {
            Compilation::classified(RefinementClass::Admitted, Vec::new())
        } else {
            compile_unit(input)
        }
    };
    let report = compare(
        corpus.path(),
        wrapper,
        &registry(&[backend("backend-a", BOTH)]),
        Some(&mut supporting_all),
    );
    let regressions: Vec<_> = report
        .results
        .iter()
        .filter_map(|result| match result {
            EntryResult::Case {
                name,
                parent,
                result: CaseResult::Regression,
                ..
            } => Some((name, parent)),
            _ => None,
        })
        .collect();
    assert_eq!(regressions.len(), 6);
    let mut layers = BTreeSet::new();
    for (name, parent) in regressions {
        assert!(name.parent.identity().ends_with("-refused/parent"));
        assert!(name.child.identity().ends_with("-refused/child"));
        assert_eq!(parent.class, "refused");
        assert!(!parent.codes.is_empty());
        layers.insert((name.parent_layer.clone(), name.child_layer.clone()));
    }
    assert_eq!(layers.len(), 6, "one regression per edge");
    assert_eq!(report.verdict, Verdict::Violation);
    assert_eq!(report.exit, 10);
}

/// FR-345-AC-7: with every request settled `supported`, a wrapper admitting
/// E1's distinguishing unit on the parent side gives a regression naming E1; verdict violation, exit 10.
#[trace("TC-868", "FR-345-AC-7")]
#[test]
fn a_parent_admitting_its_distinguishing_unit_is_a_regression_naming_the_edge() {
    let corpus = real_corpus();
    let mut supporting_all = |_: WitnessRequest, candidate: &Candidate, _: &CheckedStateClause| {
        Disposition::Supported(candidate.clone())
    };
    let wrapper = |input: &UnitInput| {
        if unit_name(input) == "e1-distinguishing/parent" {
            Compilation::classified(RefinementClass::Admitted, Vec::new())
        } else {
            compile_unit(input)
        }
    };
    let report = compare(
        corpus.path(),
        wrapper,
        &registry(&[backend("backend-a", BOTH)]),
        Some(&mut supporting_all),
    );
    let mut expected = ALL_EDGES.map(|edge| (edge.to_owned(), CaseResult::Holds));
    expected[0].1 = CaseResult::Regression;
    assert_eq!(edges(&report.results), expected);
    assert_eq!(report.exit, 10);
}

/// One request the test negotiation function received.
type Request = (WitnessKind, String);

/// FR-345-AC-6 for the six layers: with a test negotiation
/// function, each layer's witness item is requested once per candidate by
/// backend name with the layer's `witness_request` kind; with one candidate
/// supporting and one not, the layer holds; with both refusing, the layer is
/// a regression naming both backends' dispositions; with a registry whose
/// one backend advertises only `value-validity`, every `operation-contract`
/// layer is a regression over the empty candidate set while complete value
/// holds.
#[trace("TC-868", "FR-345-AC-6")]
#[test]
fn negotiated_witnesses_settle_per_candidate() {
    let corpus = real_corpus();
    let run = |backends: &[BackendDescriptor], supporting: &[&str]| {
        let mut requests: Vec<Request> = Vec::new();
        let mut negotiate =
            |request: WitnessRequest, candidate: &Candidate, _: &CheckedStateClause| {
                requests.push((request.kind, candidate.id().as_str().to_owned()));
                if supporting.contains(&candidate.id().as_str()) {
                    Disposition::Supported(candidate.clone())
                } else {
                    Disposition::Unsupported
                }
            };
        let report = compare(
            corpus.path(),
            compile_unit,
            &registry(backends),
            Some(&mut negotiate),
        );
        (report, requests)
    };
    let result_of = |results: &[EntryResult], each: HeaderLayer| {
        let EntryResult::Layer {
            witnesses, result, ..
        } = layer(results, each.identity())
        else {
            unreachable!("layer() finds layer results");
        };
        let candidates: Vec<(String, &str)> = witnesses[0]
            .candidates
            .iter()
            .map(|candidate| (candidate.backend.clone(), candidate.disposition))
            .collect();
        (*result, candidates)
    };

    let two = [backend("backend-a", BOTH), backend("backend-b", BOTH)];
    let (report, requests) = run(&two, &["backend-a"]);
    for each in LAYERS {
        assert_eq!(
            result_of(&report.results, each),
            (
                CaseResult::Holds,
                vec![
                    ("backend-a".to_owned(), "supported"),
                    ("backend-b".to_owned(), "unsupported")
                ]
            ),
            "{}",
            each.identity()
        );
    }
    let mut expected: Vec<Request> = LAYERS
        .iter()
        .flat_map(|each| {
            let kind = if *each == HeaderLayer::ValueComplete {
                WitnessKind::ValueValidity
            } else {
                WitnessKind::OperationContract
            };
            ["backend-a", "backend-b"].map(|name| (kind, name.to_owned()))
        })
        .collect();
    expected.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
    let mut requests = requests;
    requests.sort_by(|left, right| format!("{left:?}").cmp(&format!("{right:?}")));
    assert_eq!(requests, expected, "once per candidate per layer");

    let (report, _) = run(&two, &[]);
    for each in LAYERS {
        assert_eq!(
            result_of(&report.results, each),
            (
                CaseResult::Regression,
                vec![
                    ("backend-a".to_owned(), "unsupported"),
                    ("backend-b".to_owned(), "unsupported")
                ]
            )
        );
    }
    assert_eq!(report.exit, 10);

    let value_only = [backend("backend-v", &[Capability::ValueValidity])];
    let (report, _) = run(&value_only, &["backend-v"]);
    for each in LAYERS {
        let EntryResult::Layer {
            witnesses, result, ..
        } = layer(&report.results, each.identity())
        else {
            unreachable!("layer() finds layer results");
        };
        if each == HeaderLayer::ValueComplete {
            assert_eq!(*result, CaseResult::Holds);
        } else {
            assert_eq!(*result, CaseResult::Regression, "{}", each.identity());
            assert_eq!(witnesses[0].reason.as_deref(), Some("empty candidate set"));
        }
    }
}

/// Remove the entry directories `names` from `corpus`.
fn without(corpus: &Path, names: &[&str]) {
    for name in names {
        fs::remove_dir_all(corpus.join(name)).expect("the entry exists");
    }
}

/// FR-345-AC-11, its E6 entries: E6's both-refuse case holds; its
/// distinguishing unit, a state model writing `time M dense unit ms;`, is
/// prohibited by `quire.model.complete/v1` with
/// `unsupported_construct`/`declaration-form` and admitted under
/// `quire.temporal.complete/v1`; and a corpus without E6's
/// `distinguishing` entry and the temporal `witness` entry gives exactly one
/// tool-failure result for each, verdict tool failure, exit 30.
#[trace("TC-868", "FR-345-AC-11")]
#[test]
fn the_temporal_edge_holds_and_its_missing_entries_are_named() {
    let corpus = real_corpus();
    let report = compare(
        corpus.path(),
        compile_unit,
        &registry(&[backend("backend-a", BOTH)]),
        None,
    );
    let cases = cases(&report.results);
    assert!(cases.contains(&("e6-refused".to_owned(), CaseResult::Holds)));
    let e6 = report
        .results
        .iter()
        .find_map(|result| match result {
            EntryResult::Edge {
                edge,
                parent_layer,
                comparisons,
                result,
                ..
            } if edge == "E6" => Some((parent_layer, comparisons, result)),
            _ => None,
        })
        .expect("an E6 edge result");
    assert_eq!(e6.0, "quire.model.complete/v1");
    assert_eq!(*e6.2, CaseResult::Holds);
    let parent = &e6.1[0].parent;
    assert_eq!(parent.class, "prohibited");
    assert!(
        parent
            .codes
            .iter()
            .all(|code| code.code == "unsupported_construct"
                && code.cause == Some("declaration-form"))
    );
    assert_eq!(e6.1[0].child.class, "admitted");

    without(
        corpus.path(),
        &["e6-distinguishing", "witness-temporal-complete"],
    );
    let report = compare(
        corpus.path(),
        compile_unit,
        &registry(&[backend("backend-a", BOTH)]),
        None,
    );
    assert_eq!(
        tool_failures(&report.results),
        ["E6/distinguishing", "quire.temporal.complete/v1/witness"]
    );
    assert_eq!(report.verdict, Verdict::ToolFailure);
    assert_eq!(report.exit, 30);
}

/// The `model M = ...;` statement of every unit in `corpus` that selects
/// the config-version model, and how many entries the corpus holds.
fn config_model_statements(corpus: &Path) -> (BTreeSet<String>, usize) {
    let mut statements = BTreeSet::new();
    let mut entries = 0;
    for entry in fs::read_dir(corpus).expect("the corpus directory") {
        let entry = entry.expect("an entry").path();
        entries += 1;
        for file in fs::read_dir(&entry).expect("the entry directory") {
            let file = file.expect("a file").path();
            if file.extension().and_then(|extension| extension.to_str()) != Some("native") {
                continue;
            }
            let text = fs::read_to_string(&file).expect("a unit");
            statements.extend(
                text.lines()
                    .filter(|line| line.starts_with("model M = \"example/config-version\""))
                    .map(str::to_owned),
            );
        }
    }
    (statements, entries)
}

/// FR-345-AC-14: from a root holding only the config-version model file the
/// generator writes the whole corpus (24 entries), every unit over that
/// model naming it by its identity alone. A root without the model file
/// generates nothing.
#[trace("TC-868", "FR-345-AC-14")]
#[test]
fn the_corpus_is_generated_from_the_model_file_alone() {
    let model = "examples/config-version/model.semantic-ir.json";
    let root = tempfile::tempdir().expect("a temporary directory");
    fs::create_dir_all(root.path().join("examples/config-version")).unwrap();
    let original = fs::read(repository().join(model)).unwrap();
    fs::write(root.path().join(model), &original).unwrap();
    let generated = |label: &str| {
        let out = tempfile::tempdir().unwrap();
        generate(root.path(), out.path()).unwrap_or_else(|error| panic!("{label}: {error}"));
        config_model_statements(out.path())
    };

    assert_eq!(
        generated("the original model"),
        (
            BTreeSet::from(["model M = \"example/config-version\";".to_owned()]),
            24
        )
    );

    assert!(generate(
        tempfile::tempdir().unwrap().path(),
        tempfile::tempdir().unwrap().path()
    )
    .is_err());
}
