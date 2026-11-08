// SPDX-License-Identifier: AGPL-3.0-or-later
//! The QSL boundary for a process provider's hybrid request: it carries the
//! emitted v2 package bytes and the same per-item hybrid limits as the
//! in-process request (QSpec FR-331, QSL FR-250).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_package::{read_hybrids, HybridRead, ReadHybridClaim};
use quire_canonical::Limits;
use quire_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedPackageV2,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

use super::{BudgetRefusal, InconclusiveCause, SolverBudget};
use crate::proof_result::{read_plugin_inconclusive_cause, PluginResultRefusal, TerminalRecord};

/// A package document that may nest as deeply as the admitted package does;
/// dropped without recursing.
struct DeepValue(Value);

impl Drop for DeepValue {
    fn drop(&mut self) {
        qsl_foundation::node_wire::discard_node_json(std::mem::take(&mut self.0));
    }
}

/// `bytes` as a JSON value with serde_json's recursion limit disabled: the
/// package was already admitted under the reader's own limits, so node ids
/// nest as deeply as that admission allows.
fn read_deep_value(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    decoder.disable_recursion_limit();
    let value = <Value as Deserialize>::deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(value)
}

/// QSL-owned admission context for a hybrid process request's checked package.
///
/// The checked-package reader uses bounded local limits. Callers provide the
/// feature and dependency evidence that the package requires without exposing
/// the contract-model reader types at the QSL API boundary.
#[derive(Clone, Debug)]
pub struct HybridProcessPackageContext {
    evidence: CheckedPackageEvidence,
    limits: CheckedPackageReadLimits,
}

impl HybridProcessPackageContext {
    /// Start with bounded package-read limits and no feature or dependency
    /// evidence.
    pub fn new() -> Self {
        Self {
            evidence: CheckedPackageEvidence::new(),
            limits: CheckedPackageReadLimits::bounded(),
        }
    }

    /// Declare a checked-package feature supported by this QSL reader.
    pub fn support_feature(&mut self, feature: impl Into<Box<str>>) {
        self.evidence.support_feature(feature);
    }

    /// Supply the domain-package document selected by the checked package.
    pub fn insert_domain_package_document(
        &mut self,
        digest: impl Into<Box<str>>,
        document: impl Into<Box<[u8]>>,
    ) -> Result<(), HybridProcessRequestRefusal> {
        self.evidence
            .insert_domain_package_document(digest, document)
            .map_err(|_| HybridProcessRequestRefusal::PackageEvidenceConflict)
    }

    /// Admit a checked dependency package using this context's current
    /// evidence. The resulting QSL wrapper can be added under its selected
    /// library identity without exposing the contract-model package type.
    pub fn read_dependency_package(
        &self,
        bytes: &[u8],
    ) -> Result<HybridProcessDependencyPackage, HybridProcessRequestRefusal> {
        admit_package(bytes, self).map(HybridProcessDependencyPackage)
    }

    /// Supply an already admitted dependency package selected by the checked
    /// package.
    pub fn insert_dependency_package(
        &mut self,
        identity: impl Into<Box<str>>,
        package: HybridProcessDependencyPackage,
    ) -> Result<(), HybridProcessRequestRefusal> {
        self.evidence
            .insert_dependency_package(identity, Arc::from(package.0))
            .map_err(|_| HybridProcessRequestRefusal::PackageEvidenceConflict)
    }
}

impl Default for HybridProcessPackageContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A checked dependency package admitted by a
/// [`HybridProcessPackageContext`].
#[derive(Clone, Debug)]
pub struct HybridProcessDependencyPackage(Box<CheckedPackageV2>);

/// A typed hybrid item to include in a process provider's FR-331 request.
#[derive(Debug)]
pub struct HybridProcessItem {
    request_index: RequestIndex,
    claim: Value,
    extent: Value,
    candidates: Value,
    budget: SolverBudget,
}

