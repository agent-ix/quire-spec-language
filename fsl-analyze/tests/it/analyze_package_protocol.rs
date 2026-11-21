// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940-AC-6: protocol litmus and memory-model items settle equally from a
//! source-checked package and its admitted handoff bytes.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, litmus_request, package_selection, Run, Workspace,
};
use crate::litmus::{sb_run, sb_unit};

fn subject() -> Value {
    let binding = json!({"type": "model-binding", "selection": "test/litmus"});
    json!({"protocol": {
        "protocol": "SB",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"w": binding, "c": binding},
    }})
}

fn request_extra(items: Value) -> Value {
    json!({
        "subject": subject(),
        "memory": [{"parallel": "SB::Both", "model": "tso"}],
        "items": items,
    })
}

fn request_without_memory(items: Value) -> Value {
    json!({
        "subject": subject(),
        "items": items,
    })
}

fn package_request(
    ws: &Workspace,
    request_path: &std::path::Path,
) -> (std::path::PathBuf, Vec<u8>) {
    let document: Value =
        serde_json::from_slice(&std::fs::read(request_path).expect("the source request exists"))
            .expect("the source request is JSON");
    let (members, bytes) = package_selection(ws, &document["request"]);
    let package_document = json!({"format": "native-analyze/1", "request": members});
    (
        ws.write(
            "analyze-package.json",
            package_document.to_string().as_bytes(),
        ),
        bytes,
    )
}

fn package_run(ws: &Workspace, request_path: &std::path::Path) -> Run {
    analyze(&package_request(ws, request_path).0)
}

fn receive_fifo_request(
    ws: &Workspace,
    key: &str,
    capacity: u64,
    overflow: &str,
    delivery: &str,
    controls: &str,
    finish: &str,
) -> std::path::PathBuf {
    let mut model = finite_channel_model();
    let payload = model["types"]
        .as_array_mut()
        .expect("model types")
        .iter_mut()
        .find(|node| node["identity"] == "ix://test/fill/Attempt")
        .expect("the finite payload record");
    let field = payload["fields"][0].clone();
    for (name, value_type) in [
        ("ok", "ix://filament/native/Boolean"),
        ("target", "ix://test/fill/Cell"),
    ] {
        let mut member = field.clone();
        let identity = format!("ix://test/fill/Attempt/{name}");
        member["identity"] = json!(identity);
        member["name"] = json!(name);
        member["typeRef"] = json!(value_type);
        member["origin"] = json!({"generated": {
            "generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]
        }});
        payload["fields"]
            .as_array_mut()
            .expect("payload fields")
            .push(member);
    }
    let ordering = format!(
        "ordering fifo by (m: M::Attempt) {{ {key} }}
                delivery {delivery} capacity {capacity} overflow {overflow}"
    );
    channel_request(ws, model, &ordering, controls, finish)
}

fn receive_fifo_send(name: &str, binder: &str, n: u64) -> String {
    let body =
        format!("\n            {binder}.n = {n} and {binder}.ok and {binder}.target = k\n        ");
    channel_send_body(name, binder, &body)
}

fn receive_fifo_receive(name: &str, send: &str, binder: &str, constraint: &str) -> String {
    format!("receive {name} via Release of {send} as ({binder}: M::Attempt) {{ {constraint} }};")
}

/// The FIFO example's receive evaluates both scalar and reference-valued keys.
#[trace("TC-2958", "FR-940-AC-6", "FR-383-AC-4", "FR-383-AC-7", "FR-390")]
#[test]
fn package_receive_fifo_runs_the_complete_example_body() {
    for (name, key, delivery, finish, expected) in [
        (
            "reference",
            "m.target",
            "at-least-once",
            "closing",
            "proved",
        ),
        ("scalar", "m.n", "exactly-once-premise", "false", "refuted"),
    ] {
        let ws = Workspace::new(&format!("package-receive-fifo-complete-{name}"));
        let controls = receive_fifo_send("Released", "order", 0)
            + &receive_fifo_receive("Received", "Released", "got", "got.ok");
        let path = receive_fifo_request(&ws, key, 4, "block", delivery, &controls, finish);
        let item = channel_send_parity(&ws, &path, expected);
        if expected == "refuted" {
            let prefix = item["counterexample"]["prefix"]
                .as_array()
                .expect("a deadlock prefix");
            let received = prefix
                .iter()
                .find(|step| step["transition"]["step"] == "receive")
                .expect("the payload was received before the false finish");
            assert_eq!(received["transition"]["node"], "Main/Received", "{item}");
            let sent = prefix
                .iter()
                .find(|step| step["transition"]["step"] == "send")
                .expect("the payload's send");
            assert_eq!(
                received["transition"]["delivery"]["message"], sent["transition"]["message"],
                "{item}"
            );
            let fields = sent["send_record"]["payload"]["fields"]
                .as_array()
                .expect("payload fields");
            let ok = fields
                .iter()
                .find(|field| field["name"] == "ok")
                .expect("the ok field");
            assert_eq!(ok["value"]["value"], true, "{item}");
        } else {
            let states = item["exploration"]["states"]
                .as_str()
                .expect("state count")
                .parse::<u64>()
                .expect("a natural count");
            assert!(states > 3, "send, receive and finish were explored: {item}");
        }
    }
}

