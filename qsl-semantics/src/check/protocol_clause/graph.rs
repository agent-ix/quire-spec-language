//! FR-218: a protocol's checked control tree and its compiled causal edge
//! relation, with the two structural rules read off them: the relation
//! without the `repeat` back edges is acyclic, and every `repeat` body
//! necessarily takes a step (QSpec FR-464 "Derived causal relation" and
//! "Progress", QSpec FR-052).
//!
//! The tree is read from the S2 forms alone: each control declares its
//! children in the scope its own name opens, and the parts of a `repeat`
//! and an `await` are told apart by the spans S2 records for them. Every
//! walk keeps its own explicit stack, so a control tree of any depth is
//! compiled without recursion.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{AnchorSite, JoinPolicy, ParallelForm, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::Span;

use super::refusal;
use super::types::{
    CausalEdge, CheckedControl, CheckedControlTree, EdgeKind, Port, PortRef, ProtocolNodeId,
};
use crate::check::refusal::{CheckCause, CheckRefusal, KeyFault, ProtocolAnchorCause};

/// What [`compile`] builds: the control tree and its edge relation.
pub(super) struct Compiled {
    pub(super) tree: CheckedControlTree,
    pub(super) edges: Vec<CausalEdge>,
}

/// Whether a node of `kind` is a leaf control: entered before it exits,
/// with no child control.
fn is_leaf(kind: ProtocolNodeKind) -> bool {
    use ProtocolNodeKind as K;
    match kind {
        K::Send
        | K::Receive
        | K::Attempt
        | K::Effect
        | K::Event
        | K::Commit
        | K::Check
        | K::Fence => true,
        K::Sequence
        | K::Choice
        | K::Parallel
        | K::Repeat
        | K::Branch
        | K::Case
        | K::Await
        | K::Finish
        | K::CompensateTemplate => false,
    }
}

fn fault(invariant: &'static str) -> Vec<(Span, CheckRefusal)> {
    vec![(
        Span { start: 0, end: 0 },
        CheckRefusal {
            location: quire_semantic_value::location::Location::root(
                quire_semantic_value::location::Origin::Expression,
            ),
            cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                qsl_foundation::InternalFault::new("check.protocol", invariant),
            ))),
        },
    )]
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn enter(node: usize) -> PortRef {
    PortRef {
        node: ProtocolNodeId(node),
        port: Port::Enter,
    }
}

fn exit(node: usize) -> PortRef {
    PortRef {
        node: ProtocolNodeId(node),
        port: Port::Exit,
    }
}

/// The direct children of every declaration, in authored order, read from
/// the scope each control's own name opens.
fn children_of(protocol: &ProtocolDeclarationForm) -> BTreeMap<usize, Vec<usize>> {
    let mut opened: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for (index, declaration) in protocol.declarations.iter().enumerate() {
        opened.insert(
            (declaration.name.span.start, declaration.name.span.end),
            index,
        );
    }
    let owners: Vec<Option<usize>> = protocol
        .scopes
        .iter()
        .map(|entry| {
            opened
                .get(&(entry.name.span.start, entry.name.span.end))
                .copied()
        })
        .collect();
    let mut children: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, declaration) in protocol.declarations.iter().enumerate() {
        let Some(parent) = declaration
            .scope
            .and_then(|scope| owners.get(scope.index()).copied().flatten())
        else {
            continue;
        };
        children.entry(parent).or_default().push(index);
    }
    for list in children.values_mut() {
        list.sort_by_key(|&index| protocol.declarations[index].name.span.start);
    }
    children
}

/// The edges a node's own rule adds, given its children.
struct Builder<'p> {
    protocol: &'p ProtocolDeclarationForm,
    children: BTreeMap<usize, Vec<usize>>,
    edges: Vec<CausalEdge>,
}

