// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-384: the protocol occurrence schema and the runtime binding requirements,
//! derived from the protocol nodes of a checked package.
//!
//! The derivation reads protocol nodes only. It runs over the in-process
//! checked package ([`CheckedPackage::protocol_occurrence_schemas`]) and over
//! the package IR's reader admits ([`protocol_occurrence_schemas_from_read`]);
//! both adapt their node bodies into one neutral term tree and share one
//! derivation, so the two results are equal by construction and by test.
//! Nothing here fabricates a workflow instance, role instance, repeat
//! ordinal, delivery or clock observation: those are the consumer's.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::string_edge;
use qsl_foundation::source::provenance::{PackageSourceMap, SourceRegion};
use qsl_semantics::check::{
    BindingValue, BodyTerm, GroupMember, GroupTerm, LeafTerm, LiteralValue, MemberTerm, NodeTag,
    TupleMember,
};
use qsl_semantics::value::Member;
use quire_contract_model::{CheckedPackageV2, CheckedTerm};
use quire_exact::{Integer, NodeKey};

use crate::checked::CheckedPackage;
use crate::checked_v2::wire_graph;

/// Why a protocol node could not be read into the occurrence schema.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum OccurrenceRefusal {
    /// A protocol node does not have the body layout QSpec FR-464 to FR-466
    /// define, so a member FR-384 reads is missing or has the wrong shape.
    #[error("protocol node has no well-formed `{member}` member")]
    Malformed {
        /// The protocol node.
        node: NodeKey,
        /// The member that is missing or misshapen.
        member: &'static str,
    },
    /// A node a protocol names is not a node of the package.
    #[error("protocol node names a node the package does not hold")]
    UnknownNode {
        /// The protocol node holding the reference.
        node: NodeKey,
    },
    /// The wire identities of the package do not decode to structural keys.
    #[error("package node identities do not decode to structural keys")]
    Identity,
    /// The package's source map is not readable.
    #[error("package source map is not readable")]
    SourceMap,
}

/// The repeat dimension a control holds in a runtime key (FR-384 Behavior).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepeatDimension {
    /// A control held in a `repeat` body: ordinals in `[0, upper_exclusive)`.
    Iteration {
        /// The `repeat` control that introduces the dimension.
        repeat: NodeKey,
        /// The authored maximum, or `None` for a `repeat` that writes none
        /// (an unbounded domain). Zero is an empty domain.
        upper_exclusive: Option<Integer>,
    },
    /// A control held in the `exhausted` control: the single value is the
    /// authored maximum.
    Exhaustion {
        /// The `repeat` control that introduces the dimension.
        repeat: NodeKey,
        /// The authored maximum.
        value: Integer,
    },
}

/// The participant that owns a control, from the nodes alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlOwner {
    /// A role node.
    Role(NodeKey),
    /// A `sequence`, `parallel`, `await`, `check` or `fence`: S4 derives no
    /// participant owner for it.
    Structural,
}

/// How many objects a role slot holds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoleReplication {
    /// One role instance for each assessed workflow.
    Static,
    /// A role spawned for each object of a population.
    Replicated {
        /// The authored population text.
        population: String,
        /// The authored spawn maximum.
        max: Integer,
        /// `workflow`, `scope` or `until`.
        lifetime: String,
    },
}

/// One role slot of a protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleSlot {
    /// The role node's key.
    pub key: NodeKey,
    /// The role node's declaration path, one entry for each segment.
    pub path: Vec<String>,
    /// The role's declaration region, from the package's source map.
    pub region: Vec<SourceRegion>,
    /// Static, or replicated with population, maximum and lifetime.
    pub replication: RoleReplication,
}

/// One authored control occurrence of a protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlEntry {
    /// The control node's key.
    pub key: NodeKey,
    /// The control node's declaration path, one entry for each segment.
    pub path: Vec<String>,
    /// The control's operator (`sequence`, `repeat`, `send`, ...).
    pub operator: String,
    /// The control's declaration region, from the package's source map.
    pub region: Vec<SourceRegion>,
    /// A role node, or structural.
    pub owner: ControlOwner,
    /// Enclosing repeat dimensions, outermost first.
    pub dimensions: Vec<RepeatDimension>,
}

/// One compensation template of a protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompensationEntry {
    /// The template node's key.
    pub key: NodeKey,
    /// The template node's declaration path, one entry for each segment.
    pub path: Vec<String>,
    /// The template's declaration region, from the package's source map.
    pub region: Vec<SourceRegion>,
    /// The role that runs the template.
    pub role: NodeKey,
    /// The forward `effect` control the template compensates.
    pub effect: NodeKey,
    /// The template's maximum attempts.
    pub maximum_attempts: Integer,
}

