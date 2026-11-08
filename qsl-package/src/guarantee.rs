// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-529, QSpec FR-490 "Checked-package node": the strict writer and
//! reader of a `correspondence`/`implementation_guarantee` node.
//!
//! The IR crate has no term for this node, so the node's JSON is written and
//! read here: [`write_implementation_guarantee`] writes the node of a
//! checked declaration, and [`read_implementation_guarantee`] reads it back
//! in QSpec FR-490's reader order (identity and dependencies, the body term,
//! canonical order, each target, then static checks 4 to 6). The v2 byte
//! emitter refuses a package that holds such a node
//! ([`crate::EmitRefusal::ImplementationGuarantee`]).
//!
//! The node is `{node_id, node_tag, semantic_form, semantic_type, body,
//! dependencies, occurrences, source_map}`. Every node reference is a
//! structural id; `source_map` holds the byte span of the declaration and of
//! each member, `progress` and `environment` in the body's order.

use ix_trace_rs::string_edge;
use qsl_foundation::Span;
use qsl_semantics::check::wire_id::structural_id;
use qsl_semantics::check::{
    CheckedImplementationGuarantee, EnvironmentEntry, GuaranteeSpans, ImplementationGuaranteeTerm,
    MeasureEntry, NodeRef, ProgressEntry, ReadEnvironment, ReadGuarantee, ReadProgress,
    SuppliesEntry,
};
use quire_exact::{NodeKey, PathSegment};
use serde_json::{json, Map, Value};

use crate::CheckedPackage;

/// The `node_tag` of a guarantee node.
const NODE_TAG: &str = "correspondence";
/// The `semantic_form` and body `term` of a guarantee node.
const FORM: &str = "implementation_guarantee";

/// Why a guarantee node was refused on reading: the catalog code and cause,
/// and the JSON pointer of the defect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeReadRefusal {
    /// `invalid_semantic_graph`, `missing_declaration` or
    /// `invalid_model_binding`.
    pub code: &'static str,
    /// The closed cause, where the code has one.
    pub cause: Option<&'static str>,
    /// The RFC 6901 pointer of the member the defect is at, within the node.
    pub at: String,
}

impl NodeReadRefusal {
    pub(crate) fn graph(at: impl Into<String>) -> Self {
        Self {
            code: "invalid_semantic_graph",
            cause: None,
            at: at.into(),
        }
    }
}

/// Why a guarantee node cannot be written.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NodeWriteRefusal {
    /// A node reference in the body is a lowering placeholder.
    #[error("the guarantee node names an unresolved lowering placeholder")]
    UnresolvedPlaceholder,
}

pub(crate) fn span_value(span: Span) -> Value {
    json!({"start": span.start, "end": span.end})
}

pub(crate) fn id_value(key: &NodeKey) -> Result<Value, NodeWriteRefusal> {
    serde_json::to_value(structural_id(key)).map_err(|_| NodeWriteRefusal::UnresolvedPlaceholder)
}

pub(crate) fn ref_value(
    reference: &qsl_semantics::check::NodeRef,
) -> Result<Value, NodeWriteRefusal> {
    id_value(
        reference
            .admitted()
            .ok_or(NodeWriteRefusal::UnresolvedPlaceholder)?,
    )
}

/// One `environment` entry as its node member (QSpec FR-490 "Checked-package
/// node"), shared with the composition node.
pub(crate) fn environment_value(entry: &EnvironmentEntry) -> Result<Value, NodeWriteRefusal> {
    Ok(json!({
        "name": entry.name(),
        "supplier": entry.supplier(),
        "supplies": match entry.supplies() {
            SuppliesEntry::Fairness { kind, granularity, context, operation } => json!({
                "fairness": {
                    "kind": kind,
                    "granularity": granularity,
                    "context": ref_value(context)?,
                    "operation": operation,
                },
            }),
            SuppliesEntry::Assumption(node) => json!({"assumption": ref_value(node)?}),
        },
    }))
}

