// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §7.3 M-5 (QSL-139/FR-068): the layer-3 checking half of what was
//! `value::expression`. This module owns name resolution, typing, static
//! definedness and termination checking, and the checked-output types
//! ([`CheckedGraph`], [`CheckedExpression`], `CheckedFunction`) whose
//! constructors are private here (ADR-011 §4's private-constructor/
//! public-accessor mechanism). The evaluation half -- `evaluate.rs`,
//! `CheckedPackage::call`, `CheckedPackage::evaluate` and the
//! argument-admission logic -- stays at layer 5 in
//! `crate::value::expression` (a private module, not resolvable as an
//! intra-doc link) (S6a), reaching this module's state only
//! through the accessor methods below, never through a private field: the
//! two modules no longer share private state (US-009).
//!
//! # `CheckedGraph` (S3) versus `CheckedPackage` (S4) (ADR-013 T-1, FR-087,
//! QSL-158 S-3a)
//!
//! This module's own checked-output type is [`CheckedGraph`]: the S3
//! checker's stage output, produced by [`PackageDeclarations::check`].
//! `CheckedPackage` (the S4 in-process link step's output, `CheckedGraph`
//! plus the checked dependency closure) is a *different*, canonical type,
//! defined in layer-4 [`crate::package`], not here -- `check` defines no
//! `CheckedPackage` type, method or field, and imports nothing from
//! `package` (FR-087-AC-9/TC-256): the `CheckedPackage::call`/
//! `CheckedPackage::evaluate` references in this module's own doc comments
//! name `value::expression`'s re-export of `package::CheckedPackage`,
//! reached only through `package`'s own `CheckedGraph`-typed field and its
//! `graph()` accessor.
//!
//! # The interim `model` -> `check` edge is closed (ADR-011 §7.3 M-2, QSL-7)
//!
//! FR-068 (M-5) left an interim `model` -> `check` edge: `model::checked_dispatch.rs`
//! and `model::conformance.rs` imported thirteen names this module defined
//! (`DispatchCandidate`/`DispatchOperation`/`DispatchTable`/
//! `PackageDeclarations` and
//! `established_field_fact`/`Connective`/`Established`/`Location`/`Node`/
//! `NodeKind`/`OrderedKind`/`Origin`/`ProvedInterval`) directly from
//! `crate::check`, not through `crate::value`'s aggregate re-export -- a
//! legible `model` (layer-3-earlier) -> `check` (layer-3-later) reverse
//! edge, forbidden by §6.1's intra-layer-3 order, declared rather than
//! hidden (FR-068-AC-9/FR-068-CON-5; see FR-068's Behavior section, "The
//! interim `model` -> `check` edge," now superseded).
//!
//! **M-2 (QSL-7) closes it.** `model::checked_dispatch` moved to this
//! module's own `checked_dispatch` submodule (a private module, not
//! resolvable as an intra-doc link, matching this crate's own convention;
//! see [`checked_dispatch_operation`] for its re-exported entry point), and
//! `model::conformance::check_field_refinement_obligation` moved to this
//! module's own `field_refinement` submodule (see
//! [`check_field_refinement_obligation`]) -- `check` (layer-3-later)
//! importing from `model` (layer-3-earlier) is the *forward* direction
//! §6.1's order permits, not a reverse edge, so `model_check_edges`
//! (`xtask`'s TC-176 scan) is empty from here on. `grep -rn "use crate::check"
//! src/model/` finds nothing.
//!
//! # `family.rs`
//!
//! **Amended, PR #282 review F4:** FR-068's move surface now names
//! `family.rs` explicitly, alongside `check`, `evaluate`, `facts`, `ir`,
//! `mod` itself, `refusal` and `termination` -- an omission in the
//! requirement's original text, not a deliberate exclusion: `check.rs`'s
//! own `Typer::call` already called `family::mint_call_identity`/
//! `family::DEFAULT_PACKAGE_IDENTITY` directly before this move, so this
//! module carries a `family` submodule with exactly the identity-minting
//! content those existing calls need (see `family`'s own module doc).
//! Without it, `check`'s real import graph would reach back into
//! `value::expression::family`, which FR-068-AC-3 forbids.

// FR-068 itself names both the destination module (`check`) and the moved
// file (`check.rs`, holding `Typer`/`Scope`/`bind_parameters` -- name
// resolution and typing) -- the inception is the spec's own naming, not an
// accidental collision this file introduced.
mod capability;
#[allow(clippy::module_inception)]
mod check;
mod checked_dispatch;
mod facts;
mod family;
mod field_refinement;
mod identity;
mod ir;
mod refusal;
mod termination;

use std::collections::BTreeMap;

use check::{bind_parameters, Signature, Typer};
use facts::{CallSite, Definedness};

use crate::family::FamilyContract;
use crate::forms::{ClauseKind, Expression, FunctionDeclaration};
use crate::value::composite::ValueType;

pub(crate) use check::Scope;
pub(crate) use family::ValueFunctionFamily;
// `mint_declaration_identity`, `OccurrenceMap`, `DEFAULT_PACKAGE_IDENTITY`
// and `SCALAR_LIMITS_UNLIMITED` are consumed only by `value::expression::
// family`'s `#[cfg(test)]` modules (layer 5 depending on layer 3 is
// permitted), so this re-export is itself `#[cfg(test)]`-gated rather than
// plain: a plain `pub(crate) use` here is genuinely unused in a non-test
// build (`cargo check`/`cargo build`/`cargo clippy` without
// `--all-targets`), and `-D warnings` promotes that to a hard compile error
// before cargo ever reaches the test binaries where it would be used --
// gating on `cfg(test)` keeps both builds clean instead of papering over
// the non-test one with `#[allow(unused_imports)]`. `SCALAR_LIMITS_UNLIMITED`
// joined this list in PR #302 review (finding 2): `CheckedPackage::call`'s
// `contract_meter` used to be an unconditionally unlimited `Meter` built
// from it, a real (non-test) production use; it is now built from the
// caller's own configured limits (`*meter.limits()`) instead, so this
// constant has no production reader left.
#[cfg(test)]
pub(crate) use family::{
    mint_declaration_identity, OccurrenceMap, DEFAULT_PACKAGE_IDENTITY, SCALAR_LIMITS_UNLIMITED,
};
// PR #303 review, finding N7b: `empty_scope`/`root_location` used to be
// defined twice -- once here (`check::family`'s own `checking_tests`
// module) and once more, byte-for-byte, in `value::expression::family`'s
// `family_contract_tests` module. Both build the same `Scope`/`Location`
// this crate already defines once; re-exporting the one real definition
// removes the second copy instead of letting it drift.
//
// `declarations_for` joined this list in PR #303 review round 3 (finding
// F6): the same duplication, for a `ValueDeclarations` test fixture, with
// two different parameter shapes.
#[cfg(test)]
pub(crate) use family::checking_tests::{declarations_for, empty_scope, root_location};
pub(crate) use ir::{Arithmetic, Connective, Node, NodeKind, OrderedKind, RecordSlot, Slot, Visit};

