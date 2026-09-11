// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: closed control ownership and the contract's finite edge expansion.

use std::collections::{BTreeMap, BTreeSet};

use super::*;

type EdgeKey = (
    usize,
    &'static str,
    usize,
    &'static str,
    usize,
    &'static str,
);

fn endpoint(index: usize, port: Port) -> usize {
    2 * index + usize::from(port == Port::Exit)
}

struct Derived {
    edges: BTreeMap<EdgeKey, Option<i64>>,
    children: Vec<Vec<usize>>,
    parent: Vec<Option<usize>>,
}

impl Derived {
    fn new(count: usize, work: &mut Work) -> Result<Self> {
        let children = edges(count, work)?;
        work.charge(Dimension::Entries, count)?;
        Ok(Self {
            edges: BTreeMap::new(),
            children,
            parent: vec![None; count],
        })
    }
    fn child(&mut self, owner: usize, child: usize, work: &mut Work) -> Result {
        if self.parent[child].replace(owner).is_some() {
            return Err(Error::Invalid(Invalid::Owner));
        }
        edge(&mut self.children, owner, child, work)
    }
    fn edge(
        &mut self,
        owner: usize,
        kind: EdgeKind,
        from: (usize, Port),
        to: (usize, Port),
        maximum: Option<i64>,
        work: &mut Work,
    ) -> Result {
        work.visit()?;
        work.charge(Dimension::Entries, 1)?;
        let key = (
            owner,
            kind.as_str(),
            from.0,
            from.1.as_str(),
            to.0,
            to.1.as_str(),
        );
        if self.edges.insert(key, maximum).is_some() {
            return Err(Error::Invalid(Invalid::Control));
        }
        Ok(())
    }
}

