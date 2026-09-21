// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 text: validated UTF-8 payloads, the six text profiles, bounded
//! `Text[min,max; profile]` admission and metered comparison.
//!
//! Normalization uses exactly the Unicode 17.0.0 data selected by
//! `quire.value.text.unicode-17.0.0/v1`. The build refuses any other
//! `unicode-normalization` table version. No locale, case folding, collation
//! or grapheme segmentation is applied.
//!
//! Ported from QSL `value::text` as part of QSL#213 S-1 (ADR-011 X-1). One
//! cut (M-6): `TextPayload::from_source_literal`/`InvalidTextLiteral`,
//! which decoded a complete quoted source literal's JSON string escapes,
//! are dropped. Decoding a source-lexer literal spelling is a source-stage
//! concern, not an O-13 value concern, and nothing in this crate called it
//! (only its own now-removed tests did) -- QSL's own lexer/parser is
//! already the right place to decode a literal's escapes before handing
//! this crate the resulting scalar sequence via [`TextPayload::from_utf8`].
//! [`TextProvenance::SourceLiteral`] itself is unaffected and still directly
//! constructible: recording that a payload came from a source literal, and
//! which one, is a real kernel-observable fact; only the JSON-based decode
//! convenience was QSL's job, not the kernel's.

use std::cmp::Ordering;
use std::str::Chars;

use unicode_normalization::{Decompositions, Recompositions, UnicodeNormalization};

use crate::accounting::{length_amount, Charge, ChargePoint, LimitKind, Meter};
use crate::comparison::{ComparisonOperator, IllTyped, IllTypedCause};
use crate::outcome::{Outcome, Refusal, Stop};

/// Definition identity of the normalization tables.
pub const UNICODE_TEXT_DEFINITION: &str = "quire.value.text.unicode-17.0.0/v1";

/// Unicode version the normalization tables implement.
pub const UNICODE_VERSION: (u8, u8, u8) = (17, 0, 0);

const _: () = assert!(
    matches!(unicode_normalization::UNICODE_VERSION, (17, 0, 0)),
    "quire.value.text.unicode-17.0.0/v1 requires Unicode 17.0.0 normalization tables"
);

/// One FR-141 text profile.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TextProfile {
    /// Decoded scalar sequence without normalization.
    UnicodeScalars,
    /// Unicode Normalization Form C.
    Nfc,
    /// Unicode Normalization Form D.
    Nfd,
    /// Unicode Normalization Form KC.
    Nfkc,
    /// Unicode Normalization Form KD.
    Nfkd,
    /// The admitted UTF-8 payload bytes.
    BinaryUtf8,
}

impl TextProfile {
    /// Every profile in FR-141 order.
    pub const ALL: [Self; 6] = [
        Self::UnicodeScalars,
        Self::Nfc,
        Self::Nfd,
        Self::Nfkc,
        Self::Nfkd,
        Self::BinaryUtf8,
    ];

    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnicodeScalars => "unicode-scalars",
            Self::Nfc => "nfc",
            Self::Nfd => "nfd",
            Self::Nfkc => "nfkc",
            Self::Nfkd => "nfkd",
            Self::BinaryUtf8 => "binary-utf8",
        }
    }

    /// Resolve a normative spelling.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|profile| profile.as_str() == code)
    }

    /// The normalization form this profile applies, if any.
    pub fn normalization(self) -> Option<NormalizationForm> {
        match self {
            Self::Nfc => Some(NormalizationForm::Nfc),
            Self::Nfd => Some(NormalizationForm::Nfd),
            Self::Nfkc => Some(NormalizationForm::Nfkc),
            Self::Nfkd => Some(NormalizationForm::Nfkd),
            Self::UnicodeScalars | Self::BinaryUtf8 => None,
        }
    }

    /// The Unicode table definition a normalizing profile evaluates under.
    pub fn table_definition(self) -> Option<&'static str> {
        self.normalization().map(|_| UNICODE_TEXT_DEFINITION)
    }

    /// Profile length of a retained sequence: bytes for `binary-utf8`, scalars
    /// otherwise, counted on the retained (normalized) sequence.
    fn length(self, retained: &str) -> u64 {
        let count = match self {
            Self::BinaryUtf8 => retained.len(),
            Self::UnicodeScalars | Self::Nfc | Self::Nfd | Self::Nfkc | Self::Nfkd => {
                retained.chars().count()
            }
        };
        length_amount(count)
    }

    fn order(self, left: &str, right: &str) -> Ordering {
        match self {
            Self::BinaryUtf8 => left.as_bytes().cmp(right.as_bytes()),
            Self::UnicodeScalars | Self::Nfc | Self::Nfd | Self::Nfkc | Self::Nfkd => {
                left.chars().cmp(right.chars())
            }
        }
    }
}

/// A Unicode normalization form.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NormalizationForm {
    /// Canonical decomposition then canonical composition.
    Nfc,
    /// Canonical decomposition.
    Nfd,
    /// Compatibility decomposition then canonical composition.
    Nfkc,
    /// Compatibility decomposition.
    Nfkd,
}

