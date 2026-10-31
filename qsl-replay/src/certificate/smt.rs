// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-314 (ADR-018 PC-6, LA-3): the SMT proof certificate of a refinement's
//! per-step simulation obligations, and [`check_refinement_smt_proof`], the
//! core's check of it.
//!
//! The checker reads nothing from the solver but the certificate. It applies
//! FR-314's rules in this order, each to the base part and then to the step
//! parts in row order before the next rule, and reports the first that
//! fails:
//!
//! 1. `ShapeMismatch`: the certificate holds fewer or more step parts than
//!    the refinement has step rows;
//! 2. `QueryMismatch`: a carried query differs, byte for byte, from the one
//!    FR-315 encodes for its part; a row whose query FR-315 refuses has no
//!    expected query, and its part is neither compared nor checked, and a
//!    refused initial query rejects the base part;
//! 3. the proof rules of [`alethe`](super::alethe): `Malformed`,
//!    `NotRefutation` for a proof of no command, `ProofStepInvalid`, then
//!    `NotRefutation` for a last command that does not conclude the empty
//!    clause;
//! 4. `Unverifiable` at the first command whose rule is unchecked, or
//!    `Unencodable` at a refused part that precedes it.
//!
//! Every byte of a carried query and proof and of an expected query is
//! charged to `max_certificate_bytes`, and every command and premise or
//! discharge entry to `max_certificate_edges`, before the work it pays for;
//! the `Cancel` handle is polled at every charge.

use std::cell::Cell;

use qsl_eval::smt::{
    encode_refinement_query, RefinementCertificateRequest, RefinementQueryKind, SmtEncodingRefusal,
};
use quire_exact::Cancel;
use quire_semantic_value::catalog::InternalFault;

use super::alethe::{read_proof, read_query, Proof, ProofOutcome, Query, Reading};
use super::closure::CertificateRejection;
use super::zone_certificate::CertificateCheckLimits;
use super::{CertificateLocus, CertificateRule, QueryPart};
use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::IncompleteCause;

/// The proof format of an SMT proof certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofFormat {
    /// cvc5's Alethe proof format.
    Alethe,
}

/// One refuted query: the SMT-LIB 2 script and the solver's refutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefutedQuery {
    /// The SMT-LIB 2 script, in FR-315's canonical printing.
    pub query: Vec<u8>,
    /// The solver's refutation in the certificate's format.
    pub proof: Vec<u8>,
}

/// A refinement's per-step simulation obligations (QSpec FR-468): the
/// refuted initial query and one refuted step query per step row, in
/// declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementSmtCertificate {
    /// The proofs' format.
    pub format: ProofFormat,
    /// The initial query's part.
    pub base: RefutedQuery,
    /// One part per step row.
    pub steps: Vec<RefutedQuery>,
}

/// What an SMT proof check that rejects nothing returns (FR-314).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SmtProofCheck {
    /// Every proof refutes its query with checked rules alone.
    Verified,
    /// No rule rejects, and the command numbered `step` of `part`'s proof
    /// is the first whose rule is unchecked.
    Unverifiable {
        /// The part.
        part: QueryPart,
        /// The command's number in the part's proof.
        step: u64,
    },
    /// No rule rejects, and FR-315 refuses to encode the query of `part`.
    Unencodable {
        /// The part.
        part: QueryPart,
    },
    /// A limit was reached or the check was cancelled.
    Stopped(IncompleteCause),
    /// FR-315 faulted rebuilding a query, or the checker broke an
    /// invariant of its own.
    Fault(InternalFault),
}

/// Why the check ended before its verdict.
enum Halt {
    Rejected(CertificateRejection),
    Check(SmtProofCheck),
}

impl From<CertificateRejection> for Halt {
    fn from(rejection: CertificateRejection) -> Self {
        Self::Rejected(rejection)
    }
}

fn reject(rule: CertificateRule, at: CertificateLocus) -> Halt {
    Halt::Rejected(CertificateRejection { rule, at })
}