impl Builder<'_> {
    fn edge(&mut self, kind: EdgeKind, from: PortRef, to: PortRef) {
        self.edges.push(CausalEdge { kind, from, to });
    }

    fn children(&self, node: usize) -> &[usize] {
        self.children.get(&node).map_or(&[][..], Vec::as_slice)
    }

    /// The one control a `branch` or `case` holds.
    fn body(&self, label: usize) -> Option<usize> {
        match self.children(label) {
            [body] => Some(*body),
            _ => None,
        }
    }

    /// The child of `node` whose name lies inside `span`.
    fn child_within(&self, node: usize, span: Span) -> Option<usize> {
        self.children(node)
            .iter()
            .copied()
            .find(|&child| contains(span, self.protocol.declarations[child].name.span))
    }

    /// The structural edges of `node` (QSpec FR-464 table).
    fn structural(&mut self, node: usize) -> Result<(), &'static str> {
        let protocol = self.protocol;
        let kind = protocol.declarations[node].kind;
        match kind {
            ProtocolNodeKind::Sequence => {
                let children = self.children(node).to_vec();
                match (children.first(), children.last()) {
                    (Some(&first), Some(&last)) => {
                        self.edge(EdgeKind::Sequence, enter(node), enter(first));
                        for pair in children.windows(2) {
                            self.edge(EdgeKind::Sequence, exit(pair[0]), enter(pair[1]));
                        }
                        self.edge(EdgeKind::Sequence, exit(last), exit(node));
                    }
                    _ => self.edge(EdgeKind::Sequence, enter(node), exit(node)),
                }
            }
            ProtocolNodeKind::Choice => {
                for case in self.children(node).to_vec() {
                    let body = self.body(case).ok_or("protocol-case-body")?;
                    self.edge(EdgeKind::Branch, enter(node), enter(body));
                    self.edge(EdgeKind::Branch, exit(body), exit(node));
                }
            }
            ProtocolNodeKind::Parallel => {
                let form = protocol
                    .parallels
                    .iter()
                    .find(|parallel| parallel.declaration == node)
                    .ok_or("protocol-parallel-form")?;
                let joined: BTreeSet<&str> = form
                    .labels
                    .iter()
                    .map(|label| label.name.as_str())
                    .collect();
                for branch in self.children(node).to_vec() {
                    let body = self.body(branch).ok_or("protocol-branch-body")?;
                    self.edge(EdgeKind::Branch, enter(node), enter(body));
                    if joined.contains(protocol.declarations[branch].name.name.as_str()) {
                        self.edge(EdgeKind::Join, exit(body), exit(node));
                    }
                }
            }
            ProtocolNodeKind::Repeat => {
                let form = protocol
                    .repeats
                    .iter()
                    .find(|repeat| repeat.declaration == node)
                    .ok_or("protocol-repeat-form")?;
                let body = self
                    .child_within(node, form.body)
                    .ok_or("protocol-repeat-body")?;
                self.edge(EdgeKind::RepeatProgress, enter(node), enter(body));
                self.edge(EdgeKind::RepeatProgress, exit(body), exit(node));
                if let Some(span) = form.exhausted {
                    let exhausted = self
                        .child_within(node, span)
                        .ok_or("protocol-repeat-exhausted")?;
                    self.edge(EdgeKind::RepeatProgress, enter(node), enter(exhausted));
                    self.edge(EdgeKind::RepeatProgress, exit(exhausted), exit(node));
                }
                self.edge(EdgeKind::RepeatBack, exit(body), enter(body));
            }
            ProtocolNodeKind::Await => {
                let form = protocol
                    .awaits
                    .iter()
                    .find(|await_form| await_form.declaration == node)
                    .ok_or("protocol-await-form")?;
                let event = self
                    .children(node)
                    .iter()
                    .copied()
                    .find(|&child| protocol.declarations[child].name.span == form.matched.span)
                    .ok_or("protocol-await-event")?;
                let then = self
                    .child_within(node, form.then)
                    .ok_or("protocol-await-then")?;
                let timeout = self
                    .child_within(node, form.timeout)
                    .ok_or("protocol-await-timeout")?;
                self.edge(EdgeKind::AwaitSuccess, enter(node), enter(event));
                self.edge(EdgeKind::AwaitSuccess, exit(event), enter(then));
                self.edge(EdgeKind::AwaitSuccess, exit(then), exit(node));
                self.edge(EdgeKind::AwaitTimeout, enter(node), enter(timeout));
                self.edge(EdgeKind::AwaitTimeout, exit(timeout), exit(node));
            }
            ProtocolNodeKind::Send
            | ProtocolNodeKind::Receive
            | ProtocolNodeKind::Attempt
            | ProtocolNodeKind::Effect
            | ProtocolNodeKind::Event
            | ProtocolNodeKind::Commit
            | ProtocolNodeKind::Check
            | ProtocolNodeKind::Fence
            | ProtocolNodeKind::Branch
            | ProtocolNodeKind::Case
            | ProtocolNodeKind::Finish
            | ProtocolNodeKind::CompensateTemplate => {}
        }
        Ok(())
    }
}

