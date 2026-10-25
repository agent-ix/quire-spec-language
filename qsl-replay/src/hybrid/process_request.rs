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
use thiserror::Error;

use super::{BudgetRefusal, InconclusiveCause, SolverBudget};
use crate::proof_result::{read_plugin_inconclusive_cause, PluginResultRefusal, TerminalRecord};

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
    /// The request's package identity did not match the admitted package.
    #[error("the request package identity does not match its package bytes")]
    PackageIdentityMismatch,
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
    package_id: Value,
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
        evidence: &CheckedPackageEvidence,
        package_limits: CheckedPackageReadLimits,
        deadline_ms: u64,
        items: &[HybridProcessItem],
    ) -> Result<Self, HybridProcessRequestRefusal> {
        let admitted = admit_package(package, evidence, package_limits)?;
        let package_id = serde_json::to_value(admitted.package_id())
            .map_err(|_| HybridProcessRequestRefusal::PackageRefused)?;
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
            package_id,
            deadline_ms,
            items: wire_items,
        };
        let request = serde_json::to_value(request)
            .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
        let bytes = quire_canonical::to_vec(&request, Limits::new(u64::MAX))
            .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
        let composed = Self(bytes);
        Self::read(composed.bytes(), evidence, package_limits)?;
        Ok(composed)
    }

    /// The canonical request-body bytes to place in FR-331's `request` frame.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    /// Read the process request body, admit the package from its base64url
    /// bytes and bind every item's index and claim node to that package.
    pub fn read(
        bytes: &[u8],
        evidence: &CheckedPackageEvidence,
        package_limits: CheckedPackageReadLimits,
    ) -> Result<AdmittedHybridProcessRequest, HybridProcessRequestRefusal> {
        let wire: ProcessRequestWire = serde_json::from_slice(bytes)
            .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
        let package_bytes = URL_SAFE_NO_PAD
            .decode(&wire.package)
            .map_err(|_| HybridProcessRequestRefusal::MalformedRequest)?;
        if URL_SAFE_NO_PAD.encode(&package_bytes) != wire.package {
            return Err(HybridProcessRequestRefusal::MalformedRequest);
        }
        let package = admit_package(&package_bytes, evidence, package_limits)?;
        let package_id = serde_json::to_value(package.package_id())
            .map_err(|_| HybridProcessRequestRefusal::PackageRefused)?;
        if wire.package_id != package_id {
            return Err(HybridProcessRequestRefusal::PackageIdentityMismatch);
        }
        let hybrids = read_hybrids(&package, package_limits.nodes)
            .map_err(|_| HybridProcessRequestRefusal::PackageRefused)?;
        let package_wire: Value = serde_json::from_slice(&package_bytes)
            .map_err(|_| HybridProcessRequestRefusal::PackageRefused)?;
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
            let claim_node = item.claim["node"]
                .as_object()
                .and_then(|node| node.get("digest"))
                .and_then(Value::as_str)
                .ok_or(HybridProcessRequestRefusal::MalformedRequest)?;
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
            let package_node = package_wire["semantic_graph"]["nodes"]
                .as_array()
                .and_then(|nodes| {
                    nodes
                        .iter()
                        .find(|node| node["node_id"]["digest"].as_str() == Some(claim_node))
                })
                .ok_or(HybridProcessRequestRefusal::ClaimMismatch)?;
            if package_node["semantic_form"] != "temporal_clause"
                || item.claim["node"] != package_node["node_id"]
                || !package_node["occurrences"]
                    .as_array()
                    .is_some_and(|occurrences| occurrences.contains(claim_occurrence))
            {
                return Err(HybridProcessRequestRefusal::ClaimMismatch);
            }
            let claim = hybrids
                .claims
                .iter()
                .find(|claim| claim_node == wire_node_digest(&claim.node))
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
    /// admitted request item and produce its terminal record.
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
    evidence: &CheckedPackageEvidence,
    limits: CheckedPackageReadLimits,
) -> Result<Box<CheckedPackageV2>, HybridProcessRequestRefusal> {
    match read_checked_package(bytes, limits, evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => Ok(package),
        CheckedPackageDispatchResult::Refused(_) | CheckedPackageDispatchResult::Incomplete(_) => {
            Err(HybridProcessRequestRefusal::PackageRefused)
        }
    }
}

fn wire_node_digest(node: &qsl_foundation::digest::WireNodeId) -> String {
    node.to_string()
}

const MAX_SAFE_COUNT: u64 = (1_u64 << 53) - 1;