impl NormalizationForm {
    /// Stream the normalized scalars of `text`.
    pub(crate) fn apply(self, text: &str) -> Normalized<'_> {
        match self {
            Self::Nfc => Normalized::Composed(text.nfc()),
            Self::Nfd => Normalized::Decomposed(text.nfd()),
            Self::Nfkc => Normalized::Composed(text.nfkc()),
            Self::Nfkd => Normalized::Decomposed(text.nfkd()),
        }
    }
}

/// A streaming normalization of one scalar sequence.
pub(crate) enum Normalized<'a> {
    Composed(Recompositions<Chars<'a>>),
    Decomposed(Decompositions<Chars<'a>>),
}

impl Iterator for Normalized<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self {
            Self::Composed(scalars) => scalars.next(),
            Self::Decomposed(scalars) => scalars.next(),
        }
    }
}

/// Declared `Text[min,max; profile]` bounds are empty.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("text bounds [{min}, {max}] are empty")]
pub struct EmptyTextBounds {
    /// Declared minimum length.
    pub min: u64,
    /// Declared maximum length.
    pub max: u64,
}

/// A bounded text type `Text[min,max; profile]`. No unbounded text type exists.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextType {
    min: u64,
    max: u64,
    profile: TextProfile,
}

impl TextType {
    /// Declare inclusive length bounds under `profile`.
    pub fn new(min: u64, max: u64, profile: TextProfile) -> Result<Self, EmptyTextBounds> {
        if min > max {
            return Err(EmptyTextBounds { min, max });
        }
        Ok(Self { min, max, profile })
    }

    /// Inclusive minimum profile length.
    pub fn min(&self) -> u64 {
        self.min
    }

    /// Inclusive maximum profile length.
    pub fn max(&self) -> u64 {
        self.max
    }

    /// Selected profile.
    pub fn profile(&self) -> TextProfile {
        self.profile
    }

    fn admits(&self, retained: &str) -> bool {
        (self.min..=self.max).contains(&self.profile.length(retained))
    }
}

/// Bytes presented at the UTF-8 reader boundary are not valid UTF-8.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("invalid UTF-8 after {valid_up_to} valid bytes")]
pub struct InvalidUtf8 {
    /// Length of the longest valid prefix.
    pub valid_up_to: usize,
}

/// Where a payload came from. Provenance never participates in text value
/// comparison.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TextProvenance {
    /// A source literal with its exact quoted spelling.
    SourceLiteral(Box<str>),
    /// Validated runtime payload bytes.
    Runtime,
}

/// A validated text payload: the canonical UTF-8 encoding of a Unicode scalar
/// sequence plus its provenance.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TextPayload {
    text: Box<str>,
    provenance: TextProvenance,
}

impl TextPayload {
    /// Read runtime payload bytes; invalid UTF-8 refuses before any profile.
    pub fn from_utf8(bytes: &[u8]) -> Result<Self, InvalidUtf8> {
        let text = std::str::from_utf8(bytes).map_err(|error| InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        })?;
        Ok(Self {
            text: text.into(),
            provenance: TextProvenance::Runtime,
        })
    }

    /// The payload bytes.
    pub fn bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    /// The decoded scalar sequence.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Source or runtime provenance.
    pub fn provenance(&self) -> &TextProvenance {
        &self.provenance
    }
}

/// A completed text value of one [`TextType`].
#[derive(Clone, Debug)]
pub struct Text {
    text_type: TextType,
    payload: TextPayload,
    retained: Box<str>,
}

impl Text {
    /// The declared type.
    pub fn text_type(&self) -> &TextType {
        &self.text_type
    }

    /// The original admitted payload and its provenance.
    pub fn payload(&self) -> &TextPayload {
        &self.payload
    }

    /// The retained sequence: normalized under a normalizing profile, the
    /// payload otherwise.
    pub fn retained(&self) -> &str {
        &self.retained
    }

    /// Profile length of the retained sequence.
    pub fn length(&self) -> u64 {
        self.text_type.profile.length(&self.retained)
    }
}

/// Admit `payload` into `text_type` under `quire.value.accounting/v1`.
pub fn admit_text(payload: &TextPayload, text_type: &TextType, meter: &mut Meter) -> Outcome<Text> {
    Outcome::from_stop(admit(payload, text_type, meter))
}

/// Compare two text values. Operands of different profiles are ill-typed and
/// consume nothing.
pub fn compare_text(
    operator: ComparisonOperator,
    left: &Text,
    right: &Text,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    let profile = left.text_type.profile;
    if profile != right.text_type.profile {
        return Err(IllTyped {
            cause: IllTypedCause::DistinctTextProfiles,
        });
    }
    Ok(Outcome::from_stop(compare(
        operator,
        profile,
        [&left.payload, &right.payload],
        meter,
    )))
}