pub use capability::{Capability, UnknownCapabilityLabel};
pub use check::{
    CheckingLimits, DepthAboveMaximum, DispatchOperation, EnumBinding, PackageDeclarations,
    MAX_CHECKING_DEPTH,
};
pub use checked_dispatch::{
    checked_dispatch_operation, object_type_supertypes, DispatchBridgeRefusal, DispatchRoot,
    MissingClauseField, OperationClauses,
};
pub use field_refinement::check_field_refinement_obligation;
// PR #300 review finding 4: `mint_type_declaration_identity` and
// `mint_variant_id` are `pub(super)` in `identity` (visible to `check`,
// which calls them from `PackageDeclarations::check` below), not `pub` --
// they are not part of this re-export list, since no consumer outside
// `check` mints an identity.
pub use identity::{
    to_kernel_value_type, CheckedClauseKind, CheckedTypeNode, DuplicateSumVariant, Frame,
    FrameSubjects, ModelCorrespondence, ResolvedFrameSubjects, ScalarShape, SumVariant,
    SumVariants,
};
pub use ir::{CollectionLoss, CollectionProperty, DispatchCandidate, DispatchTable};
pub use refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, DispatchFunctionRole,
    InvalidDispatchDeclaration, Location, MeasureObligation, Obligation, Origin, ProvedInterval,
    WrongSnapshotCause,
};

/// How a standalone expression is checked.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CheckMode {
    /// Linking: typing and every static definedness obligation.
    Linked,
    /// Direct kernel evaluation over supplied values: typing only, so an
    /// empty `reduce` or `value(none)` is a located undefined outcome.
    Kernel,
}

/// A checked function.
#[derive(Debug)]
struct CheckedFunction {
    /// FR-062/FR-065: content-addressed identity, minted once at check from
    /// the declaration's own parsed structure (`family::
    /// mint_declaration_identity`), independent of this function's position
    /// in the package's function list.
    identity: quire_exact::NodeKey,
    signature: Signature,
    body: Node,
    measure: Option<Node>,
    slots: usize,
}

/// S3's stage output (ADR-013 T-1): a package whose every function is
/// admitted. Its constructor and every field are private to this module
/// (ADR-011 §4): `package`'s S4 link step (which builds the *different*
/// canonical `CheckedPackage`, layer-4 `package`) reaches this state only
/// through the accessor methods below, never through a private field or a
/// conversion function (R-10).
///
/// TC-244 row 1 (FR-087-AC-2): an unchecked `ParsedSource` never becomes a
/// `CheckedGraph` by any path other than [`PackageDeclarations::check`] --
/// in particular, not by naming this struct's private fields directly from
/// outside `check`:
/// ```compile_fail,E0451
/// use quire_spec_language::check::CheckedGraph;
/// let forged = CheckedGraph {
///     scope: todo!(),
///     functions: Vec::new(),
///     dispatch_tables: Vec::new(),
///     occurrences: todo!(),
/// };
/// ```
#[derive(Debug)]
pub struct CheckedGraph {
    scope: Scope,
    functions: Vec<CheckedFunction>,
    dispatch_tables: Vec<DispatchTable>,
    /// FR-062-AC-2/FR-065-AC-3: the occurrence-keyed source map (identity,
    /// role, ordinal) -> source [`Location`], for every function
    /// declaration and function-application occurrence this package
    /// checked.
    occurrences: family::OccurrenceMap<Location>,
    /// ADR-013 O-04/FR-088-AC-2: the model correspondence this package's own
    /// checking recorded, read only through [`Self::resolve_declaration`] --
    /// a node-id-keyed accessor, never a name-keyed one (R-06). PR #300
    /// review finding 1: populated from `PackageDeclarations::model_correspondence`
    /// (see that field's own doc for why the entries themselves are still
    /// caller-supplied -- no #213 slice before S-3b gives the checker a real
    /// domain-package intake to derive a frame/model declaration's own
    /// `DeclarationKey` from, FR-088-CON-2 leaves FR-340's frame semantics to
    /// #210), but the *recording* is real production code, not a test-only
    /// stub: `check` copies every caller-supplied entry onto this field
    /// itself, exactly the same "built by the caller, checker only
    /// records/resolves against it" split this struct's own
    /// `dispatch_operations`/`dispatch_tables` already use.
    model_correspondence: identity::ModelCorrespondence,
    /// ADR-013 O-14/C-26 (PR #300 review finding 1): every admitted
    /// composite and enum type declaration this package's `TypeEnvironment`
    /// and `enums` carried, converted once at `check` into a real
    /// [`identity::CheckedTypeNode`] and keyed by its own checked node id --
    /// read only through [`Self::checked_type_node`], node-id-keyed like
    /// every other accessor here (R-06).
    type_nodes: BTreeMap<quire_exact::NodeKey, identity::CheckedTypeNode>,
}

