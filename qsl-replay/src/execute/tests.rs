// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098 (TC-444): the replay executor over real complete-V1 source,
//! recompiled through the spine.

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
use crate::spine::SpineStage;
use crate::witness::{CanonicalAssignment, Witness};

const PROFILE: &str = "profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// The proved unit: `small` is the predicate a counterexample refutes.
///
/// `f`/`p` (QSL-257, the QSL-22 Layer 3 exemplar's shape): `p` is a Boolean
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

fn spine(source: &str, packages: &BTreeMap<[u8; 32], Vec<u8>>) -> Compiled {
    compile(
        SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
        IDENTITY,
        source.as_bytes(),
        packages,
        SpineLimits::default(),
    )
    .expect("the unit compiles")
}

/// The wire node id of `function`'s parameter `index` in `compiled`.
fn parameter(compiled: &Compiled, function: &str, index: usize) -> WireNodeId {
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
        contract_version: "quire.native-runtime/v1".to_owned(),
        capability_vocabulary: Some("quire.capability-kind/v1".to_owned()),
        profile_selections: vec![],
        package_id: (
            Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
            package_id.hex(),
        ),
        package_contract_version: "quire.checked-package/v2".to_owned(),
        source_digests: vec![(
            AUTHORITY.to_owned(),
            IDENTITY.to_owned(),
            NAMESPACE.to_owned(),
            REVISION.to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            digest.hex(),
        )],
        selected_function: function,
        source,
        originating_counterexample_identity: [2; 32],
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
/// `f`, a distinct declared function (QSL-257).
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
/// with the executor's toolchain pin and the call's charges.
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
    assert_eq!(result.category(), ProofCategory::Violation);
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(result.disagreement(), None);
    assert_eq!(result.toolchain_pin().as_str(), TOOLCHAIN);
    assert!(result.charges().work_units > 0);
}

/// FR-098-AC-2: a backend witness binds `x` by its parameter node id and
/// settles `reproduced-with-evaluated-witness` with its FR-351 record; a
/// transcript binding no parameter refuses with the decode's cause.
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
    let record = result.record().expect("a decisive settlement has a record");
    assert_eq!(record.deciding_element, EvaluatedValue::Boolean(false));

    let refused = replay(witness("x=8".to_owned())).unwrap_err();
    let ReplayRefusal::Witness(DecodeRefusal::Missing(missing)) = &refused else {
        panic!("expected a missing binding, got {refused:?}");
    };
    assert_eq!(missing, &x.to_string());
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
        Some(crate::DisagreementCause::Verdicts {
            proved: Verdict::from_category(ProofCategory::Violation),
            replayed: Verdict::from_category(ProofCategory::Success),
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
        Some(Verdict::from_category(ProofCategory::Incomplete))
    );
    assert_eq!(incomplete.value(), None);
}

/// QSL-257 (QSL-22 Layer 3 exemplar): `p(x) { f(x) > 3 }` replays through a
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
    assert_eq!(result.category(), ProofCategory::Violation);
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(result.disagreement(), None);
}

/// QSL-257: `p(5)` is `f(5) > 3` = `6 > 3` = `true`, which disagrees with
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
        Some(crate::DisagreementCause::Verdicts {
            proved: Verdict::from_category(ProofCategory::Violation),
            replayed: Verdict::from_category(ProofCategory::Success),
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
            ReplayRequestRefusal::IncompleteByteProvision(_)
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

/// FR-098-AC-4: an unknown contract version refuses before anything is
/// recompiled.
#[trace("TC-444", "FR-098-AC-4")]
#[test]
fn tc_444_an_unknown_version_refuses() {
    let mut unknown = small(7);
    unknown.contract_version = "quire.native-runtime/v2".to_owned();
    assert!(matches!(
        replay(unknown),
        Err(ReplayRefusal::Request(
            ReplayRequestRefusal::UnknownContractVersion(version)
        )) if version == "quire.native-runtime/v2"
    ));
}

/// FR-098-AC-4: a selection naming no function node refuses, whether the
/// name is undeclared or qualified.
#[trace("TC-444", "FR-098-AC-4")]
#[test]
fn tc_444_a_selection_naming_no_function_refuses() {
    for selection in [name(&["large"]), name(&["module", "small"])] {
        let mut wire = small(7);
        wire.selected_function = selection.clone();
        let refused = replay(wire).unwrap_err();
        let ReplayRefusal::UnknownFunction {
            selection: named,
            package,
        } = &refused
        else {
            panic!("expected an unknown function, got {refused:?}");
        };
        assert_eq!(named, &selection);
        assert_eq!(
            package,
            &spine(&proved(), &BTreeMap::new()).emitted.package_id()
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
            value: 7,
        },
        CanonicalAssignment {
            parameter: b,
            value: 1,
        },
    ]);
    assert!(
        matches!(unknown, ReplayRefusal::UnknownParameter(id) if id == b),
        "{unknown:?}"
    );
    let twice = with(vec![
        CanonicalAssignment {
            parameter: x,
            value: 7,
        },
        CanonicalAssignment {
            parameter: x,
            value: 8,
        },
    ]);
    assert!(
        matches!(twice, ReplayRefusal::DuplicateArgument(id) if id == x),
        "{twice:?}"
    );
}

/// FR-098-AC-4: an argument not of its parameter's kind -- any integer
/// for an `Option<Boolean>` parameter, `2` for a Boolean one -- refuses before the
/// call, and a value outside the declared domain (`12` for `Int[0, 9]`)
/// refuses at S6a admission; each is `WrongValueKind`
/// (`invalid_runtime_input`).
#[trace("TC-444", "FR-098-AC-4")]
#[test]
fn tc_444_a_type_or_domain_mismatch_refuses_as_wrong_value_kind() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let with = |function: &str, value: i64| {
        replay(request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&[function]),
            input(parameter(&compiled, function, 0), value),
        ))
        .unwrap_err()
    };
    for refused in [with("maybe", 1), with("flag", 2), with("small", 12)] {
        let ReplayRefusal::Input(refusal) = &refused else {
            panic!("expected an admission refusal, got {refused:?}");
        };
        assert_eq!(refusal, &InputRefusal::WrongValueKind { parameter: 0 });
        assert_eq!(refused.code(), Code::InvalidRuntimeInput);
    }
}

/// FR-098-AC-2: a Boolean parameter takes `0` as `false` and `1` as
/// `true`: `flag(0)` is `false` and agrees with the refuted property, and
/// `flag(1)` is `true` and does not.
#[trace("TC-444", "FR-098-AC-2")]
#[test]
fn tc_444_a_boolean_parameter_replays() {
    let source = proved();
    let compiled = spine(&source, &BTreeMap::new());
    let flag = |value: i64| {
        let ReplayResult::Input(result) = replay(request(
            source.as_bytes(),
            compiled.emitted.package_id(),
            name(&["flag"]),
            input(parameter(&compiled, "flag", 0), value),
        ))
        .unwrap() else {
            panic!("Input arm");
        };
        result
    };
    assert_eq!(
        flag(0).settlement(),
        InputSettlement::ReproducedWithoutWitness
    );
    assert_eq!(flag(1).settlement(), InputSettlement::Inconclusive);
    assert_eq!(flag(1).value(), Some(EvaluatedValue::Boolean(true)));
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
                value: 3,
            },
            CanonicalAssignment {
                parameter: parameter(&compiled, "lt", 0),
                value: 5,
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
