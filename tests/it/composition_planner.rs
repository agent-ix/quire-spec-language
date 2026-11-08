// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1330 (FR-530-AC-1 to AC-8) and QSpec TC-983, TC-985 over real native
//! packages: `Producer` and `Consumer` are compiled as checked packages and
//! imported by the composing unit of QSpec TC-985 case 1. Each component
//! guarantee slot holds the FR-331 request item `qsl-route` writes for it and
//! a result entry for it; the supplied guarantees are the pool.

use std::sync::Arc;

use ix_trace_rs::trace;
use qsl_package::{
    emit_checked, read_import_view, AdmittedPackages, CheckedPackage, EmitLimits, Import,
    V2ReadLimits,
};
use qsl_replay::planner::{
    DeclarationRefusal, PlannerCause, PlannerDeclaration, PlannerResult, PlannerValue, SlotRole,
};
use qsl_replay::{Certification, InconclusiveCause, ItemLabel, PremiseOutcome};
use qsl_route::request::RequestWriter;
use qsl_route::{CandidateOutcome, Registry};
use qsl_semantics::check::{
    AdmittedImport, Capability, CheckRefusal, PackageDeclarations, SlotRole as SemRole,
};
use qsl_semantics::library::LibraryName;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use crate::planner_chain::{Chain, Failure, PACKAGE};

#[path = "../../qsl-semantics/tests/support/composition.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the semantics tests load the same fixture"
)]
mod fixture;

const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.model.complete/v1\";\n\
     profile t = \"quire.temporal.infinite-trace/v1\";\n\
     model M = \"test/orders\";\n";

const PRODUCER: &str = "temporal Sends using t over (p: M::Producer) clock \"model-steps\" \
     on origin { fair weak each M::Producer::send; always eventually holds(p.sent) }\n\
     temporal Counted using t over (p: M::Producer) clock \"model-steps\" \
     on origin { always holds(p.sent_count >= 0) }\n";

const CONSUMER: &str = "temporal Done using t over (c: M::Consumer) clock \"model-steps\" \
     on origin { always (holds(c.received) implies eventually holds(c.done)) }\n\
     temporal Bounded using t over (c: M::Consumer) clock \"model-steps\" \
     on origin { always holds(c.received_count >= 0) }\n";

const COMPONENTS: &str = "component Producer from P using M; component Consumer from Q using M; ";

/// QSpec TC-985 case 1's members.
const CASE_ONE_MEMBERS: &str = "guarantee Producer::Sends; guarantee Consumer::Done; \
     guarantee Producer::Counted; rely Consumer::Done on Producer::Counted; \
     depends Consumer::Done on Producer::Sends; supplies fair weak each M::Producer::send; ";

/// The slots of QSpec TC-985 case 1 and their positions.
const SENDS: usize = 0;
const DONE: usize = 1;
const COUNTED: usize = 2;
const RELY: usize = 3;
const DEPENDS: usize = 4;
const FAIRNESS: usize = 5;

