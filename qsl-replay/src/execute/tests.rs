// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098 (TC-444): the replay executor over real complete-V1 source,
//! recompiled through the spine.

use qsl_foundation::diagnostic::Category;
use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::{Code, SourceIdentity};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::intake::{package_input, PackageDocument};
use quire_exact::{Identifier, ScalarLimits};

use super::*;
use crate::proof_result::{InconclusiveCause, TerminalValue};
use crate::request::StateEnvironment;
use crate::result::{InputSettlement, WitnessSettlement};
use crate::scalar::ScalarOutcome;
use crate::spine::{compose, ComposedUnit, SpineLimits, SpineStage};
use crate::witness::{CanonicalAssignment, Witness, WitnessValue};

mod composite;
mod composite_parity;

const PROFILE: &str = "profile v = \"quire.value.complete/v1\";\n";

/// The proved unit: `small` is the predicate a counterexample refutes.
///
/// `f`/`p` (the QSL-22 Layer 3 exemplar's shape): `p` is a Boolean
/// predicate whose body calls another declared function, `f`, rather than
/// applying an operator directly to its own parameters -- the one case
/// `small`, `flag`, `id`, `lt` and `maybe` never exercise.
fn proved() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function small using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n\
         function flag using v(b: Boolean): Boolean pure {{ b }}\n\
         function id using v(x: Int[0, 9]): Int[0, 9] pure {{ x }}\n\
         function lt using v(a: Int[0, 9], b: Int[0, 9]): Boolean pure {{ a < b }}\n\
         function maybe using v(t: Option<Boolean>): Boolean pure {{ true }}\n\
         function f using v(x: Int[0, 9]): Integer pure {{ x + 1 }}\n\
         function p using v(x: Int[0, 9]): Boolean pure {{ f(x) > 3 }}\n"
    )
}

const AUTHORITY: &str = "agent-ix";
const IDENTITY: &str = "test:replay";
const NAMESPACE: &str = "git";
const REVISION: &str = "r1";

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn spine(source: &str, packages: &BTreeMap<[u8; 32], Vec<u8>>) -> ComposedUnit {
    compose(
        SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
        IDENTITY,
        source.as_bytes(),
        packages,
        &crate::spine::DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the unit compiles")
}

/// The wire node id of `function`'s parameter `index` in `compiled`.
fn parameter(compiled: &ComposedUnit, function: &str, index: usize) -> WireNodeId {
    let graph = compiled.package.graph();
    let identity = graph.callable(function).expect("declared").identity;
    let key = graph
        .semantic_graph()
        .node(identity)
        .and_then(|node| node.function_parameters())
        .expect("a function node")[index];
    WireNodeId::from_digest(*key.as_bytes())
}

fn name(segments: &[&str]) -> QualifiedName {
    QualifiedName::new(
        segments
            .iter()
            .map(|segment| Identifier::new(*segment).unwrap())
            .collect(),
    )
    .unwrap()
}

fn source_digest(bytes: &[u8]) -> DigestRecord {
    DigestRecord::mint(
        DigestDomain::SourceBytesV1,
        ByteDigest::of(bytes).as_bytes(),
    )
}

/// A request replaying `function` against `package_id`, whose package
/// reference names `bytes` as its one source and whose byte provision
/// carries exactly `bytes`.
fn request(
    bytes: &[u8],
    package_id: PackageId,
    function: QualifiedName,
    source: ReplaySource,
) -> ReplayRequestWire {
    let digest = source_digest(bytes);
    ReplayRequestWire {
        profile_selections: vec![],
        package_id: (
            Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
            package_id.hex(),
        ),
        source_digests: vec![(
            AUTHORITY.to_owned(),
            IDENTITY.to_owned(),
            NAMESPACE.to_owned(),
            REVISION.to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            digest.hex(),
        )],
        dependencies: Vec::new(),
        selected_function: function,
        source,
        obligation_identity: [2; 32],
        backend: "kani-backend-1".to_owned(),
        state_environment: StateEnvironment::new(vec![]),
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        declared_domains: Vec::new(),
        byte_provision: vec![(
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            digest.hex(),
            bytes.to_vec(),
        )],
    }
}

fn input(parameter: WireNodeId, value: i64) -> ReplaySource {
    typed_input(parameter, WitnessValue::Integer(value))
}

fn typed_input(parameter: WireNodeId, value: WitnessValue) -> ReplaySource {
    ReplaySource::Input(vec![CanonicalAssignment { parameter, value }])
}

/// A request replaying `small(x)` against the proved unit.
fn small(x: i64) -> ReplayRequestWire {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    request(
        source.as_bytes(),
        compiled.emitted.package_id(),
        name(&["small"]),
        input(parameter(&compiled, "small", 0), x),
    )
}

/// A request replaying `p(x)` against the proved unit: `p`'s own body calls
/// `f`, a distinct declared function.
fn p(x: i64) -> ReplayRequestWire {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    request(
        source.as_bytes(),
        compiled.emitted.package_id(),
        name(&["p"]),
        input(parameter(&compiled, "p", 0), x),
    )
}

/// FR-098-AC-1, FR-098-AC-2: an `Input` counterexample `x = 7` recompiles
/// from the byte provision alone, keeps its `package_id`, joins `x` by
/// its parameter node id, and `small(7)` is `false`: the replay agrees
/// with the refuted property and settles `reproduced-without-witness`,
/// with the call's charges.
#[trace("TC-444", "FR-098-AC-1", "FR-098-AC-2")]
#[test]
fn tc_444_an_input_counterexample_replays_and_agrees() {
    let ReplayResult::Input(result) =
        replay(small(7), ReplayLimits::default()).expect("the replay runs")
    else {
        panic!("an Input-sourced request settles on the Input arm");
    };
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
    assert_eq!(result.category(), Category::Violation);
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(result.disagreement(), None);
    assert!(result.charges().work_units > 0);
}

/// FR-278-AC-3 (TC-759 step 3): the source provision of the `EmittedUnit`
/// names every source the package was compiled from by digest, and `replay`,
/// given that provision as its byte provision, recompiles the package to the
/// same `package_id`.
#[trace("TC-759", "FR-278-AC-3")]
#[test]
fn tc_759_the_emitted_provision_replays_to_the_same_package_id() {
    let source = proved();
    let limits = SpineLimits::default();
    let cancel = quire_exact::Cancel::new();
    let identity = SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION);
    let packages = BTreeMap::new();
    let parsed = crate::spine::parse(
        &crate::spine::ParseRequest {
            source: &identity,
            path: IDENTITY,
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("the unit parses")
    .into_value();
    let models = crate::spine::select(&parsed, &packages, limits.model, &cancel)
        .expect("the unit selects")
        .into_value();
    let checked = crate::spine::check(
        &parsed,
        &models,
        &crate::spine::DependencyInput::default(),
        &crate::spine::LockEvidence::default(),
        limits,
        &cancel,
    )
    .expect("the unit checks")
    .into_value();
    let emitted = crate::spine::package(&checked, crate::spine::PackageLimits::default(), &cancel)
        .expect("the unit emits")
        .into_value();

    let provision: Vec<_> = emitted
        .sources()
        .iter()
        .map(|source| {
            (
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source.reference().digest().hex(),
                source.text().as_bytes().to_vec(),
            )
        })
        .collect();
    assert_eq!(
        provision.iter().map(|entry| &entry.1).collect::<Vec<_>>(),
        vec![&source_digest(source.as_bytes()).hex()],
        "the provision names the unit's one source by its digest"
    );

    let compiled = spine(&source, &packages);
    let mut wire = request(
        source.as_bytes(),
        emitted.package().package_id(),
        name(&["small"]),
        input(parameter(&compiled, "small", 0), 7),
    );
    wire.byte_provision = provision;
    assert!(
        matches!(
            replay(wire, ReplayLimits::default()),
            Ok(ReplayResult::Input(_))
        ),
        "the provision recompiles the package the request names"
    );
}

/// FR-098-AC-2: a backend witness binds `x` by its parameter node id and
/// settles `reproduced-with-evaluated-witness` with its FR-351 record; a
/// transcript with no entry for `x`, or with an entry naming no parameter,
/// refuses with the decode's cause and the request's obligation identity.
#[trace("TC-444", "FR-098-AC-2")]
#[test]
fn tc_444_a_witness_decodes_by_parameter_node_id() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "small", 0);
    let witness = |values: String| {
        request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&["small"]),
            ReplaySource::Witness(Witness::parse(format!("<<<assertion|h|c|{values}>>>")).unwrap()),
        )
    };
    let ReplayResult::Witness(result) =
        replay(witness(format!("{x}=8")), ReplayLimits::default()).unwrap()
    else {
        panic!("a Witness-sourced request settles on the Witness arm");
    };
    assert_eq!(
        result.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    // FR-098-AC-2 (ADR-031 SW-7): a function call's result has no
    // decisive occurrence, so the agreement carries no record.
    assert!(result.record().is_none());

    let refused = replay(witness(String::new()), ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(
            refused,
            ReplayRefusal::Witness { refusal: DecodeRefusal::Missing(missing), .. } if missing == x
        ),
        "expected a missing entry, got {refused:?}"
    );
    let refused = replay(witness("x=8".to_owned()), ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(
            &refused,
            ReplayRefusal::Witness { refusal: DecodeRefusal::Unbound(name), .. } if name == "x"
        ),
        "expected an unbound entry, got {refused:?}"
    );

    // The decode refusal names the obligation the request replays, so a
    // batch of replays reports which one failed.
    let mut wire = witness(String::new());
    wire.obligation_identity = [7; 32];
    let refused = replay(wire, ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Witness { obligation, .. } = &refused else {
        panic!("expected a witness refusal, got {refused:?}");
    };
    assert_eq!(*obligation, crate::ObligationIdentity::from_digest([7; 32]));
    assert!(refused.to_string().contains(&obligation.to_string()));
}

/// FR-098-AC-5: a replay that disagrees -- `small(3)` holds -- settles
/// `inconclusive` with both verdicts as its typed cause; one whose S6a
/// outcome completes no value (no accounting budget: `incomplete`) does
/// too. Neither is repaired into agreement.
#[trace("TC-444", "FR-098-AC-5")]
#[test]
fn tc_444_a_disagreement_settles_inconclusive() {
    let ReplayResult::Input(holds) = replay(small(3), ReplayLimits::default()).unwrap() else {
        panic!("Input arm");
    };
    assert_eq!(holds.settlement(), InputSettlement::Inconclusive);
    assert_eq!(
        holds.disagreement(),
        Some(&crate::DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Success),
        })
    );
    assert_eq!(holds.value(), Some(EvaluatedValue::Boolean(true)));

    let mut starved = small(7);
    starved.accounting_limits = ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    };
    let ReplayResult::Input(incomplete) = replay(starved, ReplayLimits::default()).unwrap() else {
        panic!("Input arm");
    };
    assert_eq!(incomplete.settlement(), InputSettlement::Inconclusive);
    assert_eq!(
        incomplete
            .disagreement()
            .map(crate::DisagreementCause::replayed),
        Some(Verdict::from_category(Category::Incomplete))
    );
    assert_eq!(incomplete.value(), None);
}

/// QSL-22 Layer 3 exemplar: `p(x) { f(x) > 3 }` replays through a
/// nested call to `f(x) { x + 1 }`, a distinct declared function, and
/// agrees: `p(1)` is `f(1) > 3` = `2 > 3` = `false`, which agrees with the
/// counterexample's assumed violation. This is FR-098-AC-1's own agreement
/// case, over a predicate whose body is a `call` node rather than a direct
/// operator application -- the shape S4's emitter must write as a checked
/// `expression`/`call` node for codegen's FR-021 oracle generator to read
/// (`quire-contract-codegen/src/exact_function.rs`), confirmed structurally
/// by `qsl-package/src/emit/tests.rs`'s own `f`-calls-`g` round trip through
/// a real `quire.checked-package/v2` emit/decode (FR-065-AC-3, TC-163).
#[trace("TC-444", "FR-098-AC-1", "FR-098-AC-2")]
#[test]
fn tc_444_a_nested_function_call_replays_and_agrees() {
    let ReplayResult::Input(result) =
        replay(p(1), ReplayLimits::default()).expect("the replay runs")
    else {
        panic!("an Input-sourced request settles on the Input arm");
    };
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
    assert_eq!(result.category(), Category::Violation);
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(result.disagreement(), None);
}

/// `p(5)` is `f(5) > 3` = `6 > 3` = `true`, which disagrees with
/// the counterexample's assumed violation and settles `inconclusive` --
/// the nested call's own agreement and disagreement cases both replay
/// correctly through the same `qsl_replay::replay` entry `tc_444_a_
/// disagreement_settles_inconclusive` exercises for a direct operator body.
#[trace("TC-444", "FR-098-AC-5")]
#[test]
fn tc_444_a_nested_function_call_disagreement_settles_inconclusive() {
    let ReplayResult::Input(result) =
        replay(p(5), ReplayLimits::default()).expect("the replay runs")
    else {
        panic!("an Input-sourced request settles on the Input arm");
    };
    assert_eq!(result.settlement(), InputSettlement::Inconclusive);
    assert_eq!(
        result.disagreement(),
        Some(&crate::DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Success),
        })
    );
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(true)));
}

