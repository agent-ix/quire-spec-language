// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-487 and FR-322-AC-308/309: the tick-monitor plan root and its
//! four state-passing entries on the checked-package/v2 wire.
//!
//! The entries are ordinary checked functions of the unit the producer
//! generated (QSL FR-251 "Transitions"); this writer makes them the plan's
//! declaration-less `function` nodes (their key is FR-322-AC-309's content
//! key), spells the record fields FR-487 names that the source surface
//! cannot, and appends the one `temporal`/`monitor_plan` node that names
//! them with the plan's premises. Nothing is read back here: the node form
//! is read by IR-721.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::CheckedNodePathSegment as Segment;
use quire_contract_model::{CheckedSemanticNodeV2, CheckedTerm};
use quire_exact::Rational;

use crate::wire_form::{has_form, has_tag, WireForm, WireTag};

use super::{
    own_evidence, read_index_segments, write_index_segments, CheckedNodeId, CheckedOccurrence,
    CheckedOccurrenceRole, CheckedPackageSidecarsRef, CheckedPackageSourceCorrespondenceRef,
    CheckedPackageV2, CheckedSemanticGraphV2, CheckedSourceMapEntry, Emission, EmitRefusal,
    EmittedPackage, OmittedNode, StructuralIds, GRAPH_V2,
};

/// What the writer needs of one plan.
#[derive(Clone, Debug)]
pub struct MonitorPlanWire<'a> {
    /// The identity of the temporal clause the plan monitors (its checked
    /// node key). The root's `clause` member is bound by this identity, not
    /// by position, so a package with several clauses binds the right one.
    pub clause: &'a quire_exact::NodeKey,
    /// The clause's formula arena.
    pub formula: &'a qsl_semantics::check::CheckedFormula,
    /// The counter's name.
    pub counter: &'a str,
    /// `counter_width_bits`.
    pub counter_width_bits: u32,
    /// `max_reading_gap`.
    pub max_reading_gap: u64,
    /// `max_event_rate`, events per tick.
    pub max_event_rate: &'a Rational,
    /// `uncertainty`, in ticks.
    pub uncertainty: &'a Rational,
    /// `buffers`: formula arena index and capacity.
    pub buffers: &'a [(usize, u64)],
    /// The declared name of the `tick_unit` unit.
    pub tick_unit: &'a str,
    /// `activation_capacity`.
    pub activation_capacity: u64,
    /// The declared names of `state`, `inputs`, `scope` and the results
    /// sequence's element record.
    pub state: &'a str,
    /// See [`Self::state`].
    pub inputs: &'a str,
    /// See [`Self::state`].
    pub scope: &'a str,
    /// The declared name of the result record.
    pub result: &'a str,
    /// The entries' declared names: init, on_event, on_watermark, on_gap.
    pub entries: [&'a str; 4],
    /// The records whose fields are renamed, and the renames
    /// `(source spelling, wire spelling)`.
    pub renamed_records: &'a [&'a str],
    /// See [`Self::renamed_records`].
    pub renames: &'a [(&'a str, &'a str)],
}

fn invariant(reason: &'static str) -> EmitRefusal {
    EmitRefusal::MonitorPlanInvariant(reason)
}

/// The one node declared as `name`; two declarations of one name refuse
/// rather than bind the first (`missing` names the absent node).
fn declared(
    nodes: &[CheckedSemanticNodeV2],
    name: &str,
    missing: &'static str,
) -> Result<usize, EmitRefusal> {
    let mut found = nodes.iter().enumerate().filter(|(_, node)| {
        node.declaration.as_ref().is_some_and(|declaration| {
            declaration
                .qualified_name
                .iter()
                .map(AsRef::as_ref)
                .eq([name])
        })
    });
    let first = found.next().ok_or_else(|| invariant(missing))?;
    if found.next().is_some() {
        return Err(EmitRefusal::MonitorPlanAmbiguous {
            name: name.to_owned(),
        });
    }
    Ok(first.0)
}

fn canonical_rational(value: &Rational) -> String {
    if value.is_integer() {
        value.numerator().to_string()
    } else {
        format!("{}/{}", value.numerator(), value.denominator())
    }
}

/// `id` with every `Name` segment equal to a source spelling replaced.
fn renamed_id(id: &CheckedNodeId, renames: &[(&str, &str)]) -> CheckedNodeId {
    let path = id
        .path
        .iter()
        .map(|segment| match segment {
            Segment::Name(name) => renames
                .iter()
                .find(|(from, _)| *from == name.as_ref())
                .map_or_else(|| segment.clone(), |(_, to)| Segment::Name((*to).into())),
            other => other.clone(),
        })
        .collect();
    CheckedNodeId {
        package: id.package.clone(),
        path,
        ordinal: id.ordinal,
    }
}

fn rename_bindings(body: &mut CheckedTerm, renames: &[(&str, &str)]) {
    if let CheckedTerm::Aggregate { members } = body {
        for member in members.iter_mut() {
            if let CheckedTerm::Binding { name, value } = member {
                if let Some((_, to)) = renames.iter().find(|(from, _)| *from == name.as_ref()) {
                    *member = CheckedTerm::binding(*to, (**value).clone());
                }
            }
        }
    }
}

