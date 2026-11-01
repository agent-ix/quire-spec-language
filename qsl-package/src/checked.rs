// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-1 (FR-087): the S4 in-process checked-package
//! typestate ([`CheckedPackage`]) and its packaged wire counterpart
//! ([`EmittedPackage`]). Both are distinct from
//! `protocol_artifact::native::EmittedPackage` (SEAM-3, unrelated:
//! FR-087-AC-8/TC-247) and from `checking::CheckedPackage<'a>`
//! (lane-private, ADR-013 §6: O-15's canonical owner is this module's
//! `CheckedPackage`, not that one).
//!
//! `CheckedPackage`'s constructors ([`CheckedPackage::link`] and
//! [`CheckedPackage::link_with`]) and its fields are private to this module:
//! the only conversion into it is the S4 link step, over an already-checked
//! [`CheckedGraph`] (S3's own stage output, `qsl_semantics::check`) and
//! other already-checked `CheckedPackage`s (E4's dependency closure), never
//! over an unchecked or wire-admitted value
//! (R-10, O-15) -- there is no `From`/`Into` impl from `VerifiedPackage`,
//! `ImportView` or any `protocol_artifact`-read value, and no struct-literal
//! construction reachable from outside this module (TC-244 rows 2-6).
//!
//! Every check-owned-type accessor (`Node`, `Signature`, and the rest of
//! `CheckedGraph`'s own accessor surface) is reached only by delegating
//! through [`CheckedPackage::graph`]: this module imports exactly one
//! layer-3 `check`-core item, `CheckedGraph` itself (FR-087-AC-9/TC-256).
//! `value::expression`'s own `CheckedPackageEvaluation` trait impl for this
//! type (`call`, `evaluate`) reaches `check`-owned state through that same
//! `graph()` accessor, under its own separate, permitted
//! layer-5-depends-on-layer-3 edge -- not routed through this module, and
//! outside this module's own one-item `check`-core import count. A trait,
//! not an inherent impl: `CheckedPackage` is this crate's type, and an
//! inherent `impl CheckedPackage` outside its defining crate is E0116.
//!
//! `EmittedPackage`'s constructor ([`EmittedPackage::new`]) is the v2
//! emitter (`CheckedPackage` -> these bytes, C-03), ADR-011 T-8 (M-4,
//! #242): [`super::emit`], a sibling module of this one, so
//! it stays within this crate's `pub(super)` reach without being
//! reachable from outside `qsl-package` (ADR-013 O-02: the package name
//! comes from the checked manifest, never from a caller).

use alloc::collections::BTreeMap;
#[cfg(feature = "std")]
use alloc::collections::BTreeSet;
#[cfg(feature = "std")]
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::sync::OnceLock;

#[cfg(feature = "std")]
use qsl_foundation::Code;
use qsl_semantics::check::{
    AbstractionRefusal, AuthoredBinding, CheckedAbstractionRelation, CheckedGraph,
};
use qsl_semantics::library::LibraryName;

/// The limits the E4 emitter encodes `quire.checked-package/v2` bytes under:
/// no byte bound of its own. The bytes
/// project a checked graph the check stage already bounded (`checking.nodes`,
/// `checking.input_bytes`), the stage budget FR-259 Behavior 3 has the site
/// pass; a bound here could only refuse a package `check` admitted, with no
/// setting to raise.
#[cfg(feature = "std")]
pub(crate) const EMIT_LIMITS: quire_canonical::Limits = quire_canonical::Limits::new(u64::MAX);

#[cfg(feature = "std")]
use crate::emit::{emit_checked, Emission, EmitRefusal};

