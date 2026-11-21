// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-196 embeds the parsed model and monitor keys, not their transports.

use super::*;
use quire_canonical::{Encode, Node, NodeRef, Sink, Writer};
use std::cell::RefCell;

pub(crate) fn model_product_key(
    binding: Option<&ObjectReference>,
    model_bytes: &[u8],
    monitor: Option<&fsl_eval::automaton::AutomatonStateKey>,
    limits: ProbabilityDocumentIoLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    model_product_key_with_storage(binding, model_bytes, monitor, limits, cancel, &mut 0)
}

pub(crate) fn model_product_key_with_storage(
    binding: Option<&ObjectReference>,
    model_bytes: &[u8],
    monitor: Option<&fsl_eval::automaton::AutomatonStateKey>,
    limits: ProbabilityDocumentIoLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    poll(cancel)?;
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        *storage,
    ));
    let result = encode_product(binding, model_bytes, monitor, limits, cancel, &admission);
    *storage = admission.borrow().used();
    result
}

fn encode_product(
    binding: Option<&ObjectReference>,
    model_bytes: &[u8],
    monitor: Option<&fsl_eval::automaton::AutomatonStateKey>,
    limits: ProbabilityDocumentIoLimits,
    cancel: &Cancel,
    admission: &RefCell<writer::Admission<'_>>,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let model = read(model_bytes, limits.bytes, admission)?;
    if !model_form(model.root()) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let monitor = monitor
        .map(|key| read(key.as_bytes(), limits.bytes, admission))
        .transpose()?;
    if monitor
        .as_ref()
        .is_some_and(|key| !automaton_key(key.root()))
    {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let product = Product {
        binding,
        model: model.root(),
        monitor: monitor.as_ref().map(|key| key.root()),
    };
    let bytes = writer::encode_reserved(&product, limits.bytes, admission)?;
    poll(cancel)?;
    Ok(ProbabilityProductKey::from_canonical_bytes(bytes))
}

struct Product<'a> {
    binding: Option<&'a ObjectReference>,
    model: NodeRef<'a>,
    monitor: Option<NodeRef<'a>>,
}
impl Encode for Product<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        writer.string("filament.probability-product-key/v1")?;
        match self.binding {
            Some(binding) => fsl_eval::simulation::model::encode_reference(binding, writer)?,
            None => writer.null()?,
        }
        // The `on each` instance and its activation phase: null for the
        // `on origin` claims these profiles admit.
        writer.null()?;
        writer.null()?;
        writer.begin_array()?;
        writer.string("filament.simulation.state-key/v1")?;
        self.model.encode_into(writer)?;
        writer.end_array()?;
        match self.monitor {
            Some(monitor) => {
                writer.begin_array()?;
                writer.string("formula")?;
                monitor.encode_into(writer)?;
                writer.end_array()?;
            }
            None => writer.null()?,
        }
        writer.null()?;
        writer.null()?;
        writer.end_array()
    }
}

/// Full FR-196 key construction under the raw identity policy and actual caller.
pub(crate) fn model_product_key_with_budget(
    binding: Option<&ObjectReference>,
    model_bytes: &[u8],
    identity: fsl_foundation::IdentityLimits,
    budget: &mut dyn fsl_eval::simulation::model::ModelKeyBudget,
    cancel: &Cancel,
) -> Result<ProbabilityProductKey, fsl_eval::simulation::model::ModelKeyError> {
    formula_product_key_with_budget(binding, model_bytes, None, identity, budget, cancel)
}

