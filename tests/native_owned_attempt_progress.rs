// SPDX-License-Identifier: AGPL-3.0-only
//! Owned-attempt Boolean choices combined with immutable aliases and finite
//! repeat progress, driven through the actual parse/link/type/proof/admit/
//! emit/read stages. An admitted attempt record is a defined observation, not
//! evidence that its operation succeeded: nothing here asserts that an attempt
//! implies an effect, and the emitted packages are checked to carry no effect.

// The shared setup module also serves tests that rebind operation contracts;
// these cases author none, so its contract helper is unused in this target.
#[allow(dead_code)]
#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Error, Limits, Unsupported,
};
use setup::{Inputs, Unit};

/// One owned attempt by the deciding role; its record is the only Boolean atom.
const ATTEMPT: &str = "attempt Tried by Receiver on M::Node::step contracts []
    as (attempted: M::Plain) { true };";

/// The alias names are authored; the reader must find these exact spellings.
const SEEN: &str = "seen";
const DECIDED: &str = "decided";
const REFUSED: &str = "refused";

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]
fn an_immutable_alias_retains_the_exact_owned_attempt_boolean_atom() {
    // Every alias binds the same authored field read of the same attempt
    // record. A rebound, re-derived or independently minted atom fails here.
    let decision = choice(
        "Receiver",
        &format!("(let {SEEN} = attempted.ready in {SEEN})"),
        &format!("let {DECIDED} = attempted.ready in {DECIDED}"),
        &format!("let {REFUSED} = attempted.ready in not {REFUSED}"),
    );
    let inputs = inputs(&format!("sequence Main {{ {ATTEMPT} {decision} }}"));
    admitted(&inputs, |_, package| {
        let declaration = &package.declarations[0];
        let w::Body::Protocol {
            controls, roles, ..
        } = &declaration.body
        else {
            panic!("protocol")
        };
        let (attempt_node, attempt_binder, attempt_owner) = attempt(controls, "Tried");
        let record = &declaration.binders[attempt_binder.index as usize];
        assert_eq!(record.name, "attempted");
        assert_eq!(record.kind, w::BinderKind::Event);
        assert_eq!(
            declaration.anchors[record.anchor.index as usize].owner.0,
            Some(attempt_node),
            "the attempt record is anchored at its own authored occurrence"
        );

        let decide = controls.iter().find(|c| c.name == "Decide").unwrap();
        let w::ControlOperation::Choice { owner, cases, .. } = &decide.operation else {
            panic!("owned labeled choice")
        };
        assert_eq!(roles[owner.index as usize].name, "Receiver");
        assert_eq!(
            *owner, attempt_owner,
            "the deciding role is the attempting role, not merely an equal model type"
        );
        assert_eq!(
            cases.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
            ["yes", "no"],
            "definedness of the attempt decides no case: both alternatives stay"
        );

        for name in [SEEN, DECIDED, REFUSED] {
            let alias = declaration
                .binders
                .iter()
                .find(|binder| binder.name == name)
                .unwrap_or_else(|| panic!("authored alias {name}"));
            assert_eq!(alias.kind, w::BinderKind::Let);
            assert_ne!(
                alias.value_type, record.value_type,
                "{name} aliases the Boolean field, not the whole attempt record"
            );
            assert_ne!(
                alias.anchor, record.anchor,
                "{name} is a lexical alias with its own anchor, not a second record"
            );
            let initializer = alias
                .initializer
                .0
                .as_ref()
                .unwrap_or_else(|| panic!("{name} keeps its authored initializer"));
            let atom = &declaration.values[initializer.index as usize];
            assert_eq!(
                alias.value_type, atom.value_type,
                "{name} carries exactly the atom's type"
            );
            let w::ValueOperation::Field { base, field } = &atom.operation else {
                panic!("{name} initializer is the authored field read")
            };
            assert_eq!(
                package.models[field.model as usize].exports[field.export as usize]
                    .path
                    .last(),
                Some(&"ready".to_string())
            );
            let w::ValueOperation::Read { binder } =
                &declaration.values[base.index as usize].operation
            else {
                panic!("{name} reads a binder")
            };
            assert_eq!(
                *binder, attempt_binder,
                "{name} must read the original attempt record"
            );
            let locus = &alias.locus;
            assert_eq!(locus.source, 0);
            assert!(
                inputs.sources[0].text()[locus.span.start as usize..locus.span.end as usize]
                    .contains(name),
                "{name} keeps its original source region"
            );
        }
        no_effect(controls);
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-7", "FR-042-AC-8")]
fn an_unused_initializer_cannot_hide_an_unavailable_or_foreign_role_atom() {
    // The guard discards the alias, but the atom still has to be visible to,
    // and owned by, the deciding role.
    for (case, owner, visible, admits) in [
        ("visible own attempt", "Receiver", "attempted.ready", true),
        ("atom not made visible", "Receiver", "true", false),
        ("foreign deciding role", "Sender", "attempted.ready", false),
    ] {
        let decision = choice(
            owner,
            visible,
            "let discarded = attempted.ready in true",
            "false",
        );
        let inputs = inputs(&format!("sequence Main {{ {ATTEMPT} {decision} }}"));
        if admits {
            admitted(&inputs, |_, package| {
                let declaration = &package.declarations[0];
                let w::Body::Protocol { controls, .. } = &declaration.body else {
                    panic!("protocol")
                };
                let (_, attempt_binder, _) = attempt(controls, "Tried");
                let discarded = declaration
                    .binders
                    .iter()
                    .find(|binder| binder.name == "discarded")
                    .expect("authored alias");
                assert_eq!(discarded.kind, w::BinderKind::Let);
                let initializer = discarded
                    .initializer
                    .0
                    .as_ref()
                    .expect("an unused alias keeps its evaluated initializer");
                let w::ValueOperation::Field { base, .. } =
                    &declaration.values[initializer.index as usize].operation
                else {
                    panic!("authored field read")
                };
                let w::ValueOperation::Read { binder } =
                    &declaration.values[base.index as usize].operation
                else {
                    panic!("binder read")
                };
                assert_eq!(*binder, attempt_binder);
                let lets: Vec<_> = declaration
                    .values
                    .iter()
                    .filter_map(|value| match &value.operation {
                        w::ValueOperation::Let {
                            initializer, body, ..
                        } => Some((initializer, body)),
                        _ => None,
                    })
                    .collect();
                assert_eq!(lets.len(), 1);
                assert_eq!(lets[0].0, initializer);
                assert!(
                    matches!(
                        declaration.values[lets[0].1.index as usize].operation,
                        w::ValueOperation::Boolean { value: true }
                    ),
                    "the alias is genuinely unused in the guard body"
                );
                no_effect(controls);
            });
        } else {
            refused(&inputs, case);
        }
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-5", "FR-042-AC-7")]
fn a_continuing_repeat_admits_when_every_feasible_attempt_branch_progresses() {
    let inputs = inputs(&loop_body(
        "attempt Retried by Receiver on M::Node::step contracts []
           as (retried: M::Plain) { true };",
        "",
    ));
    admitted(&inputs, |_, package| {
        let declaration = &package.declarations[0];
        let w::Body::Protocol {
            controls,
            causal_edges,
            ..
        } = &declaration.body
        else {
            panic!("protocol")
        };
        let index = controls.iter().position(|c| c.name == "Loop").unwrap();
        let w::ControlOperation::Repeat { maximum, body, .. } = &controls[index].operation else {
            panic!("bounded repetition")
        };
        let artifact::ProtocolNumber::Integer(bound) = maximum.checked().unwrap() else {
            panic!("authored integer bound")
        };
        assert_eq!(bound.value(), 2);
        let owner = w::Handle {
            declaration: 0,
            index: index as u32,
        };
        let progress = causal_edges
            .iter()
            .find(|edge| edge.kind == w::EdgeKind::RepeatProgress && edge.owner == owner)
            .expect("continuing progress edge");
        let artifact::ProtocolNumber::Integer(edge_bound) =
            progress.maximum.0.as_ref().unwrap().checked().unwrap()
        else {
            panic!("authored integer bound")
        };
        assert_eq!(edge_bound.value(), 2);
        assert_eq!(progress.from.node, *body);

        let decide = controls.iter().find(|c| c.name == "Decide").unwrap();
        let w::ControlOperation::Choice { cases, .. } = &decide.operation else {
            panic!("choice")
        };
        assert_eq!(
            cases.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
            ["yes", "no", "dead"]
        );
        for feasible in &cases[..2] {
            assert!(
                matches!(
                    controls[feasible.body.index as usize].operation,
                    w::ControlOperation::Event { .. }
                ),
                "{} must carry its own observable record",
                feasible.label
            );
        }
        assert!(
            matches!(
                controls[cases[2].body.index as usize].operation,
                w::ControlOperation::Check { .. }
            ),
            "the infeasible case is retained without supplying progress"
        );
        no_effect(controls);
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-5", "FR-042-AC-8")]
fn a_repeat_refuses_when_a_feasible_attempt_branch_makes_no_progress() {
    let inputs = inputs(&loop_body("check Rejected using S { true };", ""));
    refused(&inputs, "feasible branch without progress");
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-5", "FR-042-AC-7")]
fn a_following_independent_event_restores_continuing_repeat_progress() {
    // The same zero-progress branch as the refusal above; only the sibling
    // event after the choice differs, so progress comes from that event.
    let inputs = inputs(&loop_body(
        "check Rejected using S { true };",
        "event Advanced by Receiver as (advanced: M::Plain) { true };",
    ));
    admitted(&inputs, |_, package| {
        let declaration = &package.declarations[0];
        let w::Body::Protocol { controls, .. } = &declaration.body else {
            panic!("protocol")
        };
        let decide = controls.iter().find(|c| c.name == "Decide").unwrap();
        let w::ControlOperation::Choice { cases, .. } = &decide.operation else {
            panic!("choice")
        };
        assert_eq!(cases[1].label, "no");
        assert!(
            matches!(
                controls[cases[1].body.index as usize].operation,
                w::ControlOperation::Check { .. }
            ),
            "the branch itself still makes no progress"
        );
        let advanced = controls
            .iter()
            .find(|c| c.name == "Advanced")
            .expect("authored following event");
        let w::ControlOperation::Event {
            event: w::Event::Event { owner, .. },
            binder,
            ..
        } = &advanced.operation
        else {
            panic!("independent observable event")
        };
        let w::Body::Protocol { roles, .. } = &declaration.body else {
            panic!("protocol")
        };
        assert_eq!(roles[owner.index as usize].name, "Receiver");
        assert_eq!(declaration.binders[binder.index as usize].name, "advanced");
        let iteration = controls.iter().find(|c| c.name == "Iteration").unwrap();
        let w::ControlOperation::Sequence { children } = &iteration.operation else {
            panic!("iteration body")
        };
        let advanced_index = controls.iter().position(|c| c.name == "Advanced").unwrap() as u32;
        let decide_index = controls.iter().position(|c| c.name == "Decide").unwrap() as u32;
        let order: Vec<_> = children.iter().map(|child| child.index).collect();
        assert_eq!(
            order,
            vec![decide_index, advanced_index],
            "the event follows the choice in the authored order"
        );
        no_effect(controls);
    });
}

/// One protocol with both roles, one channel and the authored run body.
fn inputs(run: &str) -> Inputs {
    let body = format!(
        "protocol Decisions using P over (view: M::Node) on origin {{
          role Sender on M::Node;
          role Receiver on M::Node;
          channel Messages from Sender to Receiver carries M::Plain
            ordering unordered delivery [1,1];
          run {run}
          finish Closed as (closed: M::Node) {{ true }};
        }}"
    );
    Inputs::new(&[Unit {
        name: "owned-attempt-progress",
        body: &body,
        declarations: &["Decisions"],
    }])
}

fn choice(owner: &str, visible: &str, yes: &str, no: &str) -> String {
    format!(
        "choice Decide by {owner} visible ({visible}) {{
          case yes when {{ {yes} }} check Accepted using S {{ true }};
          case no when {{ {no} }} check Rejected using S {{ true }};
        }}"
    )
}

/// A bounded repeat whose iteration decides on the earlier attempt record.
/// `no_body` is the second feasible branch; `after` follows the choice.
fn loop_body(no_body: &str, after: &str) -> String {
    format!(
        "sequence Main {{ {ATTEMPT}
          repeat Loop by Receiver visible (true) max 2 while {{ true }}
            sequence Iteration {{
              choice Decide by Receiver visible (attempted.ready) {{
                case yes when {{ attempted.ready }}
                  event Accepted by Receiver as (accepted: M::Plain) {{ true }};
                case no when {{ not attempted.ready }} {no_body}
                case dead when {{ false }} check Dead using S {{ true }};
              }} {after}
            }}
          exhausted check Limit using S {{ true }};
        }}"
    )
}

/// Locate an authored attempt and return its node, record binder and owner.
fn attempt(controls: &[w::Control], name: &str) -> (w::Handle, w::Handle, w::Handle) {
    let index = controls
        .iter()
        .position(|control| control.name == name)
        .unwrap_or_else(|| panic!("authored attempt {name}"));
    let w::ControlOperation::Event {
        event: w::Event::Attempt { owner, .. },
        binder,
        ..
    } = &controls[index].operation
    else {
        panic!("{name} is an owned attempt record")
    };
    assert_eq!(binder.declaration, 0, "{name} binds in its own declaration");
    (
        w::Handle {
            declaration: 0,
            index: index as u32,
        },
        binder.clone(),
        owner.clone(),
    )
}

/// An attempt observation is not effect-success evidence: no effect record may
/// appear in a package whose source authored none.
fn no_effect(controls: &[w::Control]) {
    assert!(
        !controls.iter().any(|control| matches!(
            &control.operation,
            w::ControlOperation::Event {
                event: w::Event::Effect { .. },
                ..
            }
        )),
        "an admitted attempt must not produce an effect record"
    );
}

/// Types and proofs discharge; the native family stage is what refuses.
fn discharged(report: &proofs::ProofReport<'_, '_, '_>) {
    assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
    for declaration in report.declarations() {
        let id = declaration.declaration();
        assert_eq!(
            report.types().disposition(id),
            Some(TypeDisposition::Typed),
            "type causes {:?}",
            report.types().declaration(id).map(|typed| typed.causes())
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
    }
}

fn refused(inputs: &Inputs, case: &str) {
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let report = native::admit(proofs, selected, Limits::default());
            assert_eq!(
                report.result().err(),
                Some(&Error::Unsupported(Unsupported::FamilyProof)),
                "{case}; locus {:?}",
                report.locus()
            );
        },
    );
}

/// Admit, emit and read back through the public reader, then inspect the
/// package the independent reader accepted.
fn admitted(inputs: &Inputs, inspect: impl FnOnce(&proofs::ProofReport<'_, '_, '_>, &w::Package)) {
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let report = native::admit(proofs, selected, Limits::default());
            assert!(
                report.result().is_ok(),
                "{:?}; locus {:?}",
                report.result().err(),
                report.locus()
            );
            let admitted = report.into_result().unwrap();
            let package = admitted.package();
            assert_eq!(package.sources.len(), 1);
            assert_eq!(
                package.sources[0].native.identity,
                inputs.sources[0].identity().identity
            );
            assert_eq!(package.sources[0].text, inputs.sources[0].text());
            inspect(proofs, package);
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            let read = inputs.read(proofs, &emitted);
            assert!(
                read.result().is_ok(),
                "{:?}; locus {:?}",
                read.result().err(),
                read.locus()
            );
            assert_eq!(read.into_result().unwrap().package(), package);
        },
    );
}
