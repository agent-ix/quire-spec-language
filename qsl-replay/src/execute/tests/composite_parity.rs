// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-358 (TC-907): a composite equality-parity claim settles `Diverged`,
//! `Agrees` or refused when falsified, and by rows V-1 to V-5 when verified.

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, FiniteBoundKind, ProofBound};
use quire_exact::{Integer, LimitKind, ScalarLimits};
use quire_semantic_value::declaration::EqualityOperator;

use super::*;
use crate::composite::{
    CompositeParityClaim, CompositeParityReport, CompositeParityResult, EqualityOutcome,
    FalsifiedParity, IncompleteStage, NativeCause, NativeParityObservation, Refinement,
    VerifiedShadow, VerifiedShadowReport, VerifiedShadowResult,
};
use crate::execute::composite_parity::{identity_tie, parity_preimage};
use crate::execute::composite_site::locate;
use crate::execute::parity_identity::parity_obligation;
use crate::execute::scalar_site::{equality_operator, in_body};
use crate::execute::ParityBoundRefusal;
use crate::identity::DeclaredDomain;
use crate::scalar::{OperandIdentity, ScalarClaim};
use crate::witness::{WitnessField, WitnessSlot};
use qsl_foundation::source::provenance::OccurrenceKey;

const UNIT: &str = "record Q { x: Int[0, 9]; s: Sequence<Int[0, 9]>[0, 3]; }\n\
     function f using v(a: Q, b: Q): Boolean pure { a = b }\n\
     function g using v(a: Q): Boolean pure { a != Q { x: 1, s: sequence[] } }\n\
     record List { head: Int[0, 9]; tail: List?; }\n\
     function h using v(a: List, b: List): Boolean pure { a = b }\n\
     enum Color { Red, Green, Blue }\n\
     function c using v(a: Color, b: Color): Boolean pure { a = b }\n\
     record R { x: Int[0, 9]; label: Rational[0, 9; 1, 9]; }\n\
     function t using v(a: R, b: R): Boolean pure { a = b }\n\
     function d using v(a: Q, b: Q): Boolean pure { (a = b) and (a = b) }\n";

struct Unit {
    source: String,
    compiled: ComposedUnit,
}

fn unit() -> Unit {
    let source = format!("language \"ix:native\" edition \"1-draft\";\n{PROFILE}{UNIT}");
    let compiled = spine(&source, &BTreeMap::new());
    Unit { source, compiled }
}

fn int(value: i64) -> WitnessValue {
    WitnessValue::ExactInteger(Integer::from(value))
}

fn present(name: &str, value: WitnessValue) -> WitnessField {
    WitnessField {
        name: name.to_owned(),
        slot: WitnessSlot::Present(value),
    }
}

fn range(lower: i64, upper: i64) -> FiniteBound {
    FiniteBound::integer_range(Integer::from(lower), Integer::from(upper)).unwrap()
}

fn bound(node: WireNodeId, path: &[u32], bound: FiniteBound) -> ProofBound {
    let kind = match bound.kind() {
        FiniteBoundKind::Cardinality => Some(DomainKind::Collection),
        FiniteBoundKind::IntegerRange => Some(DomainKind::Integer),
        FiniteBoundKind::Depth => Some(DomainKind::Recursive),
        FiniteBoundKind::Variants => None,
    };
    let domain = DomainKey::Node {
        node,
        path: path.to_vec(),
    };
    ProofBound::new(domain, kind, bound).unwrap()
}

fn outcome(equal: bool, pair_count: u64) -> EqualityOutcome {
    EqualityOutcome { equal, pair_count }
}

impl Unit {
    fn parameter(&self, function: &str, index: usize) -> WireNodeId {
        parameter(&self.compiled, function, index)
    }

    /// The claimed equality node in `function`'s body.
    fn node(&self, function: &str) -> WireNodeId {
        let graph = self.compiled.package.graph();
        let identity = graph.callable(function).expect("declared").identity;
        let semantic = graph.semantic_graph();
        let found = semantic
            .nodes()
            .find(|node| {
                equality_operator(node).is_some() && in_body(semantic, identity, node.key())
            })
            .expect("the function applies an equality");
        WireNodeId::from_digest(*found.key().as_bytes())
    }

    /// The `index`th occurrence (in the package's own order) of `function`'s
    /// equality node in the function's body.
    fn occurrence_n(&self, function: &str, index: usize) -> quire_exact::Origin {
        let graph = self.compiled.package.graph();
        let identity = graph.callable(function).expect("declared").identity;
        let semantic = graph.semantic_graph();
        let key = semantic
            .nodes()
            .find(|node| {
                equality_operator(node).is_some() && in_body(semantic, identity, node.key())
            })
            .expect("the function applies an equality")
            .key();
        graph
            .occurrences()
            .filter(|(node, _, location)| {
                *node == key
                    && matches!(&location.origin,
                        quire_semantic_value::location::Origin::Body { function: name, .. }
                            if name == function)
            })
            .map(|(_, origin, _)| origin.clone())
            .nth(index)
            .expect("the occurrence exists")
    }

    fn record_id(&self, name: &str) -> WireNodeId {
        let types = self.compiled.package.graph().scope().types();
        let record = types
            .composites()
            .find(|declaration| declaration.name() == name)
            .expect("a declared record");
        WireNodeId::from_digest(*record.key().as_bytes())
    }

    /// `Q { x, s }`.
    fn q(&self, x: i64, s: &[i64]) -> WitnessValue {
        WitnessValue::Record {
            declaration: self.record_id("Q"),
            fields: vec![
                present("x", int(x)),
                present(
                    "s",
                    WitnessValue::Sequence(s.iter().map(|e| int(*e)).collect()),
                ),
            ],
        }
    }

    /// `B`: the covering harness bounds of `f`.
    fn covering(&self) -> Vec<ProofBound> {
        let mut bounds = Vec::new();
        for index in 0..2 {
            let parameter = self.parameter("f", index);
            bounds.push(bound(parameter, &[0], range(0, 9)));
            bounds.push(bound(parameter, &[1], FiniteBound::cardinality(3)));
            bounds.push(bound(parameter, &[1, 0], range(0, 9)));
        }
        bounds
    }

    fn claim(
        &self,
        function: &str,
        operator: EqualityOperator,
        harness_bounds: Vec<ProofBound>,
    ) -> CompositeParityClaim {
        self.claim_at(function, 0, operator, harness_bounds)
    }

    /// The claim at the `index`th occurrence of `function`'s equality node.
    fn claim_at(
        &self,
        function: &str,
        index: usize,
        operator: EqualityOperator,
        harness_bounds: Vec<ProofBound>,
    ) -> CompositeParityClaim {
        CompositeParityClaim {
            node: self.node(function),
            occurrence: self.occurrence_n(function, index),
            operator,
            obligation_kind: "bounded_shadow".to_owned(),
            harness_bounds,
            limits: UNLIMITED,
            content_identity: DigestRecord::mint(DigestDomain::Sha256Jcs, [9; 32]),
        }
    }

