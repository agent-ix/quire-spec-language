// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrowed canonical event source: no wire row vectors or JSON trees.

use std::cell::RefCell;
use std::fmt::Write as _;

use super::*;
use quire_canonical::{Encode, Error, Reservation, ReservationDenied, Sink, Writer};

struct Admission<'a> {
    used: u64,
    bound: u64,
    cancel: &'a Cancel,
    stopped: Option<ProbabilityDocumentError>,
}

impl Admission<'_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        if let Err(error) = poll(self.cancel) {
            self.stopped = Some(error);
            return Err(ReservationDenied);
        }
        let actual = self.used.saturating_add(bytes as u64);
        if actual > self.bound {
            self.stopped = Some(exceeded(ProbabilityDocumentLimit::StorageBytes, self.bound, actual));
            return Err(ReservationDenied);
        }
        self.used = actual;
        Ok(())
    }
}

struct Reserve<'a, 'c>(&'a RefCell<Admission<'c>>);
impl Reservation for Reserve<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.0.borrow_mut().reserve(bytes)
    }
}

struct Events<'a> {
    document: &'a ProbabilityCertificateDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    stopped: RefCell<Option<ProbabilityDocumentError>>,
    #[cfg(test)]
    texts: std::cell::Cell<usize>,
}

impl Events<'_> {
    fn poll(&self) -> Result<(), Error> {
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
    fn string_bytes<S: Sink + ?Sized>(&self, writer: &Writer<'_, S>, len: usize) -> Result<(), Error> {
        let required = writer.produced().saturating_add(len as u64).saturating_add(2);
        if required > self.limits.bytes {
            return Err(Error::Limit(quire_canonical::LimitExceeded {
                kind: quire_canonical::LimitKind::CanonicalBytes,
                bound: self.limits.bytes,
                required,
            }));
        }
        Ok(())
    }

    fn key<S: Sink + ?Sized>(&self, key: &ProbabilityProductKey, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.poll()?;
        let len = base64::encoded_len(key.as_bytes().len(), false)
            .ok_or(Error::Allocation { requested: usize::MAX })?;
        self.string_bytes(writer, len)?;
        writer.reserve(len)?;
        self.text_reserved();
        let mut text = String::new();
        text.try_reserve_exact(len).map_err(|_| Error::Allocation { requested: len })?;
        URL_SAFE_NO_PAD.encode_string(key.as_bytes(), &mut text);
        writer.string(&text)
    }

    fn integer<S: Sink + ?Sized>(&self, integer: &Integer, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.poll()?;
        let bits = integer.magnitude_bits();
        if bits > self.limits.scalar_bits {
            *self.stopped.borrow_mut() = Some(exceeded(ProbabilityDocumentLimit::RationalBits, self.limits.scalar_bits, bits));
            return Err(Error::ReservationDenied { requested: 0 });
        }
        // A binary magnitude's bit count bounds its decimal digit count. Admit
        // two such buffers before formatting: the formatter's text and our
        // retained transport text. This is an integer bound, not a heap ratio.
        let capacity = usize::try_from(bits.saturating_add(u64::from(integer.is_negative())))
            .map_err(|_| Error::Allocation { requested: usize::MAX })?;
        writer.reserve(capacity.checked_mul(2).ok_or(Error::Allocation { requested: usize::MAX })?)?;
        struct Count(usize);
        impl std::fmt::Write for Count {
            fn write_str(&mut self, text: &str) -> std::fmt::Result {
                self.0 = self.0.checked_add(text.len()).ok_or(std::fmt::Error)?;
                Ok(())
            }
        }
        let mut count = Count(0);
        write!(&mut count, "{integer}").map_err(|_| Error::Allocation { requested: capacity })?;
        self.string_bytes(writer, count.0)?;
        self.text_reserved();
        let mut text = String::new();
        text.try_reserve_exact(count.0).map_err(|_| Error::Allocation { requested: count.0 })?;
        write!(&mut text, "{integer}").map_err(|_| Error::Allocation { requested: count.0 })?;
        writer.string(&text)
    }
}

impl Encode for Events<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.poll()?;
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
        writer.string(PROFILE)?;
        writer.name("targets")?;
        writer.begin_array()?;
        for key in &self.document.targets { self.key(key, writer)?; }
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
            self.poll()?;
            writer.begin_object()?;
            writer.name("state")?;
            self.key(&row.state, writer)?;
            writer.name("value")?;
            writer.begin_object()?;
            writer.name("numerator")?;
            self.integer(row.value.numerator(), writer)?;
            writer.name("denominator")?;
            self.integer(row.value.denominator(), writer)?;
            writer.end_object()?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.name("ranking")?;
        writer.begin_array()?;
        for (key, rank) in &self.document.ranking {
            self.poll()?;
            writer.begin_object()?;
            writer.name("state")?;
            self.key(key, writer)?;
            writer.name("rank")?;
            writer.integer(i128::from(*rank))?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.end_object()
    }
}

