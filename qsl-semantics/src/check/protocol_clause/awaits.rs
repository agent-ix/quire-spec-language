//! An `await`'s own local checks (QSpec FR-466 "Per-operator checks" 2 and
//! 6, QSpec FR-464 "Profiles and laws"): the awaited event is a `receive`,
//! an `effect` or an `event`; `within` is a closed interval with
//! `lower <= upper`; and, since an `await` reads a clock role, the unit
//! selects a `temporal_profile`.

use qsl_forms::{IntervalUpper, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::Span;

use super::refusal;
use super::types::{CheckedAwait, ProtocolNodeId};
use crate::check::protocol_obligation::CheckedDeadline;
use crate::check::refusal::{CheckRefusal, ProtocolAnchorCause};

/// The profile selections an `await` reads: the selected protocol
/// profile's identity, which an awaited-form refusal names, and whether the
/// unit selects any `temporal_profile`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Selections<'a> {
    /// The selected protocol profile's identity, when one is selected.
    pub protocol: Option<&'a str>,
    /// Whether the unit's header selects a temporal profile.
    pub temporal: bool,
}

/// Every `await` of `protocol` checked, in source order, or the refusals
/// of the ones that are not, each with its span.
pub(super) fn check(
    protocol: &ProtocolDeclarationForm,
    selections: Selections<'_>,
) -> Result<Vec<CheckedAwait>, Vec<(Span, CheckRefusal)>> {
    let mut checked = Vec::with_capacity(protocol.awaits.len());
    let mut refusals = Vec::new();
    for form in &protocol.awaits {
        let name = &form.name.name;
        let at = form.name.span;
        if !selections.temporal {
            refusals.push((
                at,
                refusal(ProtocolAnchorCause::MissingTemporalProfile {
                    node: name.clone(),
                    span: at,
                }),
            ));
        }
        let event = protocol
            .declarations
            .iter()
            .find(|declaration| declaration.name.span == form.matched.span);
        if let Some(event) = event {
            match event.kind {
                ProtocolNodeKind::Receive | ProtocolNodeKind::Effect | ProtocolNodeKind::Event => {}
                kind => refusals.push((
                    at,
                    refusal(ProtocolAnchorCause::AwaitedForm {
                        node: name.clone(),
                        event: event.name.name.clone(),
                        form: kind.label(),
                        profile: selections.protocol.map(str::to_owned),
                        span: at,
                    }),
                )),
            }
        }
        let within = match form.within.upper {
            IntervalUpper::Finite(upper) if form.within.lower <= upper => Some(CheckedDeadline {
                lower: form.within.lower,
                upper,
            }),
            IntervalUpper::Finite(upper) => {
                refusals.push(window(
                    name,
                    form.within.lower,
                    Some(upper),
                    form.within.span,
                ));
                None
            }
            IntervalUpper::Open => {
                refusals.push(window(name, form.within.lower, None, form.within.span));
                None
            }
        };
        if let Some(within) = within {
            checked.push(CheckedAwait {
                node: ProtocolNodeId(form.declaration),
                clock: form.clock.text.clone(),
                within,
            });
        }
    }
    if refusals.is_empty() {
        Ok(checked)
    } else {
        Err(refusals)
    }
}

fn window(node: &str, lower: u64, upper: Option<u64>, span: Span) -> (Span, CheckRefusal) {
    (
        span,
        refusal(ProtocolAnchorCause::AwaitWindow {
            node: node.to_owned(),
            lower,
            upper,
            span,
        }),
    )
}