impl HybridProcessItem {
    /// Construct an item from the claim locus, extent and negotiated
    /// candidates already prepared by the request writer.
    /// The claim's node uses document-held indices, never node-local operand slots.
    pub fn new(
        request_index: RequestIndex,
        claim: Value,
        extent: Value,
        candidates: Value,
        budget: SolverBudget,
    ) -> Self {
        Self {
            request_index,
            claim,
            extent,
            candidates,
            budget,
        }
    }
}

/// Why a process hybrid request could not be composed or read.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum HybridProcessRequestRefusal {
    /// The request JSON or one of its required members is malformed.
    #[error("malformed hybrid provider request")]
    MalformedRequest,
    /// The request carried a non-canonical or invalid v2 package.
    #[error("the request package was not admitted as checked-package/v2")]
    PackageRefused,
    /// The context supplied conflicting evidence for one package identity.
    #[error("the process package context contains conflicting evidence")]
    PackageEvidenceConflict,
    /// The package did not contain the hybrid claim the item names.
    #[error("the item does not name an admitted hybrid claim")]
    ClaimMismatch,
    /// The request's hybrid budget did not follow FR-250.
    #[error("the hybrid budget is invalid")]
    Budget(BudgetRefusal),
    /// A no-initial-state result did not bind to this admitted hybrid item.
    #[error("the no-initial-state result does not bind to its hybrid request item")]
    ResultMismatch,
    /// The plugin result cause reader refused the supplied cause.
    #[error("the plugin result cause was refused: {0}")]
    ResultCause(#[from] PluginResultRefusal),
}

/// The serialized FR-331 request body for a process hybrid provider.
#[derive(Debug, Eq, PartialEq)]
pub struct HybridProcessRequest(Vec<u8>);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProcessRequestWire {
    package: String,
    deadline_ms: u64,
    items: Vec<ProcessRequestItemWire>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProcessRequestItemWire {
    request_index: u64,
    kind: String,
    claim: Value,
    extent: Value,
    candidates: Value,
    requirements: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hybrid: Option<Value>,
}

impl HybridProcessRequest {
    /// Compose the process request body. `package` is the exact emitted v2
    /// byte string; each item carries `hybrid.limits` only for limits the
    /// caller set, leaving provider-published bounds to supply defaults.
    pub fn compose(
        package: &[u8],
        context: &HybridProcessPackageContext,
        deadline_ms: u64,
        items: &[HybridProcessItem],
    ) -> Result<Self, HybridProcessRequestRefusal> {
        admit_package(package, context)?;
        let mut wire_items = Vec::with_capacity(items.len());
        for (position, item) in items.iter().enumerate() {
            let position = u64::try_from(position)
                .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
            let request_index = u64::try_from(item.request_index.get())
                .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
            if request_index != position || request_index > MAX_SAFE_COUNT {
                return Err(HybridProcessRequestRefusal::MalformedRequest);
            }
            let hybrid = item.budget.hybrid_member();
            wire_items.push(ProcessRequestItemWire {
                request_index,
                kind: "temporal-satisfaction".to_owned(),
                claim: item.claim.clone(),
                extent: item.extent.clone(),
                candidates: item.candidates.clone(),
                requirements: vec!["hybrid-dynamics".to_owned()],
                hybrid,
            });
        }
        if wire_items.is_empty() {
            return Err(HybridProcessRequestRefusal::MalformedRequest);
        }
        let request = ProcessRequestWire {
            package: URL_SAFE_NO_PAD.encode(package),
            deadline_ms,
            items: wire_items,
        };
        let request = serde_json::to_value(request)
            .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
        let bytes = quire_canonical::to_vec(&request, Limits::new(u64::MAX))
            .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
        let composed = Self(bytes);
        Self::read(composed.bytes(), context)?;
        Ok(composed)
    }

