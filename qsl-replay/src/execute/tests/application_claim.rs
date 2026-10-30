// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098-AC-22 to FR-098-AC-26 (TC-444 step 10): the application claim a
//! `function` request's occurrence key selects, over real complete-V1
//! source recompiled through the spine.

use super::*;

fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function p using v(x: Int[0, 9]): Boolean pure {{ x > 3 }}\n\
         function q using v(x: Int[0, 9]): Integer pure {{ x + 1 }}\n\
         function r using v(x: Int[0, 9]): Integer pure {{ 10 mod x }}\n\
         function s using v(x: Int[0, 9]): Integer pure {{ (if x > 0 then 10 mod x else 0) + 10 mod x }}\n\
         function b using v(t: Sequence<Int[0, 9]>[0, 3]): Boolean pure {{ forall(v in t: 10 mod v > 0) }}\n\
         function u using v(x: Int[0, 9], y: Int[0, 9]): Integer pure {{ 10 mod x }}\n\
         function inc using v(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}\n\
         function ub using v(x: Int[0, 9], flag: Boolean): Integer pure {{ 10 mod x + (if flag then 0 else 10 mod (x - 1)) }}\n\
         function ur using v(x: Int[0, 9], y: Int[3, 9]): Integer pure {{ 10 mod x + 10 mod (y - 3) }}\n\
         function un using v(x: Int[0, 9], y: Int[-9, -2]): Integer pure {{ 10 mod x + 10 mod (y + 2) }}\n\
         function us using v(x: Int[0, 9], t: Sequence<Int[0, 9]>[0, 3]): Integer pure {{ 10 mod x }}\n\
         function g using v(x: Int[0, 9]): Integer pure decreases(x) {{ 10 mod x + (if x > 0 then h(x - 1) else 0) }}\n\
         function h using v(x: Int[0, 9]): Integer pure decreases(x) {{ if x > 0 then g(x - 1) else 0 }}\n\
         function f using v(x: Int[0, 9]): Integer pure decreases(x) {{ 10 mod x + (if x > 0 then f(x - 1) else 0) }}\n"
    )
}

/// The `expression` occurrence of the `nth` (in source order) application of
/// `owner` whose source text is `needle`, read from the compiled package's
/// requirement records.
fn record(
    compiled: &ComposedUnit,
    source: &str,
    owner: &str,
    needle: &str,
    nth: usize,
) -> OccurrenceKey {
    let graph = compiled.package.graph();
    let mut found: Vec<(u64, OccurrenceKey)> = graph
        .requirements()
        .iter()
        .filter(|(_, record)| record.owner() == Some(owner))
        .filter_map(|(key, _)| {
            let node = graph.semantic_graph().resolve_wire(key.node())?;
            let location = graph.occurrence(node, key.origin())?;
            let region = graph.region(location)?;
            let text = &source
                [usize::try_from(region.start()).ok()?..usize::try_from(region.end()).ok()?];
            (text == needle).then(|| (region.start(), key.clone()))
        })
        .collect();
    found.sort_by_key(|(start, _)| *start);
    found
        .get(nth)
        .unwrap_or_else(|| panic!("`{owner}` holds no record #{nth} at `{needle}`"))
        .1
        .clone()
}

fn declaration(compiled: &ComposedUnit, function: &str) -> OccurrenceKey {
    let identity = compiled
        .package
        .graph()
        .callable(function)
        .expect("declared")
        .identity;
    OccurrenceKey::new(wire_id(identity), Origin::new(Role::new("declaration"), 0))
}

/// A request for `function` at `key` carrying `source`.
fn at(
    source_text: &str,
    compiled: &ComposedUnit,
    function: &str,
    key: &OccurrenceKey,
    source: ReplaySource,
) -> ReplayRequestWire {
    let mut wire = request(
        source_text.as_bytes(),
        compiled.emitted.package_id(),
        name(&[function]),
        source,
    );
    wire.replay_target = crate::RequestTargetWire::function(name(&[function]), key.clone());
    wire
}

/// What a replay settled, reduced to the AC's terms.
#[derive(Debug, Eq, PartialEq)]
enum Settled {
    Reproduced(Option<EvaluatedValue>),
    ReproducedWitness(Option<EvaluatedValue>),
    Verdicts(Option<EvaluatedValue>),
    NoValue,
}

