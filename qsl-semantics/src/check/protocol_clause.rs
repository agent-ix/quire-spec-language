// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-113: the `ProtocolClause` family's S3 scope check over one protocol's
//! `ScopedAnchorForm`s (ADR-012 §12.2 Check row).
//!
//! Each anchor's first segment resolves in the innermost scope of its
//! `scope`, then in each enclosing scope outward, then at the top level; a
//! later segment resolves among the names its previous target declares
//! directly. A scope that declares two static nodes of one name refuses
//! `ambiguous_declaration`/`ambiguous-name` at each declaration, and an
//! anchor whose resolution decides on that name refuses the same way at the
//! anchor. A resolved anchor is checked against the admitted target kinds
//! of its site (FR-113 "Behavior" table); a mismatch refuses
//! `ill_typed`/`type-mismatch`, naming the site, the admitted kinds and the
//! actual kind. A `receive` whose channel disagrees with the `send` its
//! `receive-of` anchor names refuses the same way, naming both channels.
//!
//! FR-113's binder no-shadowing rule (a record binder, a capture, a
//! compensation trigger or a retry or recovery parameter shadowing a
//! model/profile alias, a native declaration of the package or another
//! visible binder) is checked here too (QSL-306), over the `BinderForm`s
//! QSL-306's own S2 extension (`qsl_forms::protocol_clause::BinderForm`)
//! now builds for every binder position. QSpec `shared-grammar.md` makes
//! every binder "unique in their enclosing declaration": that declaration
//! is the checked protocol as a whole, so every binder name in it shares
//! one flat namespace, independent of the lexical scope it is written in
//! (SR-765 FND-001) -- a `finish` binder can shadow a `run`-tree binder,
//! two different `compensate` declarations' binders of one name collide,
//! and so do sibling `case`/`branch` binders of one name, matching the
//! composed lane's own `DuplicateBinder` checker
//! (`src/linking/composed/scopes.rs`). Only a *different* protocol
//! declaration is a separate enclosing declaration ("Two separate
//! protocols may reuse a binder name"), which this function's own
//! per-protocol call already gives for free. Binders are still checked
//! against a package-level alias or native declaration the same way (by
//! name alone, package-wide). Only one refusal is raised, at the shadowing
//! binder, naming what it shadows -- unlike a duplicate declaration,
//! shadowing is directional, so the shadowed declaration is not itself
//! refused.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{AnchorSite, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::Span;

use super::refusal::{
    AliasKind, CheckCause, CheckRefusal, Origin, ProtocolAnchorCause, ShadowedDeclaration,
};
use super::{Scope, Signatures};

/// The identity of one static protocol node: its index into the
/// declaration's own [`ProtocolDeclarationForm::declarations`] (FR-113
/// Outputs: "recorded on the checked protocol by that node's identity"). No
/// later stage recovers a target from a name.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProtocolNodeId(pub usize);

/// One protocol declaration FR-113 has checked: every scoped anchor's
/// resolved target, in the same order as the form's own `scoped_anchors`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedProtocol {
    /// The declared protocol name.
    pub name: String,
    /// `resolved_anchors[i]` is the identity `scoped_anchors[i]` resolved
    /// to.
    pub resolved_anchors: Vec<ProtocolNodeId>,
}

/// A declaration scope, keyed the way FR-113 resolves a segment: the
/// enclosing named controls' spelled names, outermost first.
type ScopeKey = Vec<String>;