fn admit(payload: &TextPayload, text_type: &TextType, meter: &mut Meter) -> Result<Text, Stop> {
    let [retained] = prepare(text_type.profile, [payload], Some(text_type), meter)?;
    meter.charge(Charge::new(ChargePoint::TextResultRetain).results(1))?;
    Ok(Text {
        text_type: *text_type,
        payload: payload.clone(),
        retained: retained.into(),
    })
}

fn compare(
    operator: ComparisonOperator,
    profile: TextProfile,
    payloads: [&TextPayload; 2],
    meter: &mut Meter,
) -> Result<bool, Stop> {
    let [left, right] = prepare(profile, payloads, None, meter)?;
    let ordering = profile.order(&left, &right);
    meter.charge(Charge::new(ChargePoint::TextResultRetain).results(1))?;
    Ok(operator.holds(ordering))
}

/// Charge `text.input-bytes` and `text.decode-scalars`, then (for a normalizing
/// profile) `text.normalize-input` and one `text.normalize-output` before each
/// emitted scalar across all operands.
///
/// An admission's length bound refuses as soon as its profile length is
/// known: after `text.input-bytes` for `binary-utf8`, after
/// `text.decode-scalars` for `unicode-scalars`, and after every
/// `text.normalize-output` for a normalizing profile; always before
/// `text.result-retain`.
fn prepare<const N: usize>(
    profile: TextProfile,
    payloads: [&TextPayload; N],
    bound: Option<&TextType>,
    meter: &mut Meter,
) -> Result<[String; N], Stop> {
    let sum = |measure: fn(&str) -> usize| {
        payloads.iter().fold(0_u64, |total, payload| {
            // Each length is at most `isize::MAX`, so the few payloads of one
            // operation sum far below `u64::MAX`.
            total
                .checked_add(length_amount(measure(payload.as_str())))
                .expect("the payload lengths of one text operation sum within u64")
        })
    };
    let inputs = payloads.map(TextPayload::as_str);
    meter.charge(
        Charge::new(ChargePoint::TextInputBytes)
            .size(LimitKind::TextInputBytes, sum(str::len))
            .size(LimitKind::ValueOccurrences, length_amount(N)),
    )?;
    if profile == TextProfile::BinaryUtf8 {
        check_length(bound, &inputs)?;
    }
    meter.charge(
        Charge::new(ChargePoint::TextDecodeScalars)
            .size(LimitKind::TextScalars, sum(|text| text.chars().count())),
    )?;
    // Non-normalizing profiles charge no `text.normalize-*` point.
    let Some(form) = profile.normalization() else {
        if profile == TextProfile::UnicodeScalars {
            check_length(bound, &inputs)?;
        }
        return Ok(inputs.map(str::to_owned));
    };
    meter.charge(Charge::new(ChargePoint::TextNormalizeInput))?;
    let mut emitted = 0_u64;
    let mut sequences = payloads.map(|_| String::new());
    for (input, sequence) in inputs.iter().zip(sequences.iter_mut()) {
        for scalar in form.apply(input) {
            emitted = emitted.saturating_add(1);
            meter.charge(
                Charge::new(ChargePoint::TextNormalizeOutput)
                    .size(LimitKind::NormalizedScalars, emitted),
            )?;
            sequence.push(scalar);
        }
    }
    check_length(bound, &sequences.each_ref().map(String::as_str))?;
    Ok(sequences)
}

/// Refuse a retained sequence outside an admission's declared length bounds.
fn check_length(bound: Option<&TextType>, sequences: &[&str]) -> Result<(), Stop> {
    match bound {
        Some(text_type) if !sequences.iter().all(|sequence| text_type.admits(sequence)) => {
            Err(Stop::Refused(Refusal::TextLengthOutOfDomain))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::accounting::ScalarLimits;

    fn generous_meter() -> Meter {
        Meter::new(ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        })
    }

    /// TC-341: a runtime UTF-8 payload within its declared length bounds is
    /// admitted unchanged.
    #[trace("TC-341")]
    #[test]
    fn tc_341_utf8_payload_within_bound_is_admitted() {
        let payload = TextPayload::from_utf8("café".as_bytes()).unwrap();
        assert_eq!(payload.as_str(), "café");
        let text_type = TextType::new(0, 10, TextProfile::UnicodeScalars).unwrap();
        let mut meter = generous_meter();
        let outcome = admit_text(&payload, &text_type, &mut meter);
        let text = outcome.completed().expect("within the declared bound");
        assert_eq!(text.retained(), "café");
    }

    /// TC-342: a payload longer than the declared maximum is refused with
    /// `TextLengthOutOfDomain`, never silently truncated.
    #[trace("TC-342")]
    #[test]
    fn tc_342_admit_refuses_past_the_declared_maximum() {
        let payload = TextPayload::from_utf8("hello".as_bytes()).unwrap();
        let text_type = TextType::new(0, 3, TextProfile::UnicodeScalars).unwrap();
        let mut meter = generous_meter();
        let outcome = admit_text(&payload, &text_type, &mut meter);
        assert!(matches!(
            outcome,
            Outcome::Refused(Refusal::TextLengthOutOfDomain)
        ));
    }
}