/// S4 in-process checked package (ADR-013 T-1): this package's own checked
/// declarations (a [`CheckedGraph`], S3's stage output) plus the checked
/// dependency closure E4 names. Every field is private to this module
/// (ADR-011 §4); [`Self::link`] and [`Self::link_with`] are the only
/// constructors.
///
/// TC-244 row 2 (FR-087-AC-2): a `CheckedGraph` never becomes a
/// `CheckedPackage` by any path other than the S4 link step -- in
/// particular, not by naming this struct's private fields directly from
/// outside `qsl-package`:
/// ```compile_fail,E0451
/// use qsl_package::CheckedPackage;
/// let forged = CheckedPackage {
///     graph: todo!(),
///     dependencies: Default::default(),
/// };
/// ```
///
/// TC-244 row 6 (FR-087-AC-2) names a second forbidden path -- decoding
/// `EmittedPackage`'s wire bytes and forcing the result directly into a
/// `CheckedPackage`, bypassing the verified binding and the S1-S4 recompile
/// `replay`'s own E9 uses instead (ADR-013 T-2). No constructor accepting
/// decoded bytes is exposed at all; the only way to attempt it is the same
/// private struct literal row 2's doctest already forecloses, so row 6 is
/// covered by row 2 rather than carrying its own separate doctest.
///
/// Stable rustdoc does not check a `compile_fail` block's error code, so a
/// renamed or moved import would make each block below fail for the wrong
/// reason and still pass. Each is paired with a block that must compile,
/// over the same `use` lines and the same value, so a rename or a move
/// breaks the build instead.
///
/// TC-244 row 2's pair:
/// ```no_run
/// use qsl_package::CheckedPackage;
/// fn holds(package: CheckedPackage) -> CheckedPackage {
///     package
/// }
/// ```
///
/// TC-244 row 3: a wire-admitted `ImportView` has no conversion into a
/// `CheckedPackage`:
/// ```compile_fail,E0277
/// use qsl_package::CheckedPackage;
/// use qsl_semantics::library::ImportView;
/// fn forge(view: ImportView) -> CheckedPackage {
///     view.into()
/// }
/// ```
/// Its pair:
/// ```no_run
/// use qsl_package::CheckedPackage;
/// use qsl_semantics::library::ImportView;
/// fn holds(view: ImportView, package: CheckedPackage) -> CheckedPackage {
///     drop(view);
///     package
/// }
/// ```
///
/// Row 5, a `protocol_artifact`-read value, is at the root crate's
/// `protocol_artifact::AdmittedPackage`: only the root crate can name both
/// it and `CheckedPackage`.
///
/// quire:canonical
pub struct CheckedPackage {
    graph: Arc<CheckedGraph>,
    /// E4's checked dependency closure: each directly imported package's
    /// own checked package, compiled from its source, by library name
    /// ([`Self::link_with`]). Shared: one
    /// library compile serves every import that reaches it (ADR-015 D-1).
    dependencies: BTreeMap<LibraryName, Arc<CheckedPackage>>,
    /// The resolved library closure (FR-307):
    /// one selection per library identity, over the direct imports and every
    /// dependency's own closure, keyed by the library identity an import
    /// names. [`LibraryName`]'s order is UTF-8 byte order, the order FR-322
    /// writes (ADR-015 D-3).
    selections: BTreeMap<LibraryName, ResolvedDependency>,
    /// Identities named by this unit's imports. Kept separately because a
    /// direct identity may first be reached through another import's lock.
    /// Only `link_with` and the emitter read it, so it exists with `std`.
    #[cfg(feature = "std")]
    direct_imports: BTreeSet<LibraryName>,
    /// FR-304, FR-306: this package's checked abstraction relation, when it
    /// declares one. The checked package is its only authority.
    abstraction: Option<CheckedAbstractionRelation>,
    /// This package's checked emission, set only by the emitter
    /// ([`Self::remember_emission`]; the bytes stay the emitter's own
    /// output, ADR-013 O-02) so a package reached again, as a dependency or
    /// by another emit, is not emitted again. Reset when the package changes.
    #[cfg(feature = "std")]
    emission: OnceLock<Emission>,
}

/// One library identity's selection in a package's resolved closure, and the
/// first dependency path that reached it (FR-307).
#[derive(Clone, Debug)]
pub struct ResolvedDependency {
    /// The selected checked package. One identity must resolve to one
    /// package in the graph, even when reached through a diamond.
    package: Arc<CheckedPackage>,
    /// How the first dependency path reached this selection: its first hop
    /// and the package that hop imports, which holds the rest of the path.
    path: DependencyPath,
}

impl PartialEq for ResolvedDependency {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.package, &other.package) && self.path == other.path
    }
}

impl Eq for ResolvedDependency {}

impl ResolvedDependency {
    /// The library identities from the linking package's direct import down
    /// to `identity` (the library this selection is of), `identity` last.
    /// The linking package itself is the implicit first step of every path.
    /// Rebuilt from the dependency closure on each call: a closure retains
    /// one first hop per selection, never a copy of the whole path.
    pub fn path(&self, identity: &LibraryName) -> Vec<LibraryName> {
        let mut path = Vec::new();
        let mut step = &self.path;
        while let DependencyPath::Via { first, package } = step {
            path.push(first.clone());
            match package.selections.get(identity) {
                Some(next) => step = &next.path,
                None => return path,
            }
        }
        path.push(identity.clone());
        path
    }
}

