// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-307, ADR-017 AR-4: the layer-4 export of the abstraction bindings each
//! requested item references.
//!
//! The orchestrating driver calls [`export_bindings`] after S4 and before
//! `route`; the function names no backend. Each item's result depends only
//! on the checked package and that item, so a refused item never affects
//! another.

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{Code, InternalFault};
use qsl_semantics::check::{AbstractionKey, CheckedBinding};

use crate::CheckedPackage;

/// An item with every binding it references, in ascending key order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundItem {
    /// The requested item.
    pub item: OccurrenceKey,
    /// The bindings the item references.
    pub bindings: Vec<(AbstractionKey, CheckedBinding)>,
}

/// A model element an item references that the relation does not bind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnboundElement {
    /// The element's key.
    pub key: AbstractionKey,
    /// The identity of the domain package that owns the element.
    pub package: String,
}

/// Why an item has no bindings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExportRefusal {
    /// The requested occurrence key names no requirement record.
    UnknownItem {
        /// The requested key.
        item: OccurrenceKey,
    },
    /// The item references elements with no binding.
    Unbound {
        /// The requested item.
        item: OccurrenceKey,
        /// Every unbound element, in ascending key order.
        elements: Vec<UnboundElement>,
    },
    /// The package holds a reference to a declaration it does not admit.
    Fault(InternalFault),
}

impl ExportRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::UnknownItem { .. } | Self::Unbound { .. } => Code::MissingDeclaration,
            Self::Fault(_) => Code::RuntimeInvariant,
        }
    }
}

/// FR-307: for each of `items`, in order, the item with the bindings it
/// references or a refusal. A package with no relation binds nothing.
pub fn export_bindings(
    package: &CheckedPackage,
    items: &[OccurrenceKey],
) -> Vec<Result<BoundItem, ExportRefusal>> {
    items
        .iter()
        .map(|item| export_item(package, item))
        .collect()
}

fn export_item(package: &CheckedPackage, item: &OccurrenceKey) -> Result<BoundItem, ExportRefusal> {
    let elements = package
        .graph()
        .item_elements(item)
        .ok_or_else(|| ExportRefusal::UnknownItem { item: item.clone() })?
        .map_err(ExportRefusal::Fault)?;
    let relation = package.abstraction();
    let mut bindings = Vec::with_capacity(elements.len());
    let mut unbound = Vec::new();
    for key in elements {
        match relation.and_then(|relation| relation.get(&key)) {
            Some(binding) => bindings.push((key, binding.clone())),
            None => unbound.push(UnboundElement {
                package: key.package().to_owned(),
                key,
            }),
        }
    }
    if unbound.is_empty() {
        Ok(BoundItem {
            item: item.clone(),
            bindings,
        })
    } else {
        Err(ExportRefusal::Unbound {
            item: item.clone(),
            elements: unbound,
        })
    }
}

#[cfg(test)]
mod tests;