/// The checked control tree of `protocol`'s `run` control, its compiled
/// edge relation (`resolved` is each scoped anchor's resolved target, in
/// the order of `protocol.scoped_anchors`) and the refusals of the two
/// structural rules: a cycle without the `repeat` back edges, and a
/// `repeat` whose body does not necessarily step. `profile` is the
/// selected protocol profile's identity.
pub(super) fn compile(
    protocol: &ProtocolDeclarationForm,
    resolved: &[ProtocolNodeId],
    profile: Option<&str>,
) -> Result<Compiled, Vec<(Span, CheckRefusal)>> {
    let mut roots = protocol
        .declarations
        .iter()
        .enumerate()
        .filter(|(_, declaration)| {
            declaration.scope.is_none()
                && !matches!(
                    declaration.kind,
                    ProtocolNodeKind::Finish | ProtocolNodeKind::CompensateTemplate
                )
        })
        .map(|(index, _)| index);
    let (Some(root), None) = (roots.next(), roots.next()) else {
        return Err(fault("protocol-one-run-root"));
    };
    let mut builder = Builder {
        protocol,
        children: children_of(protocol),
        edges: Vec::new(),
    };

    // Tree order: each node before its children, children in authored
    // order, on an explicit stack.
    let mut controls: Vec<CheckedControl> = Vec::new();
    let mut stack: Vec<(usize, Vec<String>)> = vec![(root, Vec::new())];
    while let Some((node, mut path)) = stack.pop() {
        let declaration = &protocol.declarations[node];
        path.push(declaration.name.name.clone());
        let children = builder.children(node).to_vec();
        for &child in children.iter().rev() {
            stack.push((child, path.clone()));
        }
        controls.push(CheckedControl {
            node: ProtocolNodeId(node),
            kind: declaration.kind,
            name: declaration.name.name.clone(),
            path,
            children: children.into_iter().map(ProtocolNodeId).collect(),
        });
    }
    for control in &controls {
        if let Err(invariant) = builder.structural(control.node.0) {
            return Err(fault(invariant));
        }
    }

    // The cross-reference edges.
    for (anchor, target) in protocol.scoped_anchors.iter().zip(resolved) {
        let owner = anchor.owner;
        match anchor.site {
            AnchorSite::ReceiveOf => builder.edge(EdgeKind::Message, exit(target.0), enter(owner)),
            AnchorSite::EffectOf => builder.edge(EdgeKind::Effect, exit(target.0), enter(owner)),
            AnchorSite::AwaitAfter => {
                let template =
                    protocol.declarations[target.0].kind == ProtocolNodeKind::CompensateTemplate;
                if template {
                    // The compensation's forward effect: the target of the
                    // template's own `for` anchor.
                    let effect = protocol
                        .scoped_anchors
                        .iter()
                        .zip(resolved)
                        .find(|(other, _)| {
                            other.site == AnchorSite::CompensateFor && other.owner == target.0
                        })
                        .map(|(_, effect)| *effect);
                    let Some(effect) = effect else {
                        return Err(fault("protocol-template-effect"));
                    };
                    builder.edge(EdgeKind::AwaitAnchor, enter(effect.0), enter(owner));
                } else {
                    builder.edge(EdgeKind::AwaitAnchor, exit(target.0), enter(owner));
                }
            }
            AnchorSite::EventFor | AnchorSite::CompensateFor | AnchorSite::CompensateCommit => {}
        }
    }

    let tree = CheckedControlTree { controls };
    let mut refusals = cycles(protocol, &tree, &builder.edges);
    refusals.extend(progress(protocol, &tree, profile));
    if refusals.is_empty() {
        Ok(Compiled {
            tree,
            edges: builder.edges,
        })
    } else {
        Err(refusals)
    }
}