/// FR-098-AC-3: a meaning-affecting edit (`x < 6`) recompiles to another
/// `package_id` and refuses by it, naming both identities.
#[trace("TC-444", "FR-098-AC-3")]
#[test]
fn tc_444_a_meaning_edit_refuses_by_package_id() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let edited = source.replace("x < 5", "x < 6");
    let recompiled = spine(&edited, &BTreeMap::new()).emitted.package_id();
    assert_ne!(recompiled, compiled.emitted.package_id());
    let refused = replay(
        request(
            edited.as_bytes(),
            compiled.emitted.package_id(),
            name(&["small"]),
            input(parameter(&compiled, "small", 0), 7),
        ),
        ReplayLimits::default(),
    )
    .unwrap_err();
    let ReplayRefusal::PackageIdMismatch {
        requested,
        recompiled: found,
    } = refused
    else {
        panic!("expected a package_id mismatch, got {refused:?}");
    };
    assert_eq!(requested.hex(), compiled.emitted.package_id().hex());
    assert_eq!(found, recompiled);
}

/// FR-098-AC-3: a presentation-only edit (a blank line) keeps the
/// `package_id` but not the source digest the package reference names, so
/// the replay refuses by that digest: bytes supplied under their own new
/// digest leave the referenced source absent, and bytes supplied under the
/// referenced digest do not hash to it.
#[trace("TC-444", "FR-098-AC-3", "FR-098-AC-4")]
#[test]
fn tc_444_a_presentation_edit_refuses_by_source_digest() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let edited = source.replacen('\n', "\n\n", 1);
    assert_eq!(
        spine(&edited, &BTreeMap::new()).emitted.package_id(),
        compiled.emitted.package_id(),
        "the edit is presentation-only"
    );
    let x = parameter(&compiled, "small", 0);

    let mut absent = request(
        source.as_bytes(),
        compiled.emitted.package_id(),
        name(&["small"]),
        input(x, 7),
    );
    let edited_digest = source_digest(edited.as_bytes());
    absent.byte_provision = vec![(
        Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
        edited_digest.hex(),
        edited.as_bytes().to_vec(),
    )];
    let refused = replay(absent, ReplayLimits::default()).unwrap_err();
    assert_eq!(refused.code(), Code::MissingImport);
    let original = source_digest(source.as_bytes());
    let ReplayRefusal::Request(ReplayRequestRefusal::IncompleteByteProvision {
        requested,
        named_by,
    }) = &refused
    else {
        panic!("expected an incomplete byte provision, got {refused:?}");
    };
    assert_eq!(
        requested,
        &format!("{} {}", original.domain().as_str(), original.hex())
    );
    assert_eq!(named_by, "source_digests[0]");
    assert!(refused
        .to_string()
        .starts_with("missing_import/missing-selection:"));

    let mut mismatched = request(
        source.as_bytes(),
        compiled.emitted.package_id(),
        name(&["small"]),
        input(x, 7),
    );
    mismatched.byte_provision[0].2 = edited.clone().into_bytes();
    let refused = replay(mismatched, ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Request(ReplayRequestRefusal::ByteDigestMismatch { declared, actual }) =
        &refused
    else {
        panic!("expected a byte digest mismatch, got {refused:?}");
    };
    assert_eq!(declared, &format!("{:?}", source_digest(source.as_bytes())));
    assert_eq!(actual, &format!("{:?}", source_digest(edited.as_bytes())));
}

/// FR-098-AC-1, FR-098-AC-4: a domain package reaches I1 only from the byte
/// provision, under its `sha256-jcs` digest. With it the unit that selects
/// it replays; without it the recompile refuses at I1 (`missing_import`).
#[trace("TC-444", "FR-098-AC-1", "FR-098-AC-4")]
#[test]
fn tc_444_a_domain_package_comes_from_the_byte_provision() {
    let document = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/spine-model.semantic-ir.json"
    ))
    .unwrap();
    let jcs = PackageDocument::parse(&document).unwrap().jcs_digest();
    let hex: String = jcs.iter().map(|byte| format!("{byte:02x}")).collect();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{hex}\";\n\
         function small using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
    );
    let compiled = spine(&source, &package_input([document.as_slice()]));
    let wire = || {
        request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&["small"]),
            input(parameter(&compiled, "small", 0), 7),
        )
    };

    let mut with_package = wire();
    with_package.byte_provision.push((
        Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
        hex.clone(),
        document,
    ));
    let ReplayResult::Input(result) = replay(with_package, ReplayLimits::default()).unwrap() else {
        panic!("Input arm");
    };
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );

    let refused = replay(wire(), ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Recompile(refusal) = &refused else {
        panic!("expected a recompile refusal, got {refused:?}");
    };
    assert_eq!(refusal.stage(), SpineStage::Intake);
    assert_eq!(refusal.code(), Code::MissingImport);
}

/// FR-098-AC-4, TC-166 step 3: a well-formed selection naming no function
/// node refuses `UnknownFunction`, naming the exact selection and the
/// recompiled package -- an undeclared name, a name equal to `small` but for
/// case, and a qualified name whose last segment is `small`. None of them
/// falls back to a display-name match against `small`.
#[trace("TC-444", "TC-166", "FR-098-AC-4", "FR-062-AC-10", "FR-065-AC-6")]
#[test]
fn tc_444_a_selection_naming_no_function_refuses() {
    let recompiled = spine(&proved(), &BTreeMap::new()).emitted.package_id();
    for selection in [
        name(&["large"]),
        name(&["Small"]),
        name(&["module", "small"]),
    ] {
        let mut wire = small(7);
        wire.selected_function = selection.clone();
        let refused = replay(wire, ReplayLimits::default()).unwrap_err();
        let ReplayRefusal::UnknownFunction {
            selection: named,
            package,
        } = &refused
        else {
            panic!("expected an unknown function for {selection}, got {refused:?}");
        };
        assert_eq!(named, &selection);
        assert_eq!(package, &recompiled);
        assert_eq!(refused.code(), Code::MissingDeclaration);
    }
}

/// TC-166 step 2: a package declaring `small` and `Small`, whose names
/// differ only by case and whose bodies disagree at `x = 7`. Each request
/// selects one by its exact `QualifiedName` and replays that function's own
/// body: `small(7)` is `false`, `Small(7)` is `true`. A display-name match
/// that folded case would replay the same body for both.
#[trace("TC-166", "FR-062-AC-10", "FR-065-AC-6")]
#[test]
fn tc_166_case_variant_functions_each_replay_their_own_body() {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function small using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n\
         function Small using v(x: Int[0, 9]): Boolean pure {{ x > 5 }}\n"
    );
    let compiled = spine(&source, &BTreeMap::new());
    for (function, holds) in [("small", false), ("Small", true)] {
        let wire = request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&[function]),
            input(parameter(&compiled, function, 0), 7),
        );
        let ReplayResult::Input(result) =
            replay(wire, ReplayLimits::default()).expect("the replay runs")
        else {
            panic!("an Input-sourced request settles on the Input arm");
        };
        assert_eq!(
            result.value(),
            Some(EvaluatedValue::Boolean(holds)),
            "{function}(7) replays its own body"
        );
    }
}

/// FR-098-AC-4: arity mismatches refuse by parameter node id -- a parameter
/// with no argument, an argument naming no parameter of the selected
/// function (here `flag`'s parameter) and a parameter bound twice.
#[trace("TC-444", "FR-098-AC-4")]
#[test]
fn tc_444_an_arity_mismatch_refuses() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "small", 0);
    let b = parameter(&compiled, "flag", 0);
    let with = |assignments: Vec<CanonicalAssignment>| {
        replay(
            request(
                source.as_bytes(),
                compiled.emitted.package_id(),
                name(&["small"]),
                ReplaySource::Input(assignments),
            ),
            ReplayLimits::default(),
        )
        .unwrap_err()
    };

    let unbound = with(vec![]);
    assert!(
        matches!(unbound, ReplayRefusal::UnboundParameter(id) if id == x),
        "{unbound:?}"
    );
    let unknown = with(vec![
        CanonicalAssignment {
            parameter: x,
            value: WitnessValue::Integer(7),
        },
        CanonicalAssignment {
            parameter: b,
            value: WitnessValue::Integer(1),
        },
    ]);
    assert!(
        matches!(unknown, ReplayRefusal::UnknownParameter(id) if id == b),
        "{unknown:?}"
    );
    let twice = with(vec![
        CanonicalAssignment {
            parameter: x,
            value: WitnessValue::Integer(7),
        },
        CanonicalAssignment {
            parameter: x,
            value: WitnessValue::Integer(8),
        },
    ]);
    assert!(
        matches!(twice, ReplayRefusal::DuplicateArgument(id) if id == x),
        "{twice:?}"
    );
}

/// FR-098-AC-4: an argument not of its parameter's kind -- any integer
/// for an `Option<Boolean>` parameter, an integer for a Boolean one, a
/// Boolean for an integer one -- refuses before the call, and a value
/// outside the declared domain (`12` for `Int[0, 9]`) refuses at S6a
/// admission; each is `WrongValueKind` (`invalid_runtime_input`).
#[trace("TC-444", "FR-098-AC-4")]
#[test]
fn tc_444_a_type_or_domain_mismatch_refuses_as_wrong_value_kind() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let with = |function: &str, value: WitnessValue| {
        replay(
            request(
                source.as_bytes(),
                compiled.emitted.package_id(),
                name(&[function]),
                typed_input(parameter(&compiled, function, 0), value),
            ),
            ReplayLimits::default(),
        )
        .unwrap_err()
    };
    for refused in [
        with("maybe", WitnessValue::Integer(1)),
        with("flag", WitnessValue::Integer(1)),
        with("small", WitnessValue::Boolean(true)),
        with("small", WitnessValue::Integer(12)),
    ] {
        let ReplayRefusal::Input(refusal) = &refused else {
            panic!("expected an admission refusal, got {refused:?}");
        };
        assert_eq!(refusal, &InputRefusal::WrongValueKind { parameter: 0 });
        assert_eq!(refused.code(), Code::InvalidRuntimeInput);
    }
}

/// FR-098-AC-2: a Boolean parameter takes a witness entry `0` as `false`
/// and `1` as `true`, and an `Input` assignment's typed Boolean: `flag`
/// with `false` agrees with the refuted property, and with `true` does
/// not.
#[trace("TC-444", "FR-098-AC-2")]
#[test]
fn tc_444_a_boolean_parameter_replays() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let b = parameter(&compiled, "flag", 0);
    let flag = |source_of: ReplaySource| {
        replay(
            request(
                source.as_bytes(),
                compiled.emitted.package_id(),
                name(&["flag"]),
                source_of,
            ),
            ReplayLimits::default(),
        )
        .unwrap()
    };
    let witness = |bit: u8| {
        ReplaySource::Witness(Witness::parse(format!("<<<assertion|h|c|{b}={bit}>>>")).unwrap())
    };

    let ReplayResult::Witness(off) = flag(witness(0)) else {
        panic!("Witness arm");
    };
    assert_eq!(
        off.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    let ReplayResult::Witness(on) = flag(witness(1)) else {
        panic!("Witness arm");
    };
    assert_eq!(on.settlement(), WitnessSettlement::Inconclusive);

    let ReplayResult::Input(off) = flag(typed_input(b, WitnessValue::Boolean(false))) else {
        panic!("Input arm");
    };
    assert_eq!(off.settlement(), InputSettlement::ReproducedWithoutWitness);
    let ReplayResult::Input(on) = flag(typed_input(b, WitnessValue::Boolean(true))) else {
        panic!("Input arm");
    };
    assert_eq!(on.settlement(), InputSettlement::Inconclusive);
    assert_eq!(on.value(), Some(EvaluatedValue::Boolean(true)));
}

/// FR-098-AC-2: arguments take the function's declared parameter order,
/// not the order the assignments arrive in: `lt` with `b = 3` given before
/// `a = 5` is `5 < 3`, `false`, and agrees.
#[trace("TC-444", "FR-098-AC-2")]
#[test]
fn tc_444_arguments_follow_declared_parameter_order() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let ReplayResult::Input(result) = replay(
        request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&["lt"]),
            ReplaySource::Input(vec![
                CanonicalAssignment {
                    parameter: parameter(&compiled, "lt", 1),
                    value: WitnessValue::Integer(3),
                },
                CanonicalAssignment {
                    parameter: parameter(&compiled, "lt", 0),
                    value: WitnessValue::Integer(5),
                },
            ]),
        ),
        ReplayLimits::default(),
    )
    .unwrap() else {
        panic!("Input arm");
    };
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
}

