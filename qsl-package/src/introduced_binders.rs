// SPDX-License-Identifier: AGPL-3.0-or-later
//! The binders an expression introduces itself (FR-390 "Selection and supply
//! admission", rule 3).
//!
//! A `let`, a collection query (`map`, `filter`, `forall`, `exists`, `count`,
//! `sum`) and a `case` arm each open a scope. Within that scope the binder they
//! name hides every other binder of the same name, so a `value`/`parameter`
//! node reached under it is the introduced binder and is not a source of the
//! slot. This module walks the admitted terms with that scoping, and the slot's
//! source discovery and the reader's binder choice both use the one walk.

use std::collections::BTreeSet;

use quire_contract_model::{CheckedNodeId, CheckedSemanticNodeV2, CheckedTerm};

/// The binders in force at a point of a walk: the names `let` and collection
/// queries bound, and the nodes `case` arms bound.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Shadow {
    names: BTreeSet<String>,
    nodes: BTreeSet<CheckedNodeId>,
}

impl Shadow {
    fn with_name(&self, name: &str) -> Self {
        let mut next = self.clone();
        next.names.insert(name.to_owned());
        next
    }

    fn with_nodes(&self, nodes: impl IntoIterator<Item = CheckedNodeId>) -> Self {
        let mut next = self.clone();
        next.nodes.extend(nodes);
        next
    }

    fn hides(&self, node: &CheckedSemanticNodeV2) -> bool {
        self.nodes.contains(&node.node_id)
            || crate::reader_protocol_projection::parameter_name(&node.body)
                .is_some_and(|name| self.names.contains(name))
    }
}

/// The scopes an application opens.
enum Opening<'t> {
    /// `let`: the bound value is outside the scope, the body inside.
    Let {
        name: &'t str,
        value: &'t CheckedTerm,
        body: &'t CheckedTerm,
    },
    /// A collection query: the source is outside the scope, the body inside.
    Query {
        source: &'t CheckedTerm,
        name: &'t str,
        body: &'t CheckedTerm,
    },
    /// `case`: the scrutinee is outside, each arm's binders and body inside.
    Case(&'t [CheckedTerm]),
    /// No scope.
    None,
}

/// `#[string_edge]`: operation identities are the admitted wire's fixed
/// operation vocabulary.
#[ix_trace_rs::string_edge]
fn opening<'t>(identity: Option<&str>, arguments: &'t [CheckedTerm]) -> Opening<'t> {
    match (identity, arguments) {
        (Some("quire.op.control.let"), [CheckedTerm::Binding { name, value }, body]) => {
            Opening::Let {
                name: name.as_ref(),
                value,
                body,
            }
        }
        (
            Some(
                "quire.op.collection.map"
                | "quire.op.collection.filter"
                | "quire.op.collection.forall"
                | "quire.op.collection.exists"
                | "quire.op.collection.count"
                | "quire.op.collection.sum.integer",
            ),
            [source, CheckedTerm::Binding { name, value }],
        ) => Opening::Query {
            source,
            name: name.as_ref(),
            body: value,
        },
        (Some("quire.op.control.case"), [_, ..]) => Opening::Case(arguments),
        _ => Opening::None,
    }
}

