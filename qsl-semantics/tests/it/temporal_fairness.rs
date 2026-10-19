// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-123 (TC-518): the `TemporalTrace` family's check of fairness
//! constraints, interval operators and property forms, over TC-458's
//! `ConfigVersion` fixture package (`crate::model_operations`).
//!
//! The family check does not type `holds` conditions (the package check
//! does, FR-337), so the atoms read `c.versionNumber` and `c.healthy` as
//! text only here; [`node_identity`] runs the whole package check.

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind, TemporalInterval, TemporalUpper};
use qsl_foundation::diagnostic::{Code, StageFailure};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::Span;
use qsl_semantics::check::{
    check_clauses, Capability, CheckCause, CheckRefusal, CheckedTemporalClause, TemporalCause,
    TemporalProfile, TemporalTraceFamily,
};
use qsl_semantics::family::{ClaimExtent, FamilyContract};
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, PropertyForm,
};
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_profiles, ambiguous_operation_document,
    attempt_update_modifies_version_number, config_unit_with_profiles, config_version_document,
};

const EVENT: &str = "quire.temporal.event-position.false-extension/v1";
const FIXED: &str = "quire.temporal.fixed-sample.false-extension/v1";
const INFINITE: &str = "quire.temporal.infinite-trace/v1";

const CONFIG_VERSION: &str = "Config::ConfigVersion";
const ATTEMPT_UPDATE: &str = "Config::ConfigVersion::attemptUpdate";

type Outcome = Result<CheckedTemporalClause, StageFailure<CheckRefusal>>;

/// The `ConfigVersion` document, with its one `attemptUpdate` operation.
fn config_version() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

/// The unit's header profile `t` selecting the temporal profile `profile`.
fn profile_header(profile: &str) -> String {
    format!("profile t = \"{profile}\";\n")
}

/// The unit's one clause `C` over `c: over` whose body is `body`.
fn clause_body(over: &str, body: &str) -> String {
    format!("temporal C using t over (c: {over}) clock \"model-steps\" on origin {{ {body} }}")
}

/// The unit text and the clause checked from `document`.
fn check_in(document: &[u8], profile: &str, over: &str, body: &str) -> (String, Outcome) {
    let (profiles, declarations) = (profile_header(profile), clause_body(over, body));
    let package = admit_and_assemble_with_profiles(document, &profiles, &declarations)
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"));
    let (text, _) = config_unit_with_profiles(document, &profiles, &declarations);
    let mut checked = check_clauses(&package);
    let Some(outcome) = checked.pop() else {
        panic!("{body}: one clause");
    };
    assert!(checked.is_empty(), "{body}: one clause");
    (text, outcome)
}

/// The node identity of the one clause `C` with `body`, under the
/// infinite-trace profile, through the whole package check (FR-337).
fn node_identity(body: &str) -> quire_exact::NodeKey {
    let package = admit_and_assemble_with_profiles(
        &config_version(),
        &profile_header(INFINITE),
        &clause_body(CONFIG_VERSION, body),
    )
    .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"));
    let graph = package
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{body}: the package checks: {refusals:?}"));
    let [clause] = graph.temporal_clauses() else {
        panic!("{body}: one emitted clause");
    };
    clause.identity()
}

fn checked(profile: &str, body: &str) -> (String, CheckedTemporalClause) {
    match check_in(&config_version(), profile, CONFIG_VERSION, body) {
        (text, Ok(clause)) => (text, clause),
        (_, Err(failure)) => panic!("{body}: the clause checks, not {failure:?}"),
    }
}

/// The temporal cause `body` refuses with, and the unit text.
fn refused(profile: &str, body: &str) -> (String, TemporalCause) {
    match check_in(&config_version(), profile, CONFIG_VERSION, body) {
        (
            text,
            Err(StageFailure::Refused(CheckRefusal {
                cause: CheckCause::Temporal(cause),
                ..
            })),
        ) => (text, *cause),
        (_, other) => panic!("{body}: a temporal refusal, not {other:?}"),
    }
}

