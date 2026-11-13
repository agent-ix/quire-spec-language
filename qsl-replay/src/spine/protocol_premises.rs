// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-3, PA-6: the premises of a protocol analysis, read from a
//! checked package. This sits in `spine`, the checked route, because it
//! takes a `CheckedPackageV2`; a `qsl-replay` stage entry outside `spine`
//! may not name that type (ADR-032 CK-3, `cargo xtask checked-input`).

use qsl_package::protocol_view::protocol_view;
use quire_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2};

use crate::protocol_analysis::{choice_premises, pair_channels, PremiseRefusal, ProtocolPremises};

impl ProtocolPremises {
    /// Groups (a) and (b) of the clause `protocol`, read only through
    /// `qsl_package::protocol_view` (PA-3). Groups (c) and (d) are `None`:
    /// no premise is inferred (PA-8).
    ///
    /// # Errors
    ///
    /// [`PremiseRefusal`] when a view function refuses, a queue and a channel
    /// do not pair, or a node is not in the package.
    pub fn from_view(
        package: &CheckedPackageV2,
        protocol: &CheckedNodeId,
    ) -> Result<Self, PremiseRefusal> {
        let view = protocol_view(package, protocol)?;
        let nodes = |ids: &[&CheckedNodeId]| {
            ids.iter()
                .map(|id| node_of(package, id))
                .collect::<Result<Vec<_>, _>>()
        };
        let mut channels = pair_channels(&nodes(view.channels())?, &nodes(view.queues())?)?;
        let mut choices = choice_premises(&nodes(view.controls())?)?;
        channels.sort();
        choices.sort();
        Ok(Self {
            channels,
            choices,
            role_knowledge: None,
            environment: None,
        })
    }
}

fn node_of<'p>(
    package: &'p CheckedPackageV2,
    id: &CheckedNodeId,
) -> Result<&'p CheckedSemanticNodeV2, PremiseRefusal> {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .ok_or_else(|| PremiseRefusal::UnknownNode(id.clone()))
}