    /// The canonical request-body bytes to place in FR-331's `request` frame.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    /// Read the process request body, admit the package from its base64url
    /// bytes and bind every item's index and claim node to that package. The
    /// member and kind names are the wire's, so this reader is a string edge.
    #[ix_trace_rs::string_edge]
    pub fn read(
        bytes: &[u8],
        context: &HybridProcessPackageContext,
    ) -> Result<AdmittedHybridProcessRequest, HybridProcessRequestRefusal> {
        let wire: ProcessRequestWire = serde_json::from_slice(bytes)
            .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
        let package_bytes = URL_SAFE_NO_PAD
            .decode(&wire.package)
            .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
        if URL_SAFE_NO_PAD.encode(&package_bytes) != wire.package {
            return Err(HybridProcessRequestRefusal::MalformedRequest);
        }
        let package = admit_package(&package_bytes, context)?;
        let hybrids = read_hybrids(&package, context.limits.nodes)
            .map_err(|_| HybridProcessRequestRefusal::PackageRefused)?;
        let graph = qsl_semantics::check::wire_id::WireGraph::new(
            package
                .resolved_graph()
                .nodes
                .iter()
                .map(|node| &node.node_id),
        )
        .map_err(|_| HybridProcessRequestRefusal::PackageRefused)?;
        let package_wire = DeepValue(
            read_deep_value(&package_bytes)
                .map_err(|_| HybridProcessRequestRefusal::PackageRefused)?,
        );
        if wire.deadline_ms == 0 || wire.deadline_ms > MAX_SAFE_COUNT {
            return Err(HybridProcessRequestRefusal::MalformedRequest);
        }
        if wire.items.is_empty() {
            return Err(HybridProcessRequestRefusal::MalformedRequest);
        }
        let mut admitted_items = Vec::with_capacity(wire.items.len());
        for (position, item) in wire.items.iter().enumerate() {
            let expected_index = u64::try_from(position)
                .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
            let request_index = item.request_index;
            if request_index != expected_index
                || item.kind != "temporal-satisfaction"
                || item.requirements != ["hybrid-dynamics"]
            {
                return Err(HybridProcessRequestRefusal::ClaimMismatch);
            }
            let claim_wire = item
                .claim
                .as_object()
                .filter(|claim| claim.len() == 2)
                .ok_or(HybridProcessRequestRefusal::MalformedRequest)?;
            let claim_occurrence = claim_wire
                .get("occurrence")
                .filter(|occurrence| {
                    occurrence
                        .as_object()
                        .is_some_and(|members| members.len() == 2)
                })
                .ok_or(HybridProcessRequestRefusal::MalformedRequest)?;
            if claim_occurrence["role"] != "claim" {
                return Err(HybridProcessRequestRefusal::ClaimMismatch);
            }
            let package_position = claim_position(
                &graph,
                &package_wire.0["semantic_graph"]["nodes"],
                &item.claim["node"],
                claim_occurrence,
            )?;
            let claim_node = package.resolved_graph().nodes.get(package_position)
                .and_then(|node| graph.key(&node.node_id).ok())
                .ok_or(HybridProcessRequestRefusal::MalformedRequest)?;
            let claim = hybrids
                .claims
                .iter()
                .find(|claim| claim_node == claim.node)
                .ok_or(HybridProcessRequestRefusal::ClaimMismatch)?;
            let budget = match &item.hybrid {
                Some(hybrid) => SolverBudget::read_hybrid(hybrid)
                    .map_err(HybridProcessRequestRefusal::Budget)?,
                None => SolverBudget::from_request(None, None)
                    .map_err(HybridProcessRequestRefusal::Budget)?,
            };
            admitted_items.push(AdmittedHybridProcessItem {
                request_index: RequestIndex::new(
                    usize::try_from(request_index)
                        .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?,
                ),
                claim: claim.clone(),
                budget,
            });
        }
        Ok(AdmittedHybridProcessRequest {
            package,
            hybrids,
            items: admitted_items,
        })
    }
}

/// The checked package and hybrid claims admitted from a process request.
#[derive(Debug)]
pub struct AdmittedHybridProcessRequest {
    package: Box<CheckedPackageV2>,
    hybrids: HybridRead,
    items: Vec<AdmittedHybridProcessItem>,
}

impl AdmittedHybridProcessRequest {
    /// The package admitted from `request.package`.
    pub fn package(&self) -> &CheckedPackageV2 {
        &self.package
    }

