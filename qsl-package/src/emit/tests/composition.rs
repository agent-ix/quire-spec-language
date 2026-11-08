// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-530-AC-1 (the compile, read-back and lists-separate part), QSpec
//! TC-985 (FR-491-AC-1 to AC-6): a `compose` declaration over the checked
//! `Producer` and `Consumer` packages of TC-983 is compiled from real source
//! through S1, S2 and S3 against two imported checked packages, its derived
//! item and slots are read, its node is written and read back, and every
//! refusal carries QSpec FR-491's code and cause.

use std::sync::Arc;

use ix_trace_rs::trace;
use qsl_semantics::check::{
    AbstractionRefusal, AdmittedImport, CheckCause, CheckRefusal, CheckedComposition,
    CompositionName, CompositionRefusal, CompositionRule, CompositionSlot, CompositionSlotKey,
    ItemPlacement, MalformedMember, SlotBacking, SlotRole,
};
use qsl_semantics::library::LibraryName;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use super::*;

#[path = "../../../../qsl-semantics/tests/support/composition.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the semantics tests load the same fixture"
)]
mod fixture;

const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.model.complete/v1\";\n\
     profile t = \"quire.temporal.infinite-trace/v1\";\n\
     model M = \"test/orders\";\n";

/// `Producer`'s package: the liveness guarantee `Sends` under its fairness
/// and the safety guarantee `Counted`.
const PRODUCER: &str = "temporal Sends using t over (p: M::Producer) clock \"model-steps\" \
     on origin { fair weak each M::Producer::send; always eventually holds(p.sent) }\n\
     temporal Counted using t over (p: M::Producer) clock \"model-steps\" \
     on origin { always holds(p.sent_count >= 0) }\n";

/// `Consumer`'s package: the liveness guarantee `Done` and the safety
/// guarantee `Bounded`.
const CONSUMER: &str = "temporal Done using t over (c: M::Consumer) clock \"model-steps\" \
     on origin { always (holds(c.received) implies eventually holds(c.done)) }\n\
     temporal Bounded using t over (c: M::Consumer) clock \"model-steps\" \
     on origin { always holds(c.received_count >= 0) }\n";

const COMPONENTS: &str = "component Producer from P using M; component Consumer from Q using M; ";

/// TC-985 case 1's members.
const CASE_ONE_MEMBERS: &str = "guarantee Producer::Sends; guarantee Consumer::Done; \
     guarantee Producer::Counted; rely Consumer::Done on Producer::Counted; \
     depends Consumer::Done on Producer::Sends; supplies fair weak each M::Producer::send; ";

/// Why a unit does not compile to a checked package.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "the refusal is read through `Debug` in a failure message"
)]
enum Failure {
    Syntax(Vec<qsl_cst::CompleteDiagnostic>),
    Assembly(qsl_semantics::check::AssemblyRefusal),
    Check(Vec<CheckRefusal>),
}

/// A checked dependency package and the import that names it.
struct Dependency {
    package: Arc<CheckedPackage>,
    import: AdmittedImport,
    identity: LibraryName,
}

/// Compiles `clauses` over the fixture model as the library `identity`.
fn dependency(identity: &str, clauses: &str) -> Dependency {
    let unit = format!("{MODEL_HEADER}{clauses}");
    let bytes = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let models =
        qsl_semantics::model::intake::package_input([bytes.as_slice()]).expect("the model package");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", identity),
        "dependency.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the dependency");
    assert!(parsed.is_admissible(), "{unit}: {:?}", parsed.diagnostics());
    let raw = parsed.source().reference().clone();
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the dependency");
    let selected = qsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &models,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model is admitted");
    let declarations =
        PackageDeclarations::assemble(raw, identity.to_owned(), built, selected, Vec::new())
            .expect("the dependency assembles");
    let package = Arc::new(CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the dependency checks"),
    ));
    let emission = emit_checked(&package).expect("the dependency emits");
    let identity = LibraryName::new(identity).expect("a library identity");
    let view = crate::checked_v2::read_import_view(
        &package,
        &emission,
        identity.clone(),
        &models,
        &mut crate::checked_v2::AdmittedPackages::default(),
        crate::checked_v2::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
    )
    .expect("the dependency import view is admitted");
    Dependency {
        import: AdmittedImport {
            identity: identity.clone(),
            view,
            graph: package.shared_graph(),
        },
        package,
        identity,
    }
}