/// A receive of the second send waits only behind another send with the same key.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-4", "FR-211-AC-5")]
#[test]
fn package_receive_fifo_preserves_key_partition_and_head_blocking() {
    for (name, second_key, expected) in [("same", 0, "refuted"), ("different", 1, "proved")] {
        let ws = Workspace::new(&format!("package-receive-fifo-head-{name}"));
        let controls = receive_fifo_send("First", "sentOne", 0)
            + &receive_fifo_send("Second", "sentTwo", second_key)
            + &receive_fifo_receive("Received", "Second", "got", "got.ok");
        let path = receive_fifo_request(
            &ws,
            "m.n",
            2,
            "block",
            "exactly-once-premise",
            &controls,
            "closing",
        );
        let item = channel_send_parity(&ws, &path, expected);
        if expected == "refuted" {
            let blocked = item["counterexample"]["blocked"]
                .as_array()
                .expect("blocked receive");
            assert_eq!(blocked.len(), 1, "{item}");
            assert_eq!(blocked[0]["rest"]["node"], "Main/Received", "{item}");
            assert_eq!(blocked[0]["cause"]["type"], "head-blocked", "{item}");
            assert_eq!(blocked[0]["cause"]["channel"], "Flow::Release", "{item}");
        }
    }
}

/// Constraint, empty-queue and displaced evidence retain their own wait causes.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-7", "FR-211-AC-5")]
#[test]
fn package_receive_fifo_preserves_receive_wait_causes() {
    for cause in ["constraint", "empty-queue", "displaced"] {
        let ws = Workspace::new(&format!("package-receive-fifo-cause-{cause}"));
        let mut controls = receive_fifo_send("Released", "order", 0);
        let (capacity, overflow) = match cause {
            "constraint" => {
                controls += &receive_fifo_receive("Received", "Released", "got", "not got.ok");
                (2, "block")
            }
            "empty-queue" => {
                controls += &receive_fifo_receive("Received", "Released", "got", "got.ok");
                controls += &receive_fifo_receive("Again", "Released", "again", "again.ok");
                (2, "block")
            }
            "displaced" => {
                controls += &receive_fifo_send("Other", "other", 1);
                controls += &receive_fifo_receive("Received", "Released", "got", "got.ok");
                (1, "loss head")
            }
            _ => unreachable!("the listed wait causes"),
        };
        let path = receive_fifo_request(
            &ws,
            "true",
            capacity,
            overflow,
            "exactly-once-premise",
            &controls,
            "closing",
        );
        let item = channel_send_parity(&ws, &path, "refuted");
        let blocked = item["counterexample"]["blocked"]
            .as_array()
            .expect("blocked receive");
        assert_eq!(blocked.len(), 1, "{item}");
        assert_eq!(blocked[0]["cause"]["type"], cause, "{item}");
        let rest = if cause == "empty-queue" {
            "Main/Again"
        } else {
            "Main/Received"
        };
        assert_eq!(blocked[0]["rest"]["node"], rest, "{item}");
    }
}

/// Refused replay of a blocked FIFO receive retains the typed limit code and cause.
#[trace("TC-2958", "FR-940-AC-22", "FR-206-AC-4")]
#[test]
fn package_receive_fifo_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-receive-fifo-replay-bound");
    let controls = receive_fifo_send("First", "sentOne", 0)
        + &receive_fifo_send("Second", "sentTwo", 0)
        + &receive_fifo_receive("Received", "Second", "got", "got.ok");
    let path = receive_fifo_request(
        &ws,
        "m.n",
        2,
        "block",
        "exactly-once-premise",
        &controls,
        "closing",
    );
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert!(
        source.item(0).get("record").is_some(),
        "{}",
        source.document()
    );
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused", "{}", source.document());
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package replay");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Correlated receives retain a named refusal until package correlations execute.
#[trace("TC-2958", "FR-940", "FR-731-AC-1")]
#[test]
fn package_receive_fifo_related_premises_refuse_before_execution() {
    for (related, expected) in [(false, "refuted"), (true, "proved")] {
        let ws = Workspace::new(&format!("package-receive-fifo-related-{related}"));
        let controls = receive_fifo_send("Released", "order", 0)
            + "receive Correlated via Release of Released as (got: M::Attempt) related by Owns (k, got.target) { got.ok };";
        let path = receive_fifo_request(
            &ws,
            "m.target",
            1,
            "block",
            "exactly-once-premise",
            &controls,
            "closing",
        );
        channel_related_premise(&ws, related);
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "{}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package correlation refusal");
        let package = analyze(&package_path);
        let item = package.item(0);
        assert_eq!(package.exit, 21, "{}", package.document());
        assert_eq!(item["category"], "unsupported", "{}", package.document());
        let diagnostic = &item["record"]["diagnostic"];
        assert_eq!(diagnostic["code"], "unsupported_construct", "{item}");
        assert_eq!(diagnostic["cause"], "not-yet-implemented", "{item}");
        assert_eq!(diagnostic["detail"]["fact"], "correlation", "{item}");
        assert_eq!(
            diagnostic["detail"]["node"]["path"]
                .as_array()
                .expect("the refused admitted node's path")
                .last(),
            Some(&json!("Correlated")),
            "{item}"
        );
        assert!(item.get("exploration").is_none(), "{item}");
        assert!(item.get("counterexample").is_none(), "{item}");
    }
}