/// The ports of the tree as dense indices, with the adjacency of `edges`
/// (those `keep` admits) and every leaf's enter-before-exit edge.
struct PortGraph {
    ports: Vec<PortRef>,
    next: Vec<Vec<usize>>,
}

impl PortGraph {
    fn new(
        tree: &CheckedControlTree,
        edges: &[CausalEdge],
        keep: impl Fn(&CausalEdge) -> bool,
    ) -> Self {
        let mut index = BTreeMap::new();
        let mut ports = Vec::new();
        for control in &tree.controls {
            for port in [Port::Enter, Port::Exit] {
                let port = PortRef {
                    node: control.node,
                    port,
                };
                index.insert(port, ports.len());
                ports.push(port);
            }
        }
        let mut next = vec![Vec::new(); ports.len()];
        let mut link = |from: PortRef, to: PortRef| {
            if let (Some(&from), Some(&to)) = (index.get(&from), index.get(&to)) {
                next[from].push(to);
            }
        };
        for control in &tree.controls {
            if is_leaf(control.kind) {
                link(
                    PortRef {
                        node: control.node,
                        port: Port::Enter,
                    },
                    PortRef {
                        node: control.node,
                        port: Port::Exit,
                    },
                );
            }
        }
        for edge in edges.iter().filter(|edge| keep(edge)) {
            link(edge.from, edge.to);
        }
        Self { ports, next }
    }

    /// The strongly connected components of more than one port, by
    /// Tarjan's algorithm on an explicit stack.
    fn cycles(&self) -> Vec<Vec<usize>> {
        let count = self.ports.len();
        let mut order = vec![usize::MAX; count];
        let mut low = vec![0usize; count];
        let mut on_stack = vec![false; count];
        let mut stack: Vec<usize> = Vec::new();
        let mut components = Vec::new();
        let mut counter = 0usize;
        for start in 0..count {
            if order[start] != usize::MAX {
                continue;
            }
            // (port, index of the next successor to visit)
            let mut work: Vec<(usize, usize)> = vec![(start, 0)];
            order[start] = counter;
            low[start] = counter;
            counter += 1;
            stack.push(start);
            on_stack[start] = true;
            while let Some(top) = work.last_mut() {
                let port = top.0;
                if let Some(&to) = self.next[port].get(top.1) {
                    top.1 += 1;
                    if order[to] == usize::MAX {
                        order[to] = counter;
                        low[to] = counter;
                        counter += 1;
                        stack.push(to);
                        on_stack[to] = true;
                        work.push((to, 0));
                    } else if on_stack[to] {
                        low[port] = low[port].min(order[to]);
                    }
                    continue;
                }
                work.pop();
                if let Some(&(parent, _)) = work.last() {
                    low[parent] = low[parent].min(low[port]);
                }
                if low[port] == order[port] {
                    let mut component = Vec::new();
                    while let Some(member) = stack.pop() {
                        on_stack[member] = false;
                        component.push(member);
                        if member == port {
                            break;
                        }
                    }
                    if component.len() > 1 {
                        components.push(component);
                    }
                }
            }
        }
        components
    }
}

