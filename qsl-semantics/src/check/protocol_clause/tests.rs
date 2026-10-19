//! Tests for the `ProtocolClause` S3 check, split by what they exercise.
//! The helpers below are shared by every submodule.

use qsl_forms::{build_unit, AnchorSite, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::SourceIdentity;

use super::super::{
    CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations, Scope, Signatures,
};
use super::{CheckedProtocol, ProtocolNodeId};
use crate::check::refusal::ProtocolAnchorCause;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::TypeEnvironment;

mod attempt_fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_attempt.rs");
}
use attempt_fixture::{m_actor_model, HEADER};

mod anchors;
mod attempts;
mod binders;
mod coverage;
mod graph;
mod node_keys;

/// S1, S2 and the assembler over `declarations`, against
/// [`m_actor_model`]'s own `M::Actor::op`: FR-113's checker
/// itself resolves protocol node references only, never a model type,
/// but FR-114's own assembler resolution now runs for every attempt
/// regardless, so this module's fixtures need a real admitted model
/// even though FR-113's own checks do not read it.
fn assemble(declarations: &str) -> (String, PackageDeclarations) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![m_actor_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{declarations}: the assembler refuses: {refusal:?}"));
    (text, assembled)
}

/// An empty `Scope` and `Signatures`: no alias, type or function names
/// for a binder to shadow, so [`resolve_protocol`]'s fixtures (which
/// carry no shadowing binder) resolve exactly as the anchor-resolution
/// algorithm alone would.
fn empty_scope_and_signatures() -> (Scope, Signatures) {
    (
        Scope::new(
            TypeEnvironment::default(),
            crate::model::operation::OperationTable::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            Vec::new(),
        ),
        Signatures::new(Vec::new()),
    )
}

/// `super::check` called directly on the one protocol `declarations`
/// holds, bypassing `PackageDeclarations::check`'s whole-package pass:
/// this is the anchor-resolution algorithm's own oracle, not the whole
/// pipeline's. FR-114's own binding is bypassed too (empty `attempts`
/// and `state_clauses`), since this helper isolates FR-113 alone.
/// [`super::check`]'s own result, for [`resolve_protocol`]'s return type
/// (clippy's `type_complexity`: three levels of nesting is easier read
/// as one name than inline).
type CheckResult = Result<(CheckedProtocol, Vec<super::BoundAttempt>), Vec<CheckRefusal>>;

fn resolve_protocol(declarations: &str) -> (String, ProtocolDeclarationForm, CheckResult) {
    let (text, assembled) = assemble(declarations);
    let protocol = assembled.protocols[0].clone();
    let (scope, signatures) = empty_scope_and_signatures();
    let result = super::check(
        &protocol,
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeSet::new(),
        &scope,
        &signatures,
        &[],
        &[],
        None,
    );
    (text, protocol, result)
}

/// The full `PackageDeclarations::check` pipeline over `declarations`,
/// expecting it to refuse.
fn refusals(declarations: &str) -> (String, Vec<CheckRefusal>) {
    let (text, assembled) = assemble(declarations);
    match assembled.check(CheckingLimits::default()) {
        Err(refusals) => (text, refusals),
        Ok(_) => panic!("{declarations}: checking refuses"),
    }
}

/// The full `PackageDeclarations::check` pipeline over `declarations`,
/// expecting it to succeed.
fn checks(declarations: &str) -> CheckedGraph {
    let (_, assembled) = assemble(declarations);
    assembled
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// S1, S2 and the assembler over `declarations` (against
/// [`m_actor_model`]), expecting the assembler to refuse.
fn assembly_errors(declarations: &str) -> Vec<crate::check::AssemblyError> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![m_actor_model()],
        Vec::new(),
    )
    .map(|_| ())
    .expect_err("the assembler refuses")
    .errors
}

/// The one refusal `declarations` checks to, expecting exactly one.
fn only_cause(declarations: &str) -> (CheckRefusal, ProtocolAnchorCause) {
    let (_, found) = refusals(declarations);
    assert_eq!(found.len(), 1, "{declarations}: {found:?}");
    let cause = protocol_causes(&found)[0].clone();
    (found[0].clone(), cause)
}