    /// The request of `function` whose `obligation_identity` is FR-358
    /// step 6's digest over `claim`, with the proving run's `domains`.
    fn wire(
        &self,
        function: &str,
        claim: &CompositeParityClaim,
        domains: Vec<DeclaredDomain>,
    ) -> ReplayRequestWire {
        let mut wire = request(
            self.source.as_bytes(),
            self.compiled.emitted.package_id(),
            name(&[function]),
            ReplaySource::Input(vec![]),
        );
        wire.declared_domains = domains;
        let decoded = ReplayRequest::decode(wire.clone(), ReplayLimits::default()).unwrap();
        let limits = request_limits(decoded.stage_limits(), ReplayLimits::default()).unwrap();
        let compiled = recompile(&decoded, &limits).unwrap();
        let site = locate(
            &compiled,
            decoded.selected_function(),
            claim.node,
            &claim.occurrence,
            claim.operator,
        )
        .expect("the claim names a node of the function");
        // A claim whose bounds are defective has no identity: the refusals
        // that name the defect come before the tie.
        wire.obligation_identity = parity_preimage(&compiled, &site, claim)
            .ok()
            .and_then(|preimage| parity_obligation(&preimage).ok())
            .map_or([0; 32], |identity| *identity.as_bytes());
        wire
    }
}

fn falsified(left: WitnessValue, right: WitnessValue, shadow: EqualityOutcome) -> FalsifiedParity {
    FalsifiedParity {
        operands: [left, right],
        shadow,
        native: NativeParityObservation::Completed(shadow),
        refinement: Refinement::Exhausted,
    }
}

fn verified(success_checks: u32, refinement: Refinement) -> VerifiedShadow {
    VerifiedShadow {
        success_checks,
        refinement,
    }
}

fn not_refuted(value: &TerminalValue) {
    assert_ne!(*value, TerminalValue::Refuted);
}

fn assert_failed(value: &TerminalValue) {
    assert_eq!(*value, TerminalValue::Failed);
    assert_eq!(value.category(), Category::InternalFailure);
}

fn assert_resource_exhausted(value: &TerminalValue) {
    assert_eq!(
        *value,
        TerminalValue::Incomplete(crate::proof_result::IncompleteCause::ResourceExhausted)
    );
    assert_eq!(value.category(), Category::Incomplete);
}

fn assert_refused_input(report: &CompositeParityReport, index: usize) {
    let CompositeParityResult::RefusedInput(refusal) = report.result() else {
        panic!("expected RefusedInput, got {:?}", report.result());
    };
    assert_eq!(refusal.index, index);
    assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    assert_eq!(
        report.terminal_value(),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::InvalidRuntimeInput))
    );
}

/// The settlement of a verified item over `bounds` at `function`.
fn settle(
    unit: &Unit,
    function: &str,
    bounds: Vec<ProofBound>,
    domains: Vec<DeclaredDomain>,
    evidence: VerifiedShadow,
) -> VerifiedShadowReport {
    let claim = unit.claim(function, EqualityOperator::Equal, bounds);
    let wire = unit.wire(function, &claim, domains);
    settle_verified_shadow(wire, claim, evidence, ReplayLimits::default())
}

fn declared(node: WireNodeId, path: &[u32], finite: FiniteBound) -> DeclaredDomain {
    DeclaredDomain::new(bound(node, path, finite))
}

/// FR-358-AC-1 (TC-907 step 1): at `E`, with operands that differ in one
/// element, the exact outcome is `{ equal: false }` over every pair; a
/// shadow that differs in the verdict or in the pair count settles
/// `Diverged`, and the exact outcome settles `Agrees`.
#[trace("TC-907", "FR-358-AC-1")]
#[test]
fn tc_907_a_falsified_item_settles_diverged_or_agrees() {
    let unit = unit();
    let claim = || unit.claim("f", EqualityOperator::Equal, unit.covering());
    let operands = || (unit.q(1, &[2]), unit.q(1, &[3]));
    for native in [
        None,
        Some(NativeParityObservation::Refused(Code::InvalidRuntimeInput)),
    ] {
        for shadow in [outcome(true, 4), outcome(false, 3)] {
            let (left, right) = operands();
            let mut evidence = falsified(left, right, shadow);
            if let Some(native) = native.clone() {
                evidence.native = native;
            }
            let report = replay_composite_parity(
                unit.wire("f", &claim(), vec![]),
                claim(),
                evidence,
                ReplayLimits::default(),
            );
            let CompositeParityResult::Diverged {
                exact,
                shadow: kept,
                ..
            } = report.result()
            else {
                panic!("expected Diverged, got {:?}", report.result());
            };
            assert_eq!(*exact, outcome(false, 4));
            assert_eq!(*kept, shadow);
            assert_failed(&report.terminal_value());
        }
        let (left, right) = operands();
        let mut evidence = falsified(left, right, outcome(false, 4));
        if let Some(native) = native.clone() {
            evidence.native = native;
        }
        let report = replay_composite_parity(
            unit.wire("f", &claim(), vec![]),
            claim(),
            evidence,
            ReplayLimits::default(),
        );
        let CompositeParityResult::Agrees { agreement, .. } = report.result() else {
            panic!("expected Agrees, got {:?}", report.result());
        };
        assert!(matches!(
            agreement.claim(),
            ScalarClaim::CompositeEquality(_)
        ));
        assert_eq!(
            *agreement.outcome(),
            ScalarOutcome::Equality(outcome(false, 4))
        );
        let terminal = report.terminal_value();
        assert_eq!(terminal.category(), Category::Inconclusive);
        assert!(matches!(
            terminal,
            TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(_))
        ));
        not_refuted(&terminal);
    }
    // At `N`, `a != Q { x: 1, s: sequence[] }`: the verdict is the negation of
    // `Equal`'s, over the same three pairs.
    let g = |shadow: EqualityOutcome| {
        let claim = unit.claim("g", EqualityOperator::NotEqual, vec![]);
        let wire = unit.wire("g", &claim, vec![]);
        replay_composite_parity(
            wire,
            claim,
            falsified(unit.q(2, &[]), unit.q(1, &[]), shadow),
            ReplayLimits::default(),
        )
    };
    assert!(matches!(
        g(outcome(true, 3)).result(),
        CompositeParityResult::Agrees { .. }
    ));
    assert!(matches!(
        g(outcome(false, 3)).result(),
        CompositeParityResult::Diverged { .. }
    ));
}