/// The static occurrence schema of one `protocol_clause` node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolOccurrenceSchema {
    /// The `protocol_clause` node's key; the one `workflow-instance` slot.
    pub workflow: NodeKey,
    /// The role slots, ordered by declaration path as UTF-8 bytes.
    pub roles: Vec<RoleSlot>,
    /// The control entries, ordered the same way.
    pub controls: Vec<ControlEntry>,
    /// The compensation entries, ordered the same way.
    pub compensations: Vec<CompensationEntry>,
}

/// The kind of runtime subject a protocol node implies (QSpec FR-210).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SubjectKind {
    /// One for the protocol clause.
    WorkflowInstance,
    /// One for each assessed workflow, and for a replicated role one for each
    /// spawned object.
    RoleInstance {
        /// Whether the role is replicated.
        per_spawned_object: bool,
    },
    /// A `send`'s message.
    Message,
    /// A `send`.
    Send,
    /// A `receive`.
    Receive,
    /// A `receive`'s delivery.
    Delivery,
    /// An `attempt`.
    OperationAttempt,
    /// An `effect`.
    BusinessEffect,
    /// A `commit`.
    Commit,
    /// A compensation template.
    Compensation,
    /// A compensation template's attempt.
    CompensationAttempt,
    /// A compensation template's effect.
    CompensationEffect,
    /// The `finish` obligation's workflow-closure observation.
    WorkflowClosure,
}

/// One runtime binding a later runtime satisfies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeBindingRequirement {
    /// The subject kind.
    pub kind: SubjectKind,
    /// The subject node's key.
    pub subject: NodeKey,
    /// The `protocol_clause` node's key.
    pub scope: NodeKey,
    /// Always `None`: the consumer's binding supplies the authority.
    pub authority: Option<NodeKey>,
    /// Always `None`: the consumer's binding supplies the contract.
    pub contract: Option<NodeKey>,
}

/// The schema and the binding requirements of one protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolOccurrence {
    /// The occurrence schema.
    pub schema: ProtocolOccurrenceSchema,
    /// The runtime binding requirements, ordered by subject declaration path
    /// and then subject kind.
    pub requirements: Vec<RuntimeBindingRequirement>,
}

// ---------------------------------------------------------------------------
// The neutral term tree both node representations adapt into.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Term {
    Ref(NodeKey),
    Text(String),
    List(Vec<Term>),
    Group(Vec<(String, Term)>),
    Other,
}

struct Raw {
    key: NodeKey,
    form: String,
    operator: String,
    path: Vec<String>,
    args: Vec<(String, Term)>,
}

impl Raw {
    fn arg(&self, name: &'static str) -> Result<&Term, OccurrenceRefusal> {
        self.args
            .iter()
            .find_map(|(candidate, term)| (candidate == name).then_some(term))
            .ok_or_else(|| self.malformed(name))
    }

    fn malformed(&self, member: &'static str) -> OccurrenceRefusal {
        OccurrenceRefusal::Malformed {
            node: self.key.clone(),
            member,
        }
    }

    fn reference(&self, name: &'static str) -> Result<&NodeKey, OccurrenceRefusal> {
        match self.arg(name)? {
            Term::Ref(key) => Ok(key),
            _ => Err(self.malformed(name)),
        }
    }

    fn references(&self, name: &'static str) -> Result<Vec<&NodeKey>, OccurrenceRefusal> {
        match self.arg(name)? {
            Term::List(items) => items
                .iter()
                .map(|item| match item {
                    Term::Ref(key) => Ok(key),
                    _ => Err(self.malformed(name)),
                })
                .collect(),
            _ => Err(self.malformed(name)),
        }
    }

    /// An optional member: an aggregate of none or one entry.
    fn optional(&self, name: &'static str) -> Result<Option<&Term>, OccurrenceRefusal> {
        match self.arg(name)? {
            Term::List(items) => match items.as_slice() {
                [] => Ok(None),
                [one] => Ok(Some(one)),
                _ => Err(self.malformed(name)),
            },
            _ => Err(self.malformed(name)),
        }
    }

    fn group(&self, name: &'static str) -> Result<&[(String, Term)], OccurrenceRefusal> {
        match self.arg(name)? {
            Term::Group(fields) => Ok(fields),
            Term::List(items) if items.is_empty() => Ok(&[]),
            _ => Err(self.malformed(name)),
        }
    }

