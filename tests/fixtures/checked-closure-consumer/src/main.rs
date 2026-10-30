// SPDX-License-Identifier: AGPL-3.0-or-later
//! A CG-shaped external caller enters QSL only through qsl-replay.

use qsl_replay::{
    acquire_checked_closure, call_site, ApplicationEnvironment, DependencyInput, Identifier,
    QualifiedName, ReplayLimits, SourceIdentity, StageLimits,
};
use quire_eval::{
    Admission, Body, BodyFactory, BodyFailure, Evaluation, FamilyOutcome, Resume, Transition,
};
use quire_exact::{Integer, Meter, Outcome, ScalarLimits, Value};
use quire_semantic_value::object_closure::ObjectClosure;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction f using v(x: Int[0, 9]): Integer pure { x }\n";

struct Generated;
struct GeneratedCall;

impl BodyFactory for Generated {
    fn begin(&self) -> Box<dyn Body> {
        Box::new(GeneratedCall)
    }
}

impl Body for GeneratedCall {
    fn resume(&mut self, input: Resume, _meter: &mut Meter) -> Result<Transition, BodyFailure> {
        match input {
            Resume::Start(_) => Ok(Transition::Done(Evaluation {
                outcome: FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(
                    Integer::from(7_u64),
                ))),
                location: None,
                losses: vec![],
            })),
            Resume::Again | Resume::Child(_) => {
                unreachable!("the generated body finishes on Start")
            }
        }
    }
}

/// Exercise the complete public facade path as an external consumer.
pub fn run() {
    let source = SourceIdentity::new("a", "consumer", "git", "1");
    let name = QualifiedName::new(vec![Identifier::new("f").expect("identifier")]).expect("name");
    let site = call_site(
        source.clone(),
        "consumer.native",
        SOURCE.as_bytes(),
        [],
        &DependencyInput::default(),
        &name,
    )
    .expect("function site");
    let compilation = acquire_checked_closure(
        qsl_replay::CompileUnit {
            source: source,
            package_name: "consumer.native",
            path: SOURCE.as_bytes(),
            bytes: [],
        },
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("checked closure");
    assert_eq!(site.package_id, compilation.package().package_id());
    let selection = compilation
        .handle()
        .lookup_callable(site.package_id, site.site.function)
        .expect("callable selection");
    let mut registry = compilation.handle().registry();
    assert_eq!(
        registry.register(&selection, selection.signature(), Generated),
        Ok(Admission::Accepted)
    );
    let environment = ApplicationEnvironment::new(ObjectClosure::default());
    let limit = 1000;
    let mut meter = Meter::new(ScalarLimits {
        integer_bits: limit,
        decimal_digits: limit,
        scale_expansion: limit,
        text_input_bytes: limit,
        text_scalars: limit,
        normalized_scalars: limit,
        unit_edges: limit,
        value_occurrences: limit,
        work_units: limit,
        result_units: limit,
    });
    let session = registry
        .begin_call(
            &selection,
            vec![Value::Integer(Integer::from(3_u64))],
            &environment,
            &mut meter,
        )
        .expect("admitted call");
    let result = match session.step() {
        qsl_replay::CheckedApplicationProgress::Runnable(next) => next.run().expect("run"),
        qsl_replay::CheckedApplicationProgress::Terminal(result) => result.expect("terminal"),
    };
    assert!(matches!(result.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(ref n))) if n.to_u64() == Some(7)));
}

#[cfg(not(test))]
fn main() {
    run();
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    /// The independently compiled consumer exercises the public facade path.
    #[trace("TC-1263", "FR-485-AC-3")]
    #[test]
    fn checked_closure_external_consumer_runs() {
        super::run();
    }
}