/// Rename the field of every field projection (`quire.op.record.project`)
/// of a renamed record in `body`. The operation record is read-only in the
/// typed model, so the body goes through its wire form.
fn rename_projections(
    body: &mut CheckedTerm,
    renamed_types: &BTreeSet<CheckedNodeId>,
    renames: &[(&str, &str)],
) -> Result<(), EmitRefusal> {
    fn visit(
        value: &mut serde_json::Value,
        declarations: &[serde_json::Value],
        renames: &[(&str, &str)],
    ) -> bool {
        let mut changed = false;
        match value {
            serde_json::Value::Object(map) => {
                if let Some(member) = map
                    .get_mut("operation")
                    .and_then(|operation| operation.get_mut("member"))
                    .and_then(serde_json::Value::as_object_mut)
                {
                    let owned = member
                        .get("declaration")
                        .is_some_and(|declaration| declarations.contains(declaration));
                    let renamed = member
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|name| renames.iter().find(|(from, _)| *from == name))
                        .map(|(_, to)| (*to).to_owned());
                    if let (true, Some(to)) = (owned, renamed) {
                        member.insert("name".to_owned(), serde_json::Value::String(to));
                        changed = true;
                    }
                }
                for child in map.values_mut() {
                    changed |= visit(child, declarations, renames);
                }
            }
            serde_json::Value::Array(items) => {
                for child in items {
                    changed |= visit(child, declarations, renames);
                }
            }
            _ => {}
        }
        changed
    }
    let declarations = renamed_types
        .iter()
        .map(|id| serde_json::to_value(id).map_err(|_| invariant("a node id is JSON")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut wire = serde_json::to_value(&*body).map_err(|_| invariant("a body is JSON"))?;
    if visit(&mut wire, &declarations, renames) {
        *body = serde_json::from_value(wire)
            .map_err(|_| invariant("a renamed body is a checked term"))?;
    }
    Ok(())
}

/// Replace every reference to a node id, and the node's own id, and its
/// source-map entries, by exact match. The graph is in wire form here, whose
/// operand segments are graph indices, so a re-keyed node changes no other
/// node's id.
fn rekey(
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
    changes: &[(CheckedNodeId, CheckedNodeId)],
) {
    for (old, new) in changes {
        for node in nodes.iter_mut() {
            node.replace_node_id(old, new);
        }
        for entry in source_map.iter_mut() {
            if entry.node_id == *old {
                entry.node_id = new.clone();
            }
        }
    }
}

fn param_targets(
    node: &CheckedSemanticNodeV2,
) -> Result<(Vec<CheckedNodeId>, CheckedNodeId), EmitRefusal> {
    let CheckedTerm::Aggregate { members } = &node.body else {
        return Err(invariant("a function body is an aggregate"));
    };
    let mut parameters = Vec::new();
    let mut root = None;
    for member in members {
        if let CheckedTerm::Binding { name, value } = member {
            match (name.as_ref(), &**value) {
                ("parameters", CheckedTerm::Aggregate { members }) => {
                    for parameter in members {
                        if let CheckedTerm::Reference { target } = parameter {
                            parameters.push(target.clone());
                        }
                    }
                }
                ("body", CheckedTerm::Reference { target }) => root = Some(target.clone()),
                _ => {}
            }
        }
    }
    Ok((
        parameters,
        root.ok_or_else(|| invariant("a function body names its root"))?,
    ))
}

/// FR-322-AC-309: `[function, form, parameter count text, parameter
/// operands, body root]`, ordinal 0, the operands as graph indices on the
/// wire.
pub fn declaration_less_function_id(
    package: &str,
    form: &str,
    parameters: &[u64],
    body: u64,
) -> CheckedNodeId {
    let mut path = vec![
        Segment::Name("function".into()),
        Segment::Name(form.into()),
        Segment::Name(parameters.len().to_string().into()),
    ];
    path.extend(
        parameters
            .iter()
            .map(|parameter| Segment::Index(*parameter)),
    );
    path.push(Segment::Index(body));
    CheckedNodeId {
        package: package.into(),
        path,
        ordinal: 0,
    }
}

/// The formula arena index of each wire formula node, found by walking the
/// clause's formula root against the arena.
fn formula_nodes(
    nodes: &[CheckedSemanticNodeV2],
    clause: &CheckedSemanticNodeV2,
    formula: &qsl_semantics::check::CheckedFormula,
) -> Result<Vec<Option<CheckedNodeId>>, EmitRefusal> {
    let CheckedTerm::Application { arguments, .. } = &clause.body else {
        return Err(invariant("the clause body is an application"));
    };
    let Some(CheckedTerm::Reference { target: root }) = arguments.last() else {
        return Err(invariant("the clause names its formula root"));
    };
    let by_id = |id: &CheckedNodeId| nodes.iter().find(|node| node.node_id == *id);
    let arena = formula.nodes();
    let mut out: Vec<Option<CheckedNodeId>> = vec![None; arena.len()];
    let mut stack = vec![(formula.root().index(), root.clone())];
    while let Some((index, wire)) = stack.pop() {
        out[index] = Some(wire.clone());
        let node = by_id(&wire).ok_or_else(|| invariant("a formula node is in the graph"))?;
        let children: Vec<CheckedNodeId> = node
            .body
            .reference_targets()
            .into_iter()
            .filter(|target| {
                by_id(target)
                    .is_some_and(|child| has_form(child, WireTag::Temporal, WireForm::Formula))
            })
            .cloned()
            .collect();
        let operands: Vec<usize> = arena[index]
            .operands()
            .map(|operand| operand.index())
            .collect();
        if children.len() > operands.len() {
            return Err(invariant(
                "a formula node has more wire operands than arena operands",
            ));
        }
        let mut unique = Vec::new();
        for operand in &operands {
            if !unique.contains(operand) {
                unique.push(*operand);
            }
        }
        for (operand, child) in unique.into_iter().zip(children) {
            stack.push((operand, child));
        }
    }
    Ok(out)
}

/// Write the plan root and entries onto the package `base` was emitted as,
/// reading it back through IR first (the writer works on the admitted
/// graph) under the caller's `limits`, and writes the plan package under the
/// caller's `emit` ceiling.
///
/// `retained` is the emission `base` was written from. Every witness row it
/// retained from its acquisition (selected model rows, equality producer and
/// fact rows, temporal binding rows, omissions) is carried onto the returned
/// emission, re-keyed to the nodes this writer renames, so a consumer of the
/// plan root never rebuilds or re-acquires them.
pub fn emit_monitor_plan_over(
    retained: &Emission,
    identity: qsl_semantics::library::LibraryName,
    evidence: &quire_contract_model::CheckedPackageEvidence,
    plan: &MonitorPlanWire<'_>,
    limits: crate::checked_v2::V2ReadLimits,
    emit: crate::checked::EmitLimits,
) -> Result<Emission, EmitRefusal> {
    let read = crate::checked_v2::read_checked_package_v2(
        retained.package().bytes(),
        identity,
        limits,
        evidence,
    )
    .map_err(|_| invariant("the base package does not read back"))?;
    emit_monitor_plan(&read.into_value().admitted, retained, plan, emit)
}

/// Write the plan root and entries onto the admitted package `base`.
fn emit_monitor_plan(
    base: &CheckedPackageV2,
    retained: &Emission,
    plan: &MonitorPlanWire<'_>,
    limits: crate::checked::EmitLimits,
) -> Result<Emission, EmitRefusal> {
    let ids = &StructuralIds::default();
    let mut nodes = base.resolved_graph().nodes.clone();
    let mut source_map = base.source_map().to_vec();
    read_index_segments(ids, &mut nodes, &mut source_map)?;
    // The inspection map is read from the structural form, before the
    // graph goes to wire form for the edits. The clause is selected in that
    // form too, by its own identity.
    let inspection = super::inspection_source_map(ids, &source_map)?;
    let clause_id = qsl_semantics::check::wire_id::structural_id(plan.clause);
    let mut clauses = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.node_id == clause_id);
    let clause_at = clauses
        .next()
        .map(|(at, _)| at)
        .ok_or_else(|| invariant("the selected temporal clause is in the package"))?;
    if clauses.next().is_some() {
        return Err(EmitRefusal::MonitorPlanAmbiguous {
            name: "the selected temporal clause".to_owned(),
        });
    }
    write_index_segments(&mut nodes, &mut source_map, &mut [], &mut [])?;
    // The generated nodes belong to the package being compiled, which the
    // selected clause is a node of (a domain package's nodes may come first).
    let package: String = nodes[clause_at].node_id.package.to_string();

    // 1. The record fields FR-487 names that the surface cannot spell.
    let mut renamed_types: BTreeSet<CheckedNodeId> = BTreeSet::new();
    for name in plan.renamed_records {
        let at = declared(&nodes, name, "a renamed record is declared")?;
        rename_bindings(&mut nodes[at].body, plan.renames);
        renamed_types.insert(nodes[at].node_id.clone());
    }
    for node in nodes.iter_mut() {
        rename_projections(&mut node.body, &renamed_types, plan.renames)?;
    }
    let mut all_changes: Vec<(CheckedNodeId, CheckedNodeId)> = Vec::new();
    let mut changes = Vec::new();
    for node in nodes.iter_mut() {
        if WireForm::from_wire(&node.semantic_form) == Some(WireForm::RecordValue)
            && renamed_types.contains(&node.semantic_type)
        {
            rename_bindings(&mut node.body, plan.renames);
            let new = renamed_id(&node.node_id, plan.renames);
            if new != node.node_id {
                changes.push((node.node_id.clone(), new));
            }
        }
    }
    rekey(&mut nodes, &mut source_map, &changes);
    all_changes.extend(changes);

    // 2. The entries: declaration-less function nodes keyed by content. A
    // function the entries call stays a named declaration, keyed by its
    // name, so the entry's key carries the entry's own body and not the
    // bodies of the helpers it calls.
    let mut entry_ids = Vec::new();
    let mut changes = Vec::new();
    for name in plan.entries {
        let at = declared(&nodes, name, "an entry is declared")?;
        let (parameters, body) = param_targets(&nodes[at])?;
        let index_of = |id: &CheckedNodeId| -> Result<u64, EmitRefusal> {
            nodes
                .iter()
                .position(|node| node.node_id == *id)
                .map(|at| at as u64)
                .ok_or_else(|| invariant("a parameter or body node is in the graph"))
        };
        let parameters = parameters
            .iter()
            .map(index_of)
            .collect::<Result<Vec<_>, _>>()?;
        let body = index_of(&body)?;
        let new = declaration_less_function_id(
            &package,
            nodes[at].semantic_form.as_ref(),
            &parameters,
            body,
        );
        let old = nodes[at].node_id.clone();
        nodes[at].declaration = None;
        nodes[at].owner = None;
        nodes[at].occurrences = vec![CheckedOccurrence {
            role: CheckedOccurrenceRole::Generated,
            ordinal: 0,
        }];
        for entry in source_map.iter_mut().filter(|entry| entry.node_id == old) {
            entry.role = CheckedOccurrenceRole::Generated;
            entry.ordinal = 0;
        }
        changes.push((old, new.clone()));
        entry_ids.push(new);
    }
    rekey(&mut nodes, &mut source_map, &changes);
    all_changes.extend(changes);

    // 3. The nodes the root names.
    let node_of = |name: &str| -> Result<CheckedNodeId, EmitRefusal> {
        declared(&nodes, name, "a named type is declared").map(|at| nodes[at].node_id.clone())
    };
    let state = node_of(plan.state)?;
    let inputs = node_of(plan.inputs)?;
    let scope = node_of(plan.scope)?;
    let result = node_of(plan.result)?;
    let results = nodes
        .iter()
        .find(|node| {
            has_form(node, WireTag::CompositeType, WireForm::Sequence)
                && node
                    .body
                    .reference_targets()
                    .iter()
                    .any(|target| **target == result)
        })
        .map(|node| node.node_id.clone())
        .ok_or_else(|| invariant("the results sequence is in the graph"))?;
    let scalar = |form: WireForm| -> Option<CheckedNodeId> {
        nodes
            .iter()
            .find(|node| has_form(node, WireTag::ScalarType, form))
            .map(|node| node.node_id.clone())
    };
    let integer =
        scalar(WireForm::Integer).ok_or_else(|| invariant("the integer type is in the graph"))?;
    let text = scalar(WireForm::Text).ok_or_else(|| invariant("the text type is in the graph"))?;
    let clause_region = source_map
        .iter()
        .find(|entry| entry.node_id == nodes[clause_at].node_id)
        .map(|entry| entry.regions.clone())
        .ok_or_else(|| invariant("the clause is located"))?;
    let rational = match scalar(WireForm::Rational) {
        Some(id) => id,
        None => {
            let id = CheckedNodeId {
                package: package.as_str().into(),
                path: vec![
                    Segment::Name("scalar_type".into()),
                    Segment::Name("rational".into()),
                ],
                ordinal: 0,
            };
            source_map.push(CheckedSourceMapEntry {
                node_id: id.clone(),
                role: CheckedOccurrenceRole::Generated,
                ordinal: 0,
                regions: clause_region.clone(),
            });
            nodes.push(CheckedSemanticNodeV2 {
                owner_model: None,
                node_id: id.clone(),
                schema_version: GRAPH_V2.into(),
                node_tag: "scalar_type".into(),
                semantic_form: "rational".into(),
                semantic_type: id.clone(),
                dependencies: Vec::new(),
                occurrences: vec![CheckedOccurrence {
                    role: CheckedOccurrenceRole::Generated,
                    ordinal: 0,
                }],
                recursion_group: None,
                declaration: None,
                owner: None,
                body: CheckedTerm::aggregate(Vec::new()),
            });
            id
        }
    };
    let clause = nodes[clause_at].clone();
    if !has_form(&clause, WireTag::Temporal, WireForm::TemporalClause) {
        return Err(invariant("the selected node is a temporal clause"));
    }
    let tick_unit = nodes
        .iter()
        .find(|node| {
            has_form(node, WireTag::ScalarType, WireForm::Unit)
                && node.declaration.as_ref().is_some_and(|d| {
                    d.qualified_name
                        .iter()
                        .map(AsRef::as_ref)
                        .eq([plan.tick_unit])
                })
        })
        .map(|node| node.node_id.clone())
        .ok_or_else(|| invariant("the tick unit is a unit node of the graph"))?;
    let subformulas = formula_nodes(&nodes, &clause, plan.formula)?;

    // 4. The root.
    let mut buffers = Vec::new();
    for (index, capacity) in plan.buffers {
        let subformula = subformulas
            .get(*index)
            .and_then(Clone::clone)
            .ok_or_else(|| invariant("a buffered subformula is a wire formula node"))?;
        buffers.push((subformula, *capacity));
    }
    buffers.sort();
    let buffer_terms: Vec<CheckedTerm> = buffers
        .iter()
        .map(|(subformula, capacity)| {
            CheckedTerm::aggregate(vec![
                CheckedTerm::binding("subformula", CheckedTerm::reference(subformula.clone())),
                CheckedTerm::binding(
                    "capacity",
                    CheckedTerm::literal_integer(integer.clone(), *capacity),
                ),
            ])
        })
        .collect();
    let rate_text = canonical_rational(plan.max_event_rate);
    let uncertainty_text = canonical_rational(plan.uncertainty);
    let reference = |id: &CheckedNodeId| CheckedTerm::reference(id.clone());
    let members = vec![
        CheckedTerm::binding("clause", reference(&clause.node_id)),
        CheckedTerm::binding("state", reference(&state)),
        CheckedTerm::binding("inputs", reference(&inputs)),
        CheckedTerm::binding("scope", reference(&scope)),
        CheckedTerm::binding("results", reference(&results)),
        CheckedTerm::binding("init", reference(&entry_ids[0])),
        CheckedTerm::binding("on_event", reference(&entry_ids[1])),
        CheckedTerm::binding("on_watermark", reference(&entry_ids[2])),
        CheckedTerm::binding(
            "counter_width_bits",
            CheckedTerm::literal_integer(integer.clone(), u64::from(plan.counter_width_bits)),
        ),
        CheckedTerm::binding(
            "max_reading_gap",
            CheckedTerm::literal_integer(integer.clone(), plan.max_reading_gap),
        ),
        CheckedTerm::binding(
            "max_event_rate",
            CheckedTerm::literal_rational(rational.clone(), rate_text.clone()),
        ),
        CheckedTerm::binding("buffers", CheckedTerm::aggregate(buffer_terms)),
        CheckedTerm::binding(
            "counter",
            CheckedTerm::literal_text(text.clone(), plan.counter),
        ),
        CheckedTerm::binding("tick_unit", reference(&tick_unit)),
        CheckedTerm::binding(
            "uncertainty",
            CheckedTerm::literal_rational(rational.clone(), uncertainty_text.clone()),
        ),
        CheckedTerm::binding("on_gap", reference(&entry_ids[3])),
        CheckedTerm::binding(
            "activation_capacity",
            CheckedTerm::literal_integer(integer.clone(), plan.activation_capacity),
        ),
    ];
    let node = |id: &CheckedNodeId| -> Result<Segment, EmitRefusal> {
        nodes
            .iter()
            .position(|candidate| candidate.node_id == *id)
            .map(|at| Segment::Index(at as u64))
            .ok_or_else(|| invariant("a root operand is in the graph"))
    };
    let text_segment = |value: String| Segment::Name(value.into());
    let mut path = vec![
        Segment::Name("temporal".into()),
        Segment::Name("monitor_plan".into()),
    ];
    for id in [
        &clause.node_id,
        &state,
        &inputs,
        &scope,
        &results,
        &entry_ids[0],
        &entry_ids[1],
        &entry_ids[2],
    ] {
        path.push(node(id)?);
    }
    path.push(text_segment(plan.counter_width_bits.to_string()));
    path.push(text_segment(plan.max_reading_gap.to_string()));
    path.push(text_segment(rate_text));
    for (subformula, capacity) in &buffers {
        path.push(node(subformula)?);
        path.push(text_segment(capacity.to_string()));
    }
    path.push(text_segment(plan.counter.to_owned()));
    path.push(node(&tick_unit)?);
    path.push(text_segment(uncertainty_text));
    path.push(node(&entry_ids[3])?);
    path.push(text_segment(plan.activation_capacity.to_string()));
    let mut closure_roots: Vec<&CheckedNodeId> = entry_ids.iter().collect();
    closure_roots.push(&clause.node_id);
    for body_root in call_closure_body_roots(&nodes, &closure_roots)? {
        path.push(node(&body_root)?);
    }
    let root_id = CheckedNodeId {
        package: package.as_str().into(),
        path,
        ordinal: 0,
    };
    let body = CheckedTerm::aggregate(members);
    let mut dependencies: Vec<CheckedNodeId> =
        body.reference_targets().into_iter().cloned().collect();
    dependencies.sort();
    dependencies.dedup();
    source_map.push(CheckedSourceMapEntry {
        node_id: root_id.clone(),
        role: CheckedOccurrenceRole::Generated,
        ordinal: 0,
        regions: clause_region,
    });
    nodes.push(CheckedSemanticNodeV2 {
        owner_model: None,
        node_id: root_id,
        schema_version: GRAPH_V2.into(),
        node_tag: "temporal".into(),
        semantic_form: "monitor_plan".into(),
        semantic_type: state,
        dependencies,
        occurrences: vec![CheckedOccurrence {
            role: CheckedOccurrenceRole::Generated,
            ordinal: 0,
        }],
        recursion_group: None,
        declaration: None,
        owner: None,
        body,
    });
    finish(
        base,
        retained,
        &all_changes,
        nodes,
        source_map,
        inspection,
        limits,
    )
}