/// The span of the first occurrence of `needle` in `text`.
fn first(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

fn closed(lower: u64, upper: u64) -> Option<TemporalInterval> {
    Some(TemporalInterval::new(lower, TemporalUpper::Finite(upper)).expect("a <= b"))
}

fn only_constraint(clause: &CheckedTemporalClause) -> &FairnessConstraint {
    let [constraint] = clause.fairness() else {
        panic!("one constraint: {:?}", clause.fairness());
    };
    constraint
}

const EVENTUALLY_TWO: &str = "always eventually holds(c.versionNumber = 2)";

/// FR-123-AC-1: a weak constraint written with no granularity and one
/// written `whole` check to equal sets with granularity `Whole` and give the
/// clause one node identity; one written `each` checks to `Each` and gives
/// it a different node identity; the operation is read as
/// `Model::Type::op` or, written alone, in the view of the clause's `over`
/// type, and resolves to the declaring type's identity.
#[trace("TC-518", "FR-123-AC-1")]
#[test]
fn an_unmarked_granularity_checks_as_whole_and_each_differs() {
    let fair = |written: &str| {
        checked(
            INFINITE,
            &format!("fair {written}{ATTEMPT_UPDATE}; {EVENTUALLY_TWO}"),
        )
        .1
    };
    let unmarked = fair("weak ");
    let whole = fair("weak whole ");
    let each = fair("weak each ");
    assert_eq!(only_constraint(&unmarked).kind(), FairnessKind::Weak);
    assert_eq!(
        only_constraint(&unmarked).granularity(),
        FairnessGranularity::Whole
    );
    assert_eq!(unmarked.fairness(), whole.fairness());
    assert_eq!(
        only_constraint(&each).granularity(),
        FairnessGranularity::Each
    );
    assert_ne!(whole.fairness(), each.fairness());
    assert_eq!(only_constraint(&each).operation().name(), "attemptUpdate");

    let alone = checked(
        INFINITE,
        &format!("fair weak attemptUpdate; {EVENTUALLY_TWO}"),
    )
    .1;
    assert_eq!(alone.fairness(), unmarked.fairness());

    let identity =
        |written: &str| node_identity(&format!("fair {written}{ATTEMPT_UPDATE}; {EVENTUALLY_TWO}"));
    assert_eq!(identity("weak "), identity("weak whole "));
    assert_ne!(identity("weak whole "), identity("weak each "));
}

/// FR-123-AC-1: same-named operations declared by two distinct model types
/// check to distinct constraints with distinct declaring-type identities; an
/// operation both of two unrelated supertypes declare is ambiguous.
#[trace("TC-518", "FR-123-AC-1")]
#[test]
fn same_named_operations_of_different_types_stay_distinct() {
    let document = ambiguous_operation_document();
    let fair = |object: &str| {
        let body = format!("fair weak Config::{object}::dup; {EVENTUALLY_TWO}");
        match check_in(&document, INFINITE, &format!("Config::{object}"), &body) {
            (_, Ok(clause)) => clause,
            (_, Err(failure)) => panic!("{object}: the clause checks, not {failure:?}"),
        }
    };
    let (left, right) = (fair("Left"), fair("Right"));
    let (left, right) = (only_constraint(&left), only_constraint(&right));
    assert_eq!(left.operation().name(), right.operation().name());
    assert_ne!(
        left.operation().declaring_type(),
        right.operation().declaring_type()
    );
    assert_ne!(left, right);

    let body = format!("fair weak Config::Both::dup; {EVENTUALLY_TWO}");
    match check_in(&document, INFINITE, "Config::Both", &body) {
        (
            text,
            Err(StageFailure::Refused(CheckRefusal {
                cause: CheckCause::Temporal(cause),
                ..
            })),
        ) => {
            assert_eq!(
                *cause,
                TemporalCause::AmbiguousFairnessOperation {
                    name: "Config::Both::dup".into(),
                    span: first(&text, "Config::Both::dup"),
                }
            );
            assert_eq!(cause.code(), Code::AmbiguousDeclaration);
            assert_eq!(cause.tag(), "ambiguous-name");
        }
        (_, other) => panic!("an ambiguous operation, not {other:?}"),
    }
}

/// FR-123-AC-2: a constraint naming no operation refuses
/// `missing_declaration`/`missing-name` at the name; a constraint under the
/// event-position profile refuses `unsupported_construct`/`expression-form`
/// at the constraint; a constraint written twice checks to one member.
#[trace("TC-518", "FR-123-AC-2")]
#[test]
fn an_unresolved_or_misplaced_constraint_refuses_and_a_repeated_one_joins_once() {
    let (text, cause) = refused(INFINITE, &format!("fair weak Absent; {EVENTUALLY_TWO}"));
    assert_eq!(
        cause,
        TemporalCause::UnresolvedFairnessOperation {
            name: "Absent".into(),
            span: first(&text, "Absent")
        }
    );
    assert_eq!(cause.code(), Code::MissingDeclaration);
    assert_eq!(cause.tag(), "missing-name");

    let (text, cause) = refused(
        EVENT,
        &format!("fair weak {ATTEMPT_UPDATE}; eventually[0,5] holds(c.versionNumber = 2)"),
    );
    assert_eq!(
        cause,
        TemporalCause::FairnessUnderBoundedProfile {
            span: first(&text, &format!("fair weak {ATTEMPT_UPDATE};"))
        }
    );
    assert_eq!(cause.code(), Code::UnsupportedConstruct);
    assert_eq!(cause.tag(), "expression-form");

    let twice = checked(
        INFINITE,
        &format!("fair weak {ATTEMPT_UPDATE}; fair weak whole {ATTEMPT_UPDATE}; {EVENTUALLY_TWO}"),
    )
    .1;
    assert_eq!(twice.fairness().len(), 1);
}

const RECOVERY: &str =
    "always (holds(not c.healthy) implies eventually always[0,2] holds(c.healthy))";

/// FR-123-AC-3: under infinite-trace the recovery-stability formula checks
/// with `Some([0,2])` on the inner `always` and `None` on every other
/// operator; `eventually[3,*]` checks with `Some([3,Open])`; the same
/// operator under event-position and `eventually[5,3]` refuse at the
/// interval; the `[0,2]` interval's key differs from the event-position one.
#[trace("TC-518", "FR-123-AC-3")]
#[test]
fn intervals_nest_under_unbounded_operators_and_key_by_profile() {
    let (_, recovery) = checked(INFINITE, RECOVERY);
    let intervals: Vec<_> = recovery
        .operators()
        .iter()
        .map(|operator| operator.interval)
        .collect();
    assert_eq!(intervals, [None, None, None, closed(0, 2)]);

    let (_, open) = checked(INFINITE, "eventually[3,*] holds(c.versionNumber = 2)");
    assert_eq!(
        open.operators()
            .iter()
            .map(|op| op.interval)
            .collect::<Vec<_>>(),
        [Some(
            TemporalInterval::new(3, TemporalUpper::Open).expect("a <= b")
        )]
    );

    let (text, cause) = refused(EVENT, "eventually[3,*] holds(c.versionNumber = 2)");
    assert_eq!(
        cause,
        TemporalCause::OpenInterval {
            span: first(&text, "[3,*]")
        }
    );
    let (text, cause) = refused(INFINITE, "eventually[5,3] holds(c.versionNumber = 2)");
    assert_eq!(
        cause,
        TemporalCause::InvertedInterval {
            lower: 5,
            upper: 3,
            span: first(&text, "[5,3]")
        }
    );

    let key_of = |profile: &str| {
        let (_, clause) = checked(profile, "always[0,2] holds(c.versionNumber = 2)");
        let [operator] = clause.operators() else {
            panic!("one operator");
        };
        clause
            .interval_key(operator.node)
            .expect("a finite interval has a key")
    };
    assert_ne!(key_of(INFINITE), key_of(EVENT));
}

fn form_of(profile: &str, body: &str) -> (CheckedTemporalClause, Option<PropertyForm>) {
    let (_, clause) = checked(profile, body);
    let form = clause.property_form();
    (clause, form)
}

/// FR-123-AC-4: the five clauses classify as `ReachableInvariant`,
/// `BoundedMltl`, `Safety`, `Liveness` and `Liveness`; each infinite-trace
/// clause records (`temporal-satisfaction`, `Unbounded`); a clause over the
/// subject under the fixed-sample profile records no property form and keeps
/// its profile.
#[trace("TC-518", "FR-123-AC-4")]
#[test]
fn clauses_over_a_model_subject_classify_into_their_property_form() {
    let unbounded = |clause: &CheckedTemporalClause| {
        let [claim] = TemporalTraceFamily::requirements(clause)
            .try_into()
            .expect("one claim");
        let record = claim
            .requirements(
                WireNodeId::from_digest([1; 32]),
                &[WireNodeId::from_digest([2; 32])],
                // The population's key is FR-336's test; this one reads the
                // formula's domain.
                claim.population().map(|_| DomainKey::Population {
                    member_type: WireNodeId::from_digest([3; 32]),
                    ordinal: 0,
                }),
            )
            .expect("the record is keyed");
        assert_eq!(record.kind(), Capability::TemporalSatisfaction);
        let ClaimExtent::Unbounded(domains) = record.extent() else {
            panic!("unbounded: {:?}", record.extent());
        };
        assert!(domains
            .iter()
            .any(|(_, kind)| kind == DomainKind::InfiniteTrace));
    };
    let infinite = [
        (
            "always holds(c.versionNumber <= 1000)",
            PropertyForm::ReachableInvariant,
        ),
        (
            "always (holds(req) implies eventually[0,5] holds(ack))",
            PropertyForm::Safety,
        ),
        (EVENTUALLY_TWO, PropertyForm::Liveness),
        (RECOVERY, PropertyForm::Liveness),
        // Negation normal form: a negated `eventually` is an `always`, and
        // an `eventually[a,*]` is outside the fragment as `eventually` is.
        (
            "not eventually holds(c.versionNumber = 2)",
            PropertyForm::Safety,
        ),
        (
            "eventually[3,*] holds(c.versionNumber = 2)",
            PropertyForm::Liveness,
        ),
        (
            "always[3,*] holds(c.versionNumber = 2)",
            PropertyForm::Safety,
        ),
    ];
    for (body, expected) in infinite {
        let (clause, form) = form_of(INFINITE, body);
        assert_eq!(form, Some(expected), "{body}");
        unbounded(&clause);
    }

    let (_, form) = form_of(EVENT, "eventually[0,5] holds(c.versionNumber = 2)");
    assert_eq!(form, Some(PropertyForm::BoundedMltl));

    let (clause, form) = form_of(FIXED, "eventually[0,5] holds(c.versionNumber = 2)");
    assert_eq!(form, None);
    assert_eq!(clause.profile(), TemporalProfile::FixedSampleFalseExtension);
}