    /// The hybrid models and claims read from the admitted package.
    pub fn hybrids(&self) -> &HybridRead {
        &self.hybrids
    }

    /// The request items bound to admitted hybrid claims.
    pub fn items(&self) -> &[AdmittedHybridProcessItem] {
        &self.items
    }

    /// Read a process provider's `no-initial-state` result against the
    /// admitted request item and produce its terminal record. The member and
    /// kind names are the wire's, so this reader is a string edge.
    #[ix_trace_rs::string_edge]
    pub fn read_no_initial_state_result(
        &self,
        entry: &Value,
    ) -> Result<TerminalRecord, HybridProcessRequestRefusal> {
        let index = entry["request_index"]
            .as_u64()
            .filter(|index| *index <= MAX_SAFE_COUNT)
            .ok_or(HybridProcessRequestRefusal::ResultMismatch)?;
        let item_index =
            usize::try_from(index).map_err(|_| HybridProcessRequestRefusal::ResultMismatch)?;
        let item = self
            .items
            .get(item_index)
            .filter(|item| u64::try_from(item.request_index.get()).ok() == Some(index))
            .ok_or(HybridProcessRequestRefusal::ResultMismatch)?;
        let backend = entry["backend"]
            .as_str()
            .and_then(|backend| BackendId::from_wire(backend).ok())
            .ok_or(HybridProcessRequestRefusal::ResultMismatch)?;
        if entry["kind"] != "temporal-satisfaction"
            || entry["result"] != "inconclusive"
            || !self.hybrids.claims.iter().any(|claim| claim == &item.claim)
        {
            return Err(HybridProcessRequestRefusal::ResultMismatch);
        }
        let cause = read_plugin_inconclusive_cause(
            entry
                .get("cause")
                .ok_or(HybridProcessRequestRefusal::ResultMismatch)?,
        )?;
        if cause != InconclusiveCause::NoInitialState {
            return Err(HybridProcessRequestRefusal::ResultMismatch);
        }
        TerminalRecord::from_plugin_inconclusive(item.request_index, backend, cause)
            .map_err(HybridProcessRequestRefusal::ResultCause)
    }
}

/// One item whose claim was resolved by the package reader.
#[derive(Debug)]
pub struct AdmittedHybridProcessItem {
    request_index: RequestIndex,
    claim: ReadHybridClaim,
    budget: SolverBudget,
}

impl AdmittedHybridProcessItem {
    /// The FR-331 request index.
    pub fn request_index(&self) -> RequestIndex {
        self.request_index
    }

    /// The checked claim read from the package.
    pub fn claim(&self) -> &ReadHybridClaim {
        &self.claim
    }