fn fault(invariant: &'static str) -> Halt {
    Halt::Check(SmtProofCheck::Fault(InternalFault::new(
        "smt-proof-check",
        invariant,
    )))
}

/// The two certificate budgets and the cancellation handle.
struct Budget<'c> {
    limits: CertificateCheckLimits,
    bytes: Cell<u64>,
    edges: Cell<u64>,
    cancel: &'c Cancel,
}

impl Budget<'_> {
    fn poll(&self) -> Result<(), Halt> {
        if self.cancel.poll() {
            let Some(source) = self.cancel.cause() else {
                return Err(fault("a cancelled handle has a cause"));
            };
            return Err(Halt::Check(SmtProofCheck::Stopped(
                IncompleteCause::Cancelled { source },
            )));
        }
        Ok(())
    }

    fn charge(
        &self,
        counter: &Cell<u64>,
        limit: ModelCheckLimit,
        value: u64,
        count: u64,
    ) -> Result<(), Halt> {
        self.poll()?;
        let total = counter.get().saturating_add(count);
        if total > value {
            return Err(Halt::Check(SmtProofCheck::Stopped(
                IncompleteCause::LimitReached { limit, value },
            )));
        }
        counter.set(total);
        Ok(())
    }

    fn bytes(&self, count: usize) -> Result<(), Halt> {
        self.charge(
            &self.bytes,
            ModelCheckLimit::CertificateMaxBytes,
            self.limits.max_certificate_bytes,
            u64::try_from(count).unwrap_or(u64::MAX),
        )
    }

    fn edges(&self, count: u64) -> Result<(), Halt> {
        self.charge(
            &self.edges,
            ModelCheckLimit::CertificateMaxEdges,
            self.limits.max_certificate_edges,
            count,
        )
    }
}

/// One part of the certificate, with what the checker learned of it.
struct Part<'c> {
    part: QueryPart,
    carried: &'c RefutedQuery,
    /// The expected query, read; `None` for a part FR-315 refuses.
    query: Option<Query>,
}