/// FR-358-AC-2 (TC-907 step 2): rows F-2 to F-6 settle in order, each with an
/// agreeing shadow.
#[trace("TC-907", "FR-358-AC-2")]
#[test]
fn tc_907_the_falsified_rows_settle_in_order() {
    let unit = unit();
    let claim = || unit.claim("f", EqualityOperator::Equal, unit.covering());
    let agreeing = outcome(false, 4);
    let run = |claim: CompositeParityClaim, evidence: FalsifiedParity| {
        let wire = unit.wire("f", &claim, vec![]);
        replay_composite_parity(wire, claim, evidence, ReplayLimits::default())
    };
    let two = || (unit.q(1, &[2]), unit.q(1, &[3]));
    let native_stop = |cause: &str| {
        [
            NativeParityObservation::Incomplete(NativeCause::new(cause.to_owned())),
            NativeParityObservation::ExecutionFault(NativeCause::new(cause.to_owned())),
        ]
    };
    let tight = || ScalarLimits {
        value_occurrences: 0,
        ..UNLIMITED
    };

    // F-2: a native stop settles `GeneratedFault` with no admission and no
    // exact evaluation, whatever the operands, the limits and the
    // refinement ceiling: an artifact fault is never hidden.
    for native in native_stop("the run stopped") {
        for (limits, refinement) in [
            (UNLIMITED, Refinement::Exhausted),
            (tight(), Refinement::CeilingReached),
        ] {
            let (left, right) = two();
            let mut evidence = falsified(left, right, agreeing);
            evidence.native = native.clone();
            evidence.refinement = refinement;
            let mut claim = claim();
            claim.limits = limits;
            let report = run(claim, evidence);
            let CompositeParityResult::GeneratedFault { native: kept } = report.result() else {
                panic!("expected GeneratedFault, got {:?}", report.result());
            };
            assert_eq!(*kept, native);
            assert_failed(&report.terminal_value());
        }
        // An operand that fails admission.
        let mut evidence = falsified(unit.q(12, &[2]), unit.q(1, &[3]), agreeing);
        evidence.native = native.clone();
        let report = run(claim(), evidence);
        assert!(matches!(
            report.result(),
            CompositeParityResult::GeneratedFault { .. }
        ));
        assert_failed(&report.terminal_value());
        // Accounting limits too small to admit the operands.
        let (left, right) = two();
        let mut evidence = falsified(left, right, agreeing);
        evidence.native = native.clone();
        let claim = claim();
        let mut wire = unit.wire("f", &claim, vec![]);
        wire.accounting_limits = tight();
        let report = replay_composite_parity(wire, claim, evidence, ReplayLimits::default());
        assert!(matches!(
            report.result(),
            CompositeParityResult::GeneratedFault { .. }
        ));
        assert_failed(&report.terminal_value());
    }

    // F-3: a right operand at `N` other than the package's literal, and a
    // left operand at `E` with `x: 12`, with F-3 before F-4 to F-7.
    let g_claim = unit.claim("g", EqualityOperator::NotEqual, vec![]);
    let wire = unit.wire("g", &g_claim, vec![]);
    let report = replay_composite_parity(
        wire,
        g_claim,
        falsified(unit.q(2, &[]), unit.q(2, &[]), outcome(true, 3)),
        ReplayLimits::default(),
    );
    assert_refused_input(&report, 1);
    let mut evidence = falsified(unit.q(12, &[2]), unit.q(1, &[3]), agreeing);
    evidence.refinement = Refinement::CeilingReached;
    assert_refused_input(&run(claim(), evidence), 0);

    // F-4: the request's accounting limits stop the admission: the stage is
    // `Admission`, never `ExactEvaluation`, before F-5 and F-6.
    for refinement in [Refinement::Exhausted, Refinement::CeilingReached] {
        let (left, right) = two();
        let mut evidence = falsified(left, right, agreeing);
        evidence.refinement = refinement;
        let claim = claim();
        let mut wire = unit.wire("f", &claim, vec![]);
        wire.accounting_limits = tight();
        let report = replay_composite_parity(wire, claim, evidence, ReplayLimits::default());
        let CompositeParityResult::Incomplete {
            stage: IncompleteStage::Admission(_),
        } = report.result()
        else {
            panic!("expected the admission stage, got {:?}", report.result());
        };
        assert_resource_exhausted(&report.terminal_value());
    }

    // F-5: limits too small for the exact evaluation, before F-6.
    for refinement in [Refinement::Exhausted, Refinement::CeilingReached] {
        let (left, right) = two();
        let mut evidence = falsified(left, right, agreeing);
        evidence.refinement = refinement;
        let mut claim = claim();
        claim.limits = tight();
        let report = run(claim, evidence);
        let CompositeParityResult::Incomplete {
            stage: IncompleteStage::ExactEvaluation(incomplete),
        } = report.result()
        else {
            panic!(
                "expected the exact-evaluation stage, got {:?}",
                report.result()
            );
        };
        assert_eq!(incomplete.limit_kind, LimitKind::ValueOccurrences);
        assert_resource_exhausted(&report.terminal_value());
    }

    // F-6: a refinement ceiling with admitted operands, a `Completed`
    // native and enough limits.
    let (left, right) = two();
    let mut evidence = falsified(left, right, agreeing);
    evidence.refinement = Refinement::CeilingReached;
    let report = run(claim(), evidence);
    assert!(matches!(
        report.result(),
        CompositeParityResult::Incomplete {
            stage: IncompleteStage::RefinementCeiling
        }
    ));
    assert_resource_exhausted(&report.terminal_value());
}

/// FR-358-AC-3 (TC-907 step 3): both entries refuse before any evaluation or
/// settlement row, carrying the obligation identity.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_the_common_steps_refuse_for_both_entries() {
    let unit = unit();
    let edited_source = unit.source.replace("[0, 3]; }", "[0, 4]; }");
    assert_ne!(edited_source, unit.source);

    type Refusal = fn(&ReplayRefusal) -> bool;
    let mut cases: Vec<(&str, ReplayRequestWire, CompositeParityClaim, Refusal)> = Vec::new();
    let claim = || unit.claim("f", EqualityOperator::Equal, unit.covering());

    let mut edited = request(
        edited_source.as_bytes(),
        unit.compiled.emitted.package_id(),
        name(&["f"]),
        ReplaySource::Input(vec![]),
    );
    edited.obligation_identity = unit.wire("f", &claim(), vec![]).obligation_identity;
    cases.push(("an edited source", edited, claim(), |refusal| {
        matches!(refusal, ReplayRefusal::PackageIdMismatch { .. })
    }));

    let mut undeclared = unit.wire("f", &claim(), vec![]);
    undeclared.selected_function = name(&["nope"]);
    cases.push(("an undeclared function", undeclared, claim(), |refusal| {
        matches!(refusal, ReplayRefusal::UnknownFunction { .. })
    }));

    let mut absent = claim();
    absent.node = WireNodeId::from_digest([7; 32]);
    cases.push((
        "a node the package does not hold",
        unit.wire("f", &claim(), vec![]),
        absent,
        |refusal| {
            matches!(refusal, ReplayRefusal::ScalarIdentity(cause)
                if matches!(**cause, crate::ScalarIdentityMismatch::Node { .. }))
        },
    ));

    // An identity over `[0, 9]` on the first `x` with the supplied harness
    // bound `[0, 8]`: both digests are named, and they differ.
    let mut narrower = claim();
    narrower.harness_bounds[0] = bound(unit.parameter("f", 0), &[0], range(0, 8));
    cases.push((
        "an identity over a different harness bound",
        unit.wire("f", &claim(), vec![]),
        narrower,
        |refusal| {
            matches!(refusal, ReplayRefusal::ScalarIdentity(cause)
                if matches!(&**cause, crate::ScalarIdentityMismatch::Obligation {
                    claimed, recomputed } if claimed != recomputed))
        },
    ));

    let mut elsewhere = claim();
    elsewhere.node = unit.node("h");
    cases.push((
        "a node outside the function's body",
        unit.wire("f", &claim(), vec![]),
        elsewhere,
        |refusal| {
            matches!(refusal, ReplayRefusal::ScalarIdentity(cause)
                if matches!(**cause, crate::ScalarIdentityMismatch::Function { .. }))
        },
    ));

    let mut negated = claim();
    negated.operator = EqualityOperator::NotEqual;
    cases.push((
        "the wrong operator",
        unit.wire("f", &claim(), vec![]),
        negated,
        |refusal| {
            matches!(refusal, ReplayRefusal::ScalarIdentity(cause)
                if matches!(**cause, crate::ScalarIdentityMismatch::Equality { .. }))
        },
    ));

    for (what, wire, claim, expected) in cases {
        let obligation = ObligationIdentity::from_digest(wire.obligation_identity);
        let operands = [unit.q(1, &[2]), unit.q(1, &[3])];
        let report = replay_composite_parity(
            wire.clone(),
            claim.clone(),
            FalsifiedParity {
                operands,
                shadow: outcome(false, 4),
                native: NativeParityObservation::Completed(outcome(false, 4)),
                refinement: Refinement::Exhausted,
            },
            ReplayLimits::default(),
        );
        let CompositeParityResult::Refused(refusal) = report.result() else {
            panic!("{what}: expected a refusal, got {:?}", report.result());
        };
        assert!(expected(refusal), "{what}: {refusal:?}");
        assert_eq!(report.obligation(), obligation, "{what}");
        assert_eq!(report.claim().obligation, obligation, "{what}");
        let verified = settle_verified_shadow(
            wire,
            claim,
            self::verified(0, Refinement::Exhausted),
            ReplayLimits::default(),
        );
        let VerifiedShadowResult::Refused(refusal) = verified.result() else {
            panic!("{what}: expected a refusal, got {:?}", verified.result());
        };
        assert!(expected(refusal), "{what}: {refusal:?}");
        assert_eq!(verified.obligation(), obligation, "{what}");
    }
}

