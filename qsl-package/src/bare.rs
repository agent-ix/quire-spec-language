// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: the evaluation entry over a checked package read from its bytes
//! alone.
//!
//! A [`BarePackage`] borrows the [`CheckedPackageV2`] that Contract IR's
//! `read_checked_package` returned. It has no other constructor, so no value
//! of the source side (`CheckedPackage`, `Emission`, `AdmittedPackageRead`)
//! can reach the evaluation code that takes it. The dependency closure is read
//! from the direct dependencies the reader retained in the package
//! ([`BarePackage::dependencies`]).

use std::collections::BTreeSet;

use quire_contract_model::CheckedPackageV2;

/// A checked package as a bytes-only consumer holds it (FR-531).
///
/// `BarePackage::new` is the one constructor; nothing builds one from bytes,
/// from a decoded graph or from a term (FR-531-AC-2):
///
/// ```compile_fail,E0599
/// let _ = qsl_package::BarePackage::from_bytes(b"{}");
/// ```
///
/// ```compile_fail,E0599
/// fn forge(graph: quire_contract_model::CheckedPackageV2) {
///     let _ = qsl_package::BarePackage::from_graph(&graph);
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn forge(term: &quire_contract_model::CheckedTerm) {
///     let _ = qsl_package::BarePackage::from_term(term);
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct BarePackage<'p> {
    package: &'p CheckedPackageV2,
}

impl<'p> BarePackage<'p> {
    /// The bare package over `package`, with no supply.
    pub fn new(package: &'p CheckedPackageV2) -> Self {
        Self { package }
    }

    /// The package.
    pub fn package(&self) -> &'p CheckedPackageV2 {
        self.package
    }

    /// The package's dependency closure, read from the direct dependencies
    /// each package retains from its read evidence (IR-1325): every
    /// dependency once, nearest first, in lock-selection order. The root
    /// package is not listed.
    pub fn dependencies(&self) -> Vec<&'p CheckedPackageV2> {
        let mut closure: Vec<&'p CheckedPackageV2> = Vec::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut queue: std::collections::VecDeque<&'p CheckedPackageV2> =
            self.package.direct_dependencies().collect();
        while let Some(next) = queue.pop_front() {
            if !seen.insert(next.name()) {
                continue;
            }
            closure.push(next);
            queue.extend(next.direct_dependencies());
        }
        closure
    }
}
