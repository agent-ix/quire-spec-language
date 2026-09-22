// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared setup for the native temporal evaluation controls.
//!
//! Compiles real native temporal source through the existing parser, linker,
//! checker and protocol-artifact emitter, reads the emitted bytes back through
//! the independent reader, and hands the test an `AdmittedPackage`. Admission is
//! the constructor-private evidence the evaluator requires; no test constructs a
//! wire package directly.

use crate::support::native_protocol as protocol;

use std::collections::BTreeMap;

use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{self as artifact, native, Limits};
use quire_spec_language::temporal;

pub use protocol::{Inputs, Unit};

/// Authored profile aliases in the shared fixture preamble.
pub const EVENT_POSITION: &str = "T";
pub const FIXED_SAMPLE: &str = "F";
pub const TIMESTAMPED_WINDOW: &str = "W";

/// The registered identity each alias selects.
pub fn identity(alias: &str) -> &'static str {
    match alias {
        EVENT_POSITION => temporal::EVENT_POSITION,
        FIXED_SAMPLE => temporal::FIXED_SAMPLE,
        TIMESTAMPED_WINDOW => temporal::TIMESTAMPED_WINDOW,
        other => panic!("unknown profile alias {other}"),
    }
}

/// One authored temporal declaration, its clock name and its selected profile.
pub struct Declaration<'a> {
    pub name: &'a str,
    pub profile: &'a str,
    pub clock: &'a str,
    /// Everything between the declaration's braces.
    pub body: &'a str,
    /// Activation clause, for example `on origin` or
    /// `on each (started: M::Node) when (Positive(started.n))`.
    pub activation: &'a str,
}

impl Declaration<'_> {
    fn source(&self) -> String {
        format!(
            "temporal {name} using {profile} over (view: M::Plain) clock \"{clock}\" {activation} {{ {body} }}",
            name = self.name,
            profile = self.profile,
            clock = self.clock,
            activation = self.activation,
            body = self.body,
        )
    }
}

/// A whole-execution-origin declaration with no captures.
pub fn origin<'a>(name: &'a str, profile: &'a str, body: &'a str) -> Declaration<'a> {
    Declaration {
        name,
        profile,
        clock: "orders",
        body,
        activation: "on origin",
    }
}

/// A compiled protocol package must carry at least one protocol family, so every
/// temporal control travels with this minimal consumer. It requires the temporal
/// obligation and does nothing else; the evaluator never reads it.
fn consumer(temporal: &str) -> String {
    format!(
        "protocol Flow using P over (view: M::Plain) on origin {{
          role Service on M::Node;
          requires temporal {temporal};
          run sequence Main {{
            event Happened by Service as (happened: M::Plain) {{ happened.ready }};
          }}
          finish Closed as (closed: M::Plain) {{ closed.ready }};
        }}"
    )
}

/// Compile and admit one temporal declaration, then run the control against the
/// admitted package and that declaration's index.
pub fn admitted(
    declaration: &Declaration<'_>,
    test: impl FnOnce(&artifact::AdmittedPackage, usize),
) {
    let source = declaration.source();
    let flow = consumer(declaration.name);
    let inputs = Inputs::new(&[
        Unit {
            name: "temporal-evaluation",
            body: &source,
            declarations: &[declaration.name],
        },
        Unit {
            name: "temporal-consumer",
            body: &flow,
            declarations: &["Flow"],
        },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proved, selected| {
            for entry in proved.declarations() {
                let id = entry.declaration();
                assert_eq!(
                    proved.types().disposition(id),
                    Some(TypeDisposition::Typed),
                    "{id:?} type causes {:?}",
                    proved.types().declaration(id).map(|typed| typed.causes()),
                );
            }
            let report = native::admit(proved, selected, Limits::default());
            assert!(
                report.result().is_ok(),
                "admission: {:?}; locus {:?}",
                report.result().err(),
                report.locus(),
            );
            let admission = report.into_result().unwrap();
            let index = admission
                .package()
                .declarations
                .iter()
                .position(|entry| entry.name == declaration.name)
                .expect("authored temporal declaration");
            let emitted = native::emit(&admission, Limits::default())
                .into_result()
                .expect("emitted package");
            let read = inputs.read(proved, &emitted);
            assert!(
                read.result().is_ok(),
                "independent reader: {:?}; locus {:?}",
                read.result().err(),
                read.locus()
            );
            test(&read.into_result().unwrap(), index);
        },
    );
}

/// A trace asserting the declaration's own profile and clock, with one closed,
/// complete decision scope and no triggers beyond the execution origin.
pub fn trace(profile_alias: &str, clock: &str) -> temporal::Trace {
    temporal::Trace {
        clock: temporal::ClockBinding {
            name: clock.into(),
            profile_identity: identity(profile_alias).into(),
            profile_revision: "1-draft.3".into(),
            parameters: BTreeMap::new(),
        },
        positions: Vec::new(),
        anchor: "origin".into(),
        triggers: vec![temporal::Trigger {
            identity: "execution:1".into(),
            receipt: "receipt:1".into(),
            anchor: "origin".into(),
            payload: String::new(),
            guard: None,
            captures: Vec::new(),
        }],
        trigger_evidence: temporal::Evidence::Admitted,
        trigger_scope: temporal::Closure::Closed,
        decision_scope: temporal::Closure::Closed,
        surrounding_execution: temporal::Closure::Open,
        execution: temporal::Execution::Completed,
        completeness: temporal::Completeness::Complete,
        authoritative_origin: true,
        watermark: 0,
        evicted: Vec::new(),
    }
}

/// One admitted position at `coordinate`, valuing every listed leaf.
pub fn position(coordinate: i64, valuations: &[(u32, bool)]) -> temporal::Position {
    temporal::Position {
        coordinate,
        order: None,
        valuations: valuations.iter().copied().collect(),
    }
}

/// The temporal arena indices of every `holds` leaf the declaration emitted.
pub fn holds_leaves(package: &artifact::AdmittedPackage, declaration: usize) -> Vec<u32> {
    let entry = &package.package().declarations[declaration];
    entry
        .temporal
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            matches!(
                node.operation,
                quire_spec_language::protocol_artifact::wire::TemporalOperation::Holds { .. }
            )
        })
        .map(|(index, _)| index as u32)
        .collect()
}

/// The temporal arena index of the declaration's single `holds` leaf.
pub fn holds_leaf(package: &artifact::AdmittedPackage, declaration: usize) -> u32 {
    let found = holds_leaves(package, declaration);
    assert_eq!(found.len(), 1, "expected exactly one holds leaf");
    found[0]
}

/// The single assessed obligation of a successful report.
pub fn assessment(report: &temporal::Report) -> &temporal::Assessment {
    let obligations = report
        .result()
        .unwrap_or_else(|error| panic!("evaluation stopped: {error:?}"));
    assert_eq!(obligations.len(), 1, "expected one obligation");
    match &obligations[0] {
        temporal::Obligation::Assessed(assessment) => assessment,
        temporal::Obligation::Unactivated { error, .. } => {
            panic!("activation failed: {error:?}")
        }
    }
}
