// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-147 "The refinement's internal list": the compilation's optional
//! `refinement_internals` input, parsed once for the unit, then resolved per
//! refinement against its selected abstract protocol clause.
//!
//! A value is a JSON array of objects `{"refinement": "<name>", "internal":
//! ["<class>", ...]}`. It has no source span, so a defect is located by the
//! RFC 6901 JSON Pointer of the offending value inside it (the unit's value
//! is the only input a build supplies here; a supplied library's value
//! waits on the compile input that carries one).

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::string_edge;
use qsl_forms::{ActivationForm, ChannelDelivery, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::MemoryModelName;
use serde_json::Value;

use quire_exact::ValueType;
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};

use crate::check::protocol_clause::{BoundAttempt, CheckedProtocol, ProtocolNodeId};
use crate::check::protocol_slots::CheckedLifetime;

/// Which kind of abstract step a class is (ADR-027 ST-1 to ST-15).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AbstractStepKind {
    /// The `attempt`, `event`, `send`, `receive`, `timeout`, `finish` or
    /// `fence` of one node, or the `cattempt` of one template: a bare path.
    Node,
    /// The `fork` of one `parallel` node.
    Fork,
    /// The `join` of one `parallel` node.
    Join,
    /// The `cend` of one compensation template.
    Cend,
    /// The `duplicate` of one channel.
    Duplicate,
    /// The `lose` of one channel.
    Lose,
    /// The `spawn` of one replicated role.
    Spawn,
    /// The `retire` of one replicated role.
    Retire,
    /// The `activate` of the clause's trigger.
    Activate,
    /// One memory step kind.
    Memory,
}

impl AbstractStepKind {
    /// The leading keyword of the class's spelling, absent for a bare node
    /// path.
    fn keyword(self) -> Option<&'static str> {
        match self {
            Self::Node => None,
            Self::Fork => Some("fork"),
            Self::Join => Some("join"),
            Self::Cend => Some("cend"),
            Self::Duplicate => Some("duplicate"),
            Self::Lose => Some("lose"),
            Self::Spawn => Some("spawn"),
            Self::Retire => Some("retire"),
            Self::Activate => Some("activate"),
            Self::Memory => Some("memory"),
        }
    }

    /// The kind a leading keyword spells. `#[string_edge]`: the keyword is
    /// the wire spelling of a class in the `refinement_internals` value.
    #[string_edge]
    fn of_keyword(word: &str) -> Option<Self> {
        Some(match word {
            "fork" => Self::Fork,
            "join" => Self::Join,
            "cend" => Self::Cend,
            "duplicate" => Self::Duplicate,
            "lose" => Self::Lose,
            "spawn" => Self::Spawn,
            "retire" => Self::Retire,
            "activate" => Self::Activate,
            "memory" => Self::Memory,
            _ => return None,
        })
    }
}

/// One abstract step class a refinement's internal list makes internal,
/// named by its alias-free spelling (QSpec FR-432): the canonical spelling
/// with `<A>::` removed (`Twice::finish`, `lose Twice::ch`, `activate Twice`,
/// `memory flush`). Classes order by the UTF-8 bytes of that spelling.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AbstractStepClass {
    spelling: String,
    kind: AbstractStepKind,
}

impl AbstractStepClass {
    /// The alias-free spelling.
    pub fn spelling(&self) -> &str {
        &self.spelling
    }

    /// The kind of step.
    pub fn kind(&self) -> AbstractStepKind {
        self.kind
    }

    /// The class of a bare node path `<protocol>::<node>`, which a step
    /// row's `node <A>::<protocol>::<node>` right side can name.
    pub(crate) fn node(protocol: &str, node: &str) -> Self {
        Self {
            spelling: format!("{protocol}::{node}"),
            kind: AbstractStepKind::Node,
        }
    }
}

/// How a defect of the list is refused (QSpec FR-432's table).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ListDefectKind {
    /// `invalid_model_binding`/`malformed-declaration`: a shape or spelling
    /// defect, or a list on a model subject.
    Malformed,
    /// `missing_declaration`/`missing-name`: a refinement, alias, protocol,
    /// node, template, channel, role or memory kind that does not exist.
    Missing,
    /// `invalid_model_binding`/`conflicting-binding`: a repeated object or
    /// spelling.
    Conflicting,
}

/// A defect of the list, located by the RFC 6901 JSON Pointer of the
/// offending value inside the unit's `refinement_internals` value (FR-147).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ListDefect {
    /// How it is refused.
    pub kind: ListDefectKind,
    /// The pointer of the offending value.
    pub pointer: String,
    /// The pointer of the earlier value a repetition repeats.
    pub first: Option<String>,
}

