// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrowed canonical event source: no wire row vectors or JSON trees.

use std::cell::RefCell;

use super::*;
use quire_canonical::{Encode, Error, Reservation, ReservationDenied, Sink, Writer};

pub(super) struct Admission<'a> {
    used: u64,
    bound: u64,
    cancel: &'a Cancel,
    stopped: Option<ProbabilityDocumentError>,
}

impl<'a> Admission<'a> {
    pub(super) fn new(bound: u64, cancel: &'a Cancel) -> Self {
        Self {
            used: 0,
            bound,
            cancel,
            stopped: None,
        }
    }
    pub(super) fn take_stop(&mut self) -> Option<ProbabilityDocumentError> {
        self.stopped.take()
    }
    pub(super) fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        if let Err(error) = poll(self.cancel) {
            self.stopped = Some(error);
            return Err(ReservationDenied);
        }
        let actual = self.used.saturating_add(bytes as u64);
        if actual > self.bound {
            self.stopped = Some(exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                self.bound,
                actual,
            ));
            return Err(ReservationDenied);
        }
        self.used = actual;
        Ok(())
    }
}

pub(super) struct Reserve<'a, 'c>(pub(super) &'a RefCell<Admission<'c>>);
impl Reservation for Reserve<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.0.borrow_mut().reserve(bytes)
    }
}

struct Events<'a> {
    document: &'a ProbabilityCertificateDocument,
    text: Text<'a>,
}

pub(super) struct Text<'a> {
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    stopped: RefCell<Option<ProbabilityDocumentError>>,
    #[cfg(test)]
    texts: std::cell::Cell<usize>,
}

impl<'a> Text<'a> {
    pub(super) fn new(limits: ProbabilityDocumentLimits, cancel: &'a Cancel) -> Self {
        Self {
            limits,
            cancel,
            stopped: RefCell::new(None),
            #[cfg(test)]
            texts: std::cell::Cell::new(0),
        }
    }
    pub(super) fn take_stop(&self) -> Option<ProbabilityDocumentError> {
        self.stopped.borrow_mut().take()
    }

    pub(super) fn poll(&self) -> Result<(), Error> {
        poll(self.cancel).map_err(|error| {
            *self.stopped.borrow_mut() = Some(error);
            Error::ReservationDenied { requested: 0 }
        })
    }

    fn text_reserved(&self) {
        #[cfg(test)]
        self.texts.set(self.texts.get() + 1);
    }

    /// Account the exact emitted string length before its transport allocation.
    fn string_bytes<S: Sink + ?Sized>(
        &self,
        writer: &Writer<'_, S>,
        len: usize,
        separator: bool,
    ) -> Result<(), Error> {
        let required = writer
            .produced()
            .saturating_add(len as u64)
            .saturating_add(2 + u64::from(separator));
        if required > self.limits.bytes {
            return Err(Error::Limit(quire_canonical::LimitExceeded {
                kind: quire_canonical::LimitKind::CanonicalBytes,
                bound: self.limits.bytes,
                required,
            }));
        }
        Ok(())
    }

    pub(super) fn key<S: Sink + ?Sized>(
        &self,
        key: &ProbabilityProductKey,
        writer: &mut Writer<'_, S>,
        separator: bool,
    ) -> Result<(), Error> {
        self.poll()?;
        let len = base64::encoded_len(key.as_bytes().len(), false).ok_or(Error::Allocation {
            requested: usize::MAX,
        })?;
        self.string_bytes(writer, len, separator)?;
        writer.reserve(len)?;
        self.text_reserved();
        let mut text = String::new();
        text.try_reserve_exact(len)
            .map_err(|_| Error::Allocation { requested: len })?;
        URL_SAFE_NO_PAD.encode_string(key.as_bytes(), &mut text);
        writer.string(&text)
    }