fn settled(result: Result<ReplayResult, ReplayRefusal>) -> Settled {
    match result.expect("the replay settles") {
        ReplayResult::Input(arm) => match arm.settlement() {
            InputSettlement::ReproducedWithoutWitness => Settled::Reproduced(arm.value()),
            InputSettlement::Inconclusive => match arm.disagreement() {
                Some(crate::DisagreementCause::Verdicts { .. }) => Settled::Verdicts(arm.value()),
                Some(crate::DisagreementCause::NoValue { .. }) => Settled::NoValue,
                other => panic!("unexpected disagreement {other:?}"),
            },
        },
        ReplayResult::Witness(arm) => {
            assert!(
                arm.record().is_none(),
                "a function call carries no FR-351 record"
            );
            match arm.settlement() {
                WitnessSettlement::ReproducedWithEvaluatedWitness => {
                    Settled::ReproducedWitness(arm.value())
                }
                other => panic!("unexpected settlement {other:?}"),
            }
        }
        ReplayResult::ObservedTemporal(_) => panic!("function replay returned a temporal arm"),
        ReplayResult::ModelGraph(_) => panic!("function replay returned a state-graph arm"),
    }
}

fn boolean(value: bool) -> Option<EvaluatedValue> {
    Some(EvaluatedValue::Boolean(value))
}

fn run(
    source: &str,
    compiled: &ComposedUnit,
    function: &str,
    key: &OccurrenceKey,
    values: &[(NodeKey, WitnessValue)],
) -> Result<ReplayResult, ReplayRefusal> {
    let assignments = values
        .iter()
        .map(|(parameter, value)| CanonicalAssignment {
            parameter: *parameter,
            value: value.clone(),
        })
        .collect();
    replay(
        at(
            source,
            compiled,
            function,
            key,
            ReplaySource::Input(assignments),
        ),
        ReplayLimits::default(),
    )
}

fn int(value: i128) -> WitnessValue {
    WitnessValue::Integer(value)
}

/// FR-098-AC-22: the predicate claim at `declaration`, and the application
/// claim at a record's occurrence: undefined (or out of bound) reproduces,
/// a completed application is a `Verdicts` disagreement.
#[trace("TC-444", "FR-098-AC-22")]
#[test]
fn tc_444_the_occurrence_key_selects_the_predicate_or_the_application_claim() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let x = |function| parameter(&compiled, function, 0);

    let p_declaration = declaration(&compiled, "p");
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "p",
            &p_declaration,
            &[(x("p"), int(0))]
        )),
        Settled::Reproduced(boolean(false))
    );
    let p_application = record(&compiled, &source, "p", "x > 3", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "p",
            &p_application,
            &[(x("p"), int(0))]
        )),
        Settled::Verdicts(boolean(true))
    );

    let r_application = record(&compiled, &source, "r", "10 mod x", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "r",
            &r_application,
            &[(x("r"), int(0))]
        )),
        Settled::Reproduced(boolean(false))
    );
    let witness = |value: &str| {
        ReplaySource::Witness(
            Witness::parse(format!("<<<assertion|h|c|{}={value}>>>", x("r"))).unwrap(),
        )
    };
    assert_eq!(
        settled(replay(
            at(&source, &compiled, "r", &r_application, witness("0")),
            ReplayLimits::default()
        )),
        Settled::ReproducedWitness(boolean(false))
    );
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "r",
            &r_application,
            &[(x("r"), int(3))]
        )),
        Settled::Verdicts(boolean(true))
    );
    let q_application = record(&compiled, &source, "q", "x + 1", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "q",
            &q_application,
            &[(x("q"), int(9))]
        )),
        Settled::Verdicts(boolean(true))
    );

    // The obligation identity is provenance: any value is carried and not
    // checked against the claim.
    let mut identified = at(
        &source,
        &compiled,
        "p",
        &p_application,
        ReplaySource::Input(vec![CanonicalAssignment {
            parameter: x("p"),
            value: int(0),
        }]),
    );
    identified.obligation_identity = [9; 32];
    assert_eq!(
        settled(replay(identified, ReplayLimits::default())),
        Settled::Verdicts(boolean(true))
    );
    // The identity is carried unchanged into the request the result settles
    // from: a distinct identity per request survives decoding.
    for identity in [[1; 32], [2; 32]] {
        let mut wire = at(
            &source,
            &compiled,
            "p",
            &p_application,
            ReplaySource::Input(vec![CanonicalAssignment {
                parameter: x("p"),
                value: int(0),
            }]),
        );
        wire.obligation_identity = identity;
        let request = ReplayRequest::decode(wire, ReplayLimits::default()).unwrap();
        assert_eq!(request.obligation_identity().as_bytes(), &identity);
    }
}