/// The composing unit: imports `P` and `Q` and holds `declaration`.
fn unit(declaration: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         import \"test/producer\" as P;\n\
         import \"test/consumer\" as Q;\n{declaration}\n"
    )
}

fn compile_with(
    source: &str,
    lattice: Option<&str>,
    dependencies: &[&Dependency],
) -> Result<CheckedPackage, Failure> {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:composition"),
        "program.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        return Err(Failure::Syntax(parsed.diagnostics().to_vec()));
    }
    let raw = parsed.source().reference().clone();
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let imports: Vec<AdmittedImport> = dependencies
        .iter()
        .map(|dependency| dependency.import.clone())
        .collect();
    let no_models = qsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("an empty model package input");
    let models = qsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &no_models,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("an empty model selection is admitted");
    let mut assembled =
        PackageDeclarations::assemble(raw, "pkg".to_owned(), built, models, imports)
            .map_err(Failure::Assembly)?;
    if let Some(profile) = lattice {
        assembled = assembled.with_composition_lattice(profile);
    }
    let graph = assembled
        .check(CheckingLimits::default())
        .map_err(Failure::Check)?;
    Ok(CheckedPackage::link_with(
        graph,
        dependencies
            .iter()
            .map(|dependency| crate::checked::Import {
                identity: dependency.identity.clone(),
                package: Arc::clone(&dependency.package),
            })
            .collect(),
        crate::EmitLimits::fixture(),
    )
    .expect("the dependency closure links"))
}

struct World {
    producer: Dependency,
    consumer: Dependency,
}

impl World {
    fn new() -> Self {
        Self {
            producer: dependency("test/producer", PRODUCER),
            consumer: dependency("test/consumer", CONSUMER),
        }
    }

    fn compile(&self, declaration: &str, lattice: Option<&str>) -> Result<CheckedPackage, Failure> {
        compile_with(
            &unit(declaration),
            lattice,
            &[&self.producer, &self.consumer],
        )
    }

    /// `compose Sys rule rule { COMPONENTS members }`.
    fn declare(&self, rule: &str, members: &str) -> String {
        format!("compose Sys rule {rule} {{ {COMPONENTS}{members}}}")
    }

    fn checked(&self, rule: &str, members: &str) -> CheckedPackage {
        let declaration = self.declare(rule, members);
        self.compile(&declaration, None)
            .unwrap_or_else(|failure| panic!("{declaration}: the unit compiles: {failure:?}"))
    }

    /// The one composition refusal of `declaration`.
    fn refused(&self, declaration: &str) -> CompositionRefusal {
        let Err(Failure::Check(refusals)) = self.compile(declaration, None) else {
            panic!("{declaration}: a check refusal");
        };
        let [CheckRefusal {
            cause: CheckCause::Abstraction(cause),
            ..
        }] = &refusals[..]
        else {
            panic!("{declaration}: one composition refusal: {refusals:?}");
        };
        let AbstractionRefusal::Composition(refusal) = &**cause else {
            panic!("{declaration}: a composition refusal: {cause:?}");
        };
        (**refusal).clone()
    }
}

fn sole(package: &CheckedPackage) -> &CheckedComposition {
    let [composition] = package.graph().compositions() else {
        panic!("one composition");
    };
    composition
}

fn roles(slots: &[CompositionSlot]) -> Vec<SlotRole> {
    slots.iter().map(|slot| slot.role).collect()
}

fn text(unit: &str, span: qsl_foundation::Span) -> &str {
    &unit[span.start..span.end]
}

