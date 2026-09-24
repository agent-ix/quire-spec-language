// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed choreography productions; these declarations never execute actions.
use super::*;

impl Parser {
    pub(super) fn protocol(&mut self) -> Result<Protocol, Box<Diagnostic>> {
        self.expect(K::Over)?;
        let input = self.bound_parameter()?;
        let activation = self.activation()?;
        self.open(K::OpenBrace)?;
        let captures = self.captures()?;
        let mut roles = vec![self.role()?];
        while self.is(K::Role) {
            roles.push(self.role()?);
        }
        let mut relationships = Vec::new();
        while self.is(K::Relationship) {
            self.charge(self.peek().span)?;
            let start = self.take().span.start;
            let name = self.identifier()?;
            self.expect(K::Equal)?;
            let model = self.qualified()?;
            self.expect(K::Semicolon)?;
            relationships.push(Relationship {
                name,
                model,
                span: self.range_from(start),
            });
        }
        let mut channels = Vec::new();
        while self.is(K::Channel) {
            channels.push(self.channel()?);
        }
        let mut requirements = Vec::new();
        while self.is(K::Requires) || self.is(K::Compensate) {
            if self.is(K::Compensate) {
                requirements.push(ProtocolRequirement::Compensation(Box::new(
                    self.compensation()?,
                )));
            } else {
                self.charge(self.peek().span)?;
                let start = self.take().span.start;
                self.expect(K::Temporal)?;
                let name = self.identifier()?;
                self.expect(K::Semicolon)?;
                requirements.push(ProtocolRequirement::Temporal {
                    name,
                    span: self.range_from(start),
                });
            }
        }
        self.expect(K::Run)?;
        let run = self.control_node()?;
        self.charge(self.peek().span)?;
        let start = self.expect(K::Finish)?.span.start;
        let name = self.identifier()?;
        self.expect(K::As)?;
        let parameter = self.bound_parameter()?;
        let constraint = self.value_block()?;
        self.expect(K::Semicolon)?;
        let finish = Finish {
            name,
            parameter,
            constraint,
            span: self.range_from(start),
        };
        self.close(K::CloseBrace)?;
        Ok(Protocol {
            input,
            activation,
            captures,
            roles,
            relationships,
            channels,
            requirements,
            run,
            finish,
        })
    }

