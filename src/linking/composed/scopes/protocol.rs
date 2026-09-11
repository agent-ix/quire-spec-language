// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036: typed protocol symbols, lexical paths and compensation environments.
mod flow;
use super::*;
use std::collections::BTreeSet;

struct Names {
    children: BTreeMap<Option<SymbolId>, BTreeMap<String, Vec<SymbolId>>>,
    controls: BTreeMap<usize, SymbolId>,
    branches: BTreeMap<(usize, usize), SymbolId>,
    compensation: Vec<SymbolId>,
    preceding: BTreeMap<usize, Vec<(usize, SymbolId)>>,
    channels: BTreeMap<usize, SymbolId>,
    receives: Vec<(usize, usize, SymbolId)>,
    records: BTreeMap<usize, BinderId>,
}
impl Names {
    fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            controls: BTreeMap::new(),
            branches: BTreeMap::new(),
            compensation: Vec::new(),
            preceding: BTreeMap::new(),
            channels: BTreeMap::new(),
            receives: Vec::new(),
            records: BTreeMap::new(),
        }
    }
}

impl Resolver<'_, '_> {
    pub(super) fn protocol(&mut self, protocol: &c::Protocol) -> Result<(), Exhaustion> {
        let mut names = Names::new();
        self.collect_symbols(protocol, &mut names)?;
        self.structural_references(protocol, &mut names)?;
        let mut env = Environment::new(Anchor::ProtocolInstant);
        let input = self.parameter(&protocol.input, BinderKind::Input, Anchor::ProtocolInstant)?;
        env = self.extend(env, input)?;
        env = self.activation(&protocol.activation, env, &protocol.captures)?;
        for (index, channel) in protocol.channels.iter().enumerate() {
            self.work.charge(Dimension::References, 1)?;
            if let c::Ordering::Fifo { parameter, key, .. } = &channel.ordering {
                let anchor = Anchor::Fifo(index);
                let binder = self.parameter(parameter, BinderKind::Fifo, anchor)?;
                let isolated = self.extend(Environment::new(anchor), binder)?;
                self.expression(*key, isolated)?;
            }
        }
        for (index, requirement) in protocol.requirements.iter().enumerate() {
            self.work.charge(Dimension::References, 1)?;
            match requirement {
                c::ProtocolRequirement::Temporal { .. } => {}
                c::ProtocolRequirement::Compensation(compensation) => {
                    self.compensation(compensation, index, env)?
                }
            }
        }
        let completed = self.flow(protocol.run, env, &mut names)?;
        let finish = self.parameter(
            &protocol.finish.parameter,
            BinderKind::Finish,
            Anchor::Finish,
        )?;
        let finished = self.extend(
            Environment {
                anchor: Anchor::Finish,
                ..completed
            },
            finish,
        )?;
        self.expression(protocol.finish.constraint, finished)
    }

    fn compensation(
        &mut self,
        compensation: &c::Compensation,
        index: usize,
        env: Environment,
    ) -> Result<(), Exhaustion> {
        let registered = Environment {
            anchor: Anchor::Registration(index),
            ..env
        };
        let forward = self.parameter(
            &compensation.forward,
            BinderKind::ForwardEffect,
            registered.anchor,
        )?;
        let forward_env = self.extend(registered, forward)?;
        let captures = self.captures(
            &compensation.registration_captures,
            forward_env,
            registered.anchor,
        )?;
        let registration = self.export_frames(
            captures,
            registered,
            registered,
            Some(forward),
            compensation.span,
        )?;
        let active = Environment {
            anchor: Anchor::CompensationActivation(index),
            ..registration
        };
        let trigger = self.parameter(
            &compensation.trigger,
            BinderKind::CompensationTrigger,
            active.anchor,
        )?;
        let trigger_env = self.extend(active, trigger)?;
        self.expression(compensation.guard, trigger_env)?;
        let captured = self.captures(
            &compensation.activation_captures,
            trigger_env,
            active.anchor,
        )?;
        let captured =
            self.export_frames(captured, active, active, Some(trigger), compensation.span)?;
        let retry = Environment {
            anchor: Anchor::Retry(index),
            ..captured
        };
        let earlier = self.parameter(
            &compensation.earlier,
            BinderKind::EarlierAttempt,
            retry.anchor,
        )?;
        let later = self.parameter(&compensation.later, BinderKind::LaterAttempt, retry.anchor)?;
        let retry_env = self.extend(retry, earlier)?;
        let retry_env = self.extend(retry_env, later)?;
        self.expression(compensation.retry, retry_env)?;
        let recovery = Environment {
            anchor: Anchor::Recovery(index),
            ..captured
        };
        let binder = self.parameter(
            &compensation.recovery,
            BinderKind::Recovery,
            recovery.anchor,
        )?;
        let recovery = self.extend(recovery, binder)?;
        self.expression(compensation.recover, recovery)
    }