    pub(super) fn integer<S: Sink + ?Sized>(
        &self,
        integer: &Integer,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        self.poll()?;
        let bits = integer.magnitude_bits();
        if bits > self.limits.scalar_bits {
            *self.stopped.borrow_mut() = Some(exceeded(
                ProbabilityDocumentLimit::RationalBits,
                self.limits.scalar_bits,
                bits,
            ));
            return Err(Error::ReservationDenied { requested: 0 });
        }
        // The bit count bounds the decimal digit count. Admit that transport
        // capacity before invoking the kernel's bounded decimal conversion;
        // it refuses oversized output before allocating its output string and
        // never populates the integer's lazy BigInt cache.
        let capacity = usize::try_from(bits.saturating_add(u64::from(integer.is_negative())))
            .map_err(|_| Error::Allocation {
                requested: usize::MAX,
            })?;
        // Even zero has one digit, plus JSON quotes.
        self.string_bytes(writer, 1 + usize::from(integer.is_negative()), false)?;
        writer.reserve(capacity)?;
        let remaining = self
            .limits
            .bytes
            .saturating_sub(writer.produced())
            .saturating_sub(2);
        let text = integer
            .try_to_decimal_string(usize::try_from(remaining).unwrap_or(usize::MAX))
            .map_err(|error| match error {
                quire_exact::IntegerDecimalError::ByteLimit { required, .. } => {
                    Error::Limit(quire_canonical::LimitExceeded {
                        kind: quire_canonical::LimitKind::CanonicalBytes,
                        bound: self.limits.bytes,
                        required: writer
                            .produced()
                            .saturating_add(required as u64)
                            .saturating_add(2),
                    })
                }
                quire_exact::IntegerDecimalError::Allocation { requested } => {
                    Error::Allocation { requested }
                }
                quire_exact::IntegerDecimalError::OutputSizeOverflow => Error::Allocation {
                    requested: usize::MAX,
                },
            })?;
        self.text_reserved();
        writer.string(&text)
    }
}

impl Encode for Events<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.text.poll()?;
        writer.begin_object()?;
        writer.name("identity")?;
        writer.begin_object()?;
        writer.name("node_id")?;
        qsl_foundation::node_wire::encode_node_key(self.document.identity.node(), writer)?;
        writer.name("model")?;
        match self.document.identity.model() {
            Some(model) => writer.string(model.as_str())?,
            None => writer.null()?,
        }
        writer.name("ordinal")?;
        writer.integer(i128::from(self.document.identity.ordinal()))?;
        writer.end_object()?;
        writer.name("objective")?;
        writer.begin_object()?;
        writer.name("profile")?;
        writer.string(if self.document.reward.is_some() {
            REWARD_PROFILE
        } else if self.document.event.is_some() {
            STATE_PROFILE
        } else {
            PROFILE
        })?;
        if let Some(reward) = &self.document.reward {
            writer.name("reward")?;
            writer.begin_array()?;
            qsl_foundation::node_wire::encode_node_key(&reward.model, writer)?;
            writer.string(reward.name.as_str())?;
            writer.end_array()?;
            writer.name("unit")?;
            match &reward.unit {
                None => writer.null()?,
                Some(unit) => {
                    writer.begin_array()?;
                    if let Some(node) = unit.declared_key() {
                        writer.string("declared")?;
                        qsl_foundation::node_wire::encode_node_key(node, writer)?;
                    } else {
                        writer.string("compound")?;
                        writer.begin_array()?;
                        for (node, exponent) in unit.compound_terms().unwrap_or_default() {
                            self.poll()?;
                            writer.begin_object()?;
                            writer.name("unit")?;
                            qsl_foundation::node_wire::encode_node_key(node, writer)?;
                            writer.name("exponent")?;
                            self.integer(exponent, writer)?;
                            writer.end_object()?;
                        }
                        writer.end_array()?;
                    }
                    writer.end_array()?;
                }
            }
        }
        if let Some(event) = self.document.event {
            writer.name("event")?;
            writer.string(match event {
                ProbabilityStateEvent::Until => "until",
                ProbabilityStateEvent::Always => "always",
            })?;
            writer.name("failures")?;
            writer.begin_array()?;
            for (index, key) in self.document.failures.iter().enumerate() {
                self.text.key(key, writer, index != 0)?;
            }
            writer.end_array()?;
        }
        writer.name("targets")?;
        writer.begin_array()?;
        for (index, key) in self.document.targets.iter().enumerate() {
            self.text.key(key, writer, index != 0)?;
        }
        writer.end_array()?;
        writer.name("optimization")?;
        writer.string(match self.document.optimization {
            ProbabilityObjective::Minimum => "minimum",
            ProbabilityObjective::Maximum => "maximum",
            ProbabilityObjective::Workload => "workload",
        })?;
        writer.name("fairness")?;
        writer.begin_array()?;
        writer.end_array()?;
        writer.end_object()?;
        writer.name("kind")?;
        writer.string("exact")?;
        writer.name("values")?;
        writer.begin_array()?;
        for row in &self.document.values {
            self.text.poll()?;
            writer.begin_object()?;
            writer.name("state")?;
            self.text.key(&row.state, writer, false)?;
            writer.name("value")?;
            writer.begin_object()?;
            writer.name("numerator")?;
            self.text.integer(row.value.numerator(), writer)?;
            writer.name("denominator")?;
            self.text.integer(row.value.denominator(), writer)?;
            writer.end_object()?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.name("ranking")?;
        writer.begin_array()?;
        for (key, rank) in &self.document.ranking {
            self.text.poll()?;
            writer.begin_object()?;
            writer.name("state")?;
            self.text.key(key, writer, false)?;
            writer.name("rank")?;
            writer.integer(i128::from(*rank))?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.end_object()
    }
}