/// The body-root node of every declared `function` node the `roots` reach
/// through their bodies, directly or through the body of another reached
/// function, once each, in ascending node-id order of the called functions
/// (QSpec FR-487 "call closure"). A reached helper stays a declared node; its
/// body root is what the plan root's key covers.
fn call_closure_body_roots(
    nodes: &[CheckedSemanticNodeV2],
    roots: &[&CheckedNodeId],
) -> Result<Vec<CheckedNodeId>, EmitRefusal> {
    let by_id: std::collections::BTreeMap<&CheckedNodeId, &CheckedSemanticNodeV2> =
        nodes.iter().map(|node| (&node.node_id, node)).collect();
    let mut seen: BTreeSet<&CheckedNodeId> = BTreeSet::new();
    let mut stack: Vec<&CheckedNodeId> = roots.to_vec();
    let mut called: std::collections::BTreeMap<&CheckedNodeId, CheckedNodeId> =
        std::collections::BTreeMap::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let node = by_id
            .get(id)
            .ok_or_else(|| invariant("a node of the call closure is in the graph"))?;
        if has_tag(node, WireTag::Function) && node.declaration.is_some() {
            let (_, body) = param_targets(node)?;
            called.insert(&node.node_id, body);
        }
        stack.extend(node.body.reference_targets());
    }
    Ok(called.into_values().collect())
}