/// FR-098-AC-4: an `s1.input_bytes` entry above `replay.input_bytes`
/// refuses before the recompile; one below the source's size stops the
/// recompile at S1 with `stage_limit_exceeded`; so does an `s3.work_units`
/// entry of 0 at S3.
#[trace("TC-444", "FR-098-AC-4")]
#[test]
fn tc_444_stage_limits_bound_the_recompile() {
    let reader = crate::bounds::DEFAULT_REPLAY_INPUT_BYTES;
    let mut above = small(7);
    above
        .stage_limits
        .insert("s1.input_bytes".to_owned(), reader + 1);
    let refused = replay(above, ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(
            refused,
            ReplayRefusal::LimitAboveReader(LimitAboveReader { requested, reader: limit })
                if requested == reader + 1 && limit == reader
        ),
        "{refused:?}"
    );
    // The request itself is invalid: a refusal (exit 20), not incomplete.
    assert_eq!(refused.code(), Code::InvalidRequest);
    assert_eq!(
        refused.code().category(),
        qsl_foundation::diagnostic::Category::Refusal
    );
    assert_eq!(refused.code().category().exit_code(), 20);

    let mut tight = small(7);
    tight.stage_limits.insert("s1.input_bytes".to_owned(), 16);
    let refused = replay(tight, ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Recompile(refusal) = &refused else {
        panic!("expected a recompile refusal, got {refused:?}");
    };
    assert_eq!(refusal.stage(), SpineStage::Source);
    assert_eq!(refusal.code(), Code::StageLimitExceeded);

    let mut no_work = small(7);
    no_work.stage_limits.insert("s3.work_units".to_owned(), 0);
    let refused = replay(no_work, ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Recompile(refusal) = &refused else {
        panic!("expected a recompile refusal, got {refused:?}");
    };
    assert_eq!(refusal.stage(), SpineStage::Check);
    assert_eq!(refusal.code(), Code::StageLimitExceeded);
    assert_eq!(refused.code(), Code::StageLimitExceeded);
}

/// FR-098-AC-1 (FR-001): the recompile runs under the package reference's
/// four labels, so a whitespace-only label refuses at S1 with
/// `invalid_source_identity`.
#[trace("TC-444", "FR-098-AC-1", "FR-001-AC-6")]
#[test]
fn tc_444_a_blank_label_refuses_at_the_source_stage() {
    let mut blank = small(7);
    blank.source_digests[0].0 = "   ".to_owned();
    let refused = replay(blank, ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Recompile(refusal) = &refused else {
        panic!("expected a recompile refusal, got {refused:?}");
    };
    assert_eq!(refusal.stage(), SpineStage::Source);
    assert_eq!(refusal.code(), Code::InvalidSourceIdentity);
    // FR-001-AC-6: the cause is `blank-label` and names the whitespace-only
    // authority; the refusal names no region.
    let CompileRefusal::Source(diagnostic) = &**refusal else {
        panic!("expected an S1 refusal, got {refusal:?}");
    };
    assert_eq!(
        diagnostic.cause,
        qsl_cst::CompleteCause::Host(qsl_cst::HostCause::BlankLabel {
            label: qsl_foundation::SourceLabel::Authority
        })
    );
    assert_eq!(diagnostic.cause.as_str(), "blank-label");
    assert_eq!(diagnostic.region, None);
}

/// FR-098-AC-1: the package reference names exactly one source, and only
/// sources: a definition document's digest and a second source each
/// refuse before anything is recompiled.
#[trace("TC-444", "FR-098-AC-1")]
#[test]
fn tc_444_the_package_reference_names_one_source() {
    let definition = b"definition bytes".to_vec();
    let digest = DigestRecord::mint(
        DigestDomain::DefinitionBytesV1,
        ByteDigest::of(&definition).as_bytes(),
    );
    let mut with_definition = small(7);
    with_definition.source_digests.push((
        AUTHORITY.to_owned(),
        "edition-def".to_owned(),
        NAMESPACE.to_owned(),
        REVISION.to_owned(),
        Some(DigestDomain::DefinitionBytesV1.as_str().to_owned()),
        digest.hex(),
    ));
    with_definition.byte_provision.push((
        Some(DigestDomain::DefinitionBytesV1.as_str().to_owned()),
        digest.hex(),
        definition,
    ));
    let refused = replay(with_definition, ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(&refused, ReplayRefusal::NotASource(reference) if reference.digest() == digest),
        "{refused:?}"
    );

    let other = b"language \"ix:native\" edition \"1-draft\";\n".to_vec();
    let other_digest = source_digest(&other);
    let mut two = small(7);
    two.source_digests.push((
        AUTHORITY.to_owned(),
        "test:other".to_owned(),
        NAMESPACE.to_owned(),
        REVISION.to_owned(),
        Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
        other_digest.hex(),
    ));
    two.byte_provision.push((
        Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
        other_digest.hex(),
        other,
    ));
    assert!(matches!(
        replay(two, ReplayLimits::default()),
        Err(ReplayRefusal::SourceCount(2))
    ));
}

/// FR-098-AC-4: a selected function whose declared result is not
/// `Boolean` states no property, so its replay refuses from the
/// declaration alone -- even with no accounting budget, where a call would
/// have stopped `incomplete`.
#[trace("TC-444", "FR-098-AC-4")]
#[test]
fn tc_444_a_non_predicate_refuses() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let mut starved = request(
        source.as_bytes(),
        compiled.emitted.package_id(),
        name(&["id"]),
        input(parameter(&compiled, "id", 0), 4),
    );
    starved.accounting_limits = ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    };
    let refused = replay(starved, ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(
            &refused,
            ReplayRefusal::NotAPredicate { selection, package }
                if selection == &name(&["id"]) && package == &compiled.emitted.package_id()
        ),
        "{refused:?}"
    );
}

/// TC-444 step 7's library `test/units` version `2`.
const UNITS_IDENTITY: &str = "test:units";

fn units_source(body: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\n{PROFILE}{body}")
}

const BIG: &str = "function big using v(x: Int[0, 9]): Boolean pure { x > 5 }\n";
const BIG_EDITED: &str = "function big using v(x: Int[0, 9]): Boolean pure { x > 6 }\n";

/// A source reference in wire shape for `bytes` under (`agent-ix`,
/// `identity`, `git`, `revision`).
fn source_ref(identity: &str, revision: &str, bytes: &[u8]) -> crate::identity::SourceDigestWire {
    (
        AUTHORITY.to_owned(),
        identity.to_owned(),
        NAMESPACE.to_owned(),
        revision.to_owned(),
        Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
        source_digest(bytes).hex(),
    )
}

/// A `dependencies` entry naming `identity` at `package_id`,
/// with `sources`.
fn entry(
    identity: &str,
    package_id: PackageId,
    sources: Vec<crate::identity::SourceDigestWire>,
) -> crate::request::DependencyEntryWire {
    crate::request::DependencyEntryWire {
        identity: identity.to_owned(),
        package_id: (
            Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
            package_id.hex(),
        ),
        sources,
    }
}

/// TC-444 step 7's proved unit, compiled against `test/units` supplied from
/// `units`: the unit's bytes, the compiled unit, `test/units`'s
/// `package_id` and the request replaying `q(x)`.
struct Importing {
    unit: String,
    units: String,
    units_id: PackageId,
    compiled: ComposedUnit,
}

impl Importing {
    fn new() -> Self {
        let units = units_source(BIG);
        let units_id = compose(
            SourceIdentity::new(AUTHORITY, UNITS_IDENTITY, NAMESPACE, REVISION),
            UNITS_IDENTITY,
            units.as_bytes(),
            &BTreeMap::new(),
            &crate::spine::DependencyInput::default(),
            SpineLimits::default(),
        )
        .expect("test/units compiles")
        .emitted
        .package_id();
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
             import \"test/units\" as u;\n\
             function q using v(x: Int[0, 9]): Boolean pure {{ u::big(x) }}\n"
        );
        let dependencies =
            crate::spine::DependencyInput::new(vec![crate::spine::SuppliedLibrary {
                identity: "test/units".to_owned(),
                source: SourceIdentity::new(AUTHORITY, UNITS_IDENTITY, NAMESPACE, REVISION),
                path: UNITS_IDENTITY.to_owned(),
                bytes: units.clone().into_bytes(),
            }])
            .unwrap();
        let compiled = compose(
            SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
            IDENTITY,
            unit.as_bytes(),
            &BTreeMap::new(),
            &dependencies,
            SpineLimits::default(),
        )
        .expect("the importing unit compiles");
        Self {
            unit,
            units,
            units_id,
            compiled,
        }
    }

    /// The request replaying `q(3)`, whose package reference's
    /// `dependencies` are `entries`, with every source in `provision` in its
    /// byte provision beside the unit's.
    fn request(
        &self,
        entries: Vec<crate::request::DependencyEntryWire>,
        provision: &[&[u8]],
    ) -> ReplayRequestWire {
        let mut wire = request(
            self.unit.as_bytes(),
            self.compiled.emitted.package_id(),
            name(&["q"]),
            input(parameter(&self.compiled, "q", 0), 3),
        );
        wire.dependencies = entries;
        for bytes in provision {
            let digest = source_digest(bytes);
            if !wire
                .byte_provision
                .iter()
                .any(|(_, hex, _)| *hex == digest.hex())
            {
                wire.byte_provision.push((
                    Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                    digest.hex(),
                    bytes.to_vec(),
                ));
            }
        }
        wire
    }

    /// The `test/units` entry, from the unit's own source.
    fn units_entry(&self) -> crate::request::DependencyEntryWire {
        entry(
            "test/units",
            self.units_id,
            vec![source_ref(UNITS_IDENTITY, REVISION, self.units.as_bytes())],
        )
    }
}

/// FR-098-AC-6 (TC-444 step 7): a proved package importing `test/units`
/// replays from the byte provision alone, the `dependencies` entry saying
/// which source is `test/units`; `q(3)` is `false` and agrees. An edited
/// `test/units` source refuses `DependencyIdentityMismatch` naming
/// `test/units`, before the proved package's own id is compared; a changed
/// entry
/// `package_id` alone refuses `DependencyIdentityMismatch` naming
/// `test/units`.
#[trace("TC-444", "FR-098-AC-6")]
#[test]
fn tc_444_a_package_with_a_dependency_replays_and_names_a_stale_one() {
    let importing = Importing::new();
    let ReplayResult::Input(result) = replay(
        importing.request(vec![importing.units_entry()], &[importing.units.as_bytes()]),
        ReplayLimits::default(),
    )
    .expect("the replay runs") else {
        panic!("Input arm");
    };
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));

    // The entry's source edited, its digest updated to the new bytes.
    let edited = units_source(BIG_EDITED);
    let mut stale = importing.units_entry();
    stale.sources = vec![source_ref(UNITS_IDENTITY, REVISION, edited.as_bytes())];
    let refused = replay(
        importing.request(vec![stale], &[edited.as_bytes()]),
        ReplayLimits::default(),
    )
    .expect_err("test/units no longer compiles to the recorded id");
    assert_eq!(refused.code(), Code::StaleDependency);
    assert!(
        matches!(
            &refused,
            ReplayRefusal::DependencyIdentityMismatch { identity, requested, recompiled }
                if identity.as_str() == "test/units"
                    && *requested == importing.units_id.record()
                    && *recompiled != importing.units_id
        ),
        "{refused:?}"
    );

    // Only the entry's `package_id` changed.
    let other = importing.compiled.emitted.package_id();
    let mut changed = importing.units_entry();
    changed.package_id = (
        Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
        other.hex(),
    );
    let refused = replay(
        importing.request(vec![changed], &[importing.units.as_bytes()]),
        ReplayLimits::default(),
    )
    .expect_err("the entry names another package_id");
    assert_eq!(refused.code(), Code::StaleDependency);
    assert!(
        matches!(
            &refused,
            ReplayRefusal::DependencyIdentityMismatch { identity, requested, recompiled }
                if identity.as_str() == "test/units"
                    && *requested == other.record()
                    && *recompiled == importing.units_id
        ),
        "{refused:?}"
    );
}

/// FR-121-AC-4 (TC-516): `call_site` compiles TC-444 step 7's importing
/// unit against a dependency input supplying `test/units`, and a request
/// carrying that dependency, keyed only by `call_site`'s own `package_id`
/// and parameter pair, replays and agrees. Without the dependency input the
/// same unit refuses `Import`.
#[trace("TC-516", "FR-121-AC-4")]
#[test]
fn call_site_with_a_dependency_input_keys_a_request_replay_accepts() {
    let importing = Importing::new();
    let locate = |dependencies: &crate::DependencyInput| {
        crate::call_site(
            SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
            IDENTITY,
            importing.unit.as_bytes(),
            [],
            dependencies,
            &name(&["q"]),
        )
    };
    let dependencies = crate::DependencyInput::new(vec![crate::SuppliedLibrary {
        identity: "test/units".to_owned(),
        source: SourceIdentity::new(AUTHORITY, UNITS_IDENTITY, NAMESPACE, REVISION),
        path: UNITS_IDENTITY.to_owned(),
        bytes: importing.units.clone().into_bytes(),
    }])
    .unwrap();
    let site = locate(&dependencies).expect("the importing unit compiles against test/units");
    let [(parameter, node)] = &site.site.parameters[..] else {
        panic!("q declares one parameter: {site:?}");
    };
    assert_eq!(parameter.as_str(), "x");

    let mut wire = importing.request(vec![importing.units_entry()], &[importing.units.as_bytes()]);
    wire.package_id = (
        Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
        site.package_id.hex(),
    );
    wire.source = input(*node, 3);
    let ReplayResult::Input(result) =
        replay(wire, ReplayLimits::default()).expect("the replay runs")
    else {
        panic!("Input arm");
    };
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));

    let refusal = locate(&crate::DependencyInput::default())
        .expect_err("no library is supplied as test/units");
    assert!(
        matches!(*refusal, crate::CallSiteRefusal::Import { .. }),
        "an unsupplied import refuses Import, got {refusal:?}"
    );
}

