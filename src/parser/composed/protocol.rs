// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed choreography productions; these declarations never execute actions.
use super::*;

impl Parser {
    pub(super) fn protocol(&mut self) -> Result<Protocol, Box<Diagnostic>> {
        self.expect(K::Over)?;
        let input = self.bound_parameter()?;
        let activation = self.activation()?;
        self.expect(K::OpenBrace)?;
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
        self.expect(K::CloseBrace)?;
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
        self.expect(K::OpenBrace)?;
        let registration_captures = self.captures()?;
        self.expect(K::Activate)?;
        self.expect(K::First)?;
        let trigger = self.bound_parameter()?;
        self.expect(K::When)?;
        let guard = self.value_block()?;
        self.expect(K::OpenBrace)?;
        let activation_captures = self.captures()?;
        self.expect(K::CloseBrace)?;
        self.expect(K::Within)?;
        let within = self.interval()?;
        self.expect(K::Semicolon)?;
        self.expect(K::Attempts)?;
        let attempts = self.unsigned()?;
        self.expect(K::Of)?;
        let attempt_type = self.qualified()?;
        self.expect(K::Semicolon)?;
        self.expect(K::Retry)?;
        self.expect(K::OpenParen)?;
        let earlier = self.parameter()?;
        self.expect(K::Comma)?;
        let later = self.parameter()?;
        self.expect(K::CloseParen)?;
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
        self.expect(K::CloseBrace)?;
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
        self.expect(K::OpenParen)?;
        let values = self.value_list(K::CloseParen)?;
        self.expect(K::CloseParen)?;
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

    fn control_node(&mut self) -> Result<ControlId, Box<Diagnostic>> {
        self.enter()?;
        let result = self.control_inner();
        self.depth -= 1;
        result
    }

    fn control_inner(&mut self) -> Result<ControlId, Box<Diagnostic>> {
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
        let name = self.identifier()?;
        let kind = match token.kind {
            K::Sequence => {
                self.expect(K::OpenBrace)?;
                let mut children = Vec::new();
                while !self.is(K::CloseBrace) {
                    children.push(self.control_node()?);
                }
                self.expect(K::CloseBrace)?;
                ControlKind::Sequence(children)
            }
            K::Choice => {
                self.expect(K::By)?;
                let role = self.identifier()?;
                let visible = self.visibility()?;
                self.expect(K::OpenBrace)?;
                let mut cases = vec![self.case()?, self.case()?];
                while self.is(K::Case) {
                    cases.push(self.case()?);
                }
                self.expect(K::CloseBrace)?;
                ControlKind::Choice {
                    role,
                    visible,
                    cases,
                }
            }
            K::Parallel => {
                self.expect(K::OpenBrace)?;
                let mut branches = vec![self.branch()?, self.branch()?];
                while self.is(K::Branch) {
                    branches.push(self.branch()?);
                }
                self.expect(K::CloseBrace)?;
                self.expect(K::Join)?;
                self.expect(K::All)?;
                self.expect(K::OpenBracket)?;
                let mut join = vec![self.identifier()?];
                while self.eat(K::Comma) {
                    join.push(self.identifier()?);
                }
                self.expect(K::CloseBracket)?;
                self.expect(K::Semicolon)?;
                ControlKind::Parallel { branches, join }
            }
            K::Repeat => {
                self.expect(K::By)?;
                let role = self.identifier()?;
                let visible = self.visibility()?;
                self.expect(K::Max)?;
                let maximum = self.unsigned()?;
                self.expect(K::While)?;
                let guard = self.value_block()?;
                let body = self.control_node()?;
                self.expect(K::Exhausted)?;
                let exhausted = self.control_node()?;
                ControlKind::Repeat {
                    role,
                    visible,
                    maximum,
                    guard,
                    body,
                    exhausted,
                }
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
                let event = self.control_node()?;
                self.expect(K::Then)?;
                let then = self.control_node()?;
                self.expect(K::Timeout)?;
                let timeout = self.control_node()?;
                ControlKind::Await {
                    after,
                    profile,
                    clock,
                    within,
                    event,
                    then,
                    timeout,
                }
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
        let span = self.range_from(token.span.start);
        let id = ControlId(self.controls.len());
        self.controls.push(Control { name, kind, span });
        Ok(id)
    }

    fn case(&mut self) -> Result<Case, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::Case)?.span.start;
        let name = self.identifier()?;
        self.expect(K::When)?;
        let guard = self.value_block()?;
        let control = self.control_node()?;
        Ok(Case {
            name,
            guard,
            control,
            span: self.range_from(start),
        })
    }

    fn branch(&mut self) -> Result<Branch, Box<Diagnostic>> {
        self.charge(self.peek().span)?;
        let start = self.expect(K::Branch)?.span.start;
        let name = self.identifier()?;
        let control = self.control_node()?;
        Ok(Branch {
            name,
            control,
            span: self.range_from(start),
        })
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
                self.expect(K::OpenBracket)?;
                let contracts = self.name_list(K::CloseBracket)?;
                self.expect(K::CloseBracket)?;
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
            self.expect(K::OpenParen)?;
            let from = self.expression()?;
            self.expect(K::Comma)?;
            let to = self.expression()?;
            self.expect(K::CloseParen)?;
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