/// The name of one `refinement_internals` value's refinement and its
/// entries.
#[derive(Clone, Debug)]
pub(crate) struct ListObject {
    /// The pointer of the object.
    pub pointer: String,
    /// The entries, well-formed ones only, in array order.
    pub entries: Vec<ListEntry>,
    /// Whether the object's `internal` array held an entry at all, whatever
    /// its spelling.
    pub non_empty: bool,
}

/// One entry of an `internal` array whose spelling is well formed.
#[derive(Clone, Debug)]
pub(crate) struct ListEntry {
    /// The pointer of the entry.
    pub pointer: String,
    spelling: Spelling,
}

/// A parsed class spelling (QSpec FR-432's table).
#[derive(Clone, Debug)]
struct Spelling {
    kind: AbstractStepKind,
    /// The abstract alias, absent for `memory <kind>`.
    alias: Option<String>,
    /// The protocol, absent for `memory <kind>`.
    protocol: Option<String>,
    /// The node, template, channel, role or memory kind; absent for
    /// `activate <A>::<protocol>`.
    member: Option<String>,
}

/// The unit's `refinement_internals` value, parsed: its well-formed objects
/// by refinement name, and every defect found without a protocol to resolve
/// against.
#[derive(Debug, Default)]
pub(crate) struct ParsedLists {
    objects: BTreeMap<String, ListObject>,
    /// Defects of the value, each with the refinement it is attributable to
    /// (`None` for a value attributable to no refinement), in the order
    /// found.
    pub defects: Vec<(Option<String>, ListDefect)>,
}

/// Whether `key` is one of the two members of a `refinement_internals`
/// object. `#[string_edge]`: the keys are the value's wire names.
#[string_edge]
fn is_object_key(key: &str) -> bool {
    key == "refinement" || key == "internal"
}

fn is_identifier(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
}

/// The spelling `text` writes, when it is one of QSpec FR-432's.
fn parse_spelling(text: &str) -> Option<Spelling> {
    let (kind, rest) = match text.split_once(' ') {
        Some((word, rest)) => (AbstractStepKind::of_keyword(word)?, rest),
        None => (AbstractStepKind::Node, text),
    };
    let parts: Vec<&str> = rest.split("::").collect();
    if !parts.iter().all(|part| is_identifier(part)) {
        return None;
    }
    match (kind, &parts[..]) {
        (AbstractStepKind::Memory, [memory]) => Some(Spelling {
            kind,
            alias: None,
            protocol: None,
            member: Some((*memory).to_owned()),
        }),
        (AbstractStepKind::Activate, [alias, protocol]) => Some(Spelling {
            kind,
            alias: Some((*alias).to_owned()),
            protocol: Some((*protocol).to_owned()),
            member: None,
        }),
        (
            AbstractStepKind::Node
            | AbstractStepKind::Fork
            | AbstractStepKind::Join
            | AbstractStepKind::Cend
            | AbstractStepKind::Duplicate
            | AbstractStepKind::Lose
            | AbstractStepKind::Spawn
            | AbstractStepKind::Retire,
            [alias, protocol, member],
        ) => Some(Spelling {
            kind,
            alias: Some((*alias).to_owned()),
            protocol: Some((*protocol).to_owned()),
            member: Some((*member).to_owned()),
        }),
        _ => None,
    }
}

impl ParsedLists {
    /// Parses `value`, the unit's `refinement_internals`, for the
    /// refinements `declared` in the compiled source (FR-147 "Shape",
    /// "Class spellings", "Defects"): the shape, the spelling of each entry,
    /// the repetition of an object or of a spelling, and that `refinement`
    /// names a declared refinement. An absent value is the empty list.
    pub(crate) fn parse(value: Option<&Value>, declared: &BTreeSet<&str>) -> Self {
        let mut parsed = Self::default();
        let Some(value) = value else {
            return parsed;
        };
        let Some(array) = value.as_array() else {
            parsed.defect(None, ListDefectKind::Malformed, String::new(), None);
            return parsed;
        };
        let mut named: BTreeMap<String, String> = BTreeMap::new();
        for (index, object) in array.iter().enumerate() {
            parsed.object(index, object, declared, &mut named);
        }
        parsed
    }

    fn defect(
        &mut self,
        refinement: Option<&str>,
        kind: ListDefectKind,
        pointer: String,
        first: Option<String>,
    ) {
        self.defects.push((
            refinement.map(str::to_owned),
            ListDefect {
                kind,
                pointer,
                first,
            },
        ));
    }

