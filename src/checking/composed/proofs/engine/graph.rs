// SPDX-License-Identifier: AGPL-3.0-only
//! Composed source/type/work adapter to the existing IR proof materializer.

use super::*;

impl<'a> proof::graph::Context<'a> for Builder<'_, 'a> {
    type Error = Error;
    fn graph_node(&self, id: GraphId) -> &Node<'a> {
        &self.graph[id.0]
    }
    fn symbol(&self, key: ValueKey) -> Option<&ir::SymbolName> {
        self.key_info[key.0].symbol.as_ref()
    }
    fn source_span(&mut self, at: ExprId) -> Result<ir::SourceSpan> {
        self.source(at)
    }
    fn materialized(&mut self, count: &mut usize, depth: usize, at: ExprId) -> Result<()> {
        self.work.charge(D::Depth, depth, self.site(at))?;
        let next = count.checked_add(1).unwrap_or(usize::MAX);
        self.work.charge(D::GoalNodes, next, self.site(at))?;
        self.work.charge(D::Materialized, 1, self.site(at))?;
        *count = next;
        Ok(())
    }
    fn prepare(&mut self, id: GraphId) -> Result<()> {
        let at = self.graph[id.0].native;
        self.work.charge(D::Types, 1, self.site(at))?;
        match &self.graph[id.0].kind {
            Kind::Rational(..) => self.work.charge(D::Normalization, 128, self.site(at))?,
            Kind::Numeric(..) | Kind::Negate(_) => {
                // The IR starts each bounded numeric input with at most two
                // intervals after a nonzero guard. Products of child widths
                // conservatively bound its Cartesian interval computations.
                let width = self.widths[id.0];
                if self.ty(at).rational().is_some() {
                    self.work.charge(
                        D::Normalization,
                        width.saturating_mul(1024),
                        self.site(at),
                    )?;
                } else {
                    self.work.charge(D::Types, width, self.site(at))?;
                }
            }
            Kind::Input(key) => {
                let symbol = self.key_info[key.0].symbol.as_ref().expect("proof symbol");
                self.work
                    .charge(D::Bytes, symbol.as_str().len(), self.site(at))?;
            }
            Kind::Witness(_, ty) => {
                // Witness adds three copies of the source span and one local
                // symbol, beyond the ordinary materialized root source span.
                self.work.charge(
                    D::Bytes,
                    6 * self.formal.identity().document().as_str().len() + 7,
                    self.site(at),
                )?;
                let site = self.site(at);
                charge_ir_type(ty, self.work, site, 1)?;
            }
            Kind::Boolean(_)
            | Kind::Integer(..)
            | Kind::Present(_)
            | Kind::Unwrap(_)
            | Kind::Not(_)
            | Kind::Compare(..)
            | Kind::BooleanOp(..) => {}
        }
        Ok(())
    }
    fn invalid(&self, at: ExprId, _message: &str, _upstream: Option<ir::Diagnostic>) -> Error {
        self.unsupported(at, Unsupported::ValueRepresentation)
    }
}

fn charge_ir_type(ty: &ir::ValueType, work: &mut Work, site: Site, depth: usize) -> Result<()> {
    work.charge(D::Depth, depth, site)?;
    work.charge(D::Types, 1, site)?;
    match ty {
        ir::ValueType::Option { value } => charge_ir_type(value, work, site, depth + 1)?,
        ir::ValueType::Collection { value } => {
            charge_ir_type(value.element(), work, site, depth + 1)?
        }
        ir::ValueType::Enum { name } | ir::ValueType::Record { name } => {
            work.charge(D::Bytes, name.as_str().len(), site)?
        }
        ir::ValueType::Boolean
        | ir::ValueType::Integer { .. }
        | ir::ValueType::Rational { .. }
        | ir::ValueType::Text => {}
    }
    Ok(())
}
