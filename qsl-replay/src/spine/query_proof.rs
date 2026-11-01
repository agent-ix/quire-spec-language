// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-046-AC-5: classify checked ordered-query values at the proof transfer
//! boundary. The checked package is the authority for both the query form
//! and its result representation.

use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedNode, NodeId, NodeKind, Visit};
use quire_exact::{NodeKey, ValueType};

/// The typed proof-transfer outcome for a selected checked clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryProofDisposition {
    /// No map/filter collection value occurs in the selected clause.
    NoCollectionQuery,
    /// The checked collection representation has no supported proof-value
    /// transfer. No value or evaluation charge is produced by this boundary.
    Unsupported(Box<UnsupportedQueryProof>),
}

/// A checked query representation the proof route cannot transfer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedQueryProof {
    /// The typed reason for the unsupported proof transfer.
    pub cause: QueryProofCause,
    /// The checked query form and result type selected from the clause graph.
    pub query: CheckedQueryProof,
}

/// Why a checked query cannot be transferred as a proof value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryProofCause {
    /// The checked result's value representation has no proof transfer.
    ValueRepresentation,
}

/// The exact checked query form and its inferred result representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedQueryProof {
    /// Exact S3 identity of the containing checked clause.
    pub clause: NodeKey,
    /// Node position of the selected query in that clause's checked body.
    pub node: NodeId,
    /// The ordered query constructor.
    pub visit: Visit,
    /// The result type retained by the checked node.
    pub result_type: ValueType,
}

/// Why the requested clause could not be resolved in the checked package.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum QueryProofSelectionError {
    /// The checked package has no state clause with this authored name.
    #[error("checked package has no state clause named {0}")]
    MissingClause(String),
}

/// Select the proof-value boundary for `clause_name` from the checked graph.
///
/// Map/filter nodes are unsupported only when their checked result type is a
/// collection. The function reads no source text and accepts no caller-made
/// proof report, outcome, value, or charge table.
pub fn select_query_proof(
    package: &CheckedPackage,
    clause_name: &str,
) -> Result<QueryProofDisposition, QueryProofSelectionError> {
    let clause = package
        .graph()
        .state_clause(clause_name)
        .ok_or_else(|| QueryProofSelectionError::MissingClause(clause_name.to_owned()))?;
    let Some(query) = unsupported_collection_query(clause.identity(), clause.body()) else {
        return Ok(QueryProofDisposition::NoCollectionQuery);
    };
    Ok(QueryProofDisposition::Unsupported(Box::new(
        UnsupportedQueryProof {
            cause: QueryProofCause::ValueRepresentation,
            query,
        },
    )))
}

fn unsupported_collection_query(
    clause: NodeKey,
    root: CheckedNode<'_>,
) -> Option<CheckedQueryProof> {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if let NodeKind::Query {
            visit: visit @ (Visit::Map | Visit::Filter),
            ..
        } = node.kind()
        {
            if matches!(node.value_type(), ValueType::Collection(_)) {
                return Some(CheckedQueryProof {
                    clause,
                    node: node.id(),
                    visit: *visit,
                    result_type: node.value_type().clone(),
                });
            }
        }
        pending.extend(node.children().into_iter().rev());
    }
    None
}