impl Graph<'_, '_> {
    pub(super) fn protocol(&mut self, owner: usize) -> Result {
        let declaration = &self.package.declarations[owner];
        let Body::Protocol {
            input,
            activation,
            captures,
            roles,
            relationships,
            channels,
            compensations,
            temporal_requirements,
            controls,
            causal_edges,
            run,
            finish,
        } = &declaration.body
        else {
            return Err(Error::Invalid(Invalid::Owner));
        };
        self.binder(owner, input, &[BinderKind::Input])?;
        self.activation(owner, activation)?;
        self.captures(owner, captures)?;
        let root = self.local(owner, run, Local::Control)?;
        for (index, temporal) in temporal_requirements.iter().enumerate() {
            self.dependency(owner, *temporal)?;
            if !matches!(
                self.package.declarations[*temporal as usize].body,
                Body::Temporal { .. }
            ) {
                return Err(Error::Invalid(Invalid::Dependency));
            }
            if index > 0 && temporal_requirements[index - 1] >= *temporal {
                return Err(Error::Invalid(Invalid::Order));
            }
        }
        ordered(
            roles
                .iter()
                .enumerate()
                .map(|(index, value)| (&value.locus, index as u32)),
            self.work,
        )?;
        ordered(
            relationships
                .iter()
                .enumerate()
                .map(|(index, value)| (&value.locus, index as u32)),
            self.work,
        )?;
        ordered(
            channels
                .iter()
                .enumerate()
                .map(|(index, value)| (&value.locus, index as u32)),
            self.work,
        )?;
        ordered(
            compensations
                .iter()
                .enumerate()
                .map(|(index, value)| (&value.locus, index as u32)),
            self.work,
        )?;
        ordered(
            controls
                .iter()
                .map(|value| (&value.locus, value.original_node)),
            self.work,
        )?;
        self.unique_names(roles.iter().map(|value| value.name.as_str()))?;
        self.unique_names(relationships.iter().map(|value| value.name.as_str()))?;
        self.unique_names(channels.iter().map(|value| value.name.as_str()))?;
        self.unique_names(compensations.iter().map(|value| value.name.as_str()))?;
        for role in roles {
            self.locus(owner, &role.locus)?;
            self.export(
                &role.model,
                &[
                    ExportKind::Object,
                    ExportKind::Component,
                    ExportKind::Endpoint,
                ],
            )?;
            let binding = self.binding(owner, role.instance, &[BindingKind::RoleInstance])?;
            if binding.model.0.as_ref() != Some(&role.model) {
                return Err(Error::Invalid(Invalid::Binding));
            }
        }
        for relationship in relationships {
            self.locus(owner, &relationship.locus)?;
            self.export(&relationship.model, &[ExportKind::Relationship])?;
            let binding =
                self.binding(owner, relationship.binding, &[BindingKind::Relationship])?;
            if binding.model.0.as_ref() != Some(&relationship.model) {
                return Err(Error::Invalid(Invalid::Binding));
            }
        }
        for channel in channels {
            self.locus(owner, &channel.locus)?;
            self.local(owner, &channel.from, Local::Role)?;
            self.local(owner, &channel.to, Local::Role)?;
            self.first_type(channel.message_type, 1)?;
            self.binding(owner, channel.message, &[BindingKind::Message])?;
            self.binding(owner, channel.send, &[BindingKind::Send])?;
            self.binding(owner, channel.receive, &[BindingKind::Receive])?;
            self.binding(owner, channel.delivery_instance, &[BindingKind::Delivery])?;
            match &channel.ordering {
                Ordering::Unordered {} => {}
                Ordering::Fifo {
                    binder,
                    key,
                    anchor,
                } => {
                    self.binder(owner, binder, &[BinderKind::Fifo])?;
                    let key = self.value(owner, key)?.value_type;
                    self.equality(key)?;
                    let anchor = self.local(owner, anchor, Local::Anchor)?;
                    if declaration.anchors[anchor].kind != AnchorKind::Fifo {
                        return Err(Error::Invalid(Invalid::Binding));
                    }
                }
            }
        }
        let mut derived = Derived::new(controls.len(), self.work)?;
        self.work.charge(Dimension::Entries, controls.len())?;
        let mut originals = BTreeSet::new();
        for (index, control) in controls.iter().enumerate() {
            self.locus(owner, &control.locus)?;
            // Authored names are scoped by the native binder. Equal names in
            // distinct paths do not merge these already selected local handles.
            intake::name(&control.name)?;
            if control.original_node > 1_048_576 {
                return Err(Error::Invalid(Invalid::StructuralInteger));
            }
            if !originals.insert(control.original_node) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            let enter = (index, Port::Enter);
            let exit = (index, Port::Exit);
            match &control.operation {
                ControlOperation::Sequence { children } => {
                    let mut from = enter;
                    for child in children {
                        let child = self.local(owner, child, Local::Control)?;
                        derived.child(index, child, self.work)?;
                        derived.edge(
                            index,
                            EdgeKind::Sequence,
                            from,
                            (child, Port::Enter),
                            None,
                            self.work,
                        )?;
                        from = (child, Port::Exit);
                    }
                    derived.edge(index, EdgeKind::Sequence, from, exit, None, self.work)?;
                }
                ControlOperation::Choice {
                    owner: role,
                    visible,
                    cases,
                } => {
                    self.local(owner, role, Local::Role)?;
                    self.visible(owner, visible)?;
                    if cases.is_empty() {
                        return Err(Error::Invalid(Invalid::Control));
                    }
                    self.unique_names(cases.iter().map(|value| value.label.as_str()))?;
                    for case in cases {
                        self.locus(owner, &case.locus)?;
                        self.boolean(owner, &case.guard)?;
                        let child = self.local(owner, &case.body, Local::Control)?;
                        derived.child(index, child, self.work)?;
                        derived.edge(
                            index,
                            EdgeKind::Branch,
                            enter,
                            (child, Port::Enter),
                            None,
                            self.work,
                        )?;
                        derived.edge(
                            index,
                            EdgeKind::Join,
                            (child, Port::Exit),
                            exit,
                            None,
                            self.work,
                        )?;
                    }
                }
                ControlOperation::Parallel { branches, join } => {
                    if branches.is_empty() || join.len() != branches.len() {
                        return Err(Error::Invalid(Invalid::Control));
                    }
                    self.unique_names(branches.iter().map(|value| value.label.as_str()))?;
                    for (ordinal, (branch, joined)) in branches.iter().zip(join).enumerate() {
                        self.work.visit()?;
                        if *joined as usize != ordinal {
                            return Err(Error::Invalid(Invalid::Control));
                        }
                        self.locus(owner, &branch.locus)?;
                        let child = self.local(owner, &branch.body, Local::Control)?;
                        derived.child(index, child, self.work)?;
                        derived.edge(
                            index,
                            EdgeKind::Branch,
                            enter,
                            (child, Port::Enter),
                            None,
                            self.work,
                        )?;
                        derived.edge(
                            index,
                            EdgeKind::Join,
                            (child, Port::Exit),
                            exit,
                            None,
                            self.work,
                        )?;
                    }
                }
                ControlOperation::Repeat {
                    owner: role,
                    visible,
                    maximum,
                    guard,
                    body,
                    exhausted,
                } => {
                    self.local(owner, role, Local::Role)?;
                    self.visible(owner, visible)?;
                    self.boolean(owner, guard)?;
                    let body = self.local(owner, body, Local::Control)?;
                    let exhausted = self.local(owner, exhausted, Local::Control)?;
                    let maximum = nonnegative(maximum, self.work)?;
                    derived.child(index, body, self.work)?;
                    derived.child(index, exhausted, self.work)?;
                    derived.edge(index, EdgeKind::Branch, enter, exit, None, self.work)?;
                    derived.edge(
                        index,
                        EdgeKind::Branch,
                        enter,
                        (body, Port::Enter),
                        None,
                        self.work,
                    )?;
                    derived.edge(
                        index,
                        EdgeKind::Branch,
                        enter,
                        (exhausted, Port::Enter),
                        None,
                        self.work,
                    )?;
                    derived.edge(
                        index,
                        EdgeKind::RepeatProgress,
                        (body, Port::Exit),
                        enter,
                        Some(maximum),
                        self.work,
                    )?;
                    derived.edge(
                        index,
                        EdgeKind::Join,
                        (exhausted, Port::Exit),
                        exit,
                        None,
                        self.work,
                    )?;
                }
                ControlOperation::Await {
                    after,
                    profile,
                    clock,
                    event,
                    then_body,
                    timeout,
                    ..
                } => {
                    self.profile(*profile, Family::Temporal)?;
                    self.binding(owner, *clock, &[BindingKind::Clock])?;
                    match after {
                        AwaitAnchor::Event { node } => {
                            let after = self.local(owner, node, Local::Control)?;
                            if !matches!(controls[after].operation, ControlOperation::Event { .. })
                            {
                                return Err(Error::Invalid(Invalid::Control));
                            }
                            derived.edge(
                                index,
                                EdgeKind::Sequence,
                                (after, Port::Exit),
                                enter,
                                None,
                                self.work,
                            )?;
                        }
                        AwaitAnchor::Compensation { compensation } => {
                            self.local(owner, compensation, Local::Compensation)?;
                        }
                    }
                    let matched = self.local(owner, event, Local::Control)?;
                    if !matches!(
                        controls[matched].operation,
                        ControlOperation::Event {
                            event: Event::Receive { .. }
                                | Event::Effect { .. }
                                | Event::Event { .. },
                            ..
                        }
                    ) {
                        return Err(Error::Invalid(Invalid::Control));
                    }
                    let then_body = self.local(owner, then_body, Local::Control)?;
                    let timeout = self.local(owner, timeout, Local::Control)?;
                    for child in [matched, then_body, timeout] {
                        derived.child(index, child, self.work)?;
                    }
                    derived.edge(
                        index,
                        EdgeKind::AwaitSuccess,
                        enter,
                        (matched, Port::Enter),
                        None,
                        self.work,
                    )?;
                    derived.edge(
                        index,
                        EdgeKind::AwaitSuccess,
                        (matched, Port::Exit),
                        (then_body, Port::Enter),
                        None,
                        self.work,
                    )?;
                    derived.edge(
                        index,
                        EdgeKind::AwaitTimeout,
                        enter,
                        (timeout, Port::Enter),
                        None,
                        self.work,
                    )?;
                    derived.edge(
                        index,
                        EdgeKind::Join,
                        (then_body, Port::Exit),
                        exit,
                        None,
                        self.work,
                    )?;
                    derived.edge(
                        index,
                        EdgeKind::Join,
                        (timeout, Port::Exit),
                        exit,
                        None,
                        self.work,
                    )?;
                }
                ControlOperation::Event {
                    event,
                    binder,
                    related,
                    constraint,
                } => {
                    self.binder(owner, binder, &[BinderKind::Event])?;
                    self.boolean(owner, constraint)?;
                    for related in related {
                        self.locus(owner, &related.locus)?;
                        self.work.visit()?;
                        if related.relationship as usize >= relationships.len() {
                            return Err(Error::Invalid(Invalid::Reference));
                        }
                        self.value(owner, &related.from)?;
                        self.value(owner, &related.to)?;
                    }
                    match event {
                        Event::Send { channel } => {
                            self.local(owner, channel, Local::Channel)?;
                        }
                        Event::Receive { channel, send } => {
                            self.local(owner, channel, Local::Channel)?;
                            let send = self.local(owner, send, Local::Control)?;
                            if !matches!(&controls[send].operation,
                                ControlOperation::Event { event: Event::Send { channel: selected }, .. } if channel == selected)
                            {
                                return Err(Error::Invalid(Invalid::Control));
                            }
                            derived.edge(
                                index,
                                EdgeKind::Sequence,
                                (send, Port::Exit),
                                enter,
                                None,
                                self.work,
                            )?;
                        }
                        Event::Attempt {
                            owner: role,
                            operation,
                            contracts,
                            instance,
                        } => {
                            self.local(owner, role, Local::Role)?;
                            self.export(operation, &[ExportKind::Operation])?;
                            self.binding(owner, *instance, &[BindingKind::Attempt])?;
                            intake::sorted_indices(contracts, self.work)?;
                            for contract in contracts {
                                self.dependency(owner, *contract)?;
                                if !matches!(&self.package.declarations[*contract as usize].body,
                                    Body::State { clause_kind: ClauseKind::Pre | ClauseKind::Post, operation: Nullable(Some(selected)), .. }
                                        if selected == operation)
                                {
                                    return Err(Error::Invalid(Invalid::Control));
                                }
                            }
                        }
                        Event::Effect { attempt, instance } => {
                            let attempt = self.local(owner, attempt, Local::Control)?;
                            if !matches!(
                                controls[attempt].operation,
                                ControlOperation::Event {
                                    event: Event::Attempt { .. },
                                    ..
                                }
                            ) {
                                return Err(Error::Invalid(Invalid::Control));
                            }
                            self.binding(owner, *instance, &[BindingKind::Effect])?;
                            derived.edge(
                                index,
                                EdgeKind::Sequence,
                                (attempt, Port::Exit),
                                enter,
                                None,
                                self.work,
                            )?;
                        }
                        Event::Event {
                            owner: role,
                            compensation,
                            instance,
                        } => {
                            self.local(owner, role, Local::Role)?;
                            if let Some(compensation) = &compensation.0 {
                                self.local(owner, compensation, Local::Compensation)?;
                            }
                            self.binding(owner, *instance, &[BindingKind::Invocation])?;
                        }
                    }
                    derived.edge(index, EdgeKind::Sequence, enter, exit, None, self.work)?;
                }
                ControlOperation::Check { profile, value } => {
                    self.profile(*profile, Family::State)?;
                    self.boolean(owner, value)?;
                    derived.edge(index, EdgeKind::Sequence, enter, exit, None, self.work)?;
                }
                ControlOperation::Commit {
                    owner: role,
                    binder,
                    constraint,
                    instance,
                } => {
                    self.local(owner, role, Local::Role)?;
                    self.binder(owner, binder, &[BinderKind::Event])?;
                    self.boolean(owner, constraint)?;
                    self.binding(owner, *instance, &[BindingKind::Commit])?;
                    derived.edge(index, EdgeKind::Sequence, enter, exit, None, self.work)?;
                }
            }
        }
        if derived.parent[root].is_some() {
            return Err(Error::Invalid(Invalid::Owner));
        }
        for (index, parent) in derived.parent.iter().enumerate() {
            self.work.visit()?;
            if index != root && parent.is_none() {
                return Err(Error::Invalid(Invalid::Owner));
            }
        }
        acyclic(&derived.children, self.work)?;
        self.compare_edges(owner, causal_edges, &derived, controls.len())?;
        self.compensations(owner, compensations, controls)?;
        self.locus(owner, &finish.locus)?;
        intake::name(&finish.name)?;
        self.binder(owner, &finish.binder, &[BinderKind::Finish])?;
        self.boolean(owner, &finish.constraint)?;
        self.binding(owner, finish.closure, &[BindingKind::Closure])?;
        Ok(())
    }

