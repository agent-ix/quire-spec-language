// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facade's admitted-context entry (FR-394): a compiled unit's emitted
//! package together with the admitted enum key table its check retained.
//!
//! ADR-011 T12, FR-087 and Runtime AD-004 forbid a consumer to build a node
//! key. A consumer that needs the key of an enum declaration or case compiles
//! through [`compile_admitted`] and looks the key up in the
//! [`AdmittedContext`] it returns, by the wire node id the emitted package
//! carries. The context holds the keys the checker minted and nothing else,
//! so a lookup never creates a key; it finds one or refuses.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::{Code, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{NominalNode, SemanticGraph};
use qsl_semantics::library::PackageId;
use quire_exact::{Identifier, NodeKey};

use crate::bounds::ReplayLimits;
use crate::compile::{recompile_unit, CompiledPackage};
use crate::execute::{LimitAboveReader, ReplayRefusal};
use crate::request::StageLimits;
use crate::spine::{CompileRefusal, DependencyInput};

/// One case of an admitted enum: its identifier and the key the checker
/// minted for its member node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedEnumCase {
    case: Identifier,
    key: NodeKey,
}

impl AdmittedEnumCase {
    /// The case identifier.
    pub fn case(&self) -> &Identifier {
        &self.case
    }

    /// The admitted key of the case's member node.
    pub fn key(&self) -> NodeKey {
        self.key
    }
}

/// One admitted enum declaration: its qualified name, ordering and key, and
/// every one of its cases, used or not, in FR-141 canonical order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedEnum {
    key: NodeKey,
    qualified_name: Vec<Identifier>,
    ordered: bool,
    cases: Vec<AdmittedEnumCase>,
}

impl AdmittedEnum {
    /// The admitted key of the declaration node.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The declaration's qualified name.
    pub fn qualified_name(&self) -> &[Identifier] {
        &self.qualified_name
    }

    /// Whether the enum is ordered.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// Every case, in canonical order.
    pub fn cases(&self) -> &[AdmittedEnumCase] {
        &self.cases
    }

    /// The case named `case`.
    pub fn case(&self, case: &Identifier) -> Option<&AdmittedEnumCase> {
        self.cases.iter().find(|admitted| admitted.case == *case)
    }
}

/// Why an [`AdmittedContext`] lookup found no key.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AdmittedLookupRefusal {
    /// The declaration id names no enum declaration of this package or of
    /// any package it depends on, or the declaration has no such case.
    #[error("unknown_declaration: no admitted enum declaration {declaration}{}", case_suffix(.case))]
    Unknown {
        /// The declaration id looked up.
        declaration: WireNodeId,
        /// The case looked up, when the lookup named one and the
        /// declaration exists.
        case: Option<Identifier>,
    },
    /// The declaration id names an enum that a dependency of this package
    /// admitted, not this package. Its keys are that package's.
    #[error("foreign_declaration: enum declaration {declaration} is admitted by dependency package {}", .package.hex())]
    Foreign {
        /// The declaration id looked up.
        declaration: WireNodeId,
        /// The dependency package that admitted it.
        package: PackageId,
    },
}

fn case_suffix(case: &Option<Identifier>) -> String {
    case.as_ref()
        .map_or_else(String::new, |case| format!(" case {}", case.as_str()))
}

/// The package context a compile admitted: this package's enum key table and
/// the identity of every enum a dependency admitted.
#[derive(Clone, Debug)]
pub struct AdmittedContext {
    package: PackageId,
    enums: BTreeMap<[u8; 32], AdmittedEnum>,
    foreign: BTreeMap<[u8; 32], PackageId>,
}

impl AdmittedContext {
    /// The `package_id` of the package whose check admitted this context.
    pub fn package_id(&self) -> PackageId {
        self.package
    }

    /// Every enum declaration this package admitted, ascending by key.
    pub fn enums(&self) -> impl Iterator<Item = &AdmittedEnum> {
        self.enums.values()
    }

