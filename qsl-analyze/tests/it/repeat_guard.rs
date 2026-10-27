// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-228-AC-2 and TC-1268: the latest completed repeat-body attempt is
//! readable by the guard, whose first body pass is unconditional. Under TSO
//! flush fairness this load-until-seen protocol settles with a checked
//! component certificate.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_protocol, ProtocolCheckRequest};
use qsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use qsl_eval::simulation::model::ModelSimState;
use qsl_eval::simulation::protocol::MemorySelection;
use qsl_eval::temporal::AtomReading;
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::{MemoryModelName, SourceIdentity};
use qsl_replay::{
    compile_package, settle_protocol_check, Certification, DependencyInput, ModelCheckItem,
    ModelCheckLimits, ModelCheckOutcome, ProofBasis, ProofCertificate, ProtocolCertificateRequest,
    ReplayLimits, ResultLabel, StageLimits, TerminalValue,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, Meter};

use crate::litmus::{attempt, contracts, document, field, package, protocol, unit_text, Documents};
use crate::weak_memory::select;

const CLAIM: &str = "temporal Eventually using t0 over (k: M::Core) clock \"model-steps\" \
    on origin { eventually holds(k.r1 = 1) }\n";
const PRETEST_CLAIM: &str = "temporal NoLoad using t0 over (k: M::Core) clock \"model-steps\" \
    on origin { always holds(k.r1 = 3) }\n";

struct ReadsValue(i64);

impl ModelLetters for ReadsValue {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(if field(state, "cores", &instance.key, "r1") == self.0 {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

static READS_ONE: ReadsValue = ReadsValue(1);
static READS_THREE: ReadsValue = ReadsValue(3);

struct Fixture {
    body: String,
    checked: qsl_package::CheckedPackage,
    documents: Documents,
    memory: Vec<MemorySelection>,
    letters: &'static dyn ModelLetters,
}

impl Fixture {
    fn new() -> Self {
        Self::with_run(
            &format!(
                "parallel Both {{ branch w sequence W {{ {} }} \
             branch r repeat Poll by c visible (bLx) while {{ not bLx }} \
             sequence R {{ {} }} }} join all [w, r];",
                attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
                attempt("Lx", "c", "Core::look", "relaxed", "LooksX, Looked"),
            ),
            CLAIM,
            &READS_ONE,
        )
    }

    fn pretest() -> Self {
        let run = format!(
            "parallel Both {{ branch w sequence W {{ {} }} \
             branch r repeat Poll by c visible () while {{ false }} \
             sequence R {{ {} }} }} join all [w, r];",
            attempt("Sx", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
            attempt("Lx", "c", "Core::look", "relaxed", "LooksX, Looked"),
        );
        Self::with_run(&run, PRETEST_CLAIM, &READS_THREE)
    }

    fn with_run(run: &str, claim: &str, letters: &'static dyn ModelLetters) -> Self {
        let body = contracts()
            + "pre LooksX using v on M::Core::look { c.id = 0 }\n"
            + &protocol("SB", run)
            + claim;
        let checked = package(&body);
        Self {
            body,
            checked,
            documents: Documents::new(),
            memory: vec![select("SB::Both", MemoryModelName::Tso)],
            letters,
        }
    }

    fn subject(&self) -> qsl_eval::model_subject::ProtocolSubject<'_> {
        self.documents.subject(&self.checked, "SB", &self.memory)
    }

    fn item(&self) -> ModelCheckItem<'_> {
        ModelCheckItem::Clause {
            clause: self.checked.graph().temporal_clauses()[0].clause(),
            letters: self.letters,
        }
    }

    fn package_id(&self) -> DigestRecord {
        compile_package(
            SourceIdentity::new("agent-ix", "litmus", "git", "1"),
            "litmus.native",
            unit_text(&self.body).as_bytes(),
            [document().as_slice()],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("the unit compiles")
        .package_id()
    }
}

#[trace("TC-1268", "FR-228-AC-2")]
#[test]
fn a_repeat_guard_reads_its_latest_attempt_and_settles_certified() {
    let fixture = Fixture::new();
    let outcome = check_protocol(
        ProtocolCheckRequest {
            subject: fixture.subject(),
            item: fixture.item(),
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &Cancel::new(),
    )
    .expect("the checked request runs");
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Component(certificate),
        ..
    } = &outcome
    else {
        panic!("expected the fair load-until-seen proof, got {outcome:?}");
    };
    let request = ProtocolCertificateRequest {
        subject: fixture.subject(),
        item: fixture.item(),
        max_store_buffer: None,
        max_messages: None,
        max_live_instances: None,
    };
    assert_eq!(
        qsl_replay::check_protocol_components(
            &request,
            certificate,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the certificate checks"),
        qsl_replay::TemporalCertificateCheck::Accepted
    );

    let record = settle_protocol_check(
        &outcome,
        &ModelCheckLimits::default(),
        &fixture.package_id(),
        &request,
        &Cancel::new(),
        |_| unreachable!("a proved outcome does not replay a counterexample"),
    );
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
    assert_eq!(record.label, ResultLabel::Proved);

    let pretest = Fixture::pretest();
    let outcome = check_protocol(
        ProtocolCheckRequest {
            subject: pretest.subject(),
            item: pretest.item(),
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &Cancel::new(),
    )
    .expect("the unchanged pre-test control checks");
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "a false pre-test guard must skip the body load: {outcome:?}"
    );
}