pub(crate) fn formula_product_key_with_budget(
    binding: Option<&ObjectReference>,
    model_bytes: &[u8],
    monitor: Option<&fsl_eval::automaton::AutomatonStateKey>,
    identity: fsl_foundation::IdentityLimits,
    budget: &mut dyn fsl_eval::simulation::model::ModelKeyBudget,
    cancel: &Cancel,
) -> Result<ProbabilityProductKey, fsl_eval::simulation::model::ModelKeyError> {
    use fsl_eval::simulation::model::ModelKeyError;
    use fsl_eval::simulation::EncodingRefusal;
    use quire_canonical::{Reservation, ReservationDenied};
    struct Reserve<'a>(&'a mut dyn fsl_eval::simulation::model::ModelKeyBudget);
    impl Reservation for Reserve<'_> {
        fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
            self.0.reserve(bytes).map_err(|_| ReservationDenied)
        }
    }
    let poll = || {
        cancel.poll();
        match cancel.tripped() {
            Some(cause) => Err(ModelKeyError::Cancelled(cause)),
            None => Ok(()),
        }
    };
    poll()?;
    budget
        .work(
            u64::try_from(model_bytes.len())
                .map_err(|_| ModelKeyError::SizeOverflow)?
                .saturating_add(1),
        )
        .map_err(|_| ModelKeyError::Denied)?;
    let model = quire_canonical::read_with_reservation(
        model_bytes,
        identity.input_bytes,
        &mut Reserve(budget),
    )
    .map_err(|error| match error {
        quire_canonical::ReadError::ReservationDenied { .. } => ModelKeyError::Denied,
        quire_canonical::ReadError::Allocation { requested } => {
            ModelKeyError::Encoding(EncodingRefusal::Allocation { requested })
        }
        quire_canonical::ReadError::Limit(limit) => ModelKeyError::Encoding(
            EncodingRefusal::Limit(fsl_foundation::diagnostic::LimitExceeded::new(
                fsl_foundation::Setting::IdentityInputBytes,
                identity.input_bytes,
                u128::from(limit.required),
            )),
        ),
        other => ModelKeyError::Encoding(EncodingRefusal::NoEncoding(other.to_string())),
    })?;
    let encoded = writer::encode_admitted(&model, identity.input_bytes, &mut Reserve(budget))
        .map_err(|error| match error {
            quire_canonical::Error::ReservationDenied { .. } => ModelKeyError::Denied,
            other => ModelKeyError::Encoding(EncodingRefusal::of(&other, identity)),
        })?;
    if encoded != model_bytes || !model_form(model.root()) {
        return Err(ModelKeyError::Unsupported("canonical raw model identity"));
    }
    let monitor = monitor
        .map(|monitor| {
            budget
                .work(u64::try_from(monitor.as_bytes().len()).unwrap_or(u64::MAX))
                .map_err(|_| ModelKeyError::Denied)?;
            let parsed = quire_canonical::read_with_reservation(
                monitor.as_bytes(),
                identity.input_bytes,
                &mut Reserve(budget),
            )
            .map_err(|error| match error {
                quire_canonical::ReadError::ReservationDenied { .. } => ModelKeyError::Denied,
                quire_canonical::ReadError::Allocation { requested } => {
                    ModelKeyError::Encoding(EncodingRefusal::Allocation { requested })
                }
                quire_canonical::ReadError::Limit(limit) => ModelKeyError::Encoding(
                    EncodingRefusal::Limit(fsl_foundation::diagnostic::LimitExceeded::new(
                        fsl_foundation::Setting::IdentityInputBytes,
                        identity.input_bytes,
                        u128::from(limit.required),
                    )),
                ),
                other => ModelKeyError::Encoding(EncodingRefusal::NoEncoding(other.to_string())),
            })?;
            let canonical =
                writer::encode_admitted(&parsed, identity.input_bytes, &mut Reserve(budget))
                    .map_err(|error| match error {
                        quire_canonical::Error::ReservationDenied { .. } => ModelKeyError::Denied,
                        other => ModelKeyError::Encoding(EncodingRefusal::of(&other, identity)),
                    })?;
            if canonical != monitor.as_bytes() || !automaton_key(parsed.root()) {
                return Err(ModelKeyError::Unsupported(
                    "canonical formula monitor identity",
                ));
            }
            Ok(parsed)
        })
        .transpose()?;
    let product = Product {
        binding,
        model: model.root(),
        monitor: monitor.as_ref().map(|monitor| monitor.root()),
    };
    let bytes = writer::encode_admitted(&product, identity.input_bytes, &mut Reserve(budget))
        .map_err(|error| match error {
            quire_canonical::Error::ReservationDenied { .. } => ModelKeyError::Denied,
            other => ModelKeyError::Encoding(EncodingRefusal::of(&other, identity)),
        })?;
    poll()?;
    Ok(ProbabilityProductKey::from_canonical_bytes(bytes))
}

fn read<'a>(
    bytes: &[u8],
    max_bytes: u64,
    admission: &RefCell<writer::Admission<'a>>,
) -> Result<quire_canonical::Document, ProbabilityDocumentError> {
    let result =
        quire_canonical::read_with_reservation(bytes, max_bytes, &mut writer::Reserve(admission));
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    let document = result.map_err(|error| match error {
        quire_canonical::ReadError::Limit(limit) => {
            exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required)
        }
        _ => ProbabilityDocumentError::Malformed,
    })?;
    if writer::encode_reserved(&document, max_bytes, admission)? != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    Ok(document)
}

// This is only the owner-defined outer model envelope. Typed state content and
// actual state membership are independently rederived from ModelSystem.
pub(super) fn model_form(node: NodeRef<'_>) -> bool {
    match node.node() {
        Node::Object(members) => {
            let fields = [
                "semantic",
                "control",
                "queues",
                "roles",
                "observations",
                "bounds",
                "memory",
            ];
            members.len() == fields.len() + 1
                && matches!(
                    node.get("type").map(|kind| kind.node()),
                    Some(Node::String("simulation-state"))
                )
                && fields.into_iter().all(|field| node.get(field).is_some())
        }
        Node::Array(parts) => timed_model_form(parts.collect()),
        _ => false,
    }
}

