// SPDX-License-Identifier: AGPL-3.0-only
//! FR-040-AC-5: query-body totality and bounded prefix witnesses, never execution.

use super::*;
use crate::Spanned;

/// A source-owned proof template, instantiated afresh for each consuming binder.
/// Distinct traversals never identify their arbitrary occurrences with each other.
#[derive(Clone)]
pub(super) struct Sequence {
    collection: Value,
    domain: ExprId,
    binder: usize,
    body: ExprId,
    filter: bool,
    locals: BTreeMap<usize, Value>,
    path: Path,
    maximum: u32,
}

struct Body {
    collection: Value,
    binder: usize,
    projected: Value,
    path: Path,
    maximum: u32,
}

impl Builder<'_, '_> {
    fn merged_path(&mut self, current: &Path, prior: &Path, at: ExprId) -> Result<Path> {
        let span = self.site(at).span;
        let facts = facts::sequential(&prior.facts, &current.facts, self, span)?;
        self.work.charge(
            D::Records,
            prior.guards.len() + current.guards.len(),
            self.site(at),
        )?;
        let mut guards = prior.guards.clone();
        guards.extend_from_slice(&current.guards);
        Ok(Path {
            facts: Rc::new(facts),
            guards,
        })
    }

    fn element(
        &mut self,
        at: ExprId,
        domain: ExprId,
        collection: &Value,
        binder: usize,
        path: &Path,
        depth: usize,
    ) -> Result<(Value, Path, u32)> {
        self.work.charge(D::Depth, depth, self.site(at))?;
        self.work.charge(D::Types, 1, self.site(at))?;
        let NativeType::Sequence { element, maximum } = self.ty(domain)? else {
            return Err(upstream(self.site(at)));
        };
        let Some(sequence) = self.sequences.get(&collection.key) else {
            let maximum = *maximum;
            let element =
                self.symbolic(Key::Element(binder, collection.key), domain, element, true)?;
            self.work
                .charge(D::Records, path.guards.len(), self.site(at))?;
            return Ok((element, path.clone(), maximum));
        };
        self.work.charge(
            D::Records,
            sequence.locals.len() + sequence.path.guards.len() + 1,
            self.site(at),
        )?;
        let sequence = sequence.clone();
        let path = self.merged_path(path, &sequence.path, at)?;
        self.work.charge(D::Records, 1, self.site(at))?;
        let context = std::mem::replace(&mut self.context, self.next_context);
        self.next_context = self
            .next_context
            .checked_add(1)
            .ok_or_else(|| upstream(self.site(at)))?;
        let locals = std::mem::replace(&mut self.locals, sequence.locals);
        let result = (|| {
            let (element, path, maximum) = self.element(
                at,
                sequence.domain,
                &sequence.collection,
                binder,
                &path,
                depth + 1,
            )?;
            self.work.charge(D::Records, 1, self.site(at))?;
            self.locals.insert(sequence.binder, element.clone());
            let projected = self.visit(sequence.body, &path, depth + 1)?;
            self.goal(sequence.body, projected.graph, &path)?;
            if sequence.filter {
                let path = self.assume(&path, &projected, true, sequence.body)?;
                let maximum = if path.facts.is_none() { 0 } else { maximum };
                Ok((element, path, maximum))
            } else {
                Ok((projected, path, maximum))
            }
        })();
        self.locals = locals;
        self.context = context;
        result
    }

    fn query_body(
        &mut self,
        at: ExprId,
        name: &Spanned<String>,
        domain: ExprId,
        body: ExprId,
        path: &Path,
        depth: usize,
    ) -> Result<Body> {
        let collection = self.visit(domain, path, depth + 1)?;
        let binder = *self
            .binders
            .get(&name.span.start)
            .ok_or_else(|| upstream(self.site(at)))?;
        let (element, element_path, maximum) =
            self.element(at, domain, &collection, binder, path, depth + 1)?;
        self.work.charge(D::Records, 1, self.site(at))?;
        let previous = self.locals.insert(binder, element.clone());
        let result = self.visit(body, &element_path, depth + 1);
        if let Some(previous) = previous {
            self.locals.insert(binder, previous);
        } else {
            self.locals.remove(&binder);
        }
        let projected = result?;
        self.goal(body, projected.graph, &element_path)?;
        Ok(Body {
            collection,
            binder,
            projected,
            path: element_path,
            maximum,
        })
    }

    pub(super) fn size(
        &mut self,
        at: ExprId,
        domain: ExprId,
        path: &Path,
        depth: usize,
    ) -> Result<Value> {
        let collection = self.visit(domain, path, depth + 1)?;
        self.work.charge(D::Types, 1, self.site(at))?;
        if self
            .sequences
            .get(&collection.key)
            .is_some_and(|sequence| sequence.maximum == 0)
        {
            return self.zero(at);
        }
        // Type admission has proved that this exact named result contains 0..N.
        // The proof-only input overapproximates length; it is not its execution.
        self.symbolic(Key::Expression(at.0), at, self.ty(at)?, false)
    }

    pub(super) fn contains(
        &mut self,
        at: ExprId,
        collection: ExprId,
        member: ExprId,
        path: &Path,
        depth: usize,
    ) -> Result<Value> {
        self.visit(collection, path, depth + 1)?;
        self.visit(member, path, depth + 1)?;
        self.symbolic(Key::Expression(at.0), at, &NativeType::Boolean, false)
    }