/// FR-358-AC-4 (TC-907 step 4): a verified item whose harness covers every
/// derived position and whose refinement was exhaustive is `Proved`.
#[trace("TC-907", "FR-358-AC-4")]
#[test]
fn tc_907_a_covering_harness_proves() {
    let unit = unit();
    let proved = |report: &VerifiedShadowReport| {
        assert!(
            matches!(
                report.result(),
                VerifiedShadowResult::Proved { success_checks: 4 }
            ),
            "{:?}",
            report.result()
        );
        assert_eq!(
            report.terminal_value(),
            TerminalValue::Proved {
                basis: crate::ProofBasis::Checks { success_checks: 4 },
                certification: crate::Certification::Certified,
            }
        );
        assert_eq!(report.terminal_value().category(), Category::Success);
    };
    let exhausted = verified(4, Refinement::Exhausted);

    proved(&settle(&unit, "f", unit.covering(), vec![], exhausted));

    // Wider than declared still covers.
    let mut wider = unit.covering();
    wider[0] = bound(unit.parameter("f", 0), &[0], range(-1, 10));
    proved(&settle(&unit, "f", wider, vec![], exhausted));

    // At `N` the literal operand derives no position: `B`'s left half covers.
    let claim = unit.claim(
        "g",
        EqualityOperator::NotEqual,
        [
            bound(unit.parameter("g", 0), &[0], range(0, 9)),
            bound(unit.parameter("g", 0), &[1], FiniteBound::cardinality(3)),
            bound(unit.parameter("g", 0), &[1, 0], range(0, 9)),
        ]
        .to_vec(),
    );
    let wire = unit.wire("g", &claim, vec![]);
    proved(&settle_verified_shadow(
        wire,
        claim,
        exhausted,
        ReplayLimits::default(),
    ));

    // At `c`, the variants drawn are every variant the enum admits.
    let variants = |members: &[&str]| {
        FiniteBound::variants(members.iter().map(|member| (*member).to_owned())).unwrap()
    };
    let bounds = (0..2)
        .map(|index| {
            bound(
                unit.parameter("c", index),
                &[],
                variants(&["Blue", "Green", "Red"]),
            )
        })
        .collect();
    proved(&settle(&unit, "c", bounds, vec![], exhausted));
}

/// FR-358-AC-5 (TC-907 step 5): evidence short of covering the claim's
/// domain is `Tested`, never `Proved`.
#[trace("TC-907", "FR-358-AC-5")]
#[test]
fn tc_907_anything_short_of_covering_is_tested() {
    let unit = unit();
    let exhausted = verified(4, Refinement::Exhausted);
    let tested = |report: VerifiedShadowReport, what: &str| {
        assert!(
            matches!(report.result(), VerifiedShadowResult::Tested),
            "{what}: {:?}",
            report.result()
        );
        assert_eq!(report.terminal_value(), TerminalValue::Tested, "{what}");
        assert_eq!(
            report.terminal_value().category(),
            Category::Success,
            "{what}"
        );
    };
    let variants = |members: &[&str]| {
        FiniteBound::variants(members.iter().map(|member| (*member).to_owned())).unwrap()
    };

    tested(
        settle(
            &unit,
            "f",
            unit.covering(),
            vec![],
            verified(4, Refinement::NotExhausted),
        ),
        "a refinement that did not exhaust",
    );

    // One of `f`'s harness bounds replaced, or removed.
    let first = unit.parameter("f", 0);
    type Edit = Box<dyn Fn(&mut Vec<ProofBound>)>;
    let edits: [(&str, Edit); 3] = [
        (
            "cardinality 2 on one s",
            Box::new(move |bounds| bounds[1] = bound(first, &[1], FiniteBound::cardinality(2))),
        ),
        (
            "[0, 8] on one x",
            Box::new(move |bounds| bounds[0] = bound(first, &[0], range(0, 8))),
        ),
        (
            "no bound on one x",
            Box::new(|bounds| {
                bounds.remove(0);
            }),
        ),
    ];
    for (what, edit) in edits {
        let mut bounds = unit.covering();
        edit(&mut bounds);
        tested(settle(&unit, "f", bounds, vec![], exhausted), what);
    }
    tested(
        settle(&unit, "f", vec![], vec![], exhausted),
        "an empty harness list",
    );

    // A partial variant set does not cover.
    let bounds = vec![
        bound(unit.parameter("c", 0), &[], variants(&["Green", "Red"])),
        bound(
            unit.parameter("c", 1),
            &[],
            variants(&["Blue", "Green", "Red"]),
        ),
    ];
    tested(
        settle(&unit, "c", bounds, vec![], exhausted),
        "a partial variant set",
    );

    // A rational leaf (a text leaf needs a text-profile lock the spine test harness does not carry); its whole declared domain cannot be shown covered.
    let bounds = (0..2)
        .map(|index| bound(unit.parameter("t", index), &[0], range(0, 9)))
        .collect();
    tested(settle(&unit, "t", bounds, vec![], exhausted), "a text leaf");

    // A recursive position with no declared domain is never covered. (The
    // `K<T>` cases of AC-4 and AC-5 wait on a source form for an unbounded
    // collection type: the grammar takes only `K<T>[min, max]`.)
    let mut bounds = Vec::new();
    for index in 0..2 {
        let parameter = unit.parameter("h", index);
        bounds.push(bound(parameter, &[], FiniteBound::depth(3).unwrap()));
        bounds.push(bound(parameter, &[0], range(0, 9)));
    }
    tested(
        settle(&unit, "h", bounds, vec![], exhausted),
        "an undeclared depth",
    );
}

/// FR-358-AC-6 (TC-907 step 6): rows V-1 to V-3.
#[trace("TC-907", "FR-358-AC-6")]
#[test]
fn tc_907_refinement_and_a_vacuous_run_settle_before_coverage() {
    let unit = unit();
    for (checks, bounds) in [(4, unit.covering()), (0, vec![])] {
        let report = settle(
            &unit,
            "f",
            bounds,
            vec![],
            verified(checks, Refinement::Disagreed),
        );
        assert!(matches!(report.result(), VerifiedShadowResult::Disagreed));
        assert_failed(&report.terminal_value());
    }
    for checks in [4, 0] {
        let report = settle(
            &unit,
            "f",
            unit.covering(),
            vec![],
            verified(checks, Refinement::CeilingReached),
        );
        assert!(matches!(
            report.result(),
            VerifiedShadowResult::Incomplete {
                stage: IncompleteStage::RefinementCeiling
            }
        ));
        assert_resource_exhausted(&report.terminal_value());
    }
    let report = settle(
        &unit,
        "f",
        unit.covering(),
        vec![],
        verified(0, Refinement::Exhausted),
    );
    assert!(matches!(report.result(), VerifiedShadowResult::Vacuous));
    let terminal = report.terminal_value();
    assert_eq!(
        terminal,
        TerminalValue::Proved {
            basis: crate::ProofBasis::Checks { success_checks: 0 },
            certification: crate::Certification::Certified,
        }
    );
    assert_eq!(terminal.category(), Category::Inconclusive);
    assert_eq!(
        terminal.vacuous_proof_cause(),
        Some(crate::proof_result::ReportedInconclusiveCause::KaniVacuousProof)
    );
}

