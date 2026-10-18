//! FR-113's binder no-shadowing rule: a binder shadowing a model/profile
//! alias, a native declaration of the package or an earlier binder of the
//! same protocol.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::ProtocolDeclarationForm;
use qsl_foundation::Span;

use super::refusal;
use crate::check::refusal::{AliasKind, CheckRefusal, ProtocolAnchorCause, ShadowedDeclaration};
use crate::check::{Scope, Signatures};

/// FR-113 "Refusals" binder no-shadowing rule: every binder that shadows a
/// model/profile alias, a native declaration of the package or another
/// binder visible where it is declared, in source order.
///
/// Binders are checked in `protocol.binders`' own order (source order,
/// the S2 walk), each against every binder built before it anywhere
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
pub(super) fn shadow_refusals(
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