/// FR-098-AC-23: two records of one node are two claims, each at its own
/// location.
#[trace("TC-444", "FR-098-AC-23")]
#[test]
fn tc_444_each_occurrence_of_one_node_is_a_claim_of_its_own() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "s", 0);
    let then_branch = record(&compiled, &source, "s", "10 mod x", 0);
    let right_operand = record(&compiled, &source, "s", "10 mod x", 1);
    assert_eq!(
        settled(run(&source, &compiled, "s", &then_branch, &[(x, int(0))])),
        Settled::NoValue
    );
    assert_eq!(
        settled(run(&source, &compiled, "s", &right_operand, &[(x, int(0))])),
        Settled::Reproduced(boolean(false))
    );
    assert_eq!(
        settled(run(&source, &compiled, "s", &then_branch, &[(x, int(3))])),
        Settled::Verdicts(boolean(true))
    );
}

/// FR-098-AC-24: every evaluation of a binder body is observed; a binder
/// root is never bound by the source; an unread parameter is filled and an
/// extent root left unbound refuses.
#[trace("TC-444", "FR-098-AC-24")]
#[test]
fn tc_444_binder_bodies_are_observed_and_unread_parameters_are_filled() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let t = parameter(&compiled, "b", 0);
    let sequence = |elements: &[i128]| {
        WitnessValue::Sequence(elements.iter().map(|element| int(*element)).collect())
    };
    let b_application = record(&compiled, &source, "b", "10 mod v", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "b",
            &b_application,
            &[(t, sequence(&[3, 0]))]
        )),
        Settled::Reproduced(boolean(false))
    );
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "b",
            &b_application,
            &[(t, sequence(&[3]))]
        )),
        Settled::Verdicts(boolean(true))
    );
    // `v` is the binder of the quantifier: no parameter of `b`.
    let binder = record_binder_node(&compiled, "b");
    let refused = run(
        &source,
        &compiled,
        "b",
        &b_application,
        &[(t, sequence(&[3])), (binder, int(0))],
    )
    .unwrap_err();
    assert!(matches!(refused, ReplayRefusal::UnknownParameter(node) if node == binder));

    let x = parameter(&compiled, "u", 0);
    let y = parameter(&compiled, "u", 1);
    let u_application = record(&compiled, &source, "u", "10 mod x", 0);
    assert_eq!(
        settled(run(&source, &compiled, "u", &u_application, &[(x, int(0))])),
        Settled::Reproduced(boolean(false))
    );
    let refused = run(&source, &compiled, "u", &u_application, &[(y, int(1))]).unwrap_err();
    assert!(matches!(refused, ReplayRefusal::UnboundParameter(node) if node == x));
    // The predicate claim binds every parameter.
    let u_declaration = declaration(&compiled, "u");
    let refused = run(&source, &compiled, "u", &u_declaration, &[(x, int(0))]).unwrap_err();
    assert!(matches!(refused, ReplayRefusal::NotAPredicate { .. }));
    // A witness transcript may likewise leave the unread parameter out.
    let transcript =
        ReplaySource::Witness(Witness::parse(format!("<<<assertion|h|c|{x}=0>>>")).unwrap());
    assert_eq!(
        settled(replay(
            at(&source, &compiled, "u", &u_application, transcript),
            ReplayLimits::default()
        )),
        Settled::ReproducedWitness(boolean(false))
    );
}

