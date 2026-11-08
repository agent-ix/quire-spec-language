// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-529-AC-5 and FR-529-AC-9, QSpec TC-984 (FR-490-AC-1 to AC-9): a
//! `guarantee` declaration over the compare-and-set counter of TC-982 is
//! compiled from real source through S1, S2 and S3, its derived slots are
//! read, its node is written and read back, and every refusal carries
//! QSpec FR-490's code and cause.

use ix_trace_rs::trace;
use qsl_semantics::check::{
    AbstractionKey, AbstractionRefusal, AssemblyCause, CheckCause, CheckRefusal,
    CheckedImplementationGuarantee, GuaranteeName, GuaranteeRefusal, GuaranteeSlot, ModelFamily,
    SlotBacking, SlotRole,
};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind, PropertyForm};

use super::*;

#[path = "../../../../qsl-semantics/tests/support/guarantee.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the semantics tests load the same fixture"
)]
mod fixture;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.model.complete/v1\";\n\
     profile t = \"quire.temporal.infinite-trace/v1\";\n\
     model Spec = \"test/orders\";\n";

const LIVENESS: &str =
    "temporal Eventually2 using t over (c: Spec::Counter) clock \"model-steps\" \
     on origin { fair weak each Spec::Counter::inc; always eventually holds(c.value = 2) }\n";

const SAFETY: &str = "temporal NonNegative using t over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always holds(c.value >= 0) }\n";

const RELATION: &str = "abstraction A using v {\n\
     type Spec::Counter to \"counter::Counter\" { value to \"value\"; };\n\
     population Spec::counters to \"state::counters\";\n\
     operation Spec::Counter::inc to \"counter::Counter::inc_cas\" receiver \"self\" ();\n\
     }\n";

const RELATION_TWO: &str = "abstraction A using v {\n\
     type Spec::Counter to \"counter::Counter\" { value to \"value\"; };\n\
     population Spec::counters to \"state::counters\";\n\
     operation Spec::Counter::inc to \"counter::Counter::inc_cas\" receiver \"self\" ();\n\
     operation Spec::Counter::dec to \"counter::Counter::dec_cas\" receiver \"self\" ();\n\
     }\n";

const RELATION_WITHOUT_INC: &str = "abstraction A using v {\n\
     type Spec::Counter to \"counter::Counter\" { value to \"value\"; };\n\
     population Spec::counters to \"state::counters\";\n\
     }\n";

/// TC-984 case 1's declaration.
const CASE_ONE: &str = "guarantee G of Eventually2 using A { \
     initial to \"counter::Counter::new\"; \
     progress Spec::Counter::inc decreases \"counter::retries_left\"; \
     environment Sched by \"scheduler\" fair weak each Spec::Counter::inc; }\n";

/// Why a unit does not compile to a checked package.
#[derive(Debug)]
enum Failure {
    Syntax(Vec<qsl_cst::CompleteDiagnostic>),
    Assembly(qsl_semantics::check::AssemblyRefusal),
    Check(Vec<CheckRefusal>),
}

fn compile_with(unit: &str, documents: &[Value]) -> Result<CheckedPackage, Failure> {
    let bytes: Vec<Vec<u8>> = documents
        .iter()
        .map(|document| serde_json::to_vec(document).expect("the fixture serializes"))
        .collect();
    let packages =
        qsl_semantics::model::intake::package_input(bytes.iter().map(Vec::as_slice)).unwrap();
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:guarantee"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        return Err(Failure::Syntax(parsed.diagnostics().to_vec()));
    }
    let raw = parsed.source().reference().clone();
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the models are admitted");
    let assembled = PackageDeclarations::assemble(raw, "pkg".to_owned(), built, models, Vec::new())
        .map_err(Failure::Assembly)?;
    assembled
        .check(CheckingLimits::default())
        .map(CheckedPackage::link)
        .map_err(Failure::Check)
}

fn compile(unit: &str) -> Result<CheckedPackage, Failure> {
    compile_with(unit, &[fixture::document(&["inc"])])
}