/// How a selection's first dependency path reaches its library.
// Only `link_with` builds a selection, so without `std` no variant is
// constructed; the type stays because `ResolvedDependency` is public.
#[cfg_attr(not(feature = "std"), expect(dead_code))]
#[derive(Clone)]
enum DependencyPath {
    /// The linking package imports the library itself.
    Direct,
    /// The linking package's import `first` reaches the library through
    /// `package`'s own selection of it.
    Via {
        first: LibraryName,
        package: Arc<CheckedPackage>,
    },
}

/// Two paths are equal when they leave through the same first hop: the rest
/// of each path is the equal selection of the package that hop imports,
/// compared with that package.
impl PartialEq for DependencyPath {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Direct, Self::Direct) => true,
            (Self::Via { first, .. }, Self::Via { first: other, .. }) => first == other,
            _ => false,
        }
    }
}

impl Eq for DependencyPath {}

impl core::fmt::Debug for DependencyPath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Direct => f.write_str("Direct"),
            Self::Via { first, .. } => f.debug_struct("Via").field("first", first).finish(),
        }
    }
}

/// One import the E4 link step binds (ADR-011 §4 dependency binding): the
/// library identity `L` an `import "L"` names and the dependency's checked
/// package compiled from its source.
#[cfg(feature = "std")]
#[derive(Clone, Debug)]
pub struct Import {
    /// Library identity `L` (ADR-015 D-3).
    pub identity: LibraryName,
    /// The dependency's checked package, compiled from source through S1 to
    /// S4 (ADR-015 D-1).
    pub package: Arc<CheckedPackage>,
}

/// Why the E4 link step built no package (ADR-011 §2.3: no partial output).
#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
pub enum LinkRefusal {
    /// An import names a different library than the emitted package declares.
    #[error("import {expected} resolves to package {actual}")]
    IdentityMismatch {
        /// The imported library name.
        expected: LibraryName,
        /// The package's own manifest name.
        actual: Box<str>,
    },
    /// FR-307 diamond rule: two dependency paths select one library
    /// identity with different canonical package bytes
    /// (`invalid_package`/`conflicting-definition`), listing both paths.
    #[error(
        "invalid_package/conflicting-definition: {} and {} select {identity} differently",
        display_path(&.selections[0].path(.identity)),
        display_path(&.selections[1].path(.identity))
    )]
    ConflictingDefinition {
        /// The library identity.
        identity: LibraryName,
        /// The earlier selection and its path, then the conflicting one.
        selections: Box<[ResolvedDependency; 2]>,
    },
    /// A dependency's complete v2 bytes could not be emitted: emission
    /// refused, or would omit nodes (ADR-011 §2.3 partial output).
    #[error("the dependency {identity} does not emit a complete package")]
    DependencyEmission {
        /// The library identity.
        identity: LibraryName,
        /// The emitter's refusal, or `None` when the emission omits nodes.
        refusal: Option<EmitRefusal>,
    },
}

#[cfg(feature = "std")]
impl LinkRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::ConflictingDefinition { .. } | Self::IdentityMismatch { .. } => {
                Code::InvalidPackage
            }
            Self::DependencyEmission {
                refusal: Some(refusal),
                ..
            } => refusal.code(),
            Self::DependencyEmission { refusal: None, .. } => Code::UnsupportedProjection,
        }
    }

    /// The catalog cause, where the catalog names one.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::ConflictingDefinition { .. } => Some("conflicting-definition"),
            Self::IdentityMismatch { .. } => None,
            Self::DependencyEmission { .. } => None,
        }
    }
}

/// Drops the dependency closure over an explicit stack, so a chain of any
/// length drops on a constant native stack (ADR-030 D-1): the derived drop
/// would recurse once per link.
/// With `std`, Debug writes the graph and every dependency's graph under one
/// node table, so a node is written once for the whole package. Without `std`,
/// keys are written in full.
impl core::fmt::Debug for CheckedPackage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct Body<'a>(&'a CheckedPackage);
        impl core::fmt::Debug for Body<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let CheckedPackage {
                    graph,
                    dependencies,
                    selections,
                    #[cfg(feature = "std")]
                    direct_imports,
                    abstraction,
                    #[cfg(feature = "std")]
                    emission,
                } = self.0;
                let mut out = f.debug_struct("CheckedPackage");
                out.field("graph", graph)
                    .field("dependencies", dependencies)
                    .field("selections", selections);
                #[cfg(feature = "std")]
                out.field("direct_imports", direct_imports);
                out.field("abstraction", abstraction);
                #[cfg(feature = "std")]
                out.field("emission", emission);
                out.finish()
            }
        }
        qsl_foundation::source::node_table::scoped(f, &Body(self))
    }
}