    /// The enum declaration the wire node id `declaration` names.
    ///
    /// # Errors
    ///
    /// [`AdmittedLookupRefusal::Foreign`] when a dependency admitted it,
    /// [`AdmittedLookupRefusal::Unknown`] when no package of the closure did.
    pub fn enum_declaration(
        &self,
        declaration: WireNodeId,
    ) -> Result<&AdmittedEnum, AdmittedLookupRefusal> {
        let id = *declaration.as_bytes();
        if let Some(admitted) = self.enums.get(&id) {
            return Ok(admitted);
        }
        Err(match self.foreign.get(&id) {
            Some(&package) => AdmittedLookupRefusal::Foreign {
                declaration,
                package,
            },
            None => AdmittedLookupRefusal::Unknown {
                declaration,
                case: None,
            },
        })
    }

    /// The case `case` of the enum declaration `declaration` names.
    ///
    /// # Errors
    ///
    /// As [`Self::enum_declaration`]; and [`AdmittedLookupRefusal::Unknown`]
    /// naming the case when the declaration has no such case.
    pub fn enum_case(
        &self,
        declaration: WireNodeId,
        case: &Identifier,
    ) -> Result<&AdmittedEnumCase, AdmittedLookupRefusal> {
        self.enum_declaration(declaration)?
            .case(case)
            .ok_or_else(|| AdmittedLookupRefusal::Unknown {
                declaration,
                case: Some(case.clone()),
            })
    }
}

/// A unit compiled and emitted, with the context its check admitted.
#[derive(Clone, Debug)]
pub struct AdmittedCompilation {
    package: CompiledPackage,
    context: AdmittedContext,
}

impl AdmittedCompilation {
    /// The emitted package.
    pub fn package(&self) -> &CompiledPackage {
        &self.package
    }

    /// The context the unit's check admitted.
    pub fn context(&self) -> &AdmittedContext {
        &self.context
    }

    /// The emitted package and the admitted context.
    pub fn into_parts(self) -> (CompiledPackage, AdmittedContext) {
        (self.package, self.context)
    }
}

/// A bound that stopped [`compile_admitted`].
#[derive(Debug, thiserror::Error)]
pub enum AdmittedResource {
    /// A stage limit the compile reached (`stage_limit_exceeded`).
    #[error("{0}")]
    StageLimit(Box<CompileRefusal>),
    /// The S1 limit asked for is above the reader limit.
    #[error("invalid-request: the `s1.input_bytes` limit {} is above the reader limit `replay.input_bytes` {}", .0.requested, .0.reader)]
    AboveReader(LimitAboveReader),
}

/// Why [`compile_admitted`] returned no context.
#[derive(Debug, thiserror::Error)]
pub enum AdmittedRefusal {
    /// The unit, a library or the dependency input does not compile to a
    /// package.
    #[error("the unit does not compile to a package: {0}")]
    Package(Box<CompileRefusal>),
    /// A bound stopped the compile.
    #[error(transparent)]
    Resource(AdmittedResource),
    /// A QSL invariant the compile relies on did not hold.
    #[error("internal fault: {0:?}")]
    Fault(InternalFault),
}

impl From<ReplayRefusal> for AdmittedRefusal {
    fn from(refusal: ReplayRefusal) -> Self {
        match refusal {
            ReplayRefusal::Recompile(refusal) => classify(*refusal),
            ReplayRefusal::DependencyInput(refusal) => {
                classify(CompileRefusal::DependencyInput(refusal))
            }
            ReplayRefusal::LimitAboveReader(limit) => {
                Self::Resource(AdmittedResource::AboveReader(limit))
            }
            ReplayRefusal::Fault(fault) => Self::Fault(fault),
            // The compile path raises no other refusal; a replay-only one
            // here is a broken invariant, not a caller error.
            _ => Self::Fault(InternalFault::new(
                "replay",
                "compile-refuses-only-compile-refusals",
            )),
        }
    }
}

