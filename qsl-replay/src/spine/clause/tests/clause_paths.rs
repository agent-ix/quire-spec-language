// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1040 steps 1 to 4 (FR-107-AC-7 to AC-11): a clause evaluation's receiver
//! path and the runtime value path of each field read, over FR-108's `Config`
//! unit. A path is `(observation, subject, steps)`.

use super::clause_set::{admit, documents, Documents};
use super::frame_replay::{config_version_unit, unit_for, Unit};
use super::*;
use qsl_eval::value::{ClauseEvaluation, ObservationIdentity, RuntimeValuePath, ValuePathStep};
use qsl_eval::value::ValuePathSubject;

const UNLIMITED: u64 = 1_000_000;

/// The identity of the document `name` (`config_version_document_ref`).
fn identity(name: &str) -> ObservationIdentity {
    ObservationIdentity {
        authority: "test".to_owned(),
        identity: name.to_owned(),
        revision_namespace: "ns".to_owned(),
        revision: "1".to_owned(),
    }
}

fn path(
    observation: &str,
    subject: &ValuePathSubject,
    steps: Vec<ValuePathStep>,
) -> RuntimeValuePath {
    RuntimeValuePath {
        observation: identity(observation),
        subject: subject.clone(),
        steps,
    }
}

fn member(name: &str) -> ValuePathStep {
    ValuePathStep::Member(name.to_owned())
}

fn evaluate(
    unit: &Unit,
    clause: &str,
    observations: &AdmittedObservations,
    work_units: u64,
) -> ClauseEvaluation {
    unit.compiled
        .package
        .evaluate_clause(
            &QualifiedName::unqualified(clause).unwrap(),
            observations,
            &mut Meter::new(default_accounting(work_units)),
        )
        .expect("the clause evaluates")
}

fn read_paths(evaluation: &ClauseEvaluation) -> Vec<Option<RuntimeValuePath>> {
    evaluation
        .reads()
        .iter()
        .map(|read| read.path.clone())
        .collect()
}

fn object(observations: &AdmittedObservations) -> ValuePathSubject {
    ValuePathSubject::Object(observations.self_object.clone())
}

/// TC-1040 steps 1 and 2: `ParentOrder` over `H` (root 1, child 2 naming
/// root) and `A` (root alone).
#[trace("TC-1040", "FR-107-AC-7", "FR-107-AC-8", "FR-107-AC-9")]
#[test]
fn an_object_receivers_reads_are_located_at_its_declared_identity() {
    let unit = config_version_unit();
    let mut docs = documents();
    let h = docs.current_snapshot("H", 1, Some(2));
    let a = docs.current_snapshot("A", 1, None);
    let over = |docs: &Documents, snapshot: &DocumentRef, key: &str| {
        admit(
            &unit,
            docs,
            "ParentOrder",
            config_version_current_selection("ParentOrder", snapshot.clone(), key),
        )
    };
    let child = over(&docs, &h, "child");
    let root = over(&docs, &h, "root");
    let (child, root) = (object(&child), object(&root));

    let evaluation = evaluate(&unit, "ParentOrder", &over(&docs, &h, "child"), UNLIMITED);
    assert_eq!(evaluation.receiver_path(), Some(&path("H", &child, vec![])));
    let parent = path("H", &child, vec![member("parent")]);
    assert_eq!(
        read_paths(&evaluation),
        vec![
            Some(parent.clone()),
            Some(parent),
            Some(path(
                "H",
                &child,
                vec![
                    member("parent"),
                    ValuePathStep::OptionValue,
                    ValuePathStep::Reference(match &root {
                        ValuePathSubject::Object(reference) => reference.clone(),
                        other => panic!("root is an object subject: {other:?}"),
                    }),
                    member("versionNumber"),
                ],
            )),
            Some(path("H", &child, vec![member("versionNumber")])),
        ]
    );
    let locations: Vec<_> = evaluation.reads().iter().map(|r| &r.location).collect();
    assert_ne!(locations[0], locations[1], "two different selection nodes");

    // Over `A`, `implies` skips its right operand.
    let absent = over(&docs, &a, "root");
    let evaluation = evaluate(&unit, "ParentOrder", &absent, UNLIMITED);
    assert_eq!(
        read_paths(&evaluation),
        vec![Some(path("A", &object(&absent), vec![member("parent")]))]
    );

    // A zero work budget denies a charge before any read.
    let evaluation = evaluate(&unit, "ParentOrder", &over(&docs, &h, "child"), 0);
    assert!(evaluation.reads().is_empty());
}