impl Drop for CheckedPackage {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        self.release_into(&mut pending);
        while let Some(package) = pending.pop() {
            if let Ok(mut owned) = Arc::try_unwrap(package) {
                owned.release_into(&mut pending);
            }
        }
    }
}

impl CheckedPackage {
    /// Moves every package this one holds, as a dependency or as a selection's
    /// path hop, into `pending` for [`Drop`]'s explicit stack.
    fn release_into(&mut self, pending: &mut Vec<Arc<CheckedPackage>>) {
        pending.extend(core::mem::take(&mut self.dependencies).into_values());
        for resolved in core::mem::take(&mut self.selections).into_values() {
            if let DependencyPath::Via { package, .. } = resolved.path {
                pending.push(package);
            }
        }
    }

    /// The S4 link step (ADR-013 T-1): the sole conversion from
    /// `CheckedGraph` to `CheckedPackage`. Fed only by already-checked
    /// typestate -- a `CheckedGraph` -- never by an unchecked or
    /// wire-admitted value (R-10). The dependency closure is empty;
    /// [`Self::link_with`] is the dependency-bearing step.
    ///
    /// TC-163 (FR-065-AC-1): the function packaging/lowering
    /// public API accepts only a checked node -- built solely through the
    /// `crate::family::FamilyContract::check` hook and carried this far as
    /// a `CheckedGraph` -- never a raw S1 CST node or a raw source string
    /// it would have to derive function semantics from itself. Neither
    /// converts into `CheckedPackage`:
    /// ```compile_fail,E0277
    /// use qsl_package::CheckedPackage;
    /// fn forge(cst: qsl_cst::ParsedSource) -> CheckedPackage {
    ///     cst.into()
    /// }
    /// ```
    /// ```compile_fail,E0277
    /// use qsl_package::CheckedPackage;
    /// fn forge(source: String) -> CheckedPackage {
    ///     source.into()
    /// }
    /// ```
    /// The same restriction stated the other way: `link` itself takes no
    /// type but `CheckedGraph`, so calling it directly with a raw CST node
    /// or a raw source string also fails to compile, not only the `.into()`
    /// spelling above.
    /// ```compile_fail,E0308
    /// use qsl_package::CheckedPackage;
    /// fn forge(cst: qsl_cst::ParsedSource) -> CheckedPackage {
    ///     CheckedPackage::link(cst)
    /// }
    /// ```
    /// ```compile_fail,E0308
    /// use qsl_package::CheckedPackage;
    /// fn forge(source: String) -> CheckedPackage {
    ///     CheckedPackage::link(source)
    /// }
    /// ```
    /// Their shared pair (this file's own doc, above, states why every
    /// `compile_fail` block here is paired): a real checked node, run
    /// through `check` and linked, does compile and succeed.
    /// ```no_run
    /// use qsl_package::CheckedPackage;
    /// use qsl_semantics::check::PackageDeclarations;
    /// use quire_semantic_value::checking::CheckingLimits;
    /// fn build(
    ///     source: qsl_foundation::source::provenance::RawSourceRef,
    /// ) -> CheckedPackage {
    ///     let graph = PackageDeclarations::new(source, "test/package", qsl_foundation::IdentityLimits::default())
    ///         .check(CheckingLimits::default())
    ///         .expect("an empty package checks cleanly");
    ///     CheckedPackage::link(graph)
    /// }
    /// ```
    pub fn link(graph: CheckedGraph) -> Self {
        Self {
            abstraction: graph.abstraction().cloned(),
            graph: Arc::new(graph),
            dependencies: BTreeMap::new(),
            selections: BTreeMap::new(),
            #[cfg(feature = "std")]
            direct_imports: BTreeSet::new(),
            #[cfg(feature = "std")]
            emission: OnceLock::new(),
        }
    }