fn finite_channel_model() -> Value {
    let mut model: Value =
        serde_json::from_slice(&crate::fill::document()).expect("the Fill model is JSON");
    // The record's field ranges over Level so the three sent payloads can
    // distinguish which queue entry a head or tail policy displaced.
    model["types"]
        .as_array_mut()
        .expect("model types")
        .retain(|node| {
            matches!(
                node["identity"].as_str(),
                Some(
                    "ix://test/fill/Level"
                        | "ix://test/fill/Cell"
                        | "ix://test/fill/Idle"
                        | "ix://test/fill/Attempt"
                )
            )
        });
    let payload = model["types"]
        .as_array_mut()
        .expect("model types")
        .iter_mut()
        .find(|node| node["identity"] == "ix://test/fill/Attempt")
        .expect("the payload record");
    payload["fields"][0]["typeRef"] = json!("ix://test/fill/Level");
    model["constructs"]
        .as_array_mut()
        .expect("model constructs")
        .retain(|construct| {
            matches!(
                construct["kind"]["name"].as_str(),
                Some("value_type" | "object_type" | "population" | "record_value_type")
            )
        });
    model
}

fn channel_request(
    ws: &Workspace,
    model: Value,
    ordering: &str,
    controls: &str,
    finish: &str,
) -> std::path::PathBuf {
    let body = format!(
        "protocol Flow using pp over (k: M::Cell) on origin {{
            role Planning on M::Cell;
            role Machining on M::Cell;
            channel Release from Planning to Machining carries M::Attempt
                {ordering};
            run sequence Main {{ {controls} }}
            finish Over as (closing: Boolean) {{ {finish} }};
        }}"
    );
    let unit = crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body);
    let model = serde_json::to_vec(&model).expect("the channel model serializes");
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.values().cloned().collect();
    crate::analyze_cli_support::request_over(
        ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Flow", "kind": "temporal-satisfaction",
        "temporal": {"item_kind": "deadlock-freedom", "subject": {"protocol": {
            "protocol": "Flow",
            "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
            "roles": {
                "Planning": {"type": "model-binding", "selection": "test/fill"},
                "Machining": {"type": "model-binding", "selection": "test/fill"}
            }
        }}}}]}),
    )
}

fn channel_send_request(
    ws: &Workspace,
    capacity: u64,
    overflow: &str,
    delivery: &str,
    sends: &str,
    finish: &str,
) -> std::path::PathBuf {
    let ordering =
        format!("ordering unordered delivery {delivery} capacity {capacity} overflow {overflow}");
    channel_request(ws, finite_channel_model(), &ordering, sends, finish)
}

fn channel_send_body(name: &str, binder: &str, body: &str) -> String {
    format!("send {name} via Release as ({binder}: M::Attempt) {{{body}}};")
}

fn channel_send(name: &str, binder: &str, constraint: &str) -> String {
    channel_send_body(name, binder, &format!(" {constraint} "))
}