/// FR-358-AC-7 (TC-907 step 7): a defective bound refuses, naming its key,
/// ahead of any settlement row, V-3 included.
#[trace("TC-907", "FR-358-AC-7")]
#[test]
fn tc_907_a_defective_bound_refuses_before_any_row() {
    let unit = unit();
    let first = unit.parameter("f", 0);
    let variants = |members: &[&str]| {
        FiniteBound::variants(members.iter().map(|member| (*member).to_owned())).unwrap()
    };
    let refused = |report: VerifiedShadowReport| -> ParityBoundRefusal {
        let VerifiedShadowResult::Refused(refusal) = report.result() else {
            panic!("expected a refusal, got {:?}", report.result());
        };
        let ReplayRefusal::ParityBound(cause) = &**refusal else {
            panic!("expected a parity bound refusal, got {refusal:?}");
        };
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
        (**cause).clone()
    };
    let key = |node: WireNodeId, path: &[u32]| DomainKey::Node {
        node,
        path: path.to_vec(),
    };
    for checks in [4, 0] {
        let exhausted = verified(checks, Refinement::Exhausted);
        let with = |extra: ProofBound| {
            let mut bounds = unit.covering();
            bounds.push(extra);
            bounds
        };
        assert_eq!(
            refused(settle(
                &unit,
                "f",
                with(bound(first, &[7], range(0, 9))),
                vec![],
                exhausted
            )),
            ParityBoundRefusal::HarnessUnknownKey {
                key: key(first, &[7])
            }
        );
        assert_eq!(
            refused(settle(
                &unit,
                "f",
                with(bound(first, &[0], range(0, 9))),
                vec![],
                exhausted
            )),
            ParityBoundRefusal::HarnessDuplicate {
                key: key(first, &[0])
            }
        );
        let mut wrong_kind = unit.covering();
        wrong_kind[1] = bound(first, &[1], range(0, 9));
        assert_eq!(
            refused(settle(&unit, "f", wrong_kind, vec![], exhausted)),
            ParityBoundRefusal::HarnessKind {
                key: key(first, &[1]),
                expected: Some(FiniteBoundKind::Cardinality),
                supplied: FiniteBoundKind::IntegerRange,
            }
        );
        assert_eq!(
            refused(settle(
                &unit,
                "f",
                unit.covering(),
                vec![declared(first, &[0], range(0, 9))],
                exhausted
            )),
            ParityBoundRefusal::DeclaredOverAuthored {
                key: key(first, &[0])
            }
        );

        // At `c`.
        let color = unit.parameter("c", 0);
        let purple = vec![bound(color, &[], variants(&["Purple"]))];
        assert_eq!(
            refused(settle(&unit, "c", purple, vec![], exhausted)),
            ParityBoundRefusal::HarnessUnknownVariant {
                key: key(color, &[]),
                variant: "Purple".to_owned(),
            }
        );
        assert_eq!(
            refused(settle(
                &unit,
                "c",
                vec![],
                vec![declared(color, &[], FiniteBound::cardinality(2))],
                exhausted
            )),
            ParityBoundRefusal::DeclaredOverAuthored {
                key: key(color, &[])
            }
        );
    }
}

/// FR-358-AC-8 (TC-907 step 8): a refinement disagreement settles
/// `Disagreed` whatever else the item would give, after the common steps.
#[trace("TC-907", "FR-358-AC-8")]
#[test]
fn tc_907_a_refinement_disagreement_overrides_every_row() {
    let unit = unit();
    let claim = || unit.claim("f", EqualityOperator::Equal, unit.covering());
    let disagreed = |claim: CompositeParityClaim, mut evidence: FalsifiedParity| {
        evidence.refinement = Refinement::Disagreed;
        let wire = unit.wire("f", &claim, vec![]);
        replay_composite_parity(wire, claim, evidence, ReplayLimits::default())
    };
    let (left, right) = (unit.q(1, &[2]), unit.q(1, &[3]));
    let tight = ScalarLimits {
        value_occurrences: 0,
        ..UNLIMITED
    };

    let mut cases: Vec<(CompositeParityClaim, FalsifiedParity)> = vec![
        (
            claim(),
            falsified(left.clone(), right.clone(), outcome(true, 4)),
        ),
        (
            claim(),
            falsified(left.clone(), right.clone(), outcome(false, 4)),
        ),
        (
            claim(),
            falsified(unit.q(12, &[2]), right.clone(), outcome(false, 4)),
        ),
    ];
    let mut faulted = falsified(left.clone(), right.clone(), outcome(false, 4));
    faulted.native = NativeParityObservation::ExecutionFault(NativeCause::new("died".to_owned()));
    cases.push((claim(), faulted));
    let mut limited = claim();
    limited.limits = tight;
    cases.push((limited, falsified(left, right, outcome(false, 4))));
    for (claim, evidence) in cases {
        let report = disagreed(claim, evidence);
        assert!(matches!(report.result(), CompositeParityResult::Disagreed));
        assert_failed(&report.terminal_value());
        not_refuted(&report.terminal_value());
    }

    // A common-step refusal still refuses.
    let edited = unit.source.replace("[0, 3]; }", "[0, 4]; }");
    let mut wire = request(
        edited.as_bytes(),
        unit.compiled.emitted.package_id(),
        name(&["f"]),
        ReplaySource::Input(vec![]),
    );
    wire.obligation_identity = [3; 32];
    let mut evidence = falsified(unit.q(1, &[2]), unit.q(1, &[3]), outcome(false, 4));
    evidence.refinement = Refinement::Disagreed;
    let report = replay_composite_parity(wire, claim(), evidence, ReplayLimits::default());
    assert!(matches!(
        report.result(),
        CompositeParityResult::Refused(refusal)
            if matches!(**refusal, ReplayRefusal::PackageIdMismatch { .. })
    ));
}

/// FR-358-AC-4 and AC-5, the `k` cases: the grammar cannot author an unbounded
/// `K<T>`, so the derivation runs over a type built directly. An unbounded
/// collection is a `Cardinality` position no harness bound covers until a
/// request `DeclaredDomain` substitutes its maximum, and the element's range
/// is covered as authored.
#[trace("TC-907", "FR-358-AC-4")]
#[test]
fn tc_907_an_unbounded_collection_is_covered_only_by_a_declared_cardinality() {
    use crate::execute::composite_domain::derive;
    use quire_exact::{CollectionKind, CollectionType, IntegerInterval, ValueType};

    let unit = unit();
    let element =
        ValueType::Int(IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).unwrap());
    for kind in [
        CollectionKind::Sequence,
        CollectionKind::Set,
        CollectionKind::Bag,
        CollectionKind::OrderedSet,
    ] {
        let unbounded = ValueType::collection(CollectionType::new(kind, element.clone(), None));
        let parameter = WireNodeId::from_digest([5; 32]);
        let package = &unit.compiled.package;
        let positions = || derive(&[(parameter, &unbounded)], package, 1_000).unwrap();
        let harness = vec![
            bound(parameter, &[], FiniteBound::cardinality(5)),
            bound(parameter, &[0], range(0, 9)),
        ];

        // No declared domain: any finite harness bound is tighter than unbounded.
        let undeclared = positions();
        let drawn = undeclared.harness(&harness).unwrap();
        assert!(!undeclared.covered(&drawn));

        // A declared cardinality 5 is the claim's own domain, and the harness
        // reaches it; a harness of 4 does not.
        let mut declared_positions = positions();
        declared_positions
            .declare(&[declared(parameter, &[], FiniteBound::cardinality(5))])
            .unwrap();
        let drawn = declared_positions.harness(&harness).unwrap();
        assert!(declared_positions.covered(&drawn));
        let short = vec![
            bound(parameter, &[], FiniteBound::cardinality(4)),
            bound(parameter, &[0], range(0, 9)),
        ];
        let drawn = declared_positions.harness(&short).unwrap();
        assert!(!declared_positions.covered(&drawn));

        // The declared domain must be of the position's kind.
        let mut wrong = positions();
        assert!(wrong
            .declare(&[declared(parameter, &[], range(0, 9))])
            .is_err());
    }
}