    /// The E4 link step with a dependency closure (ADR-011 §2.1 E4, §4
    /// dependency binding): `graph` linked with each import's checked
    /// package. The closure takes the imports and every dependency's own
    /// closure; two selections of one library name unify only when their
    /// canonical package bytes are equal (FR-307), else
    /// [`LinkRefusal::ConflictingDefinition`].
    /// A refusal yields no package.
    #[cfg(feature = "std")]
    pub fn link_with(graph: CheckedGraph, imports: Vec<Import>) -> Result<Self, LinkRefusal> {
        let mut dependencies: BTreeMap<LibraryName, Arc<CheckedPackage>> = BTreeMap::new();
        let mut selections: BTreeMap<LibraryName, ResolvedDependency> = BTreeMap::new();
        let direct_imports = imports
            .iter()
            .map(|import| import.identity.clone())
            .collect();
        for import in imports {
            check_dependency_emission(&import)?;
            let direct = ResolvedDependency {
                package: Arc::clone(&import.package),
                path: DependencyPath::Direct,
            };
            let transitive = import
                .package
                .selections
                .iter()
                .map(|(identity, resolved)| {
                    (
                        identity.clone(),
                        ResolvedDependency {
                            package: Arc::clone(&resolved.package),
                            path: DependencyPath::Via {
                                first: import.identity.clone(),
                                package: Arc::clone(&import.package),
                            },
                        },
                    )
                });
            let entries: Vec<(LibraryName, ResolvedDependency)> =
                core::iter::once((import.identity.clone(), direct))
                    .chain(transitive)
                    .collect();
            for (identity, resolved) in entries {
                unify(&mut selections, identity, resolved)?;
            }
            match dependencies.get(&import.identity) {
                Some(existing) if !same_package(existing, &import.package, &import.identity)? => {
                    return Err(LinkRefusal::ConflictingDefinition {
                        identity: import.identity.clone(),
                        selections: Box::new([
                            ResolvedDependency {
                                package: Arc::clone(existing),
                                path: DependencyPath::Direct,
                            },
                            ResolvedDependency {
                                package: Arc::clone(&import.package),
                                path: DependencyPath::Direct,
                            },
                        ]),
                    });
                }
                _ => {}
            }
            dependencies.insert(import.identity, import.package);
        }
        Ok(Self {
            abstraction: graph.abstraction().cloned(),
            graph: Arc::new(graph),
            dependencies,
            selections,
            direct_imports,
            emission: OnceLock::new(),
        })
    }

    /// FR-304: gives this package the abstraction relation `bindings` check
    /// to against its own declarations. The relation is built only by S3's
    /// checker.
    ///
    /// # Errors
    ///
    /// Every refusal of the bindings; a refusal leaves the package unchanged.
    pub fn bind_abstraction(
        &mut self,
        bindings: &[AuthoredBinding],
    ) -> Result<(), Vec<AbstractionRefusal>> {
        self.abstraction = Some(self.graph.check_abstraction(bindings)?);
        #[cfg(feature = "std")]
        {
            self.emission = OnceLock::new();
        }
        Ok(())
    }

    /// As [`Self::bind_abstraction`], for a relation read from the
    /// `abstraction` declarations located at `sites`, one per declaration:
    /// the emitter writes one `declaration` occurrence of the relation node at
    /// each (QSpec FR-451).
    ///
    /// # Errors
    ///
    /// Every refusal of the bindings; a refusal leaves the package unchanged.
    pub fn bind_abstraction_at(
        &mut self,
        bindings: &[AuthoredBinding],
        sites: Vec<quire_semantic_value::location::Location>,
    ) -> Result<(), Vec<AbstractionRefusal>> {
        let relation = self.graph.check_abstraction(bindings)?;
        self.abstraction = Some(relation.with_sites(sites));
        Ok(())
    }

    /// FR-304: this package's checked abstraction relation, when it declares
    /// one.
    pub fn abstraction(&self) -> Option<&CheckedAbstractionRelation> {
        self.abstraction.as_ref()
    }

    /// Records `emission`, the emitter's output for this package. Only the
    /// emitter calls this (ADR-013 O-02).
    #[cfg(feature = "std")]
    pub(crate) fn remember_emission(&self, emission: &Emission) {
        let _ = self.emission.set(emission.clone());
    }

    /// This package's emission, if it has been emitted.
    #[cfg(feature = "std")]
    pub(crate) fn remembered_emission(&self) -> Option<&Emission> {
        self.emission.get()
    }

