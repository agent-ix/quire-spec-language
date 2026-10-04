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
use crate::request::StateEnvironment;
use crate::result::{InputSettlement, WitnessSettlement};
use crate::spine::{compose, ComposedUnit, SpineStage};
use crate::witness::{CanonicalAssignment, Witness, WitnessValue};

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

/// The proving run's stage limits: S1 admits any source a request can
/// carry, and S3's work budget is used as given.
fn stage_limits() -> StageLimits {
    let s1 = ScalarLimits {
        text_input_bytes: u64::try_from(MAX_ENCODED_BYTES).unwrap(),
        ..UNLIMITED
    };
    StageLimits {
        s1,
        s2: UNLIMITED,
        s3: UNLIMITED,
        s4: UNLIMITED,
    }
}

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
        backend: (
            "kani-backend-1".to_owned(),
            Some(DigestDomain::ToolManifestJcsV1.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::ToolManifestJcsV1, [3; 32]).hex(),
        ),
        state_environment: StateEnvironment::new(vec![]),
        accounting_limits: UNLIMITED,
        stage_limits: stage_limits(),
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
    let ReplayResult::Input(result) = replay(small(7)).expect("the replay runs") else {
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
        matches!(replay(wire), Ok(ReplayResult::Input(_))),
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
    let ReplayResult::Witness(result) = replay(witness(format!("{x}=8"))).unwrap() else {
        panic!("a Witness-sourced request settles on the Witness arm");
    };
    assert_eq!(
        result.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    // FR-098-AC-2 (ADR-031 SW-7): a function call's result has no
    // decisive occurrence, so the agreement carries no record.
    assert!(result.record().is_none());

    let refused = replay(witness(String::new())).unwrap_err();
    assert!(
        matches!(
            refused,
            ReplayRefusal::Witness { refusal: DecodeRefusal::Missing(missing), .. } if missing == x
        ),
        "expected a missing entry, got {refused:?}"
    );
    let refused = replay(witness("x=8".to_owned())).unwrap_err();
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
    let refused = replay(wire).unwrap_err();
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
    let ReplayResult::Input(holds) = replay(small(3)).unwrap() else {
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
    let ReplayResult::Input(incomplete) = replay(starved).unwrap() else {
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
    let ReplayResult::Input(result) = replay(p(1)).expect("the replay runs") else {
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
    let ReplayResult::Input(result) = replay(p(5)).expect("the replay runs") else {
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
    let refused = replay(request(
        edited.as_bytes(),
        compiled.emitted.package_id(),
        name(&["small"]),
        input(parameter(&compiled, "small", 0), 7),
    ))
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
    assert!(matches!(
        replay(absent),
        Err(ReplayRefusal::Request(
            ReplayRequestRefusal::IncompleteByteProvision { .. }
        ))
    ));

    let mut mismatched = request(
        source.as_bytes(),
        compiled.emitted.package_id(),
        name(&["small"]),
        input(x, 7),
    );
    mismatched.byte_provision[0].2 = edited.into_bytes();
    assert!(matches!(
        replay(mismatched),
        Err(ReplayRefusal::Request(
            ReplayRequestRefusal::ByteDigestMismatch(_)
        ))
    ));
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
    let ReplayResult::Input(result) = replay(with_package).unwrap() else {
        panic!("Input arm");
    };
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );

    let refused = replay(wire()).unwrap_err();
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
        let refused = replay(wire).unwrap_err();
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
        let ReplayResult::Input(result) = replay(wire).expect("the replay runs") else {
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
        replay(request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&["small"]),
            ReplaySource::Input(assignments),
        ))
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
        replay(request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&[function]),
            typed_input(parameter(&compiled, function, 0), value),
        ))
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
        replay(request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&["flag"]),
            source_of,
        ))
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
    let ReplayResult::Input(result) = replay(request(
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
    ))
    .unwrap() else {
        panic!("Input arm");
    };
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(
        result.settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
}

/// FR-098-AC-4: an S1 limit above the reader limit refuses before the
/// recompile; one below the source's size stops the recompile at S1 with
/// `stage_limit_exceeded`.
#[trace("TC-444", "FR-098-AC-4")]
#[test]
fn tc_444_stage_limits_bound_the_recompile() {
    let reader = u64::try_from(MAX_ENCODED_BYTES).unwrap();
    let mut above = small(7);
    above.stage_limits.s1.text_input_bytes = reader + 1;
    let refused = replay(above).unwrap_err();
    assert!(
        matches!(
            refused,
            ReplayRefusal::LimitAboveReader(LimitAboveReader { requested, reader: limit })
                if requested == reader + 1 && limit == reader
        ),
        "{refused:?}"
    );

    let mut tight = small(7);
    tight.stage_limits.s1.text_input_bytes = 16;
    let refused = replay(tight).unwrap_err();
    let ReplayRefusal::Recompile(refusal) = &refused else {
        panic!("expected a recompile refusal, got {refused:?}");
    };
    assert_eq!(refusal.stage(), SpineStage::Source);
    assert_eq!(refusal.code(), Code::StageLimitExceeded);

    let mut no_work = small(7);
    no_work.stage_limits.s3.work_units = 0;
    let refused = replay(no_work).unwrap_err();
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
    let refused = replay(blank).unwrap_err();
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
    let refused = replay(with_definition).unwrap_err();
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
    assert!(matches!(replay(two), Err(ReplayRefusal::SourceCount(2))));
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
    let refused = replay(starved).unwrap_err();
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
    let ReplayResult::Input(result) =
        replay(importing.request(vec![importing.units_entry()], &[importing.units.as_bytes()]))
            .expect("the replay runs")
    else {
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
    let refused = replay(importing.request(vec![stale], &[edited.as_bytes()]))
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
    let refused = replay(importing.request(vec![changed], &[importing.units.as_bytes()]))
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
    let ReplayResult::Input(result) = replay(wire).expect("the replay runs") else {
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
    let replayed = replay(wire).expect_err("large is not declared");
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
    let replayed = replay(request(
        broken.as_bytes(),
        compiled.emitted.package_id(),
        name(&["small"]),
        input(parameter(&compiled, "small", 0), 7),
    ))
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
    let replayed = replay(importing.request(Vec::new(), &[])).expect_err("test/units is absent");
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
    let replayed =
        replay(importing.request(vec![shared], &[units_bytes])).expect_err("a shared owner");
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
    let replayed = replay(importing.request(vec![broken_units], &[unparsed]))
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
    let refused = replay(importing.request(vec![units.clone(), extra.clone()], provision))
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
        let refused = replay(importing.request(entries, provision)).expect_err("out of order");
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
    let refused = replay(importing.request(vec![units.clone(), same_owner], provision))
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
    let refused = replay(importing.request(vec![unit_owner], provision))
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
    let refused =
        replay(importing.request(Vec::new(), &[])).expect_err("test/units is not supplied");
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
    let refused = replay(importing.request(vec![two], provision)).expect_err("two sources");
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
    let refused = replay(wire).expect_err("a definition document");
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