/// The request, recompilation and site of `claim` at `function`.
fn prepared(
    unit: &Unit,
    function: &str,
    claim: &CompositeParityClaim,
) -> (
    ReplayRequest,
    Recompiled,
    crate::execute::composite_site::Site,
) {
    let wire = unit.wire(function, claim, vec![]);
    let request = ReplayRequest::decode(wire, ReplayLimits::default()).unwrap();
    let limits = request_limits(request.stage_limits(), ReplayLimits::default()).unwrap();
    let compiled = recompile(&request, &limits).unwrap();
    let site = locate(
        &compiled,
        request.selected_function(),
        claim.node,
        &claim.occurrence,
        claim.operator,
    )
    .unwrap();
    (request, compiled, site)
}

/// FR-358 step 6 (FR-358-AC-3): a tampered obligation digest refuses with the
/// typed mismatch on both entries, carrying the identity the request held.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_a_tampered_obligation_digest_refuses() {
    let unit = unit();
    let claim = unit.claim("f", EqualityOperator::Equal, unit.covering());
    let mut wire = unit.wire("f", &claim, vec![]);
    wire.obligation_identity[0] ^= 1;
    let tampered = ObligationIdentity::from_digest(wire.obligation_identity);
    let is_mismatch = |refusal: &ReplayRefusal| {
        matches!(refusal, ReplayRefusal::ScalarIdentity(cause)
            if matches!(**cause, crate::ScalarIdentityMismatch::Obligation { claimed, .. } if claimed == tampered))
    };
    let report = replay_composite_parity(
        wire.clone(),
        claim.clone(),
        falsified(unit.q(1, &[2]), unit.q(1, &[3]), outcome(false, 4)),
        ReplayLimits::default(),
    );
    let CompositeParityResult::Refused(refusal) = report.result() else {
        panic!("expected a refusal, got {:?}", report.result());
    };
    assert!(is_mismatch(refusal), "{refusal:?}");
    assert_eq!(report.obligation(), tampered);
    let verified = settle_verified_shadow(
        wire,
        claim,
        verified(4, Refinement::Exhausted),
        ReplayLimits::default(),
    );
    let VerifiedShadowResult::Refused(refusal) = verified.result() else {
        panic!("expected a refusal, got {:?}", verified.result());
    };
    assert!(is_mismatch(refusal), "{refusal:?}");
}

/// FR-358 step 6: a preimage the encoder refuses (an occurrence ordinal
/// beyond 2^53) refuses with the typed encoding cause, whatever identity was
/// claimed, the all-zero one included. The zero claim over a preimage that
/// encodes is a mismatch, so the zero identity matches nothing.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_a_preimage_the_encoder_refuses_never_yields_an_identity() {
    let unit = unit();
    let claim = unit.claim("f", EqualityOperator::Equal, unit.covering());
    let (request, compiled, mut site) = prepared(&unit, "f", &claim);
    let mut zero_wire = unit.wire("f", &claim, vec![]);
    zero_wire.obligation_identity = [0; 32];
    let zero = ReplayRequest::decode(zero_wire, ReplayLimits::default()).unwrap();
    assert!(matches!(
        identity_tie(&zero, &compiled, &site, &claim),
        Err(ReplayRefusal::ScalarIdentity(cause))
            if matches!(*cause, crate::ScalarIdentityMismatch::Obligation { .. })
    ));
    site.occurrence = OccurrenceKey::new(
        claim.node,
        quire_exact::Origin::new(quire_exact::Role::new("expression"), 1_u64 << 60),
    );
    assert!(matches!(
        identity_tie(&zero, &compiled, &site, &claim),
        Err(ReplayRefusal::ScalarIdentity(cause))
            if matches!(*cause, crate::ScalarIdentityMismatch::Encoding(
                crate::IdentityEncodeError::IntegerBeyondEncoder { magnitude }
            ) if magnitude == i128::from(1_u64 << 60))
    ));
    let _ = request;
}

/// FR-358 step 6: a parameter operand that is not the node's child at its
/// position refuses `OperandChild` before any digest is computed.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_an_operand_that_is_not_the_nodes_child_refuses() {
    let unit = unit();
    let claim = unit.claim("f", EqualityOperator::Equal, unit.covering());
    let (_, compiled, mut site) = prepared(&unit, "f", &claim);
    let other = unit.parameter("h", 0);
    let crate::execute::composite_site::Operand::Parameter { node, .. } = &mut site.operands[0]
    else {
        panic!("f's left operand is a parameter");
    };
    *node = other;
    assert!(matches!(
        parity_preimage(&compiled, &site, &claim),
        Err(ReplayRefusal::ScalarIdentity(cause))
            if matches!(*cause, crate::ScalarIdentityMismatch::OperandChild { position: 0, .. })
    ));
}

