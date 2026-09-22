// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-007: Serde-owned JSON output with measured grammar events and bounded writes.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::ser::{CharEscape, CompactFormatter, Formatter};

use super::{PackageLimits, PackagePassUsage, PackagePathSegment};
use qsl_foundation::Code;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(super) struct Failure {
    pub code: Code,
    pub usage: PackagePassUsage,
    pub path: Vec<PackagePathSegment>,
    pub cause: serde_json::Error,
}

struct Output {
    bytes: Vec<u8>,
    maximum: usize,
    capture: bool,
    exhausted: bool,
}

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.capture {
            if !self
                .bytes
                .len()
                .checked_add(bytes.len())
                .is_some_and(|n| n <= self.maximum)
            {
                self.exhausted = true;
                return Err(io::Error::other("package output byte limit"));
            }
            self.bytes.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

enum Frame {
    Object { key: String, reading_key: bool },
    Array { next: usize },
}

struct Meter {
    limits: PackageLimits,
    usage: PackagePassUsage,
    path: Vec<PackagePathSegment>,
    frames: Vec<Frame>,
    exhausted: bool,
}

impl Meter {
    fn exhausted(&mut self) -> io::Error {
        self.exhausted = true;
        io::Error::other("package traversal limit")
    }

    fn entry(&mut self) -> io::Result<()> {
        let next = self
            .usage
            .entries
            .checked_add(1)
            .ok_or_else(|| self.exhausted())?;
        if next > self.limits.entries {
            return Err(self.exhausted());
        }
        self.usage.entries = next;
        Ok(())
    }

    fn text(&mut self, bytes: usize) -> io::Result<()> {
        let next = self
            .usage
            .string_bytes
            .checked_add(bytes)
            .ok_or_else(|| self.exhausted())?;
        if next > self.limits.string_bytes {
            return Err(self.exhausted());
        }
        self.usage.string_bytes = next;
        Ok(())
    }

    fn enter(&mut self, frame: Frame) -> io::Result<()> {
        let depth = self
            .frames
            .len()
            .checked_add(1)
            .ok_or_else(|| self.exhausted())?;
        if depth > self.limits.depth {
            return Err(self.exhausted());
        }
        self.usage.max_depth = self.usage.max_depth.max(depth);
        self.frames.push(frame);
        Ok(())
    }

    fn key(&mut self) -> Option<&mut String> {
        match self.frames.last_mut() {
            Some(Frame::Object {
                key,
                reading_key: true,
            }) => Some(key),
            Some(
                Frame::Object {
                    reading_key: false, ..
                }
                | Frame::Array { .. },
            )
            | None => None,
        }
    }
}

// Borrow the formatter so its counters/path remain available after Serde stops.
// CompactFormatter retains ownership of JSON punctuation and escaping.
impl Formatter for &mut Meter {
    fn begin_object<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.enter(Frame::Object {
            key: String::new(),
            reading_key: false,
        })?;
        CompactFormatter.begin_object(writer)
    }

    fn end_object<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        CompactFormatter.end_object(writer)?;
        self.frames.pop();
        Ok(())
    }

    fn begin_array<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.enter(Frame::Array { next: 0 })?;
        CompactFormatter.begin_array(writer)
    }

    fn end_array<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        CompactFormatter.end_array(writer)?;
        self.frames.pop();
        Ok(())
    }

    fn begin_object_key<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.entry()?;
        let Some(Frame::Object { reading_key, .. }) = self.frames.last_mut() else {
            return Err(io::Error::other("object key outside object"));
        };
        *reading_key = true;
        CompactFormatter.begin_object_key(writer, first)
    }

    fn begin_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        let Some(Frame::Object { key, reading_key }) = self.frames.last_mut() else {
            return Err(io::Error::other("object value outside object"));
        };
        *reading_key = false;
        let key = std::mem::take(key);
        self.path.push(PackagePathSegment::Field(key));
        CompactFormatter.begin_object_value(writer)
    }

    fn end_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        CompactFormatter.end_object_value(writer)?;
        self.path.pop();
        Ok(())
    }

    fn begin_array_value<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        let Some(Frame::Array { next }) = self.frames.last_mut() else {
            return Err(io::Error::other("array element outside array"));
        };
        let index = *next;
        *next = next
            .checked_add(1)
            .ok_or_else(|| io::Error::other("array index overflow"))?;
        self.path.push(PackagePathSegment::Index(index));
        self.entry()?;
        CompactFormatter.begin_array_value(writer, first)
    }

    fn end_array_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        CompactFormatter.end_array_value(writer)?;
        self.path.pop();
        Ok(())
    }

    fn write_string_fragment<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        self.text(fragment.len())?;
        if let Some(key) = self.key() {
            key.push_str(fragment);
        }
        CompactFormatter.write_string_fragment(writer, fragment)
    }

    fn write_char_escape<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        escape: CharEscape,
    ) -> io::Result<()> {
        self.text(1)?;
        if let Some(key) = self.key() {
            let character = match &escape {
                CharEscape::Quote => '"',
                CharEscape::ReverseSolidus => '\\',
                CharEscape::Solidus => '/',
                CharEscape::Backspace => '\u{8}',
                CharEscape::FormFeed => '\u{c}',
                CharEscape::LineFeed => '\n',
                CharEscape::CarriageReturn => '\r',
                CharEscape::Tab => '\t',
                CharEscape::AsciiControl(byte) => char::from(*byte),
            };
            key.push(character);
        }
        CompactFormatter.write_char_escape(writer, escape)
    }
}