/// A checked standalone expression over named parameters. Its constructor
/// and every field are private to this module (ADR-011 §4).
#[derive(Debug)]
pub struct CheckedExpression {
    parameters: Vec<(String, ValueType)>,
    root: Node,
    slots: usize,
}

impl CheckedExpression {
    /// The result type.
    pub fn value_type(&self) -> &ValueType {
        &self.root.value_type
    }

    /// Every `convert` loss in pre-order.
    pub fn losses(&self) -> Vec<CollectionLoss> {
        self.root.losses()
    }

    /// Every `deref(r).f` location, whose target existence is a runtime input
    /// requirement.
    pub fn dereferences(&self) -> Vec<Location> {
        self.root.dereferences()
    }

    /// The declared parameters, in evaluation-slot order -- the accessor
    /// [`crate::value::expression::CheckedPackage::evaluate`] reads to
    /// validate its caller's arguments (ADR-011 §4: evaluation never reads
    /// this type's fields directly). Crate-internal only: `Node` (see
    /// [`Self::root`]) is `pub(crate)`, so this whole accessor surface stays
    /// no more public than that.
    pub(crate) fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The checked expression tree evaluation runs.
    pub(crate) fn root(&self) -> &Node {
        &self.root
    }

    /// The evaluation slot count evaluation allocates.
    pub(crate) fn slots(&self) -> usize {
        self.slots
    }
}

/// One admitted function's evaluation-visible state: exactly what
/// `value::expression`'s evaluator needs (name, checked body, slot count --
/// FR-068's own Description names these as the accessor surface), without
/// exposing `CheckedFunction`'s private representation. Crate-internal only:
/// `body`'s `Node` type is `pub(crate)`.
pub(crate) struct FunctionState<'a> {
    /// The declared name.
    pub(crate) name: &'a str,
    /// The checked body.
    pub(crate) body: &'a Node,
    /// The evaluation slot count.
    pub(crate) slots: usize,
}

/// One callable function's evaluation-visible identity and signature --
/// what [`crate::value::expression::CheckedPackage::call`] needs to route a
/// runtime `QualifiedName` lookup through
/// [`crate::family::ReferenceEvaluation::evaluate`] and validate its
/// caller's arguments, without a direct field read.
pub(crate) struct CallableFunction<'a> {
    /// The minted identity [`crate::family::ReferenceEvaluation::evaluate`]
    /// resolves against.
    pub(crate) identity: quire_exact::NodeKey,
    /// The declared parameters, for argument admission.
    pub(crate) parameters: &'a [(String, ValueType)],
}

fn root(origin: Origin) -> Location {
    Location {
        origin,
        path: Vec::new(),
    }
}

fn invalid_dispatch(location: Location, detail: InvalidDispatchDeclaration) -> CheckRefusal {
    CheckRefusal {
        location,
        cause: CheckCause::InvalidDispatchDeclaration(detail),
    }
}

/// One dispatch-table function index against the dispatch operation it must
/// conform to: `expected_arity` is the receiver plus every declared
/// argument, `result` is `Boolean` for a precondition (clause or combinator)
/// and the operation's own declared result for a body -- validates every
/// dispatch table/candidate index and signature arity/type upfront, rather
/// than trusting a caller-supplied table at evaluation time. A function that
/// exists (valid `index`) is located at its own real
/// [`Origin::Body`]; an out-of-range `index` names no real declaration to
/// point at, so it is located at [`Origin::Expression`] instead.
fn validate_dispatch_function(
    functions: &[FunctionDeclaration],
    index: usize,
    call_parameters: &[ValueType],
    result: &ValueType,
    role: DispatchFunctionRole,
) -> Result<(), CheckRefusal> {
    let Some(function) = functions.get(index) else {
        return Err(invalid_dispatch(
            root(Origin::Expression),
            InvalidDispatchDeclaration::FunctionOutOfRange { role, index },
        ));
    };
    let location = root(Origin::Body {
        function: function.name.clone(),
        index,
    });
    let expected_arity = call_parameters.len() + 1;
    if function.parameters.len() != expected_arity {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::Arity {
                role,
                index,
                declared: function.parameters.len(),
                expected: expected_arity,
            },
        ));
    }
    let mismatched = function
        .parameters
        .iter()
        .skip(1)
        .map(|(_, value_type)| value_type)
        .zip(call_parameters)
        .any(|(declared, expected)| declared != expected);
    if mismatched {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::ParameterType { role, index },
        ));
    }
    if &function.result != result {
        return Err(invalid_dispatch(
            location,
            InvalidDispatchDeclaration::ResultType { role, index },
        ));
    }
    Ok(())
}

