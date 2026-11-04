// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-040-AC-11: QSL protocol check expressions checked by the cycle-free IR
//! semantic model with an explicit QSL-to-formal correspondence.

use qsl_foundation::{SourceIdentity, Span};
use quire_contract_model::{
    ClauseRef, DeclarationEnvironment, Diagnostic, ExecutionPoint, Expression, TypedExpression,
    ValueType,
};

use super::CheckedUnit;

/// Whether a protocol's value-definedness proof completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolProofDisposition {
    /// Every check expression was typed and its definedness obligations were
    /// discharged under the exact supplied correspondence.
    Discharged,
    /// A source, clause, execution-point, or expression refusal remains.
    Refused,
    /// A check clause or exact correspondence was not supplied.
    Unfinished,
}

/// Typed cause retained when one protocol proof cannot be discharged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolProofCause {
    /// No mapping was supplied for an authored QSL check clause.
    MissingCorrespondence {
        /// Authored QSL check clause missing a mapping.
        clause: String,
    },
    /// More than one mapping selected the same authored QSL check clause.
    DuplicateCorrespondence {
        /// Authored QSL check clause with duplicate mappings.
        clause: String,
    },
    /// A mapping's QSL source owner differs from the checked source.
    ForeignSource {
        /// Check clause whose QSL source mapping is foreign.
        clause: String,
    },
    /// A mapping selects a clause name that the protocol does not author.
    ForeignClause {
        /// Unauthored check clause selected by the mapping.
        clause: String,
    },
    /// The formal clause belongs to a different requirement environment.
    ClauseOwner {
        /// Check clause mapped to a formal clause owned by another environment.
        clause: String,
    },
    /// The formal clause identity does not correspond to the QSL check name.
    ClauseIdentity {
        /// Check clause mapped to a different formal clause identity.
        clause: String,
    },
    /// The selected execution point is not the supplied Handler point.
    ExecutionPoint {
        /// Check clause mapped to a non-Handler execution point.
        clause: String,
    },
    /// The formal expression source differs from the exact formal source id.
    FormalSource {
        /// Check clause mapped to an expression from another formal source.
        clause: String,
    },
    /// The IR semantic model refused the Boolean expression or a proof goal.
    Expression {
        /// The authored QSL check clause.
        clause: String,
        /// Typed diagnostics returned by the semantic model.
        diagnostics: Vec<Diagnostic>,
    },
}

/// One explicit QSL source/clause to formal IR clause/Handler correspondence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolProofCorrespondence {
    /// QSL source owner of the native protocol declaration.
    pub qsl_source: SourceIdentity,
    /// Authored QSL protocol name.
    pub protocol: String,
    /// Authored QSL `check` clause name.
    pub clause: String,
    /// Exact source region of the QSL check expression.
    pub qsl_region: Span,
    /// Formal source identity for the supplied IR expression.
    pub formal_source: quire_contract_model::SourceIdentity,
    /// Exact formal clause identity selected for the check.
    pub formal_clause: ClauseRef,
    /// Exact handler execution point at which the formal clause applies.
    pub execution_point: ExecutionPoint,
    /// Formal declaration environment against which the expression is typed.
    pub environment: DeclarationEnvironment,
    /// The formal Boolean expression whose definedness is checked.
    pub expression: Expression,
}

/// Inputs for one spine protocol value-proof operation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProtocolProofRequest {
    /// Exact source/clause/handler mappings in caller order.
    pub correspondences: Vec<ProtocolProofCorrespondence>,
}

/// Per-protocol proof disposition, actual checked goals, and typed causes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolProofResult {
    /// Authored QSL protocol name.
    pub protocol: String,
    /// Whether all mapped check expressions were discharged.
    pub disposition: ProtocolProofDisposition,
    /// Actual typed formal expressions and their definedness obligations.
    pub goals: Vec<TypedExpression>,
    /// Typed correspondence or expression causes.
    pub causes: Vec<ProtocolProofCause>,
}

impl CheckedUnit {
    /// Check protocol value definedness through `quire-contract-model` while
    /// retaining exact source, clause and Handler correspondence. The result
    /// is evidence only; it does not admit or execute a protocol.
    pub fn prove_protocols(&self, request: &ProtocolProofRequest) -> Vec<ProtocolProofResult> {
        prove_protocols(self, request)
    }
}