fn timed_model_form(parts: Vec<NodeRef<'_>>) -> bool {
    let boundary = matches!(
        parts.first().map(|part| part.node()),
        Some(Node::String("boundary-region"))
    );
    if parts.len() != if boundary { 8 } else { 7 }
        || !matches!(
            parts[0].node(),
            Node::String("digital" | "region" | "boundary-region")
        )
        || !matches!(parts[1].node(), Node::Object(_))
        || !model_form(parts[1])
    {
        return false;
    }
    let region = !matches!(parts[0].node(), Node::String("digital"));
    let Node::Array(unit) = parts[2].node() else {
        return false;
    };
    let unit = unit.collect::<Vec<_>>();
    if unit.len() != 2
        || !matches!(unit[0].node(), Node::String("declared"))
        || canonical_count(parts[3]).is_none_or(|scale| scale == 0)
    {
        return false;
    }
    let Node::Array(clocks) = parts[4].node() else {
        return false;
    };
    let clocks = clocks.collect::<Vec<_>>();
    for clock in &clocks {
        let Node::Array(clock) = clock.node() else {
            return false;
        };
        let clock = clock.collect::<Vec<_>>();
        if clock.len() != if region { 4 } else { 3 }
            || !matches!(clock[0].node(), Node::Array(identity) if identity.len() == 3)
        {
            return false;
        }
        let Some(value) = canonical_count(clock[1]) else {
            return false;
        };
        let Some(cap) = canonical_count(clock[2]) else {
            return false;
        };
        if value > cap || region && canonical_count(clock[3]).is_none() {
            return false;
        }
    }
    let has_deadline = match parts[5].node() {
        Node::Null => false,
        Node::Array(deadline) => {
            let deadline = deadline.collect::<Vec<_>>();
            if deadline.len() != if region { 4 } else { 3 }
                || !matches!(deadline[0].node(), Node::String("deadline"))
                || canonical_count(deadline[1]).is_none()
                || canonical_count(deadline[2]).is_none()
                || region && canonical_count(deadline[3]).is_none()
            {
                return false;
            }
            true
        }
        _ => return false,
    };
    if boundary {
        let Node::Array(corner) = parts[7].node() else {
            return false;
        };
        let corner = corner.collect::<Vec<_>>();
        if corner.len() != clocks.len() + usize::from(has_deadline)
            || corner
                .iter()
                .any(|coordinate| canonical_count(*coordinate).is_none())
        {
            return false;
        }
    }
    matches!(
        parts[6].node(),
        Node::String("running" | "idle-ready" | "idle-delayed")
    )
}

fn canonical_count(node: NodeRef<'_>) -> Option<u64> {
    let Node::String(text) = node.node() else {
        return None;
    };
    let value = text.parse::<u64>().ok()?;
    (value.to_string() == text).then_some(value)
}

fn automaton_key(node: NodeRef<'_>) -> bool {
    let Node::Array(mut parts) = node.node() else {
        return false;
    };
    if parts.len() != 3
        || !matches!(
            parts.next().map(|part| part.node()),
            Some(Node::String("filament.automaton-state-key/v1"))
        )
    {
        return false;
    }
    matches!(parts.next().map(|part| part.node()), Some(Node::Array(context)) if context.len() == 4)
        && matches!(parts.next().map(|part| part.node()), Some(Node::Array(state)) if state.len() > 0)
}