/// The unit with `clauses`, `relation` and `declarations`.
fn unit(clauses: &str, relation: &str, declarations: &str) -> String {
    format!("{HEADER}{clauses}{relation}{declarations}")
}

fn checked(unit: &str) -> CheckedPackage {
    compile(unit).unwrap_or_else(|failure| panic!("{unit}: the unit compiles: {failure:?}"))
}

/// The one guarantee refusal of `unit`.
fn refused(unit: &str, documents: &[Value]) -> GuaranteeRefusal {
    let Err(Failure::Check(refusals)) = compile_with(unit, documents) else {
        panic!("{unit}: a check refusal");
    };
    let [CheckRefusal {
        cause: CheckCause::Abstraction(cause),
        ..
    }] = &refusals[..]
    else {
        panic!("{unit}: one guarantee refusal: {refusals:?}");
    };
    let AbstractionRefusal::Guarantee(refusal) = &**cause else {
        panic!("{unit}: a guarantee refusal: {cause:?}");
    };
    (**refusal).clone()
}

/// The wire of `package` emitted, and IR's read of it against `document`.
fn emit_and_read(package: &CheckedPackage, document: &Value) -> (Value, Read) {
    let emission = emit_checked(package).expect("the package emits");
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(package_identity(&bytes), bytes.as_slice())
        .expect("a fresh evidence key");
    let read = read_with(&emission, &evidence);
    (wire(&emission), read)
}

fn sole(package: &CheckedPackage) -> &CheckedImplementationGuarantee {
    let [guarantee] = package.graph().implementation_guarantees() else {
        panic!("one guarantee");
    };
    guarantee
}

fn roles(slots: &[GuaranteeSlot]) -> Vec<SlotRole> {
    slots.iter().map(|slot| slot.role).collect()
}

fn text(unit: &str, span: qsl_foundation::Span) -> &str {
    &unit[span.start..span.end]
}

/// FR-490-AC-1: the liveness declaration derives, in order, the model claim
/// (with `stutter_closure`), initialization, transition simulation, frame,
/// environment, no-divergence and fairness-correspondence slots, each with a
/// distinct identity and the span of the member that produced it.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-1")]
#[test]
fn a_liveness_declaration_derives_its_slots_in_order() {
    let source = unit(&format!("{LIVENESS}{SAFETY}"), RELATION, CASE_ONE);
    let package = checked(&source);
    let guarantee = sole(&package);
    assert_eq!(guarantee.item_refusal(), None);
    assert_eq!(guarantee.form(), PropertyForm::Liveness);
    let slots = guarantee.slots("pkg");
    assert_eq!(
        roles(&slots),
        [
            SlotRole::ModelClaim,
            SlotRole::Initialization,
            SlotRole::TransitionSimulation,
            SlotRole::Frame,
            SlotRole::Environment,
            SlotRole::NoDivergence,
            SlotRole::FairnessCorrespondence,
        ]
    );
    assert_eq!(
        slots.iter().map(|slot| slot.backing).collect::<Vec<_>>(),
        [
            SlotBacking::Item,
            SlotBacking::Item,
            SlotBacking::Item,
            SlotBacking::Item,
            SlotBacking::Environment,
            SlotBacking::Item,
            SlotBacking::PlannerLocal,
        ]
    );
    let model_claim = slots[0].key.as_ref().expect("the claim is item-backed");
    assert!(model_claim.stutter_closure);
    assert_eq!(&model_claim.node, &guarantee.clause().node);
    let contract = slots[2].key.as_ref().expect("an item-backed slot");
    assert_eq!(&contract.node, guarantee.node());
    assert_eq!(&contract.claim, guarantee.node());
    assert_eq!(
        contract.correspondence.as_ref().map(|c| c.role),
        Some(SlotRole::TransitionSimulation)
    );
    for (index, slot) in slots.iter().enumerate() {
        assert_eq!(slot.index, index);
        for other in &slots[..index] {
            assert_ne!(slot, other, "slot {index} is distinct");
        }
    }
    // The span of each slot is the member that produced it.
    let sources: Vec<&str> = slots
        .iter()
        .map(|slot| text(&source, slot.source))
        .collect();
    assert_eq!(sources[0], "Eventually2");
    assert_eq!(sources[1], "initial to \"counter::Counter::new\";");
    assert_eq!(sources[2], "A");
    assert_eq!(sources[3], "A");
    assert_eq!(
        sources[4],
        "environment Sched by \"scheduler\" fair weak each Spec::Counter::inc;"
    );
    assert_eq!(
        sources[5],
        "progress Spec::Counter::inc decreases \"counter::retries_left\";"
    );
    assert_eq!(sources[6], "Eventually2");
    assert_eq!(
        slots[6]
            .constraint
            .as_ref()
            .map(|c| (c.kind(), c.granularity(), c.operation().name())),
        Some((FairnessKind::Weak, FairnessGranularity::Each, "inc"))
    );
}