fn channel_send_parity(ws: &Workspace, path: &std::path::Path, expected: &str) -> Value {
    let source = analyze(path);
    assert!(
        source.item(0).get("record").is_some(),
        "source channel analysis: {}",
        source.document()
    );
    assert_eq!(label(&source.item(0)), expected, "{}", source.document());
    let (package_path, _) = package_request(ws, path);
    std::fs::remove_file(ws.path("program.fsl"))
        .expect("remove source before package analysis and replay");
    let package = analyze(&package_path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
    if expected == "refuted" {
        assert_eq!(package.item(0)["replay"]["reproduced"], true);
    }
    source.item(0)
}

/// The complete loss-head example's single constrained send uses model payloads.
#[trace("TC-2958", "FR-940-AC-6", "FR-383-AC-4", "FR-383-AC-7")]
#[test]
fn package_channel_send_runs_the_complete_loss_head_body() {
    let ws = Workspace::new("package-channel-send-complete");
    let path = channel_send_request(
        &ws,
        2,
        "loss head",
        "at-most-once",
        &channel_send("Released", "order", "order.n = k.v"),
        "closing",
    );
    let item = channel_send_parity(&ws, &path, "proved");
    let states = item["exploration"]["states"]
        .as_str()
        .expect("the explored state count")
        .parse::<u64>()
        .expect("a natural count");
    assert!(states > 2, "the send and finish were explored: {item}");
}

/// Every unordered overflow policy preserves the recorded payload and displacement.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-7", "FR-206-AC-12")]
#[test]
fn package_channel_send_preserves_unordered_overflow_records() {
    let sends = channel_send("First", "sent_a", "sent_a.n = k.v")
        + &channel_send("Second", "sent_b", "sent_b.n = sent_a.n + 1")
        + &channel_send("Third", "sent_c", "sent_c.n = sent_b.n + 1");
    for (overflow, outcome, displaced) in [
        ("reject", "rejected", None),
        ("loss new", "lost", None),
        ("loss head", "accepted", Some("0")),
        ("loss tail", "accepted", Some("1")),
    ] {
        let ws = Workspace::new(&format!("package-channel-send-{overflow}"));
        let path = channel_send_request(&ws, 2, overflow, "exactly-once-premise", &sends, "false");
        let item = channel_send_parity(&ws, &path, "refuted");
        let records: Vec<_> = item["counterexample"]["prefix"]
            .as_array()
            .expect("a deadlock prefix")
            .iter()
            .filter_map(|step| step.get("send_record"))
            .collect();
        assert_eq!(records.len(), 3, "three sends are reached: {item}");
        for record in &records {
            assert_eq!(record["channel"], "Flow::Release");
        }
        assert_eq!(records[0]["outcome"], "accepted");
        assert_eq!(records[1]["outcome"], "accepted");
        assert_eq!(records[2]["outcome"], outcome);
        assert_eq!(records[2]["payload"]["fields"][0]["value"]["value"], "2");
        match displaced {
            Some(value) => {
                assert_eq!(records[2]["displaced"]["type"], "queue-entry");
                assert_eq!(
                    records[2]["displaced"]["payload"]["fields"][0]["value"]["value"], value,
                    "{item}"
                );
            }
            None => assert_eq!(records[2]["displaced"], Value::Null, "{item}"),
        }
    }
}

/// A send's constraint is checked before capacity, including in blocked evidence.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-4", "FR-211-AC-5")]
#[test]
fn package_channel_send_preserves_capacity_and_constraint_causes() {
    for (name, overflow, constraint, expected, cause) in [
        (
            "full",
            "block",
            "sent_b.n = 1",
            "refuted",
            Some("full-queue"),
        ),
        (
            "constraint",
            "block",
            "false",
            "refuted",
            Some("constraint"),
        ),
        ("loss", "loss head", "sent_b.n = 1", "proved", None),
    ] {
        let ws = Workspace::new(&format!("package-channel-send-{name}"));
        let sends = channel_send("First", "sent_a", "sent_a.n = 0")
            + &channel_send("Second", "sent_b", constraint);
        let path =
            channel_send_request(&ws, 1, overflow, "exactly-once-premise", &sends, "closing");
        let item = channel_send_parity(&ws, &path, expected);
        if let Some(cause) = cause {
            let blocked = item["counterexample"]["blocked"]
                .as_array()
                .expect("the blocked send");
            assert_eq!(blocked.len(), 1, "{item}");
            assert_eq!(blocked[0]["rest"]["node"], "Main/Second");
            assert_eq!(blocked[0]["cause"]["type"], cause, "{item}");
        }
    }
}

/// Refused replay of a full queue preserves the typed limit code and cause.
#[trace("TC-2958", "FR-940-AC-22", "FR-206-AC-4")]
#[test]
fn package_channel_send_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-channel-send-replay-bound");
    let sends = channel_send("First", "sent_a", "sent_a.n = 0")
        + &channel_send("Second", "sent_b", "sent_b.n = 1");
    let path = channel_send_request(&ws, 1, "block", "exactly-once-premise", &sends, "closing");
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert!(
        source.item(0).get("record").is_some(),
        "{}",
        source.document()
    );
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused");
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    std::fs::remove_file(ws.path("program.fsl"))
        .expect("remove source before package analysis and replay");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Extra or malformed capacity bindings refuse with identical typed records.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_channel_send_invalid_capacity_records_match_source() {
    for (name, value, cause) in [
        ("negative", "-1", "invalid-value"),
        ("unknown", "1", "unknown-member"),
    ] {
        let ws = Workspace::new(&format!("package-channel-send-capacity-{name}"));
        let path = channel_send_request(
            &ws,
            1,
            "block",
            "exactly-once-premise",
            &channel_send("Only", "only", "only.n = 0"),
            "closing",
        );
        let mut document: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("request JSON");
        document["request"]["items"][0]["temporal"]["subject"]["protocol"]["capacities"] =
            json!({"extra": value});
        std::fs::write(&path, document.to_string()).expect("write malformed capacity selection");
        let source = analyze(&path);
        let item = source.item(0);
        assert_eq!(item["category"], "refusal", "{}", source.document());
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{item}"
        );
        assert_eq!(item["record"]["diagnostic"]["cause"], cause, "{item}");
        if name == "unknown" {
            assert_eq!(
                item["record"]["diagnostic"]["detail"]["capacity"], "extra",
                "{item}"
            );
        }
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package refusal");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), item, "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// Mixed parallel/send bodies retain a typed refusal until their message
/// identities and weak constraint footprints have a package-native form.
#[trace("TC-2958", "FR-940", "FR-894", "FR-205")]
#[test]
fn package_channel_send_parallel_combinations_refuse_before_execution() {
    for memory in ["sc", "tso"] {
        for position in ["inside", "after"] {
            let ws = Workspace::new(&format!("package-channel-send-{memory}-{position}"));
            let send = channel_send("Only", "only", "only.n = 0");
            let (inside, after) = if position == "inside" {
                (send.as_str(), "")
            } else {
                ("", send.as_str())
            };
            let run = format!(
                "parallel Both memory {memory} {{
                    branch left sequence Left {{ {inside} }}
                    branch right sequence Right {{ }}
                }} join all [left, right]; {after}"
            );
            let path =
                channel_send_request(&ws, 1, "block", "exactly-once-premise", &run, "closing");
            let source = analyze(&path);
            assert!(
                source.item(0).get("record").is_some(),
                "{}",
                source.document()
            );
            assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
            let (package_path, _) = package_request(&ws, &path);
            std::fs::remove_file(ws.path("program.fsl"))
                .expect("remove source before package refusal");
            let package = analyze(&package_path);
            let item = package.item(0);
            assert_eq!(package.exit, 21, "{}", package.document());
            assert_eq!(item["category"], "unsupported", "{}", package.document());
            assert_eq!(
                item["record"]["diagnostic"]["code"], "unsupported_construct",
                "{item}"
            );
            assert_eq!(
                item["record"]["diagnostic"]["cause"], "not-yet-implemented",
                "{item}"
            );
            assert!(item.get("exploration").is_none(), "{item}");
            assert!(item.get("counterexample").is_none(), "{item}");
        }
    }
}