pub(super) fn admit_parts(parts: &[NodeRef<'_>]) -> Result<(), ProbabilityDocumentError> {
    if !matches!(parts[2].node(), Node::Null) || !matches!(parts[3].node(), Node::Null) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let Node::Array(mut model) = parts[4].node() else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    if model.len() != 2
        || !matches!(
            model.next().map(|part| part.node()),
            Some(Node::String("filament.simulation.state-key/v1"))
        )
        || !model.next().is_some_and(model_form)
    {
        return Err(ProbabilityDocumentError::Malformed);
    }
    match parts[5].node() {
        Node::Null => {}
        Node::Array(mut monitor) if monitor.len() == 2 => {
            if !matches!(
                monitor.next().map(|part| part.node()),
                Some(Node::String("formula"))
            ) || !monitor.next().is_some_and(automaton_key)
            {
                return Err(ProbabilityDocumentError::Malformed);
            }
        }
        Node::Array(mut monitor) if monitor.len() == 3 => {
            if !matches!(
                monitor.next().map(|part| part.node()),
                Some(Node::String("deadline"))
            ) || !matches!(
                monitor.next().map(|part| part.node()),
                Some(Node::String("eventually" | "always" | "until"))
            ) || !matches!(
                monitor.next().map(|part| part.node()),
                Some(Node::String("pending" | "accept" | "reject"))
            ) {
                return Err(ProbabilityDocumentError::Malformed);
            }
        }
        _ => return Err(ProbabilityDocumentError::Unsupported),
    }
    if parts[6..]
        .iter()
        .any(|part| !matches!(part.node(), Node::Null))
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("TC-636", "FR-201-AC-22", "FR-282-AC-17")]
    #[test]
    fn model_keys_share_the_exact_cumulative_graph_storage_ceiling() {
        let models = [
            super::super::tests::model_bytes(0),
            super::super::tests::model_bytes(1),
        ];
        let cancel = Cancel::new();
        let defaults = ProbabilityDocumentIoLimits::default();
        let mut used = 0;
        let mut expected = Vec::new();
        let mut individual = 0;
        for model in models {
            let mut own = 0;
            let key =
                model_product_key_with_storage(None, model, None, defaults, &cancel, &mut own)
                    .unwrap();
            individual = individual.max(own);
            expected.push(key);
            model_product_key_with_storage(None, model, None, defaults, &cancel, &mut used)
                .unwrap();
        }
        let required = used;
        assert!(
            required > individual,
            "shared graph storage must accumulate across distinct model keys"
        );
        for bound in [required, required - 1, individual] {
            let limits = ProbabilityDocumentIoLimits {
                storage_bytes: bound,
                ..defaults
            };
            let mut used = 0;
            for (index, model) in models.into_iter().enumerate() {
                let result =
                    model_product_key_with_storage(None, model, None, limits, &cancel, &mut used);
                if index == 0 || bound == required {
                    assert_eq!(result.unwrap(), expected[index]);
                } else {
                    assert!(matches!(
                        result,
                        Err(ProbabilityDocumentError::Limit {
                            limit: ProbabilityDocumentLimit::StorageBytes,
                            bound: actual_bound,
                            actual,
                        }) if actual_bound == bound && actual > bound
                    ));
                }
            }
            if bound == required {
                assert_eq!(used, required);
            }
        }
        for model in models {
            assert!(model_product_key(
                None,
                model,
                None,
                ProbabilityDocumentIoLimits {
                    storage_bytes: individual,
                    ..defaults
                },
                &cancel,
            )
            .is_ok());
        }
    }

    #[trace("FR-196-AC-6", "FR-201-AC-9", "FR-255-AC-1")]
    #[test]
    fn real_model_form_is_embedded_once_under_exact_constructor_budgets() {
        let model = super::super::tests::model_bytes(0);
        let cancel = Cancel::new();
        let make = |limits| {
            ProbabilityProductKey::for_model_state_with_limits(None, model, limits, &cancel)
        };
        let expected = make(ProbabilityDocumentIoLimits::default()).unwrap();
        let parsed: Value = serde_json::from_slice(expected.as_bytes()).unwrap();
        assert_eq!(
            parsed[4],
            json!([
                "filament.simulation.state-key/v1",
                serde_json::from_slice::<Value>(model).unwrap()
            ])
        );
        for offered in [
            b"arbitrary bytes".as_slice(),
            br#""model""#,
            expected.as_bytes(),
        ] {
            assert!(
                ProbabilityProductKey::for_model_state(None, offered).is_err(),
                "only the actual bare owner form is accepted"
            );
        }
        for limits in [
            ProbabilityDocumentIoLimits {
                bytes: model.len() as u64 - 1,
                storage_bytes: 1_000_000,
            },
            ProbabilityDocumentIoLimits {
                bytes: 1_000_000,
                storage_bytes: 0,
            },
        ] {
            assert!(
                matches!(make(limits), Err(ProbabilityDocumentError::Limit { bound, actual, .. }) if actual > bound)
            );
        }
        let mut exact = ProbabilityDocumentIoLimits {
            bytes: expected.as_bytes().len() as u64,
            storage_bytes: 0,
        };
        loop {
            match make(exact) {
                Err(ProbabilityDocumentError::Limit {
                    limit: ProbabilityDocumentLimit::StorageBytes,
                    actual,
                    ..
                }) => exact.storage_bytes = actual,
                Ok(key) => {
                    assert_eq!(key, expected);
                    break;
                }
                other => panic!("constructor exact-budget control: {other:?}"),
            }
        }
        assert!(matches!(
            make(ProbabilityDocumentIoLimits {
                bytes: exact.bytes - 1,
                ..exact
            }),
            Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Bytes,
                ..
            })
        ));
        assert!(matches!(
            make(ProbabilityDocumentIoLimits {
                storage_bytes: exact.storage_bytes - 1,
                ..exact
            }),
            Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::StorageBytes,
                ..
            })
        ));
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Deadline);
        assert_eq!(
            ProbabilityProductKey::for_model_state_with_limits(None, model, exact, &cancel),
            Err(ProbabilityDocumentError::Cancelled(CancelCause::Deadline))
        );
    }
}