/// Check `certificate` for the refinement item `request` (FR-314
/// "Refinement certificates").
///
/// # Errors
///
/// [`CertificateRejection`] naming the first rule that fails and its part,
/// or the proof command at which it fails.
pub fn check_refinement_smt_proof(
    request: &RefinementCertificateRequest<'_>,
    certificate: &RefinementSmtCertificate,
    limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<SmtProofCheck, CertificateRejection> {
    let budget = Budget {
        limits: *limits,
        bytes: Cell::new(0),
        edges: Cell::new(0),
        cancel,
    };
    match check(request, certificate, &budget) {
        Ok(check) => Ok(check),
        Err(Halt::Check(check)) => Ok(check),
        Err(Halt::Rejected(rejection)) => Err(rejection),
    }
}

fn check(
    request: &RefinementCertificateRequest<'_>,
    certificate: &RefinementSmtCertificate,
    budget: &Budget<'_>,
) -> Result<SmtProofCheck, Halt> {
    let ProofFormat::Alethe = certificate.format;
    // Rule 1: the shape.
    let rows = step_rows(request)?;
    let present = u64::try_from(certificate.steps.len()).unwrap_or(u64::MAX);
    if present != rows {
        return Err(reject(
            CertificateRule::ShapeMismatch,
            CertificateLocus::Query {
                part: QueryPart::Step {
                    row: Some(present.min(rows)),
                },
            },
        ));
    }
    // Rule 2: each carried query is the expected one.
    let kinds = std::iter::once((QueryPart::Base, RefinementQueryKind::Initial)).chain(
        (0..rows).map(|row| {
            (
                QueryPart::Step { row: Some(row) },
                RefinementQueryKind::Step { row },
            )
        }),
    );
    let carried = std::iter::once(&certificate.base).chain(&certificate.steps);
    let mut parts = Vec::with_capacity(certificate.steps.len() + 1);
    for ((part, kind), carried) in kinds.zip(carried) {
        let query = match encode_refinement_query(request, kind) {
            Ok(expected) => {
                budget.bytes(expected.len())?;
                budget.bytes(carried.query.len())?;
                if carried.query != expected {
                    return Err(reject(
                        CertificateRule::QueryMismatch,
                        CertificateLocus::Query { part },
                    ));
                }
                Some(read_query(&expected).ok_or_else(|| fault("a printed query reads back"))?)
            }
            // A refused row has no expected query (FR-315 "Refinement
            // queries"); a refused initial query rejects the base part
            // `QueryMismatch`, as FR-314 rejects any part FR-315 prints no
            // query for.
            Err(SmtEncodingRefusal::Refused(_)) if part == QueryPart::Base => {
                return Err(reject(
                    CertificateRule::QueryMismatch,
                    CertificateLocus::Query { part },
                ));
            }
            Err(SmtEncodingRefusal::Refused(_)) => None,
            Err(SmtEncodingRefusal::Fault(fault)) => {
                return Err(Halt::Check(SmtProofCheck::Fault(fault)));
            }
        };
        parts.push(Part {
            part,
            carried,
            query,
        });
    }
    // Rule 3: the proofs, rule by rule across the parts.
    let mut proofs: Vec<(QueryPart, &Query, Proof)> = Vec::new();
    for part in &parts {
        let Some(query) = &part.query else { continue };
        budget.bytes(part.carried.proof.len())?;
        let mut stop = None;
        let read = read_proof(
            &part.carried.proof,
            query,
            &mut |count| match budget.edges(count) {
                Ok(()) => true,
                Err(halt) => {
                    stop = Some(halt);
                    false
                }
            },
        );
        let read = match read {
            Reading::Read(proof) => Some(proof),
            Reading::Malformed => None,
            Reading::Stopped => {
                return Err(stop.unwrap_or_else(|| fault("a stopped proof read names its stop")));
            }
        };
        let Some(proof) = read else {
            return Err(reject(
                CertificateRule::Malformed,
                CertificateLocus::Query { part: part.part },
            ));
        };
        proofs.push((part.part, query, proof));
    }
    if let Some((part, _, _)) = proofs.iter().find(|(_, _, proof)| proof.len() == 0) {
        return Err(reject(
            CertificateRule::NotRefutation,
            CertificateLocus::Query { part: *part },
        ));
    }
    for (part, query, proof) in &proofs {
        if let Some(index) = proof.invalid_step(query) {
            return Err(reject(
                CertificateRule::ProofStepInvalid,
                CertificateLocus::ProofStep { part: *part, index },
            ));
        }
    }
    for (part, _, proof) in &proofs {
        if let Some(index) = proof.not_refutation() {
            return Err(reject(
                CertificateRule::NotRefutation,
                CertificateLocus::ProofStep { part: *part, index },
            ));
        }
    }
    // Rule 4: the first unchecked command or refused part.
    let mut checked = proofs.iter();
    for part in &parts {
        if part.query.is_none() {
            return Ok(SmtProofCheck::Unencodable { part: part.part });
        }
        let Some((_, query, proof)) = checked.next() else {
            return Err(fault("every encodable part has a proof"));
        };
        if let ProofOutcome::Unverifiable { step } = proof.outcome(query) {
            return Ok(SmtProofCheck::Unverifiable {
                part: part.part,
                step,
            });
        }
    }
    Ok(SmtProofCheck::Verified)
}

/// The number of step rows of the refinement `request` names.
fn step_rows(request: &RefinementCertificateRequest<'_>) -> Result<u64, Halt> {
    request
        .package
        .graph()
        .refinements()
        .iter()
        .find_map(|refinement| {
            let emission = refinement.v2_emission().ok()?;
            (emission.identity == request.refinement).then_some(refinement.steps().len())
        })
        .and_then(|rows| u64::try_from(rows).ok())
        .ok_or_else(|| fault("the request names a refinement of its package"))
}