struct Dependency {
    package: Arc<CheckedPackage>,
    import: AdmittedImport,
    identity: LibraryName,
}

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
    let emission = emit_checked(&package, EmitLimits::fixture()).expect("the dependency emits");
    let identity = LibraryName::new(identity).expect("a library identity");
    let view = read_import_view(
        &package,
        &emission,
        identity.clone(),
        &models,
        &mut AdmittedPackages::default(),
        V2ReadLimits::fixture(),
        EmitLimits::fixture(),
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

fn compile(
    declaration: &str,
    lattice: Option<&str>,
    dependencies: &[&Dependency],
) -> Result<CheckedPackage, Failure> {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         import \"test/producer\" as P;\n\
         import \"test/consumer\" as Q;\n{declaration}\n"
    );
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
        PackageDeclarations::assemble(raw, PACKAGE.to_owned(), built, models, imports)
            .map_err(Failure::Assembly)?;
    if let Some(profile) = lattice {
        assembled = assembled.with_composition_lattice(profile);
    }
    let graph = assembled
        .check(CheckingLimits::default())
        .map_err(|refusals: Vec<CheckRefusal>| Failure::Check(refusals))?;
    Ok(CheckedPackage::link_with(
        graph,
        dependencies
            .iter()
            .map(|dependency| Import {
                identity: dependency.identity.clone(),
                package: Arc::clone(&dependency.package),
            })
            .collect(),
        EmitLimits::fixture(),
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

    fn declare(&self, rule: &str, members: &str) -> String {
        format!("compose Sys rule {rule} {{ {COMPONENTS}{members}}}")
    }

    fn package(&self, rule: &str, members: &str, lattice: Option<&str>) -> CheckedPackage {
        let declaration = self.declare(rule, members);
        compile(&declaration, lattice, &[&self.producer, &self.consumer])
            .unwrap_or_else(|failure| panic!("{declaration}: the unit compiles: {failure:?}"))
    }

    fn chain(&self, rule: &str, members: &str, lattice: Option<&str>) -> Chain {
        let package = self.package(rule, members, lattice);
        let node = package.graph().compositions()[0].node().clone();
        Chain::of(package, &node).expect("the composition derives")
    }
}

fn invalid(pointer: &str) -> (&'static str, &'static str, String) {
    ("invalid_runtime_input", "invalid-value", pointer.to_owned())
}

fn premise(result: &PlannerResult) -> Option<(u64, PremiseOutcome)> {
    match &result.cause {
        Some(PlannerCause::Inconclusive(InconclusiveCause::PremiseUnproved {
            premise,
            result,
        })) => Some((*premise, *result)),
        _ => None,
    }
}

/// A premise result for the item of component-guarantee slot `slot`, proved.
fn pool_entry(chain: &Chain, slot: usize) -> Value {
    json!({"request": chain.item_json(slot), "result": chain.proved(slot)})
}

/// The positive request: every component guarantee proved, the `Counted` and
/// `Sends` guarantees supplied in the pool.
fn positive(chain: &Chain) -> Value {
    let mut request = chain.request(&|_| Value::Null);
    request["guarantees"] = json!([pool_entry(chain, COUNTED), pool_entry(chain, SENDS)]);
    request
}

// ---- FR-530-AC-3: the positive Producer/Consumer fixture -----------------------

/// QSpec TC-983 case 1, TC-985 case 1: relies, dependencies and fairness are
/// each discharged; the item settles `proved`, `decisive-witness`, with the
/// discharge record.
#[trace("TC-1330", "FR-530-AC-3")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-1")]
#[trace("QSpec-TC-985", "QSpec-FR-491-AC-1")]
#[test]
fn a_discharged_producer_consumer_composition_settles_proved_with_its_discharge_record() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS, None);
    assert_eq!(chain.declaration.slots().len(), 6);
    let result = chain.settled(&positive(&chain));
    assert_eq!(result.value, PlannerValue::Proved);
    assert_eq!(result.value.basis().as_str(), "decisive-witness");
    let discharge = result.discharge.as_ref().expect("a discharge record");
    let slots: Vec<usize> = discharge.entries.iter().map(|entry| entry.slot).collect();
    for slot in [RELY, DEPENDS, FAIRNESS] {
        assert!(slots.contains(&slot), "slot {slot} is discharged");
    }
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["value"], "proved");
    assert!(wire["discharge"].is_object());
}

// ---- FR-530-AC-2: a mismatched guarantee discharges nothing --------------------

#[trace("TC-1330", "FR-530-AC-2")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-2")]
#[test]
fn a_guarantee_of_another_subject_clause_kind_or_package_leaves_the_slot_undischarged() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS, None);
    let sends = pool_entry(&chain, SENDS);
    let counted = pool_entry(&chain, COUNTED);
    let done = pool_entry(&chain, DONE);
    let mut third = sends.clone();
    third["request"]["claim"]["node"]["package"] = json!("third@1");
    // The dependency's guarantee over `Consumer`, a weaker `Producer`
    // clause, a safety guarantee, and a same-named `Sends` of a third
    // package.
    let mut weaker = sends.clone();
    weaker["request"]["claim"] = counted["request"]["claim"].clone();
    for supplied in [done, weaker, counted.clone(), third] {
        let mut request = chain.request(&|_| Value::Null);
        request["guarantees"] = json!([counted, supplied]);
        let result = chain.settled(&request);
        assert_eq!(result.value, PlannerValue::Inconclusive);
        assert_eq!(
            premise(&result).map(|(slot, _)| slot),
            Some(DEPENDS as u64),
            "{supplied}"
        );
    }
}

// ---- FR-530-AC-4, AC-5: cycles ----------------------------------------------

const MUTUAL: &str = "depends Producer::Sends on Consumer::Done; ";