fn classify(refusal: CompileRefusal) -> AdmittedRefusal {
    if matches!(
        refusal.code(),
        Code::StageLimitExceeded | Code::ResourceExhausted
    ) {
        AdmittedRefusal::Resource(AdmittedResource::StageLimit(Box::new(refusal)))
    } else {
        AdmittedRefusal::Package(Box::new(refusal))
    }
}

/// Compile `bytes` as [`crate::compile_package`] does and return the emitted
/// package together with the [`AdmittedContext`] its check admitted, the
/// package-bound enum key table a contract generator looks keys up in
/// instead of building them (FR-394).
///
/// The arguments and limits are [`crate::compile_package`]'s.
///
/// # Errors
///
/// [`AdmittedRefusal::Package`] for a unit, library or dependency input that
/// does not compile, [`AdmittedRefusal::Resource`] for a stage limit reached
/// or an S1 limit above the reader limit, and [`AdmittedRefusal::Fault`] for
/// a broken QSL invariant.
pub fn compile_admitted<'a>(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    limits: StageLimits,
    replay_limits: ReplayLimits,
) -> Result<AdmittedCompilation, AdmittedRefusal> {
    let compiled = recompile_unit(
        source,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        replay_limits,
    )?;
    let context = context_of(
        compiled.checked.package(),
        compiled.emitted.package().package_id(),
    )
    .map_err(AdmittedRefusal::Fault)?;
    Ok(AdmittedCompilation {
        package: CompiledPackage::new(compiled.emitted),
        context,
    })
}

fn context_of(package: &CheckedPackage, id: PackageId) -> Result<AdmittedContext, InternalFault> {
    let enums = enum_table(package.graph().semantic_graph())?;
    let mut foreign = BTreeMap::new();
    let mut visited = BTreeSet::new();
    let mut pending: Vec<_> = package.dependencies().iter().collect();
    while let Some((&dependency_id, dependency)) = pending.pop() {
        if !visited.insert(dependency_id) {
            continue;
        }
        for declaration in enum_table(dependency.graph().semantic_graph())?.into_keys() {
            foreign.entry(declaration).or_insert(dependency_id);
        }
        pending.extend(dependency.dependencies().iter());
    }
    Ok(AdmittedContext {
        package: id,
        enums,
        foreign,
    })
}

/// Every enum declaration of `graph`, keyed by the declaration node's id
/// bytes, with each case's member key read from the member node the checker
/// built for it.
fn enum_table(graph: &SemanticGraph) -> Result<BTreeMap<[u8; 32], AdmittedEnum>, InternalFault> {
    let mut members = BTreeMap::new();
    for node in graph.nodes() {
        if let Some(NominalNode::EnumMember { declaration, case }) = node.nominal() {
            members.insert((*declaration, case.as_str()), node.key());
        }
    }
    let mut enums = BTreeMap::new();
    for node in graph.nodes() {
        let Some(NominalNode::EnumDeclaration(preimage)) = node.nominal() else {
            continue;
        };
        let identifier = |text: &str| {
            Identifier::new(text)
                .map_err(|_| InternalFault::new("replay", "admitted-enum-names-are-identifiers"))
        };
        let cases = preimage
            .members()
            .iter()
            .map(|case| {
                let key = members
                    .get(&(node.key(), case.as_str()))
                    .copied()
                    .ok_or_else(|| {
                        InternalFault::new("replay", "every-admitted-enum-case-has-a-member-node")
                    })?;
                Ok(AdmittedEnumCase {
                    case: identifier(case)?,
                    key,
                })
            })
            .collect::<Result<Vec<_>, InternalFault>>()?;
        let qualified_name = preimage
            .qualified_declaration()
            .iter()
            .map(|name| identifier(name))
            .collect::<Result<Vec<_>, _>>()?;
        enums.insert(
            *node.key().as_bytes(),
            AdmittedEnum {
                key: node.key(),
                qualified_name,
                ordered: preimage.is_ordered(),
                cases,
            },
        );
    }
    Ok(enums)
}