    pub(super) fn quantified(
        &mut self,
        at: ExprId,
        name: &Spanned<String>,
        domain: ExprId,
        body: ExprId,
        path: &Path,
        depth: usize,
    ) -> Result<Value> {
        self.query_body(at, name, domain, body, path, depth)?;
        // Body totality is independent of the quantifier's eventual truth.
        self.symbolic(Key::Expression(at.0), at, &NativeType::Boolean, false)
    }

    pub(super) fn query(&mut self, at: ExprId, path: &Path, depth: usize) -> Result<Value> {
        let c::ValueKind::Query {
            op,
            binder,
            domain,
            body,
            ..
        } = &self
            .unit
            .expression(at)
            .ok_or_else(|| upstream(self.site(at)))?
            .kind
        else {
            return Err(upstream(self.site(at)));
        };
        let body_at = *body;
        let result = self.query_body(at, binder, *domain, body_at, path, depth)?;
        match op.value {
            c::QueryOp::Filter | c::QueryOp::Map => {
                let maximum = if op.value == c::QueryOp::Filter {
                    let selected = self.assume(&result.path, &result.projected, true, body_at)?;
                    if selected.facts.is_none() {
                        0
                    } else {
                        result.maximum
                    }
                } else {
                    result.maximum
                };
                let value = self.symbolic(Key::Expression(at.0), at, self.ty(at)?, true)?;
                self.work.charge(
                    D::Records,
                    self.locals.len() + path.guards.len() + 1,
                    self.site(at),
                )?;
                self.sequences.insert(
                    value.key,
                    Sequence {
                        collection: result.collection,
                        domain: *domain,
                        binder: result.binder,
                        body: body_at,
                        filter: op.value == c::QueryOp::Filter,
                        locals: self.locals.clone(),
                        path: path.clone(),
                        maximum,
                    },
                );
                Ok(value)
            }
            c::QueryOp::Count => {
                if result.maximum == 0 {
                    self.zero(at)
                } else {
                    self.symbolic(Key::Expression(at.0), at, self.ty(at)?, false)
                }
            }
            c::QueryOp::Sum => {
                if result.maximum == 0 {
                    return self.zero(at);
                }
                self.sum_prefixes(at, body_at, result.maximum, path)?;
                self.symbolic(Key::Expression(at.0), at, self.ty(at)?, false)
            }
        }
    }

    fn zero(&mut self, at: ExprId) -> Result<Value> {
        let graph = self.numeric_literal(at, 0)?;
        self.expression(graph, at, true, Outcomes::unknown())
    }

    fn numeric_literal(&mut self, at: ExprId, value: i64) -> Result<GraphId> {
        let kind = if let Some(ty) = self.ty(at)?.integer() {
            Kind::Integer(value, ty)
        } else if let Some(ty) = self.ty(at)?.rational() {
            Kind::Rational(value, 1, ty)
        } else {
            return Err(upstream(self.site(at)));
        };
        self.node(kind, at)
    }

    fn sum_prefixes(&mut self, at: ExprId, body: ExprId, maximum: u32, path: &Path) -> Result<()> {
        let (minimum, maximum_value, total_minimum, total_maximum) =
            if let (Some(projection), Some(total)) =
                (self.ty(body)?.integer(), self.ty(at)?.integer())
            {
                (
                    projection.minimum(),
                    projection.maximum(),
                    total.minimum(),
                    total.maximum(),
                )
            } else if let (Some(projection), Some(total)) =
                (self.ty(body)?.rational(), self.ty(at)?.rational())
            {
                if projection.maximum_denominator() != 1 || total.maximum_denominator() != 1 {
                    return Err(self.unsupported(at, Unsupported::SumDomainTransfer));
                }
                (
                    projection.numerator_minimum(),
                    projection.numerator_maximum(),
                    total.numerator_minimum(),
                    total.numerator_maximum(),
                )
            } else {
                return Err(upstream(self.site(at)));
            };
        // Base is exactly zero. At each k <= the admitted maximum, addition is
        // monotone in both operands, so these two independently checked endpoint
        // transitions enclose every k-occurrence prefix, irrespective of order
        // or duplicates. No accumulator is assumed to inhabit the desired Total.
        let (mut lower, mut upper) = (0_i64, 0_i64);
        for _prefix in 1..=maximum {
            self.work.charge(D::Expressions, 1, self.site(at))?;
            for (prior, item) in [(lower, minimum), (upper, maximum_value)] {
                let prior = self.numeric_literal(at, prior)?;
                let item = self.numeric_literal(at, item)?;
                let addition =
                    self.node(Kind::Numeric(ir::NumericOperator::Add, prior, item), at)?;
                self.goal(at, addition, path)?;
            }
            let next_lower = i128::from(lower) + i128::from(minimum);
            let next_upper = i128::from(upper) + i128::from(maximum_value);
            if next_lower < i128::from(total_minimum) || next_upper > i128::from(total_maximum) {
                // The just-retained actual IR goal diagnoses the first failing
                // prefix; do not construct out-of-domain later prefix literals.
                break;
            }
            lower = i64::try_from(next_lower).map_err(|_| upstream(self.site(at)))?;
            upper = i64::try_from(next_upper).map_err(|_| upstream(self.site(at)))?;
            if minimum == 0 && maximum_value == 0 {
                break;
            }
        }
        Ok(())
    }
}