    fn symbol(
        &mut self,
        names: &mut Names,
        name: &Spanned<String>,
        kind: SymbolKind,
        parent: Option<SymbolId>,
        control: Option<c::ControlId>,
        model: Option<&QualifiedName>,
    ) -> Result<SymbolId, Exhaustion> {
        self.work.charge(Dimension::Bindings, 1)?;
        self.work.charge(Dimension::References, 1)?;
        let id = SymbolId(self.output.symbols.len());
        if let Some(previous) = names
            .children
            .get(&parent)
            .and_then(|children| children.get(name.value.as_str()))
            .and_then(|ids| ids.first())
            .copied()
        {
            self.issue(ScopeIssue::DuplicateSymbol {
                symbol: id,
                previous,
            })?;
        }
        names
            .children
            .entry(parent)
            .or_default()
            .entry(name.value.clone())
            .or_default()
            .push(id);
        self.output.symbols.push(Symbol {
            name: name.clone(),
            kind,
            parent,
            control,
            model: model.cloned(),
        });
        Ok(id)
    }

    fn collect_symbols(
        &mut self,
        protocol: &c::Protocol,
        names: &mut Names,
    ) -> Result<(), Exhaustion> {
        for role in &protocol.roles {
            self.symbol(
                names,
                &role.name,
                SymbolKind::Role,
                None,
                None,
                Some(&role.model),
            )?;
        }
        for relation in &protocol.relationships {
            self.symbol(
                names,
                &relation.name,
                SymbolKind::Relationship,
                None,
                None,
                Some(&relation.model),
            )?;
        }
        for channel in &protocol.channels {
            self.symbol(
                names,
                &channel.name,
                SymbolKind::Channel,
                None,
                None,
                Some(&channel.carries),
            )?;
        }
        for requirement in &protocol.requirements {
            self.work.charge(Dimension::References, 1)?;
            if let c::ProtocolRequirement::Compensation(compensation) = requirement {
                let id = self.symbol(
                    names,
                    &compensation.name,
                    SymbolKind::Compensation,
                    None,
                    None,
                    None,
                )?;
                names.compensation.push(id);
            }
        }
        let mut pending = vec![(protocol.run, None)];
        while let Some((id, parent)) = pending.pop() {
            self.work.charge(Dimension::References, 1)?;
            let control = self.unit.control(id).expect("parser control handle");
            let symbol = self.symbol(
                names,
                &control.name,
                control_kind(&control.kind),
                parent,
                Some(id),
                None,
            )?;
            names.controls.insert(id.0, symbol);
            match &control.kind {
                c::ControlKind::Sequence(children) => {
                    for child in children.iter().rev() {
                        self.control_child(&mut pending, *child, symbol)?;
                    }
                }
                c::ControlKind::Choice { cases, .. } => {
                    for case in cases.iter().rev() {
                        let scope = self.symbol(
                            names,
                            &case.name,
                            SymbolKind::Case,
                            Some(symbol),
                            None,
                            None,
                        )?;
                        self.control_child(&mut pending, case.control, scope)?;
                    }
                }
                c::ControlKind::Parallel { branches, .. } => {
                    for (index, branch) in branches.iter().enumerate().rev() {
                        let scope = self.symbol(
                            names,
                            &branch.name,
                            SymbolKind::Branch,
                            Some(symbol),
                            None,
                            None,
                        )?;
                        names.branches.insert((id.0, index), scope);
                        self.control_child(&mut pending, branch.control, scope)?;
                    }
                }
                c::ControlKind::Repeat {
                    body, exhausted, ..
                } => {
                    self.control_child(&mut pending, *exhausted, symbol)?;
                    self.control_child(&mut pending, *body, symbol)?;
                }
                c::ControlKind::Await {
                    event,
                    then,
                    timeout,
                    ..
                } => {
                    self.control_child(&mut pending, *timeout, symbol)?;
                    self.control_child(&mut pending, *then, symbol)?;
                    self.control_child(&mut pending, *event, symbol)?;
                }
                c::ControlKind::Event(_)
                | c::ControlKind::Check { .. }
                | c::ControlKind::Commit { .. } => {}
            }
        }
        self.symbol(
            names,
            &protocol.finish.name,
            SymbolKind::Finish,
            None,
            None,
            None,
        )?;
        Ok(())
    }
    fn control_child(
        &mut self,
        pending: &mut Vec<(c::ControlId, Option<SymbolId>)>,
        child: c::ControlId,
        parent: SymbolId,
    ) -> Result<(), Exhaustion> {
        self.work.charge(Dimension::Edges, 1)?;
        pending.push((child, Some(parent)));
        Ok(())
    }

