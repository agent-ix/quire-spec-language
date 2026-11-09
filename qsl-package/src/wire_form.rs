// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checked wire's node and operation spellings, converted once.
//!
//! A checked node spells its kind as a `node_tag` word and a `semantic_form`
//! word, and an operation spells its identity as a dotted word
//! (`quire.op.<family>.<name>`). ADR-012 section 9 lets a string select
//! behaviour only at a marked edge, so each conversion below is the one place
//! such a spelling is compared; every caller holds the closed enum.

#[cfg(feature = "std")]
use quire_contract_model::CheckedSemanticNodeV2;

/// A checked node's `node_tag`, for the kinds the package readers select by.
#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WireTag {
    ScalarType,
    CompositeType,
    Model,
    Value,
    Function,
    Temporal,
    Protocol,
}

#[cfg(feature = "std")]
impl WireTag {
    /// The tag a wire word names, `None` for a kind no reader selects by.
    #[ix_trace_rs::string_edge]
    pub(crate) fn from_wire(word: &str) -> Option<Self> {
        Some(match word {
            "scalar_type" => Self::ScalarType,
            "composite_type" => Self::CompositeType,
            "model" => Self::Model,
            "value" => Self::Value,
            "function" => Self::Function,
            "temporal" => Self::Temporal,
            "protocol" => Self::Protocol,
            _ => return None,
        })
    }
}

/// A checked node's `semantic_form`, for the forms the package readers
/// select by.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WireForm {
    Integer,
    Rational,
    Text,
    Query,
    Unit,
    CompoundUnit,
    Dimension,
    Record,
    Union,
    Option,
    Sequence,
    RecordValueType,
    RecordValue,
    TupleValue,
    UnionValue,
    CollectionValue,
    OptionValue,
    EnumValue,
    PureFunction,
    Formula,
    TemporalClause,
    ProtocolClause,
    Compensation,
}

impl WireForm {
    /// The form a wire word names, `None` for a form no reader selects by.
    #[ix_trace_rs::string_edge]
    pub(crate) fn from_wire(word: &str) -> Option<Self> {
        Some(match word {
            "integer" => Self::Integer,
            "rational" => Self::Rational,
            "text" => Self::Text,
            "query" => Self::Query,
            "unit" => Self::Unit,
            "compound_unit" => Self::CompoundUnit,
            "dimension" => Self::Dimension,
            "record" => Self::Record,
            "union" => Self::Union,
            "option" => Self::Option,
            "sequence" => Self::Sequence,
            "record_value_type" => Self::RecordValueType,
            "record_value" => Self::RecordValue,
            "tuple_value" => Self::TupleValue,
            "union_value" => Self::UnionValue,
            "collection_value" => Self::CollectionValue,
            "option_value" => Self::OptionValue,
            "enum_value" => Self::EnumValue,
            "pure_function" => Self::PureFunction,
            "formula" => Self::Formula,
            "temporal_clause" => Self::TemporalClause,
            "protocol_clause" => Self::ProtocolClause,
            "compensation" => Self::Compensation,
            _ => return None,
        })
    }
}

/// Whether `node` has tag `tag`.
#[cfg(feature = "std")]
pub(crate) fn has_tag(node: &CheckedSemanticNodeV2, tag: WireTag) -> bool {
    WireTag::from_wire(&node.node_tag) == Some(tag)
}

/// Whether `node` has tag `tag` and form `form`.
#[cfg(feature = "std")]
pub(crate) fn has_form(node: &CheckedSemanticNodeV2, tag: WireTag, form: WireForm) -> bool {
    has_tag(node, tag) && WireForm::from_wire(&node.semantic_form) == Some(form)
}

#[cfg(feature = "std")]
/// The tag and form a `tag/form` wire word (a checked-error `form`) names.
#[ix_trace_rs::string_edge]
pub(crate) fn split_tag_form(word: &str) -> Option<(WireTag, WireForm)> {
    let (tag, form) = word.split_once('/')?;
    Some((WireTag::from_wire(tag)?, WireForm::from_wire(form)?))
}

/// Whether `role` spells the `expression` occurrence role (FR-322).
#[ix_trace_rs::string_edge]
pub(crate) fn is_expression_role(role: &str) -> bool {
    role == "expression"
}

/// The suffix of an operation identity a reader selects by.
#[cfg(feature = "std")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperationSuffix {
    /// `.eq`.
    Eq,
    /// `.ne`.
    Ne,
    /// `.convert`.
    Convert,
}

#[cfg(feature = "std")]
impl OperationSuffix {
    /// The suffix an operation identity ends with, `None` for any other.
    #[ix_trace_rs::string_edge]
    pub(crate) fn of(identity: &str) -> Option<Self> {
        if identity.ends_with(".eq") {
            Some(Self::Eq)
        } else if identity.ends_with(".ne") {
            Some(Self::Ne)
        } else if identity.ends_with(".convert") {
            Some(Self::Convert)
        } else {
            None
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn tag_and_form_words_decode_to_their_variants() {
        assert_eq!(WireTag::from_wire("scalar_type"), Some(WireTag::ScalarType));
        assert_eq!(WireTag::from_wire("protocol"), Some(WireTag::Protocol));
        assert_eq!(WireTag::from_wire("claim"), None);
        assert_eq!(
            WireForm::from_wire("compound_unit"),
            Some(WireForm::CompoundUnit)
        );
        assert_eq!(
            WireForm::from_wire("union_value"),
            Some(WireForm::UnionValue)
        );
        assert_eq!(WireForm::from_wire("union"), Some(WireForm::Union));
        assert_eq!(WireForm::from_wire("refinement"), None);
    }

    #[test]
    fn joined_tag_form_words_split_into_both_parts() {
        assert_eq!(
            split_tag_form("scalar_type/dimension"),
            Some((WireTag::ScalarType, WireForm::Dimension))
        );
        assert_eq!(
            split_tag_form("composite_type/record"),
            Some((WireTag::CompositeType, WireForm::Record))
        );
        assert_eq!(split_tag_form("scalar_type"), None);
        assert_eq!(split_tag_form("scalar_type/unknown"), None);
    }

    #[test]
    fn operation_suffixes_select_the_equality_and_convert_operations() {
        assert_eq!(
            OperationSuffix::of("quire.op.integer.eq"),
            Some(OperationSuffix::Eq)
        );
        assert_eq!(
            OperationSuffix::of("quire.op.text.ne"),
            Some(OperationSuffix::Ne)
        );
        assert_eq!(
            OperationSuffix::of("quire.op.quantity.convert"),
            Some(OperationSuffix::Convert)
        );
        assert_eq!(OperationSuffix::of("quire.op.integer.lt"), None);
    }

    #[test]
    fn only_the_expression_role_is_an_expression() {
        assert!(is_expression_role("expression"));
        assert!(!is_expression_role("declaration"));
    }
}
