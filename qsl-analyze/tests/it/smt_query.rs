// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 through the `qsl-replay` facade: `encode_smt_query` over the
//! `Counter` subject with the item the engine decides.

use ix_trace_rs::trace;
use qsl_eval::model_subject::ModelSubject;
use qsl_replay::{encode_smt_query, ModelCheckItem, SmtEncodingRefusal, SmtQueryKind};

use crate::counter::{package_with_clauses, subject, Documents, Letters, CONTRACT, UNLIMITED};

fn world(
    formula: &'static str,
) -> (
    qsl_package::CheckedPackage,
    Vec<qsl_semantics::check::CheckedTemporalClause>,
    Documents,
) {
    let documents = Documents::new(crate::counter::document(false));
    let (package, clauses) =
        package_with_clauses(&documents.document, CONTRACT, &[("C", "", formula)]);
    (package, clauses, documents)
}

fn bytes(
    subject: &ModelSubject<'_>,
    clause: &qsl_semantics::check::CheckedTemporalClause,
    kind: SmtQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    let letters = Letters { atom: |_| None };
    encode_smt_query(
        subject,
        ModelCheckItem::Clause {
            clause,
            letters: &letters,
        },
        kind,
    )
}

/// The facade encodes the engine's item, byte-equal across calls, and
/// refuses the deadlock-freedom item without a script.
#[trace("TC-900", "FR-315-AC-1", "FR-315-AC-2")]
#[test]
fn the_facade_encodes_the_engine_s_item_and_refuses_the_ones_it_cannot() {
    let (package, clauses, mut documents) = world("always holds(c.value <= 3)");
    let start = documents.start("s0", &[("c", 0)]);
    let subject = subject(&package, &documents, vec![start], &["c"], UNLIMITED);
    let kind = SmtQueryKind::InductionBase { depth: 1 };
    let first = bytes(&subject, &clauses[0], kind).expect("encodes");
    let text = String::from_utf8(first.clone()).expect("UTF-8");
    assert!(text.starts_with("(set-logic QF_LIA)\n"), "{text}");
    assert!(text.contains("(assert (= s0.c.value 0))\n"), "{text}");
    assert_eq!(first, bytes(&subject, &clauses[0], kind).expect("encodes"));
    assert!(matches!(
        encode_smt_query(&subject, ModelCheckItem::DeadlockFreedom, kind),
        Err(SmtEncodingRefusal::ItemForm { .. })
    ));
}