/// The witness rows an emission retains, with every node id they hold.
pub(super) struct RetainedRows {
    pub(super) omitted: Vec<OmittedNode>,
    pub(super) selected_operation_member_rows:
        Vec<quire_contract_model::SelectedOperationMemberRow>,
    pub(super) selected_tuple_declaration_rows:
        Vec<quire_contract_model::SelectedTupleDeclarationRow>,
    pub(super) selected_source_unit_rows: Vec<quire_contract_model::SelectedSourceUnitRow>,
    pub(super) selected_model_enum_declaration_rows:
        Vec<quire_contract_model::SelectedModelEnumDeclarationRow>,
    pub(super) selected_model_enum_member_rows:
        Vec<quire_contract_model::SelectedModelEnumMemberRow>,
    pub(super) selected_enum_declaration_rows:
        Vec<quire_contract_model::SelectedEnumDeclarationRow>,
    pub(super) selected_enum_member_rows: Vec<quire_contract_model::SelectedEnumMemberRow>,
    pub(super) selected_source_union_rows: Vec<quire_contract_model::SelectedSourceUnionRow>,
    pub(super) selected_source_record_rows: Vec<super::SelectedSourceRecordRow>,
    pub(super) selected_source_tuple_rows: Vec<quire_contract_model::SelectedSourceTupleRow>,
    pub(super) selected_model_enum_shape_rows: Vec<super::SelectedModelEnumShapeRow>,
    pub(super) selected_source_collection_rows: Vec<super::SelectedSourceCollectionRow>,
    pub(super) selected_exact_declaration_rows:
        Vec<quire_contract_model::SelectedExactDeclarationRow>,
    pub(super) selected_exact_field_rows: Vec<quire_contract_model::SelectedExactFieldRow>,
    pub(super) equality_producer_identity_rows: Vec<super::EqualityProducerEmissionRow>,
    pub(super) equality_fact_witness_rows: Vec<super::EqualityFactWitnessRow>,
    pub(super) temporal_binding_rows: super::TemporalBindingRows,
    pub(super) temporal_rows: super::TemporalRows,
}