/// The guarantee node of `guarantee`, written strictly (QSpec FR-490
/// "Checked-package node").
///
/// # Errors
///
/// [`NodeWriteRefusal`] when the body holds an unresolved reference.
pub fn write_implementation_guarantee(
    guarantee: &CheckedImplementationGuarantee,
) -> Result<Value, NodeWriteRefusal> {
    let term = guarantee.term();
    let id = id_value(guarantee.node())?;
    let progress = term
        .progress()
        .iter()
        .map(|entry| {
            Ok(json!({
                "context": ref_value(entry.context())?,
                "operation": entry.operation(),
                "measure": match entry.measure() {
                    MeasureEntry::Decreases(segments) => json!({"decreases": segments}),
                    MeasureEntry::Bound(count) => json!({"bound": count}),
                },
            }))
        })
        .collect::<Result<Vec<_>, NodeWriteRefusal>>()?;
    let environment = term
        .environment()
        .iter()
        .map(environment_value)
        .collect::<Result<Vec<_>, NodeWriteRefusal>>()?;
    let dependencies = term
        .targets()
        .map_err(|_| NodeWriteRefusal::UnresolvedPlaceholder)?
        .iter()
        .map(id_value)
        .collect::<Result<Vec<_>, _>>()?;
    // The member spans, in the body's canonical order.
    let progress_spans: Vec<Value> = term
        .progress()
        .iter()
        .filter_map(|entry| {
            let context = entry.context().admitted()?;
            guarantee
                .progress()
                .iter()
                .find(|member| {
                    member.context == *context && member.operation.as_str() == entry.operation()
                })
                .map(|member| span_value(member.source))
        })
        .collect();
    let environment_spans: Vec<Value> = term
        .environment()
        .iter()
        .filter_map(|entry| {
            guarantee
                .environment()
                .iter()
                .find(|member| member.name.as_str() == entry.name())
                .map(|member| span_value(member.source))
        })
        .collect();
    let spans = guarantee.spans();
    Ok(json!({
        "node_id": id,
        "node_tag": NODE_TAG,
        "semantic_form": FORM,
        "semantic_type": id,
        "body": {
            "term": FORM,
            "clause": ref_value(term.clause())?,
            "relation": ref_value(term.relation())?,
            "initial": term.initial(),
            "progress": progress,
            "environment": environment,
        },
        "dependencies": dependencies,
        "occurrences": [{"role": "declaration", "ordinal": 0}],
        "source_map": {
            "declaration": span_value(spans.declaration),
            "clause": span_value(spans.clause),
            "relation": span_value(spans.relation),
            "initial": span_value(spans.initial),
            "progress": progress_spans,
            "environment": environment_spans,
        },
    }))
}

/// The members of `value` as an object with exactly `names`.
pub(crate) fn object<'v>(
    value: &'v Value,
    names: &[&str],
    at: &str,
) -> Result<&'v Map<String, Value>, NodeReadRefusal> {
    let map = value
        .as_object()
        .ok_or_else(|| NodeReadRefusal::graph(at))?;
    if map.len() != names.len() || names.iter().any(|name| !map.contains_key(*name)) {
        return Err(NodeReadRefusal::graph(at));
    }
    Ok(map)
}

pub(crate) fn member<'v>(
    map: &'v Map<String, Value>,
    name: &str,
    at: &str,
) -> Result<&'v Value, NodeReadRefusal> {
    map.get(name)
        .ok_or_else(|| NodeReadRefusal::graph(format!("{at}/{name}")))
}

pub(crate) fn text<'v>(value: &'v Value, at: &str) -> Result<&'v str, NodeReadRefusal> {
    value.as_str().ok_or_else(|| NodeReadRefusal::graph(at))
}

pub(crate) fn array<'v>(value: &'v Value, at: &str) -> Result<&'v Vec<Value>, NodeReadRefusal> {
    value.as_array().ok_or_else(|| NodeReadRefusal::graph(at))
}

pub(crate) fn strings(value: &Value, at: &str) -> Result<Vec<String>, NodeReadRefusal> {
    array(value, at)?
        .iter()
        .enumerate()
        .map(|(index, segment)| text(segment, &format!("{at}/{index}")).map(str::to_owned))
        .collect()
}

#[string_edge]
/// The deepest operand nesting a node id may carry.
const MAX_ID_DEPTH: usize = 512;