#[trace("TC-1330", "FR-530-AC-4")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-4", "QSpec-TC-985")]
#[test]
fn a_mutual_dependency_without_the_lattice_refuses_the_composition_item() {
    let world = World::new();
    let members = format!("{CASE_ONE_MEMBERS}{MUTUAL}");
    let package = world.package("assume-guarantee", &members, None);
    let composition = &package.graph().compositions()[0];
    let refusal = composition.item_refusal().expect("the item is refused");
    assert_eq!(refusal.code(), qsl_foundation::Code::InvalidModelBinding);
    assert_eq!(refusal.cause(), "malformed-declaration");
    let named: Vec<String> = refusal.cycles[0].iter().map(ToString::to_string).collect();
    assert_eq!(named, ["Consumer::Done", "Producer::Sends"]);
    assert_eq!(
        PlannerDeclaration::from_package(&package, composition.node()).err(),
        Some(DeclarationRefusal::ItemRefused {
            code: qsl_foundation::Code::InvalidModelBinding,
            cause: "malformed-declaration",
        })
    );
    // route writes no item for the refused composition.
    assert!(RequestWriter::new()
        .declaration_items(package.graph())
        .is_empty());
}

#[trace("TC-1330", "FR-530-AC-5")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-4", "QSpec-TC-985")]
#[test]
fn a_cycle_settles_proved_only_through_a_proved_fixed_point_result() {
    let world = World::new();
    let members = format!("{CASE_ONE_MEMBERS}{MUTUAL}");
    let chain = world.chain("assume-guarantee", &members, Some("fixed-point-lattice"));
    let slots = &chain.declaration.slots();
    let fixed = slots.len() - 1;
    assert_eq!(slots[fixed].role, SlotRole::FixedPoint);
    assert_eq!(slots[fixed].cycle, Some(0));
    let mut request = chain.request(&|_| Value::Null);
    request["guarantees"] = json!([pool_entry(&chain, COUNTED), pool_entry(&chain, SENDS)]);
    let proved = chain.settled(&request);
    assert_eq!(proved.value, PlannerValue::Proved);
    let discharge = proved.discharge.as_ref().expect("a discharge record");
    assert!(discharge.entries.iter().any(|entry| entry.slot == fixed));

    let mut inconclusive = request.clone();
    inconclusive["slots"][fixed]["result"]["result"] = chain.entry(
        fixed,
        json!({"result": "inconclusive", "cause": {"type": "induction-not-closed", "depth": 3}}),
    );
    let result = chain.settled(&inconclusive);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((fixed as u64, PremiseOutcome::Label(ItemLabel::Inconclusive)))
    );
}

// ---- FR-530-AC-6: trust, assumption, unsupported, failed, exhausted --------------

#[trace("TC-1330", "FR-530-AC-6")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-5")]
#[test]
fn each_outcome_keeps_its_own_trust_basis_and_extent() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS, None);

    // A `trusted` plugin discharger settles `proved`, `trusted`.
    let mut trusted = positive(&chain);
    let plugin = chain.entry(
        SENDS,
        json!({"result": "proved", "backend": "acme", "basis": {"type": "exhaustive"}, "certification": "trusted"}),
    );
    trusted["slots"][SENDS]["result"]["result"] = plugin;
    let read = chain.read(&trusted).expect("admitted");
    let settled = qsl_replay::planner::settle(
        &chain.package,
        &read,
        qsl_replay::planner::Budget::default(),
    )
    .expect("settles");
    assert_eq!(settled.value, PlannerValue::Proved);
    assert_eq!(settled.certification, Some(Certification::Trusted));

    // An assumption-only premise: the only discharger of the dependency is an
    // environment record.
    let members = format!(
        "{CASE_ONE_MEMBERS}environment Net by \"network\" fair weak each M::Producer::send; "
    );
    let assumed = world.chain("assume-guarantee", &members, None);
    let env = assumed.declaration.slots().len() - 1;
    assert_eq!(assumed.declaration.slots()[env].role, SlotRole::Environment);
    let context = match &assumed.declaration.environment()[0].supplies {
        qsl_replay::planner::EnvSupplies::Fairness(premise) => {
            qsl_foundation::node_wire::node_key_to_json(&premise.operation.context).expect("a node")
        }
        qsl_replay::planner::EnvSupplies::Assumption(_) => panic!("a scheduling member"),
    };
    let record = |_: &str| {
        json!({"name": "Net", "supplier": "network", "supplies": {"fairness": {
            "kind": "weak", "granularity": "each", "context": context, "operation": "send",
        }}})
    };
    let mut request = assumed.request(&record);
    request["guarantees"] = json!([pool_entry(&assumed, COUNTED)]);
    request["discharges"] = json!([{"slot": DEPENDS, "environment": env}]);
    let result = assumed.settled(&request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((DEPENDS as u64, PremiseOutcome::Assumption))
    );

    // A failed component check leaves its slot undischarged.
    let mut failed = positive(&chain);
    failed["slots"][SENDS]["result"]["result"] = chain.entry(
        SENDS,
        json!({"result": "inconclusive", "cause": {"type": "induction-not-closed", "depth": 3}}),
    );
    let result = chain.settled(&failed);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(premise(&result).map(|(slot, _)| slot), Some(SENDS as u64));

    // An exhausted budget.
    let mut exhausted = positive(&chain);
    exhausted["limits"] = json!({"max_work": 2});
    let result = chain.settled(&exhausted);
    assert_eq!(result.value, PlannerValue::Incomplete);
    assert_eq!(
        result.to_wire().expect("wire")["cause"]["limit"],
        "max_work"
    );

    // An unsupported rule: `interface` derives a FR-331 `composition` request
    // item, and with no composition-capable backend registered its candidate
    // set is empty (QSpec FR-348-AC-4).
    let package = world.package("interface", CASE_ONE_MEMBERS, None);
    let node = package.graph().compositions()[0].node().clone();
    assert_eq!(
        PlannerDeclaration::from_package(&package, &node).err(),
        Some(DeclarationRefusal::NotPlannerLocal("interface"))
    );
    let written = RequestWriter::new().declaration_items(package.graph());
    let own = written
        .iter()
        .find(|declared| declared.slot.is_none())
        .expect("the composition request item");
    assert_eq!(own.item.kind(), Capability::Composition);
    assert!(!own.item.premises().is_empty());
    assert!(own
        .item
        .premises()
        .iter()
        .any(|premise| premise.role() == SemRole::ComponentGuarantee));
    let CandidateOutcome::Candidates(candidates) =
        Registry::new().candidates(own.item.kind(), None)
    else {
        panic!("a computed candidate set");
    };
    assert!(candidates.is_empty(), "unsupported: no composition backend");
}