/// FR-491-AC-1 (TC-985 case 1): three guarantees, one `rely`, one `depends`
/// and one `supplies` member derive the planner-local item and, in order,
/// three component-guarantee slots each keyed with its FR-176 assumption set,
/// one rely slot, one liveness-dependency slot and one fairness slot, the
/// four lists kept separate and each slot's span its member.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-1")]
#[test]
fn a_producer_consumer_declaration_derives_its_slots_in_order() {
    let world = World::new();
    let source = unit(&world.declare("assume-guarantee", CASE_ONE_MEMBERS));
    let package = world
        .compile(&world.declare("assume-guarantee", CASE_ONE_MEMBERS), None)
        .expect("the unit compiles");
    let composition = sole(&package);
    assert_eq!(composition.item_refusal(), None);
    assert_eq!(composition.rule(), CompositionRule::AssumeGuarantee);
    let item = composition.item();
    assert_eq!(item.placement, ItemPlacement::PlannerLocal);
    assert_eq!(item.node, *composition.node());
    assert_eq!(item.subjects.len(), 2);
    let slots = composition.slots();
    assert_eq!(item.premises, slots);
    assert_eq!(
        roles(&slots),
        [
            SlotRole::ComponentGuarantee,
            SlotRole::ComponentGuarantee,
            SlotRole::ComponentGuarantee,
            SlotRole::Rely,
            SlotRole::LivenessDependency,
            SlotRole::Fairness,
        ]
    );
    // The four lists stay separate.
    assert_eq!(composition.relies().len(), 1);
    assert_eq!(composition.liveness_dependencies().len(), 1);
    assert!(composition.environment().is_empty());
    assert_eq!(composition.guarantees().len(), 3);
    // Each component-guarantee slot is keyed with its assumption set.
    let assumptions = |slot: &CompositionSlot| match &slot.key {
        Some(CompositionSlotKey::ComponentGuarantee(key)) => key.assumptions.len(),
        other => panic!("a component-guarantee key: {other:?}"),
    };
    assert_eq!(
        [
            assumptions(&slots[0]),
            assumptions(&slots[1]),
            assumptions(&slots[2])
        ],
        [0, 2, 0],
        "Sends and Counted assume nothing, Done assumes both peers"
    );
    for slot in &slots[..3] {
        assert_eq!(slot.backing, SlotBacking::Item);
    }
    assert_eq!(slots[3].backing, SlotBacking::PlannerLocal);
    assert_eq!(slots[5].backing, SlotBacking::PlannerLocal);
    let fairness = slots[5].constraint.as_ref().expect("the fairness premise");
    assert_eq!(
        (
            fairness.kind(),
            fairness.granularity(),
            fairness.operation().name()
        ),
        (FairnessKind::Weak, FairnessGranularity::Each, "send")
    );
    for (index, slot) in slots.iter().enumerate() {
        assert_eq!(slot.index, index);
        for other in &slots[..index] {
            assert_ne!(slot, other, "slot {index} is distinct");
        }
    }
    // Each slot's span is the member that produced it.
    let sources: Vec<&str> = slots
        .iter()
        .map(|slot| text(&source, slot.source))
        .collect();
    assert_eq!(sources[0], "guarantee Producer::Sends;");
    assert_eq!(sources[1], "guarantee Consumer::Done;");
    assert_eq!(sources[2], "guarantee Producer::Counted;");
    assert_eq!(sources[3], "rely Consumer::Done on Producer::Counted;");
    assert_eq!(sources[4], "depends Consumer::Done on Producer::Sends;");
    assert_eq!(sources[5], "guarantee Producer::Sends;");
    assert_eq!(
        composition
            .supplies()
            .iter()
            .map(|s| s.operation.as_str())
            .collect::<Vec<_>>(),
        ["send"]
    );
}