    /// The `body` references of a `cases`/`branches` record list.
    fn record_bodies(&self, name: &'static str) -> Result<Vec<&NodeKey>, OccurrenceRefusal> {
        let Term::List(records) = self.arg(name)? else {
            return Err(self.malformed(name));
        };
        records
            .iter()
            .map(|record| match record {
                Term::Group(fields) => field(fields, "body")
                    .and_then(as_ref)
                    .ok_or_else(|| self.malformed(name)),
                _ => Err(self.malformed(name)),
            })
            .collect()
    }
}

fn field<'a>(fields: &'a [(String, Term)], name: &str) -> Option<&'a Term> {
    fields
        .iter()
        .find_map(|(candidate, term)| (candidate == name).then_some(term))
}

fn as_ref(term: &Term) -> Option<&NodeKey> {
    match term {
        Term::Ref(key) => Some(key),
        _ => None,
    }
}

fn as_integer(term: &Term) -> Option<Integer> {
    match term {
        Term::Text(text) => text.parse().ok(),
        _ => None,
    }
}

/// A group is a non-empty run of named members; every other aggregate, the
/// empty one included, is a list.
fn aggregate(members: Vec<(Option<String>, Term)>) -> Term {
    if !members.is_empty() && members.iter().all(|(name, _)| name.is_some()) {
        Term::Group(
            members
                .into_iter()
                .filter_map(|(name, term)| name.map(|name| (name, term)))
                .collect(),
        )
    } else {
        Term::List(
            members
                .into_iter()
                .map(|(name, term)| match name {
                    Some(name) => Term::Group(vec![(name, term)]),
                    None => term,
                })
                .collect(),
        )
    }
}

// --- the in-process adapter ------------------------------------------------

fn leaf(leaf: &LeafTerm) -> Term {
    match leaf {
        LeafTerm::Reference { target } => target.admitted().cloned().map_or(Term::Other, Term::Ref),
        LeafTerm::Literal { value, .. } => match value {
            LiteralValue::Text(text) | LiteralValue::String(text) | LiteralValue::Enum(text) => {
                Term::Text(text.clone())
            }
            LiteralValue::Integer(integer) => Term::Text(integer.to_string()),
            LiteralValue::Count(count) => Term::Text(count.to_string()),
            _ => Term::Other,
        },
        LeafTerm::DependencyReference { .. } => Term::Other,
    }
}

fn group(group: &GroupTerm) -> Term {
    aggregate(
        group
            .members
            .iter()
            .map(|member| match member {
                GroupMember::Leaf(value) => (None, leaf(value)),
                GroupMember::Binding(binding) => (Some(binding.name.clone()), leaf(&binding.value)),
            })
            .collect(),
    )
}

fn binding_value(value: &BindingValue) -> Term {
    match value {
        BindingValue::Leaf(value) => leaf(value),
        BindingValue::Group(value) => group(value),
        BindingValue::Tuple(tuple) => Term::List(
            tuple
                .members
                .iter()
                .map(|member| match member {
                    TupleMember::Leaf(value) => leaf(value),
                    TupleMember::Group(value) => group(value),
                })
                .collect(),
        ),
        BindingValue::Aggregate(value) => {
            aggregate(value.members.iter().map(member_term).collect())
        }
        BindingValue::ExecutionPoint(_) => Term::Other,
    }
}

fn member_term(member: &MemberTerm) -> (Option<String>, Term) {
    match member {
        MemberTerm::Leaf(value) => (None, leaf(value)),
        MemberTerm::Group(value) => (None, group(value)),
        MemberTerm::Binding(binding) => (Some(binding.name.clone()), binding_value(&binding.value)),
    }
}

fn in_process_nodes(package: &CheckedPackage) -> Result<Vec<Raw>, OccurrenceRefusal> {
    let mut nodes = Vec::new();
    for node in package.graph().semantic_graph().nodes() {
        if node.node_tag() != NodeTag::Protocol {
            continue;
        }
        let key = node.key();
        let BodyTerm::Application(application) = node.body() else {
            return Err(OccurrenceRefusal::Malformed {
                node: key,
                member: "body",
            });
        };
        let Some(Member::ProfileOperator { operator }) = application.operation.member() else {
            return Err(OccurrenceRefusal::Malformed {
                node: key,
                member: "operation",
            });
        };
        let mut args = Vec::new();
        for argument in &application.arguments {
            if let (Some(name), term) = member_term(argument) {
                args.push((name, term));
            }
        }
        nodes.push(Raw {
            form: node.semantic_form().to_owned(),
            operator: operator.as_str().to_owned(),
            path: node
                .declaration()
                .map(|path| path.iter().map(|part| part.as_str().to_owned()).collect())
                .unwrap_or_default(),
            key,
            args,
        });
    }
    Ok(nodes)
}

