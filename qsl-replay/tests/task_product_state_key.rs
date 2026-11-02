// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-371 (FR-418-AC-11): task-product state keys use strict versioned framing.

use ix_trace_rs::trace;
use qsl_eval::simulation::StateKey;
use qsl_replay::{TaskProductStateKey, TaskProductStateKeyError};
use quire_semantic_value::model_transition::ModelTransition;

fn empty_task_control() -> Vec<u8> {
    let mut key = b"qsl-task-control-v1\0".to_vec();
    key.extend_from_slice(&0_u64.to_be_bytes()); // jobs
    key.push(0); // no running job
    key.extend_from_slice(&0_u64.to_be_bytes()); // activated tasks
    key.extend_from_slice(&0_u64.to_be_bytes()); // pending releases
    key
}

#[trace("TC-371", "FR-418-AC-11")]
#[test]
fn task_product_state_key_round_trips_and_rejects_malformed_frames() {
    let source = StateKey::from_canonical_bytes(br#"{"source":1}"#.to_vec());
    let scheduler_control = empty_task_control();
    let key = TaskProductStateKey::new(source.clone(), scheduler_control.clone())
        .into_state_key()
        .expect("frame lengths fit");
    let decoded = TaskProductStateKey::decode(&key)
        .expect("the encoded frame is valid")
        .expect("the key has the task-product prefix");
    assert_eq!(decoded.source(), source.as_bytes());
    assert_eq!(decoded.scheduler_control(), scheduler_control);
    let scheduler = decoded
        .scheduler_state()
        .expect("the validated scheduler state parses");
    assert!(scheduler.jobs.is_empty());
    assert!(scheduler.running.is_none());
    assert!(scheduler.activations.is_empty());
    assert!(scheduler.pending_releases.is_empty());

    let ordinary = StateKey::from_canonical_bytes(br#"{"model":true}"#.to_vec());
    assert_eq!(TaskProductStateKey::decode(&ordinary), Ok(None));

    let mut truncated = key.as_bytes().to_vec();
    truncated.pop();
    assert_eq!(
        TaskProductStateKey::decode(&StateKey::from_canonical_bytes(truncated)),
        Err(TaskProductStateKeyError::Truncated)
    );

    let mut trailing = key.as_bytes().to_vec();
    trailing.push(0);
    assert_eq!(
        TaskProductStateKey::decode(&StateKey::from_canonical_bytes(trailing)),
        Err(TaskProductStateKeyError::TrailingBytes)
    );
}

#[trace("TC-371", "FR-418-AC-11")]
#[test]
fn scheduler_control_parser_reads_typed_jobs_and_pending_releases() {
    let release = ModelTransition::new(
        serde_json::to_string("Cell::set").expect("canonical operation text"),
        None,
        Vec::new(),
        quire_canonical::Limits::new(1 << 20),
    )
    .expect("valid source transition");
    let release_bytes = release
        .canonical_bytes(quire_canonical::Limits::new(1 << 20))
        .expect("canonical transition identity");
    let mut control = b"qsl-task-control-v1\0".to_vec();
    control.extend_from_slice(&1_u64.to_be_bytes()); // one job
    put_bytes(&mut control, b"Ctl");
    put_bytes(&mut control, b"t1");
    control.extend_from_slice(&0_u64.to_be_bytes()); // rank
    control.extend_from_slice(&0_u64.to_be_bytes()); // slot
    put_bytes(&mut control, &release_bytes);
    control.push(1); // ready
    put_bytes(&mut control, b"2/1");
    control.push(1); // priority present
    control.extend_from_slice(&7_u64.to_be_bytes());
    control.push(0); // no running job
    control.extend_from_slice(&1_u64.to_be_bytes()); // one activation phase
    put_bytes(&mut control, b"Ctl");
    put_bytes(&mut control, b"t1");
    control.push(1);
    control.extend_from_slice(&1_u64.to_be_bytes()); // one pending release
    put_bytes(&mut control, b"Ctl");
    put_bytes(&mut control, b"t1");
    put_bytes(&mut control, &release_bytes);

    let key = TaskProductStateKey::new(
        StateKey::from_canonical_bytes(br#"{"source":1}"#.to_vec()),
        control,
    )
    .into_state_key()
    .expect("state key frame fits");
    let parsed = TaskProductStateKey::decode(&key)
        .expect("well-shaped task product key")
        .expect("task product key")
        .scheduler_state()
        .expect("scheduler rows parse");
    assert_eq!(parsed.jobs.len(), 1);
    assert_eq!(parsed.jobs[0].taskset, "Ctl");
    assert_eq!(parsed.jobs[0].task, "t1");
    assert_eq!(parsed.jobs[0].phase, 1);
    assert_eq!(parsed.jobs[0].demand, "2/1");
    assert_eq!(parsed.jobs[0].priority, Some(7));
    assert_eq!(parsed.jobs[0].release, release);
    assert_eq!(parsed.activations, [("Ctl".to_owned(), "t1".to_owned())]);
    assert_eq!(parsed.pending_releases.len(), 1);
    assert_eq!(
        parsed.pending_releases[0].source.operation(),
        "\"Cell::set\""
    );
}

fn put_bytes(target: &mut Vec<u8>, bytes: &[u8]) {
    target.extend_from_slice(
        &u64::try_from(bytes.len())
            .expect("test value fits")
            .to_be_bytes(),
    );
    target.extend_from_slice(bytes);
}