// ---- FR-530-AC-7: discharge records ---------------------------------------------

#[trace("TC-1330", "FR-530-AC-7")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-10")]
#[test]
fn a_discharge_record_is_admitted_against_the_pool_and_discharges_only_a_proved_result() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS, None);
    // Naming no pool entry.
    let mut missing = positive(&chain);
    missing["discharges"] = json!([{"slot": DEPENDS, "guarantee": 9}]);
    assert_eq!(chain.refusal(&missing), invalid("/discharges/0"));
    // A pool entry over another subject.
    let mut other = positive(&chain);
    other["guarantees"] = json!([pool_entry(&chain, COUNTED), pool_entry(&chain, DONE)]);
    other["discharges"] = json!([{"slot": DEPENDS, "guarantee": 1}]);
    assert_eq!(chain.refusal(&other), invalid("/discharges/0"));
    // An `inconclusive` result over the right subject is admitted and
    // discharges nothing.
    let mut weak = chain.request(&|_| Value::Null);
    let mut sends = pool_entry(&chain, SENDS);
    sends["result"] = chain.entry(
        SENDS,
        json!({"result": "inconclusive", "cause": {"type": "induction-not-closed", "depth": 3}}),
    );
    weak["guarantees"] = json!([pool_entry(&chain, COUNTED), sends]);
    weak["discharges"] = json!([{"slot": DEPENDS, "guarantee": 1}]);
    let result = chain.settled(&weak);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(premise(&result).map(|(slot, _)| slot), Some(DEPENDS as u64));
}

// ---- FR-530-AC-8: forged slots and foreign identities ---------------------------

#[trace("TC-1330", "FR-530-AC-8")]
#[trace("QSpec-TC-983", "QSpec-FR-492-AC-2", "QSpec-FR-492-AC-5")]
#[test]
fn a_forged_or_reordered_slot_and_a_result_for_another_identity_are_refused() {
    let world = World::new();
    let chain = world.chain("assume-guarantee", CASE_ONE_MEMBERS, None);
    let mut swapped = positive(&chain);
    swapped["slots"].as_array_mut().expect("slots").swap(0, 1);
    assert_eq!(chain.refusal(&swapped), invalid("/slots/0"));
    let mut forged = positive(&chain);
    forged["slots"][DONE]["identity"]["claim"]["node"]["package"] = json!("other@2");
    assert_eq!(chain.refusal(&forged), invalid("/slots/1"));
    let mut foreign = positive(&chain);
    foreign["slots"][SENDS]["result"] = foreign["slots"][COUNTED]["result"].clone();
    assert_eq!(chain.refusal(&foreign), invalid("/slots/0/result"));
    // The unrelated route items of the package are unchanged by the
    // declaration: its component-guarantee items are the only ones written.
    let written = RequestWriter::new().declaration_items(chain.package.graph());
    assert_eq!(written.len(), 3);
}
