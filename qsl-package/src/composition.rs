// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-530, QSpec FR-491 "Checked-package node": the strict writer and
//! reader of a `correspondence`/`component_composition` node.
//!
//! The IR crate has no term for this node, so the node's JSON is written and
//! read here, as the implementation guarantee's is: [`write_composition`]
//! writes the node of a checked declaration, and [`read_composition`] reads it
//! back in QSpec FR-491's reader order (identity and dependencies, the body
//! term, canonical order, each target, then static checks 2 to 5). A target in
//! a component's package resolves through the package's dependency closure
//! (FR-322). The v2 byte emitter refuses a package that holds such a node
//! ([`crate::EmitRefusal::Composition`]).
//!
//! The node is `{node_id, node_tag, semantic_form, semantic_type, body,
//! dependencies, occurrences, source_map}`. Every node reference is a
//! structural id; `source_map` holds the byte span of the declaration, of the
//! rule and of each member, each array in the body's order.

use qsl_foundation::Span;
use qsl_semantics::check::{
    CheckedComposition, ClauseEntry, ClauseRef, ComponentEntry, CompositionRule, CompositionSpans,
    CompositionTerm, PremiseEntry, ReadClause, ReadComponent, ReadComposition, ReadEnvironment,
    ReadGuaranteeMember, ReadPremise, ReadSupply, SupplyEntry,
};
use quire_exact::NodeKey;
use serde_json::{json, Value};

use crate::guarantee::{
    array, environment_value, id_value, member, node_ref, object, read_environment_entries,
    read_span, read_spans, span_value, text, NodeReadRefusal, NodeWriteRefusal,
};
use crate::CheckedPackage;

/// The `node_tag` of a composition node.
const NODE_TAG: &str = "correspondence";
/// The `semantic_form` and body `term` of a composition node.
const FORM: &str = "component_composition";

fn clause_value(entry: &ClauseEntry) -> Result<Value, NodeWriteRefusal> {
    Ok(json!({
        "component": entry.component(),
        "clause": ref_value(entry.clause())?,
    }))
}

fn ref_value(reference: &qsl_semantics::check::NodeRef) -> Result<Value, NodeWriteRefusal> {
    id_value(
        reference
            .admitted()
            .ok_or(NodeWriteRefusal::UnresolvedPlaceholder)?,
    )
}

fn premise_value(entry: &PremiseEntry) -> Result<Value, NodeWriteRefusal> {
    Ok(json!({
        "guarantee": clause_value(entry.guarantee())?,
        "on": clause_value(entry.on())?,
    }))
}