fn encode(
    document: &ProbabilityCertificateDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    poll(cancel)?;
    if document.event.is_none() && !document.failures.is_empty() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    if document.reward.is_some() && document.event.is_some() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let rows = document
        .targets
        .len()
        .max(document.failures.len())
        .max(document.values.len())
        .max(document.ranking.len());
    if rows > limits.states {
        return Err(exceeded(
            ProbabilityDocumentLimit::States,
            limits.states as u64,
            rows as u64,
        ));
    }
    let events = Events {
        document,
        text: Text {
            limits,
            cancel,
            stopped: RefCell::new(None),
            #[cfg(test)]
            texts: std::cell::Cell::new(0),
        },
    };
    encode_events(&events)
}

fn encode_events(events: &Events<'_>) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let limits = events.text.limits;
    let cancel = events.text.cancel;
    let admission = RefCell::new(Admission {
        used: 0,
        bound: limits.storage_bytes,
        cancel,
        stopped: None,
    });
    let result = encode_reserved(events, limits.bytes, &admission);
    if let Some(error) = events.text.stopped.borrow_mut().take() {
        return Err(error);
    }
    result
}

pub(super) fn encode_reserved(
    value: &impl Encode,
    bytes: u64,
    admission: &RefCell<Admission<'_>>,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let result = encode_admitted(value, bytes, &mut Reserve(admission));
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    result.map_err(|error| match error {
        Error::Limit(limit) => {
            exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required)
        }
        _ => ProbabilityDocumentError::Malformed,
    })
}

/// Reserve canonical staging and destination capacity against the same caller.
pub(super) fn encode_admitted(
    value: &impl Encode,
    bytes: u64,
    reservation: &mut dyn Reservation,
) -> Result<Vec<u8>, Error> {
    let shared = RefCell::new(reservation);
    struct Shared<'a, 'b>(&'a RefCell<&'b mut dyn Reservation>);
    impl Reservation for Shared<'_, '_> {
        fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
            self.0.borrow_mut().reserve(bytes)
        }
    }
    struct Output<'a, 'b, 'c> {
        bytes: &'a mut Vec<u8>,
        reservation: &'b RefCell<&'c mut dyn Reservation>,
    }
    impl Sink for Output<'_, '_, '_> {
        fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
            self.reservation
                .borrow_mut()
                .reserve(bytes.len())
                .map_err(|_| Error::ReservationDenied {
                    requested: bytes.len(),
                })?;
            self.bytes
                .try_reserve_exact(bytes.len())
                .map_err(|_| Error::Allocation {
                    requested: bytes.len(),
                })?;
            self.bytes.extend_from_slice(bytes);
            Ok(())
        }
    }
    let mut output = Vec::new();
    let mut sink = Output {
        bytes: &mut output,
        reservation: &shared,
    };
    quire_canonical::encode_with_reservation(
        &mut sink,
        value,
        quire_canonical::Limits::new(bytes),
        &mut Shared(&shared),
    )?;
    Ok(output)
}