/// `call_site` over TC-444 step 7's importing unit, selecting `q`, with
/// `test/units` supplied from `source` and `bytes`.
fn locate_q_with_units(
    importing: &Importing,
    source: SourceIdentity,
    bytes: &[u8],
) -> Result<crate::CallSite<crate::FunctionSite>, Box<crate::CallSiteRefusal>> {
    let dependencies = crate::DependencyInput::new(vec![crate::SuppliedLibrary {
        identity: "test/units".to_owned(),
        source,
        path: UNITS_IDENTITY.to_owned(),
        bytes: bytes.to_vec(),
    }])
    .expect("one library under one identity");
    crate::call_site(
        SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
        IDENTITY,
        importing.unit.as_bytes(),
        [],
        &dependencies,
        &name(&["q"]),
    )
}

/// FR-121-AC-10 (TC-516): a supplied library whose source has the unit's
/// own source owner refuses `DependencyInput`, carrying the
/// `DependencyInputRefusal` that names the unit and the library.
#[trace("TC-516", "FR-121-AC-10")]
#[test]
fn call_site_refuses_a_library_sharing_the_units_owner_as_dependency_input() {
    let importing = Importing::new();
    let refusal = locate_q_with_units(
        &importing,
        SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
        importing.units.as_bytes(),
    )
    .expect_err("test/units has the unit's own owner");
    let crate::CallSiteRefusal::DependencyInput(crate::DependencyInputRefusal::SharedOwner {
        first,
        second,
        authority,
        identity,
    }) = *refusal
    else {
        panic!("expected DependencyInput(SharedOwner), got {refusal:?}");
    };
    assert_eq!(first, crate::SourceHolder::Unit);
    assert_eq!(second.as_str(), "test/units");
    assert_eq!(
        (authority.as_str(), identity.as_str()),
        (AUTHORITY, IDENTITY)
    );
}

/// FR-121-AC-11 (TC-516): a supplied library whose source does not compile
/// refuses `Dependency`, its path naming that library.
#[trace("TC-516", "FR-121-AC-11")]
#[test]
fn call_site_refuses_a_library_that_does_not_compile_as_dependency() {
    let importing = Importing::new();
    let refusal = locate_q_with_units(
        &importing,
        SourceIdentity::new(AUTHORITY, UNITS_IDENTITY, NAMESPACE, REVISION),
        b"language \"ix:native\" edition \"1-draft\";\nfunction {\n",
    )
    .expect_err("test/units does not parse");
    let crate::CallSiteRefusal::Dependency { path, .. } = *refusal else {
        panic!("expected Dependency, got {refusal:?}");
    };
    let path: Vec<&str> = path.iter().map(|library| library.as_str()).collect();
    assert_eq!(path, ["test/units"]);
}

/// FR-121-AC-14 (TC-516 step 14): `CallSiteRefusal::code` gives each
/// function-selection, import and dependency refusal the code `replay`
/// gives the same unit, dependency input and selection.
#[trace("TC-516", "FR-121-AC-14")]
#[test]
fn call_site_refusal_codes_are_the_replay_refusal_codes() {
    let unit_source = SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION);

    // AC-2's `UnknownFunction`: `missing_declaration`, as `replay` gives
    // a selection naming no function of the same package.
    let proved_source = proved();
    let compiled = spine(&proved_source, &BTreeMap::new());
    let refusal = crate::call_site(
        unit_source.clone(),
        IDENTITY,
        proved_source.as_bytes(),
        [],
        &crate::DependencyInput::default(),
        &name(&["large"]),
    )
    .expect_err("large is not declared");
    assert!(
        matches!(*refusal, crate::CallSiteRefusal::UnknownFunction { .. }),
        "{refusal:?}"
    );
    let mut wire = small(7);
    wire.selected_function = name(&["large"]);
    let replayed = replay(wire, ReplayLimits::default()).expect_err("large is not declared");
    assert_eq!(refusal.code(), Code::MissingDeclaration);
    assert_eq!(refusal.code(), replayed.code());

    // A unit with a syntax error: `Compile` carries the recompile's code.
    let broken = format!("language \"ix:native\" edition \"1-draft\";\n{PROFILE}function {{\n");
    let refusal = crate::call_site(
        unit_source.clone(),
        IDENTITY,
        broken.as_bytes(),
        [],
        &crate::DependencyInput::default(),
        &name(&["small"]),
    )
    .expect_err("the unit does not parse");
    assert!(
        matches!(*refusal, crate::CallSiteRefusal::Compile { .. }),
        "{refusal:?}"
    );
    let replayed = replay(
        request(
            broken.as_bytes(),
            compiled.emitted.package_id(),
            name(&["small"]),
            input(parameter(&compiled, "small", 0), 7),
        ),
        ReplayLimits::default(),
    )
    .expect_err("the unit does not parse");
    assert!(
        matches!(replayed, ReplayRefusal::Recompile(_)),
        "{replayed:?}"
    );
    assert_eq!(refusal.code(), replayed.code());

    let importing = Importing::new();
    let units_bytes = importing.units.as_bytes();

    // AC-4's `Import`: no dependency input, and no `dependencies` entry.
    let refusal = crate::call_site(
        unit_source,
        IDENTITY,
        importing.unit.as_bytes(),
        [],
        &crate::DependencyInput::default(),
        &name(&["q"]),
    )
    .expect_err("no library is supplied as test/units");
    assert!(
        matches!(*refusal, crate::CallSiteRefusal::Import { .. }),
        "{refusal:?}"
    );
    let replayed = replay(importing.request(Vec::new(), &[]), ReplayLimits::default())
        .expect_err("test/units is absent");
    assert_eq!(refusal.code(), Code::MissingImport);
    assert_eq!(refusal.code(), replayed.code());

    // AC-10's `DependencyInput`: the library's source has the unit's owner.
    let refusal = locate_q_with_units(
        &importing,
        SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
        units_bytes,
    )
    .expect_err("test/units has the unit's own owner");
    assert!(
        matches!(*refusal, crate::CallSiteRefusal::DependencyInput(_)),
        "{refusal:?}"
    );
    let mut shared = importing.units_entry();
    shared.sources = vec![source_ref(IDENTITY, REVISION, units_bytes)];
    let replayed = replay(
        importing.request(vec![shared], &[units_bytes]),
        ReplayLimits::default(),
    )
    .expect_err("a shared owner");
    assert!(
        matches!(replayed, ReplayRefusal::DependencyInput(_)),
        "{replayed:?}"
    );
    assert_eq!(refusal.code(), replayed.code());

    // AC-11's `Dependency`: the library's source does not parse.
    let unparsed: &[u8] = b"language \"ix:native\" edition \"1-draft\";\nfunction {\n";
    let refusal = locate_q_with_units(
        &importing,
        SourceIdentity::new(AUTHORITY, UNITS_IDENTITY, NAMESPACE, REVISION),
        unparsed,
    )
    .expect_err("test/units does not parse");
    assert!(
        matches!(*refusal, crate::CallSiteRefusal::Dependency { .. }),
        "{refusal:?}"
    );
    let mut broken_units = importing.units_entry();
    broken_units.sources = vec![source_ref(UNITS_IDENTITY, REVISION, unparsed)];
    let replayed = replay(
        importing.request(vec![broken_units], &[unparsed]),
        ReplayLimits::default(),
    )
    .expect_err("test/units does not parse");
    assert!(
        matches!(replayed, ReplayRefusal::Recompile(_)),
        "{replayed:?}"
    );
    assert_eq!(refusal.code(), replayed.code());
}

/// FR-098-AC-7 (TC-444 step 7): the `dependencies` entries' order, extent
/// and sources refuse by ADR-015 D-4's rules, each with no verdict.
#[trace("TC-444", "FR-098-AC-7")]
#[test]
fn tc_444_dependency_entries_refuse_by_the_d4_rules() {
    let importing = Importing::new();
    let units = importing.units_entry();
    let units_bytes = importing.units.as_bytes();
    let extra_bytes = units_source("function spare using v(): Boolean pure { true }\n");
    let extra = entry(
        "test/zzz",
        importing.units_id,
        vec![source_ref("test:zzz", REVISION, extra_bytes.as_bytes())],
    );
    let provision: &[&[u8]] = &[units_bytes, extra_bytes.as_bytes()];

    // An extra entry no import reaches: refused after the recompile.
    let refused = replay(
        importing.request(vec![units.clone(), extra.clone()], provision),
        ReplayLimits::default(),
    )
    .expect_err("test/zzz is not selected");
    assert_eq!(refused.code(), Code::InvalidPackage);
    assert!(
        matches!(
            &refused,
            ReplayRefusal::DependencySelections(DependencySelectionsCause::Unselected { identity })
                if identity.as_str() == "test/zzz"
        ),
        "{refused:?}"
    );

    // Swapped, and repeated: refused before any recompile, so even a
    // second entry that would not compile is never read.
    for (entries, index) in [
        (vec![extra.clone(), units.clone()], 1),
        (vec![units.clone(), units.clone()], 1),
    ] {
        let refused = replay(
            importing.request(entries, provision),
            ReplayLimits::default(),
        )
        .expect_err("out of order");
        assert!(
            matches!(
                &refused,
                ReplayRefusal::DependencySelections(DependencySelectionsCause::Unordered { index: at, .. })
                    if *at == index
            ),
            "{refused:?}"
        );
        assert_eq!(refused.code(), Code::InvalidPackage);
    }

    // An entry whose source has test/units's authority and identity.
    let mut same_owner = extra.clone();
    same_owner.sources = vec![source_ref(UNITS_IDENTITY, "r2", extra_bytes.as_bytes())];
    let refused = replay(
        importing.request(vec![units.clone(), same_owner], provision),
        ReplayLimits::default(),
    )
    .expect_err("one owner per compile");
    assert_eq!(refused.code(), Code::InvalidPackage);
    assert!(
        matches!(
            &refused,
            ReplayRefusal::DependencyInput(
                crate::spine::DependencyInputRefusal::SharedOwner { .. }
            )
        ),
        "{refused:?}"
    );

    // An entry whose source has the proved unit's authority and identity:
    // refused as the dependency input (rule 3), not as the recompile.
    let mut unit_owner = units.clone();
    unit_owner.sources = vec![source_ref(IDENTITY, "r2", importing.units.as_bytes())];
    let refused = replay(
        importing.request(vec![unit_owner], provision),
        ReplayLimits::default(),
    )
    .expect_err("the unit's owner is not a library's");
    assert_eq!(refused.code(), Code::InvalidPackage);
    assert!(
        matches!(
            &refused,
            ReplayRefusal::DependencyInput(crate::spine::DependencyInputRefusal::SharedOwner {
                first: crate::spine::SourceHolder::Unit,
                ..
            })
        ),
        "{refused:?}"
    );

    // The entry removed: the import has no supplied library.
    let refused = replay(importing.request(Vec::new(), &[]), ReplayLimits::default())
        .expect_err("test/units is not supplied");
    let ReplayRefusal::Recompile(refusal) = &refused else {
        panic!("expected a recompile refusal, got {refused:?}");
    };
    assert_eq!(refused.code(), Code::MissingImport);
    assert!(
        matches!(
            &**refusal,
            CompileRefusal::Import {
                refusal: crate::spine::ImportRefusal::MissingSelection { identity },
                ..
            } if identity.as_str() == "test/units"
        ),
        "{refusal:?}"
    );

    // The entry naming a second source.
    let mut two = units.clone();
    two.sources
        .push(source_ref("test:zzz", REVISION, extra_bytes.as_bytes()));
    let refused = replay(
        importing.request(vec![two], provision),
        ReplayLimits::default(),
    )
    .expect_err("two sources");
    assert!(
        matches!(refused, ReplayRefusal::SourceCount(2)),
        "{refused:?}"
    );

    // The entry naming a definition document in place of its source.
    let definition = b"definition bytes".to_vec();
    let digest = DigestRecord::mint(
        DigestDomain::DefinitionBytesV1,
        ByteDigest::of(&definition).as_bytes(),
    );
    let mut document = units.clone();
    document.sources = vec![(
        AUTHORITY.to_owned(),
        UNITS_IDENTITY.to_owned(),
        NAMESPACE.to_owned(),
        REVISION.to_owned(),
        Some(DigestDomain::DefinitionBytesV1.as_str().to_owned()),
        digest.hex(),
    )];
    let mut wire = importing.request(vec![document], &[]);
    wire.byte_provision.push((
        Some(DigestDomain::DefinitionBytesV1.as_str().to_owned()),
        digest.hex(),
        definition,
    ));
    let refused = replay(wire, ReplayLimits::default()).expect_err("a definition document");
    assert!(
        matches!(&refused, ReplayRefusal::NotASource(reference) if reference.digest() == digest),
        "{refused:?}"
    );
}