    /// The limits the request supplied; absent fields remain unset.
    pub fn budget(&self) -> SolverBudget {
        self.budget
    }
}

fn admit_package(
    bytes: &[u8],
    context: &HybridProcessPackageContext,
) -> Result<Box<CheckedPackageV2>, HybridProcessRequestRefusal> {
    match read_checked_package(bytes, context.limits, &context.evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => Ok(package),
        CheckedPackageDispatchResult::Refused(_)
        | CheckedPackageDispatchResult::Incomplete(_)
        | CheckedPackageDispatchResult::CatalogUnavailable(_) => {
            Err(HybridProcessRequestRefusal::PackageRefused)
        }
    }
}

/// Resolve a document-held request claim through the admitted graph (FR-322).
/// Node-local slot IDs cannot identify a claim without their holding row.
/// Member and form names are wire vocabulary, so this is a string edge.
#[ix_trace_rs::string_edge]
fn claim_position(
    graph: &qsl_semantics::check::wire_id::WireGraph,
    nodes: &Value,
    claim_node: &Value,
    claim_occurrence: &Value,
) -> Result<usize, HybridProcessRequestRefusal> {
    let id: quire_contract_model::CheckedNodeId = serde_json::from_str(&claim_node.to_string())
        .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
    let key = graph.key(&id).map_err(|_| HybridProcessRequestRefusal::ClaimMismatch)?;
    let position = graph.index_of(&key).and_then(|index| usize::try_from(index).ok())
        .ok_or(HybridProcessRequestRefusal::ClaimMismatch)?;
    let node = nodes.as_array().and_then(|nodes| nodes.get(position))
        .ok_or(HybridProcessRequestRefusal::ClaimMismatch)?;
    if node["semantic_form"] != "temporal_clause"
        || graph.wire_value(&key).map_err(|_| HybridProcessRequestRefusal::MalformedRequest)? != *claim_node
        || !node["occurrences"].as_array().is_some_and(|rows| rows.contains(claim_occurrence))
    {
        return Err(HybridProcessRequestRefusal::ClaimMismatch);
    }
    Ok(position)
}

const MAX_SAFE_COUNT: u64 = (1_u64 << 53) - 1;

#[cfg(test)]
mod deep_package_tests {
    use super::*;

    #[test]
    fn a_package_document_nesting_past_the_library_limit_reads() {
        let depth = 300;
        let text = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        assert!(serde_json::from_slice::<Value>(text.as_bytes()).is_err());
        let value = DeepValue(read_deep_value(text.as_bytes()).expect("reads"));
        assert!(value.0.is_array());
    }
}

#[cfg(test)]
mod claim_position_tests {
    use super::*;
    use qsl_semantics::check::wire_id::WireGraph;
    use serde_json::json;

    fn fixture() -> (Value, WireGraph) {
        let id = json!({"package": "p", "path": ["clause", {"operand": 0}], "ordinal": 0});
        let clause = json!({"node_id": id, "semantic_form": "temporal_clause",
            "occurrences": [{"role": "claim", "ordinal": 0}]});
        let nodes = json!([clause.clone(), clause,
            {"node_id": {"package": "p", "path": ["a"], "ordinal": 0}},
            {"node_id": {"package": "p", "path": ["b"], "ordinal": 0}}]);
        let graph = WireGraph::of_package_value(&json!({"semantic_graph": {
            "nodes": nodes, "operand_rows": [[2], [3], [], []]
        }})).unwrap();
        (nodes, graph)
    }

    #[test]
    fn an_unknown_claim_is_a_mismatch_and_an_unparseable_claim_is_malformed() {
        let (nodes, graph) = fixture();
        let occurrence = json!({"role": "claim", "ordinal": 0});
        assert_eq!(claim_position(&graph, &nodes,
            &json!({"package": "p", "path": ["missing"], "ordinal": 0}), &occurrence),
            Err(HybridProcessRequestRefusal::ClaimMismatch));
        assert_eq!(claim_position(&graph, &nodes, &json!("not-an-id"), &occurrence),
            Err(HybridProcessRequestRefusal::MalformedRequest));
    }

    #[test]
    fn equal_raw_ids_require_the_document_identity_and_the_claim_occurrence() {
        let (nodes, graph) = fixture();
        let occurrence = json!({"role": "claim", "ordinal": 0});
        assert_eq!(nodes[0]["node_id"], nodes[1]["node_id"]);
        assert_eq!(claim_position(&graph, &nodes, &nodes[0]["node_id"], &occurrence),
            Err(HybridProcessRequestRefusal::ClaimMismatch));
        for position in 0..2 {
            let id = graph.wire_value(&graph.keys()[position]).unwrap();
            assert_eq!(claim_position(&graph, &nodes, &id, &occurrence), Ok(position));
            let mut wrong_role = nodes.clone();
            wrong_role[position]["occurrences"][0]["role"] = json!("anchor");
            assert_eq!(claim_position(&graph, &wrong_role, &id, &occurrence),
                Err(HybridProcessRequestRefusal::ClaimMismatch));
        }
    }
}