/// Re-keys every node id a retained row holds through `changes`, in order.
/// Each row type is destructured without a rest pattern, so a field added to
/// a row refuses to compile until it is either re-keyed here or named as
/// holding no node id.
struct Rekey<'a>(&'a [(CheckedNodeId, CheckedNodeId)]);

impl Rekey<'_> {
    fn id(&self, id: &mut CheckedNodeId) {
        for (old, new) in self.0 {
            if *id == *old {
                *id = new.clone();
            }
        }
    }

    fn entry(&self, entry: &mut CheckedSourceMapEntry) {
        let CheckedSourceMapEntry {
            node_id,
            role: _,
            ordinal: _,
            regions: _,
        } = entry;
        self.id(node_id);
    }

    fn fact(&self, fact: &mut super::EqualityFactWitness) {
        use super::EqualityFactWitness as F;
        match fact {
            F::OperandType { .. } | F::DimensionDeclaration { .. } | F::TextPlanType { .. } => {}
            F::Unit { quantity_type, .. } => self.id(quantity_type),
            F::DimensionTerm { base_node, .. } => self.id(base_node),
            F::ConversionEdge { target_node, .. } => self.id(target_node),
            F::EnumMember {
                source_declaration_node,
                declaration_node,
                member_node,
                ..
            } => {
                self.id(source_declaration_node);
                self.id(declaration_node);
                self.id(member_node);
            }
        }
    }
}