/// TC-1040 step 3: `VersionUnchanged` over changed-version.
#[trace("TC-1040", "FR-107-AC-9")]
#[test]
fn a_read_under_pre_is_located_in_the_pre_observation() {
    let unit = config_version_unit();
    let mut docs = documents();
    let invocation = docs.invocation("changed", 2, 3);
    let observations = admit(
        &unit,
        &docs,
        "VersionUnchanged",
        config_version_invocation_selection("VersionUnchanged", invocation),
    );
    let child = object(&observations);
    let evaluation = evaluate(&unit, "VersionUnchanged", &observations, UNLIMITED);
    assert_eq!(
        read_paths(&evaluation),
        vec![
            Some(path("changed-post", &child, vec![member("versionNumber")])),
            Some(path("changed-pre", &child, vec![member("versionNumber")])),
        ]
    );
}

/// TC-1040 step 4: `ProbeOrdered` over an invocation and over a pre-call
/// observation.
#[trace("TC-1040", "FR-107-AC-7", "FR-107-AC-10")]
#[test]
fn a_parameter_read_is_rooted_at_the_invocation_and_absent_without_one() {
    let (base, _) = config_version_step3_unit_and_packages();
    let unit = unit_for(
        format!(
            "{base}pre ProbeOrdered using v on Config::ConfigVersion::probe {{ \
             self.versionNumber < target.versionNumber }}\n"
        ),
        config_version_step3_domain_document(),
    );
    let digest = config_version_step3_model_digest_hex();
    let version = |key: &str, version: &str| {
        let mut object = config_version_step3_object(key, None);
        object["fields"]["versionNumber"] = json!({"integer": version});
        object
    };
    let objects = [version("a", "1"), version("b", "2")];
    let (w, w_bytes) = document_ref_and_bytes("W", |label| {
        config_version_step3_snapshot(label, &digest, "pre", &objects)
    });
    let (post, post_bytes) = document_ref_and_bytes("W-post", |label| {
        config_version_step3_snapshot(label, &digest, "post", &objects)
    });
    let (i1, i1_bytes) = document_ref_and_bytes("i1", |label| {
        config_version_probe_invocation_bytes(label, &digest, &w, &post, "a", "b")
    });
    let mut docs = documents();
    docs.snapshots.insert(w.digest, w_bytes);
    docs.snapshots.insert(post.digest, post_bytes);
    docs.invocations.insert(i1.digest, i1_bytes);

    let by_invocation = admit(
        &unit,
        &docs,
        "ProbeOrdered",
        config_version_invocation_selection("ProbeOrdered", i1),
    );
    let by_pre_call = admit(
        &unit,
        &docs,
        "ProbeOrdered",
        ClauseSelection {
            name: "ProbeOrdered".to_owned(),
            input: ClauseSelectionInput::PreCall {
                snapshot: w,
                self_object: SelectedObject {
                    population: config_version_population_identity(),
                    key: "a".to_owned(),
                },
                parameters: BTreeMap::from([(
                    quire_exact::Identifier::new("target").unwrap(),
                    qsl_semantics::model::observation::SnapshotValue::reference(SelectedObject {
                        population: config_version_population_identity(),
                        key: "b".to_owned(),
                    }),
                )]),
            },
        },
    );
    let a = object(&by_invocation);
    let Some((_, Value::Reference(b))) = by_invocation.parameters.first() else {
        panic!("target is a reference parameter");
    };
    let own = path("W", &a, vec![member("versionNumber")]);

    let evaluation = evaluate(&unit, "ProbeOrdered", &by_invocation, UNLIMITED);
    assert_eq!(evaluation.receiver_path(), Some(&path("W", &a, vec![])));
    assert_eq!(
        read_paths(&evaluation),
        vec![
            Some(own.clone()),
            Some(path(
                "W",
                &ValuePathSubject::Invocation {
                    invocation: identity("i1"),
                    binding: qsl_eval::value::InvocationBinding::Parameter("target".to_owned()),
                },
                vec![ValuePathStep::Reference(b.clone()), member("versionNumber")],
            )),
        ]
    );

    let evaluation = evaluate(&unit, "ProbeOrdered", &by_pre_call, UNLIMITED);
    assert_eq!(evaluation.receiver_path(), Some(&path("W", &a, vec![])));
    assert_eq!(read_paths(&evaluation), vec![Some(own), None]);
}
