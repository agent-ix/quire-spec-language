// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 text: validated UTF-8 payloads, the six text profiles, bounded
//! `Text[min,max; profile]` admission and metered comparison.
//!
//! Normalization uses exactly the Unicode 17.0.0 data selected by
//! `quire.value.text.unicode-17.0.0/v1`. The build refuses any other
//! `unicode-normalization` table version. No locale, case folding, collation
//! or grapheme segmentation is applied.

use super::outcome::{Outcome, Refusal, Stop};
// `UNICODE_TEXT_DEFINITION`/`UNICODE_VERSION` and the Unicode-17 table-version
// assertion are `quire_exact`'s own canonical items (QSL-131 K1): this crate
// no longer keeps a second copy of the constants its own normalizing
// profiles are checked against, since `TextProfile::table_definition()`
// already returns `quire_exact`'s constant.
use quire_exact::{
    length_amount, Charge, ChargePoint, ComparisonOperator, IllTyped, IllTypedCause, InvalidUtf8,
    LimitKind, Meter, TextProfile, TextProvenance, TextType,
};

/// A source text literal is not one complete JSON-compatible quoted string
/// denoting Unicode scalars.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("invalid text literal")]
pub struct InvalidTextLiteral;

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

    /// Decode a complete quoted source literal, such as `"é"`.
    ///
    /// The escape grammar is the JSON string grammar the source lexer already
    /// delegates to, so lone surrogates and raw controls refuse.
    pub fn from_source_literal(spelling: &str) -> Result<Self, InvalidTextLiteral> {
        if !(spelling.len() >= 2 && spelling.starts_with('"') && spelling.ends_with('"')) {
            return Err(InvalidTextLiteral);
        }
        let text: String = serde_json::from_str(spelling).map_err(|_| InvalidTextLiteral)?;
        Ok(Self {
            text: text.into(),
            provenance: TextProvenance::SourceLiteral(spelling.into()),
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
        self.text_type.profile().length(&self.retained)
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
    let profile = left.text_type.profile();
    if profile != right.text_type.profile() {
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
    let [retained] = prepare(text_type.profile(), [payload], Some(text_type), meter)?;
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