/// FR-491-AC-2 (TC-985 case 2): one `component`, an unknown rule spelling and
/// a member of another form refuse `invalid_syntax`; an unknown import alias,
/// subject or clause refuses `missing_declaration`/`missing-name`; two
/// components of one name refuse `invalid_model_binding`/`conflicting-binding`.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-2")]
#[test]
fn a_declaration_that_does_not_fit_its_packages_refuses() {
    let world = World::new();
    for (declaration, offending) in [
        (
            "compose Sys rule interface { component Producer from P using M; guarantee Producer::Sends; }",
            "guarantee",
        ),
        (
            "compose Sys rule cooperative { component Producer from P using M; component Consumer from Q using M; }",
            "cooperative",
        ),
    ] {
        let source = unit(declaration);
        let Err(Failure::Syntax(diagnostics)) = world.compile(declaration, None) else {
            panic!("{declaration}: a syntax refusal");
        };
        assert_eq!(diagnostics[0].code, qsl_cst::CompleteCode::InvalidSyntax);
        let span = diagnostics[0].byte_span().expect("located");
        assert_eq!(text(&source, span), offending, "{declaration}");
    }
    let refuse = |declaration: String| world.refused(&declaration);
    let missing = |refusal: &CompositionRefusal| match refusal {
        CompositionRefusal::MissingName { what, name, .. } => (*what, name.clone()),
        other => panic!("a missing name: {other:?}"),
    };
    let unknown_import = refuse(
        "compose Sys rule interface { component Producer from Z using M; \
         component Consumer from Q using M; }"
            .to_owned(),
    );
    assert_eq!(
        missing(&unknown_import),
        (CompositionName::Import, "Z".to_owned())
    );
    let unknown_subject = refuse(
        "compose Sys rule interface { component Producer from P using X; \
         component Consumer from Q using M; }"
            .to_owned(),
    );
    assert_eq!(
        missing(&unknown_subject),
        (CompositionName::Model, "X".to_owned())
    );
    let unknown_clause = refuse(world.declare("interface", "guarantee Producer::Missing; "));
    assert_eq!(
        missing(&unknown_clause),
        (CompositionName::Clause, "Missing".to_owned())
    );
    let unknown_component = refuse(world.declare("interface", "guarantee Nobody::Sends; "));
    assert_eq!(
        missing(&unknown_component),
        (CompositionName::Component, "Nobody".to_owned())
    );
    // A clause of the other component's package is no clause of this one.
    let other_package = refuse(world.declare("interface", "guarantee Producer::Done; "));
    assert_eq!(
        missing(&other_package),
        (CompositionName::Clause, "Done".to_owned())
    );
    for refusal in [&unknown_import, &unknown_subject, &unknown_clause] {
        assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
        assert_eq!(refusal.cause(), "missing-name");
    }
    let same_name = refuse(
        "compose Sys rule interface { component Producer from P using M; \
         component Producer from Q using M; }"
            .to_owned(),
    );
    assert!(matches!(
        &same_name,
        CompositionRefusal::ConflictingComponent { name, .. } if name == "Producer"
    ));
    assert_eq!(same_name.code(), qsl_foundation::Code::InvalidModelBinding);
    assert_eq!(same_name.cause(), "conflicting-binding");
    let same_subject = refuse(
        "compose Sys rule interface { component Producer from P using M; \
         component Other from P using M; }"
            .to_owned(),
    );
    assert!(matches!(
        &same_subject,
        CompositionRefusal::ConflictingSubject { first_component, second_component, .. }
            if first_component == "Producer" && second_component == "Other"
    ));
    assert_eq!(same_subject.cause(), "conflicting-binding");
}

/// FR-491-AC-3 (TC-985 case 3): a `rely` on a liveness guarantee, a `depends`
/// on a safety guarantee, a `rely` whose guarantee is undeclared and a `rely`
/// of a component on itself each refuse `invalid_model_binding`/
/// `malformed-declaration` naming the member.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-3")]
#[test]
fn a_premise_that_does_not_fit_its_kind_refuses() {
    let world = World::new();
    let malformed = |members: &str| {
        let declaration = world.declare("interface", members);
        let refusal = world.refused(&declaration);
        assert_eq!(refusal.code(), qsl_foundation::Code::InvalidModelBinding);
        assert_eq!(refusal.cause(), "malformed-declaration");
        match refusal {
            CompositionRefusal::Malformed {
                reason,
                member,
                span,
            } => {
                let written = text(&unit(&declaration), span).to_owned();
                (reason, member, written)
            }
            other => panic!("a malformed member: {other:?}"),
        }
    };
    let declared = "guarantee Producer::Sends; guarantee Consumer::Done; \
                    guarantee Producer::Counted; ";
    let (reason, member, written) = malformed(&format!(
        "{declared}rely Consumer::Done on Producer::Sends; "
    ));
    assert_eq!(reason, MalformedMember::RelyOnLiveness);
    assert_eq!(member, "rely Consumer::Done on Producer::Sends");
    assert_eq!(written, "rely Consumer::Done on Producer::Sends;");
    let (reason, member, _) = malformed(&format!(
        "{declared}depends Consumer::Done on Producer::Counted; "
    ));
    assert_eq!(reason, MalformedMember::DependsNotLiveness);
    assert_eq!(member, "depends Consumer::Done on Producer::Counted");
    let (reason, member, _) = malformed(
        "guarantee Producer::Sends; guarantee Producer::Counted; \
         rely Consumer::Done on Producer::Counted; ",
    );
    assert_eq!(reason, MalformedMember::UndeclaredGuarantee);
    assert_eq!(member, "rely Consumer::Done on Producer::Counted");
    let (reason, member, _) = malformed(&format!(
        "{declared}rely Producer::Sends on Producer::Counted; "
    ));
    assert_eq!(reason, MalformedMember::SelfComponent);
    assert_eq!(member, "rely Producer::Sends on Producer::Counted");
}