    /// The name the emitter used for this package, if it was
    /// emitted through `emit_checked` or `emit_checked_with_cancel`, which
    /// record the emission: the identity Contract IR's admission of those
    /// bytes declares. A package emitted only through `emit_package` has
    /// none.
    #[cfg(feature = "std")]
    pub fn emitted_package_name(&self) -> Option<&str> {
        self.emission.get().map(|emission| emission.package.name())
    }

    /// This package's own checked declarations (S3's stage output) -- the
    /// delegation point FR-087-AC-9/TC-256 names: every check-owned-type
    /// accessor a consumer needs is reached through this method, not by
    /// `qsl-package` re-declaring or re-importing `check`'s types itself.
    pub fn graph(&self) -> &CheckedGraph {
        &self.graph
    }

    /// The in-process evaluation view tied to this linked package.
    pub fn evaluation_view(&self) -> crate::evaluation_view::AdmittedEvaluationView<'_> {
        crate::evaluation_view::AdmittedEvaluationView::in_process(self)
    }

    /// This package's checked graph, shared: an importing package's E3
    /// types an imported name from it (ADR-015 D-5).
    pub fn shared_graph(&self) -> Arc<CheckedGraph> {
        Arc::clone(&self.graph)
    }

    /// The checked dependency closure (E4): each imported identity's own
    /// checked package.
    pub fn dependencies(&self) -> &BTreeMap<LibraryName, Arc<CheckedPackage>> {
        &self.dependencies
    }

    /// The resolved library closure: one selection per reachable library
    /// identity, in ascending UTF-8 byte order, with the first path that
    /// reached it. The FR-322 lock writes only this unit's direct imports.
    pub fn dependency_selections(&self) -> &BTreeMap<LibraryName, ResolvedDependency> {
        &self.selections
    }

    /// Direct imports only, for the FR-322 lock and its reader evidence.
    /// Directness comes from the authored imports, independent of which
    /// equal-package path first populated the resolved closure.
    #[cfg(feature = "std")]
    pub(crate) fn direct_dependency_selections(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&LibraryName, &ResolvedDependency)> {
        self.selections
            .iter()
            .filter(|(identity, _)| self.direct_imports.contains(*identity))
    }
}

/// Check that the imported package emits complete bytes under the imported
/// library name. A remembered emission is reused when available.
#[cfg(feature = "std")]
fn check_dependency_emission(import: &Import) -> Result<(), LinkRefusal> {
    let fresh;
    let emission = match import.package.remembered_emission() {
        Some(emission) => emission,
        None => {
            fresh = emit_checked(&import.package).map_err(|refusal| {
                LinkRefusal::DependencyEmission {
                    identity: import.identity.clone(),
                    refusal: Some(refusal),
                }
            })?;
            &fresh
        }
    };
    if !emission.omitted().is_empty() {
        return Err(LinkRefusal::DependencyEmission {
            identity: import.identity.clone(),
            refusal: None,
        });
    }
    if emission.package().name() != import.identity.as_str() {
        return Err(LinkRefusal::IdentityMismatch {
            expected: import.identity.clone(),
            actual: emission.package().name().into(),
        });
    }
    Ok(())
}

#[cfg(feature = "std")]
fn same_package(
    left: &CheckedPackage,
    right: &CheckedPackage,
    identity: &LibraryName,
) -> Result<bool, LinkRefusal> {
    if core::ptr::eq(left, right) {
        return Ok(true);
    }
    let emit = |package: &CheckedPackage| {
        emit_checked(package).map_err(|refusal| LinkRefusal::DependencyEmission {
            identity: identity.clone(),
            refusal: Some(refusal),
        })
    };
    let left = emit(left)?;
    let right = emit(right)?;
    Ok(left.package().bytes() == right.package().bytes())
}

/// A dependency path written `a -> b -> c`.
#[cfg(feature = "std")]
fn display_path(path: &[LibraryName]) -> String {
    path.iter()
        .map(LibraryName::as_str)
        .collect::<Vec<_>>()
        .join(" -> ")
}

/// Add `resolved` of `identity` to `selections`, unifying an equal
/// selection reached by another path (FR-307 diamond rule).
#[cfg(feature = "std")]
fn unify(
    selections: &mut BTreeMap<LibraryName, ResolvedDependency>,
    identity: LibraryName,
    resolved: ResolvedDependency,
) -> Result<(), LinkRefusal> {
    match selections.get(&identity) {
        Some(existing) if same_package(&existing.package, &resolved.package, &identity)? => Ok(()),
        Some(existing) => Err(LinkRefusal::ConflictingDefinition {
            selections: Box::new([existing.clone(), resolved]),
            identity,
        }),
        None => {
            selections.insert(identity, resolved);
            Ok(())
        }
    }
}