/// The key of the structural id `value` (`{package, path, ordinal}`, each
/// operand a nested id), or `None` when it is not one.
pub(crate) fn structural_key(value: &Value, depth: usize) -> Option<NodeKey> {
    if depth > MAX_ID_DEPTH {
        return None;
    }
    let map = value.as_object().filter(|map| map.len() == 3)?;
    let package = map.get("package")?.as_str()?;
    let ordinal = map.get("ordinal")?.as_u64()?;
    let path = map
        .get("path")?
        .as_array()?
        .iter()
        .map(|segment| match segment {
            Value::String(text) => Some(PathSegment::Text(text.clone())),
            nested => structural_key(nested, depth + 1).map(PathSegment::Node),
        })
        .collect::<Option<Vec<_>>>()?;
    NodeKey::new(package, path, ordinal).ok()
}

pub(crate) fn node_ref(value: &Value, at: &str) -> Result<NodeKey, NodeReadRefusal> {
    structural_key(value, 0).ok_or_else(|| NodeReadRefusal::graph(at))
}

pub(crate) fn read_span(value: &Value, at: &str) -> Result<Span, NodeReadRefusal> {
    let map = object(value, &["start", "end"], at)?;
    let number = |name: &str| {
        member(map, name, at)?
            .as_u64()
            .and_then(|number| usize::try_from(number).ok())
            .ok_or_else(|| NodeReadRefusal::graph(format!("{at}/{name}")))
    };
    Ok(Span {
        start: number("start")?,
        end: number("end")?,
    })
}

pub(crate) fn read_spans(
    value: &Value,
    count: usize,
    at: &str,
) -> Result<Vec<Span>, NodeReadRefusal> {
    let entries = array(value, at)?;
    if entries.len() != count {
        return Err(NodeReadRefusal::graph(at));
    }
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| read_span(entry, &format!("{at}/{index}")))
        .collect()
}

/// The `environment` entries `value` holds, at `at` (QSpec FR-490
/// "Checked-package node"), shared with the composition node.
pub(crate) fn read_environment_entries(
    value: &Value,
    at: &str,
) -> Result<Vec<EnvironmentEntry>, NodeReadRefusal> {
    let mut environment_entries = Vec::new();
    for (index, entry) in array(value, at)?.iter().enumerate() {
        let at = format!("{at}/{index}");
        let map = object(entry, &["name", "supplier", "supplies"], &at)?;
        let supplies_at = format!("{at}/supplies");
        let supplies_map = member(map, "supplies", &at)?
            .as_object()
            .filter(|map| map.len() == 1)
            .ok_or_else(|| NodeReadRefusal::graph(&supplies_at))?;
        let supplies = if let Some(fairness) = supplies_map.get("fairness") {
            let fairness_at = format!("{supplies_at}/fairness");
            let fairness = object(
                fairness,
                &["kind", "granularity", "context", "operation"],
                &fairness_at,
            )?;
            let kind = text(member(fairness, "kind", &fairness_at)?, &fairness_at)?;
            let granularity = text(member(fairness, "granularity", &fairness_at)?, &fairness_at)?;
            if !matches!(kind, "weak" | "strong") || !matches!(granularity, "whole" | "each") {
                return Err(NodeReadRefusal::graph(fairness_at));
            }
            SuppliesEntry::Fairness {
                kind: kind.to_owned(),
                granularity: granularity.to_owned(),
                context: NodeRef::Admitted(node_ref(
                    member(fairness, "context", &fairness_at)?,
                    &format!("{fairness_at}/context"),
                )?),
                operation: text(
                    member(fairness, "operation", &fairness_at)?,
                    &format!("{fairness_at}/operation"),
                )?
                .to_owned(),
            }
        } else if let Some(assumption) = supplies_map.get("assumption") {
            SuppliesEntry::Assumption(NodeRef::Admitted(node_ref(
                assumption,
                &format!("{supplies_at}/assumption"),
            )?))
        } else {
            return Err(NodeReadRefusal::graph(supplies_at));
        };
        environment_entries.push(EnvironmentEntry::new(
            text(member(map, "name", &at)?, &format!("{at}/name"))?.to_owned(),
            text(member(map, "supplier", &at)?, &format!("{at}/supplier"))?.to_owned(),
            supplies,
        ));
    }
    Ok(environment_entries)
}

