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

use std::collections::BTreeSet;

use quire_contract_model::CheckedNodePathSegment as Segment;
use quire_contract_model::{CheckedSemanticNodeV2, CheckedTerm};
use quire_exact::Rational;

use super::{
    own_evidence, read_index_segments, write_index_segments, CheckedNodeId, CheckedOccurrence,
    CheckedOccurrenceRole, CheckedPackageSidecarsRef, CheckedPackageSourceCorrespondenceRef,
    CheckedPackageV2, CheckedSemanticGraphV2, CheckedSourceMapEntry, Emission, EmitRefusal,
    EmittedPackage, StructuralIds, GRAPH_V2,
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
                by_id(target).is_some_and(|child| {
                    child.node_tag.as_ref() == "temporal"
                        && child.semantic_form.as_ref() == "formula"
                })
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
/// graph).
pub fn emit_monitor_plan_over(
    base: &EmittedPackage,
    identity: qsl_semantics::library::LibraryName,
    evidence: &quire_contract_model::CheckedPackageEvidence,
    plan: &MonitorPlanWire<'_>,
) -> Result<Emission, EmitRefusal> {
    let read = crate::checked_v2::read_checked_package_v2(
        base.bytes(),
        identity,
        crate::checked_v2::V2ReadLimits::default(),
        evidence,
    )
    .map_err(|_| invariant("the base package does not read back"))?;
    emit_monitor_plan(&read.into_value().admitted, plan)
}

/// Write the plan root and entries onto the admitted package `base`.
pub fn emit_monitor_plan(
    base: &CheckedPackageV2,
    plan: &MonitorPlanWire<'_>,
) -> Result<Emission, EmitRefusal> {
    let ids = &StructuralIds::default();
    let mut nodes = base.graph().nodes.clone();
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
    let mut changes = Vec::new();
    for node in nodes.iter_mut() {
        if node.semantic_form.as_ref() == "record_value"
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
            node.semantic_form.as_ref() == "sequence"
                && node.node_tag.as_ref() == "composite_type"
                && node
                    .body
                    .reference_targets()
                    .iter()
                    .any(|target| **target == result)
        })
        .map(|node| node.node_id.clone())
        .ok_or_else(|| invariant("the results sequence is in the graph"))?;
    let scalar = |form: &str| -> Option<CheckedNodeId> {
        nodes
            .iter()
            .find(|node| {
                node.node_tag.as_ref() == "scalar_type" && node.semantic_form.as_ref() == form
            })
            .map(|node| node.node_id.clone())
    };
    let integer = scalar("integer").ok_or_else(|| invariant("the integer type is in the graph"))?;
    let text = scalar("text").ok_or_else(|| invariant("the text type is in the graph"))?;
    let clause_region = source_map
        .iter()
        .find(|entry| entry.node_id == nodes[clause_at].node_id)
        .map(|entry| entry.regions.clone())
        .ok_or_else(|| invariant("the clause is located"))?;
    let rational = match scalar("rational") {
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
    if clause.node_tag.as_ref() != "temporal" || clause.semantic_form.as_ref() != "temporal_clause"
    {
        return Err(invariant("the selected node is a temporal clause"));
    }
    let tick_unit = nodes
        .iter()
        .find(|node| {
            node.node_tag.as_ref() == "scalar_type"
                && node.semantic_form.as_ref() == "unit"
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
    finish(base, nodes, source_map, inspection)
}

/// The shared tail: index the graph, encode and wrap the bytes.
fn finish(
    base: &CheckedPackageV2,
    nodes: Vec<CheckedSemanticNodeV2>,
    source_map: Vec<CheckedSourceMapEntry>,
    inspection_source_map: qsl_foundation::source::provenance::PackageSourceMap,
) -> Result<Emission, EmitRefusal> {
    let lock = base.lock().clone();
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: base.graph().graph_version.clone(),
        nodes,
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
            crate::checked::EMIT_LIMITS,
        )
        .map_err(EmitRefusal::from)
    })?;
    Ok(Emission {
        package,
        omitted: Vec::new(),
        evidence: own_evidence(&lock),
        capability_report,
        source_map: inspection_source_map,
        selected_operation_member_rows: Vec::new(),
        selected_tuple_declaration_rows: Vec::new(),
        selected_source_unit_rows: Vec::new(),
        selected_model_enum_declaration_rows: Vec::new(),
        selected_model_enum_member_rows: Vec::new(),
        equality_producer_identity_rows: Vec::new(),
        equality_fact_witness_rows: Vec::new(),
        wire_nodes: semantic_graph.nodes,
    })
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