fn encode(document: &ProbabilityCertificateDocument, limits: ProbabilityDocumentLimits, cancel: &Cancel) -> Result<Vec<u8>, ProbabilityDocumentError> {
    poll(cancel)?;
    let rows = document.targets.len().max(document.values.len()).max(document.ranking.len());
    if rows > limits.states {
        return Err(exceeded(ProbabilityDocumentLimit::States, limits.states as u64, rows as u64));
    }
    let events = Events { document, limits, cancel, stopped: RefCell::new(None), #[cfg(test)] texts: std::cell::Cell::new(0) };
    encode_events(&events)
}

fn encode_events(events: &Events<'_>) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let limits = events.limits;
    let cancel = events.cancel;
    let admission = RefCell::new(Admission { used: 0, bound: limits.storage_bytes, cancel, stopped: None });
    let mut output = Vec::new();
    // The metered canonical writer retains output internally until finish.
    // Charge its destination separately before the sink reserves that capacity.
    struct Output<'a, 'c> { bytes: &'a mut Vec<u8>, admission: &'a RefCell<Admission<'c>> }
    impl Sink for Output<'_, '_> {
        fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
            self.admission.borrow_mut().reserve(bytes.len()).map_err(|_| Error::ReservationDenied { requested: bytes.len() })?;
            self.bytes.try_reserve_exact(bytes.len()).map_err(|_| Error::Allocation { requested: bytes.len() })?;
            self.bytes.extend_from_slice(bytes);
            Ok(())
        }
    }
    // Writer::finish holds its admitted output while copying to the sink. Its
    // full cumulative reservation already includes that output; the sink's
    // additional bytes are charged against the remaining working budget.
    let mut sink = Output { bytes: &mut output, admission: &admission };
    let result = quire_canonical::encode_with_reservation(&mut sink, events, quire_canonical::Limits::new(limits.bytes), &mut Reserve(&admission));
    if let Some(error) = events.stopped.borrow_mut().take().or(admission.into_inner().stopped) { return Err(error); }
    result.map_err(|error| match error {
        Error::Limit(limit) => exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required),
        _ => ProbabilityDocumentError::Malformed,
    })?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-201-AC-9", "FR-255-AC-1")]
    #[test]
    fn writer_admits_bytes_and_storage_before_transport_text_and_preserves_exact_budget_output() {
        let mut document = super::super::tests::document();
        let key = ProbabilityProductKey::for_model_state(None, &vec![0; 131_072]).unwrap();
        document.targets = vec![key.clone()];
        document.values[0].state = key.clone();
        document.ranking[0].0 = key.clone();
        let cancel = Cancel::new();
        let generous = super::super::tests::limits();
        let encoded_key = base64::encoded_len(key.as_bytes().len(), false).unwrap() as u64;
        for (bytes, storage_bytes, expected) in [
            (0, generous.storage_bytes, ProbabilityDocumentLimit::Bytes),
            (encoded_key - 1, generous.storage_bytes, ProbabilityDocumentLimit::Bytes),
            (generous.bytes, 0, ProbabilityDocumentLimit::StorageBytes),
            (generous.bytes, encoded_key - 1, ProbabilityDocumentLimit::StorageBytes),
        ] {
            let events = Events { document: &document, limits: ProbabilityDocumentLimits { bytes, storage_bytes, ..generous }, cancel: &cancel, stopped: RefCell::new(None), texts: std::cell::Cell::new(0) };
            let error = encode_events(&events).unwrap_err();
            assert!(matches!(error, ProbabilityDocumentError::Limit { limit, bound, actual } if limit == expected && bound == if expected == ProbabilityDocumentLimit::Bytes { bytes } else { storage_bytes } && actual > bound), "{error:?}");
            assert_eq!(events.texts.get(), 0, "no base64 or decimal text was materialized before admission: {error:?}");
        }
        let expected = write(&document, generous, &cancel).unwrap();
        let mut exact = ProbabilityDocumentLimits { bytes: expected.len() as u64, storage_bytes: 0, ..generous };
        loop {
            match write(&document, exact, &cancel) {
                Err(ProbabilityDocumentError::Limit { limit: ProbabilityDocumentLimit::StorageBytes, actual, .. }) => exact.storage_bytes = actual,
                Ok(actual) => { assert_eq!(actual, expected); break; }
                other => panic!("exact byte budget changed output or exhausted another setting: {other:?}"),
            }
        }
        assert!(matches!(write(&document, ProbabilityDocumentLimits { bytes: exact.bytes - 1, ..exact }, &cancel), Err(ProbabilityDocumentError::Limit { limit: ProbabilityDocumentLimit::Bytes, .. })));
        assert!(matches!(write(&document, ProbabilityDocumentLimits { storage_bytes: exact.storage_bytes - 1, ..exact }, &cancel), Err(ProbabilityDocumentError::Limit { limit: ProbabilityDocumentLimit::StorageBytes, .. })));
    }
}

pub(super) fn write(document: &ProbabilityCertificateDocument, limits: ProbabilityDocumentLimits, cancel: &Cancel) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let bytes = encode(document, limits, cancel)?;
    read_probability_certificate(&bytes, limits, cancel)?;
    Ok(bytes)
}
