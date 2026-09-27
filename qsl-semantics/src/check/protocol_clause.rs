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
//! FR-113's binder no-shadowing rule (record binders, captures, retry and
//! recovery parameters shadowing a model/profile alias, a native
//! declaration or another visible binder) is not implemented here: no S2
//! form carries a binder's name today (FR-112 builds only scoped anchors),
//! so checking it needs a further S2 extension this ticket does not make.
//! See QSL-298's PR description for this boundary.

use std::collections::BTreeMap;

use qsl_forms::{AnchorSite, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::Span;

use super::refusal::{CheckCause, CheckRefusal, Origin, ProtocolAnchorCause};

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
/// refusals first at equal span, then per-anchor refusals).
pub fn check(protocol: &ProtocolDeclarationForm) -> Result<CheckedProtocol, Vec<CheckRefusal>> {
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

/// Resolves one anchor's segments through nested scopes (FR-113
/// "Resolution").
fn resolve(
    protocol: &ProtocolDeclarationForm,
    by_scope_name: &BTreeMap<(ScopeKey, String), Vec<usize>>,
    anchor: &qsl_forms::ScopedAnchorForm,
) -> Result<ProtocolNodeId, ProtocolAnchorCause> {
    let lexical_scope = scope_key(&anchor.scope);
    let first = &anchor.anchor.segments[0];

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
            segment: first.text.clone(),
            scope: lexical_scope,
            span: anchor.anchor.span,
        });
    };

    for segment in &anchor.anchor.segments[1..] {
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

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_forms::{build_unit, FormsLimits};
    use qsl_foundation::SourceIdentity;

    use super::super::{CheckCause, CheckRefusal, CheckingLimits, PackageDeclarations};
    use super::ProtocolAnchorCause;

    const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\" version \"1\" digest \
        \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

    /// S1, S2, the assembler and `check` over `declarations`, no `model`
    /// declared: FR-113's checker resolves protocol node references only,
    /// never a model type, so a bare profile is enough (assemble.rs reads
    /// no type or operation name a protocol declaration holds).
    fn check(declarations: &str) -> (String, Result<PackageDeclarations, Vec<CheckRefusal>>) {
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
        let result = assembled
            .clone()
            .check(CheckingLimits::default())
            .map(|_| assembled);
        (text, result)
    }

    fn refusals(declarations: &str) -> (String, Vec<CheckRefusal>) {
        let (text, result) = check(declarations);
        match result {
            Err(refusals) => (text, refusals),
            Ok(_) => panic!("{declarations}: checking refuses"),
        }
    }

    fn checks(declarations: &str) {
        let (_, result) = check(declarations);
        result.unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"));
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

    /// The `RecoveryFlow` shape TC-510/511's own fixture uses (matching
    /// `qsl-forms`'s `recovery_flow`), with `Main::Applied`/`Main::Committed`
    /// resolved through the `compensate` template and `Tried`/`Undo`
    /// resolved from inside `Main`.
    fn recovery_flow(commit: &str, main_extra: &str) -> String {
        format!(
            "protocol RecoveryFlow using v over (input: Boolean) on origin {{\n\
             role R on M::Actor;\n\
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

    /// FR-113-AC-1 (TC-511): every anchor of the `RecoveryFlow` fixture
    /// resolves, and the protocol checks with no refusal.
    #[trace("TC-511", "FR-113-AC-1")]
    #[test]
    fn every_anchor_of_the_recovery_flow_fixture_resolves() {
        checks(&recovery_flow("Main::Committed", ""));
    }

    /// FR-113-AC-2 (TC-511): a nested `sequence Inner` inside `Main`, after
    /// the outer `effect Applied`, shadows `Applied` for a reference inside
    /// `Inner`; a qualified `Main::Applied` still reaches the outer node
    /// from inside `Inner`, and an unqualified reference in `Main` (outside
    /// `Inner`) still reaches the outer node.
    #[trace("TC-511", "FR-113-AC-2")]
    #[test]
    fn a_nested_scope_resolves_its_own_name_over_an_outer_one_of_the_same_spelling() {
        let inner = "sequence Inner {\n\
             effect Applied of Tried as (applied: Boolean) { true };\n\
             await WaitInner after Applied using v clock \"ticks\" within [0,1] \
             match event Seen by R for Undo as (seen: Boolean) { true }; \
             then sequence Then { } timeout sequence Timeout { }\n\
             }\n\
             await WaitQualified after Main::Applied using v clock \"ticks\" within [0,1] \
             match event SeenQ by R for Undo as (seen: Boolean) { true }; \
             then sequence ThenQ { } timeout sequence TimeoutQ { }\n\
             await WaitOuter after Applied using v clock \"ticks\" within [0,1] \
             match event SeenO by R for Undo as (seen: Boolean) { true }; \
             then sequence ThenO { } timeout sequence TimeoutO { }\n";
        checks(&recovery_flow("Main::Committed", inner));
    }

    /// FR-113-AC-3 (TC-511): a missing anchor or member refuses
    /// `missing_declaration`/`missing-name`, naming the failing segment.
    #[trace("TC-511", "FR-113-AC-3")]
    #[test]
    fn a_missing_anchor_or_member_refuses_naming_the_failing_segment() {
        let cases = [
            ("Main::Missing", "for", "Missing"),
            ("Other::Applied", "for", "Other"),
        ];
        for (target, _site, segment) in cases {
            let source = recovery_flow("Main::Committed", "").replacen("Main::Applied", target, 1);
            let (_, refusals) = refusals(&source);
            assert_eq!(refusals.len(), 1, "{target}: {refusals:?}");
            let [cause] = protocol_causes(&refusals)[..] else {
                panic!("one cause");
            };
            match cause {
                ProtocolAnchorCause::Missing { segment: found, .. } => {
                    assert_eq!(found, segment, "{target}");
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
            ProtocolAnchorCause::Missing { segment, scope, .. } => {
                assert_eq!(segment, "Absent");
                assert_eq!(scope, &["Main".to_owned()]);
            }
            other => panic!("a Missing cause, got {other:?}"),
        }
    }

    /// FR-113-AC-4 (TC-512): two event nodes named `Applied` in `Main`
    /// refuse `ambiguous_declaration`/`ambiguous-name` at both
    /// declarations, and `Main::Applied` refuses at the anchor, naming
    /// both in source order.
    #[trace("TC-512", "FR-113-AC-4")]
    #[test]
    fn two_declarations_of_one_name_refuse_at_both_and_at_a_qualified_anchor() {
        let extra = "effect Applied of Tried as (again: Boolean) { true };\n";
        let source = recovery_flow("Main::Committed", extra);
        let (text, refusals) = refusals(&source);
        // Two duplicate-declaration refusals (one per `Applied`) plus one
        // at the `compensate ... for Main::Applied` anchor, which decides
        // on the ambiguous name.
        assert_eq!(refusals.len(), 3, "{refusals:?}");
        let causes = protocol_causes(&refusals);
        let ambiguous_names: Vec<&str> = causes
            .iter()
            .map(|cause| match cause {
                ProtocolAnchorCause::Ambiguous { name, loci, .. } => {
                    assert_eq!(loci.len(), 2, "{cause:?}");
                    name.as_str()
                }
                other => panic!("an Ambiguous cause, got {other:?}"),
            })
            .collect();
        assert_eq!(ambiguous_names, ["Applied", "Applied", "Applied"]);
        for refusal in &refusals {
            assert_eq!(refusal.cause.code().as_str(), "ambiguous_declaration");
        }
        // One refusal at each `effect Applied` declaration's own name, and
        // one at the `compensate ... for Main::Applied` anchor's whole
        // reference -- in source position order, the anchor first, since it
        // is written before `run sequence Main`.
        let anchor_span = text.find("Main::Applied").unwrap();
        let mut effect_applied = text.match_indices("effect Applied");
        let first_decl = effect_applied.next().unwrap().0 + "effect ".len();
        let second_decl = effect_applied.next().unwrap().0 + "effect ".len();
        let mut expected = [anchor_span, first_decl, second_decl];
        expected.sort_unstable();
        let starts: Vec<usize> = causes.iter().map(|cause| cause.span().start).collect();
        assert_eq!(starts, expected);
    }

    /// FR-113-AC-6 (TC-512, adapted): a protocol with one missing anchor
    /// and one wrong-kind anchor reports both refusals, ordered by source
    /// position, and checking it twice gives the same refusals in the same
    /// order. (FR-113-AC-6's own fixture pairs a missing anchor with a
    /// shadowing binder; the shadowing rule is not implemented by this
    /// checker -- see this module's own doc comment -- so this test pairs
    /// the missing anchor with a wrong-kind anchor instead, to exercise the
    /// same "two independent refusals, both reported, deterministically
    /// ordered" behavior.)
    #[trace("TC-512", "FR-113-AC-6")]
    #[test]
    fn two_independent_refusals_both_report_in_a_stable_order() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("for Main::Applied", "for Main::Missing", 1)
            .replacen("of Tried as (applied", "of Main as (applied", 1);
        let (_, refusals_a) = refusals(&source);
        let (_, refusals_b) = refusals(&source);
        assert_eq!(refusals_a, refusals_b);
        assert_eq!(refusals_a.len(), 2, "{refusals_a:?}");
        let mut starts: Vec<usize> = protocol_causes(&refusals_a)
            .iter()
            .map(|cause| cause.span().start)
            .collect();
        let mut sorted = starts.clone();
        sorted.sort_unstable();
        assert_eq!(starts, sorted, "refusals are not in source position order");
        starts.dedup();
        assert_eq!(starts.len(), 2, "the two refusals collided at one span");
    }

    /// FR-113-AC-7 (TC-512): a wrong-kind target refuses
    /// `ill_typed`/`type-mismatch` naming the site, the admitted kinds and
    /// the actual kind, for each of the six reference positions; a
    /// mismatched channel refuses the same way, naming both channels, and
    /// a matching channel checks.
    #[trace("TC-512", "FR-113-AC-7")]
    #[test]
    fn a_resolved_anchor_of_the_wrong_kind_refuses_naming_the_actual_kind() {
        // effect-of must admit only `attempt`; naming `Applied` (an
        // `effect`) instead of `Tried` (the `attempt`) is wrong-kind.
        let wrong_effect_of = recovery_flow("Main::Committed", "").replacen(
            "effect Applied of Tried",
            "effect Applied of Applied",
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
                assert_eq!(*actual, "effect");
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
        // is wrong-kind, and a channel mismatch between a matching-kind
        // `receive` and `send` refuses `type-mismatch` naming both
        // channels; the same `receive` over the matching channel checks.
        let with_send_and_receive = |channel: &str| {
            format!(
                "protocol Handoff using v over (input: Boolean) on origin {{\n\
                 role R on M::Actor;\n\
                 run sequence Main {{\n\
                 send Ping via C as (ping: Boolean) {{ true }};\n\
                 receive Got via {channel} of Ping as (got: Boolean) {{ true }};\n\
                 }}\n\
                 finish End as (outcome: Boolean) {{ true }};\n\
                 }}"
            )
        };
        checks(&with_send_and_receive("C"));
        let (_, found) = refusals(&with_send_and_receive("D"));
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
    }

    /// SR-753/SR-754-style mutation check: flipping this checker's
    /// admitted-kind test back to accept every kind (as an unresolved
    /// `Ok(())` would) must turn `a_resolved_anchor_of_the_wrong_kind_...`
    /// red, and reverting must turn it green again -- exercised here
    /// directly, not by literally editing the source: a target one step
    /// removed from the resolved anchor's own scope must still refuse, so
    /// the check is not vacuously satisfied by "no anchor resolves".
    #[trace("TC-512", "FR-113-AC-7")]
    #[test]
    fn a_wrong_kind_refusal_is_not_vacuous_over_an_otherwise_valid_protocol() {
        let base = recovery_flow("Main::Committed", "");
        checks(&base);
        let mutated = base.replacen("effect Applied of Tried", "effect Applied of Applied", 1);
        assert_ne!(base, mutated);
        let (_, refusals) = refusals(&mutated);
        assert_eq!(refusals.len(), 1);
    }
}