/// The retained rows of `retained` with every node id they hold re-keyed
/// through `changes` (the writer's renames). No row is written anew.
pub(super) fn carry_rows(
    retained: &Emission,
    changes: &[(CheckedNodeId, CheckedNodeId)],
) -> RetainedRows {
    use quire_contract_model as m;
    let rekey = Rekey(changes);
    let mut omitted = retained.omitted.clone();
    for super::OmittedNode { node, cause } in &mut omitted {
        rekey.id(node);
        match cause {
            super::OmissionCause::UnsupportedForm { .. }
            | super::OmissionCause::DeclarationOccurrenceMismatch
            | super::OmissionCause::UnlockedOwner => {}
            super::OmissionCause::NamesAbsentNode(id)
            | super::OmissionCause::NamesOmittedNode(id) => rekey.id(id),
        }
    }
    let mut operation = retained.selected_operation_member_rows.clone();
    for row in &mut operation {
        let m::SelectedOperationMemberRow {
            anchor,
            selected_document: _,
            declaring_form: _,
            original_model_identity: _,
            original_operation_identity: _,
            selected_operation_identity: _,
        } = row;
        rekey.id(anchor);
    }
    let mut tuples = retained.selected_tuple_declaration_rows.clone();
    for row in &mut tuples {
        let m::SelectedTupleDeclarationRow {
            structural_node,
            selected_document: _,
            original_identity: _,
            node_key: _,
        } = row;
        rekey.id(structural_node);
    }
    let mut units = retained.selected_source_unit_rows.clone();
    for row in &mut units {
        let m::SelectedSourceUnitRow {
            structural_node,
            source_authority: _,
            source_identity: _,
            qualified_declaration: _,
            unit_id: _,
        } = row;
        rekey.id(structural_node);
    }
    let mut enum_declarations = retained.selected_model_enum_declaration_rows.clone();
    for row in &mut enum_declarations {
        let m::SelectedModelEnumDeclarationRow {
            structural_node,
            selected_document: _,
            original_identity: _,
            node_key: _,
        } = row;
        rekey.id(structural_node);
    }
    let mut enum_members = retained.selected_model_enum_member_rows.clone();
    for row in &mut enum_members {
        let m::SelectedModelEnumMemberRow {
            structural_node,
            declaration_node,
            selected_document: _,
            enum_original_identity: _,
            member_original_identity: _,
            case: _,
            member_key: _,
        } = row;
        rekey.id(structural_node);
        rekey.id(declaration_node);
    }
    let mut source_enum_declarations = retained.selected_enum_declaration_rows.clone();
    for row in &mut source_enum_declarations {
        let m::SelectedEnumDeclarationRow {
            structural_node,
            source_authority: _,
            source_identity: _,
            qualified_declaration: _,
            node_key: _,
            shape: _,
        } = row;
        rekey.id(structural_node);
    }
    let mut source_enum_members = retained.selected_enum_member_rows.clone();
    for row in &mut source_enum_members {
        let m::SelectedEnumMemberRow {
            structural_node,
            source_authority: _,
            source_identity: _,
            declaration_node,
            declaration_qualified_name: _,
            case: _,
            member_key: _,
            variant: _,
        } = row;
        rekey.id(structural_node);
        rekey.id(declaration_node);
    }
    let mut source_unions = retained.selected_source_union_rows.clone();
    for row in &mut source_unions {
        let m::SelectedSourceUnionRow {
            structural_node,
            source_authority: _,
            source_identity: _,
            qualified_declaration: _,
            type_handle: _,
            settled_key: _,
            members,
        } = row;
        rekey.id(structural_node);
        for member in members {
            let m::SelectedUnionMemberRow {
                member: _,
                payload_nodes,
                payload: _,
            } = member;
            for node in payload_nodes {
                rekey.id(node);
            }
        }
    }
    let mut source_records = retained.selected_source_record_rows.clone();
    for row in &mut source_records {
        let super::SelectedSourceRecordRow {
            structural_node,
            source_authority: _,
            source_identity: _,
            qualified_declaration: _,
            record_key: _,
            members,
        } = row;
        rekey.id(structural_node);
        for member in members {
            let super::SelectedSourceRecordMemberRow {
                original_member_identity: _,
                admitted_name: _,
                presence: _,
                type_node,
                value_type: _,
            } = member;
            rekey.id(type_node);
        }
    }
    let mut source_tuples = retained.selected_source_tuple_rows.clone();
    for row in &mut source_tuples {
        let m::SelectedSourceTupleRow {
            structural_node,
            source_authority: _,
            source_identity: _,
            qualified_declaration: _,
            type_handle: _,
            positions,
        } = row;
        rekey.id(structural_node);
        for position in positions {
            let m::SelectedTuplePositionRow {
                ordinal: _,
                type_node,
                value_type: _,
            } = position;
            rekey.id(type_node);
        }
    }
    let mut model_enum_shapes = retained.selected_model_enum_shape_rows.clone();
    for row in &mut model_enum_shapes {
        let super::SelectedModelEnumShapeRow {
            structural_node,
            node_key: _,
            selected_document: _,
            original_identity: _,
            shape: _,
            cases,
        } = row;
        rekey.id(structural_node);
        for case in cases {
            let super::SelectedModelEnumShapeCaseRow {
                case: _,
                structural_node,
                member_key: _,
                member_original_identity: _,
                variant: _,
            } = case;
            rekey.id(structural_node);
        }
    }
    let mut source_collections = retained.selected_source_collection_rows.clone();
    for row in &mut source_collections {
        let super::SelectedSourceCollectionRow {
            structural_node,
            base_node,
            element_node,
            key_node,
            value_type: _,
        } = row;
        rekey.id(structural_node);
        rekey.id(base_node);
        rekey.id(element_node);
        if let Some(key_node) = key_node {
            rekey.id(key_node);
        }
    }
    let mut exact_declarations = retained.selected_exact_declaration_rows.clone();
    for row in &mut exact_declarations {
        let m::SelectedExactDeclarationRow {
            structural_node,
            selected_document: _,
            form: _,
            original_identity: _,
            exact_identity: _,
        } = row;
        rekey.id(structural_node);
    }
    let mut exact_fields = retained.selected_exact_field_rows.clone();
    for row in &mut exact_fields {
        let m::SelectedExactFieldRow {
            record_node,
            selected_document: _,
            record_original_identity: _,
            field_original_identity: _,
            member_id: _,
        } = row;
        rekey.id(record_node);
    }
    let mut producers = retained.equality_producer_identity_rows.clone();
    for row in &mut producers {
        let super::EqualityProducerEmissionRow {
            application: _,
            checked_node,
            occurrence: _,
            location: _,
            source_map_entry,
            producer_identity: _,
            producer_data: _,
        } = row;
        rekey.id(checked_node);
        rekey.entry(source_map_entry);
    }
    let mut facts = retained.equality_fact_witness_rows.clone();
    for row in &mut facts {
        let super::EqualityFactWitnessRow {
            application: _,
            checked_application,
            occurrence: _,
            source_map_entry,
            structural_key: _,
            structural_node,
            owner: _,
            value_type: _,
            fact,
        } = row;
        rekey.id(checked_application);
        rekey.entry(source_map_entry);
        rekey.id(structural_node);
        rekey.fact(fact);
    }
    let mut temporal = retained.temporal_binding_rows.clone();
    for (_, row) in &mut temporal.rows {
        let m::TemporalBinding {
            declaration,
            occurrence: _,
            clock_name: _,
            clock: _,
        } = row;
        rekey.id(declaration);
    }
    // The declaration lookup is re-keyed with the rows, for every rename the
    // writer performs or may perform. Today's renames name record value
    // nodes and the entry functions, never a clause declaration, so this is
    // an invariant of the carry and not a path the current writer reaches.
    for (_, _, declaration) in &mut temporal.declarations {
        rekey.id(declaration);
    }
    let mut temporal_rows = retained.temporal_rows.clone();
    for super::TemporalClauseRef {
        declaration,
        occurrence: _,
        clause: _,
    } in &mut temporal_rows.clauses
    {
        rekey.id(declaration);
    }
    for row in &mut temporal_rows.bindings {
        let m::TemporalBinding {
            declaration,
            occurrence: _,
            clock_name: _,
            clock: _,
        } = row;
        rekey.id(declaration);
    }
    RetainedRows {
        omitted,
        selected_operation_member_rows: operation,
        selected_tuple_declaration_rows: tuples,
        selected_source_unit_rows: units,
        selected_model_enum_declaration_rows: enum_declarations,
        selected_model_enum_member_rows: enum_members,
        selected_enum_declaration_rows: source_enum_declarations,
        selected_enum_member_rows: source_enum_members,
        selected_source_union_rows: source_unions,
        selected_source_record_rows: source_records,
        selected_source_tuple_rows: source_tuples,
        selected_model_enum_shape_rows: model_enum_shapes,
        selected_source_collection_rows: source_collections,
        selected_exact_declaration_rows: exact_declarations,
        selected_exact_field_rows: exact_fields,
        equality_producer_identity_rows: producers,
        equality_fact_witness_rows: facts,
        temporal_binding_rows: temporal,
        temporal_rows,
    }
}

