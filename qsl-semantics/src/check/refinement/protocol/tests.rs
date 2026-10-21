//! FR-136 "Selecting the concrete protocol", "Coverage over a protocol
//! subject" and "Order of the checks", and FR-214 (TC-541, TC-659): the S3
//! check of a refinement whose step rows select a protocol clause, over
//! fixture R of the protocol obligation tests as the concrete model and a
//! one-operation model `A` as the abstract one.
//!
//! Every assertion is on the refusal's typed cause and the text of its span.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;

use qsl_foundation::Span;

use crate::check::{
    observation, step_row_refs, step_simulation_records, CheckCause, CheckRefusal, CheckedGraph,
    CheckedRefinement, LabelStep, MemoryStepKind, PackageDeclarations, ProtocolStepKind, RefinementCause,
    RefinementRefusal, RowClass, RowObservation, StepClass, StepLabel, StepRowRefusal,
    UnsupportedFeature,
};
use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    OperationParameterRecord, PopulationRecord, ValueTypeRef,
};
use crate::model::intake::SelectedModel;
use crate::model::key::DeclarationKey;
use crate::model::normalize::{normalize, NormalizeOutcome};

#[allow(dead_code)]
mod fixture {
    use crate as semantics;
    include!("../../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model_with, HEADER};

/// Fixture R with the population `nodes` of its `Node`s.
fn concrete_model() -> SelectedModel {
    obligation_model_with(vec![DomainPackageRecord::Population(PopulationRecord {
        key: DeclarationKey {
            package: "example/protocol-obligations".to_owned(),
            node: "ix://example/protocol-obligations/nodes".to_owned(),
        },
        member_types: vec![DeclarationKey {
            package: "example/protocol-obligations".to_owned(),
            node: "ix://example/protocol-obligations/Node".to_owned(),
        }],
        extent: Extent::Closed,
        symmetric: false,
    })])
}

const ABSTRACT: &str = "example/protocol-abstract";

fn abstract_key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: ABSTRACT.to_owned(),
        node: format!("ix://{ABSTRACT}/{name}"),
    }
}

/// The abstract model `A`: an object type `Acc` with the field `total` and
/// the operations `add()` and `addBy(n)`, which modify their receiver's
/// `total`.
fn abstract_model() -> SelectedModel {
    let records = vec![
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: abstract_key("Acc"),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        }),
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: abstract_key("Acc/total"),
            owner: abstract_key("Acc"),
            value_type: ValueTypeRef::Native(NativeValueType::Integer),
            multiplicity: Multiplicity {
                lower: 1,
                upper: Some(1),
                ordered: false,
                unique: true,
            },
            presence: quire_exact::Presence::Required,
            subsets: Vec::new(),
            redefines: None,
        }),
    ];
    let operation = |name: &str, parameters: Vec<OperationParameterRecord>| {
        DomainPackageRecord::OperationMember(OperationMemberRecord {
            key: abstract_key(&format!("Acc/{name}")),
            owner: abstract_key("Acc"),
            parameters,
            result: None,
            effect: OperationEffect {
                receiver_modifies: vec![abstract_key("Acc/total")],
                ..OperationEffect::default()
            },
            requires: Vec::new(),
            ensures: Vec::new(),
            has_body: false,
            redefines: None,
        })
    };
    let amount = OperationParameterRecord {
        key: abstract_key("Acc/addBy/n"),
        value_type: ValueTypeRef::Native(NativeValueType::Integer),
        multiplicity: Multiplicity {
            lower: 1,
            upper: Some(1),
            ordered: false,
            unique: true,
        },
    };
    let records: Vec<DomainPackageRecord> = records
        .into_iter()
        .chain([
            operation("add", Vec::new()),
            operation("addBy", vec![amount]),
            DomainPackageRecord::Population(PopulationRecord {
                key: abstract_key("accs"),
                member_types: vec![abstract_key("Acc")],
                extent: Extent::Closed,
                symmetric: false,
            }),
        ])
        .collect();
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: ABSTRACT.to_owned(),
            version: "1".to_owned(),
            digest: [0_u8; 32],
        },
        records,
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the abstract fixture normalizes");
    };
    SelectedModel {
        alias: "A".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
        view,
        operation_clauses: Vec::new(),
        type_invariants: Vec::new(),
    }
}

/// The unit header: fixture R as `M`, the abstract model as `A`, and the
/// protocol profile when `protocol_profile`.
fn header(protocol_profile: bool) -> String {
    let profile = if protocol_profile {
        "profile pp = \"quire.protocol.finite-global/v1\";\n"
    } else {
        ""
    };
    HEADER.replacen("model M", &format!("{profile}model M"), 1)
        + &format!(
            "model A = \"{ABSTRACT}\" version \"1\" digest \
             \"sha256-jcs:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd\";\n"
        )
}

/// A refusal with the text of the unit it is located at.
#[derive(Debug)]
struct Refused {
    refusal: CheckRefusal,
    at: String,
}

impl Refused {
    fn refinement(&self) -> &RefinementCause {
        let CheckCause::Refinement(refusal) = &self.refusal.cause else {
            panic!("a refinement refusal: {:?}", self.refusal);
        };
        &refusal.cause
    }

    fn refinement_refusal(&self) -> &RefinementRefusal {
        let CheckCause::Refinement(refusal) = &self.refusal.cause else {
            panic!("a refinement refusal: {:?}", self.refusal);
        };
        refusal
    }

    fn pair(&self) -> (Code, Option<&'static str>) {
        (self.refusal.cause.code(), self.refusal.cause.cause())
    }
}

