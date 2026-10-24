//! FR-113 anchor resolution: scope keys, the nested-scope lookup of one
//! scoped anchor, and the wrong-kind and channel-mismatch checks over its
//! resolved target.

use std::collections::BTreeMap;

use qsl_forms::{AnchorSite, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::Span;

use super::ProtocolNodeId;
use crate::check::refusal::ProtocolAnchorCause;

/// A declaration scope, keyed the way FR-113 resolves a segment: the
/// enclosing named controls' spelled names, outermost first.
pub(super) type ScopeKey = Vec<String>;

pub(super) fn scope_key(
    protocol: &ProtocolDeclarationForm,
    scope: Option<qsl_forms::ScopeId>,
) -> ScopeKey {
    protocol
        .scope_names(scope)
        .into_iter()
        .map(|name| name.name.clone())
        .collect()
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

/// Resolves one anchor's segments through nested scopes (FR-113
/// "Resolution").
pub(super) fn resolve(
    protocol: &ProtocolDeclarationForm,
    by_scope_name: &BTreeMap<(ScopeKey, String), Vec<usize>>,
    anchor: &qsl_forms::ScopedAnchorForm,
) -> Result<ProtocolNodeId, ProtocolAnchorCause> {
    let lexical_scope = scope_key(protocol, anchor.scope);
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
        let mut child_scope = scope_key(protocol, target.scope);
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
                    scope: scope_key(protocol, anchor.scope),
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
pub(super) fn check_kind(
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
        if let (Some(receive_channel), Some(send_channel)) = (
            anchor.channel.as_ref(),
            declaration.channel.as_ref().map(|channel| &channel.name),
        ) {
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