    fn unique_names<'a>(&mut self, values: impl IntoIterator<Item = &'a str>) -> Result {
        let mut seen = BTreeSet::new();
        for name in values {
            self.work.visit()?;
            intake::name(name)?;
            self.work.bytes(name.len())?;
            self.work.charge(Dimension::Entries, 1)?;
            if !seen.insert(name) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
        }
        Ok(())
    }

    fn visible(&mut self, owner: usize, values: &[Handle]) -> Result {
        for value in values {
            self.value(owner, value)?;
        }
        Ok(())
    }

    fn compare_edges(
        &mut self,
        owner: usize,
        offered: &[CausalEdge],
        required: &Derived,
        count: usize,
    ) -> Result {
        if offered.len() != required.edges.len() {
            return Err(Error::Invalid(Invalid::Control));
        }
        let mut graph = edges(
            count
                .checked_mul(2)
                .ok_or(Error::Invalid(Invalid::Control))?,
            self.work,
        )?;
        let mut previous = None;
        for (offered, (expected, maximum)) in offered.iter().zip(&required.edges) {
            let owner_index = self.local(owner, &offered.owner, Local::Control)?;
            let from = self.local(owner, &offered.from.node, Local::Control)?;
            let to = self.local(owner, &offered.to.node, Local::Control)?;
            let key = (
                owner_index,
                offered.kind.as_str(),
                from,
                offered.from.port.as_str(),
                to,
                offered.to.port.as_str(),
            );
            if previous.is_some_and(|previous| previous >= key) {
                return Err(Error::Invalid(Invalid::Order));
            }
            previous = Some(key);
            let selected_maximum = offered
                .maximum
                .0
                .as_ref()
                .map(|value| integer(value, self.work))
                .transpose()?;
            if &key != expected || &selected_maximum != maximum {
                return Err(Error::Invalid(Invalid::Control));
            }
            if offered.kind != EdgeKind::RepeatProgress {
                edge(
                    &mut graph,
                    endpoint(from, offered.from.port),
                    endpoint(to, offered.to.port),
                    self.work,
                )?;
            }
        }
        acyclic(&graph, self.work)
    }

    fn compensations(
        &mut self,
        owner: usize,
        values: &[Compensation],
        controls: &[Control],
    ) -> Result {
        for value in values {
            self.locus(owner, &value.locus)?;
            let forward = self.local(owner, &value.forward_effect, Local::Control)?;
            if !matches!(
                controls[forward].operation,
                ControlOperation::Event {
                    event: Event::Effect { .. },
                    ..
                }
            ) {
                return Err(Error::Invalid(Invalid::Control));
            }
            self.binder(owner, &value.forward, &[BinderKind::ForwardEffect])?;
            self.local(owner, &value.owner, Local::Role)?;
            self.export(&value.operation, &[ExportKind::Operation])?;
            self.profile(value.profile, Family::Temporal)?;
            self.binding(owner, value.clock, &[BindingKind::Clock])?;
            let registration = self.local(owner, &value.registration_anchor, Local::Anchor)?;
            let activation = self.local(owner, &value.activation_anchor, Local::Anchor)?;
            let declaration = &self.package.declarations[owner];
            if registration == activation
                || declaration.anchors[registration].kind != AnchorKind::Registration
                || declaration.anchors[activation].kind != AnchorKind::CompensationActivation
            {
                return Err(Error::Invalid(Invalid::Binding));
            }
            self.binding(
                owner,
                value.registration_instance,
                &[BindingKind::CompensationRegistration],
            )?;
            self.captures(owner, &value.registration_captures)?;
            self.captures(owner, &value.activation_captures)?;
            self.binder(owner, &value.trigger, &[BinderKind::CompensationTrigger])?;
            self.boolean(owner, &value.guard)?;
            self.first_type(value.attempt_type, 1)?;
            let earlier = self.binder(owner, &value.earlier, &[BinderKind::EarlierAttempt])?;
            let later = self.binder(owner, &value.later, &[BinderKind::LaterAttempt])?;
            if value.earlier == value.later
                || earlier.value_type != value.attempt_type
                || later.value_type != value.attempt_type
            {
                return Err(Error::Invalid(Invalid::Binding));
            }
            self.boolean(owner, &value.retry)?;
            self.binding(
                owner,
                value.attempt_instance,
                &[BindingKind::CompensationAttempt],
            )?;
            self.binding(
                owner,
                value.effect_instance,
                &[BindingKind::CompensationEffect],
            )?;
            if let Some(commit) = &value.commit.0 {
                let commit = self.local(owner, commit, Local::Control)?;
                if !matches!(controls[commit].operation, ControlOperation::Commit { .. }) {
                    return Err(Error::Invalid(Invalid::Control));
                }
            }
            self.binder(owner, &value.recovery, &[BinderKind::Recovery])?;
            self.boolean(owner, &value.recover)?;
            intake::sorted_indices(&value.recovery_bindings, self.work)?;
            for binding in &value.recovery_bindings {
                self.binding(
                    owner,
                    *binding,
                    &[
                        BindingKind::Population,
                        BindingKind::Relationship,
                        BindingKind::Snapshot,
                        BindingKind::Observation,
                        BindingKind::Progress,
                        BindingKind::Closure,
                    ],
                )?;
            }
        }
        Ok(())
    }
}