fn scope_key(scope: &[qsl_forms::ScopeName]) -> ScopeKey {
    scope.iter().map(|name| name.name.clone()).collect()
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // A protocol has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads a
        // `ProtocolAnchor` cause's own span directly instead, so this
        // `Location` is never resolved.
        location: super::refusal::Location {
            origin: Origin::Expression,
            path: Vec::new(),
        },
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// The kinds `site` admits, and the word FR-113 names `site` by.
fn admitted_kinds(site: AnchorSite) -> (&'static str, &'static [ProtocolNodeKind]) {
    use ProtocolNodeKind as K;
    match site {
        AnchorSite::ReceiveOf => ("receive-of", &[K::Send]),
        AnchorSite::EffectOf => ("effect-of", &[K::Attempt]),
        AnchorSite::EventFor => ("event-for", &[K::CompensateTemplate]),
        AnchorSite::CompensateFor => ("compensate-for", &[K::Effect]),
        AnchorSite::CompensateCommit => ("compensate-commit", &[K::Commit]),
        AnchorSite::AwaitAfter => (
            "await-after",
            &[
                K::Send,
                K::Receive,
                K::Attempt,
                K::Effect,
                K::Event,
                K::Commit,
                K::CompensateTemplate,
            ],
        ),
    }
}

/// FR-113: the protocol node `protocol`'s scoped anchors resolve to, or the
/// refusals below, in source position order (declaration-order duplicate
/// refusals first at equal span, then binder-shadow refusals, then
/// per-anchor refusals). `alias_names` (the unit's own `profile`/`model`
/// selection aliases), `native_names` (its `dimension` and `unit`
/// declaration names), `scope` (the package's other native type
/// declarations: `type`, `record`, `tuple` and `enum`) and `signatures`
/// (its native function declarations) are the only package-wide state this
/// checker reads, for FR-113 "Refusals" binder no-shadowing rule; every
/// other check is over the protocol's own form.
pub fn check(
    protocol: &ProtocolDeclarationForm,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: &Scope,
    signatures: &Signatures,
) -> Result<CheckedProtocol, Vec<CheckRefusal>> {
    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();

    // Every static node by (its scope, its name): the declaration
    // collection FR-113 Inputs asks S2 for. A scope with two or more
    // entries of one name is refused up front, independent of whether any
    // anchor names it (FR-113 "Refusals").
    let mut by_scope_name: BTreeMap<(ScopeKey, String), Vec<usize>> = BTreeMap::new();
    for (index, declaration) in protocol.declarations.iter().enumerate() {
        by_scope_name
            .entry((scope_key(&declaration.scope), declaration.name.name.clone()))
            .or_default()
            .push(index);
    }
    for ((_, name), indices) in &by_scope_name {
        if indices.len() < 2 {
            continue;
        }
        let loci: Vec<Span> = indices
            .iter()
            .map(|&index| protocol.declarations[index].name.span)
            .collect();
        for &index in indices {
            let span = protocol.declarations[index].name.span;
            refusals.push((
                span,
                refusal(ProtocolAnchorCause::Ambiguous {
                    name: name.clone(),
                    loci: loci.clone(),
                    span,
                }),
            ));
        }
    }

    refusals.extend(shadow_refusals(
        protocol,
        alias_names,
        native_names,
        scope,
        signatures,
    ));

    let mut resolved = Vec::with_capacity(protocol.scoped_anchors.len());
    for anchor in &protocol.scoped_anchors {
        match resolve(protocol, &by_scope_name, anchor) {
            Ok(target) => {
                if let Some(cause) = check_kind(protocol, anchor, target) {
                    refusals.push((anchor.anchor.span, refusal(cause)));
                } else {
                    resolved.push(target);
                }
            }
            Err(cause) => refusals.push((anchor.anchor.span, refusal(cause))),
        }
    }

    if !refusals.is_empty() {
        // FR-113 "Outputs": refusals in source position order. `sort_by_key`
        // is stable, so refusals at equal spans (the up-front duplicate
        // pass, which emits one per declaration at that declaration's own
        // span) keep their relative order.
        refusals.sort_by_key(|(span, _)| span.start);
        return Err(refusals.into_iter().map(|(_, refusal)| refusal).collect());
    }

    Ok(CheckedProtocol {
        name: protocol.name.name.clone(),
        resolved_anchors: resolved,
    })
}

/// FR-113 checks anchor resolution only: a protocol's types, roles, binder
/// types and block bodies have no checker yet, and nothing emits a checked
/// protocol (QSL-299). A protocol whose anchors all resolve is still kept
/// refused, rather than silently accepted with no diagnostic and dropped
/// from the emitted package (QSL-306 tracks completing protocol checking
/// and emission; the caller pushes this refusal only when `check` above
/// returns `Ok`, so a protocol with an anchor refusal is not refused
/// twice).
pub fn unimplemented(protocol: &ProtocolDeclarationForm) -> CheckRefusal {
    refusal(ProtocolAnchorCause::Unimplemented {
        name: protocol.name.name.clone(),
        span: protocol.name.span,
    })
}

/// Resolves one anchor's segments through nested scopes (FR-113
/// "Resolution").
fn resolve(
    protocol: &ProtocolDeclarationForm,
    by_scope_name: &BTreeMap<(ScopeKey, String), Vec<usize>>,
    anchor: &qsl_forms::ScopedAnchorForm,
) -> Result<ProtocolNodeId, ProtocolAnchorCause> {
    let lexical_scope = scope_key(&anchor.scope);
    // `ScopedAnchorForm`'s fields are public, so a hand-built form (not one
    // S2 built, which the grammar guarantees at least one segment for) can
    // hold no segments; refuse rather than index into an empty slice.
    let Some((first, rest)) = anchor.anchor.segments.split_first() else {
        return Err(ProtocolAnchorCause::Missing {
            segments: Vec::new(),
            segment: String::new(),
            scope: lexical_scope,
            span: anchor.anchor.span,
        });
    };
    let segments: Vec<String> = anchor
        .anchor
        .segments
        .iter()
        .map(|segment| segment.text.clone())
        .collect();

    let mut target_index = None;
    for depth in (0..=lexical_scope.len()).rev() {
        let key = (lexical_scope[..depth].to_vec(), first.text.clone());
        let Some(indices) = by_scope_name.get(&key) else {
            continue;
        };
        if indices.len() > 1 {
            return Err(ambiguous(
                protocol,
                indices,
                &first.text,
                anchor.anchor.span,
            ));
        }
        target_index = Some(indices[0]);
        break;
    }
    let Some(mut target_index) = target_index else {
        return Err(ProtocolAnchorCause::Missing {
            segments: segments.clone(),
            segment: first.text.clone(),
            scope: lexical_scope,
            span: anchor.anchor.span,
        });
    };

    for segment in rest {
        let target = &protocol.declarations[target_index];
        let mut child_scope = scope_key(&target.scope);
        child_scope.push(target.name.name.clone());
        let key = (child_scope, segment.text.clone());
        match by_scope_name.get(&key) {
            Some(indices) if indices.len() > 1 => {
                return Err(ambiguous(
                    protocol,
                    indices,
                    &segment.text,
                    anchor.anchor.span,
                ));
            }
            Some(indices) => target_index = indices[0],
            None => {
                return Err(ProtocolAnchorCause::Missing {
                    segments: segments.clone(),
                    segment: segment.text.clone(),
                    scope: scope_key(&anchor.scope),
                    span: anchor.anchor.span,
                });
            }
        }
    }
    Ok(ProtocolNodeId(target_index))
}

fn ambiguous(
    protocol: &ProtocolDeclarationForm,
    indices: &[usize],
    name: &str,
    span: Span,
) -> ProtocolAnchorCause {
    ProtocolAnchorCause::Ambiguous {
        name: name.to_owned(),
        loci: indices
            .iter()
            .map(|&index| protocol.declarations[index].name.span)
            .collect(),
        span,
    }
}

/// FR-113's wrong-kind and channel-mismatch checks over an anchor already
/// resolved to `target`.
fn check_kind(
    protocol: &ProtocolDeclarationForm,
    anchor: &qsl_forms::ScopedAnchorForm,
    target: ProtocolNodeId,
) -> Option<ProtocolAnchorCause> {
    let declaration = &protocol.declarations[target.0];
    let (site, admitted) = admitted_kinds(anchor.site);
    if !admitted.contains(&declaration.kind) {
        return Some(ProtocolAnchorCause::WrongKind {
            site,
            admitted: admitted.iter().map(|kind| kind.label()).collect(),
            actual: declaration.kind.label(),
            span: anchor.anchor.span,
        });
    }
    if anchor.site == AnchorSite::ReceiveOf {
        if let (Some(receive_channel), Some(send_channel)) =
            (anchor.channel.as_ref(), declaration.channel.as_ref())
        {
            if receive_channel != send_channel {
                return Some(ProtocolAnchorCause::ChannelMismatch {
                    receive_channel: receive_channel.clone(),
                    send_channel: send_channel.clone(),
                    span: anchor.anchor.span,
                });
            }
        }
    }
    None
}

/// FR-113 "Refusals" binder no-shadowing rule: every binder that shadows a
/// model/profile alias, a native declaration of the package or another
/// binder visible where it is declared, in source order.
///
/// Binders are checked in `protocol.binders`' own order (source order,
/// QSL-306's S2 walk), each against every binder built before it anywhere
/// in the protocol: a later binder shadows an earlier one, never the other
/// way round, so only the later binder is refused (FR-113: "refuse ... at
/// that binder, naming the declaration it would shadow"). QSpec
/// `shared-grammar.md` makes binders "unique in their enclosing
/// declaration" -- the whole protocol, not the lexical scope a binder's
/// own `BinderForm::scope` names -- so this check ignores scope entirely
/// when comparing binder against binder; `scope` still locates a binder for
/// FR-114's later binding pass. A package-level alias or native
/// declaration is checked only when no earlier binder in the protocol
/// already shares the name.
fn shadow_refusals(
    protocol: &ProtocolDeclarationForm,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: &Scope,
    signatures: &Signatures,
) -> Vec<(Span, CheckRefusal)> {
    let mut refusals = Vec::new();
    // Every earlier binder's own span, by name, flat across the whole
    // protocol (SR-765 FND-001).
    let mut seen: BTreeMap<String, Span> = BTreeMap::new();
    for binder in &protocol.binders {
        let shadowed = seen
            .get(binder.name.name.as_str())
            .map(|&span| ShadowedDeclaration::Binder(span))
            .or_else(|| {
                package_shadow(
                    &binder.name.name,
                    alias_names,
                    native_names,
                    scope,
                    signatures,
                )
            });
        if let Some(shadowed) = shadowed {
            refusals.push((
                binder.name.span,
                refusal(ProtocolAnchorCause::Shadow {
                    name: binder.name.name.clone(),
                    shadowed,
                    span: binder.name.span,
                }),
            ));
        }
        seen.insert(binder.name.name.clone(), binder.name.span);
    }
    refusals
}

/// Whether `name` is a model/profile alias or a native declaration of the
/// package (SR-765 FND-003: dimensions and units too), and which kind, for
/// [`shadow_refusals`]'s package-wide half of FR-113's no-shadowing rule.
fn package_shadow(
    name: &str,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: &Scope,
    signatures: &Signatures,
) -> Option<ShadowedDeclaration> {
    if let Some(alias) = alias_names.get(name) {
        return Some(match alias {
            AliasKind::Profile => ShadowedDeclaration::ProfileAlias,
            AliasKind::Model => ShadowedDeclaration::ModelAlias,
        });
    }
    if !scope.named_types(name).is_empty() {
        return Some(ShadowedDeclaration::Type);
    }
    if signatures.declares(name) {
        return Some(ShadowedDeclaration::Function);
    }
    if native_names.contains(name) {
        return Some(ShadowedDeclaration::Quantity);
    }
    None
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_forms::{
        build_unit, AnchorSite, FormsLimits, ProtocolDeclarationForm, ProtocolNodeKind,
    };
    use qsl_foundation::SourceIdentity;

    use super::super::{
        CheckCause, CheckRefusal, CheckingLimits, PackageDeclarations, Scope, Signatures,
    };
    use super::{CheckedProtocol, ProtocolAnchorCause, ProtocolNodeId, ShadowedDeclaration};
    use crate::value::declaration::TypeEnvironment;

    const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\" version \"1\" digest \
        \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

    /// S1, S2 and the assembler over `declarations`, no `model` declared:
    /// FR-113's checker resolves protocol node references only, never a
    /// model type, so a bare profile is enough (assemble.rs reads no type
    /// or operation name a protocol declaration holds).
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
        let unit = build_unit(&parsed, FormsLimits::default()).expect("S2 builds the unit");
        let assembled = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            Vec::new(),
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
    /// holds, bypassing `PackageDeclarations::check`'s whole-package pass
    /// (and its FND-001 `Unimplemented` refusal, always added once anchor
    /// resolution succeeds): this is the anchor-resolution algorithm's own
    /// oracle, not the whole pipeline's.
    fn resolve_protocol(
        declarations: &str,
    ) -> (
        String,
        ProtocolDeclarationForm,
        Result<CheckedProtocol, Vec<CheckRefusal>>,
    ) {
        let (text, assembled) = assemble(declarations);
        let protocol = assembled.protocols[0].clone();
        let (scope, signatures) = empty_scope_and_signatures();
        let result = super::check(
            &protocol,
            &std::collections::BTreeMap::new(),
            &std::collections::BTreeSet::new(),
            &scope,
            &signatures,
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
                    && declaration
                        .scope
                        .iter()
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
             within [0,10]; attempts 2 of M::Actor;\n\
             retry (current_attempt: Boolean, prior: Boolean) {{ true }};\n\
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

    /// FR-113-AC-1 (TC-511): every anchor of the `RecoveryFlow` fixture
    /// resolves to the exact node FR-113-AC-1 names, by identity.
    #[trace("TC-511", "FR-113-AC-1")]
    #[test]
    fn every_anchor_of_the_recovery_flow_fixture_resolves_to_its_named_node() {
        let (_, protocol, result) = resolve_protocol(&recovery_flow("Main::Committed", ""));
        let checked = result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
        assert_eq!(
            checked.resolved_anchors.len(),
            protocol.scoped_anchors.len()
        );
        // Source order: compensate-for, compensate-commit, effect-of,
        // event-for (FR-112-AC-1).
        assert_eq!(
            checked.resolved_anchors[0],
            declaration_id(&protocol, "Applied", ProtocolNodeKind::Effect, &["Main"])
        );
        assert_eq!(
            checked.resolved_anchors[1],
            declaration_id(&protocol, "Committed", ProtocolNodeKind::Commit, &["Main"])
        );
        assert_eq!(
            checked.resolved_anchors[2],
            declaration_id(&protocol, "Tried", ProtocolNodeKind::Attempt, &["Main"])
        );
        assert_eq!(
            checked.resolved_anchors[3],
            declaration_id(&protocol, "Undo", ProtocolNodeKind::CompensateTemplate, &[])
        );
    }

    /// FR-113-AC-2 (TC-511): a nested `sequence Inner` inside `Main`, after
    /// the outer `effect Applied`, declares its own `Applied`. An `await`
    /// inside `Inner` naming `Applied` resolves to the *inner* node; one
    /// naming `Main::Applied` resolves to the outer node from inside
    /// `Inner`; and one in `Main` (outside `Inner`) naming `Applied`
    /// resolves to the outer node -- asserted against the two nodes'
    /// distinct identities, not just "no refusal" (a mutation to
    /// outermost-first resolution passes every other test in this module
    /// but fails this one).
    #[trace("TC-511", "FR-113-AC-2")]
    #[test]
    fn a_nested_scope_resolves_its_own_name_over_an_outer_one_of_the_same_spelling() {
        // Every record binder here is uniquely named (`applied_inner`,
        // `seen_inner`/`seen_qualified`/`seen_outer`), distinct from the
        // outer fixture's own `applied`: FR-113's binder no-shadowing rule
        // is now protocol-wide (QSL-306, SR-765 FND-001), and this test's
        // own assertions are about anchor *resolution* through nested and
        // sibling named-control scopes, not about binder shadowing --
        // reusing a binder name here would make the fixture itself refuse.
        let inner = "sequence Inner {\n\
             effect Applied of Tried as (applied_inner: Boolean) { true };\n\
             await WaitInner after Applied using v clock \"ticks\" within [0,1] \
             match event Seen by R for Undo as (seen_inner: Boolean) { true }; \
             then sequence Then { } timeout sequence Timeout { }\n\
             }\n\
             await WaitQualified after Main::Applied using v clock \"ticks\" within [0,1] \
             match event SeenQ by R for Undo as (seen_qualified: Boolean) { true }; \
             then sequence ThenQ { } timeout sequence TimeoutQ { }\n\
             await WaitOuter after Applied using v clock \"ticks\" within [0,1] \
             match event SeenO by R for Undo as (seen_outer: Boolean) { true }; \
             then sequence ThenO { } timeout sequence TimeoutO { }\n";
        let (_, protocol, result) = resolve_protocol(&recovery_flow("Main::Committed", inner));
        let checked = result.unwrap_or_else(|refusals| panic!("{refusals:?}"));

        let outer_applied =
            declaration_id(&protocol, "Applied", ProtocolNodeKind::Effect, &["Main"]);
        let inner_applied = declaration_id(
            &protocol,
            "Applied",
            ProtocolNodeKind::Effect,
            &["Main", "Inner"],
        );
        assert_ne!(outer_applied, inner_applied);

        // Built in this order: `WaitInner` (inside `Inner`), `WaitQualified`
        // and `WaitOuter` (both directly in `Main`).
        let await_afters = anchors_at(&protocol, AnchorSite::AwaitAfter);
        assert_eq!(await_afters.len(), 3, "{await_afters:?}");
        let [wait_inner, wait_qualified, wait_outer] = await_afters[..] else {
            unreachable!()
        };
        assert_eq!(
            checked.resolved_anchors[wait_inner], inner_applied,
            "an unqualified reference inside Inner must resolve to Inner's own Applied"
        );
        assert_eq!(
            checked.resolved_anchors[wait_qualified], outer_applied,
            "Main::Applied must reach the outer node from inside Inner"
        );
        assert_eq!(
            checked.resolved_anchors[wait_outer], outer_applied,
            "an unqualified reference in Main (outside Inner) must resolve to the outer node"
        );
    }

    /// FR-113-AC-3 (TC-511): a missing anchor or member refuses
    /// `missing_declaration`/`missing-name`, naming the failing segment,
    /// the whole anchor's segments and the anchor's own (empty, at the top
    /// level) lexical scope.
    #[trace("TC-511", "FR-113-AC-3")]
    #[test]
    fn a_missing_anchor_or_member_refuses_naming_the_failing_segment() {
        let cases = [
            ("Main::Missing", vec!["Main", "Missing"], "Missing"),
            ("Other::Applied", vec!["Other", "Applied"], "Other"),
        ];
        for (target, segments, segment) in cases {
            let source = recovery_flow("Main::Committed", "").replacen("Main::Applied", target, 1);
            let (_, refusals) = refusals(&source);
            assert_eq!(refusals.len(), 1, "{target}: {refusals:?}");
            let [cause] = protocol_causes(&refusals)[..] else {
                panic!("one cause");
            };
            match cause {
                ProtocolAnchorCause::Missing {
                    segment: found,
                    segments: found_segments,
                    scope,
                    ..
                } => {
                    assert_eq!(found, segment, "{target}");
                    assert_eq!(found_segments, &segments, "{target}");
                    assert!(scope.is_empty(), "{target}: {scope:?}");
                }
                other => panic!("{target}: a Missing cause, got {other:?}"),
            }
        }

        let (_, refusals) = refusals(&recovery_flow("Main::Committed", "").replacen(
            "effect Applied of Tried",
            "effect Applied of Absent",
            1,
        ));
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        let [cause] = protocol_causes(&refusals)[..] else {
            panic!("one cause");
        };
        match cause {
            ProtocolAnchorCause::Missing {
                segment,
                segments,
                scope,
                ..
            } => {
                assert_eq!(segment, "Absent");
                assert_eq!(segments, &["Absent".to_owned()]);
                assert_eq!(scope, &["Main".to_owned()]);
            }
            other => panic!("a Missing cause, got {other:?}"),
        }
    }

    /// The two `effect Applied` declarations' own name spans, in the order
    /// they appear in `text` (not the earlier `compensate ... for
    /// Main::Applied` reference).
    fn duplicate_applied_spans(text: &str) -> [usize; 2] {
        let mut effect_applied = text.match_indices("effect Applied");
        let first = effect_applied.next().unwrap().0 + "effect ".len();
        let second = effect_applied.next().unwrap().0 + "effect ".len();
        [first, second]
    }

    /// Every `Ambiguous` cause of `refusals` names `Applied` and lists its
    /// two loci in exactly `expected` order (FR-113-AC-4: "naming both in
    /// source order").
    fn assert_ambiguous_loci_order(refusals: &[CheckRefusal], expected: [usize; 2]) {
        for cause in protocol_causes(refusals) {
            match cause {
                ProtocolAnchorCause::Ambiguous { name, loci, .. } => {
                    assert_eq!(name, "Applied");
                    let starts: Vec<usize> = loci.iter().map(|span| span.start).collect();
                    assert_eq!(starts, expected, "{cause:?}");
                }
                other => panic!("an Ambiguous cause, got {other:?}"),
            }
        }
    }

    /// FR-113-AC-4 (TC-512): two event nodes named `Applied` in `Main`
    /// refuse `ambiguous_declaration`/`ambiguous-name` at both
    /// declarations, and `Main::Applied` refuses at the anchor, naming
    /// both in source order. Swapping the two declarations swaps nothing
    /// but their order in the refusal.
    #[trace("TC-512", "FR-113-AC-4")]
    #[test]
    fn two_declarations_of_one_name_refuse_at_both_and_at_a_qualified_anchor() {
        let extra = "effect Applied of Tried as (again: Boolean) { true };\n";
        let source = recovery_flow("Main::Committed", extra);
        let (text, found) = refusals(&source);
        // Two duplicate-declaration refusals (one per `Applied`) plus one
        // at the `compensate ... for Main::Applied` anchor, which decides
        // on the ambiguous name.
        assert_eq!(found.len(), 3, "{found:?}");
        for refusal in &found {
            assert_eq!(refusal.cause.code().as_str(), "ambiguous_declaration");
        }
        // One refusal at each `effect Applied` declaration's own name, and
        // one at the `compensate ... for Main::Applied` anchor's whole
        // reference -- in source position order, the anchor first, since it
        // is written before `run sequence Main`.
        let anchor_span = text.find("Main::Applied").unwrap();
        let [first_decl, second_decl] = duplicate_applied_spans(&text);
        let mut expected = [anchor_span, first_decl, second_decl];
        expected.sort_unstable();
        let starts: Vec<usize> = protocol_causes(&found)
            .iter()
            .map(|cause| cause.span().start)
            .collect();
        assert_eq!(starts, expected);
        assert_ambiguous_loci_order(&found, [first_decl, second_decl]);

        // Swap the two declarations' physical order (the `extra` copy,
        // marked `again`, now comes after the fixture's own `effect
        // Applied`, marked `applied`, instead of before it): the same
        // three refusals occur, and the loci list simply swaps to match
        // the new source order.
        assert!(
            text.find("again").unwrap() < text.find("applied").unwrap(),
            "fixture sanity: `again` (extra) must be declared before `applied` (outer)"
        );
        let without_extra = recovery_flow("Main::Committed", "");
        let outer_line = "effect Applied of Tried as (applied: Boolean) { true };\n";
        assert!(without_extra.contains(outer_line));
        let swapped = without_extra.replacen(outer_line, &format!("{outer_line}{extra}"), 1);
        assert_ne!(source, swapped);
        let (swapped_text, swapped_refusals) = refusals(&swapped);
        assert_eq!(swapped_refusals.len(), 3, "{swapped_refusals:?}");
        assert!(
            swapped_text.find("applied").unwrap() < swapped_text.find("again").unwrap(),
            "swap sanity: `applied` (outer) must now be declared before `again` (extra)"
        );
        let [swapped_first, swapped_second] = duplicate_applied_spans(&swapped_text);
        assert_ambiguous_loci_order(&swapped_refusals, [swapped_first, swapped_second]);
    }

    /// A regression fixture distinct from FR-113-AC-6's own (a duplicate
    /// declaration, not a shadowing binder, paired with a missing anchor):
    /// a protocol with a duplicate declaration (physically inside `run`,
    /// so later in source) and an unrelated missing anchor (physically
    /// before `run`) reports both, ordered strictly by source position --
    /// not by the order the checker happens to build them in, which is the
    /// opposite: the up-front duplicate-declaration pass runs before the
    /// per-anchor pass, so without the sort this test's own refusals would
    /// come back in exactly the wrong order. Checking it twice gives the
    /// same refusals in the same order.
    /// [`a_missing_anchor_and_a_later_shadowing_binder_report_in_source_order`]
    /// carries AC-6's own trace tag, over its own fixture (a missing anchor
    /// paired with a shadowing binder, QSL-306).
    #[trace("TC-512")]
    #[test]
    fn a_missing_anchor_and_a_later_duplicate_declaration_report_in_source_order() {
        let extra = "effect Applied of Tried as (again: Boolean) { true };\n";
        let source = recovery_flow("Main::Committed", extra).replacen(
            "for Main::Applied",
            "for Main::Missing",
            1,
        );
        let (text, refusals_a) = refusals(&source);
        let (_, refusals_b) = refusals(&source);
        assert_eq!(refusals_a, refusals_b, "checking twice must agree");
        assert_eq!(refusals_a.len(), 3, "{refusals_a:?}");

        let missing_span = text.find("Main::Missing").unwrap();
        let [first_decl, second_decl] = duplicate_applied_spans(&text);
        // The missing-anchor refusal is textually first (before `run`), so
        // it must sort first even though the checker builds the two
        // duplicate-declaration refusals before it.
        let mut expected = [missing_span, first_decl, second_decl];
        expected.sort_unstable();
        assert_eq!(
            expected[0], missing_span,
            "fixture sanity: missing must be earliest"
        );
        let starts: Vec<usize> = protocol_causes(&refusals_a)
            .iter()
            .map(|cause| cause.span().start)
            .collect();
        assert_eq!(
            starts, expected,
            "refusals are not in source position order"
        );
        assert!(
            matches!(
                protocol_causes(&refusals_a)[0],
                ProtocolAnchorCause::Missing { .. }
            ),
            "the earliest refusal must be the Missing one, {:?}",
            refusals_a[0]
        );
    }

    /// FR-113-AC-7 (TC-512): a wrong-kind target refuses
    /// `ill_typed`/`type-mismatch` naming the site, the admitted kinds and
    /// the actual kind, for each of the six reference positions; a
    /// mismatched channel refuses the same way, naming both channels, and
    /// a matching channel resolves.
    #[trace("TC-512", "FR-113-AC-7")]
    #[test]
    fn a_resolved_anchor_of_the_wrong_kind_refuses_naming_the_actual_kind() {
        // effect-of must admit only `attempt`; naming `Recovered` (an
        // `event`, FR-113's own example) instead of `Tried` (the
        // `attempt`) is wrong-kind.
        let wrong_effect_of = recovery_flow("Main::Committed", "").replacen(
            "effect Applied of Tried",
            "effect Applied of Recovered",
            1,
        );
        let (_, found) = refusals(&wrong_effect_of);
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "effect-of");
                assert_eq!(*actual, "event");
                assert_eq!(admitted.as_slice(), ["attempt"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // compensate-for must admit only `effect`; naming `Tried` (an
        // `attempt`) is wrong-kind.
        let wrong_compensate_for =
            recovery_flow("Main::Committed", "").replacen("Main::Applied", "Main::Tried", 1);
        let (_, found) = refusals(&wrong_compensate_for);
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "compensate-for");
                assert_eq!(*actual, "attempt");
                assert_eq!(admitted.as_slice(), ["effect"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // compensate-commit must admit only `commit`; naming `Tried` (an
        // `attempt`) is wrong-kind.
        let wrong_compensate_commit = recovery_flow("Main::Tried", "");
        let (_, found) = refusals(&wrong_compensate_commit);
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "compensate-commit");
                assert_eq!(*actual, "attempt");
                assert_eq!(admitted.as_slice(), ["commit"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // event-for must admit only a `compensate` template; naming
        // `Tried` (an `attempt`) is wrong-kind.
        let wrong_event_for =
            recovery_flow("Main::Committed", "").replacen("for Undo", "for Tried", 1);
        let (_, found) = refusals(&wrong_event_for);
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "event-for");
                assert_eq!(*actual, "attempt");
                assert_eq!(admitted.as_slice(), ["compensate"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // await-after admits every event-like kind and a compensate
        // template, but never a structural control: naming `Main` (a
        // `sequence`) is wrong-kind.
        let awaiting_a_sequence = "await Wait after Main using v clock \"ticks\" within [0,1] \
             match event Seen by R for Undo as (seen: Boolean) { true }; \
             then sequence Then { } timeout sequence Timeout { }\n";
        let (_, found) = refusals(&recovery_flow("Main::Committed", awaiting_a_sequence));
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "await-after");
                assert_eq!(*actual, "sequence");
                assert!(admitted.contains(&"send"));
                assert!(!admitted.contains(&"sequence"));
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // receive-of must admit only `send`; naming `Tried` (an `attempt`)
        // is wrong-kind.
        let wrong_receive_of = "receive Got via C of Tried as (got: Boolean) { true };\n";
        let (_, found) = refusals(&recovery_flow("Main::Committed", wrong_receive_of));
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "receive-of");
                assert_eq!(*actual, "attempt");
                assert_eq!(admitted.as_slice(), ["send"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // A channel mismatch between a matching-kind `receive` and `send`
        // refuses `type-mismatch` naming both channels; the same `receive`
        // over the matching channel resolves.
        let (_, _, result) = resolve_protocol(&handoff_protocol("C"));
        result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
        let (_, found) = refusals(&handoff_protocol("D"));
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::ChannelMismatch {
                receive_channel,
                send_channel,
                ..
            } => {
                assert_eq!(receive_channel, "D");
                assert_eq!(send_channel, "C");
            }
            other => panic!("a ChannelMismatch cause, got {other:?}"),
        }
    }

    /// SR-753/SR-754-style mutation check, over a path
    /// `a_resolved_anchor_of_the_wrong_kind_...` does not itself exercise:
    /// flipping the channel comparison to always agree (as an unresolved
    /// check would) must turn this test red, and reverting must turn it
    /// green again.
    #[trace("TC-512", "FR-113-AC-7")]
    #[test]
    fn a_channel_mismatch_refusal_is_not_vacuous_over_an_otherwise_valid_protocol() {
        let matching = handoff_protocol("C");
        let (_, _, result) = resolve_protocol(&matching);
        result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
        let mutated = handoff_protocol("D");
        assert_ne!(matching, mutated);
        let (_, refusals) = refusals(&mutated);
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(matches!(
            protocol_causes(&refusals)[0],
            ProtocolAnchorCause::ChannelMismatch { .. }
        ));
    }

    /// SR-761/SR-762 FND-001: a protocol whose anchors all resolve is still
    /// refused `unsupported_construct`/`not-yet-implemented`, naming the
    /// protocol, since nothing checks its other content or emits it yet
    /// (QSL-306) -- proving the full pipeline never silently accepts a
    /// protocol, unlike the defect this replaces (SR-753 FND-002).
    #[trace("TC-511", "FR-113")]
    #[test]
    fn a_protocol_whose_anchors_all_resolve_still_refuses_as_unimplemented() {
        let source = recovery_flow("Main::Committed", "");
        // The anchor-resolution algorithm itself succeeds...
        let (_, _, result) = resolve_protocol(&source);
        result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
        // ...but the full pipeline still refuses, since nothing else checks
        // or emits this protocol yet.
        let (_, refusals) = refusals(&source);
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        match protocol_causes(&refusals)[0] {
            ProtocolAnchorCause::Unimplemented { name, .. } => {
                assert_eq!(name, "RecoveryFlow");
            }
            other => panic!("an Unimplemented cause, got {other:?}"),
        }
        assert_eq!(refusals[0].cause.code().as_str(), "unsupported_construct");
        assert_eq!(refusals[0].cause.cause(), Some("not-yet-implemented"));
    }

    /// The `Shadow` cause of `refusals`, expecting exactly one.
    fn shadow_cause(refusals: &[CheckRefusal]) -> (&str, &ShadowedDeclaration) {
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        match protocol_causes(refusals)[0] {
            ProtocolAnchorCause::Shadow { name, shadowed, .. } => (name, shadowed),
            other => panic!("a Shadow cause, got {other:?}"),
        }
    }

    /// FR-113-AC-5 (TC-512), first case: a `capture` named `forward` inside
    /// `Undo`, where `forward` is already the template's own bound
    /// parameter (`as (forward: Boolean)`, renamed from the fixture's own
    /// `failure`), refuses `ambiguous_declaration`/`ambiguous-name` at the
    /// capture, naming the parameter it shadows.
    #[trace("TC-512", "FR-113-AC-5")]
    #[test]
    fn a_capture_shadowing_its_templates_own_bound_parameter_refuses() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("as (failure: Boolean)", "as (forward: Boolean)", 1)
            .replacen(
                "clock \"ticks\" {\nactivate first",
                "clock \"ticks\" {\ncapture forward: Boolean = true;\nactivate first",
                1,
            );
        assert!(
            source.contains("capture forward: Boolean = true;"),
            "fixture sanity: the capture must be inserted"
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].cause.code().as_str(), "ambiguous_declaration");
        assert_eq!(found[0].cause.cause(), Some("ambiguous-name"));
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "forward");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    /// FR-113-AC-5 (TC-512), second case: an event record binder named
    /// after the unit's own `v` profile alias (FR-113 "Refusals" reaches a
    /// model or profile alias the same way; this fixture declares no
    /// `model`, so its `v` profile alias stands in for a model alias)
    /// refuses at the binder, naming the alias. A second, unrelated
    /// protocol in the same unit whose own record binder is also `v`
    /// checks (FR-113 "Refusals": "Two separate protocols may reuse a
    /// binder name" -- this asserts the alias/native-declaration half is
    /// package-wide while a binder's own shadow check stays local to its
    /// protocol).
    #[trace("TC-512", "FR-113-AC-5")]
    #[test]
    fn a_record_binder_shadowing_the_units_profile_alias_refuses() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "as (applied: Boolean)",
            "as (v: Boolean)",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].cause.code().as_str(), "ambiguous_declaration");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "v");
        assert!(
            matches!(shadowed, ShadowedDeclaration::ProfileAlias),
            "{shadowed:?}"
        );

        // A second protocol, naming its own `effect`'s record binder `v`
        // too: this protocol's own anchors and declarations are entirely
        // separate from the first, but `v` is still package-wide, so it
        // still refuses -- the "two separate protocols" carve-out is about
        // reusing a binder name *between* protocols, not about escaping the
        // package-level alias/native-declaration check.
        let second = recovery_flow("Main::Committed", "")
            .replacen("protocol RecoveryFlow", "protocol RecoveryFlowTwo", 1)
            .replacen("as (applied: Boolean)", "as (v: Boolean)", 1);
        let combined = format!("{source}\n{second}");
        let (_, found) = refusals(&combined);
        let shadow_causes: Vec<&ProtocolAnchorCause> = protocol_causes(&found)
            .into_iter()
            .filter(|cause| matches!(cause, ProtocolAnchorCause::Shadow { .. }))
            .collect();
        assert_eq!(shadow_causes.len(), 2, "{shadow_causes:?}");
        for cause in shadow_causes {
            match cause {
                ProtocolAnchorCause::Shadow { name, shadowed, .. } => {
                    assert_eq!(name, "v");
                    assert!(matches!(shadowed, ShadowedDeclaration::ProfileAlias));
                }
                other => panic!("a Shadow cause, got {other:?}"),
            }
        }
    }

    /// A `compensate` declaration's `activate first (p)` trigger parameter
    /// shadowing a native `predicate` declaration of the package (FR-113
    /// "Refusals": a native declaration is checked the same way an alias
    /// is, `ShadowedDeclaration::Function`).
    #[trace("TC-512")]
    #[test]
    fn a_trigger_parameter_shadowing_a_native_function_declaration_refuses() {
        let source = format!(
            "predicate forward using v(): Boolean {{ true }}\n{}",
            recovery_flow("Main::Committed", "").replacen(
                "(trigger: Boolean)",
                "(forward: Boolean)",
                1
            )
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "forward");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Function),
            "{shadowed:?}"
        );
    }

    /// A `retry (a, b)` parameter shadowing another binder visible where it
    /// is declared: `Undo`'s own record binder, renamed `shared`, and
    /// `retry`'s first parameter, also renamed `shared` -- both inside the
    /// same `compensate` declaration, the retry parameter one scope level
    /// inside the declaration's own record binder.
    #[trace("TC-512")]
    #[test]
    fn a_retry_parameter_shadowing_its_declarations_own_record_binder_refuses() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("as (failure: Boolean)", "as (shared: Boolean)", 1)
            .replacen("current_attempt: Boolean", "shared: Boolean", 1);
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "shared");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    /// A bare `commit` control's own record binder shadowing the unit's `v`
    /// profile alias, spreading FR-113-AC-5's coverage across a binder
    /// position AC-5's own fixture does not exercise.
    #[trace("TC-512")]
    #[test]
    fn a_commit_controls_record_binder_shadowing_an_alias_refuses() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "as (committed: Boolean)",
            "as (v: Boolean)",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "v");
        assert!(
            matches!(shadowed, ShadowedDeclaration::ProfileAlias),
            "{shadowed:?}"
        );
    }

    /// FR-113-AC-6 (TC-512 step 3, exactly): `effect Applied of Absent` (a
    /// missing member, TC-512's own step-3 fixture, physically inside
    /// `run`) combined with the shadowing `forward` capture (TC-512 step
    /// 2's own fixture: `Undo` binds `forward` as its record binder, and a
    /// `capture forward = ...;` inside it shadows that binder), physically
    /// inside `compensate Undo`, so earlier in source. Both refusals
    /// report, ordered by source position (SR-766 FND-004: this fixture
    /// used to differ from TC-512 step 3's own procedure -- a
    /// `Main::Missing` anchor and a `v`-named record binder shadowing the
    /// unit's own alias, neither of which step 3 names -- so it is now the
    /// same fixture step 3 describes). Checking it twice gives the same
    /// refusals in the same order, and no checked protocol node emits (the
    /// full pipeline refuses).
    #[trace("TC-512", "FR-113-AC-6")]
    #[test]
    fn a_missing_anchor_and_a_later_shadowing_binder_report_in_source_order() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("effect Applied of Tried", "effect Applied of Absent", 1)
            .replacen("as (failure: Boolean)", "as (forward: Boolean)", 1)
            .replacen(
                "clock \"ticks\" {\nactivate first",
                "clock \"ticks\" {\ncapture forward: Boolean = true;\nactivate first",
                1,
            );
        let (text, refusals_a) = refusals(&source);
        let (_, refusals_b) = refusals(&source);
        assert_eq!(refusals_a, refusals_b, "checking twice must agree");
        assert_eq!(refusals_a.len(), 2, "{refusals_a:?}");

        let shadow_span = text.find("capture forward").unwrap();
        let missing_span = text.find("effect Applied of Absent").unwrap();
        assert!(
            shadow_span < missing_span,
            "fixture sanity: the shadowing capture must come first in source"
        );
        let causes = protocol_causes(&refusals_a);
        assert!(
            matches!(causes[0], ProtocolAnchorCause::Shadow { .. }),
            "the earliest refusal must be the Shadow one, {:?}",
            refusals_a[0]
        );
        assert!(
            matches!(causes[1], ProtocolAnchorCause::Missing { .. }),
            "the later refusal must be the Missing one, {:?}",
            refusals_a[1]
        );
    }

    /// SR-765 FND-001: FR-113's binder no-shadowing rule is protocol-wide
    /// (QSpec `shared-grammar.md`: binders are "unique in their enclosing
    /// declaration"), not merely within scopes that lexically nest one
    /// inside the other. Three cases the old lexical-prefix check missed,
    /// each a fresh `RecoveryFlow`-shaped protocol so one refusal per case
    /// stays unambiguous:
    ///
    /// - `finish`'s own record binder reusing `run`'s own `attempt`
    ///   binder's name (neither scope is a prefix of the other: `finish` is
    ///   top-level, `attempt` is scoped `["Main"]`).
    /// - A `sequence Inner` inside `Main`, whose own `effect`'s record
    ///   binder reuses a name already bound by `Main`'s own outer `effect`.
    /// - Two sibling `case`s of one `choice`, each binding an event record
    ///   binder of the same name (neither `case`'s scope is a prefix of the
    ///   other's).
    #[trace("TC-512", "FR-113")]
    #[test]
    fn a_finish_binder_reusing_a_run_binders_name_refuses() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "as (outcome: Boolean)",
            "as (tried: Boolean)",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "tried");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    #[trace("TC-512", "FR-113")]
    #[test]
    fn a_nested_sequences_binder_reusing_an_outer_siblings_name_refuses() {
        let inner = "sequence Inner {\n\
             event Recovered2 by R for Undo as (tried: Boolean) { true };\n\
             }\n";
        let source = recovery_flow("Main::Committed", inner);
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "tried");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    #[trace("TC-512", "FR-113")]
    #[test]
    fn sibling_case_binders_of_one_name_refuse() {
        let choice = "choice Which by R visible () {\n\
             case left when { true } sequence Left {\n\
             event LeftSeen by R for Undo as (dup: Boolean) { true };\n\
             }\n\
             case right when { false } sequence Right {\n\
             event RightSeen by R for Undo as (dup: Boolean) { true };\n\
             }\n\
             }\n";
        let source = recovery_flow("Main::Committed", choice);
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "dup");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    /// SR-765 FND-002: the protocol's own `over (p)` input parameter and
    /// `activation on each (p)` parameter are binders too, visible
    /// everywhere in the protocol (`BinderKind::Input` and
    /// `BinderKind::ActivationParameter`); a record binder reusing either
    /// name refuses.
    #[trace("TC-510", "FR-113")]
    #[test]
    fn a_record_binder_shadowing_the_protocols_own_input_parameter_refuses() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "as (tried: Boolean)",
            "as (input: Boolean)",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "input");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    #[trace("TC-510", "FR-113")]
    #[test]
    fn a_record_binder_shadowing_the_protocols_own_activation_parameter_refuses() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("on origin", "on each (activated: Boolean) when (true)", 1)
            .replacen("as (tried: Boolean)", "as (activated: Boolean)", 1);
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "activated");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    /// SR-765 FND-003: a `dimension`/`unit` declaration participates in the
    /// package-wide half of FR-113's no-shadowing rule the same way an
    /// alias or a function does, not just `Scope::named_types`'s composites,
    /// enums and object types.
    #[trace("TC-512", "FR-113")]
    #[test]
    fn a_record_binder_shadowing_a_native_dimension_declaration_refuses() {
        let source = format!("dimension tried;\n{}", recovery_flow("Main::Committed", ""));
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "tried");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Quantity),
            "{shadowed:?}"
        );
    }

    // SR-766 FND-001's model-alias regression (the `.chain(selections.
    // models...)` half of `PackageDeclarations::alias_names`'s assembly)
    // lives in `qsl-semantics/tests/it/model_operations.rs`
    // (`a_models_own_alias_is_recorded_in_the_packages_alias_names`), whose
    // existing `admit_and_assemble` pipeline admits a real domain package;
    // this module's own `assemble` test helper has no such admission and
    // refuses `UnadmittedModel` for any `model` declaration.

    /// SR-766 FND-002: AC-5's own "two separate protocols" carve-out, tested
    /// directly rather than only implicitly (the pre-existing test's second
    /// protocol reused the alias `v` itself, so the carve-out was never
    /// actually exercised on an ordinary, non-alias binder name). Two
    /// protocols in one unit, each with a record binder named `shared` (not
    /// an alias or any native declaration): neither refuses, since a
    /// binder's own protocol-wide uniqueness check (SR-765 FND-001) stays
    /// scoped to its own protocol.
    #[trace("TC-512", "FR-113-AC-5")]
    #[test]
    fn two_protocols_sharing_an_ordinary_binder_name_both_check() {
        let first = recovery_flow("Main::Committed", "").replacen(
            "as (tried: Boolean)",
            "as (shared: Boolean)",
            1,
        );
        let second = recovery_flow("Main::Committed", "")
            .replacen("protocol RecoveryFlow", "protocol RecoveryFlowTwo", 1)
            .replacen("as (tried: Boolean)", "as (shared: Boolean)", 1);
        let combined = format!("{first}\n{second}");
        let (text, assembled) = assemble(&combined);
        // Both protocols still refuse `unsupported_construct` (QSL-306:
        // nothing emits a protocol yet), but neither refuses `Shadow`.
        let refusals = assembled
            .check(CheckingLimits::default())
            .expect_err("still refused as unimplemented");
        assert_eq!(refusals.len(), 2, "{text}: {refusals:?}");
        for refusal in &refusals {
            assert_eq!(refusal.cause.code().as_str(), "unsupported_construct");
        }
    }
}
