// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-006 / FR-007: fresh charge-before-work accounting and retained error facts.

use super::{
    RuntimeLocation, ValidationDiagnostic, ValidationLimits, ValidationReport, ValidationStatus,
    ValidationUsage,
};
use crate::{Code, Phase, Source, Span};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Stage {
    Identity,
    Binding,
    Observation,
    Population,
    Value,
    Delta,
    Frame,
}

#[derive(Debug)]
pub(super) struct Stopped;
pub(super) type Result<T> = std::result::Result<T, Stopped>;

pub(super) struct Budget<'a, F> {
    pub limits: ValidationLimits,
    pub usage: ValidationUsage,
    pub location: RuntimeLocation,
    pub span: Span,
    source: &'a Source,
    poll: F,
    details: Vec<(Stage, ValidationDiagnostic)>,
    terminal: Option<Box<ValidationDiagnostic>>,
    invalid: bool,
    incomplete: bool,
}

impl<'a, F: FnMut() -> bool> Budget<'a, F> {
    pub fn new(
        source: &'a Source,
        location: RuntimeLocation,
        limits: ValidationLimits,
        poll: F,
    ) -> Self {
        Self {
            limits: limits.bounded(),
            usage: ValidationUsage::default(),
            location,
            span: Span { start: 0, end: 0 },
            source,
            poll,
            details: Vec::new(),
            terminal: None,
            invalid: false,
            incomplete: false,
        }
    }

    pub fn diagnostic(&self, code: Code, message: &'static str) -> Box<ValidationDiagnostic> {
        let diagnostic = crate::diagnostic::error(
            self.source,
            code,
            Phase::Validate,
            self.span.start,
            self.span.end,
            message,
        );
        Box::new(ValidationDiagnostic {
            diagnostic,
            runtime: self.location.clone(),
            related: Vec::new(),
        })
    }

    fn stop(&mut self, code: Code, message: &'static str) -> Stopped {
        self.incomplete = true;
        self.terminal = Some(self.diagnostic(code, message));
        Stopped
    }

    pub fn poll(&mut self) -> Result<()> {
        if (self.poll)() {
            return Err(self.stop(Code::Cancelled, "caller cancelled runtime validation"));
        }
        Ok(())
    }

    pub fn ceiling(
        &mut self,
        amount: Option<usize>,
        maximum: usize,
        message: &'static str,
    ) -> Result<usize> {
        self.poll()?;
        amount
            .filter(|amount| *amount <= maximum)
            .ok_or_else(|| self.stop(Code::ResourceExhausted, message))
    }

    pub fn visit(&mut self) -> Result<()> {
        self.usage.work = self.ceiling(
            self.usage.work.checked_add(1),
            self.limits.work,
            "validation work limit",
        )?;
        Ok(())
    }

    pub fn artifact(&mut self, bytes: usize) -> Result<()> {
        let count = self.ceiling(
            self.usage.artifacts.checked_add(1),
            self.limits.artifacts,
            "validation inventory count limit",
        )?;
        let bytes = self.ceiling(
            self.usage.artifact_bytes.checked_add(bytes),
            self.limits.artifact_bytes,
            "validation inventory content limit",
        )?;
        self.visit()?;
        self.usage.artifacts = count;
        self.usage.artifact_bytes = bytes;
        Ok(())
    }

    pub fn objects(&mut self, count: usize) -> Result<()> {
        self.usage.objects = self.ceiling(
            self.usage.objects.checked_add(count),
            self.limits.objects,
            "validation selected object limit",
        )?;
        Ok(())
    }

    fn scalar_advance(&mut self, chars: &mut std::str::Chars<'_>) -> Result<Option<char>> {
        self.usage.text_steps = self.ceiling(
            self.usage.text_steps.checked_add(1),
            self.limits.text_steps,
            "validation Unicode inspection limit",
        )?;
        Ok(chars.next())
    }

    pub fn text_bound(&mut self, text: &str, maximum: u32) -> Result<bool> {
        let mut chars = text.chars();
        let mut length = 0_u64;
        while self.scalar_advance(&mut chars)?.is_some() {
            length += 1; // At most the already admitted artifact's UTF-8 byte count.
            if length > u64::from(maximum) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn text_equal(&mut self, left: &str, right: &str) -> Result<bool> {
        let mut left = left.chars();
        let mut right = right.chars();
        loop {
            let a = self.scalar_advance(&mut left)?;
            let b = self.scalar_advance(&mut right)?;
            if a != b {
                return Ok(false);
            }
            if a.is_none() {
                return Ok(true);
            }
        }
    }

    pub fn observe(&mut self, stage: Stage, diagnostic: ValidationDiagnostic) -> Result<()> {
        if diagnostic.is_incomplete() {
            self.incomplete = true;
        } else {
            self.invalid = true;
        }
        if self.details.len() >= self.limits.diagnostics {
            return Err(self.stop(
                Code::ResourceExhausted,
                "validation detail diagnostic limit",
            ));
        }
        self.poll()?;
        self.details.push((stage, diagnostic));
        self.usage.diagnostics = self.details.len();
        Ok(())
    }

    pub fn issue(&mut self, stage: Stage, code: Code, message: &'static str) -> Result<()> {
        self.observe(stage, *self.diagnostic(code, message))
    }

    pub fn failed(&self) -> bool {
        self.invalid || self.incomplete
    }

    pub fn report(mut self) -> Box<ValidationReport> {
        self.details.sort_by(|(a_stage, a), (b_stage, b)| {
            a_stage
                .cmp(b_stage)
                .then_with(|| a.runtime.compare(&b.runtime))
                .then_with(|| {
                    (a.span.start.byte, a.span.end.byte).cmp(&(b.span.start.byte, b.span.end.byte))
                })
                .then_with(|| a.code.as_str().cmp(b.code.as_str()))
                // Several independent defects can share the prescribed key.
                // Break those ties by retained content, never discovery order.
                .then_with(|| a.related.cmp(&b.related))
                .then_with(|| a.message.cmp(&b.message))
        });
        Box::new(ValidationReport {
            status: if self.invalid {
                ValidationStatus::Refused
            } else {
                ValidationStatus::Incomplete
            },
            diagnostics: self.details.into_iter().map(|(_, value)| value).collect(),
            terminal: self.terminal,
            usage: self.usage,
        })
    }
}