fn prove_protocols(
    checked: &CheckedUnit,
    request: &ProtocolProofRequest,
) -> Vec<ProtocolProofResult> {
    let mut results = Vec::new();
    let mut used = vec![false; request.correspondences.len()];
    let mut protocols: Vec<String> = checked
        .protocol_check_sites
        .iter()
        .map(|site| site.protocol.clone())
        .collect();
    protocols.sort();
    protocols.dedup();

    for protocol in protocols {
        let sites: Vec<_> = checked
            .protocol_check_sites
            .iter()
            .filter(|site| site.protocol == protocol)
            .collect();
        let mut goals = Vec::new();
        let mut causes = Vec::new();
        for site in sites {
            let candidates: Vec<_> = request
                .correspondences
                .iter()
                .enumerate()
                .filter(|(_, mapping)| {
                    mapping.protocol == site.protocol && mapping.qsl_region == site.region
                })
                .collect();
            let [(index, mapping)] = candidates.as_slice() else {
                causes.push(if candidates.is_empty() {
                    ProtocolProofCause::MissingCorrespondence {
                        clause: site.clause.clone(),
                    }
                } else {
                    ProtocolProofCause::DuplicateCorrespondence {
                        clause: site.clause.clone(),
                    }
                });
                continue;
            };
            used[*index] = true;
            if mapping.qsl_source != *checked.source().identity() {
                causes.push(ProtocolProofCause::ForeignSource {
                    clause: site.clause.clone(),
                });
                continue;
            }
            if mapping.clause != site.clause {
                causes.push(ProtocolProofCause::ForeignClause {
                    clause: mapping.clause.clone(),
                });
                continue;
            }
            if mapping.environment.owner() != mapping.formal_clause.requirement()
                || mapping.formal_clause.requirement().requirement().as_str() != site.protocol
            {
                causes.push(ProtocolProofCause::ClauseOwner {
                    clause: site.clause.clone(),
                });
                continue;
            }
            if mapping.formal_clause.clause().as_str() != site.clause.to_ascii_lowercase() {
                causes.push(ProtocolProofCause::ClauseIdentity {
                    clause: site.clause.clone(),
                });
                continue;
            }
            if !matches!(&mapping.execution_point, ExecutionPoint::Handler { .. }) {
                causes.push(ProtocolProofCause::ExecutionPoint {
                    clause: site.clause.clone(),
                });
                continue;
            }
            let formal_expression_source = mapping.expression.source();
            if formal_expression_source.start().source() != &mapping.formal_source
                || formal_expression_source.end().source() != &mapping.formal_source
            {
                causes.push(ProtocolProofCause::FormalSource {
                    clause: site.clause.clone(),
                });
                continue;
            }
            match mapping.environment.check_expression(
                &mapping.expression,
                &ValueType::Boolean,
                &mapping.execution_point,
                true,
            ) {
                Ok(goal) => goals.push(goal),
                Err(diagnostics) => causes.push(ProtocolProofCause::Expression {
                    clause: site.clause.clone(),
                    diagnostics,
                }),
            }
        }
        results.push(ProtocolProofResult {
            protocol,
            disposition: if !causes.is_empty() {
                ProtocolProofDisposition::Refused
            } else if goals.is_empty() {
                ProtocolProofDisposition::Unfinished
            } else {
                ProtocolProofDisposition::Discharged
            },
            goals,
            causes,
        });
    }

    for (index, mapping) in request.correspondences.iter().enumerate() {
        if used[index] {
            continue;
        }
        let protocol_exists = checked
            .protocol_check_sites
            .iter()
            .any(|site| site.protocol == mapping.protocol);
        let cause = if !protocol_exists
            || checked
                .protocol_check_sites
                .iter()
                .any(|site| site.protocol == mapping.protocol && site.clause == mapping.clause)
        {
            ProtocolProofCause::ForeignSource {
                clause: mapping.clause.clone(),
            }
        } else {
            ProtocolProofCause::ForeignClause {
                clause: mapping.clause.clone(),
            }
        };
        if let Some(result) = results
            .iter_mut()
            .find(|result| result.protocol == mapping.protocol)
        {
            result.causes.push(cause);
            result.disposition = ProtocolProofDisposition::Refused;
        } else {
            results.push(ProtocolProofResult {
                protocol: mapping.protocol.clone(),
                disposition: ProtocolProofDisposition::Refused,
                goals: Vec::new(),
                causes: vec![cause],
            });
        }
    }
    results
}