/// FR-490-AC-2: the same declaration over a safety clause derives no
/// no-divergence, fairness-correspondence or unfilled environment slot, and
/// its model-claim slot has no `stutter_closure`.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-2")]
#[test]
fn a_safety_declaration_derives_no_progress_slot() {
    let source = unit(
        &format!("{LIVENESS}{SAFETY}"),
        RELATION,
        &CASE_ONE.replace("Eventually2", "NonNegative"),
    );
    let package = checked(&source);
    let guarantee = sole(&package);
    assert_eq!(guarantee.form(), PropertyForm::ReachableInvariant);
    let slots = guarantee.slots("pkg");
    assert_eq!(
        roles(&slots),
        [
            SlotRole::ModelClaim,
            SlotRole::Initialization,
            SlotRole::TransitionSimulation,
            SlotRole::Frame,
            SlotRole::Environment,
        ]
    );
    assert!(!slots[0].key.as_ref().expect("item").stutter_closure);
}

/// FR-490-AC-3: a package not selecting `quire.model.complete/v1` refuses
/// `unsupported_construct` at the declaration; a declaration with no `initial`
/// member and one with an unquoted Rust path refuse `invalid_syntax` at the
/// offending token.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-3")]
#[test]
fn the_declaration_form_refusals_are_located() {
    let without_model = unit(&format!("{LIVENESS}{RELATION}"), "", CASE_ONE).replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
    );
    let Err(Failure::Assembly(refusal)) = compile(&without_model) else {
        panic!("an assembly refusal");
    };
    let gate = refusal
        .errors
        .iter()
        .find_map(|error| match &error.cause {
            AssemblyCause::UnselectedModelConstruct(gate)
                if gate.family == ModelFamily::Guarantee =>
            {
                Some((gate, error.span))
            }
            _ => None,
        })
        .expect("the declaration is refused as an unselected model construct");
    assert_eq!(gate.0.family.refusal_cause(), "declaration-form");
    assert_eq!(
        gate.1.start,
        without_model.find("guarantee G").expect("the declaration")
    );
    assert_eq!(
        qsl_semantics::check::AssemblyCause::UnselectedModelConstruct(gate.0.clone()).code(),
        qsl_foundation::Code::UnsupportedConstruct
    );
    for (declaration, offending) in [
        ("guarantee G of Eventually2 using A { }", "}"),
        (
            "guarantee G of Eventually2 using A { initial to counter::Counter::new; }",
            "counter",
        ),
    ] {
        let source = unit(LIVENESS, RELATION, declaration);
        let Err(Failure::Syntax(diagnostics)) = compile(&source) else {
            panic!("{declaration}: a syntax refusal");
        };
        assert_eq!(diagnostics[0].code, qsl_cst::CompleteCode::InvalidSyntax);
        let span = diagnostics[0].byte_span().expect("located");
        assert_eq!(text(&source, span), offending, "{declaration}");
    }
}