    fn role(&mut self) -> Result<Role, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::Role)?.span.start;
        let name = self.identifier()?;
        self.expect(K::On)?;
        let model = self.qualified()?;
        self.expect(K::Semicolon)?;
        Ok(Role {
            name,
            model,
            span: self.range_from(start),
        })
    }

    fn channel(&mut self) -> Result<Channel, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::Channel)?.span.start;
        let name = self.identifier()?;
        self.expect(K::From)?;
        let from = self.identifier()?;
        self.expect(K::To)?;
        let to = self.identifier()?;
        self.expect(K::Carries)?;
        let carries = self.qualified()?;
        self.expect(K::Ordering)?;
        let ordering = if self.is(K::Unordered) {
            Ordering::Unordered(self.take().span)
        } else {
            let start = self.expect(K::Fifo)?.span.start;
            self.expect(K::By)?;
            let parameter = self.bound_parameter()?;
            let key = self.value_block()?;
            Ordering::Fifo {
                parameter,
                key,
                span: self.range_from(start),
            }
        };
        self.expect(K::Delivery)?;
        let delivery = self.interval()?;
        self.expect(K::Semicolon)?;
        Ok(Channel {
            name,
            from,
            to,
            carries,
            ordering,
            delivery,
            span: self.range_from(start),
        })
    }

    fn node_ref(&mut self) -> Result<NodeRef, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.peek().span.start;
        let mut path = vec![self.identifier()?];
        while self.eat(K::Qualify) {
            path.push(self.identifier()?);
        }
        Ok(NodeRef {
            path,
            span: self.range_from(start),
        })
    }

    fn compensation(&mut self) -> Result<Compensation, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::Compensate)?.span.start;
        let name = self.identifier()?;
        self.expect(K::For)?;
        let effect = self.node_ref()?;
        self.expect(K::As)?;
        let forward = self.bound_parameter()?;
        self.expect(K::By)?;
        let role = self.identifier()?;
        self.expect(K::On)?;
        let operation = self.operation()?;
        self.expect(K::Using)?;
        let profile = self.identifier()?;
        self.expect(K::Clock)?;
        let clock = self.string()?;
        self.open(K::OpenBrace)?;
        let registration_captures = self.captures()?;
        self.expect(K::Activate)?;
        self.expect(K::First)?;
        let trigger = self.bound_parameter()?;
        self.expect(K::When)?;
        let guard = self.value_block()?;
        self.open(K::OpenBrace)?;
        let activation_captures = self.captures()?;
        self.close(K::CloseBrace)?;
        self.expect(K::Within)?;
        let within = self.interval()?;
        self.expect(K::Semicolon)?;
        self.expect(K::Attempts)?;
        let attempts = self.unsigned()?;
        self.expect(K::Of)?;
        let attempt_type = self.qualified()?;
        self.expect(K::Semicolon)?;
        self.expect(K::Retry)?;
        self.open(K::OpenParen)?;
        let earlier = self.parameter()?;
        self.expect(K::Comma)?;
        let later = self.parameter()?;
        self.close(K::CloseParen)?;
        let retry = self.value_block()?;
        self.expect(K::Semicolon)?;
        self.expect(K::Commit)?;
        let commit = if self.is(K::Never) {
            CommitBoundary::Never(self.take().span)
        } else {
            CommitBoundary::Node(self.node_ref()?)
        };
        self.expect(K::Semicolon)?;
        self.expect(K::Recover)?;
        let recovery = self.bound_parameter()?;
        let recover = self.value_block()?;
        self.expect(K::Semicolon)?;
        self.close(K::CloseBrace)?;
        Ok(Compensation {
            name,
            effect,
            forward,
            role,
            operation,
            profile,
            clock,
            registration_captures,
            trigger,
            guard,
            activation_captures,
            within,
            attempts,
            attempt_type,
            earlier,
            later,
            retry,
            commit,
            recovery,
            recover,
            span: self.range_from(start),
        })
    }

    fn visibility(&mut self) -> Result<Vec<ExprId>, Box<Diagnostic>> {
        self.expect(K::Visible)?;
        self.open(K::OpenParen)?;
        let values = self.value_list(K::CloseParen)?;
        self.close(K::CloseParen)?;
        Ok(values)
    }

    fn name_list(&mut self, close: K) -> Result<Vec<Spanned<String>>, Box<Diagnostic>> {
        let mut names = Vec::new();
        if !self.is(close) {
            names.push(self.identifier()?);
            while self.eat(K::Comma) {
                names.push(self.identifier()?);
            }
        }
        Ok(names)
    }

    /// Parse one protocol control node with an explicit stack of partially
    /// built parents instead of Rust recursion. `repeat` bodies and `await`
    /// event/then/timeout nodes nest another control node without a bracket,
    /// so such a chain has nesting depth 0 (NFR-001) and is bounded only by
    /// the token and node ceilings. Controls enter the arena in the order the
    /// recursive formulation pushed them: every child before its parent.
    fn control_node(&mut self) -> Result<ControlId, Box<Diagnostic>> {
        /// A control node waiting for the child control being parsed.
        enum Pending {
            Sequence {
                start: usize,
                name: Spanned<String>,
                children: Vec<ControlId>,
            },
            Choice {
                start: usize,
                name: Spanned<String>,
                role: Spanned<String>,
                visible: Vec<ExprId>,
                cases: Vec<Case>,
                case: CaseHeader,
            },
            Parallel {
                start: usize,
                name: Spanned<String>,
                branches: Vec<Branch>,
                branch: BranchHeader,
            },
            RepeatBody {
                start: usize,
                name: Spanned<String>,
                header: RepeatHeader,
            },
            RepeatExhausted {
                start: usize,
                name: Spanned<String>,
                header: RepeatHeader,
                body: ControlId,
            },
            AwaitEvent {
                start: usize,
                name: Spanned<String>,
                header: AwaitHeader,
            },
            AwaitThen {
                start: usize,
                name: Spanned<String>,
                header: AwaitHeader,
                event: ControlId,
            },
            AwaitTimeout {
                start: usize,
                name: Spanned<String>,
                header: AwaitHeader,
                event: ControlId,
                then: ControlId,
            },
        }
        let mut stack: Vec<Pending> = Vec::new();
        'control: loop {
            self.charge(self.peek().span)?;
            let token = self.take();
            if !matches!(
                token.kind,
                K::Sequence
                    | K::Choice
                    | K::Parallel
                    | K::Repeat
                    | K::Await
                    | K::Send
                    | K::Receive
                    | K::Attempt
                    | K::Effect
                    | K::Event
                    | K::Check
                    | K::Commit
            ) {
                return Err(self.failure(
                    Code::InvalidSyntax,
                    Phase::Parse,
                    token.span,
                    "expected protocol control node",
                ));
            }
            let start = token.span.start;
            let name = self.identifier()?;
            let kind = match token.kind {
                K::Sequence => {
                    self.open(K::OpenBrace)?;
                    if !self.is(K::CloseBrace) {
                        stack.push(Pending::Sequence {
                            start,
                            name,
                            children: Vec::new(),
                        });
                        continue 'control;
                    }
                    self.close(K::CloseBrace)?;
                    ControlKind::Sequence(Vec::new())
                }
                K::Choice => {
                    self.expect(K::By)?;
                    let role = self.identifier()?;
                    let visible = self.visibility()?;
                    self.open(K::OpenBrace)?;
                    let case = self.case_header()?;
                    stack.push(Pending::Choice {
                        start,
                        name,
                        role,
                        visible,
                        cases: Vec::new(),
                        case,
                    });
                    continue 'control;
                }
                K::Parallel => {
                    self.open(K::OpenBrace)?;
                    let branch = self.branch_header()?;
                    stack.push(Pending::Parallel {
                        start,
                        name,
                        branches: Vec::new(),
                        branch,
                    });
                    continue 'control;
                }
                K::Repeat => {
                    self.expect(K::By)?;
                    let role = self.identifier()?;
                    let visible = self.visibility()?;
                    self.expect(K::Max)?;
                    let maximum = self.unsigned()?;
                    self.expect(K::While)?;
                    let guard = self.value_block()?;
                    stack.push(Pending::RepeatBody {
                        start,
                        name,
                        header: RepeatHeader {
                            role,
                            visible,
                            maximum,
                            guard,
                        },
                    });
                    continue 'control;
                }
                K::Await => {
                    self.expect(K::After)?;
                    let after = self.node_ref()?;
                    self.expect(K::Using)?;
                    let profile = self.identifier()?;
                    self.expect(K::Clock)?;
                    let clock = self.string()?;
                    self.expect(K::Within)?;
                    let within = self.interval()?;
                    self.expect(K::Match)?;
                    if !matches!(
                        self.peek().kind,
                        K::Send | K::Receive | K::Attempt | K::Effect | K::Event
                    ) {
                        return Err(self.unexpected("event node after match"));
                    }
                    stack.push(Pending::AwaitEvent {
                        start,
                        name,
                        header: AwaitHeader {
                            after,
                            profile,
                            clock,
                            within,
                        },
                    });
                    continue 'control;
                }
                K::Check => {
                    self.expect(K::Using)?;
                    let profile = self.identifier()?;
                    let expression = self.value_block()?;
                    self.expect(K::Semicolon)?;
                    ControlKind::Check {
                        profile,
                        expression,
                    }
                }
                K::Commit => {
                    self.expect(K::By)?;
                    let role = self.identifier()?;
                    self.expect(K::As)?;
                    let parameter = self.bound_parameter()?;
                    let constraint = self.value_block()?;
                    self.expect(K::Semicolon)?;
                    ControlKind::Commit {
                        role,
                        parameter,
                        constraint,
                    }
                }
                _ => ControlKind::Event(self.event(token.kind)?),
            };
            let mut completed = self.push_control(start, name, kind);
            loop {
                let Some(pending) = stack.pop() else {
                    return Ok(completed);
                };
                let (start, name, kind) = match pending {
                    Pending::Sequence {
                        start,
                        name,
                        mut children,
                    } => {
                        children.push(completed);
                        if !self.is(K::CloseBrace) {
                            stack.push(Pending::Sequence {
                                start,
                                name,
                                children,
                            });
                            continue 'control;
                        }
                        self.close(K::CloseBrace)?;
                        (start, name, ControlKind::Sequence(children))
                    }
                    Pending::Choice {
                        start,
                        name,
                        role,
                        visible,
                        mut cases,
                        case,
                    } => {
                        cases.push(Case {
                            name: case.name,
                            guard: case.guard,
                            control: completed,
                            span: self.range_from(case.start),
                        });
                        if cases.len() < 2 || self.is(K::Case) {
                            let case = self.case_header()?;
                            stack.push(Pending::Choice {
                                start,
                                name,
                                role,
                                visible,
                                cases,
                                case,
                            });
                            continue 'control;
                        }
                        self.close(K::CloseBrace)?;
                        (
                            start,
                            name,
                            ControlKind::Choice {
                                role,
                                visible,
                                cases,
                            },
                        )
                    }
                    Pending::Parallel {
                        start,
                        name,
                        mut branches,
                        branch,
                    } => {
                        branches.push(Branch {
                            name: branch.name,
                            control: completed,
                            span: self.range_from(branch.start),
                        });
                        if branches.len() < 2 || self.is(K::Branch) {
                            let branch = self.branch_header()?;
                            stack.push(Pending::Parallel {
                                start,
                                name,
                                branches,
                                branch,
                            });
                            continue 'control;
                        }
                        self.close(K::CloseBrace)?;
                        self.expect(K::Join)?;
                        self.expect(K::All)?;
                        self.open(K::OpenBracket)?;
                        let mut join = vec![self.identifier()?];
                        while self.eat(K::Comma) {
                            join.push(self.identifier()?);
                        }
                        self.close(K::CloseBracket)?;
                        self.expect(K::Semicolon)?;
                        (start, name, ControlKind::Parallel { branches, join })
                    }
                    Pending::RepeatBody {
                        start,
                        name,
                        header,
                    } => {
                        self.expect(K::Exhausted)?;
                        stack.push(Pending::RepeatExhausted {
                            start,
                            name,
                            header,
                            body: completed,
                        });
                        continue 'control;
                    }
                    Pending::RepeatExhausted {
                        start,
                        name,
                        header,
                        body,
                    } => (
                        start,
                        name,
                        ControlKind::Repeat {
                            role: header.role,
                            visible: header.visible,
                            maximum: header.maximum,
                            guard: header.guard,
                            body,
                            exhausted: completed,
                        },
                    ),
                    Pending::AwaitEvent {
                        start,
                        name,
                        header,
                    } => {
                        self.expect(K::Then)?;
                        stack.push(Pending::AwaitThen {
                            start,
                            name,
                            header,
                            event: completed,
                        });
                        continue 'control;
                    }
                    Pending::AwaitThen {
                        start,
                        name,
                        header,
                        event,
                    } => {
                        self.expect(K::Timeout)?;
                        stack.push(Pending::AwaitTimeout {
                            start,
                            name,
                            header,
                            event,
                            then: completed,
                        });
                        continue 'control;
                    }
                    Pending::AwaitTimeout {
                        start,
                        name,
                        header,
                        event,
                        then,
                    } => (
                        start,
                        name,
                        ControlKind::Await {
                            after: header.after,
                            profile: header.profile,
                            clock: header.clock,
                            within: header.within,
                            event,
                            then,
                            timeout: completed,
                        },
                    ),
                };
                completed = self.push_control(start, name, kind);
            }
        }
    }

    /// Append one completed control node spanning from `start` to the last
    /// consumed token.
    fn push_control(
        &mut self,
        start: usize,
        name: Spanned<String>,
        kind: ControlKind,
    ) -> ControlId {
        let span = self.range_from(start);
        let id = ControlId(self.controls.len());
        self.controls.push(Control { name, kind, span });
        id
    }

    /// `case <name> when { <guard> }`, up to the case's control node.
    fn case_header(&mut self) -> Result<CaseHeader, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::Case)?.span.start;
        let name = self.identifier()?;
        self.expect(K::When)?;
        let guard = self.value_block()?;
        Ok(CaseHeader { start, name, guard })
    }

    /// `branch <name>`, up to the branch's control node.
    fn branch_header(&mut self) -> Result<BranchHeader, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::Branch)?.span.start;
        let name = self.identifier()?;
        Ok(BranchHeader { start, name })
    }

    fn event(&mut self, keyword: K) -> Result<Event, Box<Diagnostic>> {
        let kind = match keyword {
            K::Send => {
                self.expect(K::Via)?;
                EventKind::Send {
                    channel: self.identifier()?,
                }
            }
            K::Receive => {
                self.expect(K::Via)?;
                let channel = self.identifier()?;
                self.expect(K::Of)?;
                let send = self.node_ref()?;
                EventKind::Receive { channel, send }
            }
            K::Attempt => {
                self.expect(K::By)?;
                let role = self.identifier()?;
                self.expect(K::On)?;
                let operation = self.operation()?;
                self.expect(K::Contracts)?;
                self.open(K::OpenBracket)?;
                let contracts = self.name_list(K::CloseBracket)?;
                self.close(K::CloseBracket)?;
                EventKind::Attempt {
                    role,
                    operation,
                    contracts,
                }
            }
            K::Effect => {
                self.expect(K::Of)?;
                EventKind::Effect {
                    attempt: self.node_ref()?,
                }
            }
            K::Event => {
                self.expect(K::By)?;
                let role = self.identifier()?;
                let compensation = if self.eat(K::For) {
                    Some(self.node_ref()?)
                } else {
                    None
                };
                EventKind::Event { role, compensation }
            }
            _ => return Err(self.unexpected("event")),
        };
        self.expect(K::As)?;
        let parameter = self.bound_parameter()?;
        let mut related = Vec::new();
        while self.is(K::Related) {
            self.charge(self.peek().span)?;
            let start = self.take().span.start;
            self.expect(K::By)?;
            let relationship = self.identifier()?;
            self.open(K::OpenParen)?;
            let from = self.expression()?;
            self.expect(K::Comma)?;
            let to = self.expression()?;
            self.close(K::CloseParen)?;
            related.push(Related {
                relationship,
                from,
                to,
                span: self.range_from(start),
            });
        }
        let constraint = self.value_block()?;
        self.expect(K::Semicolon)?;
        Ok(Event {
            kind,
            parameter,
            related,
            constraint,
        })
    }
}

/// A parsed `case` header waiting for its control node.
struct CaseHeader {
    start: usize,
    name: Spanned<String>,
    guard: ExprId,
}

/// A parsed `branch` header waiting for its control node.
struct BranchHeader {
    start: usize,
    name: Spanned<String>,
}

/// The fields of a `repeat` control node that precede its body.
struct RepeatHeader {
    role: Spanned<String>,
    visible: Vec<ExprId>,
    maximum: Spanned<String>,
    guard: ExprId,
}

/// The fields of an `await` control node that precede its event node.
struct AwaitHeader {
    after: NodeRef,
    profile: Spanned<String>,
    clock: Spanned<String>,
    within: Interval,
}