fn check_text(text: &str) -> Result<CheckedGraph, Vec<Refused>> {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![concrete_model(), abstract_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"))
    .with_unwritten_refinements();
    let regions = assembled.regions();
    assembled
        .check(CheckingLimits::default())
        .map_err(|refusals| {
            refusals
                .into_iter()
                .map(|refusal| {
                    let region = regions
                        .refusal_region(&refusal)
                        .unwrap_or_else(|| panic!("{refusal:?} names a region"));
                    let (start, end) = (
                        usize::try_from(region.start()).expect("a start"),
                        usize::try_from(region.end()).expect("an end"),
                    );
                    Refused {
                        at: text[start..end].to_owned(),
                        refusal,
                    }
                })
                .collect()
        })
}

fn checks(text: &str) -> CheckedGraph {
    check_text(text).unwrap_or_else(|refusals| panic!("{text}: {refusals:?}"))
}

fn refused(text: &str) -> Vec<Refused> {
    match check_text(text) {
        Ok(_) => panic!("{text}: expected a refusal"),
        Err(refusals) => refusals,
    }
}

fn only(found: &[Refused]) -> &Refused {
    let [only] = found else {
        panic!("one refusal: {found:?}");
    };
    only
}

/// ADR-027 §7's `Fill` over fixture R: `step` and `undo` in the two branches
/// of the `parallel` `Both`, and the `finish` node `Done`. `parallel` is the
/// text between the name `Both` and its brace.
fn fill(memory: &str) -> String {
    format!(
        "protocol Fill using v over (k: M::Node) on origin {{\n\
         role w on M::Node;\n\
         run sequence Main {{\n\
         parallel Both {memory} {{\n\
         branch left attempt A by w on M::Node::step contracts [] \
         as (a: M::Flag) {{ true }};\n\
         branch right attempt B by w on M::Node::undo contracts [] \
         as (b: M::Flag) {{ true }};\n\
         }} join all [left, right];\n\
         }}\n\
         finish Done as (d: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

/// The refinement of `A::Acc` by `Fill` that ADR-027 §7 gives (FR-214's
/// first example), with `rows` as its step rows.
fn refinement(rows: &str) -> String {
    named_refinement("FillRefinesAcc", rows)
}

fn named_refinement(name: &str, rows: &str) -> String {
    format!(
        "refinement {name} using t abstract A concrete M {{\n\
         population A::accs from M::nodes;\n\
         object A::Acc from M::Node {{ total = self.total; }}\n\
         {rows}\n\
         }}\n"
    )
}

/// The rows of FR-214-AC-1.
const ROWS: &str = "step M::Node::step -> A::Acc::add(self);\n\
    step M::Node::undo -> A::Acc::add(self);\n\
    step fork M::Fill::Both -> stutter;\n\
    step join M::Fill::Both -> stutter;\n\
    step M::Fill::Done -> stutter;";

fn unit(protocol: &str, rows: &str) -> String {
    header(true) + protocol + &refinement(rows)
}

fn only_refinement(graph: &CheckedGraph) -> &CheckedRefinement {
    let [refinement] = graph.refinements() else {
        panic!("one refinement: {:?}", graph.refinements());
    };
    refinement
}

fn missing(class: RowClass, name: &str) -> RefinementCause {
    RefinementCause::MissingRow {
        class,
        name: name.to_owned(),
    }
}

/// The text of the unit that `span` covers.
fn text_of(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// `Fill`'s refinement checks, every step class of `Fill` has exactly one row
/// and the attempts are visible while the control steps are internal.
#[trace("TC-659", "FR-214-AC-1")]
#[test]
fn a_protocol_refinement_has_one_row_per_step_class() {
    let graph = checks(&unit(&fill(""), ROWS));
    let refinement = only_refinement(&graph);
    let classes: Vec<&StepClass> = refinement.steps().iter().map(|row| &row.class).collect();
    let node = |name: &str| {
        let protocol = graph.protocol("Fill").expect("the checked Fill");
        protocol
            .controls
            .controls
            .iter()
            .find(|control| control.name == name)
            .map(|control| control.node)
            .or_else(|| {
                protocol
                    .finish
                    .as_ref()
                    .filter(|(_, finish)| finish == name)
                    .map(|(node, _)| *node)
            })
            .unwrap_or_else(|| panic!("a node {name}"))
    };
    let operation = |name: &str| {
        StepClass::Operation(DeclarationKey {
            package: "example/protocol-obligations".to_owned(),
            node: format!("ix://example/protocol-obligations/Node/{name}"),
        })
    };
    assert_eq!(
        classes,
        [
            &operation("step"),
            &operation("undo"),
            &StepClass::ProtocolNode {
                kind: ProtocolStepKind::Fork,
                node: node("Both"),
            },
            &StepClass::ProtocolNode {
                kind: ProtocolStepKind::Join,
                node: node("Both"),
            },
            &StepClass::ProtocolNode {
                kind: ProtocolStepKind::Finish,
                node: node("Done"),
            },
        ]
    );
    // The attempts are visible with `add`'s label, the control steps internal.
    let observed: Vec<RowObservation> = refinement.steps().iter().map(observation).collect();
    let label = |observed: &RowObservation| match observed {
        RowObservation::Visible { label } => label.clone(),
        other => panic!("a visible row, got {other:?}"),
    };
    let add = StepLabel {
        step: LabelStep::Operation(abstract_key("Acc/add")),
        open_arguments: Vec::new(),
        binds_result: false,
    };
    assert_eq!(label(&observed[0]), add);
    assert_eq!(label(&observed[1]), add);
    assert_eq!(
        observed[2..],
        [
            RowObservation::Internal,
            RowObservation::Internal,
            RowObservation::Internal
        ]
    );
    assert!(refinement.concrete_protocol().is_some());
}

/// Selection half of FR-135-AC-10 (the header grammar is not built): the rows
/// that name `Fill` select it, and the clause is recorded by its
/// node whichever way the clause and the refinement are ordered; a unit that
/// selects no protocol profile has no node to record, so it refuses at the
/// first row that names the clause.
#[trace("TC-541", "FR-136-AC-6", "TC-540", "FR-135-AC-10")]
#[test]
fn the_rows_select_the_concrete_protocol_and_the_clause_has_a_node() {
    let graph = checks(&unit(&fill(""), ROWS));
    let protocol = only_refinement(&graph).concrete_protocol();
    assert!(protocol.is_some());
    // The same key whichever way the clause and the refinement are ordered.
    let reversed = header(true) + &refinement(ROWS) + &fill("");
    assert_eq!(
        only_refinement(&checks(&reversed)).concrete_protocol(),
        protocol
    );

    let text = header(false) + &fill("") + &refinement(ROWS);
    let found = refused(&text);
    let unselected = only(&found);
    assert_eq!(
        unselected.pair(),
        (Code::MissingDeclaration, Some("missing-selection"))
    );
    assert_eq!(unselected.at, "step fork M::Fill::Both -> stutter;");
    assert!(matches!(
        unselected.refinement(),
        RefinementCause::Check(CheckCause::MissingSelection { .. })
    ));
}

/// A missing `join` or `finish` row refuses naming it, and a second row for
/// one class conflicts with the first.
#[trace("TC-659", "FR-214-AC-2", "FR-136-AC-4")]
#[test]
fn a_missing_or_doubled_row_refuses() {
    let protocol = fill("");
    let without = |row: &str| refinement(&ROWS.replace(row, ""));
    let join = "step join M::Fill::Both -> stutter;";
    let found = refused(&(header(true) + &protocol + &without(join)));
    let join_missing = only(&found);
    assert_eq!(
        join_missing.pair(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(
        join_missing.refinement(),
        &missing(RowClass::Node, "join M::Fill::Both")
    );
    assert_eq!(join_missing.at, "FillRefinesAcc");

    let finish = "step M::Fill::Done -> stutter;";
    let found = refused(&(header(true) + &protocol + &without(finish)));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Node, "M::Fill::Done")
    );

    // The `fork` and `join` rows absent: the `fork` is named first, in
    // source order. With no row naming `Fill` at all the subject is a model
    // subject, so `Done`'s row is what selects the protocol here.
    let rows = "step M::Node::step -> A::Acc::add(self);\n\
                step M::Node::undo -> A::Acc::add(self);\n\
                step M::Fill::Done -> stutter;";
    let found = refused(&unit(&protocol, rows));
    let names: Vec<&RefinementCause> = found.iter().map(Refused::refinement).collect();
    assert_eq!(
        names,
        [
            &missing(RowClass::Node, "fork M::Fill::Both"),
            &missing(RowClass::Node, "join M::Fill::Both"),
        ]
    );

    // Two rows for the `step` operation, and two for the `join` node.
    let doubled = format!("{ROWS}\nstep M::Node::step -> stutter;");
    let text = unit(&protocol, &doubled);
    let found = refused(&text);
    let second = only(&found);
    assert_eq!(
        second.pair(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(second.at, "step M::Node::step -> stutter;");
    let RefinementCause::Conflicting { name, first } = second.refinement() else {
        panic!("a conflict: {:?}", second.refinement());
    };
    assert_eq!(name, "M::Node::step");
    assert_eq!(
        text_of(&text, *first),
        "step M::Node::step -> A::Acc::add(self);"
    );

    let doubled = format!("{ROWS}\nstep join M::Fill::Both -> any;");
    let found = refused(&unit(&protocol, &doubled));
    let join_twice = only(&found);
    assert_eq!(
        join_twice.pair(),
        (Code::InvalidModelBinding, Some("conflicting-binding"))
    );
    assert_eq!(
        join_twice.refinement(),
        &RefinementCause::Conflicting {
            name: "join M::Fill::Both".to_owned(),
            first: join_twice_first(&unit(&protocol, &doubled)),
        }
    );
}

fn join_twice_first(text: &str) -> Span {
    let start = text
        .find("step join M::Fill::Both -> stutter;")
        .expect("the first join row");
    Span {
        start,
        end: start + "step join M::Fill::Both -> stutter;".len(),
    }
}

/// A node row for an attempt maps it and takes precedence over its
/// operation's row, which may remain beside it unused; an attempt with
/// neither row refuses naming its node.
#[trace("TC-659", "FR-214-AC-3", "FR-136-AC-4")]
#[test]
fn an_attempt_node_row_takes_precedence_over_its_operation_row() {
    let protocol = fill("");
    // `A` maps by its node row; `step` stays beside it, unused by `A`.
    let rows = format!("{ROWS}\nstep M::Fill::A -> any;");
    let graph = checks(&unit(&protocol, &rows));
    let refinement = only_refinement(&graph);
    let attempt = refinement
        .steps()
        .iter()
        .find(|row| matches!(row.class, StepClass::AttemptNode(_)))
        .expect("the node row of A");
    assert_eq!(observation(attempt), RowObservation::Either);
    assert_eq!(refinement.steps().len(), 6);

    // With the node row, `step`'s operation row is not needed: `A` is the
    // only attempt of `step`.
    let rows = "step M::Fill::A -> stutter;\n\
                step M::Node::undo -> A::Acc::add(self);\n\
                step fork M::Fill::Both -> stutter;\n\
                step join M::Fill::Both -> stutter;\n\
                step M::Fill::Done -> stutter;";
    let graph = checks(&unit(&protocol, rows));
    assert_eq!(only_refinement(&graph).steps().len(), 5);

    // `B` has neither a node row nor an operation row.
    let rows = "step M::Node::step -> A::Acc::add(self);\n\
                step fork M::Fill::Both -> stutter;\n\
                step join M::Fill::Both -> stutter;\n\
                step M::Fill::Done -> stutter;";
    let found = refused(&unit(&protocol, rows));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Node, "M::Fill::B")
    );
}

/// One protocol per refinement. Covers FR-136-AC-9's first sentence only: a
/// clause declared in another package cannot be supplied by these fixtures,
/// so its sentence is untested. Rows that name `P`, `Q` and `R` select `P` and
/// refuse `Q` and `R` once each at their first row, naming `P`'s first row; a name that is no
/// protocol refuses at its segment; a clause whose roles act on another model
/// refuses `malformed-declaration` at the protocol name.
#[trace("TC-541", "FR-136-AC-6", "FR-136-AC-9")]
#[test]
fn a_refinement_checks_one_protocol_and_refuses_the_others_once() {
    let other = fill("").replace("protocol Fill", "protocol Other");
    let third = fill("").replace("protocol Fill", "protocol Third");
    let protocols = fill("") + &other + &third;
    let rows = format!(
        "{ROWS}\nstep M::Other::Done -> stutter;\nstep M::Third::Done -> stutter;\n\
         step M::Other::Done -> stutter;\nstep M::Third::Done -> stutter;"
    );
    let text = header(true) + &protocols + &refinement(&rows);
    let found = refused(&text);
    let [other_refusal, third_refusal] = &found[..] else {
        panic!("two refusals: {found:?}");
    };
    for (refused, name, row) in [
        (other_refusal, "Other", "step M::Other::Done -> stutter;"),
        (third_refusal, "Third", "step M::Third::Done -> stutter;"),
    ] {
        assert_eq!(
            refused.pair(),
            (Code::InvalidModelBinding, Some("conflicting-binding"))
        );
        // The first row naming the protocol, not the repeated one.
        assert_eq!(refused.at, row);
        assert_eq!(
            refused.refinement_refusal().span.start,
            text.find(row).unwrap()
        );
        let RefinementCause::Conflicting { name: named, first } = refused.refinement() else {
            panic!("a conflict: {:?}", refused.refinement());
        };
        assert_eq!(named, name);
        assert_eq!(
            text_of(&text, *first),
            "step fork M::Fill::Both -> stutter;"
        );
    }

    // A protocol name that is no clause, at the segment.
    let rows = format!("{ROWS}\nstep M::Nope::Done -> stutter;");
    let found = refused(&unit(&fill(""), &rows));
    let nope = only(&found);
    assert_eq!(
        nope.pair(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(nope.at, "Nope");

    // A clause whose roles act on another model.
    let abstract_clause = fill("")
        .replace("protocol Fill", "protocol Abs")
        .replace("role w on M::Node;", "role w on A::Acc;");
    let rows = format!("{ROWS}\nstep M::Abs::Done -> stutter;\nstep M::Abs::Done -> any;");
    let text = header(true) + &fill("") + &abstract_clause + &refinement(&rows);
    let found = refused(&text);
    let wrong_model = found
        .iter()
        .find(|refused| refused.pair().1 == Some("malformed-declaration"))
        .unwrap_or_else(|| panic!("a malformed refusal: {found:?}"));
    assert_eq!(wrong_model.pair().0, Code::InvalidModelBinding);
    assert_eq!(wrong_model.at, "Abs");
    assert_eq!(
        wrong_model.refinement(),
        &RefinementCause::Malformed {
            name: "Abs".to_owned()
        }
    );
    // Once, whichever number of rows name it.
    assert_eq!(
        found
            .iter()
            .filter(|refused| refused.pair().1 == Some("malformed-declaration"))
            .count(),
        1,
        "{found:?}"
    );
}

/// Model-subject half of FR-136-AC-6: a refinement whose rows name no protocol has a model subject and no
/// `concrete_protocol`; a `memory` row then refuses at its kind.
#[trace("TC-541", "FR-136-AC-6")]
#[test]
fn a_refinement_with_no_protocol_row_is_a_model_subject() {
    let rows = "step M::Node::step -> A::Acc::add(self);\n\
                step M::Node::undo -> A::Acc::add(self);";
    let graph = checks(&(header(true) + &refinement(rows)));
    assert_eq!(only_refinement(&graph).concrete_protocol(), None);
    assert_eq!(only_refinement(&graph).steps().len(), 2);

    let with_memory = format!("{rows}\nstep memory flush -> stutter;");
    let found = refused(&(header(true) + &refinement(&with_memory)));
    let flush = only(&found);
    assert_eq!(
        flush.pair(),
        (Code::MissingDeclaration, Some("missing-name"))
    );
    assert_eq!(flush.at, "flush");
}

/// The rows check whether `Fill` is declared before or after the refinement, and a
/// clause refused by its own checks reports its refusals alone.
#[trace("TC-659", "FR-214-AC-5")]
#[test]
fn the_coverage_waits_for_the_clause_and_a_refused_clause_reports_alone() {
    let before = header(true) + &fill("") + &refinement(ROWS);
    let after = header(true) + &refinement(ROWS) + &fill("");
    assert!(check_text(&before).is_ok());
    assert!(check_text(&after).is_ok());

    // A missing row is reported in either order.
    let partial = "step M::Node::step -> A::Acc::add(self);\n\
                   step M::Node::undo -> A::Acc::add(self);\n\
                   step fork M::Fill::Both -> stutter;\n\
                   step M::Fill::Done -> stutter;";
    for text in [
        header(true) + &fill("") + &refinement(partial),
        header(true) + &refinement(partial) + &fill(""),
    ] {
        let found = refused(&text);
        assert_eq!(
            only(&found).refinement(),
            &missing(RowClass::Node, "join M::Fill::Both")
        );
    }

    // `Fill` refused by its own checks: a nested `memory` default that
    // differs from its outermost `parallel`'s.
    let broken = fill("memory tso").replace(
        "branch left attempt A by w on M::Node::step contracts [] as (a: M::Flag) { true };",
        "branch left parallel Inner memory sc { \
         branch x attempt X by w on M::Node::step contracts [] as (xx: M::Flag) { true }; \
         branch y attempt Y by w on M::Node::step contracts [] as (yy: M::Flag) { true }; \
         } join all [x, y];",
    );
    for text in [
        header(true) + &broken + &refinement(ROWS),
        header(true) + &refinement(ROWS) + &broken,
    ] {
        let found = refused(&text);
        assert!(!found.is_empty());
        for refused in &found {
            assert!(
                matches!(refused.refusal.cause, CheckCause::ProtocolAnchor(_)),
                "the clause's own refusal alone: {refused:?}"
            );
        }
    }
}

/// A refusal of a row the clause does not decide is still
/// reported beside a refused clause's own refusals.
#[trace("TC-541", "FR-136-AC-8")]
#[test]
fn a_refusal_of_a_row_outside_the_clause_is_reported_beside_a_refused_clause() {
    let broken = fill("memory tso").replace(
        "branch left attempt A by w on M::Node::step contracts [] as (a: M::Flag) { true };",
        "branch left parallel Inner memory sc { \
         branch x attempt X by w on M::Node::step contracts [] as (xx: M::Flag) { true }; \
         branch y attempt Y by w on M::Node::step contracts [] as (yy: M::Flag) { true }; \
         } join all [x, y];",
    );
    // `add` takes no argument: the row is refused whatever the clause is.
    let rows = ROWS.replace(
        "step M::Node::step -> A::Acc::add(self);",
        "step M::Node::step -> A::Acc::add(self, 1);",
    );
    let found = refused(&(header(true) + &broken + &refinement(&rows)));
    assert!(
        found
            .iter()
            .any(|refused| refused.pair() == (Code::IllTyped, Some("operator-ineligible"))),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|refused| matches!(refused.refusal.cause, CheckCause::ProtocolAnchor(_))),
        "the clause's own refusal: {found:?}"
    );
    assert!(
        found.iter().all(|refused| !matches!(
            refused.refinement_or_none(),
            Some(RefinementCause::MissingRow { .. })
        )),
        "no coverage refusal: {found:?}"
    );
}

impl Refused {
    fn refinement_or_none(&self) -> Option<&RefinementCause> {
        match &self.refusal.cause {
            CheckCause::Refinement(refusal) => Some(&refusal.cause),
            _ => None,
        }
    }
}

/// Row half of FR-214-AC-4; the abstract-protocol half (the internal list,
/// FR-147) is not built. A row's target gives its observation: `-> any` gives
/// `Either`, `stutter` `Internal` and an abstract application `Visible` with
/// the operation and the arguments the row leaves open as its label.
#[trace("TC-659", "FR-214-AC-4")]
#[test]
fn a_row_target_gives_its_observation() {
    let rows = "step M::Node::step -> A::Acc::addBy(self, _);\n\
                step M::Node::undo -> A::Acc::addBy(self, 1);\n\
                step fork M::Fill::Both -> stutter;\n\
                step join M::Fill::Both -> any;\n\
                step M::Fill::Done -> stutter;";
    let graph = checks(&unit(&fill(""), rows));
    let observed: Vec<RowObservation> = only_refinement(&graph)
        .steps()
        .iter()
        .map(observation)
        .collect();
    let label = |open_arguments| RowObservation::Visible {
        label: StepLabel {
            step: LabelStep::Operation(abstract_key("Acc/addBy")),
            open_arguments,
            binds_result: false,
        },
    };
    assert_eq!(
        observed,
        [
            label(vec![0]),
            label(Vec::new()),
            RowObservation::Internal,
            RowObservation::Either,
            RowObservation::Internal,
        ]
    );
}

/// A protocol subject's rows have no `StepRowRef` order yet: its nodes have
/// no node key.
#[trace("TC-551")]
#[test]
fn a_protocol_subjects_rows_have_no_order_yet() {
    let graph = checks(&unit(&fill(""), ROWS));
    let refinement = only_refinement(&graph);
    let key = quire_exact::NodeKey::from_digest([1; 32]);
    assert_eq!(
        step_row_refs(refinement, |_| key),
        Err(StepRowRefusal::ProtocolSubject)
    );
    assert_eq!(
        step_simulation_records(key, refinement, &[]),
        Err(StepRowRefusal::ProtocolSubject)
    );
}

/// TC-1057's `Refunds`: an attempt `Tried` of `step`, the effect `Applied`,
/// the commit `Committed`, the `Refund` template (which compensates with
/// `step`), the `finish` node `Closed` and the `activation on each` trigger.
const REFUNDS: &str = "protocol Refunds using v over (view: M::Node)\n\
    on each (opened: M::Tick) when (opened.ready) {\n\
    capture order: M::Total = view.total;\n\
    role Service on M::Node;\n\
    compensate Refund for Main::Applied as (forward: M::Receipt)\n\
        by Service on M::Node::step using v clock \"refund-clock\" {\n\
      capture target: M::Total = forward.total;\n\
      activate first (trigger: M::Flag) when { not trigger.ready and target >= 0 } {\n\
        capture activated: Boolean = trigger.ready;\n\
      }\n\
      within [0,30]; attempts 3 of M::Attempt;\n\
      retry (earlier: M::Attempt, later: M::Attempt) {\n\
        earlier.tally < 3 and later.tally = earlier.tally + 1\n\
        and target >= 0 and not activated\n\
      };\n\
      commit Main::Committed;\n\
      recover (recovered: M::Receipt) { recovered.n = 1 and target >= 0 and not activated };\n\
    }\n\
    run sequence Main {\n\
      attempt Tried by Service on M::Node::step contracts [] as (attempted: M::Flag) { true };\n\
      effect Applied of Main::Tried as (applied: M::Receipt) { true };\n\
      commit Committed by Service as (committed: M::Flag) { true };\n\
    }\n\
    finish Closed as (closed: M::Receipt) { true };\n\
    }\n";

/// The rows `Refunds` needs.
const REFUND_ROWS: &str = "step M::Node::step -> A::Acc::add(self);\n\
    step M::Refunds::Applied -> stutter;\n\
    step M::Refunds::Committed -> stutter;\n\
    step M::Refunds::Closed -> stutter;\n\
    step cend M::Refunds::Refund -> stutter;\n\
    step activate M::Refunds -> stutter;";

/// `rows` without the row `row`.
fn without(rows: &str, row: &str) -> String {
    assert_eq!(rows.matches(row).count(), 1, "{row} is a row");
    rows.replace(row, "")
}

/// The `Refund` template's `cend`, the `effect` and `commit` nodes, the
/// `finish` node and the trigger each need a row, and the template's
/// `cattempt` is covered by the operation's row.
#[trace("TC-659", "FR-214-AC-3")]
#[test]
fn a_template_a_trigger_and_event_nodes_need_their_own_rows() {
    let graph = checks(&unit(REFUNDS, REFUND_ROWS));
    let refinement = only_refinement(&graph);
    let classes: Vec<&StepClass> = refinement.steps().iter().map(|row| &row.class).collect();
    assert!(classes.contains(&&StepClass::CompensationEnd("Refund".to_owned())));
    assert!(classes.contains(&&StepClass::Trigger));
    assert_eq!(
        classes
            .iter()
            .filter(|class| matches!(
                class,
                StepClass::ProtocolNode {
                    kind: ProtocolStepKind::Event,
                    ..
                }
            ))
            .count(),
        2
    );

    for (omitted, class, name) in [
        (
            "step cend M::Refunds::Refund -> stutter;",
            RowClass::Template,
            "cend M::Refunds::Refund",
        ),
        (
            "step activate M::Refunds -> stutter;",
            RowClass::Trigger,
            "activate M::Refunds",
        ),
        (
            "step M::Refunds::Applied -> stutter;",
            RowClass::Node,
            "M::Refunds::Applied",
        ),
        (
            "step M::Refunds::Committed -> stutter;",
            RowClass::Node,
            "M::Refunds::Committed",
        ),
        (
            "step M::Refunds::Closed -> stutter;",
            RowClass::Node,
            "M::Refunds::Closed",
        ),
    ] {
        let found = refused(&unit(REFUNDS, &without(REFUND_ROWS, omitted)));
        assert_eq!(
            only(&found).refinement(),
            &missing(class, name),
            "{omitted}"
        );
    }
}

/// FR-136 "Coverage over a protocol subject": a `cattempt` is covered by the
/// operation's row or by a row naming its template, and an attempt by its
/// operation's row or its node row.
#[trace("TC-541")]
#[test]
fn a_cattempt_is_covered_by_its_operations_row_or_its_templates() {
    // Neither `Tried` nor the template's `cattempt` has a row.
    let rows = without(REFUND_ROWS, "step M::Node::step -> A::Acc::add(self);");
    let found = refused(&unit(REFUNDS, &rows));
    let names: Vec<&RefinementCause> = found.iter().map(Refused::refinement).collect();
    assert_eq!(
        names,
        [
            &missing(RowClass::Node, "M::Refunds::Tried"),
            &missing(RowClass::Template, "M::Refunds::Refund"),
        ]
    );
    // The node row and the template row cover them with no operation row.
    let rows =
        format!("{rows}\nstep M::Refunds::Tried -> any;\nstep M::Refunds::Refund -> stutter;");
    let graph = checks(&unit(REFUNDS, &rows));
    let classes: Vec<&StepClass> = only_refinement(&graph)
        .steps()
        .iter()
        .map(|row| &row.class)
        .collect();
    assert!(classes.contains(&&StepClass::CompensationAttempt("Refund".to_owned())));
    assert!(classes
        .iter()
        .any(|class| matches!(class, StepClass::AttemptNode(_))));
}

/// A row that names a declaration the clause has no such step of refuses
/// `missing-name` at the name: a `fork` of an attempt, an unknown node.
#[trace("TC-541")]
#[test]
fn a_row_naming_no_class_of_the_clause_refuses_at_its_name() {
    for (extra, at) in [
        ("step fork M::Refunds::Tried -> stutter;", "Tried"),
        ("step M::Refunds::Nowhere -> stutter;", "Nowhere"),
        ("step cend M::Refunds::Tried -> stutter;", "Tried"),
        ("step spawn M::Refunds::Service -> stutter;", "Service"),
        ("step duplicate M::Refunds::Wire -> stutter;", "Wire"),
    ] {
        let rows = format!("{REFUND_ROWS}\n{extra}");
        let found = refused(&unit(REFUNDS, &rows));
        let unresolved = only(&found);
        assert_eq!(
            unresolved.pair(),
            (Code::MissingDeclaration, Some("missing-name")),
            "{extra}"
        );
        assert_eq!(unresolved.at, at, "{extra}");
    }
    // `Refunds` is `on each`; a protocol that is `on origin` has no trigger.
    let on_origin = fill("");
    let rows = format!("{ROWS}\nstep activate M::Fill -> stutter;");
    let found = refused(&unit(&on_origin, &rows));
    assert_eq!(only(&found).at, "Fill");
}

/// A channel whose delivery
/// is `unknown` has a `duplicate` and a `lose` class, `at-least-once` only
/// the first, `at-most-once` only the second and the exactly-once premise
/// none.
#[trace("TC-659", "FR-214-AC-3")]
#[test]
fn a_channel_needs_a_row_for_each_step_its_delivery_admits() {
    let chatter = |delivery: &str| {
        fill("").replace(
            "role w on M::Node;",
            &format!(
                "role w on M::Node;\n\
                 channel Ch from w to w carries M::Flag ordering unordered \
                 delivery {delivery} capacity 1 overflow reject;"
            ),
        )
    };
    let duplicate = "step duplicate M::Fill::Ch -> stutter;";
    let lose = "step lose M::Fill::Ch -> stutter;";
    let rows = format!("{ROWS}\n{duplicate}\n{lose}");
    let graph = checks(&unit(&chatter("unknown"), &rows));
    assert_eq!(only_refinement(&graph).steps().len(), 7);

    let found = refused(&unit(&chatter("unknown"), &format!("{ROWS}\n{lose}")));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Channel, "duplicate M::Fill::Ch")
    );
    let found = refused(&unit(&chatter("unknown"), &format!("{ROWS}\n{duplicate}")));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Channel, "lose M::Fill::Ch")
    );

    checks(&unit(
        &chatter("at-least-once"),
        &format!("{ROWS}\n{duplicate}"),
    ));
    let found = refused(&unit(&chatter("at-least-once"), &rows));
    assert_eq!(only(&found).pair().1, Some("missing-name"));
    assert_eq!(only(&found).at, "Ch");

    checks(&unit(&chatter("at-most-once"), &format!("{ROWS}\n{lose}")));
    let found = refused(&unit(&chatter("at-most-once"), &rows));
    assert_eq!(only(&found).at, "Ch");

    checks(&unit(&chatter("exactly-once-premise"), ROWS));
    let found = refused(&unit(&chatter("exactly-once-premise"), &rows));
    assert_eq!(found.len(), 2, "{found:?}");
}

/// A replicated role over fixture R: `r` drawn from `nodes` with `lifetime`
/// and an `event Work by r`.
fn workers(lifetime: &str) -> String {
    format!(
        "protocol Workers using v over (k: M::Node) on origin {{\n\
         capture start: M::Total = k.total;\n\
         role w on M::Node;\n\
         role r each M::Node from nodes max 1 lifetime {lifetime};\n\
         run sequence Main {{\n\
         sequence S {{ event Work by r as (worked: M::Flag) {{ true }}; }}\n\
         check After using v {{ true }};\n\
         }}\n\
         finish Done as (d: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

/// FR-136 "Coverage over a protocol subject": each replicated role needs a
/// `spawn` row, and a role whose lifetime is `until` a `retire` row too.
#[trace("TC-541")]
#[test]
fn a_replicated_role_needs_a_spawn_row_and_an_until_role_a_retire_row() {
    let base = "step M::Workers::Work -> stutter;\n\
                step M::Workers::Done -> stutter;";
    let spawn = "step spawn M::Workers::r -> stutter;";
    let retire = "step retire M::Workers::r -> stutter;";
    let until = workers("until(r.total > 0 and k.total >= start)");

    checks(&unit(&workers("workflow"), &format!("{base}\n{spawn}")));
    let found = refused(&unit(&workers("workflow"), base));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Role, "spawn M::Workers::r")
    );
    // No retire class for a `workflow` role.
    let found = refused(&unit(
        &workers("workflow"),
        &format!("{base}\n{spawn}\n{retire}"),
    ));
    assert_eq!(only(&found).at, "r");

    checks(&unit(&until, &format!("{base}\n{spawn}\n{retire}")));
    let found = refused(&unit(&until, &format!("{base}\n{spawn}")));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Role, "retire M::Workers::r")
    );
    let found = refused(&unit(&until, &format!("{base}\n{retire}")));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Role, "spawn M::Workers::r")
    );
}