/// FR-490-AC-4: `P` or `A` naming no declaration refuses `missing_declaration`
/// / `missing-name`; a finite-trace `P`, and `P` and `A` over different model
/// aliases, refuse `invalid_model_binding`/`malformed-declaration` naming `P`.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-4")]
#[test]
fn a_clause_or_relation_that_does_not_fit_refuses() {
    let missing_clause = refused(
        &unit(
            LIVENESS,
            RELATION,
            &CASE_ONE.replace("Eventually2", "Missing"),
        ),
        &[fixture::document(&["inc"])],
    );
    assert!(matches!(
        &missing_clause,
        GuaranteeRefusal::MissingName { what: GuaranteeName::Clause, name, .. } if name == "Missing"
    ));
    let missing_relation = refused(
        &unit(
            LIVENESS,
            RELATION,
            &CASE_ONE.replace("using A", "using Missing"),
        ),
        &[fixture::document(&["inc"])],
    );
    assert!(matches!(
        &missing_relation,
        GuaranteeRefusal::MissingName { what: GuaranteeName::Relation, name, .. } if name == "Missing"
    ));
    for refusal in [&missing_clause, &missing_relation] {
        assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
        assert_eq!(refusal.cause(), "missing-name");
    }

    // A finite-trace `P`: the unit selects the finite profile only.
    let finite = format!(
        "{}temporal Bounded using t over (c: Spec::Counter) clock \"model-steps\" on origin {{ \
         eventually[0,1] holds(c.value = 2) }}\n",
        HEADER.replace(
            "quire.temporal.infinite-trace/v1",
            "quire.temporal.event-position.false-extension/v1"
        )
    );
    let source = format!(
        "{finite}{RELATION}{}",
        CASE_ONE.replace("Eventually2", "Bounded")
    );
    let not_infinite = refused(&source, &[fixture::document(&["inc"])]);
    assert!(matches!(
        &not_infinite,
        GuaranteeRefusal::ClauseNotInfiniteTrace { clause, .. } if clause == "Bounded"
    ));

    // `P` over another model alias than `A`.
    let other = format!(
        "{HEADER}model Other = \"test/other\";\n\
         temporal Elsewhere using t over (c: Other::Counter) clock \"model-steps\" on origin {{ \
         always holds(c.value >= 0) }}\n{RELATION}{}",
        CASE_ONE.replace("Eventually2", "Elsewhere")
    );
    let mismatch = refused(
        &other,
        &[
            fixture::document(&["inc"]),
            fixture::other_document(&["inc"]),
        ],
    );
    assert!(matches!(
        &mismatch,
        GuaranteeRefusal::ModelAliasMismatch { clause, clause_alias, .. }
            if clause == "Elsewhere" && clause_alias == "Other"
    ));
    for refusal in [&not_infinite, &mismatch] {
        assert_eq!(refusal.code(), qsl_foundation::Code::InvalidModelBinding);
        assert_eq!(refusal.cause(), "malformed-declaration");
    }
}

/// FR-490-AC-5: a `progress` operation that is not an operation of the subject
/// refuses `missing_declaration`/`missing-name`; `bound 0` and the Rust path
/// `"9lives"` refuse `invalid_model_binding`/`malformed-declaration`; two
/// `progress` members for one operation refuse `invalid_model_binding`/
/// `conflicting-binding` naming both.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-5")]
#[test]
fn a_progress_member_that_does_not_fit_refuses() {
    let with = |members: &str| {
        let source = unit(
            LIVENESS,
            RELATION,
            &format!(
                "guarantee G of Eventually2 using A {{ initial to \"counter::Counter::new\"; {members} }}\n"
            ),
        );
        (
            source.clone(),
            refused(&source, &[fixture::document(&["inc"])]),
        )
    };
    let (_, no_operation) = with("progress Spec::Counter::dec bound 3;");
    assert!(matches!(
        &no_operation,
        GuaranteeRefusal::MissingName { what: GuaranteeName::Operation, name, .. }
            if name == "Spec::Counter::dec"
    ));
    assert_eq!(
        no_operation.code(),
        qsl_foundation::Code::MissingDeclaration
    );
    assert_eq!(no_operation.cause(), "missing-name");

    let (source, zero) = with("progress Spec::Counter::inc bound 0;");
    assert!(matches!(&zero, GuaranteeRefusal::InvalidBound { text, .. } if text == "0"));
    assert_eq!(
        text(&source, zero.span().expect("located")),
        "progress Spec::Counter::inc bound 0;"
    );
    let (_, lives) = with("progress Spec::Counter::inc decreases \"9lives\";");
    assert!(matches!(
        &lives,
        GuaranteeRefusal::RustPath { segment, spelling, .. }
            if segment == "9lives" && spelling == "9lives"
    ));
    for refusal in [&zero, &lives] {
        assert_eq!(refusal.code(), qsl_foundation::Code::InvalidModelBinding);
        assert_eq!(refusal.cause(), "malformed-declaration");
    }

    let (source, twice) = with(
        "progress Spec::Counter::inc bound 3; progress Spec::Counter::inc decreases \"counter::m\";",
    );
    let GuaranteeRefusal::ConflictingProgress { first, second, .. } = &twice else {
        panic!("a conflicting progress refusal: {twice:?}");
    };
    assert_eq!(
        text(&source, *first),
        "progress Spec::Counter::inc bound 3;"
    );
    assert_eq!(
        text(&source, *second),
        "progress Spec::Counter::inc decreases \"counter::m\";"
    );
    assert_eq!(twice.code(), qsl_foundation::Code::InvalidModelBinding);
    assert_eq!(twice.cause(), "conflicting-binding");

    let (_, two_environments) = with(
        "environment E by \"a\" fair Spec::Counter::inc; environment E by \"b\" fair Spec::Counter::inc;",
    );
    assert!(matches!(
        &two_environments,
        GuaranteeRefusal::ConflictingEnvironment { name, .. } if name == "E"
    ));
}