    fn object(
        &mut self,
        index: usize,
        object: &Value,
        declared: &BTreeSet<&str>,
        named: &mut BTreeMap<String, String>,
    ) {
        let pointer = format!("/{index}");
        let Some(members) = object.as_object() else {
            self.defect(None, ListDefectKind::Malformed, pointer, None);
            return;
        };
        // The refinement the object is attributable to, when `refinement`
        // is a string naming a declared one.
        let mut refinement: Option<&str> = None;
        match members.get("refinement") {
            None => {}
            Some(Value::String(name)) => {
                if declared.contains(name.as_str()) {
                    refinement = Some(name);
                } else {
                    self.defect(
                        None,
                        ListDefectKind::Missing,
                        format!("{pointer}/refinement"),
                        None,
                    );
                }
            }
            Some(_) => self.defect(
                None,
                ListDefectKind::Malformed,
                format!("{pointer}/refinement"),
                None,
            ),
        }
        // An object is closed to its two keys, which it must both hold; the
        // defect is the named refinement's when `refinement` names one.
        let closed = members.keys().all(|key| is_object_key(key));
        if !closed || members.len() < 2 {
            self.defect(refinement, ListDefectKind::Malformed, pointer.clone(), None);
        }
        let mut entries = Vec::new();
        let mut non_empty = false;
        match members.get("internal") {
            None => {}
            Some(Value::Array(array)) => {
                non_empty = !array.is_empty();
                let mut seen: BTreeMap<&str, String> = BTreeMap::new();
                for (position, entry) in array.iter().enumerate() {
                    let entry_pointer = format!("{pointer}/internal/{position}");
                    let Some(text) = entry.as_str() else {
                        self.defect(refinement, ListDefectKind::Malformed, entry_pointer, None);
                        continue;
                    };
                    let Some(spelling) = parse_spelling(text) else {
                        self.defect(refinement, ListDefectKind::Malformed, entry_pointer, None);
                        continue;
                    };
                    if let Some(first) = seen.get(text) {
                        self.defect(
                            refinement,
                            ListDefectKind::Conflicting,
                            entry_pointer,
                            Some(first.clone()),
                        );
                        continue;
                    }
                    seen.insert(text, entry_pointer.clone());
                    entries.push(ListEntry {
                        pointer: entry_pointer,
                        spelling,
                    });
                }
            }
            Some(_) => self.defect(
                refinement,
                ListDefectKind::Malformed,
                format!("{pointer}/internal"),
                None,
            ),
        }
        let Some(name) = refinement else {
            return;
        };
        if let Some(first) = named.get(name) {
            self.defect(
                Some(name),
                ListDefectKind::Conflicting,
                pointer,
                Some(first.clone()),
            );
            return;
        }
        named.insert(name.to_owned(), pointer.clone());
        self.objects.insert(
            name.to_owned(),
            ListObject {
                pointer,
                entries,
                non_empty,
            },
        );
    }

    /// The well-formed object that names `refinement`, when the value holds
    /// one.
    pub(crate) fn object_of(&self, refinement: &str) -> Option<&ListObject> {
        self.objects.get(refinement)
    }

    /// Whether a defect of the value is attributable to `refinement`, which
    /// then has no list and no `CheckedRefinement`.
    pub(crate) fn is_defective(&self, refinement: &str) -> bool {
        self.defects
            .iter()
            .any(|(owner, _)| owner.as_deref() == Some(refinement))
    }
}

/// The fields of a binder of type `binder_type`: one per field of a record
/// type, in declaration order, and otherwise the one field of that type.
fn binder_record(types: &TypeEnvironment, binder_type: &ValueType) -> Vec<ValueType> {
    if let ValueType::Composite(key) = binder_type {
        if let Some(CompositeShape::Record(fields)) =
            types.composite(*key).map(|declared| declared.shape())
        {
            return fields
                .iter()
                .map(|field| field.value_type().clone())
                .collect();
        }
    }
    vec![binder_type.clone()]
}

/// What one node declaration is, as a step class (ADR-027 ST-1 to ST-12).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NodeClass {
    /// A node with one class of its own, named by its bare path.
    Bare(ProtocolNodeKind),
    /// A `parallel` node: the classes `fork` and `join`.
    Parallel,
    /// A compensation template: the classes `cattempt` and `cend`.
    Template,
}