/// S3 half of FR-136-AC-10; the check-request half (another memory model
/// selected for a `parallel`) is not built. A `parallel` under `tso` has the
/// memory step `flush`, which needs a `memory flush` row; under `sc`, a
/// `memory flush` row names no kind; one row covers every outermost
/// `parallel`.
#[trace("TC-541", "FR-136-AC-6", "FR-136-AC-10")]
#[test]
fn a_weak_memory_model_needs_a_row_for_each_internal_step() {
    let flush = "step memory flush -> stutter;";
    let tso = fill("memory tso");
    let graph = checks(&unit(&tso, &format!("{ROWS}\n{flush}")));
    assert!(only_refinement(&graph)
        .steps()
        .iter()
        .any(|row| row.class == StepClass::MemoryStep(MemoryStepKind::Flush)));
    let found = refused(&unit(&tso, ROWS));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Memory, "memory flush")
    );

    // A kind the model lacks, under `tso` and under `sc`.
    let found = refused(&unit(
        &tso,
        &format!("{ROWS}\n{flush}\nstep memory nope -> stutter;"),
    ));
    assert_eq!(only(&found).at, "nope");
    assert_eq!(only(&found).pair().1, Some("missing-name"));
    let found = refused(&unit(&fill(""), &format!("{ROWS}\n{flush}")));
    assert_eq!(only(&found).at, "flush");
    // `ra` has no internal step either.
    let found = refused(&unit(&fill("memory ra"), &format!("{ROWS}\n{flush}")));
    assert_eq!(only(&found).at, "flush");
    checks(&unit(&fill("memory ra"), ROWS));

    // Two outermost `parallel` nodes, one `sc` and one `tso`: one row.
    let two = "protocol Two using v over (k: M::Node) on origin {\n\
         role w on M::Node;\n\
         run sequence Main {\n\
         parallel First memory sc {\n\
         branch l1 attempt A1 by w on M::Node::step contracts [] as (a1: M::Flag) { true };\n\
         branch r1 attempt B1 by w on M::Node::undo contracts [] as (b1: M::Flag) { true };\n\
         } join all [l1, r1];\n\
         parallel Second memory tso {\n\
         branch l2 attempt A2 by w on M::Node::step contracts [] as (a2: M::Flag) { true };\n\
         branch r2 attempt B2 by w on M::Node::undo contracts [] as (b2: M::Flag) { true };\n\
         } join all [l2, r2];\n\
         }\n\
         finish Done as (d: M::Receipt) { true };\n\
         }\n";
    let rows = "step M::Node::step -> A::Acc::add(self);\n\
                step M::Node::undo -> A::Acc::add(self);\n\
                step fork M::Two::First -> stutter;\n\
                step join M::Two::First -> stutter;\n\
                step fork M::Two::Second -> stutter;\n\
                step join M::Two::Second -> stutter;\n\
                step M::Two::Done -> stutter;";
    let found = refused(&unit(two, rows));
    assert_eq!(
        only(&found).refinement(),
        &missing(RowClass::Memory, "memory flush")
    );
    let graph = checks(&unit(two, &format!("{rows}\n{flush}")));
    let flushes = only_refinement(&graph)
        .steps()
        .iter()
        .filter(|row| matches!(row.class, StepClass::MemoryStep(_)))
        .count();
    assert_eq!(flushes, 1);
}