    fn structural(
        &mut self,
        names: &Names,
        parent: Option<SymbolId>,
        site: Option<c::ControlId>,
        path: &[Spanned<String>],
        span: Span,
        required: StructuralKind,
    ) -> Result<Option<(usize, SymbolId)>, Exhaustion> {
        self.work.charge(Dimension::References, 1)?;
        self.work.charge(Dimension::Bindings, 1)?;
        let reference = self.output.references.len();
        let mut target = None;
        let mut ambiguous = false;
        let mut scope = parent;
        loop {
            self.work.charge(Dimension::Edges, 1)?;
            if let Some(candidates) = names
                .children
                .get(&scope)
                .and_then(|children| children.get(path[0].value.as_str()))
            {
                match candidates.as_slice() {
                    [id] => target = Some(*id),
                    _ => ambiguous = true,
                }
                break;
            }
            match scope {
                Some(id) => scope = self.output.symbols[id.0].parent,
                None => break,
            }
        }
        for member in &path[1..] {
            if let Some(parent) = target {
                self.work.charge(Dimension::Edges, 1)?;
                match names
                    .children
                    .get(&Some(parent))
                    .and_then(|children| children.get(member.value.as_str()))
                    .map(Vec::as_slice)
                {
                    Some([id]) => target = Some(*id),
                    None => target = None,
                    Some(_) => {
                        target = None;
                        ambiguous = true;
                    }
                }
            } else {
                break;
            }
        }
        self.output.references.push(StructuralReference {
            site,
            span,
            path: path.to_vec(),
            required,
            target,
        });
        if ambiguous {
            self.issue(ScopeIssue::AmbiguousTarget { reference })?;
        } else if let Some(target) = target {
            if required.accepts(self.output.symbols[target.0].kind) {
                return Ok(Some((reference, target)));
            }
            self.issue(ScopeIssue::WrongTargetKind { reference, target })?;
        } else {
            self.issue(ScopeIssue::MissingTarget { reference })?;
        }
        Ok(None)
    }

    fn local(
        &mut self,
        names: &Names,
        parent: Option<SymbolId>,
        site: Option<c::ControlId>,
        name: &Spanned<String>,
        required: StructuralKind,
    ) -> Result<Option<(usize, SymbolId)>, Exhaustion> {
        self.structural(
            names,
            parent,
            site,
            std::slice::from_ref(name),
            name.span,
            required,
        )
    }
    fn path(
        &mut self,
        names: &Names,
        parent: Option<SymbolId>,
        site: Option<c::ControlId>,
        path: &c::NodeRef,
        required: StructuralKind,
    ) -> Result<Option<(usize, SymbolId)>, Exhaustion> {
        self.structural(names, parent, site, &path.path, path.span, required)
    }