/// Reads the guarantee node `node` of `package` in QSpec FR-490's reader
/// order and reports the first defect: identity and dependencies, else
/// `invalid_semantic_graph`; a body `term` that is not the node's
/// `semantic_form`, `invalid_semantic_graph`; canonical order, else
/// `invalid_semantic_graph`; a target that is no node of the package,
/// `missing_declaration`/`missing-name`; a `clause` target that is not an
/// infinite-trace temporal claim, a `relation` target that is not an
/// `abstraction_relation` node, or an `assumption` target that is not a
/// `predicate`, `invalid_model_binding`/`malformed-declaration`; then static
/// checks 4 to 6 on the read values.
///
/// # Errors
///
/// The first defect.
#[ix_trace_rs::string_edge]
pub fn read_implementation_guarantee(
    package: &CheckedPackage,
    node: &Value,
) -> Result<CheckedImplementationGuarantee, NodeReadRefusal> {
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
            "clause",
            "relation",
            "initial",
            "progress",
            "environment",
        ],
        "/body",
    )?;
    let clause = node_ref(member(body, "clause", "/body")?, "/body/clause")?;
    let relation = node_ref(member(body, "relation", "/body")?, "/body/relation")?;
    let initial = strings(member(body, "initial", "/body")?, "/body/initial")?;
    let mut progress_entries = Vec::new();
    for (index, entry) in array(member(body, "progress", "/body")?, "/body/progress")?
        .iter()
        .enumerate()
    {
        let at = format!("/body/progress/{index}");
        let map = object(entry, &["context", "operation", "measure"], &at)?;
        let measure = member(map, "measure", &at)?;
        let measure_at = format!("{at}/measure");
        let measure_map = measure
            .as_object()
            .filter(|map| map.len() == 1)
            .ok_or_else(|| NodeReadRefusal::graph(&measure_at))?;
        let measure = if let Some(path) = measure_map.get("decreases") {
            MeasureEntry::Decreases(strings(path, &format!("{measure_at}/decreases"))?)
        } else if let Some(count) = measure_map.get("bound") {
            MeasureEntry::Bound(
                count
                    .as_u64()
                    .ok_or_else(|| NodeReadRefusal::graph(format!("{measure_at}/bound")))?,
            )
        } else {
            return Err(NodeReadRefusal::graph(measure_at));
        };
        progress_entries.push(ProgressEntry::new(
            node_ref(member(map, "context", &at)?, &format!("{at}/context"))?,
            text(member(map, "operation", &at)?, &format!("{at}/operation"))?.to_owned(),
            measure,
        ));
    }
    let environment_entries =
        read_environment_entries(member(body, "environment", "/body")?, "/body/environment")?;
    let term = ImplementationGuaranteeTerm::from_wire(
        clause.clone(),
        relation.clone(),
        initial.clone(),
        progress_entries,
        environment_entries,
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
            "clause",
            "relation",
            "initial",
            "progress",
            "environment",
        ],
        "/source_map",
    )?;
    let span_of = |name: &str| {
        read_span(
            member(source_map, name, "/source_map")?,
            &format!("/source_map/{name}"),
        )
    };
    let spans = GuaranteeSpans {
        declaration: span_of("declaration")?,
        clause: span_of("clause")?,
        relation: span_of("relation")?,
        initial: span_of("initial")?,
    };
    let progress_spans = read_spans(
        member(source_map, "progress", "/source_map")?,
        term.progress().len(),
        "/source_map/progress",
    )?;
    let environment_spans = read_spans(
        member(source_map, "environment", "/source_map")?,
        term.environment().len(),
        "/source_map/environment",
    )?;
    let read = ReadGuarantee {
        name,
        clause,
        relation,
        initial,
        progress: term
            .progress()
            .iter()
            .zip(progress_spans)
            .map(|(entry, span)| {
                Ok(ReadProgress {
                    context: entry
                        .context()
                        .admitted()
                        .cloned()
                        .ok_or_else(|| NodeReadRefusal::graph("/body/progress"))?,
                    operation: entry.operation().to_owned(),
                    measure: entry.measure().clone(),
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
    package
        .graph()
        .read_implementation_guarantee(package.abstraction(), &read)
        .map_err(|refusal| NodeReadRefusal {
            code: refusal.code().as_str(),
            cause: Some(refusal.cause()),
            at: String::new(),
        })
}
