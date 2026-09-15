// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-001/004: exact byte integrity, independent of semantic canonicalization.
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};

/// SHA-256 value with canonical lowercase, algorithm-prefixed text encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ByteDigest([u8; 32]);

impl ByteDigest {
    /// Hash precisely the supplied bytes, without parsing or normalization.
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    // Native input fields declare the algorithm through their enclosing format.
    // Other digest domains retain the algorithm-prefixed FromStr contract.
    pub(crate) fn from_hex(hex: &str) -> Result<Self, InvalidDigest> {
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(InvalidDigest);
        }
        let mut bytes = [0; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).map_err(|_| InvalidDigest)?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for ByteDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("sha256:")?;
        fmt::LowerHex::fmt(self, f)
    }
}

impl fmt::LowerHex for ByteDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Refusal of a noncanonical or malformed textual byte digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDigest;

impl fmt::Display for InvalidDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected sha256: followed by 64 lowercase hexadecimal digits")
    }
}
impl std::error::Error for InvalidDigest {}

impl FromStr for ByteDigest {
    type Err = InvalidDigest;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let hex = text.strip_prefix("sha256:").ok_or(InvalidDigest)?;
        Self::from_hex(hex)
    }
}