/// One `definition-cycle` refusal per cycle of the relation without the
/// `repeat` back edges, naming its controls in tree order.
fn cycles(
    protocol: &ProtocolDeclarationForm,
    tree: &CheckedControlTree,
    edges: &[CausalEdge],
) -> Vec<(Span, CheckRefusal)> {
    let graph = PortGraph::new(tree, edges, |edge| edge.kind != EdgeKind::RepeatBack);
    let position: BTreeMap<ProtocolNodeId, usize> = tree
        .controls
        .iter()
        .enumerate()
        .map(|(position, control)| (control.node, position))
        .collect();
    let mut refusals = Vec::new();
    for component in graph.cycles() {
        let mut nodes: Vec<ProtocolNodeId> = component
            .iter()
            .map(|&port| graph.ports[port].node)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        nodes.sort_by_key(|node| position.get(node).copied().unwrap_or(usize::MAX));
        let Some(first) = nodes.first() else {
            continue;
        };
        let span = protocol.declarations[first.0].name.span;
        refusals.push((
            span,
            refusal(ProtocolAnchorCause::CausalCycle {
                nodes: nodes
                    .iter()
                    .map(|node| protocol.declarations[node.0].name.name.clone())
                    .collect(),
                span,
            }),
        ));
    }
    refusals
}

/// One `declaration-form` refusal per `repeat` whose body does not
/// necessarily step (QSpec FR-464 "Progress").
fn progress(
    protocol: &ProtocolDeclarationForm,
    tree: &CheckedControlTree,
    profile: Option<&str>,
) -> Vec<(Span, CheckRefusal)> {
    // Children follow their parent in tree order, so a reverse walk sees
    // every child before its parent.
    let mut steps: BTreeMap<ProtocolNodeId, bool> = BTreeMap::new();
    for control in tree.controls.iter().rev() {
        let child = |index: usize| {
            control
                .children
                .get(index)
                .and_then(|child| steps.get(child))
                .copied()
                .unwrap_or(false)
        };
        use ProtocolNodeKind as K;
        let necessarily = match control.kind {
            K::Send | K::Receive | K::Attempt | K::Effect | K::Event | K::Commit | K::Fence => true,
            K::Await | K::Parallel => true,
            K::Check | K::Repeat | K::Finish | K::CompensateTemplate => false,
            K::Branch | K::Case => child(0),
            K::Sequence => control
                .children
                .iter()
                .any(|child| steps.get(child).copied().unwrap_or(false)),
            K::Choice => control
                .children
                .iter()
                .all(|child| steps.get(child).copied().unwrap_or(false)),
        };
        steps.insert(control.node, necessarily);
    }
    let mut refusals = Vec::new();
    for control in &tree.controls {
        if control.kind != ProtocolNodeKind::Repeat {
            continue;
        }
        let Some(form) = protocol
            .repeats
            .iter()
            .find(|repeat| repeat.declaration == control.node.0)
        else {
            continue;
        };
        let body = control
            .children
            .iter()
            .copied()
            .find(|child| contains(form.body, protocol.declarations[child.0].name.span));
        if body.and_then(|body| steps.get(&body).copied()) == Some(true) {
            continue;
        }
        let span = form.name.span;
        refusals.push((
            span,
            refusal(ProtocolAnchorCause::NoProgress {
                repeat: form.name.name.clone(),
                profile: profile.map(str::to_owned),
                span,
            }),
        ));
    }
    refusals
}