/// What a walk reports: each parameter node reached outside every scope that
/// hides it.
pub(crate) type OnParameter<'a, 'w> = dyn FnMut(&'w CheckedSemanticNodeV2) + 'a;

/// The lookup of a node by id.
pub(crate) type Lookup<'a, 'w> = dyn Fn(&CheckedNodeId) -> Option<&'w CheckedSemanticNodeV2> + 'a;

struct Walk<'a, 'w> {
    lookup: &'a Lookup<'a, 'w>,
    on_parameter: &'a mut OnParameter<'a, 'w>,
    seen: BTreeSet<(CheckedNodeId, Shadow)>,
}

/// Walks `start` and every `expression`/`value` node it reaches, reporting
/// each parameter node that no scope of the walk hides. The walk stops at a
/// parameter and at any node that is not an expression or a value (a
/// function, a type), as the dependency closure of FR-390 rule 3 does.
pub(crate) fn reach_term<'a, 'w>(
    lookup: &'a Lookup<'a, 'w>,
    start: &CheckedTerm,
    on_parameter: &'a mut OnParameter<'a, 'w>,
) {
    let mut walk = Walk {
        lookup,
        on_parameter,
        seen: BTreeSet::new(),
    };
    walk.term(start, &Shadow::default(), &mut BTreeSet::new());
}

impl<'w> Walk<'_, 'w> {
    /// `covered` collects every node the term references, so a dependency the
    /// term does not reference can be told from one it does.
    fn term(&mut self, term: &CheckedTerm, shadow: &Shadow, covered: &mut BTreeSet<CheckedNodeId>) {
        match term {
            CheckedTerm::Reference { target } => {
                covered.insert(target.clone());
                self.node(target, shadow);
            }
            CheckedTerm::Application {
                operation,
                arguments,
                ..
            } => self.application(operation.identity(), arguments, shadow, covered),
            CheckedTerm::Aggregate { members } => {
                for member in members {
                    self.term(member, shadow, covered);
                }
            }
            CheckedTerm::Binding { value, .. } => self.term(value, shadow, covered),
            _ => {}
        }
    }

    fn application(
        &mut self,
        identity: Option<&str>,
        arguments: &[CheckedTerm],
        shadow: &Shadow,
        covered: &mut BTreeSet<CheckedNodeId>,
    ) {
        match opening(identity, arguments) {
            Opening::Let { name, value, body } => {
                self.term(value, shadow, covered);
                self.term(body, &shadow.with_name(name), covered);
            }
            Opening::Query { source, name, body } => {
                self.term(source, shadow, covered);
                self.term(body, &shadow.with_name(name), covered);
            }
            Opening::Case(arms) => {
                if let Some((scrutinee, arms)) = arms.split_first() {
                    self.term(scrutinee, shadow, covered);
                    for arm in arms {
                        self.arm(arm, shadow, covered);
                    }
                }
            }
            Opening::None => {
                for argument in arguments {
                    self.term(argument, shadow, covered);
                }
            }
        }
    }

    /// A `case` arm: `Binding{member, Aggregate[Aggregate[binders], body]}`.
    fn arm(&mut self, arm: &CheckedTerm, shadow: &Shadow, covered: &mut BTreeSet<CheckedNodeId>) {
        let CheckedTerm::Binding { value, .. } = arm else {
            return self.term(arm, shadow, covered);
        };
        let CheckedTerm::Aggregate { members } = value.as_ref() else {
            return self.term(value, shadow, covered);
        };
        let [CheckedTerm::Aggregate { members: binders }, body] = members.as_slice() else {
            return self.term(value, shadow, covered);
        };
        let mut introduced = Vec::with_capacity(binders.len());
        for binder in binders {
            if let CheckedTerm::Reference { target } = binder {
                covered.insert(target.clone());
                introduced.push(target.clone());
            }
        }
        self.term(body, &shadow.with_nodes(introduced), covered);
    }

    fn node(&mut self, id: &CheckedNodeId, shadow: &Shadow) {
        let Some(node) = (self.lookup)(id) else {
            return;
        };
        match (&*node.node_tag, &*node.semantic_form) {
            ("value", "parameter") => {
                if !shadow.hides(node) {
                    (self.on_parameter)(node);
                }
            }
            ("expression", _) | ("value", _) => {
                if !self.seen.insert((id.clone(), shadow.clone())) {
                    return;
                }
                let mut covered = BTreeSet::new();
                self.term(&node.body, shadow, &mut covered);
                // A dependency the body does not reference is still part of
                // the closure, under the scopes the node is reached in; a
                // binder the node's own scopes name is not read from outside.
                let own = own_names(&node.body);
                let outside = own
                    .iter()
                    .fold(shadow.clone(), |acc, name| acc.with_name(name));
                for dependency in &node.dependencies {
                    if !covered.contains(dependency) {
                        self.node(dependency, &outside);
                    }
                }
            }
            _ => {}
        }
    }
}

