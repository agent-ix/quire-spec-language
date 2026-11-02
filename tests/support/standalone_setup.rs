// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026–029, FR-031/033: example setup through public native APIs.

// File-relative #[path], not `crate::support::runtime_setup` -- this file is
// also compiled directly into `examples/standalone_fixtures.rs` (its own,
// separate crate, pre-dating and outside this ticket's scope), where no
// `crate::support` exists. A self-relative `mod` resolves identically in
// both hosts. `tests/support/mod.rs` re-exports this same nested copy as
// `crate::support::runtime_setup` rather than declaring its own, so the file
// is not loaded twice within the `it` crate (`clippy::duplicate_mod`).
#[allow(dead_code)]
#[path = "runtime_setup.rs"]
pub(crate) mod runtime;

use quire_spec_language::{
    formal_source::FormalSource,
    package::{NativePackage, PackageLimits},
    runtime::{ObservationSelection, ValueBinding, ValueId, ValueNode},
    syntax::ClauseKind,
};
use serde_json::{json, Value};
use std::{io, path::Path};

// Each command test target uses a different subset of the shared generator cases.
#[allow(dead_code)]
pub enum Case {
    Aggregate(i64),
    Operation { violate_frame: bool },
    Boolean { flag: bool },
    Integer(i64),
}

fn source(source: &FormalSource, file: &str) -> Value {
    json!({"file":file,"authority":source.source().identity().authority,"identity":source.source().identity().identity,
        "revision_namespace":source.source().identity().revision_namespace,"revision":source.source().identity().revision,
        "document":source.identity().document().as_str(),"formal_revision":source.identity().revision().get()})
}

/// Write licensed fixtures, propagating I/O failure separately from static setup defects.
pub fn write(directory: &Path, case: Case) -> io::Result<(Value, Vec<u8>)> {
    std::fs::create_dir_all(directory)?;
    let models = [match case {
        Case::Boolean { .. } => runtime::authored_model(|model| {
            model["values"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"flag","kind":"state","type":{"kind":"boolean"}}));
        }),
        Case::Integer(_) => runtime::authored_model(|model| {
            model["values"].as_array_mut().unwrap().push(json!({
                "name":"amount","kind":"state","type":{"kind":"scalar","name":"Version"}
            }));
        }),
        Case::Aggregate(_) | Case::Operation { .. } => runtime::native_rule_model::parts().model(),
    }];
    let model = &models[0];
    let (expression, kind, input, selection) = match case {
        Case::Boolean { flag: value } => {
            let mut draft = runtime::draft(model);
            let id = ValueId::new(u32::try_from(draft.arena.len()).unwrap());
            draft.arena.push(ValueNode::Boolean { value });
            draft.values.push(ValueBinding {
                declaration: runtime::qualified(model, "flag"),
                value: id,
            });
            let snapshot = runtime::snapshot(draft);
            let selection = runtime::selection(model, snapshot.reference());
            (
                "true implies flag",
                ClauseKind::Invariant,
                runtime::input(snapshot),
                selection,
            )
        }
        Case::Integer(value) => {
            let mut draft = runtime::draft(model);
            let id = ValueId::new(u32::try_from(draft.arena.len()).unwrap());
            draft.arena.push(ValueNode::Integer { value });
            draft.values.push(ValueBinding {
                declaration: runtime::qualified(model, "amount"),
                value: id,
            });
            let snapshot = runtime::snapshot(draft);
            let selection = runtime::selection(model, snapshot.reference());
            (
                "amount < 7",
                ClauseKind::Invariant,
                runtime::input(snapshot),
                selection,
            )
        }
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
        Case::Operation {
            violate_frame: bad_frame,
        } => {
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
    std::fs::write(directory.join("model.json"), model.source().source().text())?;
    std::fs::write(directory.join("program.native"), program.text())?;
    let owner = |owner: &quire_contract_model::RequirementRef| json!({"package":owner.package().as_str(),"requirement":owner.requirement().as_str(),"revision":owner.revision().get()});
    let clauses: Vec<_> = checked.bindings().clauses.iter().map(|clause| json!({"name":clause.name,"owner":owner(&clause.requirement),"clause":clause.clause.as_str(),"point":clause.execution_point})).collect();
    let snapshots: Vec<_> = input
        .snapshots
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let file = format!("snapshot-{index}.json");
            std::fs::write(directory.join(&file), value.bytes())?;
            Ok(json!({"file":file,"reference":value.reference()}))
        })
        .collect::<io::Result<_>>()?;
    let invocations: Vec<_> = input
        .invocations
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let file = format!("invocation-{index}.json");
            std::fs::write(directory.join(&file), value.bytes())?;
            Ok(json!({"file":file,"reference":value.reference()}))
        })
        .collect::<io::Result<_>>()?;
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
        serde_json::to_vec_pretty(&job)?,
    )?;
    let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
    std::fs::write(directory.join("package.json"), package.bytes())?;
    let compilation = json!({"format":"native-compile/1","request":{
        "models":job["request"]["models"],"program":job["request"]["program"]}});
    std::fs::write(
        directory.join("compile.json"),
        serde_json::to_vec_pretty(&compilation)?,
    )?;
    Ok((job, package.bytes().to_vec()))
}