/// The node of `function`'s first `forall` binder: the node of a
/// quantifier's bound variable, which is no parameter.
fn record_binder_node(compiled: &ComposedUnit, function: &str) -> NodeKey {
    let graph = compiled.package.graph();
    let record = graph
        .requirements()
        .values()
        .find(|record| record.owner() == Some(function))
        .expect("a record");
    let parameters: Vec<NodeKey> = (0..1).map(|i| parameter(compiled, function, i)).collect();
    *record
        .extent_roots()
        .iter()
        .find(|root| !parameters.contains(root))
        .expect("the binder is an extent root")
}

/// FR-098-AC-25: a key that selects no claim of the function refuses
/// `ClaimOccurrence` before any call; the predicate and value refusals are
/// unchanged.
#[trace("TC-444", "FR-098-AC-25")]
#[test]
fn tc_444_a_key_selecting_no_claim_of_the_function_refuses() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let x = |function| parameter(&compiled, function, 0);
    let r_application = record(&compiled, &source, "r", "10 mod x", 0);
    let no_node = OccurrenceKey::new(
        crate::fixture_node(0x11),
        Origin::new(Role::new("expression"), 0),
    );
    // The narrow that wraps `inc`'s `x + 1` is located with the application
    // but holds no record.
    let graph = compiled.package.graph();
    let inc_application = record(&compiled, &source, "inc", "x + 1", 0);
    let narrows: Vec<OccurrenceKey> = graph
        .occurrences()
        .filter(|(_, origin, location)| {
            origin.role() == &Role::new("expression")
                && graph.region(location).is_some_and(|region| {
                    let (start, end) = (region.start() as usize, region.end() as usize);
                    source.get(start..end) == Some("x + 1")
                        && source[..start].ends_with("Int[0, 10] pure { ")
                })
        })
        .map(|(node, origin, _)| OccurrenceKey::new(wire_id(node), origin))
        .filter(|key| *key != inc_application && graph.requirements().get(key).is_none())
        .collect();
    assert!(
        !narrows.is_empty(),
        "inc's narrow holds an occurrence and no record"
    );
    for narrow in narrows {
        let refused = run(&source, &compiled, "inc", &narrow, &[(x("inc"), int(0))]).unwrap_err();
        assert!(
            matches!(refused, ReplayRefusal::ClaimOccurrence { .. }),
            "{narrow:?}: {refused:?}"
        );
    }
    let cases = [
        ("p", r_application.clone()),
        ("p", declaration(&compiled, "r")),
        ("p", no_node),
    ];
    for (function, key) in cases {
        let mut limited = at(
            &source,
            &compiled,
            function,
            &key,
            ReplaySource::Input(vec![CanonicalAssignment {
                parameter: x(function),
                value: int(0),
            }]),
        );
        // No charge is possible before a call: even no budget refuses alike.
        limited.accounting_limits = ScalarLimits {
            work_units: 0,
            ..UNLIMITED
        };
        let refused = replay(limited, ReplayLimits::default()).unwrap_err();
        let ReplayRefusal::ClaimOccurrence {
            key: named,
            function: selected,
        } = &refused
        else {
            panic!("expected ClaimOccurrence for {function} at {key:?}, got {refused:?}");
        };
        assert_eq!(named, &key);
        assert_eq!(selected, &name(&[function]));
        assert_eq!(refused.code(), Code::MissingDeclaration);
        assert_eq!(refused.cause(), Some("missing-name"));
    }
    let r_declaration = declaration(&compiled, "r");
    let refused = run(&source, &compiled, "r", &r_declaration, &[(x("r"), int(0))]).unwrap_err();
    assert!(matches!(refused, ReplayRefusal::NotAPredicate { .. }));
    let refused = run(
        &source,
        &compiled,
        "r",
        &r_application,
        &[(x("r"), int(12))],
    )
    .unwrap_err();
    assert_eq!(
        refused.input_refusal(),
        Some(&InputRefusal::WrongValueKind { parameter: 0 }),
        "{refused:?}"
    );
}