/// SR-746 FND-002 (FR-096-AC-15): `call_failure_to_replay_refusal`, the
/// mapping `replay`'s own `package.call` site applies, turns a
/// `CallFailure::Fault` -- which is what a kernel `Refusal::CheckedInvariant`
/// becomes at the S6a seam, never an `Ok` `Refused` outcome -- into
/// `ReplayRefusal::Fault`, never a settled `Category::Refusal` result.
#[test]
fn a_call_fault_settles_as_a_replay_fault_not_a_refusal() {
    let fault = InternalFault::new("S6a", "checked-program-invariant");
    let refusal = call_failure_to_replay_refusal(CallFailure::Fault(fault));
    match refusal {
        ReplayRefusal::Fault(fault) => {
            assert_eq!(fault.stage(), "S6a");
            assert_eq!(fault.invariant(), "checked-program-invariant");
        }
        other => panic!("expected ReplayRefusal::Fault, got {other:?}"),
    }
}

/// The unit FR-357's tests replay: `inc`, `dec`, `dbl` and `neg` are
/// unbounded-integer functions over one `+`, `-`, `*` and unary `-`, `inv` is
/// `Rational`-valued, and `small` is a predicate.
fn parity_unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function inc using v(x: Int[0, 9]): Integer pure {{ x + 1 }}\n\
         function inv using v(x: Int[1, 9]): Rational[0, 1; 1, 9] pure {{ 1 / x }}\n\
         function dec using v(x: Int[0, 9]): Integer pure {{ x - 1 }}\n\
         function dbl using v(x: Int[0, 9]): Integer pure {{ x * 2 }}\n\
         function neg using v(x: Int[0, 9]): Integer pure {{ -x }}\n\
         function small using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
    )
}

/// A request replaying `function` of [`parity_unit`] with `source`, built
/// from its parameter 0's node id.
fn parity_request(
    function: &str,
    bind: impl FnOnce(WireNodeId) -> ReplaySource,
) -> ReplayRequestWire {
    let source = parity_unit();
    let compiled = spine(&source, &BTreeMap::new());
    request(
        source.as_bytes(),
        compiled.emitted.package_id(),
        name(&[function]),
        bind(parameter(&compiled, function, 0)),
    )
}

fn parity(function: &str, x: i64) -> ReplayRequestWire {
    parity_request(function, |parameter| input(parameter, x))
}

fn integer(value: i64) -> ScalarOutcome {
    ScalarOutcome::Value(Value::Integer(Integer::from(value)))
}

fn one_over(denominator: i64) -> ScalarOutcome {
    ScalarOutcome::Value(Value::Rational(
        quire_exact::Rational::new(Integer::from(1_i64), Integer::from(denominator)).unwrap(),
    ))
}

/// `replay.input_bytes` is the caller's on the value entry: an S1 limit above
/// the default refuses `LimitAboveReader` under the default and settles once
/// the caller raises the bound.
#[trace("TC-904", "FR-357-AC-10")]
#[test]
fn tc_904_a_raised_replay_input_bound_admits_what_the_default_refuses_for_a_value() {
    let mut wire = parity("inc", 3);
    let above = crate::DEFAULT_REPLAY_INPUT_BYTES * 2;
    wire.stage_limits.insert("s1.input_bytes".to_owned(), above);
    let report = replay_value_parity(wire.clone(), integer(4), ReplayLimits::default());
    assert!(matches!(
        report.result(),
        ValueParityResult::Refused(refusal)
            if matches!(**refusal, ReplayRefusal::LimitAboveReader(_))
    ));
    let raised = ReplayLimits::default().with_input_bytes(above);
    let report = replay_value_parity(wire, integer(4), raised);
    assert!(matches!(report.result(), ValueParityResult::Agrees { .. }));
}

/// FR-357-AC-1: an integer function whose generated value differs from
/// QSL's `f(b)`, and one whose generated value stands where QSL's outcome is
/// out of range, each settle `Diverged` with both outcomes and the
/// evaluation's charges.
#[trace("TC-904", "FR-357-AC-1")]
#[test]
fn tc_904_an_integer_function_diverges_from_the_generated_outcome() {
    let report = replay_value_parity(parity("inc", 3), integer(5), ReplayLimits::default());
    let ValueParityResult::Diverged {
        qsl,
        generated,
        charges,
    } = report.result()
    else {
        panic!("inc(3) is 4, not 5");
    };
    assert!(qsl.same_as(&integer(4)), "{qsl:?}");
    assert!(generated.same_as(&integer(5)), "{generated:?}");
    assert!(charges.work_units > 0);

    let report = replay_value_parity(
        parity("inc", 3),
        ScalarOutcome::OutOfRange,
        ReplayLimits::default(),
    );
    let ValueParityResult::Diverged { qsl, .. } = report.result() else {
        panic!("inc(3) is a value, not out of range");
    };
    assert!(qsl.same_as(&integer(4)), "{qsl:?}");
}

/// FR-357-AC-1: a `Rational`-valued function's generated value differs from
/// QSL's.
#[trace("TC-904", "FR-357-AC-1")]
#[test]
fn tc_904_a_rational_function_diverges_from_the_generated_outcome() {
    let report = replay_value_parity(parity("inv", 2), one_over(3), ReplayLimits::default());
    let ValueParityResult::Diverged { qsl, generated, .. } = report.result() else {
        panic!("inv(2) is 1/2, not 1/3");
    };
    assert!(qsl.same_as(&one_over(2)), "{qsl:?}");
    assert!(generated.same_as(&one_over(3)), "{generated:?}");
}

/// FR-357-AC-2: a generated outcome equal to QSL's settles `Agrees`: the counterexample does not reproduce.
#[trace("TC-904", "FR-357-AC-2")]
#[test]
fn tc_904_an_integer_function_agrees_with_the_generated_outcome() {
    let report = replay_value_parity(parity("inc", 3), integer(4), ReplayLimits::default());
    let ValueParityResult::Agrees { agreement, .. } = report.result() else {
        panic!("inc(3) is 4");
    };
    let outcome = agreement.outcome();
    assert!(outcome.same_as(&integer(4)), "{outcome:?}");
}

/// FR-357-AC-2: the same for a `Rational`-valued function.
#[trace("TC-904", "FR-357-AC-2")]
#[test]
fn tc_904_a_rational_function_agrees_with_the_generated_outcome() {
    let report = replay_value_parity(parity("inv", 2), one_over(2), ReplayLimits::default());
    let ValueParityResult::Agrees { agreement, .. } = report.result() else {
        panic!("inv(2) is 1/2");
    };
    let outcome = agreement.outcome();
    assert!(outcome.same_as(&one_over(2)), "{outcome:?}");
}

/// FR-357-AC-3: bindings that fail S6a admission -- a value outside a
/// parameter's declared domain, or a Boolean for an integer parameter --
/// settle `RefusedInput` for each function, with no outcome compared.
#[trace("TC-904", "FR-357-AC-3")]
#[test]
fn tc_904_bindings_that_fail_admission_are_refused_input() {
    for (function, outcome) in [("inc", integer(13)), ("inv", one_over(12))] {
        let report = replay_value_parity(parity(function, 12), outcome, ReplayLimits::default());
        let result = report.result();
        assert!(
            matches!(
                result,
                ValueParityResult::RefusedInput(InputRefusal::WrongValueKind { parameter: 0 })
            ),
            "{function}: {result:?}"
        );
        let report = replay_value_parity(
            parity_request(function, |parameter| {
                typed_input(parameter, WitnessValue::Boolean(true))
            }),
            integer(0),
            ReplayLimits::default(),
        );
        let result = report.result();
        assert!(
            matches!(
                result,
                ValueParityResult::RefusedInput(InputRefusal::WrongValueKind { parameter: 0 })
            ),
            "{function}: {result:?}"
        );
    }
}

/// FR-357-AC-4: the bindings are the function's declared parameters and
/// nothing else: a binding naming a node that is no parameter, and a
/// parameter left unbound, each refuse, with no result.
#[trace("TC-904", "FR-357-AC-4")]
#[test]
fn tc_904_only_the_declared_parameters_are_bound() {
    let source = parity_unit();
    let compiled = spine(&source, &BTreeMap::new());
    let other = parameter(&compiled, "inv", 0);
    let stray = replay_value_parity(
        parity_request("inc", |_| input(other, 3)),
        integer(4),
        ReplayLimits::default(),
    );
    assert!(
        matches!(stray.result(), ValueParityResult::Refused(refusal) if matches!(**refusal, ReplayRefusal::UnknownParameter(node) if node == other)),
        "{stray:?}"
    );
    let unbound = replay_value_parity(
        parity_request("inc", |_| ReplaySource::Input(Vec::new())),
        integer(4),
        ReplayLimits::default(),
    );
    assert!(
        matches!(unbound.result(), ValueParityResult::Refused(refusal) if matches!(**refusal, ReplayRefusal::UnboundParameter(_))),
        "{unbound:?}"
    );
}

/// FR-357-AC-5: a value-parity replay of a function declared `Boolean`
/// refuses before any call: its claim is a predicate's.
#[trace("TC-904", "FR-357-AC-5")]
#[test]
fn tc_904_a_boolean_function_has_no_value_parity_replay() {
    let refused = replay_value_parity(
        parity("small", 3),
        ScalarOutcome::Value(Value::Boolean(true)),
        ReplayLimits::default(),
    );
    assert!(
        matches!(refused.result(), ValueParityResult::Refused(refusal) if matches!(&**refusal, ReplayRefusal::NotAValueFunction { selection, .. } if selection == &name(&["small"]))),
        "{refused:?}"
    );
}

/// FR-357-AC-6: predicate replay of a non-`Boolean` function still refuses
/// `NotAPredicate`, and a Boolean predicate replays as before.
#[trace("TC-904", "FR-357-AC-6")]
#[test]
fn tc_904_predicate_replay_still_refuses_a_non_boolean_function() {
    let refused = replay(parity("inc", 3), crate::ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(&refused, ReplayRefusal::NotAPredicate { selection, .. } if selection == &name(&["inc"])),
        "{refused:?}"
    );
    let ReplayResult::Input(result) =
        replay(parity("small", 7), crate::ReplayLimits::default()).expect("the replay runs")
    else {
        panic!("an Input-sourced request settles on the Input arm");
    };
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));
}

/// Starved limits: the function arm's exact evaluation stops at a limit.
#[trace("TC-904", "FR-357-AC-10")]
#[test]
fn tc_904_a_starved_function_evaluation_is_incomplete() {
    let mut starved = parity("inc", 3);
    starved.accounting_limits = ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    };
    let report = replay_value_parity(starved, integer(4), ReplayLimits::default());
    assert!(
        matches!(report.result(), ValueParityResult::Incomplete(_)),
        "{report:?}"
    );
    assert_eq!(
        report.terminal_value(),
        TerminalValue::Incomplete(crate::IncompleteCause::ResourceExhausted)
    );
}

/// The identity a function-level report must carry for `wire` and
/// `generated`, built as a consumer builds the claim it sent.
fn sent_value_claim(wire: &ReplayRequestWire, generated: &ScalarOutcome) -> crate::ValueIdentity {
    crate::ValueIdentity::sent(wire, generated)
}

/// The settlement a value-parity result names, for asserting each case of a
/// claim-identity test reached the outcome it is labelled with.
fn value_kind(result: &ValueParityResult) -> &'static str {
    match result {
        ValueParityResult::Diverged { .. } => "diverged",
        ValueParityResult::Agrees { .. } => "agrees",
        ValueParityResult::RefusedInput(_) => "refused input",
        ValueParityResult::Incomplete(_) => "incomplete",
        ValueParityResult::Refused(_) => "refused",
    }
}

/// FR-357-AC-13: a function-level report carries the claim identity the
/// request and generated outcome fixed on every outcome: diverged, agrees,
/// refused input, incomplete and each kind of refusal. Two claims that differ
/// in any member are not equal.
#[trace("TC-904", "FR-357-AC-13")]
#[test]
fn tc_904_a_value_report_carries_the_sent_claim_on_every_outcome() {
    let mut starved = parity("inc", 3);
    starved.accounting_limits = ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    };
    let mut stale = parity("inc", 3);
    stale.package_id = (
        Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
        DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]).hex(),
    );
    let mut undecodable = parity("inc", 3);
    undecodable.backend = String::new();
    let cases = [
        (parity("inc", 3), integer(5), "diverged"),
        (parity("inc", 3), integer(4), "agrees"),
        (parity("inc", 12), integer(13), "refused input"),
        (starved, integer(4), "incomplete"),
        (
            parity("small", 3),
            ScalarOutcome::Value(Value::Boolean(true)),
            "refused",
        ),
        (stale, integer(4), "refused"),
        (undecodable, integer(4), "refused"),
    ];
    for (wire, generated, kind) in cases {
        let sent = sent_value_claim(&wire, &generated);
        let report = replay_value_parity(wire, generated, ReplayLimits::default());
        assert_eq!(value_kind(report.result()), kind, "{:?}", report.result());
        assert_eq!(report.claim(), &sent, "{kind}: {:?}", report.result());
    }
    let sent = sent_value_claim(&parity("inc", 3), &integer(4));
    for other in [
        sent_value_claim(&parity("inc", 3), &integer(5)),
        sent_value_claim(&parity("inc", 4), &integer(4)),
        sent_value_claim(&parity("dec", 3), &integer(4)),
    ] {
        assert_ne!(sent, other);
    }
    let agrees = replay_value_parity(parity("inc", 3), integer(4), ReplayLimits::default());
    let ValueParityResult::Agrees { agreement, .. } = agrees.result() else {
        panic!("inc(3) is 4");
    };
    assert_eq!(
        agreement.claim(),
        &crate::ScalarClaim::Function(Box::new(agrees.claim().clone()))
    );
}

