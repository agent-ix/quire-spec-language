// SPDX-License-Identifier: AGPL-3.0-or-later
//! The composition fixture (QSpec TC-983, TC-985): the `Producer` and
//! `Consumer` object types of one model. `Producer` has the fields `sent`
//! and `sent_count` and the operation `send`; `Consumer` has `received`,
//! `done` and `received_count` and the operation `receive`. `producers` and
//! `consumers` are the populations.

#![allow(dead_code, reason = "each test binary uses part of the fixture")]

use qsl_semantics::model::intake::meaning;
use serde_json::{json, Value};

#[path = "tc235_k2.rs"]
#[allow(dead_code, reason = "this fixture uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary also loads the K2 builder directly"
)]
mod base;
pub(crate) use base::id;

const INTEGER: &str = "ix://quire/native/Integer";
const BOOLEAN: &str = "ix://quire/native/Boolean";

/// The operation `name` of `owner`, framed on `modifies`.
fn operation(owner: &str, name: &str, modifies: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "name": name, "params": [], "pre": [], "post": [],
        "frame": {"modifies": [id(&format!("{owner}/{modifies}"))], "creates": [], "deletes": []},
        "origin": base::origin(&node),
    })
}

/// The model document with `Producer` and `Consumer`.
pub(crate) fn document() -> Value {
    let producer = base::declaration(
        "Producer",
        "thing",
        json!({
            "supertypes": [],
            "identityFields": [id("Producer/producer_id")],
            "fields": [
                base::field("Producer", "producer_id", INTEGER),
                base::field("Producer", "sent", BOOLEAN),
                base::field("Producer", "sent_count", INTEGER),
            ],
            "operations": [operation("Producer", "send", "sent_count")],
        }),
    );
    let consumer = base::declaration(
        "Consumer",
        "thing",
        json!({
            "supertypes": [],
            "identityFields": [id("Consumer/consumer_id")],
            "fields": [
                base::field("Consumer", "consumer_id", INTEGER),
                base::field("Consumer", "received", BOOLEAN),
                base::field("Consumer", "done", BOOLEAN),
                base::field("Consumer", "received_count", INTEGER),
            ],
            "operations": [operation("Consumer", "receive", "received_count")],
        }),
    );
    let mut document = base::domain_document(
        vec![producer, consumer],
        vec![
            base::construct("thing", meaning::OBJECT_TYPE),
            base::construct("population", meaning::POPULATION),
        ],
    );
    document["populations"] = json!([
        {
            "identity": id("producers"), "displayName": id("producers"),
            "kind": {"module": base::BUSINESS, "name": "population"},
            "members": [id("Producer")], "extent": "closed",
            "origin": base::origin("producers"),
        },
        {
            "identity": id("consumers"), "displayName": id("consumers"),
            "kind": {"module": base::BUSINESS, "name": "population"},
            "members": [id("Consumer")], "extent": "closed",
            "origin": base::origin("consumers"),
        },
    ]);
    document
}