/// FR-098-AC-26: the refusal order. A stale `package_id` and an unknown
/// function precede `ClaimOccurrence`, which precedes the argument join.
#[trace("TC-444", "FR-098-AC-26")]
#[test]
fn tc_444_claim_selection_comes_after_the_function_and_before_the_join() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "p", 0);
    let r_application = record(&compiled, &source, "r", "10 mod x", 0);
    let build = || {
        at(
            &source,
            &compiled,
            "p",
            &r_application,
            ReplaySource::Input(vec![CanonicalAssignment {
                parameter: x,
                value: int(0),
            }]),
        )
    };
    let mut stale = build();
    stale.package_id = (
        Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
        DigestRecord::mint(DigestDomain::PackageSemanticV2, [5; 32]).hex(),
    );
    assert!(matches!(
        replay(stale, ReplayLimits::default()).unwrap_err(),
        ReplayRefusal::PackageIdMismatch { .. }
    ));
    let mut unknown = build();
    unknown.replay_target =
        crate::RequestTargetWire::function(name(&["nope"]), r_application.clone());
    assert!(matches!(
        replay(unknown, ReplayLimits::default()).unwrap_err(),
        ReplayRefusal::UnknownFunction { .. }
    ));
    let mut stray = build();
    stray.source = ReplaySource::Input(vec![
        CanonicalAssignment {
            parameter: x,
            value: int(0),
        },
        CanonicalAssignment {
            parameter: crate::fixture_node(0x22),
            value: int(0),
        },
    ]);
    assert!(matches!(
        replay(stray, ReplayLimits::default()).unwrap_err(),
        ReplayRefusal::ClaimOccurrence { .. }
    ));
}

/// FR-098-AC-24, the unread-parameter rule per type: a `Boolean` fills
/// `false`, a range without 0 fills its end nearest 0 (observably), and a parameter of any
/// other type stays unbound and refuses `UnboundParameter`.
#[trace("TC-444", "FR-098-AC-24")]
#[test]
fn tc_444_unread_parameters_fill_by_type() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    // Each body also reads the unread parameter so that only the correct
    // fill is undefined there: the claim site (`10 mod x`) is defined at
    // `x = 1`, and the later undefined step is another location, so only the
    // correct fill (`false`, 3, -2) settles `NoValue`. Any other in-range
    // fill completes and settles `Verdicts(true)`.
    for function in ["ub", "ur", "un"] {
        let x = parameter(&compiled, function, 0);
        let key = record(&compiled, &source, function, "10 mod x", 0);
        assert_eq!(
            settled(run(&source, &compiled, function, &key, &[(x, int(1))])),
            Settled::NoValue,
            "{function}"
        );
    }
    let x = parameter(&compiled, "us", 0);
    let key = record(&compiled, &source, "us", "10 mod x", 0);
    let refused = run(&source, &compiled, "us", &key, &[(x, int(0))]).unwrap_err();
    assert!(
        matches!(refused, ReplayRefusal::UnboundParameter(_)),
        "{refused:?}"
    );
}

/// FR-098 "Application claim: the observation", recursion: `Location` has no
/// frame, so a stop at the claim site inside a recursive call cannot be told
/// from the outer evaluation. At `x = 1` the outer `10 mod 1` is defined and
/// the nested `f(0)` stops at the same site: the claim holds at `x = 1`, so
/// replay must not reproduce a violation. It settles `NoValue`.
#[trace("TC-444", "FR-098-AC-22")]
#[test]
fn tc_444_a_stop_inside_a_recursive_call_is_not_the_outer_observation() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "f", 0);
    let key = record(&compiled, &source, "f", "10 mod x", 0);
    assert_eq!(
        settled(run(&source, &compiled, "f", &key, &[(x, int(1))])),
        Settled::NoValue
    );
}

/// The same hole through mutual recursion: `g` calls `h` calls `g`, and the
/// nested `g(0)` stops at `g`'s claim site. At `x = 2` the outer `10 mod 2`
/// is defined, so the claim holds and replay settles `NoValue`.
#[trace("TC-444", "FR-098-AC-22")]
#[test]
fn tc_444_a_stop_inside_a_mutually_recursive_call_is_not_the_outer_observation() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "g", 0);
    let key = record(&compiled, &source, "g", "10 mod x", 0);
    assert_eq!(
        settled(run(&source, &compiled, "g", &key, &[(x, int(2))])),
        Settled::NoValue
    );
}