/// S4 wire output (ADR-013 T-1): canonical checked-package/v2 bytes and
/// the manifest package name. Distinct from `protocol_artifact::native::EmittedPackage`
/// (SEAM-3, unrelated: FR-087-AC-8/TC-247) -- two separate types under two
/// separate module paths, with no shared field, method or re-export, and
/// with no `pub use`/glob anywhere that would place both names in one import
/// scope.
///
/// A caller cannot supply a forged name or bytes: both fields are private,
/// and this type's own (`pub(super)`) constructor is crate-local --
/// naming this struct's private fields directly from outside `qsl-package`
/// (ADR-013 O-02) does not compile:
/// ```compile_fail,E0451
/// use qsl_package::EmittedPackage;
/// let forged = EmittedPackage {
///     bytes: Vec::new(),
///     name: todo!(),
/// };
/// ```
///
/// quire:canonical
#[cfg(feature = "std")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmittedPackage {
    bytes: Vec<u8>,
    name: Box<str>,
}

#[cfg(feature = "std")]
impl EmittedPackage {
    /// The v2 emitter's crate-local constructor (ADR-011 T-8, M-4).
    /// The encoder writes canonical bytes under `name`; an encoding refusal
    /// is returned as `E`. External callers cannot construct this type.
    pub(super) fn new<E>(
        name: &str,
        encode: impl FnOnce(&str) -> Result<Vec<u8>, E>,
    ) -> Result<Self, E> {
        Ok(Self {
            bytes: encode(name)?,
            name: name.into(),
        })
    }