/// The labels of `parallel`'s branches its join requires: those in every
/// set of completed branches that satisfies the policy (FR-381 "Retry,
/// commit and recovery"). `join all` requires every branch it lists; a
/// `quorum(n)` whose `n` is the number of distinct labels it lists, and a
/// `join any` listing one label, require each of their labels; any other
/// `join any` or `quorum(n)`, and every `join predicate`, require none.
fn required_labels(parallel: &ParallelForm) -> BTreeSet<&str> {
    let listed: BTreeSet<&str> = parallel
        .labels
        .iter()
        .map(|label| label.name.as_str())
        .collect();
    let all = match &parallel.join.value {
        JoinPolicy::All => true,
        JoinPolicy::Quorum(count) => usize::try_from(*count).is_ok_and(|n| n == listed.len()),
        JoinPolicy::Any => listed.len() == 1,
        JoinPolicy::Predicate(_) => false,
    };
    if all {
        listed
    } else {
        BTreeSet::new()
    }
}

/// Whether `commit` dominates `effect` (FR-381 "Retry, commit and
/// recovery"), decided on the control tree only, which is conservative and
/// sound: some `sequence` holds, in order, a child that passes the commit
/// and a later child that contains the effect. A control passes the commit
/// when it is the commit, when it is a `sequence` (or the `branch` holding a
/// body) with a child that passes it, or when it is a `parallel` whose join
/// requires a branch that passes it; a `choice`, an `await` and a `repeat`
/// pass none. The causal edge relation does not decide dominance: a commit
/// in one branch of a `parallel` that a `send`/`receive` orders before an
/// effect in a sibling branch is admitted (a later extension, STD-189).
pub(crate) fn dominates(
    protocol: &ProtocolDeclarationForm,
    tree: &CheckedControlTree,
    commit: ProtocolNodeId,
    effect: ProtocolNodeId,
) -> bool {
    use ProtocolNodeKind as K;
    let mut passes: BTreeMap<ProtocolNodeId, bool> = BTreeMap::new();
    // Children follow their parent in tree order, so the reverse order
    // decides every child before its parent.
    for control in tree.controls.iter().rev() {
        let child = |node: &ProtocolNodeId| passes.get(node).copied().unwrap_or(false);
        let passed = match control.kind {
            K::Sequence | K::Branch | K::Case => control.children.iter().any(child),
            K::Parallel => {
                let required = protocol
                    .parallels
                    .iter()
                    .find(|parallel| parallel.declaration == control.node.0)
                    .map(required_labels)
                    .unwrap_or_default();
                control.children.iter().any(|branch| {
                    protocol
                        .declarations
                        .get(branch.0)
                        .is_some_and(|declaration| {
                            required.contains(declaration.name.name.as_str())
                        })
                        && child(branch)
                })
            }
            // A `choice` takes one case, an `await` one of its two paths,
            // and a `repeat` may be skipped: none passes a commit.
            K::Choice | K::Await | K::Repeat => false,
            K::Send
            | K::Receive
            | K::Attempt
            | K::Effect
            | K::Event
            | K::Commit
            | K::Check
            | K::Fence
            | K::Finish
            | K::CompensateTemplate => control.node == commit,
        };
        passes.insert(control.node, passed);
    }

    let mut parent: BTreeMap<ProtocolNodeId, &CheckedControl> = BTreeMap::new();
    for control in &tree.controls {
        for child in &control.children {
            parent.insert(*child, control);
        }
    }
    let mut current = effect;
    // Each step moves to a strict ancestor, so the walk ends within the
    // tree's size; the bound guards a malformed parent map.
    for _ in 0..tree.controls.len() {
        let Some(holder) = parent.get(&current) else {
            return false;
        };
        if holder.kind == K::Sequence {
            let earlier = holder
                .children
                .iter()
                .take_while(|child| **child != current);
            if earlier
                .into_iter()
                .any(|child| passes.get(child).copied().unwrap_or(false))
            {
                return true;
            }
        }
        current = holder.node;
    }
    false
}