/// FR-358 step 6, the worked example of ADR-013 O-09 through the composite
/// arm: the argument the arm builds for a record parameter with a `Sequence`
/// field (the example's shape: cardinality 3 at its field and the element's
/// range below it) digests as the canonical text the ADR shows, written here
/// with this package's node ids. The ADR's own digest (4c36290f...659f) is the
/// same text over node id 11...11, which `parity_identity`'s test reproduces;
/// a package's node ids cannot be 11...11.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_the_composite_arm_builds_the_adr_worked_argument() {
    use crate::execute::parity_identity::argument_digest;
    let unit = unit();
    let claim = unit.claim("f", EqualityOperator::Equal, unit.covering());
    let (_, compiled, site) = prepared(&unit, "f", &claim);
    let preimage = parity_preimage(&compiled, &site, &claim).unwrap();
    let id = unit.parameter("f", 0);
    let range = r#"{"lower":"0","tag":"integer_range","upper":"9"}"#;
    let key = |path: &str| format!(r#"{{"node_id":"{id}","path":{path},"tag":"node"}}"#);
    let entry = |bound: &str, path: &str| format!(r#"{{"bound":{bound},"key":{}}}"#, key(path));
    let text = format!(
        r#"{{"domain":{{"entries":[{},{},{}],"tag":"bounds"}},"operand":{{"node_id":"{id}","tag":"graph_child"}},"position":0}}"#,
        entry(range, "[0]"),
        entry(range, "[1,0]"),
        entry(r#"{"maximum":"3","tag":"cardinality"}"#, "[1]"),
    );
    let digest =
        argument_digest(&preimage.arguments[0], claim.node, &preimage.occurrence, 0).unwrap();
    assert_eq!(
        digest,
        qsl_foundation::ByteDigest::of(text.as_bytes()).as_bytes()
    );
}

/// FR-358 step 2 (the occurrence key): `(a = b) and (a = b)` is one node with
/// two occurrences, and each settles under its own identity. An identity
/// minted for the other occurrence, an occurrence the node does not have in
/// the function and an occurrence of another function refuse.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_each_occurrence_of_a_shared_node_settles_under_its_own_identity() {
    let unit = unit();
    let bounds = || {
        let mut bounds = Vec::new();
        for index in 0..2 {
            let parameter = unit.parameter("d", index);
            bounds.push(bound(parameter, &[0], range(0, 9)));
            bounds.push(bound(parameter, &[1], FiniteBound::cardinality(3)));
            bounds.push(bound(parameter, &[1, 0], range(0, 9)));
        }
        bounds
    };
    let at = |index: usize| unit.claim_at("d", index, EqualityOperator::Equal, bounds());
    let evidence = || falsified(unit.q(1, &[2]), unit.q(1, &[3]), outcome(false, 4));
    let first = at(0);
    let second = at(1);
    assert_ne!(first.occurrence, second.occurrence);
    assert_eq!(first.node, second.node);
    let first_wire = unit.wire("d", &first, vec![]);
    let second_wire = unit.wire("d", &second, vec![]);
    assert_ne!(
        first_wire.obligation_identity,
        second_wire.obligation_identity
    );
    for (wire, claim) in [(first_wire.clone(), at(0)), (second_wire.clone(), at(1))] {
        let report = replay_composite_parity(
            wire.clone(),
            claim.clone(),
            evidence(),
            ReplayLimits::default(),
        );
        assert!(
            matches!(report.result(), CompositeParityResult::Agrees { .. }),
            "{:?}",
            report.result()
        );
        let verified = settle_verified_shadow(
            wire,
            claim,
            verified(4, Refinement::Exhausted),
            ReplayLimits::default(),
        );
        assert!(
            matches!(verified.result(), VerifiedShadowResult::Proved { .. }),
            "{:?}",
            verified.result()
        );
    }
    // Occurrence 1's claim under occurrence 0's identity.
    let report = replay_composite_parity(first_wire, at(1), evidence(), ReplayLimits::default());
    let CompositeParityResult::Refused(refusal) = report.result() else {
        panic!("expected a refusal, got {:?}", report.result());
    };
    assert!(matches!(&**refusal, ReplayRefusal::ScalarIdentity(cause)
        if matches!(**cause, crate::ScalarIdentityMismatch::Obligation { .. })));
    // An occurrence the node has in no body, and one it has in another
    // function.
    let mut absent = at(1);
    absent.occurrence = quire_exact::Origin::new(quire_exact::Role::new("expression"), 99);
    let mut elsewhere = at(1);
    elsewhere.occurrence = unit.occurrence_n("f", 0);
    for claim in [absent, elsewhere] {
        let report = replay_composite_parity(
            second_wire.clone(),
            claim,
            evidence(),
            ReplayLimits::default(),
        );
        let CompositeParityResult::Refused(refusal) = report.result() else {
            panic!("expected a refusal, got {:?}", report.result());
        };
        assert!(
            matches!(&**refusal, ReplayRefusal::ScalarIdentity(cause)
            if matches!(**cause, crate::ScalarIdentityMismatch::Occurrence { .. })),
            "{refusal:?}"
        );
    }
}

/// FR-358 (the full claim): every outcome of both entries carries the full
/// claim, the identity members and the observation, so
/// `report.claim() == CompositeIdentity::new(obligation, &claim, evidence)`
/// holds on every row and refusal, and a changed observation member breaks
/// it.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_every_outcome_carries_the_full_claim() {
    use crate::composite::{CompositeEvidence, CompositeIdentity};

    let unit = unit();
    let claim = || unit.claim("f", EqualityOperator::Equal, unit.covering());
    let agreeing = outcome(false, 4);
    let base = || falsified(unit.q(1, &[2]), unit.q(1, &[3]), agreeing);
    let tight = || ScalarLimits {
        value_occurrences: 0,
        ..UNLIMITED
    };
    type Scenario<'a> =
        Box<dyn Fn(&mut ReplayRequestWire, &mut CompositeParityClaim, &mut FalsifiedParity) + 'a>;
    let scenarios: Vec<(&str, Scenario<'_>)> = vec![
        (
            "F-1",
            Box::new(|_, _, e| e.refinement = Refinement::Disagreed),
        ),
        (
            "F-2",
            Box::new(|_, _, e| {
                e.native = NativeParityObservation::ExecutionFault(NativeCause::new("x".into()))
            }),
        ),
        ("F-3", Box::new(|_, _, e| e.operands[0] = unit.q(12, &[2]))),
        (
            "F-4",
            Box::new(move |w, _, _| w.accounting_limits = tight()),
        ),
        ("F-5", Box::new(move |_, c, _| c.limits = tight())),
        (
            "F-6",
            Box::new(|_, _, e| e.refinement = Refinement::CeilingReached),
        ),
        ("F-7 agrees", Box::new(|_, _, _| {})),
        (
            "F-7 diverged",
            Box::new(|_, _, e| e.shadow = outcome(true, 4)),
        ),
        (
            "refused",
            Box::new(|w, _, _| w.obligation_identity = [0; 32]),
        ),
    ];
    for (row, edit) in &scenarios {
        let mut claim = claim();
        let mut evidence = base();
        let mut wire = unit.wire("f", &claim, vec![]);
        edit(&mut wire, &mut claim, &mut evidence);
        let obligation = ObligationIdentity::from_digest(wire.obligation_identity);
        let report = replay_composite_parity(
            wire,
            claim.clone(),
            evidence.clone(),
            ReplayLimits::default(),
        );
        let full = |evidence: &FalsifiedParity| {
            CompositeIdentity::new(
                obligation,
                &claim,
                CompositeEvidence::Falsified(Box::new(evidence.clone())),
            )
        };
        assert_eq!(*report.claim(), full(&evidence), "{row}");
        let mut changed = vec![evidence.clone(); 4];
        changed[0].shadow.pair_count += 1;
        changed[1].native = NativeParityObservation::Refused(Code::InvalidRuntimeInput);
        changed[2].refinement = Refinement::NotExhausted;
        changed[3].operands[1] = unit.q(1, &[4]);
        for other in &changed {
            assert_ne!(*report.claim(), full(other), "{row}");
        }
    }

    let rows: Vec<(&str, Refinement, u32, bool)> = vec![
        ("V-1", Refinement::Disagreed, 4, false),
        ("V-2", Refinement::CeilingReached, 4, false),
        ("V-3", Refinement::Exhausted, 0, false),
        ("V-4", Refinement::Exhausted, 4, false),
        ("V-5", Refinement::NotExhausted, 4, false),
        ("refused", Refinement::Exhausted, 4, true),
    ];
    for (row, refinement, success_checks, refuse) in rows {
        let claim = claim();
        let mut wire = unit.wire("f", &claim, vec![]);
        if refuse {
            wire.obligation_identity = [0; 32];
        }
        let obligation = ObligationIdentity::from_digest(wire.obligation_identity);
        let evidence = verified(success_checks, refinement);
        let report = settle_verified_shadow(wire, claim.clone(), evidence, ReplayLimits::default());
        let full = |evidence: VerifiedShadow| {
            CompositeIdentity::new(obligation, &claim, CompositeEvidence::Verified(evidence))
        };
        assert_eq!(*report.claim(), full(evidence), "{row}");
        assert_ne!(
            *report.claim(),
            full(verified(success_checks + 1, refinement))
        );
        let other = if refinement == Refinement::Exhausted {
            Refinement::NotExhausted
        } else {
            Refinement::Exhausted
        };
        assert_ne!(*report.claim(), full(verified(success_checks, other)));
    }
}