/// The names a node's own application binds.
fn own_names(body: &CheckedTerm) -> Vec<&str> {
    let CheckedTerm::Application {
        operation,
        arguments,
        ..
    } = body
    else {
        return Vec::new();
    };
    match opening(operation.identity(), arguments) {
        Opening::Let { name, .. } | Opening::Query { name, .. } => vec![name],
        Opening::Case(_) | Opening::None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;
    use serde_json::{json, Value};

    use super::*;

    fn id(name: &str) -> Value {
        json!({"ordinal": 0, "package": "p", "path": [name]})
    }

    fn node(name: &str, tag: &str, form: &str, dependencies: &[&str], body: Value) -> Value {
        json!({
            "node_id": id(name),
            "schema_version": "quire.checked-semantic-graph/v2",
            "node_tag": tag,
            "semantic_form": form,
            "semantic_type": id("type"),
            "dependencies": dependencies.iter().map(|d| id(d)).collect::<Vec<_>>(),
            "occurrences": [],
            "body": body,
        })
    }

    fn parameter(name: &str, text: &str) -> Value {
        let literal = |kind: &str, value: Value| json!({"term": "literal", "type": id("type"), "value": value, "value_kind": kind});
        node(
            name,
            "value",
            "parameter",
            &[],
            json!({"term": "aggregate", "members": [
                {"term": "binding", "name": "name", "value": literal("text", json!(text))},
                {"term": "binding", "name": "level", "value": literal("integer", json!("0"))},
            ]}),
        )
    }

    fn reference(name: &str) -> Value {
        json!({"term": "reference", "target": id(name)})
    }

    /// The parameter nodes a walk from `root` reaches outside every scope.
    fn sources(nodes: Vec<Value>, root: &str) -> Vec<String> {
        let nodes: BTreeMap<CheckedNodeId, CheckedSemanticNodeV2> = nodes
            .into_iter()
            .map(|value| {
                let node: CheckedSemanticNodeV2 = serde_json::from_value(value).expect("a node");
                (node.node_id.clone(), node)
            })
            .collect();
        let lookup = |id: &CheckedNodeId| nodes.get(id);
        let mut found = Vec::new();
        let start: CheckedTerm = serde_json::from_value(reference(root)).expect("a term");
        reach_term(&lookup, &start, &mut |node| {
            found.push(
                crate::reader_protocol_projection::parameter_name(&node.body)
                    .unwrap_or_default()
                    .to_owned(),
            );
        });
        found.sort();
        found
    }

    /// FR-390 rule 3: the binders of a `case` arm are not sources. The package
    /// reader admits no arm binder in a protocol slot today, so this walks the
    /// term directly. The scrutinee `s` and the body's read of `outer` stay
    /// sources.
    #[trace("FR-390-AC-68", "TC-1071")]
    #[test]
    fn a_case_arm_binder_is_not_a_source() {
        let arm = json!({"term": "binding", "name": "Circle", "value": {
            "term": "aggregate",
            "members": [
                {"term": "aggregate", "members": [reference("r")]},
                {"term": "aggregate", "members": [reference("r"), reference("outer")]},
            ],
        }});
        let case = node(
            "case",
            "expression",
            "case",
            &["s", "r", "outer"],
            json!({
                "term": "application",
                "operator": "case",
                "operation": {"identity": "quire.op.control.case", "laws": [], "leaves": [], "member": null, "mode": null},
                "arguments": [reference("s"), arm],
                "result_type": id("type"),
            }),
        );
        let found = sources(
            vec![
                case,
                parameter("s", "s"),
                parameter("r", "r"),
                parameter("outer", "outer"),
            ],
            "case",
        );
        assert_eq!(found, vec!["outer", "s"]);
    }
}