fn protocol_causes(refusals: &[CheckRefusal]) -> Vec<&ProtocolAnchorCause> {
    refusals
        .iter()
        .map(|refusal| match &refusal.cause {
            CheckCause::ProtocolAnchor(cause) => cause.as_ref(),
            other => panic!("a ProtocolAnchor cause, got {other:?}"),
        })
        .collect()
}

/// The one declaration named `name` of kind `kind`, in scope `scope`
/// (outermost first): the algorithm-level oracle `resolved_anchors`
/// assertions read against, so a test names the expected *target*, not
/// just "no refusal".
fn declaration_id(
    protocol: &ProtocolDeclarationForm,
    name: &str,
    kind: ProtocolNodeKind,
    scope: &[&str],
) -> ProtocolNodeId {
    let index = protocol
        .declarations
        .iter()
        .position(|declaration| {
            declaration.name.name == name
                && declaration.kind == kind
                && protocol
                    .scope_names(declaration.scope)
                    .into_iter()
                    .map(|name| name.name.as_str())
                    .eq(scope.iter().copied())
        })
        .unwrap_or_else(|| {
            panic!(
                "no {kind:?} named {name} in scope {scope:?}: {:?}",
                protocol.declarations
            )
        });
    ProtocolNodeId(index)
}

/// The indices of every scoped anchor at `site`, in source order.
fn anchors_at(protocol: &ProtocolDeclarationForm, site: AnchorSite) -> Vec<usize> {
    protocol
        .scoped_anchors
        .iter()
        .enumerate()
        .filter(|(_, anchor)| anchor.site == site)
        .map(|(index, _)| index)
        .collect()
}

/// The `RecoveryFlow` shape TC-510/511's own fixture uses (matching
/// `qsl-forms`'s `recovery_flow`), with `Main::Applied`/`Main::Committed`
/// resolved through the `compensate` template and `Tried`/`Undo`
/// resolved from inside `Main`. Declares channel `C` (FR-113-AC-7's
/// `receive-of` cases route a `receive` through it).
fn recovery_flow(commit: &str, main_extra: &str) -> String {
    format!(
        "protocol RecoveryFlow using v over (input: Boolean) on origin {{\n\
         role R on M::Actor;\n\
         channel C from R to R carries Boolean ordering unordered delivery at-most-once \
         capacity 1 overflow reject;\n\
         compensate Undo for Main::Applied as (failure: Boolean) by R \
         on M::Actor::op using v clock \"ticks\" {{\n\
         activate first (trigger: Boolean) when {{ true }} {{ }}\n\
         within [0,10]; attempts 2 of M::Try;\n\
         retry (current_attempt: M::Try, prior: M::Try) {{ true }};\n\
         commit {commit}; recover (recovery: Boolean) {{ true }}; }}\n\
         run sequence Main {{\n\
         attempt Tried by R on M::Actor::op contracts [] as (tried: Boolean) {{ true }};\n\
         {main_extra}\
         effect Applied of Tried as (applied: Boolean) {{ true }};\n\
         event Recovered by R for Undo as (notice: Boolean) {{ true }};\n\
         commit Committed by R as (committed: Boolean) {{ true }};\n\
         }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

/// A minimal `send`/`receive` protocol, its `receive` routed via
/// `channel`, with both `C` and `D` declared (FR-113-AC-7's channel
/// cases).
fn handoff_protocol(channel: &str) -> String {
    format!(
        "protocol Handoff using v over (input: Boolean) on origin {{\n\
         role R on M::Actor;\n\
         channel C from R to R carries Boolean ordering unordered delivery at-most-once \
         capacity 1 overflow reject;\n\
         channel D from R to R carries Boolean ordering unordered delivery at-most-once \
         capacity 1 overflow reject;\n\
         run sequence Main {{\n\
         send Ping via C as (ping: Boolean) {{ true }};\n\
         receive Got via {channel} of Ping as (got: Boolean) {{ true }};\n\
         }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}
