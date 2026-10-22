// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 (ADR-018 LA-3, PC-6): the facade entry through which CG's SMT
//! backend builds the proof queries it sends to the solver and FR-314's
//! checker rebuilds the queries a certificate claims to refute. The encoding
//! is `qsl-eval`'s ([`qsl_eval::smt`]); this entry maps the item the engine
//! decides onto it and adds nothing.

use qsl_eval::model_subject::ModelSubject;
use qsl_eval::smt::SmtItem;
pub use qsl_eval::smt::{SmtEncodingRefusal, SmtQueryKind};

use crate::model_check::item::ModelCheckItem;

/// FR-315: encode the proof query `kind` of `item` over `subject` as an
/// SMT-LIB 2 script in canonical printing. Equal inputs give byte-equal
/// scripts.
///
/// # Errors
///
/// [`SmtEncodingRefusal`] naming the construct, with its locus, when the item
/// or the subject uses a construct the encoding has no form for; no script is
/// written.
pub fn encode_smt_query(
    subject: &ModelSubject<'_>,
    item: ModelCheckItem<'_>,
    kind: SmtQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    let item = match item {
        ModelCheckItem::Clause { clause, .. } => SmtItem::Clause(clause),
        ModelCheckItem::DeadlockFreedom => SmtItem::DeadlockFreedom,
    };
    qsl_eval::smt::encode_smt_query(subject, item, kind)
}