/// An operation row for an operation no attempt uses is admitted
/// (FR-136-AC-6); two attempts with one node name in different scopes make
/// a row naming it ambiguous.
#[trace("TC-541", "FR-136-AC-6")]
#[test]
fn an_unused_operation_row_is_admitted_and_a_repeated_node_name_is_ambiguous() {
    let solo = "protocol Solo using v over (k: M::Node) on origin {\n\
         role w on M::Node;\n\
         run sequence Main {\n\
         attempt A by w on M::Node::step contracts [] as (a: M::Flag) { true };\n\
         }\n\
         finish Done as (d: M::Receipt) { true };\n\
         }\n";
    let rows = "step M::Node::step -> A::Acc::add(self);\n\
                step M::Node::undo -> A::Acc::add(self);\n\
                step M::Solo::Done -> stutter;";
    assert_eq!(only_refinement(&checks(&unit(solo, rows))).steps().len(), 3);

    let twice = "protocol Twice using v over (k: M::Node) on origin {\n\
         role w on M::Node;\n\
         run sequence Main {\n\
         sequence S1 { attempt A by w on M::Node::step contracts [] as (a1: M::Flag) { true }; }\n\
         sequence S2 { attempt A by w on M::Node::step contracts [] as (a2: M::Flag) { true }; }\n\
         }\n\
         finish Done as (d: M::Receipt) { true };\n\
         }\n";
    let rows = "step M::Twice::A -> stutter;\n\
                step M::Twice::Done -> stutter;";
    let found = refused(&unit(twice, rows));
    // The row is refused, so neither attempt has a row.
    let [ambiguous, first, second] = &found[..] else {
        panic!("an ambiguity and two missing rows: {found:?}");
    };
    assert_eq!(
        ambiguous.pair(),
        (Code::AmbiguousDeclaration, Some("ambiguous-name"))
    );
    assert_eq!(ambiguous.at, "M::Twice::A");
    for missing_row in [first, second] {
        assert_eq!(
            missing_row.refinement(),
            &missing(RowClass::Node, "M::Twice::A")
        );
    }
}