// --- the read-back adapter -------------------------------------------------

fn read_term(
    term: &CheckedTerm,
    keys: &mut dyn FnMut(&quire_contract_model::CheckedNodeId) -> Option<NodeKey>,
) -> (Option<String>, Term) {
    match term {
        CheckedTerm::Reference { target } => (None, keys(target).map_or(Term::Other, Term::Ref)),
        CheckedTerm::Literal { value, .. } => (
            None,
            match value {
                serde_json::Value::String(text) => Term::Text(text.clone()),
                serde_json::Value::Number(number) => Term::Text(number.to_string()),
                _ => Term::Other,
            },
        ),
        CheckedTerm::Aggregate { members } => (
            None,
            aggregate(
                members
                    .iter()
                    .map(|member| read_term(member, keys))
                    .collect(),
            ),
        ),
        CheckedTerm::Binding { name, value } => (Some(name.to_string()), read_term(value, keys).1),
        _ => (None, Term::Other),
    }
}

fn read_nodes(package: &CheckedPackageV2) -> Result<Vec<Raw>, OccurrenceRefusal> {
    let graph = wire_graph(package).map_err(|_| OccurrenceRefusal::Identity)?;
    let mut nodes = Vec::new();
    for node in &package.graph().nodes {
        if &*node.node_tag != "protocol" {
            continue;
        }
        let key = graph
            .key(&node.node_id)
            .map_err(|_| OccurrenceRefusal::Identity)?;
        let CheckedTerm::Application {
            operation,
            arguments,
            ..
        } = &node.body
        else {
            return Err(OccurrenceRefusal::Malformed {
                node: key,
                member: "body",
            });
        };
        let operation =
            serde_json::to_value(operation).map_err(|_| OccurrenceRefusal::Malformed {
                node: key.clone(),
                member: "operation",
            })?;
        let Some(operator) = operation
            .pointer("/member/operator")
            .and_then(serde_json::Value::as_str)
        else {
            return Err(OccurrenceRefusal::Malformed {
                node: key,
                member: "operation",
            });
        };
        let mut lookup = |id: &quire_contract_model::CheckedNodeId| graph.key(id).ok();
        let mut args = Vec::new();
        for argument in arguments {
            if let (Some(name), term) = read_term(argument, &mut lookup) {
                args.push((name, term));
            }
        }
        nodes.push(Raw {
            form: (*node.semantic_form).to_owned(),
            operator: operator.to_owned(),
            path: node
                .declaration
                .as_ref()
                .map(|declaration| {
                    declaration
                        .qualified_name
                        .iter()
                        .map(|part| (**part).to_owned())
                        .collect()
                })
                .unwrap_or_default(),
            key,
            args,
        });
    }
    Ok(nodes)
}

// --- the derivation --------------------------------------------------------

fn declaration_region(map: &PackageSourceMap, node: &NodeKey) -> Vec<SourceRegion> {
    map.occurrences(node)
        .filter(|(key, _)| key.origin().role().as_str() == "declaration")
        .flat_map(|(_, regions)| regions.iter().cloned())
        .collect()
}

/// The control nodes a control holds: a `repeat`'s `body` and `exhausted`
/// control, a `sequence`'s children, a `choice` case's and a `parallel`
/// branch's body, and an `await`'s inline `event`, `then` and `timeout`. Each is paired with
/// the repeat dimension holding it adds.
#[string_edge]
fn held<'a>(
    control: &'a Raw,
) -> Result<Vec<(&'a NodeKey, Option<RepeatDimension>)>, OccurrenceRefusal> {
    let plain =
        |keys: Vec<&'a NodeKey>| keys.into_iter().map(|key| (key, None)).collect::<Vec<_>>();
    Ok(match control.operator.as_str() {
        "sequence" => plain(control.references("children")?),
        "choice" => plain(control.record_bodies("cases")?),
        "parallel" => plain(control.record_bodies("branches")?),
        "await" => plain(vec![
            control.reference("event")?,
            control.reference("then")?,
            control.reference("timeout")?,
        ]),
        "repeat" => {
            let maximum = control
                .optional("maximum")?
                .map(|term| as_integer(term).ok_or_else(|| control.malformed("maximum")))
                .transpose()?;
            let mut held = vec![(
                control.reference("body")?,
                Some(RepeatDimension::Iteration {
                    repeat: control.key.clone(),
                    upper_exclusive: maximum.clone(),
                }),
            )];
            if let Some(exhausted) = control.optional("exhausted")? {
                let (Some(target), Some(value)) = (as_ref(exhausted), maximum) else {
                    return Err(control.malformed("exhausted"));
                };
                held.push((
                    target,
                    Some(RepeatDimension::Exhaustion {
                        repeat: control.key.clone(),
                        value,
                    }),
                ));
            }
            held
        }
        _ => Vec::new(),
    })
}