/// The composition node of `composition`, written strictly (QSpec FR-491
/// "Checked-package node").
///
/// # Errors
///
/// [`NodeWriteRefusal`] when the body holds an unresolved reference.
pub fn write_composition(composition: &CheckedComposition) -> Result<Value, NodeWriteRefusal> {
    let term = composition.term();
    let id = id_value(composition.node())?;
    let components = term
        .components()
        .iter()
        .map(|entry| {
            Ok(json!({
                "name": entry.name(),
                "package": entry.package(),
                "subject": ref_value(entry.subject())?,
            }))
        })
        .collect::<Result<Vec<_>, NodeWriteRefusal>>()?;
    let guarantees = term
        .guarantees()
        .iter()
        .map(clause_value)
        .collect::<Result<Vec<_>, _>>()?;
    let relies = term
        .relies()
        .iter()
        .map(premise_value)
        .collect::<Result<Vec<_>, _>>()?;
    let liveness_dependencies = term
        .liveness_dependencies()
        .iter()
        .map(premise_value)
        .collect::<Result<Vec<_>, _>>()?;
    let supplies = term
        .supplies()
        .iter()
        .map(|entry| {
            Ok(json!({
                "kind": entry.kind(),
                "granularity": entry.granularity(),
                "context": ref_value(entry.context())?,
                "operation": entry.operation(),
            }))
        })
        .collect::<Result<Vec<_>, NodeWriteRefusal>>()?;
    let environment = term
        .environment()
        .iter()
        .map(environment_value)
        .collect::<Result<Vec<_>, _>>()?;
    let dependencies = term
        .targets()
        .map_err(|_| NodeWriteRefusal::UnresolvedPlaceholder)?
        .iter()
        .map(id_value)
        .collect::<Result<Vec<_>, _>>()?;
    // The member spans, in the body's canonical order.
    let clause_of = |entry: &ClauseEntry| -> Option<ClauseRef> {
        let node = entry.clause().admitted()?;
        composition
            .clauses()
            .iter()
            .map(|facts| &facts.reference)
            .find(|reference| reference.component == entry.component() && reference.node == *node)
            .cloned()
    };
    let component_spans: Vec<Value> = term
        .components()
        .iter()
        .filter_map(|entry| {
            composition
                .components()
                .iter()
                .find(|member| member.name == entry.name())
                .map(|member| span_value(member.source))
        })
        .collect();
    let guarantee_spans: Vec<Value> = term
        .guarantees()
        .iter()
        .filter_map(|entry| {
            let reference = clause_of(entry)?;
            composition
                .guarantees()
                .iter()
                .find(|member| member.reference == reference)
                .map(|member| span_value(member.source))
        })
        .collect();
    let premise_spans = |entries: &[PremiseEntry], members: &[qsl_semantics::check::CompositionPremise]| {
        entries
            .iter()
            .filter_map(|entry| {
                let guarantee = clause_of(entry.guarantee())?;
                let on = clause_of(entry.on())?;
                members
                    .iter()
                    .find(|member| member.guarantee == guarantee && member.on == on)
                    .map(|member| span_value(member.source))
            })
            .collect::<Vec<Value>>()
    };
    let supply_spans: Vec<Value> = term
        .supplies()
        .iter()
        .filter_map(|entry| {
            let context = entry.context().admitted()?;
            composition
                .supplies()
                .iter()
                .find(|member| {
                    member.context == *context
                        && member.operation.as_str() == entry.operation()
                        && qsl_semantics::check::kind_spelling(member.kind) == entry.kind()
                        && qsl_semantics::check::granularity_spelling(member.granularity)
                            == entry.granularity()
                })
                .map(|member| span_value(member.source))
        })
        .collect();
    let environment_spans: Vec<Value> = term
        .environment()
        .iter()
        .filter_map(|entry| {
            composition
                .environment()
                .iter()
                .find(|member| member.name.as_str() == entry.name())
                .map(|member| span_value(member.source))
        })
        .collect();
    let spans = composition.spans();
    Ok(json!({
        "node_id": id,
        "node_tag": NODE_TAG,
        "semantic_form": FORM,
        "semantic_type": id,
        "body": {
            "term": FORM,
            "rule": term.rule().as_str(),
            "components": components,
            "guarantees": guarantees,
            "relies": relies,
            "liveness_dependencies": liveness_dependencies,
            "supplies": supplies,
            "environment": environment,
        },
        "dependencies": dependencies,
        "occurrences": [{"role": "declaration", "ordinal": 0}],
        "source_map": {
            "declaration": span_value(spans.declaration),
            "rule": span_value(spans.rule),
            "components": component_spans,
            "guarantees": guarantee_spans,
            "relies": premise_spans(term.relies(), composition.relies()),
            "liveness_dependencies":
                premise_spans(term.liveness_dependencies(), composition.liveness_dependencies()),
            "supplies": supply_spans,
            "environment": environment_spans,
        },
    }))
}

fn read_clause(value: &Value, at: &str) -> Result<ClauseEntry, NodeReadRefusal> {
    let map = object(value, &["component", "clause"], at)?;
    Ok(ClauseEntry::new(
        text(member(map, "component", at)?, &format!("{at}/component"))?.to_owned(),
        node_ref(member(map, "clause", at)?, &format!("{at}/clause"))?,
    ))
}

fn read_premises(value: &Value, at: &str) -> Result<Vec<PremiseEntry>, NodeReadRefusal> {
    array(value, at)?
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let at = format!("{at}/{index}");
            let map = object(entry, &["guarantee", "on"], &at)?;
            Ok(PremiseEntry::new(
                read_clause(member(map, "guarantee", &at)?, &format!("{at}/guarantee"))?,
                read_clause(member(map, "on", &at)?, &format!("{at}/on"))?,
            ))
        })
        .collect()
}

fn admitted(
    reference: &qsl_semantics::check::NodeRef,
    at: &str,
) -> Result<NodeKey, NodeReadRefusal> {
    reference
        .admitted()
        .cloned()
        .ok_or_else(|| NodeReadRefusal::graph(at))
}

fn read_clause_target(entry: &ClauseEntry, at: &str) -> Result<ReadClause, NodeReadRefusal> {
    Ok(ReadClause {
        component: entry.component().to_owned(),
        clause: admitted(entry.clause(), at)?,
    })
}