    fn structural_references(
        &mut self,
        protocol: &c::Protocol,
        names: &mut Names,
    ) -> Result<(), Exhaustion> {
        for channel in &protocol.channels {
            self.local(names, None, None, &channel.from, StructuralKind::Role)?;
            self.local(names, None, None, &channel.to, StructuralKind::Role)?;
        }
        for requirement in &protocol.requirements {
            self.work.charge(Dimension::References, 1)?;
            if let c::ProtocolRequirement::Compensation(compensation) = requirement {
                self.path(
                    names,
                    None,
                    None,
                    &compensation.effect,
                    StructuralKind::Effect,
                )?;
                self.local(names, None, None, &compensation.role, StructuralKind::Role)?;
                match &compensation.commit {
                    c::CommitBoundary::Never(_) => {}
                    c::CommitBoundary::Node(node) => {
                        self.path(names, None, None, node, StructuralKind::Commit)?;
                    }
                }
            }
        }
        let controls: Vec<_> = names
            .controls
            .iter()
            .map(|(control, symbol)| (*control, *symbol))
            .collect();
        for (index, symbol) in controls {
            self.work.charge(Dimension::References, 1)?;
            let id = c::ControlId(index);
            let control = self.unit.control(id).expect("parser control handle");
            let parent = self.output.symbols[symbol.0].parent;
            match &control.kind {
                c::ControlKind::Event(event) => {
                    match &event.kind {
                        c::EventKind::Send { channel } => {
                            if let Some((_, target)) = self.local(
                                names,
                                parent,
                                Some(id),
                                channel,
                                StructuralKind::Channel,
                            )? {
                                names.channels.insert(index, target);
                            }
                        }
                        c::EventKind::Receive { channel, send } => {
                            if let Some((_, target)) = self.local(
                                names,
                                parent,
                                Some(id),
                                channel,
                                StructuralKind::Channel,
                            )? {
                                names.channels.insert(index, target);
                            }
                            if let Some((reference, target)) =
                                self.path(names, parent, Some(id), send, StructuralKind::Send)?
                            {
                                names
                                    .preceding
                                    .entry(index)
                                    .or_default()
                                    .push((reference, target));
                                names.receives.push((index, reference, target));
                            }
                        }
                        c::EventKind::Attempt { role, .. } => {
                            self.local(names, parent, Some(id), role, StructuralKind::Role)?;
                        }
                        c::EventKind::Effect { attempt } => {
                            if let Some(reference) = self.path(
                                names,
                                parent,
                                Some(id),
                                attempt,
                                StructuralKind::Attempt,
                            )? {
                                names.preceding.entry(index).or_default().push(reference);
                            }
                        }
                        c::EventKind::Event { role, compensation } => {
                            self.local(names, parent, Some(id), role, StructuralKind::Role)?;
                            if let Some(compensation) = compensation {
                                self.path(
                                    names,
                                    parent,
                                    Some(id),
                                    compensation,
                                    StructuralKind::Compensation,
                                )?;
                            }
                        }
                    }
                    for related in &event.related {
                        self.local(
                            names,
                            parent,
                            Some(id),
                            &related.relationship,
                            StructuralKind::Relationship,
                        )?;
                    }
                }
                c::ControlKind::Choice { role, .. }
                | c::ControlKind::Repeat { role, .. }
                | c::ControlKind::Commit { role, .. } => {
                    self.local(names, parent, Some(id), role, StructuralKind::Role)?;
                }
                c::ControlKind::Await { after, event, .. } => {
                    if let Some((reference, target)) =
                        self.path(names, parent, Some(id), after, StructuralKind::AwaitAnchor)?
                    {
                        if self.output.symbols[target.0].kind != SymbolKind::Compensation {
                            names
                                .preceding
                                .entry(index)
                                .or_default()
                                .push((reference, target));
                        }
                    }
                    let awaited = self.unit.control(*event).expect("parser awaited event");
                    if !matches!(
                        awaited.kind,
                        c::ControlKind::Event(c::Event {
                            kind: c::EventKind::Receive { .. }
                                | c::EventKind::Effect { .. }
                                | c::EventKind::Event { .. },
                            ..
                        })
                    ) {
                        self.issue(ScopeIssue::InvalidAwaitEvent {
                            control: *event,
                            span: awaited.span,
                        })?;
                    }
                }
                c::ControlKind::Parallel { branches, join } => {
                    let mut joined = BTreeSet::new();
                    let mut valid = join.len() == branches.len();
                    for name in join {
                        match self.local(
                            names,
                            Some(symbol),
                            Some(id),
                            name,
                            StructuralKind::Branch,
                        )? {
                            Some((_, target)) => valid &= joined.insert(target),
                            None => valid = false,
                        }
                    }
                    for (branch, _) in branches.iter().enumerate() {
                        self.work.charge(Dimension::References, 1)?;
                        valid &= joined.contains(&names.branches[&(index, branch)]);
                    }
                    if !valid {
                        self.issue(ScopeIssue::InvalidJoin { span: control.span })?;
                    }
                }
                c::ControlKind::Sequence(_) | c::ControlKind::Check { .. } => {}
            }
        }
        for (receiver, reference, send) in &names.receives {
            self.work.charge(Dimension::Edges, 1)?;
            let sent = self.output.symbols[send.0].control.expect("send control");
            if let (Some(left), Some(right)) =
                (names.channels.get(receiver), names.channels.get(&sent.0))
            {
                if left != right {
                    self.issue(ScopeIssue::IncompatibleReference {
                        reference: *reference,
                    })?;
                }
            }
        }
        Ok(())
    }
}

