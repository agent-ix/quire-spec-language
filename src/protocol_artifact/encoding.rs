// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: bounded CompactFormatter output, without an emission authority.

mod census;

use std::{
    cell::RefCell,
    io::{self, Write},
};

use serde::Serialize;
use serde_json::ser::{CompactFormatter, Formatter};

use super::{report, wire, work::Work, Candidate, Dimension, Error, Invalid, Limits, Report};
use crate::ByteDigest;

struct Meter<'a> {
    work: &'a mut Work,
    error: Option<Error>,
    depth: usize,
}

impl Meter<'_> {
    fn charge(&mut self, dimension: Dimension, amount: usize) -> io::Result<()> {
        self.work.charge(dimension, amount).map_err(|error| {
            self.error = Some(error.into());
            io::Error::other("compiled protocol output limit")
        })
    }
}

struct Output<'a, 'b> {
    bytes: Vec<u8>,
    meter: &'a RefCell<Meter<'b>>,
}

impl Write for Output<'_, '_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut meter = self.meter.borrow_mut();
        let required = self.bytes.len().saturating_add(bytes.len());
        meter.charge(Dimension::OutputBytes, required)?;
        meter.charge(Dimension::ByteWork, bytes.len())?;
        if required > self.bytes.capacity() {
            // A bounded geometric reservation avoids quadratic tiny writes.
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(required)
                .min(meter.work.limits.output_bytes);
            meter.charge(Dimension::ByteWork, self.bytes.len())?;
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| {
                    meter.error = Some(Error::Allocation);
                    io::Error::other("compiled protocol output allocation")
                })?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Compact<'a, 'b>(&'a RefCell<Meter<'b>>);

impl Compact<'_, '_> {
    fn begin(&mut self) -> io::Result<()> {
        let mut meter = self.0.borrow_mut();
        let depth = meter.depth + 1;
        meter.charge(Dimension::Depth, depth)?;
        meter.depth = depth;
        Ok(())
    }
    fn end(&mut self) {
        self.0.borrow_mut().depth -= 1;
    }
}

// Unoverridden Formatter methods have the same defaults as CompactFormatter.
impl Formatter for Compact<'_, '_> {
    fn write_u32<W: Write + ?Sized>(&mut self, writer: &mut W, value: u32) -> io::Result<()> {
        if value > 1_048_576 {
            self.0.borrow_mut().error = Some(Error::Invalid(Invalid::StructuralInteger));
            return Err(io::Error::other("compiled protocol structural integer"));
        }
        CompactFormatter.write_u32(writer, value)
    }
    fn begin_array<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
        self.begin()?;
        CompactFormatter.begin_array(writer)
    }
    fn end_array<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
        self.end();
        CompactFormatter.end_array(writer)
    }
    fn begin_object<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
        self.begin()?;
        CompactFormatter.begin_object(writer)
    }
    fn end_object<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
        self.end();
        CompactFormatter.end_object(writer)
    }
}

pub(super) fn bytes(package: &wire::Package, work: &mut Work) -> Result<Vec<u8>, Error> {
    // Canonical output is package-wide; a prior semantic pass's last value is
    // not the source owner of an output allocation or byte-budget refusal.
    work.locus = None;
    let meter = RefCell::new(Meter {
        work,
        error: None,
        depth: 0,
    });
    let mut output = Output {
        bytes: Vec::new(),
        meter: &meter,
    };
    let mut encoder = serde_json::Serializer::with_formatter(&mut output, Compact(&meter));
    if package.serialize(&mut encoder).is_err() {
        return Err(meter
            .borrow_mut()
            .error
            .take()
            .unwrap_or(Error::Invalid(Invalid::Encoding)));
    }
    Ok(output.bytes)
}

/// Encode an untrusted transport candidate using the contract's exact spelling.
/// This utility returns no accepted external reference or compilation authority.
pub fn encode_candidate(package: &wire::Package, limits: Limits) -> Report<Candidate> {
    let mut work = Work::new(limits);
    let result = (|| {
        super::validate::numbers(package, &mut work)?;
        work.locus = None;
        census::reserve(package, &mut work)?;
        let bytes = bytes(package, &mut work)?;
        work.bytes(bytes.len())?;
        let digest = ByteDigest::of(&bytes);
        Ok(Candidate { bytes, digest })
    })();
    report(work, result)
}