/// Sets every member's `admitted_name` of each source record row to the
/// binding name the final record node holds at that position, so a binding
/// the writer renamed is named by its final name and not the name the base
/// package gave it. Everything else in the row is left as carried. A row
/// whose node is absent, or whose node's bindings disagree with its members
/// in count, is a writer fault.
fn follow_final_bindings(
    rows: &mut [super::SelectedSourceRecordRow],
    nodes: &[CheckedSemanticNodeV2],
) -> Result<(), EmitRefusal> {
    for row in rows {
        let node = nodes
            .iter()
            .find(|node| node.node_id == row.structural_node)
            .ok_or_else(|| invariant("a source record row names a node of the package"))?;
        let CheckedTerm::Aggregate { members } = &node.body else {
            return Err(invariant("a source record node's body is an aggregate"));
        };
        let names: Vec<&str> = members
            .iter()
            .filter_map(|member| match member {
                CheckedTerm::Binding { name, .. } => Some(name.as_ref()),
                _ => None,
            })
            .collect();
        if names.len() != row.members.len() || names.len() != members.len() {
            return Err(invariant("a source record node binds each row member"));
        }
        for (member, name) in row.members.iter_mut().zip(names) {
            member.admitted_name = name.into();
        }
    }
    Ok(())
}

/// Sets every position's `type_node` of each source tuple row to the node the
/// final tuple node references at that ordinal, so a position the writer
/// re-pointed is named by its final node and not the one the base package
/// gave it. Everything else in the row is left as carried. A row whose node
/// is absent, or whose node's references disagree with its positions in
/// count or form, is a writer fault.
pub(super) fn follow_final_positions(
    rows: &mut [quire_contract_model::SelectedSourceTupleRow],
    nodes: &[CheckedSemanticNodeV2],
) -> Result<(), EmitRefusal> {
    let by_id: BTreeMap<&CheckedNodeId, &CheckedSemanticNodeV2> =
        nodes.iter().map(|node| (&node.node_id, node)).collect();
    for row in rows {
        let node = by_id
            .get(&row.structural_node)
            .ok_or_else(|| invariant("a source tuple row names a node of the package"))?;
        let CheckedTerm::Aggregate { members } = &node.body else {
            return Err(invariant("a source tuple node's body is an aggregate"));
        };
        let targets: Vec<&CheckedNodeId> = members
            .iter()
            .filter_map(|member| match member {
                CheckedTerm::Reference { target } => Some(target),
                _ => None,
            })
            .collect();
        if targets.len() != row.positions.len() || targets.len() != members.len() {
            return Err(invariant(
                "a source tuple node references each row position",
            ));
        }
        for (position, target) in row.positions.iter_mut().zip(targets) {
            position.type_node = target.clone();
        }
    }
    Ok(())
}

/// Sets each collection row's `element_node` (and a map row's `key_node`) to
/// the node the final collection node references, and requires a bounded collection's domain node to be
/// typed by that collection node, so a re-pointed element or bound is named
/// by its final node. A row whose nodes are absent or malformed is a writer
/// fault.
pub(super) fn follow_final_collections(
    rows: &mut [super::SelectedSourceCollectionRow],
    nodes: &[CheckedSemanticNodeV2],
) -> Result<(), EmitRefusal> {
    let by_id: BTreeMap<&CheckedNodeId, &CheckedSemanticNodeV2> =
        nodes.iter().map(|node| (&node.node_id, node)).collect();
    let find = |id: &CheckedNodeId| {
        by_id
            .get(id)
            .copied()
            .ok_or_else(|| invariant("a collection row names a node of the package"))
    };
    for row in rows {
        let base = find(&row.base_node)?;
        let CheckedTerm::Aggregate { members } = &base.body else {
            return Err(invariant("a collection node's body is an aggregate"));
        };
        match (&row.key_node, members.as_slice()) {
            (None, [CheckedTerm::Reference { target }]) => row.element_node = target.clone(),
            (Some(_), [key, value]) => {
                let (Some(key), Some(value)) = (
                    binding_reference(key, "key"),
                    binding_reference(value, "value"),
                ) else {
                    return Err(invariant("a map node binds its key then its value"));
                };
                row.key_node = Some(key.clone());
                row.element_node = value.clone();
            }
            _ => return Err(invariant("a collection node references its element")),
        }
        if row.structural_node != row.base_node
            && find(&row.structural_node)?.semantic_type != row.base_node
        {
            return Err(invariant("a bounded collection is typed by its collection"));
        }
    }
    Ok(())
}