fn control_kind(kind: &c::ControlKind) -> SymbolKind {
    match kind {
        c::ControlKind::Sequence(_) => SymbolKind::Sequence,
        c::ControlKind::Choice { .. } => SymbolKind::Choice,
        c::ControlKind::Parallel { .. } => SymbolKind::Parallel,
        c::ControlKind::Repeat { .. } => SymbolKind::Repeat,
        c::ControlKind::Await { .. } => SymbolKind::Await,
        c::ControlKind::Check { .. } => SymbolKind::Check,
        c::ControlKind::Commit { .. } => SymbolKind::Commit,
        c::ControlKind::Event(event) => match event.kind {
            c::EventKind::Send { .. } => SymbolKind::Send,
            c::EventKind::Receive { .. } => SymbolKind::Receive,
            c::EventKind::Attempt { .. } => SymbolKind::Attempt,
            c::EventKind::Effect { .. } => SymbolKind::Effect,
            c::EventKind::Event { .. } => SymbolKind::Event,
        },
    }
}
impl StructuralKind {
    fn accepts(self, kind: SymbolKind) -> bool {
        match self {
            Self::Role => kind == SymbolKind::Role,
            Self::Channel => kind == SymbolKind::Channel,
            Self::Relationship => kind == SymbolKind::Relationship,
            Self::Send => kind == SymbolKind::Send,
            Self::Attempt => kind == SymbolKind::Attempt,
            Self::Effect => kind == SymbolKind::Effect,
            Self::Commit => kind == SymbolKind::Commit,
            Self::Compensation => kind == SymbolKind::Compensation,
            Self::Branch => kind == SymbolKind::Branch,
            Self::AwaitAnchor => matches!(
                kind,
                SymbolKind::Send
                    | SymbolKind::Receive
                    | SymbolKind::Attempt
                    | SymbolKind::Effect
                    | SymbolKind::Event
                    | SymbolKind::Commit
                    | SymbolKind::Compensation
            ),
        }
    }
}
