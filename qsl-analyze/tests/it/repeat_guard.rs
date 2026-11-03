// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1268: entry-readable binders keep repeat guards pre-test. A guard that
//! reads a body binder would require post-test semantics, which the current
//! checked-package writer refuses explicitly.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_protocol, ProtocolCheckRequest};
use qsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use qsl_eval::simulation::model::ModelSimState;
use qsl_eval::simulation::protocol::{MemorySelection, StepKind};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::MemoryModelName;
use qsl_replay::{ModelCheckItem, ModelCheckLimits, ModelCheckOutcome};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, Meter};

use crate::fill::{projections, reachable, sc_system};
use crate::litmus::{attempt, contracts, field, package, protocol, Documents};
use crate::weak_memory::select;

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

static READS_THREE: ReadsValue = ReadsValue(3);

struct Fixture {
    checked: qsl_package::CheckedPackage,
    documents: Documents,
    memory: Vec<MemorySelection>,
    letters: &'static dyn ModelLetters,
}

impl Fixture {
    /// The pre-test fixture whose guard reads `ready`, the binder of an
    /// event that precedes the repeat (so is readable at its entry) and is
    /// constrained false: a guard that
    /// reads no body binder stays pre-test, and as it is false at entry the
    /// body load never runs (SR-1737002 FND-001).
    fn entry_guard() -> Self {
        let run = format!(
            "parallel Both {{ branch w sequence W {{ {} }} \
             branch r sequence Rr {{ event Ready by c as (ready: Boolean) {{ not ready }}; \
             repeat Poll by c visible (ready) while {{ ready }} sequence R {{ {} }} }} }} \
             join all [w, r];",
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
}

/// FR-218-AC-33 / TC-1268 and QSpec FR-228-AC-2: the repeat's body runs once
/// before its guard reads the body binder. The false binder then exits the
/// repeat, producing one `Ready` event and no second body iteration.
#[trace("TC-1268", "FR-218-AC-33", "QSpec-FR-228-AC-2")]
#[test]
fn a_repeat_guard_over_its_body_binder_runs_the_body_before_testing() {
    let run = format!(
        "sequence Main {{ repeat Poll by c visible (ready) max 2 while {{ ready }} \
         sequence Body {{ {} event Ready by c as (ready: Boolean) {{ false }}; }} \
         exhausted sequence Done {{ }} }}",
        attempt("Store", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
    );
    let fixture = Fixture::with_run(&run, PRETEST_CLAIM, &READS_THREE);
    let system = sc_system(&fixture.subject());
    let reached = reachable(&system);

    assert_eq!(
        projections(&reached),
        [["Store", "Ready", "Done"]]
            .into_iter()
            .map(|steps| steps.into_iter().map(str::to_owned).collect())
            .collect()
    );
    let ready_events = reached
        .iter()
        .flat_map(|state| &state.steps)
        .filter(|(transition, _)| transition.kind() == StepKind::Event)
        .count();
    assert_eq!(
        ready_events, 1,
        "the false post-test guard exits after one body"
    );
}

/// SR-1737002 FND-001: a repeat whose guard reads only a binder readable at
/// its entry is pre-test. Its guard is false on entry, so the body's load
/// never runs and `r1` keeps its initial value on every path.
#[trace("TC-1268", "FR-228-AC-2")]
#[test]
fn a_guard_over_an_entry_binder_is_pre_test_and_skips_the_body() {
    let fixture = Fixture::entry_guard();
    let outcome = check_protocol(
        ProtocolCheckRequest {
            temporal_clock_selections: &[],
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
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "a false entry guard must skip the body load: {outcome:?}"
    );
}