mod operator_arm {
    use super::*;
    use crate::{
        GeneratedFault, NativeOutcome, OperandRefusal, OperatorClaim, OperatorIdentity,
        OperatorParityResult, ScalarClaim, ScalarOperand, ScalarOperation,
    };

    fn int(value: i64) -> Integer {
        Integer::from(value)
    }

    fn range(lower: i64, upper: i64) -> quire_exact::IntegerInterval {
        quire_exact::IntegerInterval::new(int(lower), int(upper)).unwrap()
    }

    /// An operand drawn from `[0, 100]`.
    fn operand(value: i64) -> ScalarOperand {
        ScalarOperand {
            value,
            range: range(0, 100),
            identity: crate::OperandIdentity::GraphChild(WireNodeId::from_digest([0x11; 32])),
        }
    }

    /// The node of `parity_unit` whose operation identity is `identity`.
    fn node_of(identity: &str) -> WireNodeId {
        let compiled = spine(&parity_unit(), &BTreeMap::new());
        let node = compiled
            .package
            .graph()
            .semantic_graph()
            .nodes()
            .find(|node| {
                serde_json::from_slice::<serde_json::Value>(node.preimage())
                    .map(|json| json["body"]["operation"]["identity"] == identity)
                    .unwrap_or(false)
            })
            .expect("the unit has the application");
        WireNodeId::from_digest(*node.key().as_bytes())
    }

    /// The identity and enclosing function of each operator's node.
    fn site(operation: &ScalarOperation) -> (&'static str, &'static str) {
        match operation {
            ScalarOperation::Add { .. } => ("quire.op.integer.add", "inc"),
            ScalarOperation::Subtract { .. } => ("quire.op.integer.sub", "dec"),
            ScalarOperation::Multiply { .. } => ("quire.op.integer.mul", "dbl"),
            ScalarOperation::Negate { .. } => ("quire.op.integer.negate", "neg"),
        }
    }

    /// The nodes `node`'s application takes as arguments, in order.
    fn children_of(node: WireNodeId) -> Vec<WireNodeId> {
        let compiled = spine(&parity_unit(), &BTreeMap::new());
        let graph = compiled.package.graph().semantic_graph();
        let key = graph.resolve_wire(node).expect("the node is in the unit");
        let preimage: serde_json::Value =
            serde_json::from_slice(graph.node(key).expect("the node").preimage()).unwrap();
        preimage["body"]["arguments"]
            .as_array()
            .expect("an application")
            .iter()
            .map(|argument| {
                WireNodeId::from_hex(argument["target"]["digest"].as_str().unwrap()).unwrap()
            })
            .collect()
    }

    /// `operation` with each operand still naming the placeholder node
    /// renamed to the application's real child at its position.
    fn with_real_children(operation: ScalarOperation, node: WireNodeId) -> ScalarOperation {
        let children = children_of(node);
        let placeholder = operand(0).identity;
        let fix = |operand: ScalarOperand, position: usize| {
            if operand.identity == placeholder {
                ScalarOperand {
                    identity: crate::OperandIdentity::GraphChild(children[position]),
                    ..operand
                }
            } else {
                operand
            }
        };
        match operation {
            ScalarOperation::Add { left, right } => ScalarOperation::Add {
                left: fix(left, 0),
                right: fix(right, 1),
            },
            ScalarOperation::Subtract { left, right } => ScalarOperation::Subtract {
                left: fix(left, 0),
                right: fix(right, 1),
            },
            ScalarOperation::Multiply { left, right } => ScalarOperation::Multiply {
                left: fix(left, 0),
                right: fix(right, 1),
            },
            ScalarOperation::Negate { operand } => ScalarOperation::Negate {
                operand: fix(operand, 0),
            },
        }
    }

    /// A claim over `operation` at its own node, its placeholder operands
    /// naming the node's real children, result range `[-200, 200]`.
    fn claim(operation: ScalarOperation, generated: NativeOutcome) -> OperatorClaim {
        let (identity, _) = site(&operation);
        let node = node_of(identity);
        OperatorClaim {
            operation: with_real_children(operation, node),
            ..claim_raw(node, add(0, 0), generated)
        }
    }

    /// A claim over `operation` at `node`, exactly as given.
    fn claim_raw(
        node: WireNodeId,
        operation: ScalarOperation,
        generated: NativeOutcome,
    ) -> OperatorClaim {
        OperatorClaim {
            node,
            operation,
            occurrence: quire_exact::Origin::new(quire_exact::Role::new("expression"), 0),
            obligation_kind: "bounded_shadow".to_owned(),
            result_range: range(-200, 200),
            limits: UNLIMITED,
            generated,
            identity: DigestRecord::mint(DigestDomain::Sha256Jcs, [9; 32]),
        }
    }

    /// The obligation identity O-09's preimage gives `claim`.
    fn minted(claim: &OperatorClaim) -> crate::ObligationIdentity {
        let identity = claim.identity(obligation());
        crate::parity_obligation(&super::operator_parity::preimage_of(&identity))
            .expect("the preimage encodes")
    }

    /// A request for `function` carrying the identity minted for `claim`.
    fn wire_for(function: &str, claim: &OperatorClaim) -> ReplayRequestWire {
        let mut wire = parity(function, 3);
        wire.obligation_identity = *minted(claim).as_bytes();
        wire
    }

    /// Replay `claim` through the public entry, in its own function.
    fn replayed(claim: OperatorClaim) -> crate::OperatorParityReport {
        let (_, function) = site(&claim.operation);
        let expected = minted(&claim);
        let report = crate::replay_operator_parity(
            wire_for(function, &claim),
            claim,
            ReplayLimits::default(),
        );
        assert_eq!(report.obligation(), expected);
        report
    }

    fn add(left: i64, right: i64) -> ScalarOperation {
        ScalarOperation::Add {
            left: operand(left),
            right: operand(right),
        }
    }

    fn sub(left: i64, right: i64) -> ScalarOperation {
        ScalarOperation::Subtract {
            left: operand(left),
            right: operand(right),
        }
    }

    fn mul(left: i64, right: i64) -> ScalarOperation {
        ScalarOperation::Multiply {
            left: operand(left),
            right: operand(right),
        }
    }

    fn negate(value: i64) -> ScalarOperation {
        ScalarOperation::Negate {
            operand: operand(value),
        }
    }

    fn completed(value: i64) -> NativeOutcome {
        NativeOutcome::Completed(int(value))
    }