/// FR-491-AC-4 (TC-985 case 4): a mutual `depends` pair with no selected
/// profile declaring FR-176's lattice checks and its composition item refuses
/// `invalid_model_binding`/`malformed-declaration` naming both guarantees;
/// under a profile declaring the lattice the pair derives one fixed-point
/// slot.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-4")]
#[test]
fn a_mutual_dependency_is_a_cycle_the_item_settles() {
    let world = World::new();
    let members = format!("{CASE_ONE_MEMBERS}depends Producer::Sends on Consumer::Done; ");
    let without = world.checked("assume-guarantee", &members);
    let composition = sole(&without);
    let refusal = composition
        .item_refusal()
        .expect("the composition item refuses");
    assert_eq!(refusal.code(), qsl_foundation::Code::InvalidModelBinding);
    assert_eq!(refusal.cause(), "malformed-declaration");
    let [cycle] = &refusal.cycles[..] else {
        panic!("one cycle: {:?}", refusal.cycles);
    };
    let named: Vec<String> = cycle.iter().map(ToString::to_string).collect();
    assert_eq!(named, ["Consumer::Done", "Producer::Sends"]);
    assert!(!roles(&composition.slots()).contains(&SlotRole::FixedPoint));
    // The declaration is not refused: the package's other facts are those of
    // the same unit without the cycle.
    let acyclic = world.checked("assume-guarantee", CASE_ONE_MEMBERS);
    assert_eq!(
        without.graph().requirements(),
        acyclic.graph().requirements()
    );

    let declaration = world.declare("assume-guarantee", &members);
    let with = world
        .compile(&declaration, Some("fixed-point-lattice"))
        .expect("the unit compiles");
    let composition = sole(&with);
    assert_eq!(composition.item_refusal(), None);
    let slots = composition.slots();
    let [.., fixed_point] = &slots[..] else {
        panic!("slots");
    };
    assert_eq!(fixed_point.role, SlotRole::FixedPoint);
    assert_eq!(fixed_point.backing, SlotBacking::Item);
    let Some(CompositionSlotKey::FixedPoint {
        node,
        premises,
        profile,
        ..
    }) = &fixed_point.key
    else {
        panic!("a fixed-point key: {:?}", fixed_point.key);
    };
    assert_eq!(node, composition.node());
    assert_eq!(profile, "fixed-point-lattice");
    assert_eq!(premises.len(), 2);
    assert_eq!(
        roles(&slots)
            .iter()
            .filter(|role| **role == SlotRole::FixedPoint)
            .count(),
        1
    );
}

/// FR-491-AC-5 (TC-985 case 5): under `interface` the declaration derives
/// one FR-331 `composition` request item whose `premises` member holds the
/// slots, and no planner-local item.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-5")]
#[test]
fn a_non_assume_guarantee_rule_derives_a_request_item() {
    let world = World::new();
    let local = world.checked("assume-guarantee", CASE_ONE_MEMBERS);
    for rule in ["interface", "protocol-refinement", "refinement-transfer"] {
        let package = world.checked(rule, CASE_ONE_MEMBERS);
        let item = sole(&package).item();
        assert_eq!(item.placement, ItemPlacement::Request, "{rule}");
        let keys = |slots: &[CompositionSlot]| -> Vec<_> {
            slots
                .iter()
                .map(|slot| (slot.role, slot.key.clone()))
                .collect()
        };
        assert_eq!(keys(&item.premises), keys(&sole(&local).slots()), "{rule}");
    }
    assert_eq!(sole(&local).item().placement, ItemPlacement::PlannerLocal);
}

/// FR-491-AC-6 (TC-985 case 6): the v2 byte emitter refuses a package holding
/// a composition, typed, because its component clauses are foreign nodes with
/// operands that IR's reader cannot yet admit.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-6")]
#[test]
fn a_composition_is_refused_for_its_foreign_component_operands() {
    let world = World::new();
    let package = world.checked("assume-guarantee", CASE_ONE_MEMBERS);
    assert_eq!(
        emit_checked(&package).expect_err("the emission refuses"),
        EmitRefusal::CompositionForeignOperands {
            name: Some("Sys".to_owned())
        }
    );
}
