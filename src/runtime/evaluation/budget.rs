// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-006: atomic expression/event entry and independent bounded work counters.

use super::{EvaluationLimits, EvaluationUsage, ImplicationEvent, Result};
use crate::{Code, Diagnostic, Phase, Source, Span};

pub(super) struct Budget<'a, P> {
    pub source: &'a Source,
    pub limits: EvaluationLimits,
    pub usage: EvaluationUsage,
    pub events: Vec<ImplicationEvent>,
    pub poll: P,
}

impl<P: FnMut() -> bool> Budget<'_, P> {
    pub fn error(&self, code: Code, span: Span, message: &'static str) -> Box<Diagnostic> {
        crate::diagnostic::error(
            self.source,
            code,
            Phase::Evaluate,
            span.start,
            span.end,
            message,
        )
    }

    pub fn poll(&mut self, span: Span) -> Result<()> {
        if (self.poll)() {
            return Err(self.error(Code::Cancelled, span, "caller cancelled native evaluation"));
        }
        Ok(())
    }

    pub fn enter(
        &mut self,
        span: Span,
        depth: usize,
        event: Option<ImplicationEvent>,
    ) -> Result<()> {
        self.poll(span)?;
        if self.usage.expression_steps == self.limits.expression_steps {
            return Err(self.error(
                Code::ResourceExhausted,
                span,
                "reference expression-step limit exceeded",
            ));
        }
        if depth > self.limits.depth {
            return Err(self.error(
                Code::ResourceExhausted,
                span,
                "reference expression-depth limit exceeded",
            ));
        }
        if let Some(event) = event {
            self.event_ready(event.span)?;
        }
        // Entry and its optional event commit together after every preflight.
        self.usage.expression_steps += 1;
        self.usage.expression_depth = self.usage.expression_depth.max(depth);
        if let Some(event) = event {
            self.store_event(event);
        }
        Ok(())
    }

    fn event_ready(&mut self, span: Span) -> Result<()> {
        self.poll(span)?;
        if self.usage.events == self.limits.events {
            return Err(self.error(
                Code::ResourceExhausted,
                span,
                "reference event-storage limit exceeded",
            ));
        }
        Ok(())
    }

    fn store_event(&mut self, event: ImplicationEvent) {
        self.events.push(event);
        self.usage.events += 1;
    }

    pub fn event(&mut self, event: ImplicationEvent) -> Result<()> {
        self.event_ready(event.span)?;
        self.store_event(event);
        Ok(())
    }

    pub fn graph(&mut self, span: Span) -> Result<()> {
        self.poll(span)?;
        if self.usage.graph_steps == self.limits.graph_steps {
            return Err(self.error(
                Code::ResourceExhausted,
                span,
                "reference graph-step limit exceeded",
            ));
        }
        self.usage.graph_steps += 1;
        Ok(())
    }

    pub fn comparison(&mut self, span: Span, depth: usize) -> Result<()> {
        self.poll(span)?;
        if self.usage.comparisons == self.limits.comparisons {
            return Err(self.error(
                Code::ResourceExhausted,
                span,
                "reference value-comparison limit exceeded",
            ));
        }
        if depth > self.limits.depth {
            return Err(self.error(
                Code::ResourceExhausted,
                span,
                "reference comparison-depth limit exceeded",
            ));
        }
        self.usage.comparisons += 1;
        self.usage.comparison_depth = self.usage.comparison_depth.max(depth);
        Ok(())
    }

    pub fn text(&mut self, span: Span) -> Result<()> {
        self.poll(span)?;
        if self.usage.text_steps == self.limits.text_steps {
            return Err(self.error(
                Code::ResourceExhausted,
                span,
                "reference text-inspection limit exceeded",
            ));
        }
        self.usage.text_steps += 1;
        Ok(())
    }
}