    /// The settlement an operator result names, for asserting each case of a
    /// claim-identity test reached the outcome it is labelled with.
    fn kind(result: &OperatorParityResult) -> &'static str {
        match result {
            OperatorParityResult::Diverged { .. } => "diverged",
            OperatorParityResult::Agrees { .. } => "agrees",
            OperatorParityResult::GeneratedFault(_) => "generated fault",
            OperatorParityResult::RefusedInput(_) => "refused input",
            OperatorParityResult::Incomplete(_) => "incomplete",
            OperatorParityResult::Refused(_) => "refused",
        }
    }

    fn obligation() -> crate::ObligationIdentity {
        crate::ObligationIdentity::from_digest([2; 32])
    }

    fn exact_value(value: i64) -> ScalarOutcome {
        ScalarOutcome::Value(Value::Integer(int(value)))
    }

    fn agreed(report: &crate::OperatorParityReport) -> &ScalarOutcome {
        match report.result() {
            OperatorParityResult::Agrees { agreement, .. } => agreement.outcome(),
            other => panic!("expected an agreement, got {other:?}"),
        }
    }

    /// FR-357-AC-7: `Completed(v)` agrees when the exact result equals `v`,
    /// for add, subtract, multiply and negate through the public entry; the
    /// agreement names the claim, the node and the obligation.
    #[trace("TC-904", "FR-357-AC-7")]
    #[test]
    fn tc_904_a_completed_native_value_agrees_with_the_exact_result() {
        for (operation, value) in [
            (add(40, 2), 42),
            (sub(40, 2), 38),
            (mul(6, 7), 42),
            (negate(7), -7),
        ] {
            let sent_claim = claim(operation.clone(), completed(value));
            let sent_operation = sent_claim.operation.clone();
            let expected = minted(&sent_claim);
            let report = replayed(sent_claim);
            assert!(agreed(&report).same_as(&exact_value(value)));
            let OperatorParityResult::Agrees { agreement, charges } = report.result() else {
                unreachable!()
            };
            assert!(charges.work_units > 0);
            let ScalarClaim::Operator(found) = agreement.claim() else {
                panic!("an operator claim");
            };
            let OperatorIdentity {
                obligation: carried,
                node,
                identity,
                operation: named,
                ..
            } = &**found;
            assert_eq!(*carried, expected);
            assert_eq!(*node, node_of(site(&operation).0));
            assert_eq!(
                *identity,
                DigestRecord::mint(DigestDomain::Sha256Jcs, [9; 32])
            );
            assert_eq!(named, &sent_operation);
        }
    }

    /// FR-357-AC-7: `Completed(v)` with a `v` that differs, or where the exact
    /// result is out of range, diverges.
    #[trace("TC-904", "FR-357-AC-7")]
    #[test]
    fn tc_904_a_completed_native_value_that_differs_diverges() {
        for (operation, generated, exact) in [
            (add(40, 2), completed(43), exact_value(42)),
            (mul(6, 7), completed(41), exact_value(42)),
            (mul(100, 100), completed(10_000), ScalarOutcome::OutOfRange),
        ] {
            let report = replayed(claim(operation, generated));
            let OperatorParityResult::Diverged { exact: found, .. } = report.result() else {
                panic!("expected a divergence: {:?}", report.result());
            };
            assert!(found.same_as(&exact), "{found:?}");
            assert_eq!(report.terminal_value(), TerminalValue::Failed);
        }
    }

    /// FR-357-AC-7: `RefusedOutOfRange` agrees iff the exact result is outside
    /// the result range, and diverges when it is in range.
    #[trace("TC-904", "FR-357-AC-7")]
    #[test]
    fn tc_904_a_native_refusal_agrees_iff_the_exact_result_is_out_of_range() {
        let report = replayed(claim(mul(100, 100), NativeOutcome::RefusedOutOfRange));
        assert!(agreed(&report).same_as(&ScalarOutcome::OutOfRange));
        let report = replayed(claim(add(1, 2), NativeOutcome::RefusedOutOfRange));
        assert!(matches!(
            report.result(),
            OperatorParityResult::Diverged { .. }
        ));
    }

    /// FR-357-AC-7: no operator QSL lowers is undefined, so a native
    /// `Undefined` diverges from the exact value.
    #[trace("TC-904", "FR-357-AC-7")]
    #[test]
    fn tc_904_a_native_undefined_diverges_from_a_defined_operator() {
        let report = replayed(claim(add(1, 2), NativeOutcome::Undefined));
        assert!(matches!(
            report.result(),
            OperatorParityResult::Diverged { .. }
        ));
    }

    /// FR-357-AC-8: a native run that stopped at a limit or failed is a
    /// generated-artifact fault, settled `Failed` with no comparison, even
    /// under starved exact limits.
    #[trace("TC-904", "FR-357-AC-8")]
    #[test]
    fn tc_904_an_incomplete_or_failed_native_run_is_a_generated_fault() {
        for (native, fault) in [
            (NativeOutcome::Incomplete, GeneratedFault::Incomplete),
            (
                NativeOutcome::ExecutionFault,
                GeneratedFault::ExecutionFault,
            ),
        ] {
            let mut starved = claim(add(1, 2), native);
            starved.limits = ScalarLimits {
                work_units: 0,
                ..UNLIMITED
            };
            let report = replayed(starved);
            assert!(
                matches!(report.result(), OperatorParityResult::GeneratedFault(found) if *found == fault)
            );
            assert_eq!(report.terminal_value(), TerminalValue::Failed);
        }
    }

    /// FR-357-AC-9: an operand outside its own range refuses
    /// `invalid_runtime_input` before any evaluation; a literal's range is the
    /// singleton `(value, value)`.
    #[trace("TC-904", "FR-357-AC-9")]
    #[test]
    fn tc_904_an_operand_outside_its_range_is_refused() {
        let report = replayed(claim(add(1, 101), completed(102)));
        assert!(
            matches!(
                report.result(),
                OperatorParityResult::RefusedInput(OperandRefusal { index: 1 })
            ),
            "{:?}",
            report.result()
        );
        assert_eq!(
            report.terminal_value(),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                Code::InvalidRuntimeInput
            ))
        );
        let literal = ScalarOperand::literal(7);
        assert_eq!(literal.range, range(7, 7));
        let with_literal = |value: i64| ScalarOperation::Add {
            left: operand(1),
            right: ScalarOperand {
                value,
                range: literal.range.clone(),
                identity: operand(0).identity,
            },
        };
        assert!(matches!(
            replayed(claim(with_literal(7), completed(8))).result(),
            OperatorParityResult::Agrees { .. }
        ));
        assert!(matches!(
            replayed(claim(with_literal(8), completed(9))).result(),
            OperatorParityResult::RefusedInput(OperandRefusal { index: 1 })
        ));
    }

    /// FR-357-AC-10: an agreement settles `Inconclusive(ScalarAgrees)` and a
    /// divergence `Failed`; neither is `Refuted`.
    #[trace("TC-904", "FR-357-AC-10")]
    #[test]
    fn tc_904_an_agreement_is_inconclusive_and_nothing_is_refuted() {
        let agrees = replayed(claim(add(40, 2), completed(42)));
        let TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(agreement)) =
            agrees.terminal_value()
        else {
            panic!("an agreement is inconclusive with ScalarAgrees");
        };
        assert!(agreement.outcome().same_as(&exact_value(42)));
        let diverged = replayed(claim(add(40, 2), completed(43)));
        assert_eq!(diverged.terminal_value(), TerminalValue::Failed);
        assert_ne!(agrees.terminal_value(), TerminalValue::Refuted);
        assert_ne!(diverged.terminal_value(), TerminalValue::Refuted);
    }

    /// FR-357-AC-10: an exact evaluation starved of its limits has no outcome.
    #[trace("TC-904", "FR-357-AC-10")]
    #[test]
    fn tc_904_a_starved_exact_evaluation_is_incomplete() {
        let mut starved = claim(add(1, 2), completed(3));
        starved.limits = ScalarLimits {
            work_units: 0,
            ..UNLIMITED
        };
        let report = replayed(starved);
        assert!(matches!(
            report.result(),
            OperatorParityResult::Incomplete(_)
        ));
        assert_eq!(
            report.terminal_value(),
            TerminalValue::Incomplete(crate::IncompleteCause::ResourceExhausted)
        );
    }

    /// `replay.input_bytes` is the caller's on the operator entry: an S1
    /// limit above the default refuses `LimitAboveReader` under the default
    /// and passes that stage once the caller raises the bound.
    #[trace("TC-904", "FR-357-AC-11")]
    #[test]
    fn tc_904_a_raised_replay_input_bound_admits_what_the_default_refuses() {
        let claim = claim_raw(WireNodeId::from_digest([7; 32]), add(1, 2), completed(3));
        let (_, function) = site(&claim.operation);
        let mut wire = wire_for(function, &claim);
        let above = crate::DEFAULT_REPLAY_INPUT_BYTES * 2;
        wire.stage_limits.insert("s1.input_bytes".to_owned(), above);
        let report =
            crate::replay_operator_parity(wire.clone(), claim.clone(), ReplayLimits::default());
        assert!(matches!(
            report.result(),
            OperatorParityResult::Refused(refusal)
                if matches!(**refusal, ReplayRefusal::LimitAboveReader(_))
        ));
        let raised = ReplayLimits::default().with_input_bytes(above);
        let report = crate::replay_operator_parity(wire, claim, raised);
        assert!(matches!(
            report.result(),
            OperatorParityResult::Refused(refusal)
                if !matches!(**refusal, ReplayRefusal::LimitAboveReader(_))
        ));
    }

    /// FR-357-AC-11: the request's obligation identity is carried unchanged
    /// into the agreement's claim identity (checked in the AC-7 test) and onto
    /// every settlement (`replayed` asserts it for each report above).
    ///
    /// FR-357-AC-11: a stale `package_id` refuses
    /// `PackageIdMismatch` (`stale_dependency/content-mismatch`); a node the
    /// package does not hold, a node that is not in the selected function's
    /// body and an operator the node is not an application of each refuse
    /// `ScalarIdentity` (`stale_dependency/revision-mismatch`) with their own
    /// cause; an enclosing function the package does not declare refuses
    /// `UnknownFunction` (`missing_declaration/missing-name`).
    #[trace("TC-904", "FR-357-AC-11")]
    #[test]
    fn tc_904_an_operator_claim_against_another_identity_is_refused() {
        use crate::ScalarIdentityMismatch as Mismatch;
        let add_claim = || claim(add(1, 2), completed(3));
        let refusal_of = |wire: ReplayRequestWire, claim: OperatorClaim| {
            let sent_id = crate::ObligationIdentity::from_digest(wire.obligation_identity);
            let report = crate::replay_operator_parity(wire, claim, ReplayLimits::default());
            assert_eq!(report.obligation(), sent_id);
            let OperatorParityResult::Refused(refusal) = report.result() else {
                panic!("{:?}", report.result());
            };
            let code = refusal.code();
            let settled = report.terminal_value();
            (refusal.to_string(), code, settled, report)
        };
        let stale_code =
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency));

        let mut stale = parity("inc", 3);
        stale.package_id = (
            Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]).hex(),
        );
        let (text, code, settled, report) = refusal_of(stale, add_claim());
        assert!(text.contains("content-mismatch"), "{text}");
        assert_eq!(code, Code::StaleDependency);
        assert_eq!(settled, stale_code);
        assert!(matches!(
            report.result(),
            OperatorParityResult::Refused(refusal)
                if matches!(**refusal, ReplayRefusal::PackageIdMismatch { .. })
        ));

        let mut missing_node = add_claim();
        missing_node.node = WireNodeId::from_digest([1; 32]);
        let mut wrong_function = add_claim();
        wrong_function.node = node_of("quire.op.integer.mul");
        wrong_function.operation = mul(1, 2);
        let mut wrong_operator = add_claim();
        wrong_operator.operation = mul(1, 2);
        for (wire, claim, expected) in [
            (parity("inc", 3), missing_node, "Node"),
            (parity("inc", 3), wrong_function, "Function"),
            (parity("inc", 3), wrong_operator, "Operator"),
        ] {
            let (text, code, settled, report) = refusal_of(wire, claim);
            assert!(text.contains("revision-mismatch"), "{text}");
            assert_eq!(code, Code::StaleDependency);
            assert_eq!(settled, stale_code);
            let OperatorParityResult::Refused(refusal) = report.result() else {
                unreachable!()
            };
            let ReplayRefusal::ScalarIdentity(cause) = &**refusal else {
                panic!("{refusal:?}");
            };
            let found = match &**cause {
                Mismatch::Node { .. } => "Node",
                Mismatch::Function { .. } => "Function",
                Mismatch::Operator { .. } => "Operator",
                Mismatch::Obligation { .. } => "Obligation",
                Mismatch::Encoding(_)
                | Mismatch::Occurrence { .. }
                | Mismatch::OperandCount { .. }
                | Mismatch::OperandChild { .. }
                | Mismatch::NotInlineLiteral { .. }
                | Mismatch::LiteralValue { .. } => "Other",
                Mismatch::Equality { .. } => "Other",
                Mismatch::Operand { .. } => "Other",
            };
            assert_eq!(found, expected);
        }

        let mut unknown = parity("inc", 3);
        unknown.selected_function = name(&["nope"]);
        let (text, code, settled, report) = refusal_of(unknown, add_claim());
        assert!(text.contains("missing-name"), "{text}");
        assert_eq!(code, Code::MissingDeclaration);
        assert_eq!(
            settled,
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::MissingDeclaration))
        );
        assert!(matches!(
            report.result(),
            OperatorParityResult::Refused(refusal)
                if matches!(**refusal, ReplayRefusal::UnknownFunction { .. })
        ));
    }

    /// The identity an operator report must carry for `claim` sent under the
    /// request's obligation.
    fn sent(claim: &OperatorClaim) -> OperatorIdentity {
        claim.identity(minted(claim))
    }

    /// FR-357-AC-13: an operator report carries the claim identity, with the
    /// obligation, node, operator, operands and ranges, result range, limits
    /// and observation digest, on every outcome: agrees, diverged, generated
    /// fault, refused operand, incomplete and each kind of refusal.
    #[trace("TC-904", "FR-357-AC-13")]
    #[test]
    fn tc_904_an_operator_report_carries_the_sent_claim_on_every_outcome() {
        let mut starved = claim(add(1, 2), completed(3));
        starved.limits = ScalarLimits {
            work_units: 0,
            ..UNLIMITED
        };
        let mut missing_node = claim(add(1, 2), completed(3));
        missing_node.node = WireNodeId::from_digest([1; 32]);
        let mut stale = parity("inc", 3);
        stale.package_id = (
            Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]).hex(),
        );
        let mut undecodable = parity("inc", 3);
        undecodable.backend = String::new();
        let cases = [
            (parity("inc", 3), claim(add(40, 2), completed(42)), "agrees"),
            (
                parity("inc", 3),
                claim(add(40, 2), completed(43)),
                "diverged",
            ),
            (
                parity("inc", 3),
                claim(add(1, 2), NativeOutcome::Incomplete),
                "generated fault",
            ),
            (
                parity("inc", 3),
                claim(add(1, 101), completed(102)),
                "refused input",
            ),
            (parity("inc", 3), starved, "incomplete"),
            (parity("inc", 3), missing_node, "refused"),
            (stale, claim(add(1, 2), completed(3)), "refused"),
            (undecodable, claim(add(1, 2), completed(3)), "refused"),
        ];
        for (mut wire, claim, expected_kind) in cases {
            wire.obligation_identity = *minted(&claim).as_bytes();
            let expected = sent(&claim);
            let report = crate::replay_operator_parity(wire, claim, ReplayLimits::default());
            assert_eq!(
                kind(report.result()),
                expected_kind,
                "{:?}",
                report.result()
            );
            assert_eq!(report.claim(), &expected, "{expected_kind}");
        }
        let base = claim(add(40, 2), completed(42));
        let mut other_node = base.clone();
        other_node.node = node_of("quire.op.integer.mul");
        let mut other_range = base.clone();
        other_range.result_range = range(-100, 100);
        let mut other_limits = base.clone();
        other_limits.limits = ScalarLimits {
            work_units: 7,
            ..UNLIMITED
        };
        let mut other_outcome = base.clone();
        other_outcome.generated = completed(43);
        for other in [
            claim(add(40, 3), completed(42)),
            claim(sub(40, 2), completed(42)),
            other_node,
            other_range,
            other_limits,
            other_outcome,
        ] {
            assert_ne!(sent(&base), sent(&other));
        }
    }

    /// FR-357-AC-14: the observation digest is carry-and-bind: the report
    /// carries exactly the digest the claim carried, on every outcome, and an
    /// agreement's claim holds it too. A digest that matches nothing is not
    /// refused, and claims that differ only in the digest are different
    /// claims.
    #[trace("TC-904", "FR-357-AC-14")]
    #[test]
    fn tc_904_the_observation_digest_is_carried_unchanged_and_never_recomputed() {
        let digest = |byte: u8| DigestRecord::mint(DigestDomain::Sha256Jcs, [byte; 32]);
        let with = |byte: u8, generated: NativeOutcome| {
            let mut claim = claim(add(40, 2), generated);
            claim.identity = digest(byte);
            claim
        };
        let agrees = replayed(with(7, completed(42)));
        assert_eq!(kind(agrees.result()), "agrees");
        assert_eq!(agrees.claim().identity, digest(7));
        let OperatorParityResult::Agrees { agreement, .. } = agrees.result() else {
            panic!("{:?}", agrees.result());
        };
        let ScalarClaim::Operator(held) = agreement.claim() else {
            panic!("an operator claim");
        };
        assert_eq!(held.identity, digest(7));
        for (byte, native, expected_kind) in [
            (0, completed(43), "diverged"),
            (255, NativeOutcome::ExecutionFault, "generated fault"),
            (1, completed(42), "agrees"),
        ] {
            let report = replayed(with(byte, native));
            assert_eq!(kind(report.result()), expected_kind);
            assert_eq!(report.claim().identity, digest(byte));
        }
        let other = replayed(with(8, completed(42)));
        assert_ne!(agrees.claim(), other.claim());
    }

    /// The SHA-256 of `text`, taken here independently of the encoder under
    /// test.
    fn digest_of(text: &str) -> crate::ObligationIdentity {
        crate::ObligationIdentity::from_digest(
            qsl_foundation::ByteDigest::of(text.as_bytes()).as_bytes(),
        )
    }

    fn at(node: WireNodeId, operation: ScalarOperation) -> OperatorClaim {
        claim_raw(node, operation, completed(0))
    }

    /// FR-357-AC-15: the recomputed obligation identity is the digest of the
    /// preimage's RFC 8785 text, written out here by hand, for graph-child
    /// operands and for inline literals; two literals at different positions
    /// of one application have distinct identities.
    #[trace("TC-904", "FR-357-AC-15")]
    #[test]
    fn tc_904_the_obligation_identity_is_recomputed_from_the_o09_preimage() {
        let node = WireNodeId::from_digest([0xaa; 32]);
        let app = "aa".repeat(32);
        let child = "11".repeat(32);

        let graph = at(
            node,
            ScalarOperation::Add {
                left: ScalarOperand::graph_child(
                    1,
                    range(-5, 100),
                    WireNodeId::from_digest([0x11; 32]),
                ),
                right: operand(2),
            },
        );
        let expected = format!(
            "{{\"arguments\":[\
{{\"domain\":{{\"lower\":\"-5\",\"tag\":\"range\",\"upper\":\"100\"}},\"operand\":{{\"node_id\":\"{child}\",\"tag\":\"graph_child\"}},\"position\":0}},\
{{\"domain\":{{\"lower\":\"0\",\"tag\":\"range\",\"upper\":\"100\"}},\"operand\":{{\"node_id\":\"{child}\",\"tag\":\"graph_child\"}},\"position\":1}}],\
\"node\":\"{app}\",\"obligation_kind\":\"bounded_shadow\",\"occurrence_key\":{{\"ordinal\":0,\"role\":\"expression\"}}}}"
        );
        assert_eq!(minted(&graph), digest_of(&expected));

        let literals = at(
            node,
            ScalarOperation::Add {
                left: ScalarOperand::literal(7),
                right: ScalarOperand::literal(7),
            },
        );
        let literal = |position: u8| {
            format!(
                "{{\"domain\":{{\"lower\":\"7\",\"tag\":\"range\",\"upper\":\"7\"}},\"operand\":{{\"node_id\":\"{app}\",\"occurrence_key\":{{\"ordinal\":0,\"role\":\"expression\"}},\"position\":{position},\"tag\":\"inline_literal\"}},\"position\":{position}}}"
            )
        };
        let expected = format!(
            "{{\"arguments\":[{},{}],\"node\":\"{app}\",\"obligation_kind\":\"bounded_shadow\",\"occurrence_key\":{{\"ordinal\":0,\"role\":\"expression\"}}}}",
            literal(0),
            literal(1)
        );
        assert_eq!(minted(&literals), digest_of(&expected));

        // Two literals of one application with the same value and range
        // differ only in position, and a literal and a graph child with the
        // same position and range differ in operand identity.
        let swapped = at(
            node,
            ScalarOperation::Add {
                left: ScalarOperand::literal(7),
                right: ScalarOperand {
                    identity: crate::OperandIdentity::GraphChild(node),
                    ..ScalarOperand::literal(7)
                },
            },
        );
        assert_ne!(minted(&literals), minted(&swapped));
        let only_second = at(
            node,
            ScalarOperation::Add {
                left: operand(7),
                right: ScalarOperand::literal(7),
            },
        );
        let only_first = at(
            node,
            ScalarOperation::Add {
                left: ScalarOperand::literal(7),
                right: operand(7),
            },
        );
        assert_ne!(minted(&only_first), minted(&only_second));
    }

    /// FR-357-AC-16: a request whose obligation identity is not the
    /// recomputed one refuses `ScalarIdentity` (`revision-mismatch`) with the
    /// cause `Obligation` naming both digests, after any member of the
    /// preimage changes.
    #[trace("TC-904", "FR-357-AC-16")]
    #[test]
    fn tc_904_a_tampered_obligation_identity_is_refused() {
        let base = claim(add(40, 2), completed(42));
        let mut other_range = base.clone();
        if let ScalarOperation::Add { left, .. } = &mut other_range.operation {
            left.range = range(0, 99);
        }
        let mut other_identity = base.clone();
        if let ScalarOperation::Add { right, .. } = &mut other_identity.operation {
            right.identity = crate::OperandIdentity::InlineLiteral;
        }
        let mut other_key = base.clone();
        other_key.occurrence = quire_exact::Origin::new(quire_exact::Role::new("expression"), 1);
        let mut other_kind = base.clone();
        other_kind.obligation_kind = "other".to_owned();
        for (label, minted_over) in [
            ("range", other_range),
            ("operand identity", other_identity),
            ("occurrence key", other_key),
            ("kind", other_kind),
        ] {
            let wire = wire_for("inc", &minted_over);
            let sent_id = crate::ObligationIdentity::from_digest(wire.obligation_identity);
            let report = crate::replay_operator_parity(wire, base.clone(), ReplayLimits::default());
            let OperatorParityResult::Refused(refusal) = report.result() else {
                panic!("{label}: {:?}", report.result());
            };
            assert!(
                matches!(&**refusal, ReplayRefusal::ScalarIdentity(cause)
                    if matches!(&**cause, crate::ScalarIdentityMismatch::Obligation { claimed, recomputed }
                        if *claimed == sent_id && *recomputed == minted(&base))),
                "{label}: {refusal:?}"
            );
            assert_eq!(refusal.code(), Code::StaleDependency, "{label}");
            assert_eq!(refusal.cause(), Some("revision-mismatch"), "{label}");
            let text = refusal.to_string();
            assert!(
                text.contains(&sent_id.to_string()) && text.contains(&minted(&base).to_string()),
                "{text}"
            );
            assert_eq!(report.claim().obligation, sent_id);
        }
    }

    fn mismatch_of(report: &crate::OperatorParityReport) -> &crate::ScalarIdentityMismatch {
        let OperatorParityResult::Refused(refusal) = report.result() else {
            panic!("expected a refusal: {:?}", report.result());
        };
        let ReplayRefusal::ScalarIdentity(cause) = &**refusal else {
            panic!("expected ScalarIdentity: {refusal:?}");
        };
        assert_eq!(refusal.code(), Code::StaleDependency);
        assert_eq!(refusal.cause(), Some("revision-mismatch"));
        cause
    }

    /// A claim at `inc`'s `+` node: `x` (a graph child) and the literal `1`
    /// (also a graph node in this package), replayed with its minted
    /// identity.
    fn refused_claim(claim: OperatorClaim) -> crate::OperatorParityReport {
        let wire = wire_for("inc", &claim);
        crate::replay_operator_parity(wire, claim, ReplayLimits::default())
    }

    /// FR-357-AC-17: the application node and its operator must be the
    /// recompiled package's, even under a correctly minted identity.
    #[trace("TC-904", "FR-357-AC-17")]
    #[test]
    fn tc_904_a_claim_on_another_node_or_operator_is_refused_before_the_identity() {
        let mut missing = claim(add(1, 2), completed(3));
        missing.node = WireNodeId::from_digest([1; 32]);
        assert!(matches!(
            mismatch_of(&refused_claim(missing)),
            crate::ScalarIdentityMismatch::Node { .. }
        ));
        let mut other_operator = claim(add(1, 2), completed(3));
        other_operator.operation = mul(1, 2);
        assert!(matches!(
            mismatch_of(&refused_claim(other_operator)),
            crate::ScalarIdentityMismatch::Operator { .. }
        ));
    }

    /// FR-357-AC-17: the occurrence key must be one of the node's.
    #[trace("TC-904", "FR-357-AC-17")]
    #[test]
    fn tc_904_an_occurrence_the_node_does_not_have_is_refused() {
        for occurrence in [
            quire_exact::Origin::new(quire_exact::Role::new("expression"), 1),
            quire_exact::Origin::new(quire_exact::Role::new("claim"), 0),
        ] {
            let mut claim = claim(add(1, 2), completed(3));
            claim.occurrence = occurrence.clone();
            let report = refused_claim(claim);
            assert!(
                matches!(mismatch_of(&report), crate::ScalarIdentityMismatch::Occurrence { occurrence: found, .. } if *found == occurrence),
                "{:?}",
                report.result()
            );
        }
    }

    /// FR-357-AC-17: a `GraphChild` operand must be the application's child
    /// at its position.
    #[trace("TC-904", "FR-357-AC-17")]
    #[test]
    fn tc_904_a_graph_child_that_is_not_the_nodes_child_is_refused() {
        let node = node_of("quire.op.integer.add");
        let children = children_of(node);
        let mut swapped = claim(add(1, 2), completed(3));
        if let ScalarOperation::Add { left, right } = &mut swapped.operation {
            left.identity = crate::OperandIdentity::GraphChild(children[1]);
            right.identity = crate::OperandIdentity::GraphChild(children[0]);
        }
        let report = refused_claim(swapped);
        assert!(
            matches!(mismatch_of(&report), crate::ScalarIdentityMismatch::OperandChild { position: 0, claimed, found: Some(found), .. } if *claimed == children[1] && *found == children[0]),
            "{:?}",
            report.result()
        );
        let mut invented = claim(add(1, 2), completed(3));
        if let ScalarOperation::Add { right, .. } = &mut invented.operation {
            right.identity = crate::OperandIdentity::GraphChild(WireNodeId::from_digest([7; 32]));
        }
        assert!(matches!(
            mismatch_of(&refused_claim(invented)),
            crate::ScalarIdentityMismatch::OperandChild { position: 1, .. }
        ));
    }

    /// FR-357-AC-17: an `InlineLiteral` operand must be an inline integer
    /// literal of the node at its position, with the literal's value as its
    /// value and its singleton range. This package lowers every operand as a
    /// node, so a claim of an inline literal refuses; the application's JSON
    /// is replaced by hand for the literal cases.
    #[trace("TC-904", "FR-357-AC-17")]
    #[test]
    fn tc_904_an_inline_literal_the_node_does_not_have_is_refused() {
        let report = refused_claim(claim(
            ScalarOperation::Add {
                left: operand(1),
                right: ScalarOperand::literal(1),
            },
            completed(2),
        ));
        assert!(
            matches!(
                mismatch_of(&report),
                crate::ScalarIdentityMismatch::NotInlineLiteral { position: 1, .. }
            ),
            "{:?}",
            report.result()
        );

        let child = WireNodeId::from_digest([0x22; 32]);
        let application = WireNodeId::from_digest([0xaa; 32]);
        let body = serde_json::json!({"body": {"arguments": [
            {"term": "reference", "target": {"digest": child.to_string()}},
            {"term": "literal", "value_kind": "integer", "value": "7"},
        ]}});
        let check = |operation: ScalarOperation| {
            super::super::scalar_site::check_operands(&body, application, &operation)
        };
        let mixed = |value: i64| ScalarOperation::Add {
            left: ScalarOperand::graph_child(5, range(0, 100), child),
            right: ScalarOperand::literal(value),
        };
        // A graph child and an inline literal together pass.
        assert!(check(mixed(7)).is_ok());
        // A different value, and a literal whose range is not the singleton.
        assert!(matches!(
            check(mixed(8)),
            Err(crate::ScalarIdentityMismatch::LiteralValue { position: 1, ref found, .. }) if found == "7"
        ));
        let mut wide = mixed(7);
        if let ScalarOperation::Add { right, .. } = &mut wide {
            right.range = range(0, 9);
        }
        assert!(matches!(
            check(wide),
            Err(crate::ScalarIdentityMismatch::LiteralValue { position: 1, .. })
        ));
        // The same operands in the other order name a literal where the node
        // has a reference.
        let reversed = ScalarOperation::Add {
            left: ScalarOperand::literal(7),
            right: ScalarOperand::graph_child(5, range(0, 100), child),
        };
        assert!(matches!(
            check(reversed),
            Err(crate::ScalarIdentityMismatch::NotInlineLiteral { position: 0, .. })
        ));
        // A claim of one operand on a node of two.
        assert!(matches!(
            check(negate(1)),
            Err(crate::ScalarIdentityMismatch::OperandCount {
                claimed: 1,
                found: 2,
                ..
            })
        ));
    }

    /// FR-357-AC-18: a claimed identity and a preimage the encoder refuses
    /// (an occurrence ordinal beyond 2^53) refuse with the typed encoding
    /// cause, whatever identity was claimed, the all-zero one included; a
    /// zero identity over a preimage that encodes is a mismatch.
    #[trace("TC-904", "FR-357-AC-18")]
    #[test]
    fn tc_904_a_preimage_the_encoder_refuses_never_yields_an_identity() {
        let verify = super::super::operator_parity::verify_obligation;
        let zero = crate::ObligationIdentity::from_digest([0; 32]);
        let mut huge = claim(add(1, 2), completed(3)).identity(zero);
        huge.occurrence =
            quire_exact::Origin::new(quire_exact::Role::new("expression"), 1_u64 << 60);
        assert!(matches!(
            verify(&huge, zero),
            Err(crate::ScalarIdentityMismatch::Encoding(
                crate::IdentityEncodeError::IntegerBeyondEncoder { magnitude }
            )) if magnitude == i128::from(1_u64 << 60)
        ));
        let sane = claim(add(1, 2), completed(3)).identity(zero);
        assert!(matches!(
            verify(&sane, zero),
            Err(crate::ScalarIdentityMismatch::Obligation { .. })
        ));
        // The same through the entry: a huge ordinal is no occurrence of the
        // node, so the membership check refuses first, and it still refuses.
        let mut claim = claim(add(1, 2), completed(3));
        claim.occurrence =
            quire_exact::Origin::new(quire_exact::Role::new("expression"), 1_u64 << 60);
        let mut wire = parity("inc", 3);
        wire.obligation_identity = [0; 32];
        let report = crate::replay_operator_parity(wire, claim, ReplayLimits::default());
        assert_eq!(kind(report.result()), "refused");
    }
}
