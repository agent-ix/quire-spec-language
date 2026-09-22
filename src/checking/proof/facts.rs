// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-016: presence-only propositional consequences, with charged set operations.

use std::collections::BTreeMap;

use super::{Meter, ValueKey};
use crate::checking::CheckingError;
use crate::syntax::ExprId;
use qsl_foundation::Span;

#[derive(Clone, Copy, Debug)]
pub(in crate::checking) struct Fact {
    pub present: bool,
    pub guard: ExprId,
    pub outcome: bool,
}

/// None denotes an impossible path, not absence of information.
pub(in crate::checking) type Facts = Option<BTreeMap<ValueKey, Fact>>;

/// Each caller keeps source ownership and its existing accounting/error type.
pub(in crate::checking) trait FactMeter {
    type Error;
    fn facts(&mut self, count: usize, span: Span) -> Result<(), Self::Error>;
}

impl FactMeter for Meter<'_> {
    type Error = Box<CheckingError>;
    fn facts(&mut self, count: usize, span: Span) -> Result<(), Self::Error> {
        Meter::facts(self, count, span)
    }
}

#[derive(Clone, Debug)]
pub(in crate::checking) struct Outcomes {
    pub yes: Facts,
    pub no: Facts,
}

impl Outcomes {
    pub fn unknown() -> Self {
        Self {
            yes: Some(BTreeMap::new()),
            no: Some(BTreeMap::new()),
        }
    }
    pub fn constant(value: bool) -> Self {
        if value {
            Self {
                yes: Some(BTreeMap::new()),
                no: None,
            }
        } else {
            Self {
                yes: None,
                no: Some(BTreeMap::new()),
            }
        }
    }
    pub fn presence(key: ValueKey, guard: ExprId) -> Self {
        Self {
            yes: Some(BTreeMap::from([(
                key,
                Fact {
                    present: true,
                    guard,
                    outcome: true,
                },
            )])),
            no: Some(BTreeMap::from([(
                key,
                Fact {
                    present: false,
                    guard,
                    outcome: false,
                },
            )])),
        }
    }
    pub fn selected(&self, truth: bool) -> &Facts {
        if truth {
            &self.yes
        } else {
            &self.no
        }
    }
    pub fn rebase<M: FactMeter>(
        &mut self,
        guard: ExprId,
        meter: &mut M,
        span: Span,
    ) -> Result<(), M::Error> {
        for (truth, facts) in [(true, &mut self.yes), (false, &mut self.no)] {
            if let Some(facts) = facts {
                meter.facts(facts.len(), span)?;
                for fact in facts.values_mut() {
                    fact.guard = guard;
                    fact.outcome = truth;
                }
            }
        }
        Ok(())
    }
}

pub(in crate::checking) fn sequential<M: FactMeter>(
    left: &Facts,
    right: &Facts,
    meter: &mut M,
    span: Span,
) -> Result<Facts, M::Error> {
    let (Some(left), Some(right)) = (left, right) else {
        return Ok(None);
    };
    meter.facts(left.len() + right.len(), span)?;
    let mut combined = left.clone();
    let mut conflict = false;
    for (&key, &fact) in right {
        if let Some(prior) = combined.get(&key) {
            conflict |= prior.present != fact.present;
        } else {
            combined.insert(key, fact);
        }
    }
    Ok((!conflict).then_some(combined))
}

pub(in crate::checking) fn alternative<M: FactMeter>(
    left: &Facts,
    right: &Facts,
    meter: &mut M,
    span: Span,
) -> Result<Facts, M::Error> {
    match (left, right) {
        (None, None) => Ok(None),
        (Some(facts), None) | (None, Some(facts)) => {
            meter.facts(facts.len(), span)?;
            Ok(Some(facts.clone()))
        }
        (Some(left), Some(right)) => {
            meter.facts(left.len() + right.len(), span)?;
            // Merge ordered maps so the charged work matches bounded iteration,
            // rather than repeated unaccounted searches through another map.
            let mut a = left.iter().peekable();
            let mut b = right.iter().peekable();
            let mut common = BTreeMap::new();
            while let (Some((ak, av)), Some((bk, bv))) = (a.peek(), b.peek()) {
                match ak.cmp(bk) {
                    std::cmp::Ordering::Less => {
                        a.next();
                    }
                    std::cmp::Ordering::Greater => {
                        b.next();
                    }
                    std::cmp::Ordering::Equal => {
                        if av.present == bv.present {
                            common.insert(**ak, **av);
                        }
                        a.next();
                        b.next();
                    }
                }
            }
            Ok(Some(common))
        }
    }
}