/// Reads the composition node `node` of `package` in QSpec FR-491's reader
/// order and reports the first defect: identity and dependencies, else
/// `invalid_semantic_graph`; a body `term` that is not the node's
/// `semantic_form`, `invalid_semantic_graph`; canonical order, else
/// `invalid_semantic_graph`; a target that is no node of its component's
/// package, `missing_declaration`/`missing-name`; then static checks 2 to 5
/// on the read values. A component's package resolves through `package`'s
/// dependency closure.
///
/// # Errors
///
/// The first defect.
#[ix_trace_rs::string_edge]
pub fn read_composition(
    package: &CheckedPackage,
    node: &Value,
) -> Result<CheckedComposition, NodeReadRefusal> {
    let top = object(
        node,
        &[
            "node_id",
            "node_tag",
            "semantic_form",
            "semantic_type",
            "body",
            "dependencies",
            "occurrences",
            "source_map",
        ],
        "",
    )?;
    // Identity and dependencies.
    let id = node_ref(member(top, "node_id", "")?, "/node_id")?;
    let name = match id.path().as_slice() {
        [quire_exact::PathSegment::Text(tag), quire_exact::PathSegment::Text(form), quire_exact::PathSegment::Text(name)]
            if tag == NODE_TAG && form == FORM && id.ordinal() == 0 =>
        {
            name.clone()
        }
        _ => return Err(NodeReadRefusal::graph("/node_id")),
    };
    if id.package() != package.graph().package_name() {
        return Err(NodeReadRefusal::graph("/node_id"));
    }
    if text(member(top, "node_tag", "")?, "/node_tag")? != NODE_TAG {
        return Err(NodeReadRefusal::graph("/node_tag"));
    }
    if text(member(top, "semantic_form", "")?, "/semantic_form")? != FORM {
        return Err(NodeReadRefusal::graph("/semantic_form"));
    }
    if node_ref(member(top, "semantic_type", "")?, "/semantic_type")? != id {
        return Err(NodeReadRefusal::graph("/semantic_type"));
    }
    let occurrences = array(member(top, "occurrences", "")?, "/occurrences")?;
    let expected = json!({"role": "declaration", "ordinal": 0});
    if occurrences.as_slice() != std::slice::from_ref(&expected) {
        return Err(NodeReadRefusal::graph("/occurrences"));
    }
    // The body, shaped.
    let body = object(
        member(top, "body", "")?,
        &[
            "term",
            "rule",
            "components",
            "guarantees",
            "relies",
            "liveness_dependencies",
            "supplies",
            "environment",
        ],
        "/body",
    )?;
    let rule = CompositionRule::from_spelling(text(member(body, "rule", "/body")?, "/body/rule")?)
        .ok_or_else(|| NodeReadRefusal::graph("/body/rule"))?;
    let mut components = Vec::new();
    for (index, entry) in array(member(body, "components", "/body")?, "/body/components")?
        .iter()
        .enumerate()
    {
        let at = format!("/body/components/{index}");
        let map = object(entry, &["name", "package", "subject"], &at)?;
        components.push(ComponentEntry::new(
            text(member(map, "name", &at)?, &format!("{at}/name"))?.to_owned(),
            text(member(map, "package", &at)?, &format!("{at}/package"))?.to_owned(),
            node_ref(member(map, "subject", &at)?, &format!("{at}/subject"))?,
        ));
    }
    let guarantees = array(member(body, "guarantees", "/body")?, "/body/guarantees")?
        .iter()
        .enumerate()
        .map(|(index, entry)| read_clause(entry, &format!("/body/guarantees/{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    let relies = read_premises(member(body, "relies", "/body")?, "/body/relies")?;
    let liveness_dependencies = read_premises(
        member(body, "liveness_dependencies", "/body")?,
        "/body/liveness_dependencies",
    )?;
    let mut supplies = Vec::new();
    for (index, entry) in array(member(body, "supplies", "/body")?, "/body/supplies")?
        .iter()
        .enumerate()
    {
        let at = format!("/body/supplies/{index}");
        let map = object(entry, &["kind", "granularity", "context", "operation"], &at)?;
        let kind = text(member(map, "kind", &at)?, &format!("{at}/kind"))?;
        let granularity = text(member(map, "granularity", &at)?, &format!("{at}/granularity"))?;
        if !matches!(kind, "weak" | "strong") || !matches!(granularity, "whole" | "each") {
            return Err(NodeReadRefusal::graph(at));
        }
        supplies.push(SupplyEntry::new(
            kind.to_owned(),
            granularity.to_owned(),
            node_ref(member(map, "context", &at)?, &format!("{at}/context"))?,
            text(member(map, "operation", &at)?, &format!("{at}/operation"))?.to_owned(),
        ));
    }
    let environment =
        read_environment_entries(member(body, "environment", "/body")?, "/body/environment")?;
    let term = CompositionTerm::from_wire(
        rule,
        components,
        guarantees,
        relies,
        liveness_dependencies,
        supplies,
        environment,
    );
    // Dependencies: exactly the unique, ascending set of every target.
    let expected_dependencies: Vec<Value> = term
        .targets()
        .map_err(|_| NodeReadRefusal::graph("/body"))?
        .iter()
        .map(|key| id_value(key).map_err(|_| NodeReadRefusal::graph("/dependencies")))
        .collect::<Result<_, _>>()?;
    if array(member(top, "dependencies", "")?, "/dependencies")?.as_slice()
        != expected_dependencies.as_slice()
    {
        return Err(NodeReadRefusal::graph("/dependencies"));
    }
    // The body term is the node's semantic form.
    if text(member(body, "term", "/body")?, "/body/term")? != FORM {
        return Err(NodeReadRefusal::graph("/body/term"));
    }
    // Canonical order.
    if !term.is_canonical() {
        return Err(NodeReadRefusal::graph("/body"));
    }
    // The source map.
    let source_map = object(
        member(top, "source_map", "")?,
        &[
            "declaration",
            "rule",
            "components",
            "guarantees",
            "relies",
            "liveness_dependencies",
            "supplies",
            "environment",
        ],
        "/source_map",
    )?;
    let spans_of = |name: &str, count: usize| {
        read_spans(
            member(source_map, name, "/source_map")?,
            count,
            &format!("/source_map/{name}"),
        )
    };
    let span_of = |name: &str| -> Result<Span, NodeReadRefusal> {
        read_span(
            member(source_map, name, "/source_map")?,
            &format!("/source_map/{name}"),
        )
    };
    let spans = CompositionSpans {
        declaration: span_of("declaration")?,
        rule: span_of("rule")?,
    };
    let component_spans = spans_of("components", term.components().len())?;
    let guarantee_spans = spans_of("guarantees", term.guarantees().len())?;
    let rely_spans = spans_of("relies", term.relies().len())?;
    let dependency_spans = spans_of("liveness_dependencies", term.liveness_dependencies().len())?;
    let supply_spans = spans_of("supplies", term.supplies().len())?;
    let environment_spans = spans_of("environment", term.environment().len())?;
    let read_premise_list = |entries: &[PremiseEntry],
                             spans: Vec<Span>,
                             at: &str|
     -> Result<Vec<ReadPremise>, NodeReadRefusal> {
        entries
            .iter()
            .zip(spans)
            .map(|(entry, span)| {
                Ok(ReadPremise {
                    guarantee: read_clause_target(entry.guarantee(), at)?,
                    on: read_clause_target(entry.on(), at)?,
                    span,
                })
            })
            .collect()
    };
    let read = ReadComposition {
        name,
        rule,
        components: term
            .components()
            .iter()
            .zip(component_spans)
            .map(|(entry, span)| {
                Ok(ReadComponent {
                    name: entry.name().to_owned(),
                    package: entry.package().to_owned(),
                    subject: admitted(entry.subject(), "/body/components")?,
                    span,
                })
            })
            .collect::<Result<_, NodeReadRefusal>>()?,
        guarantees: term
            .guarantees()
            .iter()
            .zip(guarantee_spans)
            .map(|(entry, span)| {
                Ok(ReadGuaranteeMember {
                    clause: read_clause_target(entry, "/body/guarantees")?,
                    span,
                })
            })
            .collect::<Result<_, NodeReadRefusal>>()?,
        relies: read_premise_list(term.relies(), rely_spans, "/body/relies")?,
        liveness_dependencies: read_premise_list(
            term.liveness_dependencies(),
            dependency_spans,
            "/body/liveness_dependencies",
        )?,
        supplies: term
            .supplies()
            .iter()
            .zip(supply_spans)
            .map(|(entry, span)| {
                Ok(ReadSupply {
                    kind: entry.kind().to_owned(),
                    granularity: entry.granularity().to_owned(),
                    context: admitted(entry.context(), "/body/supplies")?,
                    operation: entry.operation().to_owned(),
                    span,
                })
            })
            .collect::<Result<_, NodeReadRefusal>>()?,
        environment: term
            .environment()
            .iter()
            .zip(environment_spans)
            .map(|(entry, span)| ReadEnvironment {
                name: entry.name().to_owned(),
                supplier: entry.supplier().to_owned(),
                supplies: entry.supplies().clone(),
                span,
            })
            .collect(),
        spans,
    };
    let dependency = |identity: &str| {
        qsl_semantics::library::LibraryName::new(identity)
            .ok()
            .and_then(|name| package.dependencies().get(&name))
            .map(|dependency| dependency.graph())
    };
    package
        .graph()
        .read_composition(&dependency, &read)
        .map_err(|refusal| NodeReadRefusal {
            code: refusal.code().as_str(),
            cause: Some(refusal.cause()),
            at: String::new(),
        })
}