/// A sender and a receiver in the two branches of a `parallel`: the nodes
/// whose steps are ST-3 and ST-4.
const MESSAGES: &str = "protocol Msg using v over (k: M::Node) on origin {\n\
    role w on M::Node;\n\
    role z on M::Node;\n\
    channel Ch from w to z carries M::Flag ordering unordered \
    delivery exactly-once-premise capacity 1 overflow reject;\n\
    run sequence Main {\n\
    parallel P {\n\
    branch s sequence S { send Tx via Ch as (tx: M::Flag) { true }; }\n\
    branch r sequence R { receive Rx via Ch of Main::P::s::S::Tx \
    as (rx: M::Flag) { true }; }\n\
    } join all [s, r];\n\
    }\n\
    finish Done as (d: M::Receipt) { true };\n\
    }\n";

const MESSAGE_ROWS: &str = "step fork M::Msg::P -> stutter;\n\
    step join M::Msg::P -> stutter;\n\
    step M::Msg::Tx -> stutter;\n\
    step M::Msg::Rx -> stutter;\n\
    step M::Msg::Done -> stutter;";

/// A `send` and a `receive` each need a node row of the step kind the node
/// has.
#[trace("TC-541")]
#[test]
fn a_send_and_a_receive_need_node_rows() {
    let graph = checks(&unit(MESSAGES, MESSAGE_ROWS));
    let kinds: Vec<ProtocolStepKind> = only_refinement(&graph)
        .steps()
        .iter()
        .filter_map(|row| match row.class {
            StepClass::ProtocolNode { kind, .. } => Some(kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ProtocolStepKind::Fork,
            ProtocolStepKind::Join,
            ProtocolStepKind::Send,
            ProtocolStepKind::Receive,
            ProtocolStepKind::Finish,
        ]
    );
    for node in ["Tx", "Rx"] {
        let omitted = format!("step M::Msg::{node} -> stutter;\n");
        let found = refused(&unit(MESSAGES, &without(MESSAGE_ROWS, &omitted)));
        assert_eq!(
            only(&found).refinement(),
            &missing(RowClass::Node, &format!("M::Msg::{node}")),
            "{node}"
        );
    }
}

/// An `await` is the node whose step is the `timeout`, and the event it
/// matches has a step of its own.
#[trace("TC-541")]
#[test]
fn an_await_needs_a_timeout_row_and_its_matched_event_a_row() {
    let waiting = REFUNDS.replace(
        "effect Applied of Main::Tried as (applied: M::Receipt) { true };\n",
        "effect Applied of Main::Tried as (applied: M::Receipt) { true };\n\
         await Wait after Applied using v clock \"ticks\" within [0,1] \
         match event Seen by Service for Refund as (seen: M::Flag) { true }; \
         then sequence Then { } timeout sequence Late { }\n",
    );
    let rows = format!(
        "{REFUND_ROWS}\nstep M::Refunds::Wait -> stutter;\nstep M::Refunds::Seen -> stutter;"
    );
    let graph = checks(&unit(&waiting, &rows));
    let kinds: Vec<ProtocolStepKind> = only_refinement(&graph)
        .steps()
        .iter()
        .filter_map(|row| match row.class {
            StepClass::ProtocolNode { kind, .. } => Some(kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ProtocolStepKind::Event,
            ProtocolStepKind::Event,
            ProtocolStepKind::Finish,
            ProtocolStepKind::Timeout,
            ProtocolStepKind::Event,
        ]
    );
    for node in ["Wait", "Seen"] {
        let omitted = format!("step M::Refunds::{node} -> stutter;");
        let found = refused(&unit(&waiting, &without(&rows, &omitted)));
        assert_eq!(
            only(&found).refinement(),
            &missing(RowClass::Node, &format!("M::Refunds::{node}")),
            "{node}"
        );
    }
}

/// Two refinements of one unit, over two protocols, record the two clauses'
/// own nodes.
#[trace("TC-541", "FR-136-AC-6")]
#[test]
fn each_refinement_records_the_node_of_its_own_clause() {
    let other = fill("").replace("protocol Fill", "protocol Other");
    let other_rows = ROWS.replace("M::Fill", "M::Other");
    let text = header(true)
        + &fill("")
        + &other
        + &named_refinement("OverFill", ROWS)
        + &named_refinement("OverOther", &other_rows);
    let graph = checks(&text);
    let [over_fill, over_other] = graph.refinements() else {
        panic!("two refinements: {:?}", graph.refinements());
    };
    assert_eq!(over_fill.name(), "OverFill");
    assert_eq!(over_other.name(), "OverOther");
    assert!(over_fill.concrete_protocol().is_some());
    assert!(over_other.concrete_protocol().is_some());
    assert_ne!(
        over_fill.concrete_protocol(),
        over_other.concrete_protocol()
    );
}

/// Every subset of `Fill`'s candidate rows gives the verdict and the missing
/// rows that a direct statement of the coverage rule gives: an attempt is
/// covered by its node row or its operation's, every other class by its own
/// row, and with no row naming `Fill` the subject is a model subject that
/// needs a row for each of its two operations.
#[trace("TC-541", "FR-136-AC-4")]
#[test]
fn coverage_agrees_with_a_direct_statement_of_the_rule_over_every_subset() {
    let candidates = [
        "step M::Node::step -> A::Acc::add(self);",
        "step M::Node::undo -> A::Acc::add(self);",
        "step M::Fill::A -> stutter;",
        "step M::Fill::B -> any;",
        "step fork M::Fill::Both -> stutter;",
        "step join M::Fill::Both -> stutter;",
        "step M::Fill::Done -> stutter;",
    ];
    let protocol = fill("");
    for chosen in 0_u32..(1 << candidates.len()) {
        let has = |index: usize| chosen & (1 << index) != 0;
        let rows: Vec<&str> = (0..candidates.len())
            .filter(|index| has(*index))
            .map(|index| candidates[index])
            .collect();
        let names_protocol = (2..candidates.len()).any(has);
        // The classes without a row, as a refusal names them.
        let missing_rows: Vec<&str> = if names_protocol {
            [
                (!(has(2) || has(0))).then_some("M::Fill::A"),
                (!(has(3) || has(1))).then_some("M::Fill::B"),
                (!has(4)).then_some("fork M::Fill::Both"),
                (!has(5)).then_some("join M::Fill::Both"),
                (!has(6)).then_some("M::Fill::Done"),
            ]
            .into_iter()
            .flatten()
            .collect()
        } else {
            [
                (!has(0)).then_some("M::Node::step"),
                (!has(1)).then_some("M::Node::undo"),
            ]
            .into_iter()
            .flatten()
            .collect()
        };
        let text = unit(&protocol, &rows.join("\n"));
        match check_text(&text) {
            Ok(_) => assert!(missing_rows.is_empty(), "{rows:?} checks"),
            Err(found) => {
                let mut named: Vec<String> = found
                    .iter()
                    .map(|refused| match refused.refinement() {
                        RefinementCause::MissingRow { name, .. } => name.clone(),
                        other => panic!("{rows:?}: a missing row, got {other:?}"),
                    })
                    .collect();
                let mut expected: Vec<String> =
                    missing_rows.iter().map(|name| (*name).to_owned()).collect();
                named.sort();
                expected.sort();
                assert_eq!(named, expected, "{rows:?}");
            }
        }
    }
}

/// A protocol subject's own refusals are reported with those of another
/// refinement that stops the unit before the clauses are checked, in either
/// order of the two refinements.
#[trace("TC-541", "FR-136-AC-8")]
#[test]
fn a_protocol_subjects_refusals_survive_another_refinements_refusal() {
    let over_fill = named_refinement(
        "OverFill",
        &ROWS.replace(
            "step M::Node::step -> A::Acc::add(self);",
            "step M::Node::step -> A::Acc::add(self, 1);",
        ),
    );
    // A model subject missing its `undo` row.
    let over_model = named_refinement("OverModel", "step M::Node::step -> A::Acc::add(self);");
    for refinements in [
        over_fill.clone() + &over_model,
        over_model.clone() + &over_fill,
    ] {
        let found = refused(&(header(true) + &fill("") + &refinements));
        let pairs: Vec<(Code, Option<&'static str>)> = found.iter().map(Refused::pair).collect();
        assert!(
            pairs.contains(&(Code::IllTyped, Some("operator-ineligible"))),
            "the protocol subject's own refusal: {found:?}"
        );
        assert!(
            found.iter().any(|refused| refused.refinement_or_none()
                == Some(&missing(RowClass::Step, "M::Node::undo"))),
            "the model subject's refusal: {found:?}"
        );
    }
}

/// A right side that names an abstract protocol node is malformed while the
/// abstract side is a model, and an application on a row with no operation
/// is unsupported.
#[trace("TC-541")]
#[test]
fn a_protocol_row_refuses_a_node_target_and_an_application_without_an_operation() {
    let finish = "step M::Fill::Done -> stutter;";
    let node_target = ROWS.replace(finish, "step M::Fill::Done -> node A::P::x;");
    let found = refused(&unit(&fill(""), &node_target));
    assert_eq!(
        only(&found).refinement(),
        &RefinementCause::Malformed {
            name: "A::P::x".to_owned()
        }
    );

    let applied = ROWS.replace(finish, "step M::Fill::Done -> A::Acc::add(self);");
    let found = refused(&unit(&fill(""), &applied));
    let [unsupported] = &found[..] else {
        panic!("one refusal: {found:?}");
    };
    assert_eq!(
        unsupported.pair(),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
    assert!(matches!(
        unsupported.refinement(),
        RefinementCause::Unsupported {
            feature: UnsupportedFeature::ProtocolStepApplication,
            ..
        }
    ));
    assert_eq!(unsupported.at, "A::Acc::add(self)");
}

/// An application on an attempt-node row or a template's `cattempt` row is
/// typed over the operation the step applies, as an operation row is.
#[trace("TC-541")]
#[test]
fn an_application_on_an_attempt_or_template_row_is_typed_over_its_operation() {
    let rows = "step M::Node::undo -> A::Acc::add(self);\n\
                step M::Fill::A -> A::Acc::addBy(self, _);\n\
                step fork M::Fill::Both -> stutter;\n\
                step join M::Fill::Both -> stutter;\n\
                step M::Fill::Done -> stutter;";
    let graph = checks(&unit(&fill(""), rows));
    let attempt = only_refinement(&graph)
        .steps()
        .iter()
        .find(|row| matches!(row.class, StepClass::AttemptNode(_)))
        .expect("the row of A");
    assert!(matches!(
        observation(attempt),
        RowObservation::Visible { label } if label.open_arguments == [0]
    ));
    // A population-valued argument is in scope on the node row, as on an
    // operation row.
    let counted = rows.replace(
        "addBy(self, _)",
        "addBy(self, count(x in M::nodes where x.total = 0))",
    );
    checks(&unit(&fill(""), &counted));
    // One argument too many is the operation row's refusal.
    let wrong = rows.replace("addBy(self, _)", "add(self, 1)");
    let found = refused(&unit(&fill(""), &wrong));
    assert_eq!(
        only(&found).pair(),
        (Code::IllTyped, Some("operator-ineligible"))
    );

    let template_rows = REFUND_ROWS.replace("step M::Node::step -> A::Acc::add(self);\n", "")
        + "\nstep M::Refunds::Tried -> any;\nstep M::Refunds::Refund -> A::Acc::add(self);";
    let graph = checks(&unit(REFUNDS, &template_rows));
    let template = only_refinement(&graph)
        .steps()
        .iter()
        .find(|row| matches!(row.class, StepClass::CompensationAttempt(_)))
        .expect("the row of Refund");
    assert!(matches!(
        observation(template),
        RowObservation::Visible { .. }
    ));
}

/// A model subject keeps FR-136's refusal at the row's span: a keyworded row
/// names no operation, and a name whose type is unknown is no operation's.
#[trace("TC-541")]
#[test]
fn a_model_subject_refuses_a_protocol_shaped_row_at_its_name() {
    let base = "step M::Node::step -> A::Acc::add(self);\n\
                step M::Node::undo -> A::Acc::add(self);";
    for (extra, at) in [
        ("step fork M::Node::step -> stutter;", "M::Node::step"),
        ("step M::Nope::x -> stutter;", "M::Nope::x"),
    ] {
        let found = refused(&(header(true) + &refinement(&format!("{base}\n{extra}"))));
        let unresolved = only(&found);
        assert_eq!(
            unresolved.pair(),
            (Code::MissingDeclaration, Some("missing-name")),
            "{extra}"
        );
        assert_eq!(unresolved.at, at, "{extra}");
    }
}

/// Two clauses of different shape: the refinement is checked against the
/// clause its rows name, whichever the unit declares first.
#[trace("TC-541", "FR-136-AC-6")]
#[test]
fn a_refinement_is_covered_against_the_clause_it_selects() {
    let solo = "protocol Solo using v over (k: M::Node) on origin {\n\
         role w on M::Node;\n\
         run sequence Main {\n\
         attempt A by w on M::Node::step contracts [] as (a: M::Flag) { true };\n\
         }\n\
         finish Done as (d: M::Receipt) { true };\n\
         }\n";
    let solo_rows = "step M::Node::step -> A::Acc::add(self);\n\
                     step M::Solo::Done -> stutter;";
    for protocols in [fill("") + solo, solo.to_owned() + &fill("")] {
        checks(&(header(true) + &protocols + &named_refinement("OverSolo", solo_rows)));
        checks(&(header(true) + &protocols + &named_refinement("OverFill", ROWS)));
    }
}