    /// The emitted `quire.checked-package/v2` bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The package name these bytes declare.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A chain of `length` packages over one empty checked graph, each
/// depending on the next under a distinct library name: the shape a
/// `length`-long library import chain links to, for the stack-depth tests.
#[cfg(test)]
#[cfg(feature = "std")]
pub(crate) fn dependency_chain(length: usize) -> CheckedPackage {
    use qsl_semantics::check::PackageDeclarations;
    use quire_semantic_value::checking::CheckingLimits;
    let source = qsl_foundation::Source::read(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "u".to_owned(),
        b"",
        0,
    )
    .expect("an empty source reads")
    .reference()
    .clone();
    let graph = Arc::new(
        PackageDeclarations::new(
            source,
            "test/package",
            qsl_foundation::IdentityLimits::default(),
        )
        .check(CheckingLimits::default())
        .expect("an empty package checks"),
    );
    (0..length).fold(
        CheckedPackage {
            graph: Arc::clone(&graph),
            dependencies: BTreeMap::new(),
            selections: BTreeMap::new(),
            direct_imports: BTreeSet::new(),
            abstraction: None,
            emission: OnceLock::new(),
        },
        |dependency, link| CheckedPackage {
            graph: Arc::clone(&graph),
            dependencies: BTreeMap::from([(
                LibraryName::new(format!("test/chain-{link}")).expect("a library identity"),
                Arc::new(dependency),
            )]),
            selections: BTreeMap::new(),
            direct_imports: BTreeSet::new(),
            abstraction: None,
            emission: OnceLock::new(),
        },
    )
}

/// As [`dependency_chain`], but every package also carries a selection of the
/// package below it, whose path hop holds that package.
#[cfg(test)]
#[cfg(feature = "std")]
pub(crate) fn dependency_chain_with_selections(length: usize) -> CheckedPackage {
    let mut chain = dependency_chain(0);
    for link in 0..length {
        let below = Arc::new(chain);
        let mut package = dependency_chain(0);
        let identity = LibraryName::new(format!("test/chain-{link}")).expect("a library identity");
        package.dependencies = BTreeMap::from([(identity.clone(), Arc::clone(&below))]);
        package.selections = BTreeMap::from([(
            identity.clone(),
            ResolvedDependency {
                package: Arc::clone(&below),
                path: DependencyPath::Via {
                    first: identity,
                    package: below,
                },
            },
        )]);
        chain = package;
    }
    chain
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_semantics::check::PackageDeclarations;
    use quire_semantic_value::checking::CheckingLimits;

    /// ADR-030: dropping a 100,000-long dependency chain runs on a 128 KiB
    /// stack. A derived, recursive drop would use one frame per link.
    #[trace("FR-099-AC-7", "TC-446")]
    #[test]
    fn a_dependency_chain_of_any_length_drops_on_a_small_stack() {
        let chain = dependency_chain(100_000);
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(move || drop(chain))
            .expect("spawn a 128 KiB thread")
            .join()
            .expect("the drop must not overflow a 128 KiB stack");
    }

    /// ADR-030: the packages a selection's path hop holds drop on the same
    /// explicit stack; without that release the chain drops recursively
    /// through the selections.
    #[trace("FR-099-AC-7", "TC-446")]
    #[test]
    fn a_chain_of_selection_hops_of_any_length_drops_on_a_small_stack() {
        let chain = dependency_chain_with_selections(100_000);
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(move || drop(chain))
            .expect("spawn a 128 KiB thread")
            .join()
            .expect("the drop must not overflow a 128 KiB stack");
    }

    /// FR-094 (TC-417), through the layer-4 `CheckedPackage`: `check` is
    /// the model correspondence's only writer. A function over a
    /// `Reference<M::A>` parameter keys `model.A`'s model node and records
    /// it, read back through `CheckedPackage::graph().resolve_declaration`
    /// (FR-088-AC-2, ADR-013 O-04: "Consumers read the correspondence from
    /// the `CheckedPackage`"). It lives here, in layer-4 `qsl-package`: a
    /// layer-3 test cannot name `CheckedPackage` (X-6a).
    #[trace("TC-248", "FR-088-AC-2", "TC-417", "FR-094-AC-2")]
    #[test]
    fn model_correspondence_is_recorded_by_a_real_check_run() {
        use qsl_semantics::check::{AdmittedModel, NodeTag};
        use qsl_semantics::model::accounting::ModelNormalizationLimits;
        use qsl_semantics::model::domain_package::{
            DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
        };
        use qsl_semantics::model::normalize::{normalize, NormalizeOutcome};
        use quire_checked::key::DeclarationKey;
        use quire_semantic_value::declaration::{ObjectTypeDeclaration, TypeEnvironment};

        let declaration = DeclarationKey::fixture("model.A");
        let domain_package = DomainPackage::new(
            DomainPackageRef::fixture("bundle.n01"),
            vec![DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: declaration.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: Vec::new(),
            })],
        );
        let NormalizeOutcome::Completed(view) =
            normalize(&domain_package, ModelNormalizationLimits::UNLIMITED)
        else {
            panic!("the one-type domain package normalizes");
        };
        let a = view.type_identities()[&declaration].clone();
        let span = qsl_foundation::Span { start: 0, end: 0 };
        let graph = PackageDeclarations {
            types: TypeEnvironment::new([], [ObjectTypeDeclaration::new(a, "M::A", vec![])])
                .expect("one object type admits"),
            models: vec![AdmittedModel::new(&domain_package, &view).expect("the view is its own")],
            functions: vec![qsl_forms::FunctionDeclaration::new(
                "f",
                vec![("r".to_owned(), qsl_forms::TypeForm::name("M::A", span))],
                qsl_forms::TypeForm::builtin(qsl_forms::BuiltinType::Boolean, span),
                None,
                qsl_forms::Expression::boolean(true),
            )],
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                "test/package",
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .expect("a function over a model reference checks");
        let model_nodes: Vec<quire_exact::NodeKey> = graph
            .semantic_graph()
            .nodes()
            .filter(|node| node.node_tag() == NodeTag::Model)
            .map(|node| node.key())
            .collect();
        assert_eq!(model_nodes.len(), 1, "one model node: model.A's");
        let package = CheckedPackage::link(graph);
        assert_eq!(
            package.graph().resolve_declaration(model_nodes[0].clone()),
            Some(&declaration)
        );

        // Adverse (R-05): a node `check` never keyed resolves to nothing.
        let other = quire_exact::NodeKey::new(
            "test/package",
            vec![quire_exact::PathSegment::Text("missing".into())],
            0,
        )
        .expect("a structural node key");
        assert_eq!(package.graph().resolve_declaration(other), None);
    }

    /// The emitted name comes from the required package manifest name.
    #[trace("TC-253", "FR-087-AC-3")]
    #[test]
    fn emitted_package_retains_declared_name() {
        let emitted = EmittedPackage::new("test/package", |_| {
            Ok::<_, core::convert::Infallible>(vec![1, 2, 3])
        })
        .expect("the encoder is infallible");
        assert_eq!(emitted.name(), "test/package");
        assert_eq!(emitted.bytes(), [1, 2, 3]);
    }
}