/// FR-490-AC-6: an operation of the model subject with no frame binding in `A`
/// refuses only the guarantee item `missing_declaration`/`missing-name`; the
/// package checks and its other requirement records are derived unchanged.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-6")]
#[test]
fn an_unbound_operation_refuses_only_the_guarantee_item() {
    let clauses = format!("{LIVENESS}{SAFETY}");
    let with = checked(&unit(&clauses, RELATION_WITHOUT_INC, CASE_ONE));
    let without = checked(&unit(&clauses, RELATION_WITHOUT_INC, ""));
    let guarantee = sole(&with);
    let refusal = guarantee.item_refusal().expect("the item refuses");
    assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
    assert_eq!(refusal.cause(), "missing-name");
    assert!(
        matches!(&refusal.element, AbstractionKey::Frame(key) if key.operation.as_str() == "inc")
    );
    assert_eq!(refusal.package(), "test/orders");
    assert_eq!(with.graph().requirements(), without.graph().requirements());
    // The bound relation leaves the item unrefused.
    let bound = checked(&unit(&clauses, RELATION, CASE_ONE));
    assert_eq!(sole(&bound).item_refusal(), None);
}

/// FR-490-AC-7: the v2 byte emitter writes a guarantee as one
/// `implementation_guarantee` node whose body holds the declaration's members
/// in canonical order, typed by itself, with one `declaration` occurrence,
/// and IR's reader admits the bytes.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-7")]
#[test]
fn the_guarantee_is_emitted_as_a_node_the_ir_reader_admits() {
    let source = unit(
        LIVENESS,
        RELATION_TWO,
        "guarantee G of Eventually2 using A { initial to \"counter::Counter::new\"; \
         progress Spec::Counter::inc decreases \"counter::retries_left\"; \
         progress Spec::Counter::dec bound 5; \
         environment Sched by \"scheduler\" fair weak each Spec::Counter::inc; \
         environment Net by \"network\" fair Spec::Counter::dec; }\n",
    );
    let document = fixture::document(&["inc", "dec"]);
    let package =
        compile_with(&source, std::slice::from_ref(&document)).expect("the unit compiles");
    let (wire, read) = emit_and_read(&package, &document);
    let [emitted] =
        super::owners::node_where(&wire, "correspondence", "implementation_guarantee")[..]
    else {
        panic!("one guarantee node");
    };
    let body = &emitted["body"];
    assert_eq!(body["term"], "implementation_guarantee");
    assert_eq!(body["initial"], json!(["counter", "Counter", "new"]));
    // `progress` ascends by operation, `environment` by name.
    let operations: Vec<&Value> = body["progress"]
        .as_array()
        .expect("progress")
        .iter()
        .map(|entry| &entry["operation"])
        .collect();
    assert_eq!(operations, ["dec", "inc"]);
    assert_eq!(body["progress"][0]["measure"], json!({"bound": 5}));
    assert_eq!(
        body["progress"][1]["measure"],
        json!({"decreases": ["counter", "retries_left"]})
    );
    let names: Vec<&Value> = body["environment"]
        .as_array()
        .expect("environment")
        .iter()
        .map(|entry| &entry["name"])
        .collect();
    assert_eq!(names, ["Net", "Sched"]);
    assert_eq!(emitted["semantic_type"], emitted["node_id"]);
    assert_eq!(
        emitted["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}])
    );
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}

/// FR-490-AC-8 (FR-529-AC-9): a unit holding the abstraction relation and no
/// `guarantee` derives exactly the items it derives without the relation, and
/// carries no guarantee.
#[trace("FR-529-AC-9", "QSpec-TC-984", "QSpec-FR-490-AC-8")]
#[test]
fn a_relation_alone_derives_nothing() {
    let clauses = format!("{LIVENESS}{SAFETY}");
    let with = checked(&unit(&clauses, RELATION, ""));
    let without = checked(&unit(&clauses, "", ""));
    assert!(with.graph().implementation_guarantees().is_empty());
    assert!(with.abstraction().is_some());
    assert!(without.abstraction().is_none());
    assert_eq!(with.graph().requirements(), without.graph().requirements());
}

/// FR-490-AC-9: the liveness declaration without its `progress` member
/// derives an unfilled no-divergence slot, and without its `environment`
/// member an unfilled environment slot naming `F_P`'s constraint; neither is
/// a refusal.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-9")]
#[test]
fn a_missing_progress_or_environment_member_leaves_an_unfilled_slot() {
    let no_progress = checked(&unit(
        LIVENESS,
        RELATION,
        "guarantee G of Eventually2 using A { initial to \"counter::Counter::new\"; \
         environment Sched by \"scheduler\" fair weak each Spec::Counter::inc; }\n",
    ));
    let slots = sole(&no_progress).slots("pkg");
    let divergence = slots
        .iter()
        .find(|slot| slot.role == SlotRole::NoDivergence)
        .expect("a no-divergence slot");
    assert_eq!(divergence.backing, SlotBacking::Unfilled);
    assert!(divergence.key.is_none());

    let no_environment = checked(&unit(
        LIVENESS,
        RELATION,
        "guarantee G of Eventually2 using A { initial to \"counter::Counter::new\"; \
         progress Spec::Counter::inc decreases \"counter::retries_left\"; }\n",
    ));
    let slots = sole(&no_environment).slots("pkg");
    let environment: Vec<&GuaranteeSlot> = slots
        .iter()
        .filter(|slot| slot.role == SlotRole::Environment)
        .collect();
    let [unfilled] = environment[..] else {
        panic!("one environment slot: {environment:?}");
    };
    assert_eq!(unfilled.backing, SlotBacking::Unfilled);
    assert_eq!(
        unfilled
            .constraint
            .as_ref()
            .map(|c| (c.kind(), c.granularity(), c.operation().name())),
        Some((FairnessKind::Weak, FairnessGranularity::Each, "inc"))
    );
}

/// QSpec FR-380 / FR-492 "Slot outcome": the first interval operator nested
/// under an unbounded one is named; a clause without one names none.
#[test]
fn the_fragment_operator_names_a_nested_interval_operator() {
    let nested = "temporal Nested using t over (c: Spec::Counter) clock \"model-steps\" \
         on origin { fair weak each Spec::Counter::inc; \
         always (holds(c.value >= 0) implies eventually always[0,1] holds(c.value = 2)) }\n";
    let source = unit(nested, RELATION, &CASE_ONE.replace("Eventually2", "Nested"));
    let package = checked(&source);
    assert_eq!(
        sole(&package).fragment_operator().as_deref(),
        Some("always[0,1]")
    );
    let plain = checked(&unit(LIVENESS, RELATION, CASE_ONE));
    assert_eq!(sole(&plain).fragment_operator(), None);
}