/// The step classes of one checked protocol clause, which an internal list
/// entry and a step row's `node` right side resolve against.
#[derive(Debug)]
pub(crate) struct ProtocolClasses {
    pub(crate) protocol: String,
    nodes: BTreeMap<String, NodeClass>,
    /// The binder record of each node that has one (QSpec FR-432 "The
    /// binder record of an abstract node"): the field types in order.
    records: BTreeMap<String, Vec<ValueType>>,
    /// Channel name to (admits `duplicate`, admits `lose`).
    channels: BTreeMap<String, (bool, bool)>,
    /// Replicated role name to whether its lifetime is `until`.
    roles: BTreeMap<String, bool>,
    activates: bool,
    memory: BTreeSet<&'static str>,
    /// The names of the protocol's own state: its binders, nodes and
    /// channels (queues), which no object row's field names.
    state_names: BTreeSet<String>,
}

impl ProtocolClasses {
    /// The classes of the clause `form`, checked as `checked`, whose
    /// attempts' operations are `attempts`, over the types `types`.
    pub(crate) fn of(
        form: &ProtocolDeclarationForm,
        checked: &CheckedProtocol,
        attempts: &[BoundAttempt],
        types: &TypeEnvironment,
    ) -> Self {
        let mut records: BTreeMap<String, Vec<ValueType>> = BTreeMap::new();
        for bound in attempts {
            let Some(declaration) = form.declarations.get(bound.declaration) else {
                continue;
            };
            let operation = &bound.operation.declaration;
            let record = std::iter::once(ValueType::Reference(bound.receiver))
                .chain(operation.parameters().iter().map(|(_, ty)| ty.clone()))
                .chain(operation.result().cloned())
                .collect();
            records.insert(declaration.name.name.clone(), record);
        }
        for (index, declaration) in form.declarations.iter().enumerate() {
            let node = ProtocolNodeId(index);
            let binder_type = match declaration.kind {
                ProtocolNodeKind::Effect
                | ProtocolNodeKind::Event
                | ProtocolNodeKind::Commit
                | ProtocolNodeKind::Finish => checked
                    .slots
                    .binders
                    .iter()
                    .find(|binder| binder.node == Some(node))
                    .map(|binder| &binder.value_type),
                ProtocolNodeKind::Send | ProtocolNodeKind::Receive => checked
                    .vias
                    .iter()
                    .find(|via| via.node == node)
                    .and_then(|via| {
                        checked
                            .channels
                            .iter()
                            .find(|channel| channel.name == via.channel)
                    })
                    .map(|channel| &channel.carries),
                _ => None,
            };
            if let Some(binder_type) = binder_type {
                records
                    .entry(declaration.name.name.clone())
                    .or_insert_with(|| binder_record(types, binder_type));
            }
        }
        let mut nodes = BTreeMap::new();
        for declaration in &form.declarations {
            let class = match declaration.kind {
                ProtocolNodeKind::Attempt
                | ProtocolNodeKind::Effect
                | ProtocolNodeKind::Event
                | ProtocolNodeKind::Commit
                | ProtocolNodeKind::Send
                | ProtocolNodeKind::Receive
                | ProtocolNodeKind::Finish
                | ProtocolNodeKind::Await
                | ProtocolNodeKind::Fence => NodeClass::Bare(declaration.kind),
                ProtocolNodeKind::Parallel => NodeClass::Parallel,
                ProtocolNodeKind::CompensateTemplate => NodeClass::Template,
                ProtocolNodeKind::Sequence
                | ProtocolNodeKind::Choice
                | ProtocolNodeKind::Repeat
                | ProtocolNodeKind::Branch
                | ProtocolNodeKind::Case
                | ProtocolNodeKind::Check => continue,
            };
            nodes.entry(declaration.name.name.clone()).or_insert(class);
        }
        let channels = checked
            .channels
            .iter()
            .map(|channel| {
                let (duplicate, lose) = match channel.delivery {
                    ChannelDelivery::Unknown => (true, true),
                    ChannelDelivery::AtMostOnce => (false, true),
                    ChannelDelivery::AtLeastOnce => (true, false),
                    ChannelDelivery::ExactlyOncePremise => (false, false),
                };
                (channel.name.clone(), (duplicate, lose))
            })
            .collect();
        let roles = checked
            .slots
            .roles
            .iter()
            .filter_map(|replicated| {
                let role = checked.roles.get(replicated.role)?;
                Some((
                    role.name.clone(),
                    matches!(replicated.lifetime, CheckedLifetime::Until(_)),
                ))
            })
            .collect();
        // One `memory <kind>` row covers a kind for every `parallel`, so the
        // kinds are the union over the source-resolved models (FR-136).
        let memory = checked
            .memory
            .parallels
            .iter()
            .filter_map(|parallel| memory_step(parallel.memory))
            .collect();
        let state_names = form
            .binders
            .iter()
            .map(|binder| binder.name.name.clone())
            .chain(form.declarations.iter().map(|node| node.name.name.clone()))
            .chain(checked.channels.iter().map(|channel| channel.name.clone()))
            .collect();
        Self {
            protocol: form.name.name.clone(),
            state_names,
            nodes,
            records,
            channels,
            roles,
            activates: matches!(form.activation, ActivationForm::Each { .. }),
            memory,
        }
    }