/// The shared tail: index the graph, encode and wrap the bytes.
fn finish(
    base: &CheckedPackageV2,
    retained: &Emission,
    changes: &[(CheckedNodeId, CheckedNodeId)],
    nodes: Vec<CheckedSemanticNodeV2>,
    source_map: Vec<CheckedSourceMapEntry>,
    inspection_source_map: qsl_foundation::source::provenance::PackageSourceMap,
    limits: crate::checked::EmitLimits,
) -> Result<Emission, EmitRefusal> {
    let lock = base.lock().clone();
    let (slot_nodes, operand_rows) = super::write_operand_slots(&nodes)?;
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: base.resolved_graph().graph_version.clone(),
        nodes: slot_nodes,
        operand_rows,
    };
    let capability_report = base.capability_report().to_vec();
    let diagnostics = base.diagnostics().clone();
    let operation_effects = base.operation_effects().map(|rows| {
        rows.map(|row| quire_contract_model::CheckedOperationEffectRow {
            document: row.document().into(),
            declaring: row.declaring().clone(),
            operation: row.operation().into(),
            effect: row.effect().clone(),
            provenance: row.provenance().clone(),
        })
        .collect::<Vec<_>>()
    });
    let package = EmittedPackage::new(base.name(), |name| {
        quire_contract_model::encode_checked_package_v2(
            name,
            &lock,
            &semantic_graph,
            CheckedPackageSourceCorrespondenceRef {
                source_map: &source_map,
                refinement_step_rows: base.refinement_step_rows(),
            },
            CheckedPackageSidecarsRef {
                checked_term_data: base.checked_term_data(),
                temporal_bindings: base.temporal_bindings(),
                temporal_clause_names: base.temporal_clause_names(),
                operation_effects: operation_effects.as_deref(),
                schedulability_claims: None,
            },
            &capability_report,
            &diagnostics,
            limits.canonical(),
        )
        .map_err(EmitRefusal::from)
    })?;
    let mut rows = carry_rows(retained, changes);
    follow_final_bindings(&mut rows.selected_source_record_rows, &nodes)?;
    follow_final_positions(&mut rows.selected_source_tuple_rows, &nodes)?;
    follow_final_collections(&mut rows.selected_source_collection_rows, &nodes)?;
    Ok(Emission {
        package,
        omitted: rows.omitted,
        evidence: own_evidence(&lock),
        capability_report,
        source_map: inspection_source_map,
        selected_operation_member_rows: rows.selected_operation_member_rows,
        selected_tuple_declaration_rows: rows.selected_tuple_declaration_rows,
        selected_source_unit_rows: rows.selected_source_unit_rows,
        selected_model_enum_declaration_rows: rows.selected_model_enum_declaration_rows,
        selected_model_enum_member_rows: rows.selected_model_enum_member_rows,
        selected_enum_declaration_rows: rows.selected_enum_declaration_rows,
        selected_enum_member_rows: rows.selected_enum_member_rows,
        selected_source_union_rows: rows.selected_source_union_rows,
        selected_source_record_rows: rows.selected_source_record_rows,
        selected_source_tuple_rows: rows.selected_source_tuple_rows,
        selected_model_enum_shape_rows: rows.selected_model_enum_shape_rows,
        selected_source_collection_rows: rows.selected_source_collection_rows,
        selected_exact_declaration_rows: rows.selected_exact_declaration_rows,
        selected_exact_field_rows: rows.selected_exact_field_rows,
        equality_producer_identity_rows: rows.equality_producer_identity_rows,
        equality_fact_witness_rows: rows.equality_fact_witness_rows,
        wire_nodes: nodes,
        node_wire_ids: std::collections::HashMap::new(),
        temporal_binding_rows: rows.temporal_binding_rows,
        temporal_rows: rows.temporal_rows,
    })
}

/// The node a `binding` term named `name` references, or `None` for any other
/// term.
fn binding_reference<'t>(term: &'t CheckedTerm, name: &str) -> Option<&'t CheckedNodeId> {
    let CheckedTerm::Binding { name: found, value } = term else {
        return None;
    };
    match (found.as_ref() == name, value.as_ref()) {
        (true, CheckedTerm::Reference { target }) => Some(target),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quire_contract_model::CheckedDeclaration;

    fn node(name: &str, form: &str) -> CheckedSemanticNodeV2 {
        let id = CheckedNodeId {
            package: "p".into(),
            path: vec![Segment::Name(name.into()), Segment::Name(form.into())],
            ordinal: 0,
        };
        CheckedSemanticNodeV2 {
            owner_model: None,
            node_id: id.clone(),
            schema_version: GRAPH_V2.into(),
            node_tag: "scalar_type".into(),
            semantic_form: form.into(),
            semantic_type: id,
            dependencies: Vec::new(),
            occurrences: Vec::new(),
            recursion_group: None,
            declaration: Some(CheckedDeclaration {
                qualified_name: vec![name.into()],
            }),
            owner: None,
            body: CheckedTerm::aggregate(Vec::new()),
        }
    }

    /// A name binds only when exactly one node declares it.
    #[test]
    fn a_declared_name_binds_only_when_exactly_one_node_declares_it() {
        let nodes = [node("State", "record"), node("Other", "record")];
        assert_eq!(declared(&nodes, "State", "missing").ok(), Some(0));
        assert_eq!(declared(&nodes, "Other", "missing").ok(), Some(1));
        assert!(matches!(
            declared(&nodes, "Absent", "missing"),
            Err(EmitRefusal::MonitorPlanInvariant("missing"))
        ));
        let twice = [node("State", "record"), node("State", "function")];
        assert!(matches!(
            declared(&twice, "State", "missing"),
            Err(EmitRefusal::MonitorPlanAmbiguous { name }) if name == "State"
        ));
    }
}