fn channel_related_premise(ws: &Workspace, related: bool) {
    let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("the source unit");
    std::fs::write(
        ws.path("program.fsl"),
        unit.replace(
            "channel Release",
            "relationship Owns = M::owns; channel Release",
        ),
    )
    .expect("select the relationship premise");
    let mut model: Value =
        serde_json::from_slice(&std::fs::read(ws.path("model.json")).expect("model document"))
            .expect("model JSON");
    crate::related_by::add_owns_relationship(&mut model);
    std::fs::write(ws.path("model.json"), model.to_string()).expect("write the related model");
    let mut snapshot: Value = serde_json::from_slice(
        &std::fs::read(ws.path("snapshot-0.json")).expect("the initial snapshot"),
    )
    .expect("snapshot JSON");
    let object = json!({"population": "ix://test/fill/cells", "key": "c"});
    let instances = if related {
        vec![json!({"source": object, "target": object})]
    } else {
        Vec::new()
    };
    snapshot["relationships"] = json!([{
        "relationship": "ix://test/fill/relationship/owns", "complete": true,
        "instances": instances
    }]);
    std::fs::write(ws.path("snapshot-0.json"), snapshot.to_string())
        .expect("write the relationship instances");
}

/// A nonempty admitted correlation group is named and refused before package execution.
#[trace("TC-2958", "FR-940", "FR-731-AC-1")]
#[test]
fn package_channel_send_related_premises_refuse_before_execution() {
    for (related, expected) in [(false, "refuted"), (true, "proved")] {
        let ws = Workspace::new(&format!("package-channel-send-related-{related}"));
        let path = channel_send_request(
            &ws,
            1,
            "block",
            "exactly-once-premise",
            "send Correlated via Release as (sent: M::Attempt) related by Owns (k, k) { sent.n = 0 };",
            "closing",
        );
        channel_related_premise(&ws, related);
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "{}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package correlation refusal");
        let package = analyze(&package_path);
        let item = package.item(0);
        assert_eq!(package.exit, 21, "{}", package.document());
        assert_eq!(item["category"], "unsupported", "{}", package.document());
        let diagnostic = &item["record"]["diagnostic"];
        assert_eq!(diagnostic["code"], "unsupported_construct", "{item}");
        assert_eq!(diagnostic["cause"], "not-yet-implemented", "{item}");
        assert_eq!(diagnostic["detail"]["fact"], "correlation", "{item}");
        assert_eq!(
            diagnostic["detail"]["node"]["path"]
                .as_array()
                .expect("the refused admitted node's path")
                .last(),
            Some(&json!("Correlated")),
            "{item}"
        );
        assert!(item.get("exploration").is_none(), "{item}");
        assert!(item.get("counterexample").is_none(), "{item}");
    }
}

fn parallel_request(ws: &Workspace, run: &str) -> std::path::PathBuf {
    let body = format!(
        "{}{}",
        crate::fill::contracts("false"),
        crate::fill::protocol("Flow", "role a on M::Cell;", run, "d")
    );
    let unit = crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body);
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.values().cloned().collect();
    // Both roles and every attempted operation use the Cell/Level closure.
    // Idle remains because the shared model declares its empty population.
    let mut model: Value =
        serde_json::from_slice(&crate::fill::document()).expect("the Fill model is JSON");
    model["types"]
        .as_array_mut()
        .expect("model types")
        .retain(|node| {
            matches!(
                node["identity"].as_str(),
                Some("ix://test/fill/Level" | "ix://test/fill/Cell" | "ix://test/fill/Idle")
            )
        });
    model["constructs"]
        .as_array_mut()
        .expect("model constructs")
        .retain(|construct| {
            matches!(
                construct["kind"]["name"].as_str(),
                Some("value_type" | "object_type" | "population")
            )
        });
    let model = serde_json::to_vec(&model).expect("the parallel model serializes");
    crate::analyze_cli_support::request_over(
        ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Flow", "kind": "temporal-satisfaction",
        "temporal": {"item_kind": "deadlock-freedom", "subject": {"protocol": {
            "protocol": "Flow",
            "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
            "roles": {
                "w": {"type": "model-binding", "selection": "test/fill"},
                "a": {"type": "model-binding", "selection": "test/fill"}
            }
        }}}}]}),
    )
}

