// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315-AC-14: a CG-shaped caller whose only QSL dependency is
//! `qsl-replay` names the hyper query types and calls the identity and the
//! encoder through it.

use qsl_replay::{
    encode_hyper_query, HyperProductIdentity, HyperQuery, HyperQueryRequest, SmtEncodingLimits,
    SmtEncodingRefusal, SmtQueryKind, SubjectIdentity,
};
use quire_exact::Meter;

/// The request again, with each of its five public members set.
pub fn reissued<'a>(
    request: &HyperQueryRequest<'a>,
    meter: &'a mut Meter,
    encoding: SmtEncodingLimits,
) -> HyperQueryRequest<'a> {
    HyperQueryRequest {
        clause: request.clause,
        subjects: request.subjects,
        instance: request.instance,
        meter,
        encoding,
    }
}

/// The identity of `request`, then its query of `kind`, which carries an
/// equal identity.
///
/// # Errors
///
/// The refusal of either call.
pub fn identified_query(
    request: HyperQueryRequest<'_>,
    kind: SmtQueryKind,
) -> Result<(HyperProductIdentity, HyperQuery), SmtEncodingRefusal> {
    let identity = HyperProductIdentity::of(&request)?;
    let query = encode_hyper_query(request, kind)?;
    Ok((identity, query))
}

/// FR-336's projection of each subject of an identity.
pub fn subjects(identity: &HyperProductIdentity) -> &[SubjectIdentity] {
    identity.subjects()
}