pub(super) fn run(
    value: &impl Serialize,
    limits: PackageLimits,
    capture: bool,
) -> Result<(Vec<u8>, PackagePassUsage), Failure> {
    let mut output = Output {
        bytes: Vec::new(),
        maximum: limits.artifact_bytes,
        capture,
        exhausted: false,
    };
    let mut meter = Meter {
        limits,
        usage: PackagePassUsage::default(),
        path: Vec::new(),
        frames: Vec::new(),
        exhausted: false,
    };
    let result = value.serialize(&mut serde_json::Serializer::with_formatter(
        &mut output,
        &mut meter,
    ));
    meter.usage.output_bytes = output.bytes.len();
    result.map_err(|cause| Failure {
        code: if meter.exhausted || output.exhausted {
            Code::ResourceExhausted
        } else {
            Code::InvalidPackage
        },
        usage: meter.usage,
        path: meter.path,
        cause,
    })?;
    Ok((output.bytes, meter.usage))
}

/// Compare decoded typed claims in producer field order without another JSON tree.
pub(super) fn compare(
    value: &impl Serialize,
    expected: &[u8],
    limits: PackageLimits,
) -> Result<PackagePassUsage, Failure> {
    struct Comparison<'a> {
        expected: &'a [u8],
        position: usize,
    }
    impl Write for Comparison<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let end = self
                .position
                .checked_add(bytes.len())
                .ok_or_else(|| io::Error::other("package comparison length"))?;
            if self.expected.get(self.position..end) != Some(bytes) {
                return Err(io::Error::other(
                    "package claim differs from reconstructed content",
                ));
            }
            self.position = end;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut output = Comparison {
        expected,
        position: 0,
    };
    let mut meter = Meter {
        limits,
        usage: PackagePassUsage::default(),
        path: Vec::new(),
        frames: Vec::new(),
        exhausted: false,
    };
    let result = value.serialize(&mut serde_json::Serializer::with_formatter(
        &mut output,
        &mut meter,
    ));
    let result = result.and_then(|()| {
        if output.position == expected.len() {
            Ok(())
        } else {
            Err(serde::ser::Error::custom(
                "package comparison ended before reconstructed content",
            ))
        }
    });
    result.map(|()| meter.usage).map_err(|cause| Failure {
        code: if meter.exhausted {
            Code::ResourceExhausted
        } else {
            Code::InvalidPackage
        },
        usage: meter.usage,
        path: meter.path,
        cause,
    })
}
