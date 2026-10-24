//! The strict wire reader of `limits.supplied` (QSpec FR-323), shared by the
//! replay request and the command request.

use qsl_foundation::diagnostic::Code;
use qsl_semantics::value::supplied::SuppliedLimits;

/// Why a wire `limits.supplied` object is refused before anything is
/// consumed (QSpec FR-323 AC-10): the object is closed and complete, and
/// each ceiling is an exact unsigned 64-bit integer. No ceiling is borrowed
/// from a stage setting, the evaluator or the population ceiling.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SuppliedLimitsRefusal {
    /// `limits.supplied` is not a JSON object.
    #[error("invalid_runtime_input/invalid-value: limits.supplied is not an object")]
    NotAnObject,
    /// The object names a member that is no ceiling.
    #[error("invalid_runtime_input/unknown-member: limits.supplied has no member `{name}`")]
    UnknownMember {
        /// The member as the wire wrote it.
        name: String,
    },
    /// A ceiling is absent: the effective object is complete, with no
    /// fallback to another limit or to a default.
    #[error("invalid_runtime_input/invalid-value: limits.supplied lacks `{member}`")]
    Incomplete {
        /// The absent ceiling's setting name.
        member: &'static str,
    },
    /// A ceiling is not an exact unsigned 64-bit integer.
    #[error("invalid_runtime_input/invalid-value: limits.supplied `{member}` is not an unsigned 64-bit integer")]
    NotUnsigned64 {
        /// The ceiling's setting name.
        member: &'static str,
    },
}

impl SuppliedLimitsRefusal {
    /// The catalog code of this refusal.
    pub const fn code(&self) -> Code {
        Code::InvalidRuntimeInput
    }
}

/// Read a wire `limits.supplied` object (QSpec FR-323 AC-10): exactly
/// `admission_work_units`, `conversion_work_units` and
/// `numeric_magnitude_bits` (B), each an exact `u64`.
///
/// # Errors
///
/// [`SuppliedLimitsRefusal`] for a non-object, an unknown member, a missing
/// ceiling or a ceiling that is no `u64`.
pub fn read_supplied_limits(
    value: &serde_json::Value,
) -> Result<SuppliedLimits, SuppliedLimitsRefusal> {
    const ADMISSION: &str = "admission_work_units";
    const CONVERSION: &str = "conversion_work_units";
    const MAGNITUDE: &str = "numeric_magnitude_bits";
    let serde_json::Value::Object(members) = value else {
        return Err(SuppliedLimitsRefusal::NotAnObject);
    };
    if let Some(name) = members
        .keys()
        .find(|name| !matches!(name.as_str(), ADMISSION | CONVERSION | MAGNITUDE))
    {
        return Err(SuppliedLimitsRefusal::UnknownMember { name: name.clone() });
    }
    let ceiling = |member: &'static str| {
        members
            .get(member)
            .ok_or(SuppliedLimitsRefusal::Incomplete { member })?
            .as_u64()
            .ok_or(SuppliedLimitsRefusal::NotUnsigned64 { member })
    };
    Ok(SuppliedLimits::default()
        .with_admission_work_units(ceiling(ADMISSION)?)
        .with_conversion_work_units(ceiling(CONVERSION)?)
        .with_numeric_magnitude_bits(ceiling(MAGNITUDE)?))
}