/// ADR-013 O-09, a composite literal operand: the argument names the literal's
/// own graph node and ranges over exactly its value, so its domain is the
/// empty bounds. Written out as the canonical text and hashed here.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_a_composite_literal_operand_is_its_own_node_with_empty_bounds() {
    use crate::execute::parity_identity::argument_digest;
    let unit = unit();
    let claim = unit.claim("g", EqualityOperator::NotEqual, vec![]);
    let (_, compiled, site) = prepared(&unit, "g", &claim);
    let preimage = parity_preimage(&compiled, &site, &claim).unwrap();
    let OperandIdentity::GraphChild(literal) = preimage.arguments[1].identity else {
        panic!("a composite literal is a graph child");
    };
    assert_ne!(literal, unit.parameter("g", 0));
    let text = format!(
        r#"{{"domain":{{"entries":[],"tag":"bounds"}},"operand":{{"node_id":"{literal}","tag":"graph_child"}},"position":1}}"#
    );
    let digest =
        argument_digest(&preimage.arguments[1], claim.node, &preimage.occurrence, 1).unwrap();
    assert_eq!(
        digest,
        qsl_foundation::ByteDigest::of(text.as_bytes()).as_bytes()
    );
}

/// ADR-013 O-09, a collection operand with no declared maximum (a recursive
/// type's depth): the domain is the bounds the harness drew, so two drawn
/// depths are two obligations. Written out as the canonical text and hashed
/// here.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_an_unbounded_operand_is_identified_by_the_bounds_the_harness_drew() {
    use crate::execute::parity_identity::argument_digest;
    let unit = unit();
    let id = unit.parameter("h", 0);
    let drawn = |depth: u64| {
        vec![
            bound(id, &[], FiniteBound::depth(depth).unwrap()),
            bound(id, &[0], range(0, 9)),
        ]
    };
    let digest = |depth: u64| {
        let claim = unit.claim("h", EqualityOperator::Equal, drawn(depth));
        let (_, compiled, site) = prepared(&unit, "h", &claim);
        let preimage = parity_preimage(&compiled, &site, &claim).unwrap();
        argument_digest(&preimage.arguments[0], claim.node, &preimage.occurrence, 0).unwrap()
    };
    let text = |depth: u64| {
        format!(
            r#"{{"domain":{{"entries":[{{"bound":{{"lower":"0","tag":"integer_range","upper":"9"}},"key":{{"node_id":"{id}","path":[0],"tag":"node"}}}},{{"bound":{{"maximum":"{depth}","tag":"depth"}},"key":{{"node_id":"{id}","path":[],"tag":"node"}}}}],"tag":"bounds"}},"operand":{{"node_id":"{id}","tag":"graph_child"}},"position":0}}"#
        )
    };
    for depth in [3, 4] {
        assert_eq!(
            digest(depth),
            qsl_foundation::ByteDigest::of(text(depth).as_bytes()).as_bytes()
        );
    }
    assert_ne!(digest(3), digest(4));
}

/// FR-358 step 6, written independently of the code under test: the
/// canonical text of ADR-013 O-09's preimage is spelled out here and hashed
/// here, and it equals the identity the fixture's recompute gives, for a
/// parameter against a composite literal (`g`) and for two parameters over
/// the covering bounds (`f`).
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_the_identity_equals_a_hand_written_canonical_text() {
    let unit = unit();
    let hashed = |text: &str| qsl_foundation::ByteDigest::of(text.as_bytes()).as_bytes();
    let tail = |claim: &CompositeParityClaim| {
        format!(
            r#""node":"{}","obligation_kind":"bounded_shadow","occurrence_key":{{"ordinal":{},"role":"{}"}}"#,
            claim.node,
            claim.occurrence.ordinal(),
            claim.occurrence.role().as_str(),
        )
    };

    // `a != Q { x: 1, s: sequence[] }`: no harness bound on `a`, and the
    // literal is its own graph node with no free position.
    let claim = unit.claim("g", EqualityOperator::NotEqual, vec![]);
    let wire = unit.wire("g", &claim, vec![]);
    let (_, compiled, site) = prepared(&unit, "g", &claim);
    let literal = match parity_preimage(&compiled, &site, &claim).unwrap().arguments[1].identity {
        OperandIdentity::GraphChild(node) => node,
        OperandIdentity::InlineLiteral => panic!("a composite literal is a graph child"),
    };
    let a = unit.parameter("g", 0);
    let argument = |node: WireNodeId, position: u32, entries: &str| {
        format!(
            r#"{{"domain":{{"entries":[{entries}],"tag":"bounds"}},"operand":{{"node_id":"{node}","tag":"graph_child"}},"position":{position}}}"#
        )
    };
    let text = format!(
        r#"{{"arguments":[{},{}],{}}}"#,
        argument(a, 0, ""),
        argument(literal, 1, ""),
        tail(&claim),
    );
    assert_eq!(wire.obligation_identity, hashed(&text));

    // `a = b` over the covering bounds: each parameter's entries ascend by
    // the bytes of the encoded key, so `[1, 0]` precedes `[1]`.
    let claim = unit.claim("f", EqualityOperator::Equal, unit.covering());
    let wire = unit.wire("f", &claim, vec![]);
    let entries = |id: WireNodeId| {
        let key = |path: &str| format!(r#"{{"node_id":"{id}","path":{path},"tag":"node"}}"#);
        let range = r#"{"lower":"0","tag":"integer_range","upper":"9"}"#;
        format!(
            r#"{{"bound":{range},"key":{}}},{{"bound":{range},"key":{}}},{{"bound":{{"maximum":"3","tag":"cardinality"}},"key":{}}}"#,
            key("[0]"),
            key("[1,0]"),
            key("[1]"),
        )
    };
    let (a, b) = (unit.parameter("f", 0), unit.parameter("f", 1));
    let text = format!(
        r#"{{"arguments":[{},{}],{}}}"#,
        argument(a, 0, &entries(a)),
        argument(b, 1, &entries(b)),
        tail(&claim),
    );
    assert_eq!(wire.obligation_identity, hashed(&text));
}

/// `replay.input_bytes` is the caller's on both composite entries: an S1
/// limit above the default refuses `LimitAboveReader` under the default and
/// settles once the caller raises the bound.
#[trace("TC-907", "FR-358-AC-3")]
#[test]
fn tc_907_a_raised_replay_input_bound_admits_what_the_default_refuses() {
    let unit = unit();
    let claim = unit.claim("f", EqualityOperator::Equal, unit.covering());
    let mut wire = unit.wire("f", &claim, vec![]);
    let above = crate::DEFAULT_REPLAY_INPUT_BYTES * 2;
    wire.stage_limits.insert("s1.input_bytes".to_owned(), above);
    let raised = ReplayLimits::default().with_input_bytes(above);
    let evidence = || falsified(unit.q(1, &[2]), unit.q(1, &[3]), outcome(false, 4));
    let refused = |refusal: &ReplayRefusal| matches!(refusal, ReplayRefusal::LimitAboveReader(_));

    let report = replay_composite_parity(
        wire.clone(),
        claim.clone(),
        evidence(),
        ReplayLimits::default(),
    );
    assert!(matches!(report.result(),
        CompositeParityResult::Refused(refusal) if refused(refusal)));
    let report = replay_composite_parity(wire.clone(), claim.clone(), evidence(), raised);
    assert!(matches!(
        report.result(),
        CompositeParityResult::Agrees { .. }
    ));

    let proved = verified(4, Refinement::Exhausted);
    let report =
        settle_verified_shadow(wire.clone(), claim.clone(), proved, ReplayLimits::default());
    assert!(matches!(report.result(),
        VerifiedShadowResult::Refused(refusal) if refused(refusal)));
    let report = settle_verified_shadow(wire, claim, proved, raised);
    assert!(matches!(
        report.result(),
        VerifiedShadowResult::Proved { .. }
    ));
}