fn parallel_attempts(join: &str) -> String {
    format!(
        "parallel Both {{ branch Machine {} branch Assemble {} }} {join};",
        crate::fill::attempt("Reserved", "setTwo", "TwoPre, TwoPost", "reserved", "true"),
        "attempt Assembled by a on M::Cell::touch contracts [TouchPre] as (assembled: Boolean) { true };"
    )
}

/// The complete two-attempt Any/cancel control-flow example, with one blocked
/// contract, distinguishes cancellation from leaving the other branch running.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-3", "FR-206-AC-6")]
#[test]
fn package_parallel_any_preserves_outstanding_and_joined_labels() {
    for (name, join, expected) in [
        (
            "cancel",
            "join any [Machine, Assemble] outstanding cancel",
            "proved",
        ),
        (
            "continue",
            "join any [Machine, Assemble] outstanding continue",
            "refuted",
        ),
        (
            "blocked-label",
            "join any [Machine] outstanding cancel",
            "refuted",
        ),
        (
            "ready-label",
            "join any [Assemble] outstanding cancel",
            "proved",
        ),
        ("all", "join all [Machine, Assemble]", "refuted"),
        ("all-ready", "join all [Machine, Assemble]", "proved"),
    ] {
        let ws = Workspace::new(&format!("package-parallel-any-{name}"));
        let mut run = parallel_attempts(join);
        if name == "all-ready" {
            run = run.replace("TwoPre, TwoPost", "TwoPost");
        }
        let path = parallel_request(&ws, &run);
        let source = analyze(&path);
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// Cancelling an outer branch also cancels its inner running branches; keeping
/// them running prevents finish and leaves the same deadlock on both routes.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-9", "FR-206-AC-6")]
#[test]
fn package_parallel_any_cancels_nested_running_branches() {
    for (outstanding, expected) in [("cancel", "proved"), ("continue", "refuted")] {
        let ws = Workspace::new(&format!("package-parallel-any-nested-{outstanding}"));
        let blocked = crate::fill::attempt("Blocked", "setTwo", "TwoPre, TwoPost", "b", "true");
        let ready = crate::fill::attempt("Ready", "touch", "TouchPre", "r", "true");
        let run = format!(
            "parallel Outer {{ branch fast {ready} branch nested parallel Inner {{ \
             branch left {blocked} branch right {} }} join all [left, right]; \
             }} join any [fast] outstanding {outstanding};",
            crate::fill::attempt("Other", "setTwo", "TwoPre, TwoPost", "o", "true")
        );
        let path = parallel_request(&ws, &run);
        let source = analyze(&path);
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// Any/continue deadlocks retain the replay refusal's typed code and cause.
#[trace("TC-2958", "FR-940-AC-22", "FR-206-AC-3")]
#[test]
fn package_parallel_any_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-parallel-any-replay-bound");
    let path = parallel_request(
        &ws,
        &parallel_attempts("join any [Machine, Assemble] outstanding continue"),
    );
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused");
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package analysis");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

fn terminal_request(
    ws: &Workspace,
    captures: &str,
    terminal: &str,
    blocked: bool,
) -> std::path::PathBuf {
    let run = if blocked {
        format!(
            "sequence Main {{ {} }}",
            crate::fill::attempt("Blocked", "setTwo", "TwoPre, TwoPost", "attempted", "true")
        )
    } else {
        "sequence Main {}".to_owned()
    };
    let body = format!(
        "{}protocol Flow using pp over (k: M::Cell) on origin {{
            {captures}
            role w on M::Cell;
            {terminal}
            run {run}
            finish Done as (d: Boolean) {{ d }};
        }}",
        crate::fill::contracts("false")
    );
    let unit = crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body);
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.values().cloned().collect();
    // Select the terminal fixture's domain closure; the shared Fill corpus also
    // contains unrelated variant and compensation record declarations.
    let mut model: Value =
        serde_json::from_slice(&crate::fill::document()).expect("the Fill model is JSON");
    model["types"]
        .as_array_mut()
        .expect("model types")
        .retain(|node| {
            matches!(
                node["identity"].as_str(),
                Some("ix://test/fill/Level" | "ix://test/fill/Cell" | "ix://test/fill/Idle")
            )
        });
    model["constructs"]
        .as_array_mut()
        .expect("model constructs")
        .retain(|construct| {
            matches!(
                construct["kind"]["name"].as_str(),
                Some("value_type" | "object_type" | "population")
            )
        });
    let model = serde_json::to_vec(&model).expect("the terminal model serializes");
    crate::analyze_cli_support::request_over(
        ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Flow", "kind": "temporal-satisfaction",
        "temporal": {"item_kind": "deadlock-freedom", "subject": {"protocol": {
            "protocol": "Flow",
            "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
            "roles": {"w": {"type": "model-binding", "selection": "test/fill"}}
        }}}}]}),
    )
}

