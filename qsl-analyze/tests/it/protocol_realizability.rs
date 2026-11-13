// SPDX-License-Identifier: AGPL-3.0-or-later
//! probe

use qsl_eval::model_subject::{ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::protocol::{ProtocolSystem, Sc};
use qsl_eval::simulation::TransitionSystem;

use crate::fill::{instance, model_subject, package, source_memory, Documents, PACKAGE};

const MEMBERS: &str = "role Buyer on M::Cell;\nrole Seller on M::Cell;\nrole Warehouse on M::Cell;\n\
    channel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
    channel Pick from Seller to Warehouse carries PickMsg ordering unordered delivery exactly-once-premise capacity 4 overflow reject;";
const RECORDS: &str = "record Ack { ok: Boolean; }\nrecord PickMsg { ok: Boolean; }\n";

#[test]
fn probe() {
    let text = format!(
        "{RECORDS}protocol Flow using pp over (k: M::Cell) on origin {{\n{MEMBERS}\nrun sequence Main {{\n\
         send Placed via Order as (placed: Ack) {{ true }};\n\
         receive Got via Order of Placed as (got: Ack) {{ true }};\n\
         choice Decide by Seller visible (got.ok) {{\n\
           case no when {{ not got.ok }} sequence Cancel {{ }}\n\
           case yes when {{ got.ok }} sequence Ship {{ send PickIt via Pick as (p: PickMsg) {{ true }};\n\
    receive PickedUp via Pick of PickIt as (q: PickMsg) {{ true }}; }}\n\
         }} }}\nfinish Done as (zz: Boolean) {{ true }};\n}}\n"
    );
    let checked = package(&text);
    let mut documents = Documents::new();
    let initial = vec![documents.start("s0", &[("c", 0)])];
    let subject = ProtocolSubject {
        model: model_subject(&checked, &documents, initial, &["c"]),
        protocol: "Flow".to_owned(),
        over: instance("c"),
        roles: ["Buyer", "Seller", "Warehouse"]
            .iter()
            .map(|role| RoleBinding { role: (*role).to_owned(), model: PACKAGE.to_owned() })
            .collect(),
        memory: source_memory(&checked, "Flow"),
    };
    let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
    let system = ProtocolSystem::new(model, &subject, Sc).expect("builds");
    let mut stack = vec![(0usize, system.initial()[0].clone())];
    while let Some((depth, state)) = stack.pop() {
        let (expansion, terminal) = system.successors_and_terminal(&state).unwrap();
        eprintln!("{depth} terminal={terminal:?} findings={:?}", expansion.findings);
        for (t, next) in expansion.successors {
            eprintln!("  {:?} thread={:?} node={:?} chan={:?} recv={:?} send={:?} rec={:?}", t.kind(), t.thread(), t.node(), t.channel_queue(), t.received(), t.send_record().map(|s| s.channel().to_owned()), system.binder_record_json(&t).unwrap()); eprintln!("    decides={:?}", system.deciding_choices(&t));
            stack.push((depth + 1, next));
        }
    }
    let v2 = crate::protocol_projection::admitted_source(&text);
    for node in &v2.resolved_graph().nodes {
        eprintln!("{} {:?}", node.semantic_form, node.declaration.as_ref().map(|d| d.qualified_name.iter().map(|s| s.to_string()).collect::<Vec<_>>()));
    }
}