    /// Whether `name` is a binder, node or queue of the protocol's state.
    pub(crate) fn has_state(&self, name: &str) -> bool {
        self.state_names.contains(name)
    }

    /// The field types of the binder record of the node `name`.
    pub(crate) fn record(&self, name: &str) -> Option<&[ValueType]> {
        self.records.get(name).map(Vec::as_slice)
    }

    /// What the node `name` is, when it has a step class.
    pub(crate) fn node(&self, name: &str) -> Option<NodeClass> {
        self.nodes.get(name).copied()
    }

    /// The class `spelling` names, when the clause has it.
    fn resolve(&self, alias: &str, spelling: &Spelling) -> Option<AbstractStepClass> {
        let class = |text: String| AbstractStepClass {
            spelling: text,
            kind: spelling.kind,
        };
        let member = spelling.member.as_deref();
        if spelling.kind == AbstractStepKind::Memory {
            let kind = member.filter(|kind| self.memory.contains(kind))?;
            return Some(class(format!("memory {kind}")));
        }
        if spelling.alias.as_deref() != Some(alias)
            || spelling.protocol.as_deref() != Some(self.protocol.as_str())
        {
            return None;
        }
        let protocol = &self.protocol;
        let keyword = spelling.kind.keyword();
        let prefixed = |path: String| match keyword {
            Some(keyword) => format!("{keyword} {path}"),
            None => path,
        };
        if spelling.kind == AbstractStepKind::Activate {
            return self.activates.then(|| class(prefixed(protocol.clone())));
        }
        let member = member?;
        let resolved = match spelling.kind {
            AbstractStepKind::Node => matches!(
                self.nodes.get(member),
                Some(NodeClass::Bare(_) | NodeClass::Template)
            ),
            AbstractStepKind::Fork | AbstractStepKind::Join => {
                self.nodes.get(member) == Some(&NodeClass::Parallel)
            }
            AbstractStepKind::Cend => self.nodes.get(member) == Some(&NodeClass::Template),
            AbstractStepKind::Duplicate => self.channels.get(member).is_some_and(|c| c.0),
            AbstractStepKind::Lose => self.channels.get(member).is_some_and(|c| c.1),
            AbstractStepKind::Spawn => self.roles.contains_key(member),
            AbstractStepKind::Retire => self.roles.get(member) == Some(&true),
            AbstractStepKind::Activate | AbstractStepKind::Memory => false,
        };
        resolved.then(|| class(prefixed(format!("{protocol}::{member}"))))
    }

    /// Resolves `object`'s entries for the refinement whose abstract alias
    /// is `alias`: the classes, each with the pointer of the entry that
    /// names it, and the pointer of every entry that names none.
    pub(crate) fn resolve_list(
        &self,
        alias: &str,
        object: &ListObject,
    ) -> (Vec<(AbstractStepClass, String)>, Vec<String>) {
        let mut classes = Vec::new();
        let mut missing = Vec::new();
        for entry in &object.entries {
            match self.resolve(alias, &entry.spelling) {
                Some(class) => classes.push((class, entry.pointer.clone())),
                None => missing.push(entry.pointer.clone()),
            }
        }
        (classes, missing)
    }
}

/// The memory step kind a `parallel` under `model` adds, if any (ADR-027
/// SE-4): `tso` adds the store-buffer `flush`.
fn memory_step(model: MemoryModelName) -> Option<&'static str> {
    match model {
        MemoryModelName::Tso => Some("flush"),
        MemoryModelName::Sc | MemoryModelName::Ra => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spelling_has_one_space_after_its_keyword_and_no_other_character() {
        for good in [
            "Spec::Twice::finish",
            "fork Spec::Twice::par",
            "lose spec::Twice::ch",
            "activate Spec::Twice",
            "memory flush",
        ] {
            assert!(parse_spelling(good).is_some(), "{good}");
        }
        for bad in [
            "",
            "frob x",
            "Spec::Twice",
            " Spec::Twice::finish",
            "Spec::Twice::finish ",
            "fork  Spec::Twice::par",
            "fork Spec::Twice",
            "activate Spec::Twice::x",
            "memory flush::x",
            "Spec::Twice::fi nish",
            "Spec::Twice::1x",
        ] {
            assert!(parse_spelling(bad).is_none(), "{bad:?}");
        }
    }
}