/// Terminal declarations classify stopped live instances identically on both routes.
#[trace("TC-2958", "FR-940-AC-6", "FR-850-AC-11", "FR-211-AC-2")]
#[test]
fn package_terminal_classifies_blocked_live_instances() {
    for (name, terminal, expected) in [
        ("absent", "", "refuted"),
        ("any", "terminal any;", "proved"),
        ("true", "terminal when { k.v = 0 };", "proved"),
        ("false", "terminal when { k.v = 1 };", "refuted"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-{name}"));
        let path = terminal_request(&ws, "", terminal, true);
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source terminal {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert!(
                source.item(0)["counterexample"]["blocked"]
                    .as_array()
                    .is_some_and(|blocked| !blocked.is_empty()),
                "{}",
                source.document()
            );
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// A finished instance is intended even when its terminal predicate is false.
#[trace("TC-2958", "FR-940-AC-6", "FR-211-AC-2")]
#[test]
fn package_terminal_preserves_finished_instances() {
    for (name, terminal) in [
        ("any", "terminal any; scheduling adversarial;"),
        ("false", "terminal when { k.v = 1 };"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-finished-{name}"));
        let path = terminal_request(&ws, "", terminal, false);
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source finished {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
        let package = package_run(&ws, &path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// Origin captures initialize in order from the admitted observation and feed terminal predicates.
#[trace("TC-2958", "FR-940-AC-6", "FR-211-AC-2", "FR-383-AC-5")]
#[test]
fn package_terminal_reads_origin_captures_in_authored_order() {
    for (name, predicate, expected) in [("true", "saved", "proved"), ("false", "later", "refuted")]
    {
        let ws = Workspace::new(&format!("package-terminal-captures-{name}"));
        let members = format!("terminal when {{ {predicate} }};");
        let path = terminal_request(
            &ws,
            "capture saved: Boolean = k.v = 0; capture later: Boolean = not saved;",
            &members,
            true,
        );
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source capture {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// Refusing replay retains the same typed cause for a terminal deadlock on both routes.
#[trace("TC-2958", "FR-940-AC-22", "FR-211-AC-2")]
#[test]
fn package_terminal_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-terminal-replay-bound");
    let path = terminal_request(
        &ws,
        "capture saved: Boolean = k.v = 1;",
        "terminal when { saved };",
        true,
    );
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert!(
        source.item(0).get("record").is_some(),
        "source replay refusal: {}",
        source.document()
    );
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused");
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Invalid deadlock subjects refuse before exploration with identical typed records.
#[trace("TC-2958", "FR-940-AC-6", "FR-850-AC-11")]
#[test]
fn package_terminal_invalid_subject_records_match_source() {
    for (name, member) in [
        ("missing", "temporal.subject.protocol"),
        ("mismatched", "temporal.subject.protocol"),
        ("capacity", "capacities"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-invalid-{name}"));
        let path = terminal_request(&ws, "", "terminal any;", false);
        let mut document: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("source request"))
                .expect("request JSON");
        let subject = &mut document["request"]["items"][0]["temporal"]["subject"];
        match name {
            "missing" => {
                subject
                    .as_object_mut()
                    .expect("subject object")
                    .remove("protocol");
            }
            "mismatched" => subject["protocol"]["protocol"] = json!("Other"),
            "capacity" => subject["protocol"]["capacities"] = json!({"unused": "-1"}),
            _ => unreachable!("the listed malformed cases"),
        }
        std::fs::write(&path, document.to_string()).expect("write malformed selection");
        let source = analyze(&path);
        let item = source.item(0);
        assert_eq!(item["category"], "refusal", "{}", source.document());
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["cause"], "invalid-value",
            "{item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["detail"]["member"], member,
            "{item}"
        );
        assert!(
            item.get("exploration").is_none(),
            "invalid selection explored states: {item}"
        );
        assert_eq!(source.exit, 20, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package refusal");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), item, "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// FR-940-AC-6: the store-buffering litmus claim under TSO has equal records,
/// counterexamples and package-byte replay on both analyze routes.
#[trace("TC-2958", "FR-940-AC-6", "TC-3050", "FR-850-AC-11")]
#[test]
fn the_tso_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("package-litmus-tso");
    let source_request =
        litmus_request(&ws, &body, request_extra(json!([{"claim": "NoBothZero"}])));
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}

/// Release/acquire uses the same package protocol, effective memory model
/// and bytes-only counterexample replay as the source route.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_ra_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("package-litmus-ra");
    let mut extra = request_extra(json!([{"claim": "NoBothZero"}]));
    extra["memory"][0]["model"] = json!("ra");
    let source_request = litmus_request(&ws, &body, extra);
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0), source.item(0));
    assert_eq!(package.exit, source.exit);
}

/// FR-940-AC-6: the protocol's race-freedom result reads the same admitted
/// protocol and memory-model facts on the package route.
#[trace("TC-2958", "FR-940-AC-6", "TC-3050", "FR-850-AC-11")]
#[test]
fn the_tso_race_freedom_item_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "", ""));
    let ws = Workspace::new("package-race-tso");
    let source_request = litmus_request(
        &ws,
        &body,
        request_extra(json!([{
            "claim": "SB::Both",
            "temporal": {"item_kind": "race-freedom"},
        }])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}

/// FR-850-AC-11: the same SB protocol with no request memory member uses its
/// source `sc` default on both analyze routes, under which the litmus claim
/// holds.
#[trace("TC-3050", "FR-850-AC-11")]
#[test]
fn the_default_sc_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "", ""));
    let ws = Workspace::new("package-litmus-default-sc");
    let source_request = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "NoBothZero"}])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "proved", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
}

/// A later attempt retains the earlier attempt's result binder while its
/// constraint reads it, on both protocol routes.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_later_package_attempt_can_read_an_earlier_result_binder() {
    let run = "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        attempt B by c on M::Core::copy contracts [CopyR] as (b: Boolean) { not a }; \
        }";
    let body = sb_unit(run).replace(
        crate::litmus::NO_BOTH_ZERO,
        "temporal BinderLifetime using t0 over (k: M::Core) clock \
         \"model-steps\" on origin { always holds(true) }\n",
    );
    let ws = Workspace::new("package-protocol-binder-lifetime");
    let source_request = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(source.exit, 0, "{}", source.document());

    let package = package_run(&ws, &source_request);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

fn true_claim(run: &str) -> String {
    sb_unit(run).replace(
        crate::litmus::NO_BOTH_ZERO,
        "temporal BinderLifetime using t0 over (k: M::Core) clock \
         \"model-steps\" on origin { always holds(true) }\n",
    )
}

/// Finish retains a result binder and observes the current model state.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_finish_reads_the_previous_result_and_current_model() {
    let body = true_claim(
        "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        }",
    )
    .replace("{ d };", "{ d and not a and k.r1 = 0 };");
    let ws = Workspace::new("package-protocol-finish-binders");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// An attempt's object reference is admitted against its current observation.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_attempt_constraint_reads_the_current_model() {
    let body = true_claim(
        "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        attempt B by c on M::Core::copy contracts [CopyR] as (b: Boolean) { k.r1 = 0 }; \
        }",
    );
    let ws = Workspace::new("package-protocol-attempt-observation");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// The protocol replay shares the request's package-byte ceiling on both routes.
#[trace("TC-2958", "FR-940-AC-22")]
#[test]
fn package_protocol_replay_obeys_the_request_input_bytes_bound() {
    let ws = Workspace::new("package-protocol-replay-input-bytes");
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let path = litmus_request(&ws, &body, request_extra(json!([{"claim": "NoBothZero"}])));
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    assert_eq!(source.item(0)["record"]["cause"]["kind"], "replay-refused");
    assert_eq!(
        source.item(0)["record"]["cause"]["code"],
        "stage_limit_exceeded"
    );
    assert_eq!(
        source.item(0)["record"]["cause"]["cause"],
        "input-bytes-exceeded"
    );
    assert_eq!(
        source.item(0)["record"]["cause"]["setting"],
        "replay.input_bytes"
    );
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Every memory entry is admitted before items run, including unused protocols.
#[trace("TC-2959", "FR-1033-AC-4", "FR-850-AC-11")]
#[test]
fn bad_package_memory_entries_refuse_the_whole_request() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    for (index, memory) in [
        json!([{"parallel": "Nope::Both", "model": "tso"}]),
        json!([{"parallel": "SB::Both::left::L::Sx", "model": "tso"}]),
        json!([{"parallel": "SB::Both", "model": "tso"}, {"parallel": "SB::Both", "model": "ra"}]),
    ]
    .into_iter()
    .enumerate()
    {
        let ws = Workspace::new(&format!("package-invalid-memory-{index}"));
        let path = litmus_request(
            &ws,
            &body,
            json!({"subject": subject(), "memory": memory, "items": [{"claim": "NoBothZero"}]}),
        );
        let source = analyze(&path);
        assert_eq!(source.exit, 20, "{}", source.document());
        assert_eq!(source.document()["items"], json!([]));
        let package = package_run(&ws, &path);
        assert_eq!(package.exit, 20, "{}", package.document());
        assert_eq!(package.document()["items"], json!([]));
        assert_eq!(
            package.document()["diagnostics"],
            source.document()["diagnostics"]
        );
        assert_eq!(
            package.document()["category"],
            source.document()["category"]
        );
    }
}

/// A valid memory entry for another protocol leaves this subject's identity alone.
#[trace("TC-2959", "FR-1033", "FR-850-AC-11")]
#[test]
fn another_protocols_memory_does_not_change_the_package_subject() {
    let run = sb_run("", "release", "acquire");
    let body = sb_unit(&run) + &crate::litmus::protocol("Other", &run);
    let ws = Workspace::new("package-other-protocol-memory");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "NoBothZero"}])),
    );
    let baseline = analyze(&path);
    assert_eq!(
        label(&baseline.item(0)),
        "proved",
        "{}",
        baseline.document()
    );
    let path = litmus_request(
        &ws,
        &body,
        json!({"subject": subject(), "memory": [{"parallel": "Other::Both", "model": "tso"}],
            "items": [{"claim": "NoBothZero"}]}),
    );
    let source = analyze(&path);
    assert_eq!(source.item(0), baseline.item(0), "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}