pub(super) fn write(
    document: &ProbabilityCertificateDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let bytes = encode(document, limits, cancel)?;
    read_probability_certificate(&bytes, limits, cancel)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-201-AC-9", "FR-255-AC-1")]
    #[test]
    fn bounded_kernel_decimal_output_is_admitted_before_retaining_transport_text() {
        let document = super::super::tests::document();
        let integer = Integer::power_of_ten(10_000);
        let cancel = Cancel::new();
        for (bytes, storage_bytes) in [(3, 1_000_000), (1_000_000, 0)] {
            let events = Events {
                document: &document,
                text: Text {
                    limits: ProbabilityDocumentLimits {
                        bytes,
                        storage_bytes,
                        scalar_bits: 100_000,
                        ..super::super::tests::limits()
                    },
                    cancel: &cancel,
                    stopped: RefCell::new(None),
                    texts: std::cell::Cell::new(0),
                },
            };
            let admission = RefCell::new(Admission {
                used: 0,
                bound: storage_bytes,
                cancel: &cancel,
                stopped: None,
            });
            let mut output = Vec::new();
            let mut reservation = Reserve(&admission);
            let mut writer = Writer::with_reservation(
                &mut output,
                quire_canonical::Limits::new(bytes),
                &mut reservation,
            );
            let error = events.text.integer(&integer, &mut writer).unwrap_err();
            assert_eq!(
                events.text.texts.get(),
                0,
                "oversized decimal transport was not retained"
            );
            if storage_bytes == 0 {
                assert!(
                    matches!(admission.borrow().stopped, Some(ProbabilityDocumentError::Limit { limit: ProbabilityDocumentLimit::StorageBytes, bound: 0, actual }) if actual > 0)
                );
            } else {
                assert!(matches!(
                    error,
                    Error::Limit(quire_canonical::LimitExceeded {
                        bound: 3,
                        required: 10_003,
                        ..
                    })
                ));
            }
        }
    }

    #[trace("FR-201-AC-9", "FR-255-AC-1")]
    #[test]
    fn writer_admits_bytes_and_storage_before_transport_text_and_preserves_exact_budget_output() {
        let mut document = super::super::tests::document();
        let key = super::super::tests::large_key(131_072);
        document.targets = vec![key.clone()];
        document.values[0].state = key.clone();
        document.ranking[0].0 = key.clone();
        let cancel = Cancel::new();
        let generous = super::super::tests::limits();
        let encoded_key = base64::encoded_len(key.as_bytes().len(), false).unwrap() as u64;
        // Refuse an array's second key before encoding it even when only its
        // separator, rather than a character of its payload, exceeds the cap.
        let cap = 1 + 2 * (encoded_key + 2);
        let events = Events {
            document: &document,
            text: Text {
                limits: ProbabilityDocumentLimits {
                    bytes: cap,
                    ..generous
                },
                cancel: &cancel,
                stopped: RefCell::new(None),
                texts: std::cell::Cell::new(0),
            },
        };
        let mut output = Vec::new();
        let mut writer = Writer::new(&mut output, quire_canonical::Limits::new(cap));
        writer.begin_array().unwrap();
        events.text.key(&key, &mut writer, false).unwrap();
        assert!(
            matches!(events.text.key(&key, &mut writer, true), Err(Error::Limit(quire_canonical::LimitExceeded { required, .. })) if required == cap + 1)
        );
        assert_eq!(
            events.text.texts.get(),
            1,
            "the rejected second transport string was never encoded"
        );
        for (bytes, storage_bytes, expected) in [
            (0, generous.storage_bytes, ProbabilityDocumentLimit::Bytes),
            (
                encoded_key - 1,
                generous.storage_bytes,
                ProbabilityDocumentLimit::Bytes,
            ),
            (generous.bytes, 0, ProbabilityDocumentLimit::StorageBytes),
            (
                generous.bytes,
                encoded_key - 1,
                ProbabilityDocumentLimit::StorageBytes,
            ),
        ] {
            let events = Events {
                document: &document,
                text: Text {
                    limits: ProbabilityDocumentLimits {
                        bytes,
                        storage_bytes,
                        ..generous
                    },
                    cancel: &cancel,
                    stopped: RefCell::new(None),
                    texts: std::cell::Cell::new(0),
                },
            };
            let error = encode_events(&events).unwrap_err();
            assert!(
                matches!(error, ProbabilityDocumentError::Limit { limit, bound, actual } if limit == expected && bound == if expected == ProbabilityDocumentLimit::Bytes { bytes } else { storage_bytes } && actual > bound),
                "{error:?}"
            );
            assert_eq!(
                events.text.texts.get(),
                0,
                "no base64 or decimal text was materialized before admission: {error:?}"
            );
        }
        let expected = write(&document, generous, &cancel).unwrap();
        let mut exact = ProbabilityDocumentLimits {
            bytes: expected.len() as u64,
            storage_bytes: 0,
            ..generous
        };
        loop {
            match write(&document, exact, &cancel) {
                Err(ProbabilityDocumentError::Limit {
                    limit: ProbabilityDocumentLimit::StorageBytes,
                    actual,
                    ..
                }) => exact.storage_bytes = actual,
                Ok(actual) => {
                    assert_eq!(actual, expected);
                    break;
                }
                other => panic!(
                    "exact byte budget changed output or exhausted another setting: {other:?}"
                ),
            }
        }
        assert!(matches!(
            write(
                &document,
                ProbabilityDocumentLimits {
                    bytes: exact.bytes - 1,
                    ..exact
                },
                &cancel
            ),
            Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Bytes,
                ..
            })
        ));
        assert!(matches!(
            write(
                &document,
                ProbabilityDocumentLimits {
                    storage_bytes: exact.storage_bytes - 1,
                    ..exact
                },
                &cancel
            ),
            Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::StorageBytes,
                ..
            })
        ));
    }
}