impl PackageDeclarations {
    /// Check every function: duplicate names, declared types, typing, static
    /// definedness and termination, in that order. Every refusal is made
    /// before any charge; a reached checking limit is `resource_exhausted`
    /// and yields no admission verdict.
    pub fn check(self, limits: CheckingLimits) -> Result<CheckedGraph, Vec<CheckRefusal>> {
        let body_location = |index: usize, name: &str| {
            root(Origin::Body {
                function: name.to_owned(),
                index,
            })
        };
        let mut refusals = Vec::new();
        for (index, function) in self.functions.iter().enumerate() {
            let loci: Vec<Location> = self
                .functions
                .iter()
                .enumerate()
                .filter(|(_, other)| other.name == function.name)
                .map(|(other, declaration)| body_location(other, &declaration.name))
                .collect();
            if loci.len() > 1 {
                refusals.push(CheckRefusal {
                    location: body_location(index, &function.name),
                    cause: CheckCause::AmbiguousName {
                        name: function.name.clone(),
                        loci,
                    },
                });
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let mut seen_operations = std::collections::BTreeSet::new();
        for operation in &self.dispatch_operations {
            if !seen_operations.insert((operation.receiver_type, operation.member.clone())) {
                refusals.push(invalid_dispatch(
                    root(Origin::Expression),
                    InvalidDispatchDeclaration::DuplicateOperation {
                        receiver_type: operation.receiver_type,
                        member: operation.member.clone(),
                    },
                ));
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        for operation in &self.dispatch_operations {
            let Some(table) = self.dispatch_tables.get(operation.table) else {
                refusals.push(invalid_dispatch(
                    root(Origin::Expression),
                    InvalidDispatchDeclaration::TableOutOfRange {
                        member: operation.member.clone(),
                        table: operation.table,
                    },
                ));
                continue;
            };
            for (_, candidate) in table.entries() {
                if let Err(refusal) = validate_dispatch_function(
                    &self.functions,
                    candidate.body,
                    &operation.parameters,
                    &operation.result,
                    DispatchFunctionRole::Body,
                ) {
                    refusals.push(refusal);
                }
                if let Some(precondition) = candidate.precondition {
                    if let Err(refusal) = validate_dispatch_function(
                        &self.functions,
                        precondition,
                        &operation.parameters,
                        &ValueType::Boolean,
                        DispatchFunctionRole::Precondition,
                    ) {
                        refusals.push(refusal);
                    }
                }
                for &clause in &candidate.precondition_clauses {
                    if let Err(refusal) = validate_dispatch_function(
                        &self.functions,
                        clause,
                        &operation.parameters,
                        &ValueType::Boolean,
                        DispatchFunctionRole::PreconditionClause,
                    ) {
                        refusals.push(refusal);
                    }
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let scope = Scope {
            types: self.types,
            enums: self.enums,
            aliases: self.aliases,
            model_operations: self.model_operations,
            ieee_profile: self.ieee_profile,
            dispatch_operations: self.dispatch_operations,
        };
        let dispatch_tables = self.dispatch_tables;
        let signatures: Vec<Signature> = self
            .functions
            .iter()
            .map(|function| Signature {
                name: function.name.clone(),
                parameters: function.parameters.clone(),
                result: function.result.clone(),
                callable_by_name: function.callable_by_name,
            })
            .collect();
        let mut functions = Vec::with_capacity(self.functions.len());
        let mut calls: Vec<Vec<CallSite>> = Vec::with_capacity(self.functions.len());
        // QSL-148: identity, the real typing/definedness verdict and one
        // success diagnostic are all produced through one call into the
        // checked-family contract's own `check` hook
        // (`family::ValueFunctionFamily::check`) -- not, as before this
        // ticket, identity alone through the contract with the real
        // typing/definedness verdict made separately by a `Typer` this loop
        // constructed and drove directly on the side. `check` itself now
        // calls `family::check_declaration_body`; this loop calls the
        // contract exactly once per declaration and reaches that verdict
        // only through it. Termination stays below, over every declaration's
        // own `calls` this loop collects: see `check_declaration_body`'s
        // own doc for why that one part cannot move the same way.
        let package_identity = family::DEFAULT_PACKAGE_IDENTITY.to_owned();
        // ADR-013 O-14/C-26 (PR #300 review finding 1): every admitted
        // composite and enum type declaration this package's own
        // `TypeEnvironment`/`enums` already carry becomes a real
        // `CheckedTypeNode`, minted from its own declared shape through
        // `identity::mint_type_declaration_identity` -- real production
        // code, not only exercised by this module's unit tests.
        let mut type_nodes = BTreeMap::new();
        for composite in scope.types.composites() {
            // Complete-V1's own `TypeEnvironment` admits any string as a
            // composite's declared name (existing fixtures use
            // package-convention names such as `"P::R"`, never validated as
            // a grammar identifier -- that is a pre-existing, unrelated
            // property of this native-v1 type system, not something O-11's
            // qualified-name mechanism can retrofit). A declaration whose
            // name is not itself a valid `Identifier` simply gets no
            // `CheckedTypeNode` yet, rather than a panic; it is not a
            // reachable refusal this stage owns today.
            let Ok(name) = crate::value::Identifier::new(composite.name().to_owned()) else {
                continue;
            };
            let node = identity::mint_type_declaration_identity(
                &package_identity,
                std::slice::from_ref(&name),
                identity::DeclaredShape::Composite(composite.shape()),
            );
            type_nodes.insert(node, identity::CheckedTypeNode::Composite { node });
        }
        for enum_binding in &scope.enums {
            let Ok(name) = crate::value::Identifier::new(enum_binding.name.clone()) else {
                continue;
            };
            let variants: Option<Vec<identity::SumVariant>> = enum_binding
                .members
                .iter()
                .map(|member| {
                    crate::value::Identifier::new(member.case().to_owned())
                        .ok()
                        .map(identity::SumVariant::new)
                })
                .collect();
            let Some(variants) = variants else {
                continue;
            };
            // Two members sharing one declared case name is already refused
            // upstream of `check` (an `EnumBinding` whose own admission
            // rejected a duplicate case never reaches here) -- this is
            // defense in depth over an input `check` treats as already
            // well-formed, not a reachable production refusal, so this
            // declaration's checked type node is simply omitted rather than
            // panicking on it.
            if let Ok(variants) = identity::SumVariants::new(variants) {
                let node = identity::mint_type_declaration_identity(
                    &package_identity,
                    std::slice::from_ref(&name),
                    identity::DeclaredShape::Sum(&variants),
                );
                type_nodes.insert(node, identity::CheckedTypeNode::Sum { node, variants });
            }
        }
        // ADR-013 O-04 (PR #300 review finding 1): `check` records every
        // caller-supplied correspondence entry verbatim -- see
        // `PackageDeclarations::model_correspondence`'s own doc for why the
        // entries themselves are still caller-supplied today.
        let mut model_correspondence = identity::ModelCorrespondence::default();
        for (node, declaration) in self.model_correspondence {
            model_correspondence.record(node, declaration);
        }
        // PR #262 review, finding F4: this used to hardcode
        // `MAX_CHECKING_DEPTH` here regardless of what `limits` (this
        // method's own caller-supplied `CheckingLimits`) declared, and a
        // fresh `CheckContext` is built inside the per-declaration loop
        // below (`depth` starts at 0 every time) -- so `enter_nesting`'s
        // `0 >= nesting_depth` never held for any real caller, and the
        // `StageFailure::Limit` arm below was dead through this, the only
        // production entry point. Reading `limits.depth()` here (the same
        // `CheckingLimits` the unchanged `Typer` below already honors)
        // makes the contract's own resource bound live.
        // QSL-153: `node_count` reads the same `CheckingLimits.nodes()` the
        // unchanged `Typer` below also honors, but the two are separate,
        // deliberately different-shaped bounds over the same underlying
        // quantity (PR #303 review round 3, finding F1): the contract's own
        // `check_node_count` compares one declaration's own preimage node
        // count against `limits.nodes()` in isolation, while `Typer`'s
        // counter (seeded from `nodes_used` below) accumulates across every
        // declaration in the package, against that same unmodified bound --
        // the contract's check can refuse a single oversized declaration
        // first, but it is not a substitute for the package-wide budget, and
        // does not make it redundant. `input_bytes` reads `CheckingLimits`' own
        // dedicated knob (`with_input_bytes`), unlimited unless a caller
        // configures it -- the same real-default-until-configured shape
        // `nesting_depth` itself had before this exact fix wired it to
        // `limits.depth()` (`StageLimits`'s own doc). `work_budget` has no
        // `StageLimits` field at all (PR #302 review finding 3): it is
        // charged against `contract_meter`'s own `work_units` bound
        // instead, read from `CheckingLimits::work_budget` (also unlimited
        // unless a caller configures it via `with_work_budget`) -- the same
        // unbounded-by-default shape. The mechanism is real (`CheckContext::
        // check_input_bytes`/`check_node_count`, and a `cx.meter` charge for
        // `work_budget`) and is exercised directly against tight fixtures in
        // `src/value/expression/family.rs`'s `family_contract_tests`.
        let contract_limits = crate::family::StageLimits {
            nesting_depth: limits.depth(),
            input_bytes: limits.input_bytes(),
            node_count: limits.nodes(),
        };
        let contract_meter_limits = quire_exact::ScalarLimits {
            work_units: limits.work_budget(),
            ..family::SCALAR_LIMITS_UNLIMITED
        };
        let mut contract_meter = quire_exact::Meter::new(contract_meter_limits);
        let mut contract_diagnostics = crate::family::DiagnosticSink::default();
        let mut contract_scopes = crate::family::ScopeStack::default();
        // PR #303 review round 3, finding F1: the running total of
        // `Expression` nodes every declaration admitted so far in this
        // package has consumed -- this loop's own state, read by each
        // iteration's `ValueDeclarations::nodes_used` and advanced from each
        // admitted declaration's own `CheckedDeclarationBody::nodes_used`,
        // restoring `CheckingLimits::new`'s documented, package-wide `nodes`
        // bound (see `check::family::check_declaration_body`'s own doc for
        // why this, not a `Cell` or a second call, is the mechanism).
        let mut nodes_used = 0_u64;
        for (index, function) in self.functions.into_iter().enumerate() {
            let location = body_location(index, &function.name);
            let measure_location = root(Origin::Measure {
                function: function.name.clone(),
                index,
            });
            let declarations = family::ValueDeclarations {
                package_identity: &package_identity,
                scope: &scope,
                signatures: &signatures,
                dispatch_tables: &dispatch_tables,
                checking_limits: limits,
                location: &location,
                measure_location: &measure_location,
                nodes_used,
            };
            let mut contract_cx = crate::family::CheckContext::new(
                &declarations,
                contract_limits,
                &mut contract_meter,
                &mut contract_diagnostics,
                &mut contract_scopes,
            );
            match family::ValueFunctionFamily::check(&function, &mut contract_cx) {
                Ok(staged) => {
                    // The real typing/definedness verdict travels out
                    // through `Staged::value` itself (a `CheckedDeclaration`
                    // carrying both the minted identity and the real checked
                    // body -- PR #303 review, finding N3), not a side
                    // channel: no `.expect(...)` unwrap of a slot `check`
                    // might not have filled.
                    let checked = staged.value;
                    // PR #303 review round 3, finding F1: advance the
                    // package's running node total from this admitted
                    // declaration's own final count, so the next
                    // declaration's `Typer` picks up where this one left
                    // off instead of restarting at zero.
                    nodes_used = checked.body.nodes_used;
                    functions.push(CheckedFunction {
                        identity: checked.identity,
                        signature: Signature {
                            name: function.name,
                            parameters: function.parameters,
                            result: function.result,
                            callable_by_name: function.callable_by_name,
                        },
                        body: checked.body.body,
                        measure: checked.body.measure,
                        slots: checked.body.slots,
                    });
                    calls.push(checked.body.calls);
                }
                Err(crate::family::StageFailure::Limit(limit)) => {
                    // PR #262 review (coordinator round 3, finding 4):
                    // `limit.kind` is matched, not read past into a
                    // hardcoded `CheckingLimitKind::Depth` -- this exhaustive
                    // match (not a `_` catch-all) is what forces a real
                    // decision here, not a guess, now that `StageLimitKind`
                    // has grown three more variants (QSL-153). `NodeCount`
                    // maps onto the pre-existing `CheckingLimitKind::Nodes`
                    // (both name "how many expression nodes"); `InputBytes`
                    // and `WorkBudget` have no pre-existing counterpart in
                    // this older `Typer`-era enum, so QSL-153 adds one each.
                    let kind = match limit.kind {
                        crate::family::StageLimitKind::NestingDepth => CheckingLimitKind::Depth,
                        crate::family::StageLimitKind::NodeCount => CheckingLimitKind::Nodes,
                        crate::family::StageLimitKind::InputBytes => CheckingLimitKind::InputBytes,
                        crate::family::StageLimitKind::WorkBudget => CheckingLimitKind::WorkBudget,
                    };
                    refusals.push(CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::ResourceExhausted {
                            stage: CheckingStage::Typing,
                            kind,
                            limit: limit.configured_bound,
                        },
                    });
                }
                Err(crate::family::StageFailure::Refused(refusal)) => {
                    let exhausted = matches!(refusal.cause, CheckCause::ResourceExhausted { .. });
                    refusals.push(refusal);
                    if exhausted {
                        return Err(refusals);
                    }
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // QSL-148: static definedness is checked per declaration now, inside
        // `family::check_declaration_body`, immediately after that same
        // declaration's typing -- `calls` (one entry per admitted function,
        // in the same order as `functions`) is collected there, not by a
        // separate pass over `&functions` here. Termination is unchanged: it
        // is still a whole-package call-graph analysis over every
        // declaration's own `calls` at once (see `check_declaration_body`'s
        // own doc for why it cannot move alongside typing/definedness).
        let members: Vec<termination::Member<'_>> = functions
            .iter()
            .zip(&calls)
            .map(|(function, calls)| termination::Member {
                name: &function.signature.name,
                parameters: &function.signature.parameters,
                measure: function.measure.as_ref(),
                calls,
            })
            .collect();
        let refusals = termination::check(&members);
        if !refusals.is_empty() {
            return Err(refusals);
        }
        // FR-062-AC-2/FR-065-AC-3: the occurrence-keyed source map, built
        // from each function's own minted declaration identity and every
        // `NodeKind::Call` identity its checked body (and measure, if any)
        // already carries -- reads locations `Typer` already recorded, mints
        // no new identity or span here.
        let mut occurrences = family::OccurrenceMap::default();
        for (index, function) in functions.iter().enumerate() {
            occurrences.record(
                function.identity,
                "declaration",
                body_location(index, &function.signature.name),
            );
            for (identity, location) in function.body.call_occurrences() {
                occurrences.record(identity, "reference", location);
            }
            if let Some(measure) = &function.measure {
                for (identity, location) in measure.call_occurrences() {
                    occurrences.record(identity, "reference", location);
                }
            }
        }
        Ok(CheckedGraph {
            scope,
            functions,
            dispatch_tables,
            occurrences,
            model_correspondence,
            type_nodes,
        })
    }
}

impl CheckedGraph {
    /// ADR-013 O-04: `node`'s `DeclarationKey`, read only from the model
    /// correspondence this package's own checking recorded (FR-088-AC-2).
    /// Node-id-keyed, never name-keyed (R-06/FR-088-AC-5): `CheckedGraph`
    /// exposes no accessor that takes a name and returns a node id or
    /// declaration.
    pub fn resolve_declaration(
        &self,
        node: quire_exact::NodeKey,
    ) -> Option<&crate::model::key::DeclarationKey> {
        self.model_correspondence.resolve(node)
    }

    /// ADR-013 O-14/C-26 (PR #300 review finding 1): `node`'s own checked
    /// type descriptor, when this package admitted a composite or enum
    /// declaration that minted that node id. Node-id-keyed, never
    /// name-keyed (R-06), like every other accessor here.
    pub fn checked_type_node(&self, node: quire_exact::NodeKey) -> Option<&CheckedTypeNode> {
        self.type_nodes.get(&node)
    }

    /// Every checked type node this package admitted, in node-id order --
    /// the listing accessor a test or a future C-26 consumer uses to find a
    /// declaration's own minted node id without independently recomputing
    /// it (there is no name-keyed lookup, R-06).
    pub fn checked_type_nodes(&self) -> impl Iterator<Item = &CheckedTypeNode> {
        self.type_nodes.values()
    }

    /// Check a standalone expression over `parameters`, against `expected`
    /// when given, as a function or operation body (`ClauseKind::Body`).
    /// `pre(...)` refuses `wrong_snapshot`/`wrong-anchor` here: this is not
    /// an operation's postcondition, the only clause FR-153's anchor table
    /// admits it in. Use [`Self::check_postcondition_expression`] to check a
    /// real postcondition, where `pre(...)` is legal, or
    /// [`Self::check_clause_expression`] for any other [`ClauseKind`].
    pub fn check_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        self.check_clause_expression(
            parameters,
            expression,
            expected,
            ClauseKind::Body,
            mode,
            limits,
        )
    }

    /// Check a standalone expression as an operation's postcondition
    /// (`ClauseKind::Postcondition`): identical to [`Self::check_expression`],
    /// except `pre(...)` is legal (FR-153's own anchor table;
    /// shared-grammar.md's caller-side anchor operations), subject to its
    /// own eligible-operand rule (FR-042's Behavior clause, `Typer`'s
    /// `Expression::Pre` arm).
    pub fn check_postcondition_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        self.check_clause_expression(
            parameters,
            expression,
            expected,
            ClauseKind::Postcondition,
            mode,
            limits,
        )
    }

    /// Check a standalone expression over `parameters` as `clause_kind`,
    /// against `expected` when given. FR-151's dispatch-call restriction
    /// (TC-196 D06/D07) gates on `clause_kind`, not on syntax alone; `pre(...)`
    /// is legal exactly when `clause_kind` is [`ClauseKind::Postcondition`]
    /// (`Typer`'s `Expression::Pre` arm).
    pub fn check_clause_expression(
        &self,
        parameters: Vec<(String, ValueType)>,
        expression: &Expression,
        expected: Option<&ValueType>,
        clause_kind: ClauseKind,
        mode: CheckMode,
        limits: CheckingLimits,
    ) -> Result<CheckedExpression, CheckRefusal> {
        let location = root(Origin::Expression);
        let signatures: Vec<Signature> = self
            .functions
            .iter()
            .map(|function| function.signature.clone())
            .collect();
        let mut nodes = 0_u64;
        let mut typer = Typer::new(&self.scope, &signatures, limits, &mut nodes, clause_kind);
        bind_parameters(&mut typer, &parameters, &location)?;
        let root = match expected {
            Some(expected) => {
                typer.check_declared_type(expected, &location)?;
                typer.check_as(expression, expected, &location)?
            }
            None => typer.infer(expression, None, &location)?,
        };
        let slots = typer.slots();
        if mode == CheckMode::Linked {
            Definedness::new(
                parameters.len(),
                &self.dispatch_tables,
                &self.scope.dispatch_operations,
            )
            .check(&root)?;
        }
        Ok(CheckedExpression {
            parameters,
            root,
            slots,
        })
    }

    fn function(&self, name: &str) -> Option<(usize, &CheckedFunction)> {
        self.functions
            .iter()
            .enumerate()
            .find(|(_, function)| function.signature.name == name)
    }

    fn function_by_identity_raw(&self, identity: quire_exact::NodeKey) -> Option<&CheckedFunction> {
        self.functions
            .iter()
            .find(|function| function.identity == identity)
    }

    /// The `deref(r).f` locations of a function body, or `None` for an
    /// undeclared name.
    pub fn dereferences(&self, function: &str) -> Option<Vec<Location>> {
        self.function(function)
            .map(|(_, function)| function.body.dereferences())
    }

    /// The `convert` losses of a function body, or `None` for an undeclared
    /// name.
    pub fn losses(&self, function: &str) -> Option<Vec<CollectionLoss>> {
        self.function(function)
            .map(|(_, function)| function.body.losses())
    }

    /// FR-065-AC-2: `name`'s checked identity, minted once at `check` and
    /// unchanged by anything else in the package. `None` for an undeclared
    /// name.
    pub fn function_identity(&self, name: &str) -> Option<quire_exact::NodeKey> {
        self.function(name).map(|(_, function)| function.identity)
    }

    /// FR-062-AC-2/FR-065-AC-3: the source location recorded for one
    /// occurrence (identity, role, ordinal) of a checked function
    /// declaration or function-application call, if `check` recorded one.
    pub fn occurrence(
        &self,
        identity: quire_exact::NodeKey,
        origin: &quire_exact::Origin,
    ) -> Option<&Location> {
        self.occurrences.resolve(identity, origin)
    }

    /// Every admitted function's own name, checked body and evaluation slot
    /// count -- the accessor surface
    /// [`crate::value::expression::CheckedPackage::evaluate`] reads to build
    /// its own `Callable` list, since `Callable` is a layer-5 type this
    /// module must not construct itself (that would be a `check` ->
    /// `value::expression` edge, forbidden by FR-068-AC-3).
    pub(crate) fn function_states(&self) -> impl Iterator<Item = FunctionState<'_>> + '_ {
        self.functions.iter().map(|function| FunctionState {
            name: &function.signature.name,
            body: &function.body,
            slots: function.slots,
        })
    }

    /// One admitted function's evaluation-visible state, by its minted
    /// identity -- the accessor
    /// [`crate::family::ReferenceEvaluation::evaluate`]'s `Value` family
    /// implementation (`value::expression::family`) resolves a checked call
    /// against.
    pub(crate) fn function_by_identity(
        &self,
        identity: quire_exact::NodeKey,
    ) -> Option<FunctionState<'_>> {
        self.function_by_identity_raw(identity)
            .map(|function| FunctionState {
                name: &function.signature.name,
                body: &function.body,
                slots: function.slots,
            })
    }

    /// `name`'s identity and declared parameters, filtered to functions a
    /// plain named call may resolve to (`callable_by_name`) -- the accessor
    /// [`crate::value::expression::CheckedPackage::call`] uses to resolve a
    /// runtime `QualifiedName` lookup and validate its caller's arguments,
    /// without a direct field read (TC-196 D07's bypass: a crate-internal
    /// FR-151 synthesized dispatch candidate is never reachable this way).
    pub(crate) fn callable(&self, name: &str) -> Option<CallableFunction<'_>> {
        self.function(name)
            .filter(|(_, function)| function.signature.callable_by_name)
            .map(|(_, function)| CallableFunction {
                identity: function.identity,
                parameters: &function.signature.parameters,
            })
    }

    /// Every admitted function's own name and minted identity -- the
    /// accessor
    /// [`crate::value::expression::CheckedPackage::emit_function_package_v2`]
    /// reads to build its v2 entries, since the v2 codec and `QualifiedName`
    /// are `value::expression::family` types this module must not import
    /// (FR-068-AC-3).
    pub(crate) fn function_identities(
        &self,
    ) -> impl Iterator<Item = (&str, quire_exact::NodeKey)> + '_ {
        self.functions
            .iter()
            .map(|function| (function.signature.name.as_str(), function.identity))
    }

    /// The scope every declared name resolves against -- the accessor
    /// [`crate::value::expression::CheckedPackage::evaluate`] passes through
    /// to the evaluator.
    pub(crate) fn scope(&self) -> &Scope {
        &self.scope
    }

    /// The checked dispatch tables `scope`'s dispatch operations index into
    /// -- the accessor the evaluator reads directly, alongside `scope`.
    pub(crate) fn dispatch_tables(&self) -> &[DispatchTable] {
        &self.dispatch_tables
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{Origin, Role};

    use super::*;

    fn declarations(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
        PackageDeclarations {
            functions,
            ..PackageDeclarations::default()
        }
    }

    /// PR #300 review finding 1: `ModelCorrespondence` is recorded by a
    /// real `PackageDeclarations::check` run, from its own new
    /// `model_correspondence` field, and read back only through
    /// `CheckedGraph::resolve_declaration` -- not a hand-built
    /// `ModelCorrespondence` sitting outside the checker (FR-088-AC-2). The
    /// frame-subject *resolution mechanics* over that correspondence stay
    /// covered by `identity::tests::frame_subjects_resolve_only_through_the_recorded_correspondence`,
    /// since FR-340 frame syntax does not exist yet (FR-088-CON-2); this
    /// test is the "the checker really records it" half.
    #[trace("TC-248", "FR-088-AC-2")]
    #[test]
    fn model_correspondence_is_recorded_by_a_real_check_run() {
        let node = quire_exact::NodeKey::from_digest([7_u8; 32]);
        let declaration = crate::model::key::DeclarationKey {
            package: "test/orders".to_owned(),
            node: "Order.status".to_owned(),
        };
        let graph = PackageDeclarations {
            model_correspondence: vec![(node, declaration.clone())],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("an empty package with a correspondence seed checks cleanly");

        assert_eq!(graph.resolve_declaration(node), Some(&declaration));

        // Adverse (R-05): a node the caller never supplied resolves to
        // nothing -- `check` never re-derives an entry by search.
        let other = quire_exact::NodeKey::from_digest([8_u8; 32]);
        assert_eq!(graph.resolve_declaration(other), None);
    }

    /// PR #300 review finding 1: a real composite declaration in
    /// `PackageDeclarations.types` becomes a real `CheckedTypeNode`,
    /// convertible through C-26, not only in `identity`'s own
    /// hand-constructed unit tests.
    #[trace("TC-252", "FR-088-AC-9")]
    #[test]
    fn composite_declaration_becomes_a_real_checked_type_node() {
        use crate::value::composite::{
            CompositeDeclaration, CompositeShape, FieldDeclaration, Presence, TypeEnvironment,
        };
        use crate::value::node::NodeKey as ValueNodeKey;

        let field = FieldDeclaration::new("flag", ValueType::Boolean, Presence::Required);
        let composite = CompositeDeclaration::new(
            ValueNodeKey::from_hex(&"11".repeat(32)).expect("64 lowercase hex digits"),
            "Flagged",
            CompositeShape::Record(vec![field]),
        );
        let types = TypeEnvironment::new([composite], []).expect("one record admits cleanly");
        let graph = PackageDeclarations {
            types,
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("one record declaration checks cleanly");

        let nodes: Vec<&CheckedTypeNode> = graph.checked_type_nodes().collect();
        assert_eq!(nodes.len(), 1, "exactly the one declared composite");
        let node = nodes[0].node();
        assert_eq!(graph.checked_type_node(node), Some(nodes[0]));
        assert_eq!(
            to_kernel_value_type(nodes[0]),
            quire_exact::ValueType::Composite(node)
        );
    }

    /// PR #300 review finding 7: TC-249's own identity/occurrence-key
    /// mechanism, demonstrated through a real `check()` run rather than
    /// only a hand-built `OccurrenceMap` (`identity`'s own unit test keeps
    /// that mechanism coverage; this is the real-checker, adverse
    /// companion FR-088-AC-3 also names). This crate has no `claim`/
    /// `temporal`/`protocol` clause syntax yet (FR-088-CON-1 scopes clause
    /// identity to the identity/occurrence-key mechanism only, not the
    /// syntax that would produce one), so the closest real,
    /// checker-produced case of "one identity, two distinct occurrences"
    /// is two calls to the same function with the same arguments:
    /// `family::mint_call_identity` mints one identity from the callee
    /// name and arguments alone, never from a resolved index or the
    /// caller's own identity, so both call sites share it -- the same
    /// "structurally identical content shares one id" premise ADR-013 O-04
    /// states for clause identity -- while `check`'s own `OccurrenceMap`
    /// still gives each call site its own (role, ordinal), exactly O-07's
    /// shape.
    #[trace("TC-249", "FR-088-AC-3")]
    #[test]
    fn call_occurrences_of_the_same_callee_share_an_identity_and_disambiguate_by_occurrence_key() {
        fn literal_function(name: &str, body: Expression) -> FunctionDeclaration {
            FunctionDeclaration::new(name, Vec::new(), ValueType::Boolean, None, body)
        }
        let call_helper = || Expression::Call {
            name: "helper".to_owned(),
            arguments: Vec::new(),
        };
        let graph = declarations(vec![
            literal_function("helper", Expression::Boolean(true)),
            literal_function("caller_one", call_helper()),
            literal_function("caller_two", call_helper()),
        ])
        .check(CheckingLimits::default())
        .expect("two callers of one no-argument function check cleanly");

        // Steps 2-3: both call sites mint the same identity, but distinct
        // occurrence keys.
        let call_identity =
            family::mint_call_identity(family::DEFAULT_PACKAGE_IDENTITY, "helper", &[]);
        let first = Origin::new(Role::new("reference"), 0);
        let second = Origin::new(Role::new("reference"), 1);
        let first_location = graph
            .occurrence(call_identity, &first)
            .expect("the first call site was recorded")
            .clone();
        let second_location = graph
            .occurrence(call_identity, &second)
            .expect("the second call site was recorded")
            .clone();

        // Step 4: (identity, occurrence key) disambiguates what identity
        // alone cannot; a third, never-recorded ordinal resolves to
        // nothing rather than aliasing an existing occurrence.
        assert_ne!(
            first_location, second_location,
            "two distinct source occurrences must not collapse to one location"
        );
        let third = Origin::new(Role::new("reference"), 2);
        assert_eq!(graph.occurrence(call_identity, &third), None);

        // Step 5 (adverse, R-05): renaming both callers changes their
        // *diagnostic* location text (each `Location` names its own
        // enclosing function, exactly as a diagnostic pointer should), but
        // leaves the shared call *identity* and both *occurrence keys*
        // (role, ordinal) unchanged -- display/diagnostic text is never
        // load-bearing for identity or the occurrence key that
        // disambiguates it, only for where a human-readable message points.
        let renamed = declarations(vec![
            literal_function("helper", Expression::Boolean(true)),
            literal_function("renamed_caller_one", call_helper()),
            literal_function("renamed_caller_two", call_helper()),
        ])
        .check(CheckingLimits::default())
        .expect("renaming the callers changes no checking outcome");
        assert!(
            renamed.occurrence(call_identity, &first).is_some(),
            "the same call identity and first occurrence key still resolve after renaming"
        );
        assert!(
            renamed.occurrence(call_identity, &second).is_some(),
            "the same call identity and second occurrence key still resolve after renaming"
        );
        assert_ne!(
            renamed.occurrence(call_identity, &first),
            Some(&first_location),
            "the diagnostic location's own embedded function name is expected to change"
        );
    }
}