#[string_edge]
fn owner_of(
    control: &Raw,
    nodes: &BTreeMap<&NodeKey, &Raw>,
    visiting: &mut BTreeSet<NodeKey>,
) -> Result<ControlOwner, OccurrenceRefusal> {
    let channel_end = |end: &'static str| -> Result<ControlOwner, OccurrenceRefusal> {
        let channel = control.reference("channel")?;
        let channel = nodes
            .get(channel)
            .ok_or_else(|| OccurrenceRefusal::UnknownNode {
                node: control.key.clone(),
            })?;
        Ok(ControlOwner::Role(channel.reference(end)?.clone()))
    };
    Ok(match control.operator.as_str() {
        "choice" | "repeat" => ControlOwner::Role(control.reference("owner")?.clone()),
        "attempt" | "event" | "commit" => ControlOwner::Role(control.reference("role")?.clone()),
        "send" => channel_end("from")?,
        "receive" => channel_end("to")?,
        "effect" => {
            let attempt = control.reference("of")?;
            if !visiting.insert(attempt.clone()) {
                return Err(control.malformed("of"));
            }
            let attempt = nodes
                .get(attempt)
                .ok_or_else(|| OccurrenceRefusal::UnknownNode {
                    node: control.key.clone(),
                })?;
            ControlOwner::Role(attempt.reference("role")?.clone())
        }
        _ => ControlOwner::Structural,
    })
}

