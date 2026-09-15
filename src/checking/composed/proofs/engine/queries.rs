// SPDX-License-Identifier: AGPL-3.0-or-later
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
                if !rational_sum_domain_contains_all_prefixes(projection, total, maximum) {
                    return Err(Error::Cause(Cause {
                        site: self.site(at),
                        kind: CauseKind::UnprovedAggregateDomain,
                    }));
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
        // Type admission establishes zero and the complete projection domain
        // within Total. Keep that precondition explicit before endpoint division
        // or construction of literals in the actual Total representation.
        if total_minimum > 0
            || total_maximum < 0
            || minimum < total_minimum
            || maximum_value > total_maximum
            || maximum == 0
        {
            return Err(upstream(self.site(at)));
        }
        // For every k <= N, every k-occurrence prefix is in [k*a, k*b].
        // Each endpoint is monotone from zero, independently of occurrence
        // order, duplicates or later cancellation. Its final extreme therefore
        // covers all prefixes. If it crosses Total first, select that transition
        // instead, so both literal operands remain valid and the actual IR Add
        // retains the first CheckedRange failure at the original sum.
        for item in [minimum, maximum_value] {
            self.work.charge(D::Expressions, 1, self.site(at))?;
            let prior = if item == 0 {
                0
            } else {
                let bound = if item < 0 {
                    total_minimum
                } else {
                    total_maximum
                };
                // Both operands have the same sign, so integer division gives
                // the exact number of safe endpoint prefixes. Widen before
                // division, including i64::MIN / -1, and before multiplication.
                let safe = i128::from(bound) / i128::from(item);
                let prefix = i128::from(maximum).min(safe + 1);
                let prior = (prefix - 1) * i128::from(item);
                i64::try_from(prior).map_err(|_| upstream(self.site(at)))?
            };
            // This prior is derived from cardinality and the element bound,
            // never supplied as an assumption that an accumulator is in Total.
            let prior = self.numeric_literal(at, prior)?;
            let item = self.numeric_literal(at, item)?;
            let addition = self.node(Kind::Numeric(ir::NumericOperator::Add, prior, item), at)?;
            self.goal(at, addition, path)?;
        }
        Ok(())
    }
}

/// Conservatively proves every normalized rational prefix against the selected
/// total domain. Every admitted denominator divides `lcm(1..=D)`, so every
/// prefix can be represented over that common denominator. Reduction can only
/// decrease the absolute numerator and denominator checked below.
fn rational_sum_domain_contains_all_prefixes(
    projection: &ir::RationalType,
    total: &ir::RationalType,
    maximum: u32,
) -> bool {
    if maximum == 0
        || total.numerator_minimum() > 0
        || total.numerator_maximum() < 0
        || total.numerator_minimum() > projection.numerator_minimum()
        || total.numerator_maximum() < projection.numerator_maximum()
        || total.maximum_denominator() < projection.maximum_denominator()
    {
        return false;
    }
    // A one-element prefix is either zero or one already-normalized projection
    // value. The domain checks above are then complete; scaling it to the LCM
    // would compare an unreduced numerator against normalized total bounds.
    if maximum == 1 {
        return true;
    }

    let mut common_denominator = 1_u128;
    let total_denominator = u128::from(total.maximum_denominator());
    for denominator in 2..=projection.maximum_denominator() {
        let denominator = u128::from(denominator);
        common_denominator = match common_denominator
            .checked_div(gcd(common_denominator, denominator))
            .and_then(|reduced| reduced.checked_mul(denominator))
        {
            Some(value) if value <= total_denominator => value,
            Some(_) | None => return false,
        };
    }

    let Some(scale) = i128::try_from(common_denominator).ok() else {
        return false;
    };
    let count = i128::from(maximum);
    let minimum = i128::from(projection.numerator_minimum()).min(0);
    let maximum_value = i128::from(projection.numerator_maximum()).max(0);
    let Some(minimum) = minimum
        .checked_mul(scale)
        .and_then(|value| value.checked_mul(count))
    else {
        return false;
    };
    let Some(maximum_value) = maximum_value
        .checked_mul(scale)
        .and_then(|value| value.checked_mul(count))
    else {
        return false;
    };

    minimum >= i128::from(total.numerator_minimum())
        && maximum_value <= i128::from(total.numerator_maximum())
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}
