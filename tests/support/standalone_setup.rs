// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026: example file setup through the existing public model/compiler/runtime APIs.

// The shared fixture module also serves other runtime test targets.
#[allow(dead_code)]
#[path = "runtime_setup.rs"]
mod runtime;

use quire_spec_language::{
    formal_source::FormalSource,
    package::{NativePackage, PackageLimits},
    runtime::{ObservationSelection, ValueId, ValueNode},
    syntax::ClauseKind,
};
use serde_json::{json, Value};
use std::path::Path;

pub enum Case {
    Aggregate(i64),
    Operation(bool),
}

fn source(source: &FormalSource, file: &str) -> Value {
    json!({"file":file,"identity":source.source().identity().identity,"revision":source.source().identity().revision,
        "digest":source.source().digest().to_string(),"document":source.identity().document().as_str(),"formal_revision":source.identity().revision().get()})
}

/// Write synthetic licensed fixtures. Invalid setup is a generator error, not a result.
pub fn write(directory: &Path, case: Case) -> (Value, String) {
    std::fs::create_dir_all(directory).unwrap();
    let models = [runtime::native_rule_model::parts().model()];
    let model = &models[0];
    let (expression, kind, input, selection) = match case {
        Case::Aggregate(number) => {
            let mut draft = runtime::draft(model);
            runtime::change_field(&mut draft, "n", ValueNode::Integer { value: number });
            runtime::change_field(
                &mut draft,
                "items",
                ValueNode::Sequence {
                    values: vec![ValueId::new(0); 3],
                },
            );
            let snapshot = runtime::snapshot(draft);
            let selection = runtime::selection(model, snapshot.reference());
            (
                "true implies forall(item in self.items: item < self.n)",
                ClauseKind::Invariant,
                runtime::input(snapshot),
                selection,
            )
        }
        Case::Operation(bad_frame) => {
            let before = runtime::draft(model);
            let mut after = before.clone();
            runtime::change_field(&mut after, "n", ValueNode::Integer { value: 2 });
            if bad_frame {
                runtime::change_field(&mut after, "signed", ValueNode::Integer { value: 1 });
            }
            let (input, selection) = runtime::recorded(model, before, after, |_| {});
            (
                "pre(self.n) = 1 and self.n = 2 and result",
                ClauseKind::Postcondition,
                input,
                selection,
            )
        }
    };
    let checked = runtime::checked_kind(&models, expression, kind);
    let program = checked.linked().unit().source();
    std::fs::write(directory.join("model.json"), model.source().source().text()).unwrap();
    std::fs::write(directory.join("program.native"), program.text()).unwrap();
    let owner = |owner: &quire_contract_ir::RequirementRef| json!({"package":owner.package().as_str(),"requirement":owner.requirement().as_str(),"revision":owner.revision().get()});
    let clauses: Vec<_> = checked.bindings().clauses.iter().map(|clause| json!({"name":clause.name,"owner":owner(&clause.requirement),"clause":clause.clause.as_str(),"point":clause.execution_point})).collect();
    let snapshots: Vec<_> = input
        .snapshots
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let file = format!("snapshot-{index}.json");
            std::fs::write(directory.join(&file), value.bytes()).unwrap();
            json!({"file":file,"reference":value.reference()})
        })
        .collect();
    let invocations: Vec<_> = input
        .invocations
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let file = format!("invocation-{index}.json");
            std::fs::write(directory.join(&file), value.bytes()).unwrap();
            json!({"file":file,"reference":value.reference()})
        })
        .collect();
    let observation = match selection.observation {
        ObservationSelection::Current {
            snapshot,
            self_object,
        } => json!({"kind":"current","snapshot":snapshot,"self_object":self_object}),
        ObservationSelection::Invocation { invocation } => {
            json!({"kind":"invocation","invocation":invocation})
        }
    };
    let job = json!({"format":"native-run/1","request":{
        "models":[{"format":"native-rule-model/1","source":source(model.source(),"model.json")}],
        "program":{"source":source(&checked.bindings().source,"program.native"),"clauses":clauses},
        "snapshots":snapshots,"invocations":invocations,
        "selection":{"owner":owner(&selection.requirement),"clause":selection.clause.as_str(),"observation":observation}
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec_pretty(&job).unwrap(),
    )
    .unwrap();
    let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
    (job, package.digest().to_string())
}