#[string_edge]
fn derive(
    nodes: &[Raw],
    map: &PackageSourceMap,
) -> Result<Vec<ProtocolOccurrence>, OccurrenceRefusal> {
    let by_key: BTreeMap<&NodeKey, &Raw> = nodes.iter().map(|node| (&node.key, node)).collect();
    let lookup = |from: &Raw, key: &NodeKey| -> Result<&Raw, OccurrenceRefusal> {
        by_key
            .get(key)
            .copied()
            .ok_or_else(|| OccurrenceRefusal::UnknownNode {
                node: from.key.clone(),
            })
    };
    let mut derived = Vec::new();
    for clause in nodes.iter().filter(|node| node.form == "protocol_clause") {
        let mut roles = Vec::new();
        for key in clause.references("roles")? {
            let role = lookup(clause, key)?;
            let replication = match role.group("replication")? {
                [] => RoleReplication::Static,
                fields => {
                    let text = |name: &'static str| match field(fields, name) {
                        Some(Term::Text(text)) => Ok(text.clone()),
                        _ => Err(role.malformed("replication")),
                    };
                    RoleReplication::Replicated {
                        population: text("population")?,
                        max: field(fields, "max")
                            .and_then(as_integer)
                            .ok_or_else(|| role.malformed("replication"))?,
                        lifetime: text("lifetime")?,
                    }
                }
            };
            roles.push(RoleSlot {
                key: role.key.clone(),
                path: role.path.clone(),
                region: declaration_region(map, &role.key),
                replication,
            });
        }
        roles.sort_by(|left, right| left.path.cmp(&right.path));

        // Walk the control tree from `run`, outermost repeat first.
        let mut controls = Vec::new();
        let mut seen = BTreeSet::new();
        let mut pending = vec![(clause.reference("run")?.clone(), Vec::new())];
        while let Some((key, dimensions)) = pending.pop() {
            if !seen.insert(key.clone()) {
                continue;
            }
            let control = lookup(clause, &key)?;
            let owner = owner_of(control, &by_key, &mut BTreeSet::new())?;
            for (child, dimension) in held(control)? {
                let mut inner = dimensions.clone();
                inner.extend(dimension);
                pending.push((child.clone(), inner));
            }
            controls.push(ControlEntry {
                region: declaration_region(map, &control.key),
                key: control.key.clone(),
                path: control.path.clone(),
                operator: control.operator.clone(),
                owner,
                dimensions,
            });
        }
        controls.sort_by(|left, right| left.path.cmp(&right.path));

        let mut compensations = Vec::new();
        for key in clause.references("compensations")? {
            let template = lookup(clause, key)?;
            let attempts = template.group("attempts")?;
            compensations.push(CompensationEntry {
                key: template.key.clone(),
                path: template.path.clone(),
                region: declaration_region(map, &template.key),
                role: template.reference("role")?.clone(),
                effect: template.reference("forward")?.clone(),
                maximum_attempts: field(attempts, "maximum")
                    .and_then(as_integer)
                    .ok_or_else(|| template.malformed("attempts"))?,
            });
        }
        compensations.sort_by(|left, right| left.path.cmp(&right.path));

        // The requirements: one record for each (kind, subject) the operators
        // of the protocol's nodes imply.
        let mut subjects: Vec<(Vec<String>, SubjectKind, NodeKey)> = Vec::new();
        let mut subject = |node: &Raw, kinds: &[SubjectKind]| {
            for kind in kinds {
                subjects.push((node.path.clone(), *kind, node.key.clone()));
            }
        };
        subject(clause, &[SubjectKind::WorkflowInstance]);
        for role in &roles {
            let replicated = role.replication != RoleReplication::Static;
            let node = lookup(clause, &role.key)?;
            subject(
                node,
                &[SubjectKind::RoleInstance {
                    per_spawned_object: replicated,
                }],
            );
        }
        for entry in &controls {
            let node = lookup(clause, &entry.key)?;
            let kinds: &[SubjectKind] = match entry.operator.as_str() {
                "send" => &[SubjectKind::Message, SubjectKind::Send],
                "receive" => &[SubjectKind::Receive, SubjectKind::Delivery],
                "attempt" => &[SubjectKind::OperationAttempt],
                "effect" => &[SubjectKind::BusinessEffect],
                "commit" => &[SubjectKind::Commit],
                _ => &[],
            };
            subject(node, kinds);
        }
        for entry in &compensations {
            let node = lookup(clause, &entry.key)?;
            subject(
                node,
                &[
                    SubjectKind::Compensation,
                    SubjectKind::CompensationAttempt,
                    SubjectKind::CompensationEffect,
                ],
            );
        }
        subject(
            lookup(clause, clause.reference("finish")?)?,
            &[SubjectKind::WorkflowClosure],
        );
        subjects.sort();
        subjects.dedup();
        let requirements = subjects
            .into_iter()
            .map(|(_, kind, node)| RuntimeBindingRequirement {
                kind,
                subject: node,
                scope: clause.key.clone(),
                authority: None,
                contract: None,
            })
            .collect();

        derived.push(ProtocolOccurrence {
            schema: ProtocolOccurrenceSchema {
                workflow: clause.key.clone(),
                roles,
                controls,
                compensations,
            },
            requirements,
        });
    }
    derived.sort_by(|left, right| left.schema.workflow.cmp(&right.schema.workflow));
    Ok(derived)
}

impl CheckedPackage {
    /// Derives the occurrence schema and runtime binding requirements of
    /// every local protocol from the in-process protocol nodes (FR-384).
    ///
    /// `source_map` is the source map emitted for this package (the
    /// emission's [`source_map`](crate::Emission::source_map)); it supplies
    /// each declaration region and nothing else. The result is ordered by
    /// protocol clause key. No source text, display string or name is read
    /// for a node's key, and no runtime value is fabricated.
    pub fn protocol_occurrence_schemas(
        &self,
        source_map: &PackageSourceMap,
    ) -> Result<Vec<ProtocolOccurrence>, OccurrenceRefusal> {
        derive(&in_process_nodes(self)?, source_map)
    }
}

/// Derives the same result from a package IR's reader admitted (FR-384-AC-3):
/// the schema and requirements equal those of the in-process package.
pub fn protocol_occurrence_schemas_from_read(
    package: &CheckedPackageV2,
) -> Result<Vec<ProtocolOccurrence>, OccurrenceRefusal> {
    let graph = wire_graph(package).map_err(|_| OccurrenceRefusal::Identity)?;
    let map = crate::checked_v2::package_source_map(&graph, package.source_map())
        .map_err(|_| OccurrenceRefusal::SourceMap)?;
    derive(&read_nodes(package)?, &map)
}
