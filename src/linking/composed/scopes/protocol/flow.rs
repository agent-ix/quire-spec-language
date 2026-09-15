// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036: bounded control flow; only necessarily produced records escape a node.
use super::*;

enum Task<'a> {
    Control(c::ControlId, Environment),
    Sequence(&'a [c::ControlId], usize, Environment),
    SequenceAfter(&'a [c::ControlId], usize),
    Parallel(&'a [c::Branch], usize, Environment, Environment, Span),
    ParallelAfter(&'a [c::Branch], usize, Environment, Environment, Span),
    AwaitAfter(c::ControlId, c::ControlId, Environment),
    Restore(Environment),
}

impl Resolver<'_, '_> {
    fn schedule<'a>(
        &mut self,
        tasks: &mut Vec<Task<'a>>,
        task: Task<'a>,
    ) -> Result<(), Exhaustion> {
        self.work.charge(Dimension::Edges, 1)?;
        tasks.push(task);
        Ok(())
    }
    fn contains(&mut self, env: Environment, binder: BinderId) -> Result<bool, Exhaustion> {
        let mut cursor = env.frame;
        while let Some(index) = cursor {
            self.work.charge(Dimension::Edges, 1)?;
            let frame = &self.frames[index];
            if frame.binder == binder {
                return Ok(true);
            }
            cursor = frame.parent;
        }
        Ok(false)
    }
    fn preceding(
        &mut self,
        id: c::ControlId,
        env: Environment,
        names: &Names,
    ) -> Result<(), Exhaustion> {
        if let Some(references) = names.preceding.get(&id.0) {
            for (reference, target) in references {
                self.work.charge(Dimension::Edges, 1)?;
                let target_control = self.output.symbols[target.0]
                    .control
                    .expect("event/commit target");
                let available = match names.records.get(&target_control.0) {
                    Some(binder) => self.contains(env, *binder)?,
                    None => false,
                };
                if !available {
                    self.issue(ScopeIssue::UnavailableTarget {
                        reference: *reference,
                        target: *target,
                    })?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn flow(
        &mut self,
        root: c::ControlId,
        input: Environment,
        names: &mut Names,
    ) -> Result<Environment, Exhaustion> {
        let mut tasks = vec![Task::Control(root, input)];
        let mut result = input;
        while let Some(task) = tasks.pop() {
            self.work.charge(Dimension::References, 1)?;
            match task {
                Task::Restore(env) => result = env,
                Task::Sequence(children, index, env) => {
                    if let Some(child) = children.get(index) {
                        self.schedule(&mut tasks, Task::SequenceAfter(children, index + 1))?;
                        self.schedule(&mut tasks, Task::Control(*child, env))?;
                    } else {
                        result = env;
                    }
                }
                Task::SequenceAfter(children, next) => {
                    self.schedule(&mut tasks, Task::Sequence(children, next, result))?
                }
                Task::Parallel(branches, index, base, joined, span) => {
                    if let Some(branch) = branches.get(index) {
                        self.schedule(
                            &mut tasks,
                            Task::ParallelAfter(branches, index + 1, base, joined, span),
                        )?;
                        self.schedule(&mut tasks, Task::Control(branch.control, base))?;
                    } else {
                        result = joined;
                    }
                }
                Task::ParallelAfter(branches, next, base, joined, span) => {
                    let joined = self.export_frames(result, base, joined, None, span)?;
                    self.schedule(
                        &mut tasks,
                        Task::Parallel(branches, next, base, joined, span),
                    )?;
                }
                Task::AwaitAfter(then, timeout, base) => {
                    self.schedule(&mut tasks, Task::Restore(base))?;
                    self.schedule(&mut tasks, Task::Control(timeout, base))?;
                    self.schedule(&mut tasks, Task::Control(then, result))?;
                }
                Task::Control(id, inherited) => {
                    let env = Environment {
                        anchor: Anchor::Control(id),
                        ..inherited
                    };
                    self.preceding(id, env, names)?;
                    let node = self.unit.control(id).expect("parser control handle");
                    match &node.kind {
                        c::ControlKind::Sequence(children) => {
                            self.schedule(&mut tasks, Task::Sequence(children, 0, env))?
                        }
                        c::ControlKind::Parallel { branches, .. } => self.schedule(
                            &mut tasks,
                            Task::Parallel(branches, 0, env, env, node.span),
                        )?,
                        c::ControlKind::Choice { visible, cases, .. } => {
                            for value in visible {
                                self.expression(*value, env)?;
                            }
                            for case in cases {
                                self.expression(case.guard, env)?;
                            }
                            self.schedule(&mut tasks, Task::Restore(env))?;
                            for case in cases.iter().rev() {
                                self.schedule(&mut tasks, Task::Control(case.control, env))?;
                            }
                        }
                        c::ControlKind::Repeat {
                            visible,
                            guard,
                            body,
                            exhausted,
                            ..
                        } => {
                            for value in visible {
                                self.expression(*value, env)?;
                            }
                            self.expression(*guard, env)?;
                            self.schedule(&mut tasks, Task::Restore(env))?;
                            self.schedule(&mut tasks, Task::Control(*exhausted, env))?;
                            self.schedule(&mut tasks, Task::Control(*body, env))?;
                        }
                        c::ControlKind::Await {
                            event,
                            then,
                            timeout,
                            ..
                        } => {
                            self.schedule(&mut tasks, Task::AwaitAfter(*then, *timeout, env))?;
                            self.schedule(&mut tasks, Task::Control(*event, env))?;
                        }
                        c::ControlKind::Check { expression, .. } => {
                            self.expression(*expression, env)?;
                            result = env;
                        }
                        c::ControlKind::Event(event) => {
                            let binder = self.parameter(
                                &event.parameter,
                                BinderKind::EventRecord,
                                env.anchor,
                            )?;
                            let bound = self.extend(env, binder)?;
                            names.records.insert(id.0, binder);
                            for related in &event.related {
                                self.expression(related.from, bound)?;
                                self.expression(related.to, bound)?;
                            }
                            self.expression(event.constraint, bound)?;
                            result = bound;
                        }
                        c::ControlKind::Commit {
                            parameter,
                            constraint,
                            ..
                        } => {
                            let binder =
                                self.parameter(parameter, BinderKind::EventRecord, env.anchor)?;
                            let bound = self.extend(env, binder)?;
                            names.records.insert(id.0, binder);
                            self.expression(*constraint, bound)?;
                            result = bound;
                        }
                    }
                }
            }
        }
        Ok(result)
    }
}
