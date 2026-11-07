// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 "The equality file": the producer's equality witness rows
//! (`equality.json`, `quire.checked-handoff.equality/1`) written beside the
//! checked package and read back for a bytes-only consumer.
//!
//! The rows are Contract IR's `SelectedEqualityApplicationRow`,
//! `SelectedEqualityFactRow` and `SelectedSourceUnitRow`. Their node ids and
//! keys are spelled with the wire node ids of the package
//! (`WireGraph::of_package_bytes`). `read` derives no row from the package
//! bytes, a display name or source text.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::{
    AdmittedEqualityReadError, CheckedNodeId, CheckedNodeOwner, CheckedOccurrence,
    CheckedPackageV2, CheckedRational, CheckedSourceMapEntry, EqualityDimensionRole,
    EqualityFactWitness, EqualityOperandTypeRole, EqualityUnitRole, SelectedEqualityApplicationRow,
    SelectedEqualityFactRow, SelectedSourceUnitRow,
};
use quire_exact::{
    BoundedQuantityType, CardinalityBound, CarriedQuantityType, CollectionKind, CollectionType,
    DecimalType, EffectiveId, EnumShape, FloatType, IeeeWidth, Integer, IntegerInterval, Location,
    MapType, NodeKey, Origin, OriginalDeclarationKey, Quantity, QuantityCarrier, Rational,
    RationalDomain, RoundingMode, TextProfile, TextType, UnitId, ValueType, VariantId,
};
use serde::{Deserialize, Serialize};

use crate::checked_v2::wire_graph;

/// The format of `equality.json` (FR-531 "The equality file").
pub const EQUALITY_FORMAT: &str = "quire.checked-handoff.equality/1";

/// The equality rows of one package, read from its `equality.json` or taken
/// from the in-process unit that produced them (FR-390, FR-531).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadEqualityRows {
    package: String,
    applications: Vec<SelectedEqualityApplicationRow>,
    facts: Vec<SelectedEqualityFactRow>,
    source_units: Vec<SelectedSourceUnitRow>,
}

/// Why an equality file was not read or written: a closed set.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EqualityFileRefusal {
    /// Not JSON or not this shape, an unknown or repeated member or row
    /// member, an unknown fact tag or a non-integer number; `detail` names it.
    #[error("the equality file is malformed at {detail}")]
    Malformed {
        /// The member, tag or value the parse stopped at.
        detail: String,
    },
    /// Another `format` string.
    #[error("the equality file has format `{0}`")]
    Format(String),
    /// A row whose node id the package does not hold, or whose application
    /// is not an equality application of the package.
    #[error("the equality file row {index} of `{member}` is not joined to the package")]
    Unjoined {
        /// `applications`, `facts` or `source_units`.
        member: &'static str,
        /// The first row that does not join.
        index: usize,
    },
}

/// Why equality rows are no witness for a slot call (FR-531 "Refusals").
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EqualityWitnessRefusal {
    /// The supply holds no equality rows.
    #[error("the bare package holds no equality rows")]
    Missing,
    /// The rows join to no equality application of the package.
    #[error("the equality rows join to no equality application of the package")]
    Unknown,
    /// The rows were produced for another package.
    #[error("the equality rows belong to another package")]
    Foreign,
}

// ---- the ordered JSON tree ------------------------------------------------

/// A JSON value whose objects keep member order and refuse a repeated member.
#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Int(u64),
    Text(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Serialize for Json {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Int(value) => serializer.serialize_u64(*value),
            Self::Text(value) => serializer.serialize_str(value),
            Self::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Object(members) => {
                let mut map = serializer.serialize_map(Some(members.len()))?;
                for (name, value) in members {
                    map.serialize_entry(name, value)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Json;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter
                    .write_str("null, a boolean, an unsigned integer, text, an array or an object")
            }
            fn visit_unit<E>(self) -> Result<Json, E> {
                Ok(Json::Null)
            }
            fn visit_bool<E>(self, value: bool) -> Result<Json, E> {
                Ok(Json::Bool(value))
            }
            fn visit_u64<E>(self, value: u64) -> Result<Json, E> {
                Ok(Json::Int(value))
            }
            fn visit_str<E>(self, value: &str) -> Result<Json, E> {
                Ok(Json::Text(value.to_owned()))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Json::Array(items))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
                let mut members: Vec<(String, Json)> = Vec::new();
                while let Some(name) = map.next_key::<String>()? {
                    if members.iter().any(|(held, _)| *held == name) {
                        return Err(serde::de::Error::custom(format!(
                            "repeated member `{name}`"
                        )));
                    }
                    members.push((name, map.next_value()?));
                }
                Ok(Json::Object(members))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

/// What a decode stopped at.
#[derive(Debug)]
enum Bad {
    Malformed(String),
    /// A node id or key the package does not hold.
    Unjoined,
}

fn malformed<T>(detail: impl Into<String>) -> Result<T, Bad> {
    Err(Bad::Malformed(detail.into()))
}

fn object(members: Vec<(&str, Json)>) -> Json {
    Json::Object(
        members
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn text(value: impl Into<String>) -> Json {
    Json::Text(value.into())
}

fn count(value: usize) -> Json {
    Json::Int(value as u64)
}

fn of<T: Serialize>(value: &T) -> Result<Json, Bad> {
    let bytes = serde_json::to_vec(value).map_err(|e| Bad::Malformed(e.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|e| Bad::Malformed(e.to_string()))
}

fn to<T: for<'de> Deserialize<'de>>(json: &Json, what: &str) -> Result<T, Bad> {
    let bytes = serde_json::to_vec(json).map_err(|e| Bad::Malformed(e.to_string()))?;
    serde_json::from_slice(&bytes).or_else(|_| malformed(what))
}

impl Json {
    /// The members named `names`, which the object holds exactly.
    fn members(&self, names: &[&str]) -> Result<Vec<&Json>, Bad> {
        let Self::Object(held) = self else {
            return malformed("an object");
        };
        if held.len() != names.len() {
            return malformed(format!("members {names:?}"));
        }
        names
            .iter()
            .map(|name| match held.iter().find(|(held, _)| held == name) {
                Some((_, value)) => Ok(value),
                None => malformed(format!("member `{name}`")),
            })
            .collect()
    }

    fn text(&self) -> Result<&str, Bad> {
        match self {
            Self::Text(value) => Ok(value),
            _ => malformed("text"),
        }
    }

    fn int(&self) -> Result<u64, Bad> {
        match self {
            Self::Int(value) => Ok(*value),
            _ => malformed("an integer"),
        }
    }

    fn index(&self) -> Result<usize, Bad> {
        usize::try_from(self.int()?).or_else(|_| malformed("an index"))
    }

    fn boolean(&self) -> Result<bool, Bad> {
        match self {
            Self::Bool(value) => Ok(*value),
            _ => malformed("a boolean"),
        }
    }

    fn array(&self) -> Result<&[Json], Bad> {
        match self {
            Self::Array(items) => Ok(items),
            _ => malformed("an array"),
        }
    }

    /// A tagged object `{ "kind": tag, ... }`: the table row for its tag and
    /// the members that row names.
    #[ix_trace_rs::string_edge]
    fn tagged<'j, 't>(
        &'j self,
        table: &'t [(&'t str, &'t [&'t str])],
    ) -> Result<(&'t str, Vec<&'j Json>), Bad> {
        let Self::Object(held) = self else {
            return malformed("a tagged object");
        };
        let Some((_, Json::Text(tag))) = held.iter().find(|(name, _)| name == "kind") else {
            return malformed("member `kind`");
        };
        let Some((tag, names)) = table.iter().find(|(known, _)| known == tag) else {
            return malformed(format!("tag `{tag}`"));
        };
        let mut all = vec!["kind"];
        all.extend(names.iter().copied());
        let members = self.members(&all)?;
        Ok((tag, members.into_iter().skip(1).collect()))
    }
}

// ---- node joins -----------------------------------------------------------

/// The package's nodes in both directions, for spelling a key as the wire
/// node id and back, and the package's equality applications.
struct Joins {
    ids: BTreeMap<NodeKey, CheckedNodeId>,
    keys: BTreeMap<CheckedNodeId, NodeKey>,
    equality: BTreeSet<CheckedNodeId>,
}

impl Joins {
    fn of_package(package: &CheckedPackageV2) -> Option<Self> {
        let graph = wire_graph(package).ok()?;
        let mut ids = BTreeMap::new();
        let mut keys = BTreeMap::new();
        for node in &package.graph().nodes {
            let key = graph.key(&node.node_id).ok()?;
            ids.insert(key.clone(), node.node_id.clone());
            keys.insert(node.node_id.clone(), key);
        }
        let equality = package
            .checked_term_data()
            .map(|data| {
                data.equality_decisions
                    .iter()
                    .map(|decision| decision.application.clone())
                    .collect()
            })
            .unwrap_or_default();
        Some(Self {
            ids,
            keys,
            equality,
        })
    }

    fn id(&self, key: &NodeKey) -> Result<&CheckedNodeId, Bad> {
        self.ids
            .get(key)
            .map_or_else(|| malformed("a node the package does not name"), Ok)
    }

    fn node_id(&self, key: &NodeKey) -> Result<Json, Bad> {
        of(self.id(key)?)
    }

    fn held(&self, json: &Json) -> Result<(CheckedNodeId, NodeKey), Bad> {
        let id: CheckedNodeId = to(json, "a node id")?;
        let key = self.keys.get(&id).cloned().ok_or(Bad::Unjoined)?;
        Ok((id, key))
    }
}

/// The identity grammar used inside an Exact value type. Equality rows use
/// package references; source declaration rows retain the producer's original
/// opaque values, which need not name nodes of the emitted graph.
trait TypeKeys {
    fn write_key(&self, key: &NodeKey) -> Result<Json, Bad>;
    fn read_key(&self, json: &Json) -> Result<NodeKey, Bad>;
}

impl TypeKeys for Joins {
    fn write_key(&self, key: &NodeKey) -> Result<Json, Bad> {
        self.node_id(key)
    }

    fn read_key(&self, json: &Json) -> Result<NodeKey, Bad> {
        self.held(json).map(|(_, key)| key)
    }
}

// A node identity uses an object and a path array at each nesting level.
// Bound both bridges before recursive serde work; the shared text codec itself
// traverses opaque identities iteratively, without constructing a JSON Value.
const ORIGINAL_KEY_NESTING_LIMIT: usize = 32;

struct OriginalKeys;

fn original_key_depth(key: &NodeKey) -> Result<(), Bad> {
    let mut stack = Vec::new();
    stack
        .try_reserve_exact(ORIGINAL_KEY_NESTING_LIMIT)
        .map_err(|_| Bad::Malformed("original node identity depth storage".into()))?;
    stack.push(key.view().path());
    while let Some(top) = stack.last_mut() {
        match top.next() {
            Some(quire_exact::PathSegmentView::Node(view)) => {
                if stack.len() == ORIGINAL_KEY_NESTING_LIMIT {
                    return malformed("original node identity nesting limit");
                }
                stack.push(view.path());
            }
            Some(quire_exact::PathSegmentView::Text(_)) => {}
            None => {
                stack.pop();
            }
        }
    }
    Ok(())
}

fn original_json_depth(json: &Json) -> Result<(), Bad> {
    let mut pending = vec![(json, 1_usize)];
    while let Some((value, depth)) = pending.pop() {
        let children = match value {
            Json::Array(items) => {
                if depth > 2 * ORIGINAL_KEY_NESTING_LIMIT {
                    return malformed("original node identity nesting limit");
                }
                pending
                    .try_reserve(items.len())
                    .map_err(|_| Bad::Malformed("original node identity depth storage".into()))?;
                pending.extend(items.iter().map(|item| (item, depth + 1)));
                continue;
            }
            Json::Object(members) => members,
            _ => continue,
        };
        if depth > 2 * ORIGINAL_KEY_NESTING_LIMIT {
            return malformed("original node identity nesting limit");
        }
        pending
            .try_reserve(children.len())
            .map_err(|_| Bad::Malformed("original node identity depth storage".into()))?;
        pending.extend(children.iter().map(|(_, value)| (value, depth + 1)));
    }
    Ok(())
}

impl TypeKeys for OriginalKeys {
    fn write_key(&self, key: &NodeKey) -> Result<Json, Bad> {
        original_key_depth(key)?;
        let text = qsl_foundation::node_wire::node_key_to_text(key)
            .map_err(|_| Bad::Malformed("an original node identity".into()))?;
        serde_json::from_str(&text).map_err(|_| Bad::Malformed("an original node identity".into()))
    }

    fn read_key(&self, json: &Json) -> Result<NodeKey, Bad> {
        original_json_depth(json)?;
        let text = serde_json::to_string(json)
            .map_err(|_| Bad::Malformed("an original node identity".into()))?;
        qsl_foundation::node_wire::node_key_from_text(&text)
            .map_err(|_| Bad::Malformed("an original node identity".into()))
    }
}

/// The existing complete Exact value-type grammar with original opaque keys.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct OriginalValueType(Json);

/// An original opaque key, using the shared structural-value codec.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct OriginalNodeKey(Json);

#[derive(Clone, Copy, Debug, thiserror::Error)]
#[error("a malformed original Exact value type")]
pub(crate) struct OriginalValueTypeError;

impl OriginalNodeKey {
    pub(crate) fn of(key: &NodeKey) -> Result<Self, OriginalValueTypeError> {
        OriginalKeys
            .write_key(key)
            .map(Self)
            .map_err(|_| OriginalValueTypeError)
    }

    pub(crate) fn key(&self) -> Result<NodeKey, OriginalValueTypeError> {
        OriginalKeys
            .read_key(&self.0)
            .map_err(|_| OriginalValueTypeError)
    }
}

impl OriginalValueType {
    pub(crate) fn of(value: &ValueType) -> Result<Self, OriginalValueTypeError> {
        value_type(&OriginalKeys, value)
            .map(Self)
            .map_err(|_| OriginalValueTypeError)
    }

    pub(crate) fn value(&self) -> Result<ValueType, OriginalValueTypeError> {
        value_type_from(&OriginalKeys, &self.0).map_err(|_| OriginalValueTypeError)
    }
}

// ---- leaf encodings -------------------------------------------------------

fn unit_id(joins: &impl TypeKeys, unit: &UnitId, compound: bool) -> Result<Json, Bad> {
    match unit {
        UnitId::Declared(key) => Ok(object(vec![
            ("kind", text("declared")),
            ("node", joins.write_key(key)?),
        ])),
        UnitId::Compound(terms) if compound => {
            let terms = terms
                .iter()
                .map(|(key, exponent)| {
                    Ok(object(vec![
                        ("node", joins.write_key(key)?),
                        ("exponent", text(exponent.to_string())),
                    ]))
                })
                .collect::<Result<Vec<_>, Bad>>()?;
            Ok(object(vec![
                ("kind", text("compound")),
                ("terms", Json::Array(terms)),
            ]))
        }
        UnitId::Compound(_) => malformed("a compound unit in a source unit row"),
    }
}

#[ix_trace_rs::string_edge]

fn unit_from(joins: &impl TypeKeys, json: &Json, compound: bool) -> Result<UnitId, Bad> {
    let (tag, members) = json.tagged(&[("declared", &["node"]), ("compound", &["terms"])])?;
    if tag == "declared" {
        return Ok(UnitId::declared(joins.read_key(members[0])?));
    }
    if !compound {
        return malformed("a compound unit in a source unit row");
    }
    let terms = members[0]
        .array()?
        .iter()
        .map(|term| {
            let parts = term.members(&["node", "exponent"])?;
            Ok((joins.read_key(parts[0])?, integer(parts[1])?))
        })
        .collect::<Result<Vec<_>, Bad>>()?;
    Ok(UnitId::compound(terms))
}

#[ix_trace_rs::string_edge]

fn canonical_integer(spelling: &str) -> bool {
    let digits = spelling.strip_prefix('-').unwrap_or(spelling);
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && spelling != "-0"
}

fn integer(json: &Json) -> Result<Integer, Bad> {
    let spelling = json.text()?;
    if !canonical_integer(spelling) {
        return malformed(format!("integer `{spelling}`"));
    }
    spelling
        .parse()
        .or_else(|_| malformed(format!("integer `{spelling}`")))
}

/// The encoding of a canonical rational; the writer refuses what the reader
/// refuses.
fn rational(value: &CheckedRational) -> Result<Json, Bad> {
    let json = object(vec![
        ("numerator", text(&*value.numerator)),
        ("denominator", text(&*value.denominator)),
    ]);
    rational_from(&json)?;
    Ok(json)
}

#[ix_trace_rs::string_edge]

fn rational_from(json: &Json) -> Result<CheckedRational, Bad> {
    let parts = json.members(&["numerator", "denominator"])?;
    let (numerator, denominator) = (parts[0].text()?, parts[1].text()?);
    if !canonical_integer(numerator)
        || !canonical_integer(denominator)
        || denominator.starts_with('-')
        || denominator == "0"
    {
        return malformed("a checked rational");
    }
    Ok(CheckedRational {
        numerator: numerator.into(),
        denominator: denominator.into(),
    })
}

fn location(joins: &Joins, value: &Location) -> Result<Json, Bad> {
    Ok(object(vec![
        ("node", joins.node_id(value.node())?),
        (
            "occurrence",
            object(vec![
                ("role", text(value.occurrence().role().as_str())),
                ("ordinal", Json::Int(value.occurrence().ordinal())),
            ]),
        ),
    ]))
}

fn location_from(joins: &Joins, json: &Json) -> Result<Location, Bad> {
    let parts = json.members(&["node", "occurrence"])?;
    let occurrence = parts[1].members(&["role", "ordinal"])?;
    Ok(Location::new(
        joins.held(parts[0])?.1,
        Origin::new(occurrence[0].text()?.into(), occurrence[1].int()?),
    ))
}

fn effective(value: &EffectiveId) -> Json {
    let chain = value
        .owner_chain()
        .iter()
        .map(|link| {
            object(vec![
                ("package", text(link.package())),
                ("node", text(link.node())),
            ])
        })
        .collect();
    object(vec![("chain", Json::Array(chain))])
}

fn effective_from(json: &Json) -> Result<EffectiveId, Bad> {
    let mut chain = json.members(&["chain"])?[0].array()?.iter();
    let link = |json: &Json| -> Result<OriginalDeclarationKey, Bad> {
        let parts = json.members(&["package", "node"])?;
        Ok(OriginalDeclarationKey::new(
            parts[0].text()?,
            parts[1].text()?,
        ))
    };
    let Some(first) = chain.next() else {
        return malformed("an empty effective id");
    };
    let mut id = EffectiveId::new(None, link(first)?);
    for next in chain {
        id = EffectiveId::new(Some(id), link(next)?);
    }
    Ok(id)
}

const ROUNDINGS: [(&str, RoundingMode); 6] = [
    ("exact", RoundingMode::Exact),
    ("toward_zero", RoundingMode::TowardZero),
    ("toward_positive", RoundingMode::TowardPositive),
    ("toward_negative", RoundingMode::TowardNegative),
    ("nearest_even", RoundingMode::NearestEven),
    ("nearest_away", RoundingMode::NearestAway),
];

const PROFILES: [(&str, TextProfile); 6] = [
    ("unicode_scalars", TextProfile::UnicodeScalars),
    ("nfc", TextProfile::Nfc),
    ("nfd", TextProfile::Nfd),
    ("nfkc", TextProfile::Nfkc),
    ("nfkd", TextProfile::Nfkd),
    ("binary_utf8", TextProfile::BinaryUtf8),
];

const KINDS: [(&str, CollectionKind); 4] = [
    ("sequence", CollectionKind::Sequence),
    ("set", CollectionKind::Set),
    ("bag", CollectionKind::Bag),
    ("ordered_set", CollectionKind::OrderedSet),
];

const WIDTHS: [(&str, IeeeWidth); 2] = [
    ("binary32", IeeeWidth::Binary32),
    ("binary64", IeeeWidth::Binary64),
];

fn spelled<T: Copy + PartialEq>(table: &[(&'static str, T)], value: T) -> Result<Json, Bad> {
    table
        .iter()
        .find(|(_, held)| *held == value)
        .map(|(name, _)| text(*name))
        .map_or_else(|| malformed("an unlisted enumerator"), Ok)
}

fn unspelled<T: Copy>(table: &[(&'static str, T)], json: &Json) -> Result<T, Bad> {
    let name = json.text()?;
    table
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, value)| *value)
        .map_or_else(|| malformed(format!("`{name}`")), Ok)
}

fn interval(value: &IntegerInterval) -> Json {
    object(vec![
        ("lower", text(value.lower().to_string())),
        ("upper", text(value.upper().to_string())),
    ])
}

fn interval_from(json: &Json) -> Result<IntegerInterval, Bad> {
    let parts = json.members(&["lower", "upper"])?;
    IntegerInterval::new(integer(parts[0])?, integer(parts[1])?)
        .or_else(|_| malformed("an empty interval"))
}

fn value_type(joins: &impl TypeKeys, value: &ValueType) -> Result<Json, Bad> {
    let simple = |tag: &str| Ok(object(vec![("kind", text(tag))]));
    match value {
        ValueType::Boolean => simple("boolean"),
        ValueType::Integer => simple("integer"),
        ValueType::String => simple("string"),
        ValueType::Uuid => simple("uuid"),
        ValueType::Timestamp => simple("timestamp"),
        ValueType::Int(range) => {
            let Json::Object(mut members) = interval(range) else {
                return malformed("an interval");
            };
            members.insert(0, ("kind".to_owned(), text("int")));
            Ok(Json::Object(members))
        }
        ValueType::Rational(domain) => Ok(object(vec![
            ("kind", text("rational")),
            ("numerator", interval(domain.numerator())),
            ("denominator", interval(domain.denominator())),
        ])),
        ValueType::Decimal(decimal) => Ok(object(vec![
            ("kind", text("decimal")),
            ("lower", text(decimal.lower().to_string())),
            ("upper", text(decimal.upper().to_string())),
            ("min_scale", Json::Int(decimal.min_scale().into())),
            ("max_scale", Json::Int(decimal.max_scale().into())),
            ("rounding", spelled(&ROUNDINGS, decimal.rounding())?),
        ])),
        ValueType::Float(float) => Ok(object(vec![
            ("kind", text("float")),
            ("width", spelled(&WIDTHS, float.width())?),
            ("rounding", spelled(&ROUNDINGS, float.rounding())?),
        ])),
        ValueType::Quantity(unit) => Ok(object(vec![
            ("kind", text("quantity")),
            ("unit", unit_id(joins, unit, true)?),
        ])),
        ValueType::Text(shape) => Ok(object(vec![
            ("kind", text("text")),
            ("min", Json::Int(shape.min())),
            ("max", Json::Int(shape.max())),
            ("profile", spelled(&PROFILES, shape.profile())?),
        ])),
        ValueType::Enum(shape) => {
            let variants = shape
                .variants()
                .map(|variant| {
                    Ok(object(vec![
                        ("owner", joins.write_key(variant.owner())?),
                        ("case", text(variant.case())),
                    ]))
                })
                .collect::<Result<Vec<_>, Bad>>()?;
            Ok(object(vec![
                ("kind", text("enum")),
                ("ordered", Json::Bool(shape.is_ordered())),
                ("variants", Json::Array(variants)),
            ]))
        }
        ValueType::Option(element) => Ok(object(vec![
            ("kind", text("option")),
            ("element", value_type(joins, element)?),
        ])),
        ValueType::Composite(key) => Ok(object(vec![
            ("kind", text("composite")),
            ("node", joins.write_key(key)?),
        ])),
        ValueType::Collection(collection) => Ok(object(vec![
            ("kind", text("collection")),
            ("collection", spelled(&KINDS, collection.kind())?),
            ("element", value_type(joins, collection.element())?),
            (
                "bound",
                collection.bound().map_or(Json::Null, |bound| {
                    object(vec![
                        ("minimum", Json::Int(bound.minimum())),
                        ("maximum", Json::Int(bound.maximum())),
                    ])
                }),
            ),
        ])),
        ValueType::Reference(id) => Ok(object(vec![
            ("kind", text("reference")),
            ("effective", effective(id)),
        ])),
        ValueType::Population(maximum) => Ok(object(vec![
            ("kind", text("population")),
            ("maximum", maximum.map_or(Json::Null, Json::Int)),
        ])),
        ValueType::CarriedQuantity(carried) => Ok(object(vec![
            ("kind", text("carried_quantity")),
            ("unit", unit_id(joins, carried.unit(), true)?),
            ("carrier", carrier(carried.carrier())?),
        ])),
        ValueType::BoundedQuantity(bounded) => Ok(object(vec![
            ("kind", text("bounded_quantity")),
            ("unit", unit_id(joins, bounded.unit(), true)?),
            ("min", bound_quantity(joins, bounded.min())?),
            ("max", bound_quantity(joins, bounded.max())?),
        ])),
        ValueType::Map(map) => Ok(object(vec![
            ("kind", text("map")),
            ("values", value_type(joins, map.values())?),
            ("bound", cardinality(map.bound())),
        ])),
    }
}

fn cardinality(bound: Option<CardinalityBound>) -> Json {
    bound.map_or(Json::Null, |bound| {
        object(vec![
            ("minimum", Json::Int(bound.minimum())),
            ("maximum", Json::Int(bound.maximum())),
        ])
    })
}

fn cardinality_from(json: &Json) -> Result<Option<CardinalityBound>, Bad> {
    match json {
        Json::Null => Ok(None),
        other => {
            let parts = other.members(&["minimum", "maximum"])?;
            CardinalityBound::new(parts[0].int()?, parts[1].int()?)
                .map(Some)
                .or_else(|_| malformed("an empty bound"))
        }
    }
}

fn carrier(value: &QuantityCarrier) -> Result<Json, Bad> {
    Ok(match value {
        QuantityCarrier::Decimal(decimal) => object(vec![
            ("kind", text("decimal")),
            ("lower", text(decimal.lower().to_string())),
            ("upper", text(decimal.upper().to_string())),
            ("min_scale", Json::Int(decimal.min_scale().into())),
            ("max_scale", Json::Int(decimal.max_scale().into())),
            ("rounding", spelled(&ROUNDINGS, decimal.rounding())?),
        ]),
        QuantityCarrier::Integer { domain, rounding } => object(vec![
            ("kind", text("integer")),
            ("lower", text(domain.lower().to_string())),
            ("upper", text(domain.upper().to_string())),
            ("rounding", spelled(&ROUNDINGS, *rounding)?),
        ]),
        QuantityCarrier::Rational(domain) => object(vec![
            ("kind", text("rational")),
            ("numerator", interval(domain.numerator())),
            ("denominator", interval(domain.denominator())),
        ]),
    })
}

const CARRIERS: &[(&str, &[&str])] = &[
    (
        "decimal",
        &["lower", "upper", "min_scale", "max_scale", "rounding"],
    ),
    ("integer", &["lower", "upper", "rounding"]),
    ("rational", &["numerator", "denominator"]),
];

#[ix_trace_rs::string_edge]

fn carrier_from(json: &Json) -> Result<QuantityCarrier, Bad> {
    let (tag, m) = json.tagged(CARRIERS)?;
    let scale = |json: &Json| -> Result<u64, Bad> {
        u32::try_from(json.int()?)
            .map(u64::from)
            .or_else(|_| malformed("a decimal scale"))
    };
    Ok(match tag {
        "decimal" => QuantityCarrier::Decimal(
            DecimalType::new(
                integer(m[0])?,
                integer(m[1])?,
                scale(m[2])?,
                scale(m[3])?,
                unspelled(&ROUNDINGS, m[4])?,
            )
            .or_else(|_| malformed("a decimal carrier"))?,
        ),
        "integer" => QuantityCarrier::Integer {
            domain: IntegerInterval::new(integer(m[0])?, integer(m[1])?)
                .or_else(|_| malformed("an empty interval"))?,
            rounding: unspelled(&ROUNDINGS, m[2])?,
        },
        _ => QuantityCarrier::Rational(
            RationalDomain::new(interval_from(m[0])?, interval_from(m[1])?)
                .or_else(|_| malformed("a rational carrier"))?,
        ),
    })
}

fn bound_quantity(joins: &impl TypeKeys, value: Option<&Quantity>) -> Result<Json, Bad> {
    let Some(quantity) = value else {
        return Ok(Json::Null);
    };
    Ok(object(vec![
        (
            "magnitude",
            rational(&CheckedRational {
                numerator: quantity.magnitude().numerator().to_string().into(),
                denominator: quantity.magnitude().denominator().to_string().into(),
            })?,
        ),
        ("unit", unit_id(joins, quantity.unit(), true)?),
    ]))
}

fn bound_quantity_from(joins: &impl TypeKeys, json: &Json) -> Result<Option<Quantity>, Bad> {
    if matches!(json, Json::Null) {
        return Ok(None);
    }
    let parts = json.members(&["magnitude", "unit"])?;
    let magnitude = rational_from(parts[0])?;
    let magnitude = Rational::new(
        integer(&text(&*magnitude.numerator))?,
        integer(&text(&*magnitude.denominator))?,
    )
    .or_else(|_| malformed("a zero denominator"))?;
    Ok(Some(Quantity::new(
        magnitude,
        unit_from(joins, parts[1], true)?,
    )))
}

const VALUE_TYPES: &[(&str, &[&str])] = &[
    ("boolean", &[]),
    ("integer", &[]),
    ("string", &[]),
    ("uuid", &[]),
    ("timestamp", &[]),
    ("int", &["lower", "upper"]),
    ("rational", &["numerator", "denominator"]),
    (
        "decimal",
        &["lower", "upper", "min_scale", "max_scale", "rounding"],
    ),
    ("float", &["width", "rounding"]),
    ("quantity", &["unit"]),
    ("text", &["min", "max", "profile"]),
    ("enum", &["ordered", "variants"]),
    ("option", &["element"]),
    ("composite", &["node"]),
    ("collection", &["collection", "element", "bound"]),
    ("reference", &["effective"]),
    ("population", &["maximum"]),
    ("carried_quantity", &["unit", "carrier"]),
    ("bounded_quantity", &["unit", "min", "max"]),
    ("map", &["values", "bound"]),
];

#[ix_trace_rs::string_edge]

fn value_type_from(joins: &impl TypeKeys, json: &Json) -> Result<ValueType, Bad> {
    let (tag, m) = json.tagged(VALUE_TYPES)?;
    let width = |json: &Json| -> Result<u32, Bad> {
        u32::try_from(json.int()?).or_else(|_| malformed("a scale"))
    };
    Ok(match tag {
        "boolean" => ValueType::Boolean,
        "integer" => ValueType::Integer,
        "string" => ValueType::String,
        "uuid" => ValueType::Uuid,
        "timestamp" => ValueType::Timestamp,
        "int" => ValueType::Int(
            IntegerInterval::new(integer(m[0])?, integer(m[1])?)
                .or_else(|_| malformed("an empty interval"))?,
        ),
        "rational" => ValueType::Rational(
            RationalDomain::new(interval_from(m[0])?, interval_from(m[1])?)
                .or_else(|_| malformed("a rational denominator below one"))?,
        ),
        "decimal" => ValueType::Decimal(
            DecimalType::new(
                integer(m[0])?,
                integer(m[1])?,
                width(m[2])?.into(),
                width(m[3])?.into(),
                unspelled(&ROUNDINGS, m[4])?,
            )
            .or_else(|_| malformed("a decimal type"))?,
        ),
        "float" => ValueType::Float(FloatType::new(
            unspelled(&WIDTHS, m[0])?,
            unspelled(&ROUNDINGS, m[1])?,
        )),
        "quantity" => ValueType::Quantity(unit_from(joins, m[0], true)?),
        "text" => ValueType::Text(
            TextType::new(m[0].int()?, m[1].int()?, unspelled(&PROFILES, m[2])?)
                .or_else(|_| malformed("text bounds"))?,
        ),
        "enum" => {
            let variants = m[1]
                .array()?
                .iter()
                .map(|variant| {
                    let parts = variant.members(&["owner", "case"])?;
                    VariantId::new(joins.read_key(parts[0])?, parts[1].text()?)
                        .or_else(|_| malformed("an empty case"))
                })
                .collect::<Result<Vec<_>, Bad>>()?;
            ValueType::Enum(EnumShape::new(m[0].boolean()?, variants))
        }
        "option" => ValueType::Option(value_type_from(joins, m[0])?.into()),
        "composite" => ValueType::Composite(joins.read_key(m[0])?),
        "collection" => {
            let bound = match m[2] {
                Json::Null => None,
                other => {
                    let parts = other.members(&["minimum", "maximum"])?;
                    Some(
                        CardinalityBound::new(parts[0].int()?, parts[1].int()?)
                            .or_else(|_| malformed("an empty bound"))?,
                    )
                }
            };
            ValueType::collection(CollectionType::new(
                unspelled(&KINDS, m[0])?,
                value_type_from(joins, m[1])?,
                bound,
            ))
        }
        "reference" => ValueType::Reference(effective_from(m[0])?),
        "carried_quantity" => ValueType::CarriedQuantity(
            CarriedQuantityType::new(unit_from(joins, m[0], true)?, carrier_from(m[1])?).into(),
        ),
        "bounded_quantity" => {
            let unit = unit_from(joins, m[0], true)?;
            ValueType::BoundedQuantity(
                BoundedQuantityType::new(
                    unit,
                    bound_quantity_from(joins, m[1])?,
                    bound_quantity_from(joins, m[2])?,
                )
                .or_else(|_| malformed("a bound in another unit"))?
                .into(),
            )
        }
        "map" => ValueType::Map(
            MapType::new(value_type_from(joins, m[0])?, cardinality_from(m[1])?).into(),
        ),
        _ => ValueType::Population(match m[0] {
            Json::Null => None,
            other => Some(other.int()?),
        }),
    })
}

const OPERAND_ROLES: [(&str, EqualityOperandTypeRole); 2] = [
    ("source", EqualityOperandTypeRole::Source),
    (
        "conversion_target",
        EqualityOperandTypeRole::ConversionTarget,
    ),
];

const UNIT_ROLES: [(&str, EqualityUnitRole); 2] = [
    ("source", EqualityUnitRole::Source),
    ("conversion_target", EqualityUnitRole::ConversionTarget),
];

const DIMENSION_ROLES: [(&str, EqualityDimensionRole); 4] = [
    ("unit_dimension", EqualityDimensionRole::UnitDimension),
    (
        "conversion_dimension",
        EqualityDimensionRole::ConversionDimension,
    ),
    ("compound_base", EqualityDimensionRole::CompoundBase),
    ("base_dimension", EqualityDimensionRole::BaseDimension),
];

fn fact(joins: &Joins, value: &EqualityFactWitness) -> Result<Json, Bad> {
    use EqualityFactWitness as F;
    Ok(match value {
        F::OperandType { position, role } => object(vec![
            ("kind", text("operand_type")),
            ("position", count(*position)),
            ("role", spelled(&OPERAND_ROLES, *role)?),
        ]),
        F::Unit {
            unit_id: unit,
            quantity_type,
            operand_position,
            role,
        } => object(vec![
            ("kind", text("unit")),
            ("unit_id", unit_id(joins, unit, true)?),
            ("quantity_type", of(quantity_type)?),
            ("operand_position", count(*operand_position)),
            ("role", spelled(&UNIT_ROLES, *role)?),
        ]),
        F::DimensionDeclaration {
            unit_id: unit,
            operand_position,
            role,
            ordinal,
        } => object(vec![
            ("kind", text("dimension_declaration")),
            ("unit_id", unit_id(joins, unit, true)?),
            ("operand_position", count(*operand_position)),
            ("role", spelled(&DIMENSION_ROLES, *role)?),
            ("ordinal", count(*ordinal)),
        ]),
        F::DimensionTerm {
            unit_id: unit,
            operand_position,
            ordinal,
            base,
            base_node,
            exponent,
        } => object(vec![
            ("kind", text("dimension_term")),
            ("unit_id", unit_id(joins, unit, true)?),
            ("operand_position", count(*operand_position)),
            ("ordinal", count(*ordinal)),
            ("base", joins.node_id(base)?),
            ("base_node", of(base_node)?),
            ("exponent", text(&**exponent)),
        ]),
        F::ConversionEdge {
            unit_id: unit,
            operand_position,
            ordinal,
            target,
            target_node,
            scale,
            offset,
        } => object(vec![
            ("kind", text("conversion_edge")),
            ("unit_id", unit_id(joins, unit, true)?),
            ("operand_position", count(*operand_position)),
            ("ordinal", count(*ordinal)),
            ("target", joins.node_id(target)?),
            ("target_node", of(target_node)?),
            ("scale", rational(scale)?),
            ("offset", rational(offset)?),
        ]),
        F::TextPlanType { position, role } => object(vec![
            ("kind", text("text_plan_type")),
            ("position", count(*position)),
            ("role", spelled(&OPERAND_ROLES, *role)?),
        ]),
        F::EnumMember {
            source_declaration,
            source_declaration_node,
            declaration,
            declaration_node,
            member,
            member_node,
            case,
            ordinal,
            ordered,
        } => object(vec![
            ("kind", text("enum_member")),
            ("source_declaration", joins.node_id(source_declaration)?),
            ("source_declaration_node", of(source_declaration_node)?),
            ("declaration", joins.node_id(declaration)?),
            ("declaration_node", of(declaration_node)?),
            ("member", joins.node_id(member)?),
            ("member_node", of(member_node)?),
            ("case", text(&**case)),
            ("ordinal", count(*ordinal)),
            ("ordered", Json::Bool(*ordered)),
        ]),
    })
}

const FACTS: &[(&str, &[&str])] = &[
    ("operand_type", &["position", "role"]),
    (
        "unit",
        &["unit_id", "quantity_type", "operand_position", "role"],
    ),
    (
        "dimension_declaration",
        &["unit_id", "operand_position", "role", "ordinal"],
    ),
    (
        "dimension_term",
        &[
            "unit_id",
            "operand_position",
            "ordinal",
            "base",
            "base_node",
            "exponent",
        ],
    ),
    (
        "conversion_edge",
        &[
            "unit_id",
            "operand_position",
            "ordinal",
            "target",
            "target_node",
            "scale",
            "offset",
        ],
    ),
    ("text_plan_type", &["position", "role"]),
    (
        "enum_member",
        &[
            "source_declaration",
            "source_declaration_node",
            "declaration",
            "declaration_node",
            "member",
            "member_node",
            "case",
            "ordinal",
            "ordered",
        ],
    ),
];

#[ix_trace_rs::string_edge]

fn fact_from(joins: &Joins, json: &Json) -> Result<EqualityFactWitness, Bad> {
    use EqualityFactWitness as F;
    let (tag, m) = json.tagged(FACTS)?;
    Ok(match tag {
        "operand_type" => F::OperandType {
            position: m[0].index()?,
            role: unspelled(&OPERAND_ROLES, m[1])?,
        },
        "unit" => F::Unit {
            unit_id: unit_from(joins, m[0], true)?,
            quantity_type: joins.held(m[1])?.0,
            operand_position: m[2].index()?,
            role: unspelled(&UNIT_ROLES, m[3])?,
        },
        "dimension_declaration" => F::DimensionDeclaration {
            unit_id: unit_from(joins, m[0], true)?,
            operand_position: m[1].index()?,
            role: unspelled(&DIMENSION_ROLES, m[2])?,
            ordinal: m[3].index()?,
        },
        "dimension_term" => F::DimensionTerm {
            unit_id: unit_from(joins, m[0], true)?,
            operand_position: m[1].index()?,
            ordinal: m[2].index()?,
            base: joins.held(m[3])?.1,
            base_node: joins.held(m[4])?.0,
            exponent: m[5].text()?.into(),
        },
        "conversion_edge" => F::ConversionEdge {
            unit_id: unit_from(joins, m[0], true)?,
            operand_position: m[1].index()?,
            ordinal: m[2].index()?,
            target: joins.held(m[3])?.1,
            target_node: joins.held(m[4])?.0,
            scale: rational_from(m[5])?,
            offset: rational_from(m[6])?,
        },
        "text_plan_type" => F::TextPlanType {
            position: m[0].index()?,
            role: unspelled(&OPERAND_ROLES, m[1])?,
        },
        _ => F::EnumMember {
            source_declaration: joins.held(m[0])?.1,
            source_declaration_node: joins.held(m[1])?.0,
            declaration: joins.held(m[2])?.1,
            declaration_node: joins.held(m[3])?.0,
            member: joins.held(m[4])?.1,
            member_node: joins.held(m[5])?.0,
            case: m[6].text()?.into(),
            ordinal: m[7].index()?,
            ordered: m[8].boolean()?,
        },
    })
}

fn owner_from(json: &Json) -> Result<Option<CheckedNodeOwner>, Bad> {
    match json {
        Json::Null => Ok(None),
        other => {
            other.tagged(&[
                ("model_root", &["identity"]),
                ("source", &["authority", "identity"]),
                ("model", &["identity", "node"]),
            ])?;
            to(other, "an owner").map(Some)
        }
    }
}

// ---- rows -----------------------------------------------------------------

const APPLICATION_MEMBERS: [&str; 5] = [
    "application",
    "occurrence",
    "source_map_entry",
    "node_key",
    "location",
];
const FACT_MEMBERS: [&str; 9] = [
    "application",
    "checked_application",
    "occurrence",
    "source_map_entry",
    "structural_key",
    "structural_node",
    "owner",
    "value_type",
    "fact",
];
const UNIT_MEMBERS: [&str; 5] = [
    "structural_node",
    "source_authority",
    "source_identity",
    "qualified_declaration",
    "unit_id",
];

fn source_map_entry(value: &CheckedSourceMapEntry) -> Result<Json, Bad> {
    if value.regions.is_empty() {
        return malformed("a source map entry without regions");
    }
    of(value)
}

fn source_map_entry_from(json: &Json) -> Result<CheckedSourceMapEntry, Bad> {
    let entry: CheckedSourceMapEntry = to(json, "a source map entry")?;
    if entry.regions.is_empty() {
        return malformed("a source map entry without regions");
    }
    Ok(entry)
}

impl ReadEqualityRows {
    /// The rows of the in-process unit, for a package it was emitted as.
    pub fn of_rows(
        package: &CheckedPackageV2,
        applications: Vec<SelectedEqualityApplicationRow>,
        facts: Vec<SelectedEqualityFactRow>,
        source_units: Vec<SelectedSourceUnitRow>,
    ) -> Self {
        Self {
            package: package.name().to_owned(),
            applications,
            facts,
            source_units,
        }
    }

    /// The package the rows were produced for.
    pub fn package_name(&self) -> &str {
        &self.package
    }

    /// The equality application rows, in file order.
    pub fn applications(&self) -> &[SelectedEqualityApplicationRow] {
        &self.applications
    }

    /// The equality fact rows, in file order.
    pub fn facts(&self) -> &[SelectedEqualityFactRow] {
        &self.facts
    }

    /// The selected source-unit rows, in file order.
    pub fn source_units(&self) -> &[SelectedSourceUnitRow] {
        &self.source_units
    }

    /// The admitted equality decision of `application`, read through IR's
    /// `admitted_equality` from these rows (FR-533 "Equality slots").
    ///
    /// # Errors
    ///
    /// [`EqualityWitnessRefusal::Foreign`] for rows of another package,
    /// [`EqualityWitnessRefusal::Unknown`] for an application the rows do not
    /// join or IR does not admit.
    pub(crate) fn decision(
        &self,
        package: &CheckedPackageV2,
        application: &CheckedNodeId,
        budget: &mut quire_contract_model::NativeJoinBudget,
    ) -> Result<quire_semantic_value::declaration::AdmittedEqualityDecision, EqualityDecisionFailure>
    {
        if self.package != package.name() {
            return Err(EqualityDecisionFailure::Witness(
                EqualityWitnessRefusal::Foreign,
            ));
        }
        let row = self
            .applications
            .iter()
            .find(|row| &row.application == application)
            .ok_or(EqualityDecisionFailure::Witness(
                EqualityWitnessRefusal::Unknown,
            ))?;
        package
            .admitted_equality(
                application,
                &row.occurrence,
                &self.applications,
                &self.facts,
                &self.source_units,
                budget,
            )
            .map_err(|error: AdmittedEqualityReadError| {
                if matches!(
                    error.code,
                    quire_contract_model::AdmittedEqualityReadErrorCode::WorkLimit
                        | quire_contract_model::AdmittedEqualityReadErrorCode::StorageLimit
                ) {
                    EqualityDecisionFailure::Exhausted(Box::new(error))
                } else {
                    EqualityDecisionFailure::Witness(EqualityWitnessRefusal::Unknown)
                }
            })
    }

    /// Whether these rows were produced for `package` (FR-531-AC-16).
    pub fn is_foreign_to(&self, package: &CheckedPackageV2) -> bool {
        self.package != package.name()
    }

    /// The equality file of these rows (FR-531 "The equality file"): one JSON
    /// object, pretty-printed with two-space indent, members in the documented
    /// order, UTF-8, ending in one newline, byte-identical for equal inputs.
    ///
    /// # Errors
    ///
    /// [`EqualityFileRefusal::Malformed`] for a key the package does not name,
    /// a compound unit in a source unit row, or a value type outside the
    /// file's vocabulary.
    pub fn to_bytes(&self, package: &CheckedPackageV2) -> Result<Vec<u8>, EqualityFileRefusal> {
        let joins = Joins::of_package(package).ok_or_else(|| EqualityFileRefusal::Malformed {
            detail: "the package's node keys".to_owned(),
        })?;
        self.encode(&joins)
    }

    fn encode(&self, joins: &Joins) -> Result<Vec<u8>, EqualityFileRefusal> {
        let refuse = |bad: Bad| match bad {
            Bad::Malformed(detail) => EqualityFileRefusal::Malformed { detail },
            Bad::Unjoined => EqualityFileRefusal::Malformed {
                detail: "a node the package does not name".to_owned(),
            },
        };
        let applications = self
            .applications
            .iter()
            .map(|row| {
                Ok(object(vec![
                    ("application", of(&row.application)?),
                    ("occurrence", of(&row.occurrence)?),
                    ("source_map_entry", source_map_entry(&row.source_map_entry)?),
                    ("node_key", joins.node_id(&row.node_key)?),
                    ("location", location(joins, &row.location)?),
                ]))
            })
            .collect::<Result<Vec<_>, Bad>>()
            .map_err(refuse)?;
        let facts = self
            .facts
            .iter()
            .map(|row| {
                Ok(object(vec![
                    ("application", joins.node_id(&row.application)?),
                    ("checked_application", of(&row.checked_application)?),
                    ("occurrence", of(&row.occurrence)?),
                    ("source_map_entry", source_map_entry(&row.source_map_entry)?),
                    ("structural_key", joins.node_id(&row.structural_key)?),
                    ("structural_node", of(&row.structural_node)?),
                    ("owner", of(&row.owner)?),
                    (
                        "value_type",
                        row.value_type
                            .as_ref()
                            .map_or(Ok(Json::Null), |ty| value_type(joins, ty))?,
                    ),
                    ("fact", fact(joins, &row.fact)?),
                ]))
            })
            .collect::<Result<Vec<_>, Bad>>()
            .map_err(refuse)?;
        let source_units = self
            .source_units
            .iter()
            .map(|row| {
                Ok(object(vec![
                    ("structural_node", of(&row.structural_node)?),
                    ("source_authority", text(&*row.source_authority)),
                    ("source_identity", text(&*row.source_identity)),
                    (
                        "qualified_declaration",
                        Json::Array(
                            row.qualified_declaration
                                .iter()
                                .map(|segment| text(&**segment))
                                .collect(),
                        ),
                    ),
                    ("unit_id", unit_id(joins, &row.unit_id, false)?),
                ]))
            })
            .collect::<Result<Vec<_>, Bad>>()
            .map_err(refuse)?;
        let file = object(vec![
            ("format", text(EQUALITY_FORMAT)),
            ("applications", Json::Array(applications)),
            ("facts", Json::Array(facts)),
            ("source_units", Json::Array(source_units)),
        ]);
        let mut bytes =
            serde_json::to_vec_pretty(&file).map_err(|e| EqualityFileRefusal::Malformed {
                detail: e.to_string(),
            })?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    /// Read `equality_json`, joining its rows to `package`.
    ///
    /// The rows grant no authority beyond what IR's `admitted_equality`
    /// validates when a decision is read. A refusal returns no rows.
    ///
    /// # Errors
    ///
    /// [`EqualityFileRefusal`].
    pub fn read(
        equality_json: &[u8],
        package: &CheckedPackageV2,
    ) -> Result<Self, EqualityFileRefusal> {
        let joins = Joins::of_package(package).ok_or_else(|| EqualityFileRefusal::Malformed {
            detail: "the package's node keys".to_owned(),
        })?;
        Self::decode(equality_json, package.name(), &joins)
    }

    #[ix_trace_rs::string_edge]

    fn decode(
        equality_json: &[u8],
        package: &str,
        joins: &Joins,
    ) -> Result<Self, EqualityFileRefusal> {
        let file: Json =
            serde_json::from_slice(equality_json).map_err(|e| EqualityFileRefusal::Malformed {
                detail: e.to_string(),
            })?;
        let bad = |bad: Bad, member: &'static str, index: usize| match bad {
            Bad::Malformed(detail) => EqualityFileRefusal::Malformed {
                detail: format!("`{member}` row {index}: {detail}"),
            },
            Bad::Unjoined => EqualityFileRefusal::Unjoined { member, index },
        };
        let top = file
            .members(&["format", "applications", "facts", "source_units"])
            .map_err(|b| bad(b, "file", 0))?;
        let format = top[0].text().map_err(|b| bad(b, "format", 0))?;
        if format != EQUALITY_FORMAT {
            return Err(EqualityFileRefusal::Format(format.to_owned()));
        }
        let rows = |json: &Json, member: &'static str| -> Result<Vec<Json>, EqualityFileRefusal> {
            json.array()
                .map(<[Json]>::to_vec)
                .map_err(|b| bad(b, member, 0))
        };
        let mut applications = Vec::new();
        for (index, row) in rows(top[1], "applications")?.iter().enumerate() {
            let parse = || -> Result<SelectedEqualityApplicationRow, Bad> {
                let m = row.members(&APPLICATION_MEMBERS)?;
                let (application, _) = joins.held(m[0])?;
                if !joins.equality.contains(&application) {
                    return Err(Bad::Unjoined);
                }
                Ok(SelectedEqualityApplicationRow::new(
                    application,
                    to(m[1], "an occurrence")?,
                    source_map_entry_from(m[2])?,
                    joins.held(m[3])?.1,
                    location_from(joins, m[4])?,
                ))
            };
            applications.push(parse().map_err(|b| bad(b, "applications", index))?);
        }
        let mut facts = Vec::new();
        for (index, row) in rows(top[2], "facts")?.iter().enumerate() {
            let parse = || -> Result<SelectedEqualityFactRow, Bad> {
                let m = row.members(&FACT_MEMBERS)?;
                let (checked_application, _) = joins.held(m[1])?;
                if !joins.equality.contains(&checked_application) {
                    return Err(Bad::Unjoined);
                }
                Ok(SelectedEqualityFactRow {
                    application: joins.held(m[0])?.1,
                    checked_application,
                    occurrence: to::<CheckedOccurrence>(m[2], "an occurrence")?,
                    source_map_entry: source_map_entry_from(m[3])?,
                    structural_key: joins.held(m[4])?.1,
                    structural_node: joins.held(m[5])?.0,
                    owner: owner_from(m[6])?,
                    value_type: match m[7] {
                        Json::Null => None,
                        other => Some(value_type_from(joins, other)?),
                    },
                    fact: fact_from(joins, m[8])?,
                })
            };
            facts.push(parse().map_err(|b| bad(b, "facts", index))?);
        }
        let mut source_units = Vec::new();
        for (index, row) in rows(top[3], "source_units")?.iter().enumerate() {
            let parse = || -> Result<SelectedSourceUnitRow, Bad> {
                let m = row.members(&UNIT_MEMBERS)?;
                let segments = m[3]
                    .array()?
                    .iter()
                    .map(|segment| segment.text().map(Box::<str>::from))
                    .collect::<Result<Vec<_>, Bad>>()?;
                Ok(SelectedSourceUnitRow::new(
                    joins.held(m[0])?.0,
                    m[1].text()?,
                    m[2].text()?,
                    segments,
                    unit_from(joins, m[4], false)?,
                ))
            };
            source_units.push(parse().map_err(|b| bad(b, "source_units", index))?);
        }
        Ok(Self {
            package: package.to_owned(),
            applications,
            facts,
            source_units,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::{json, Value};

    fn id(name: &str) -> CheckedNodeId {
        serde_json::from_value(json!({ "package": "p", "path": [name], "ordinal": 0 })).unwrap()
    }

    /// Joins over the nodes `E` (an equality application), `U` and `V`.
    fn joins() -> (Joins, BTreeMap<&'static str, NodeKey>) {
        let ids = ["E", "U", "V"].map(id);
        let graph = qsl_semantics::check::wire_id::WireGraph::new(ids.iter()).unwrap();
        let mut joins = Joins {
            ids: BTreeMap::new(),
            keys: BTreeMap::new(),
            equality: BTreeSet::from([id("E")]),
        };
        let mut named = BTreeMap::new();
        for (name, node) in [("E", id("E")), ("U", id("U")), ("V", id("V"))] {
            let key = graph.key(&node).unwrap();
            joins.ids.insert(key.clone(), node.clone());
            joins.keys.insert(node, key.clone());
            named.insert(name, key);
        }
        (joins, named)
    }

    fn entry() -> CheckedSourceMapEntry {
        serde_json::from_value(json!({
            "node_id": { "package": "p", "path": ["E"], "ordinal": 0 },
            "role": "expression", "ordinal": 0,
            "regions": [{ "source": { "authority": "a", "identity": "s" }, "start": 4, "end": 9 }]
        }))
        .unwrap()
    }

    fn occurrence() -> CheckedOccurrence {
        serde_json::from_value(json!({ "role": "expression", "ordinal": 0 })).unwrap()
    }

    fn fact_row(
        keys: &BTreeMap<&str, NodeKey>,
        value_type: Option<ValueType>,
        fact: EqualityFactWitness,
    ) -> SelectedEqualityFactRow {
        SelectedEqualityFactRow {
            application: keys["E"].clone(),
            checked_application: id("E"),
            occurrence: occurrence(),
            source_map_entry: entry(),
            structural_key: keys["E"].clone(),
            structural_node: id("E"),
            owner: Some(CheckedNodeOwner::Source {
                authority: "a".into(),
                identity: "s".into(),
            }),
            value_type,
            fact,
        }
    }

    fn vector_rows(keys: &BTreeMap<&str, NodeKey>) -> ReadEqualityRows {
        ReadEqualityRows {
            package: "p".to_owned(),
            applications: vec![SelectedEqualityApplicationRow::new(
                id("E"),
                occurrence(),
                entry(),
                keys["E"].clone(),
                Location::new(keys["E"].clone(), Origin::new("expression".into(), 0)),
            )],
            facts: vec![fact_row(
                keys,
                Some(ValueType::String),
                EqualityFactWitness::OperandType {
                    position: 0,
                    role: EqualityOperandTypeRole::Source,
                },
            )],
            source_units: vec![SelectedSourceUnitRow::new(
                id("U"),
                "a",
                "s",
                vec!["U".into()],
                UnitId::declared(keys["U"].clone()),
            )],
        }
    }

    /// The vector FR-531 "The equality file" states, read from the spec.
    fn vector() -> Vec<u8> {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/functional");
        let path = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("FR-531-"))
            })
            .unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        let start = text
            .find("```\n{\n  \"format\": \"quire.checked-handoff.equality/1\"")
            .unwrap()
            + 4;
        let end = start + text[start..].find("\n```").unwrap() + 1;
        text.as_bytes()[start..end].to_vec()
    }

    /// FR-531-AC-17: the encoding of the vector's rows equals its bytes and
    /// reading them returns the rows.
    #[trace("FR-531-AC-17", "TC-1331")]
    #[test]
    fn the_vector_rows_encode_to_the_vector_bytes_and_read_back() {
        let (joins, keys) = joins();
        let rows = vector_rows(&keys);
        let bytes = rows.encode(&joins).unwrap();
        assert_eq!(
            String::from_utf8(bytes.clone()).unwrap(),
            String::from_utf8(vector()).unwrap()
        );
        assert_eq!(ReadEqualityRows::decode(&bytes, "p", &joins).unwrap(), rows);
    }

    fn every_value_type(keys: &BTreeMap<&str, NodeKey>) -> Vec<ValueType> {
        let int = |lo: i64, hi: i64| IntegerInterval::new(lo.into(), hi.into()).unwrap();
        let unit = UnitId::compound(vec![(keys["U"].clone(), Integer::from(-2_i64))]);
        let element = ValueType::Option(ValueType::String.into());
        vec![
            ValueType::Boolean,
            ValueType::Integer,
            ValueType::String,
            ValueType::Uuid,
            ValueType::Timestamp,
            ValueType::Int(int(-5, 7)),
            ValueType::Rational(RationalDomain::new(int(-3, 3), int(1, 9)).unwrap()),
            ValueType::Decimal(
                DecimalType::new(
                    Integer::from(0_i64),
                    Integer::from(99_i64),
                    1,
                    4,
                    RoundingMode::NearestEven,
                )
                .unwrap(),
            ),
            ValueType::Float(FloatType::new(
                IeeeWidth::Binary32,
                RoundingMode::TowardZero,
            )),
            ValueType::Quantity(unit),
            ValueType::Quantity(UnitId::declared(keys["U"].clone())),
            ValueType::Text(TextType::new(1, 8, TextProfile::Nfkc).unwrap()),
            ValueType::Enum(EnumShape::new(
                true,
                [
                    VariantId::new(keys["V"].clone(), "on").unwrap(),
                    VariantId::new(keys["V"].clone(), "off").unwrap(),
                ],
            )),
            element.clone(),
            ValueType::Composite(keys["V"].clone()),
            ValueType::collection(CollectionType::new(
                CollectionKind::Bag,
                element,
                CardinalityBound::new(1, 4).ok(),
            )),
            ValueType::collection(CollectionType::new(
                CollectionKind::Set,
                ValueType::Integer,
                None,
            )),
            ValueType::Reference(EffectiveId::new(
                Some(EffectiveId::new(
                    None,
                    OriginalDeclarationKey::new("m", "root"),
                )),
                OriginalDeclarationKey::new("m", "member"),
            )),
            ValueType::Population(Some(3)),
            ValueType::Population(None),
        ]
    }

    #[trace("TC-1331", "FR-531-AC-19")]
    #[test]
    fn original_value_types_round_trip_without_a_graph_key_substitution() {
        let (_, mut keys) = joins();
        keys.insert(
            "U",
            NodeKey::new(
                "original-owner",
                vec![quire_exact::PathSegment::Text("unit".into())],
                17,
            )
            .unwrap(),
        );
        keys.insert(
            "V",
            NodeKey::new(
                "original-owner",
                vec![quire_exact::PathSegment::Text("record".into())],
                23,
            )
            .unwrap(),
        );
        let unit = UnitId::declared(keys["U"].clone());
        let mut types = every_value_type(&keys);
        types.extend([
            ValueType::CarriedQuantity(
                CarriedQuantityType::new(
                    unit.clone(),
                    QuantityCarrier::Decimal(
                        DecimalType::new(0_i64.into(), 20_i64.into(), 0, 2, RoundingMode::Exact)
                            .unwrap(),
                    ),
                )
                .into(),
            ),
            ValueType::BoundedQuantity(
                BoundedQuantityType::new(
                    unit.clone(),
                    Some(Quantity::new(
                        Rational::new(0_i64.into(), 1_i64.into()).unwrap(),
                        unit.clone(),
                    )),
                    Some(Quantity::new(
                        Rational::new(5_i64.into(), 1_i64.into()).unwrap(),
                        unit,
                    )),
                )
                .unwrap()
                .into(),
            ),
            ValueType::Map(
                MapType::new(
                    ValueType::Composite(keys["V"].clone()),
                    Some(CardinalityBound::new(0, 2).unwrap()),
                )
                .into(),
            ),
        ]);
        for wanted in types {
            let wire = OriginalValueType::of(&wanted).expect("the Exact type encodes");
            let bytes = serde_json::to_vec(&wire).unwrap();
            let read: OriginalValueType = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(read.value().unwrap(), wanted);
            assert_eq!(serde_json::to_vec(&read).unwrap(), bytes);
        }
        let wire = serde_json::to_value(
            OriginalValueType::of(&ValueType::Composite(keys["V"].clone())).unwrap(),
        )
        .unwrap();
        assert_eq!(
            wire["node"],
            qsl_foundation::node_wire::node_key_to_json(&keys["V"]).unwrap()
        );
        let mut bad = wire;
        bad["node"]["extra"] = json!(true);
        let decoded: OriginalValueType = serde_json::from_value(bad).unwrap();
        assert!(decoded.value().is_err());
        let bad: OriginalValueType = serde_json::from_value(json!({"kind":"carried_quantity", "unit":{"kind":"declared", "node": qsl_foundation::node_wire::node_key_to_json(&keys["U"]).unwrap()}, "carrier":{"kind":"decimal","lower":"0","upper":"5","min_scale":"not-a-number","max_scale":4294967295_u64,"rounding":"exact"}})).unwrap();
        assert!(
            bad.value().is_err(),
            "a malformed scale must not clamp to an accepted type"
        );
    }

    #[trace("TC-1331", "FR-531-AC-19", "FR-531-AC-20")]
    #[test]
    fn original_identity_nesting_has_the_same_finite_write_and_read_boundary() {
        let mut key = NodeKey::new(
            "original",
            vec![quire_exact::PathSegment::Text("leaf".into())],
            0,
        )
        .unwrap();
        for ordinal in 1..32 {
            key = NodeKey::new(
                "original",
                vec![quire_exact::PathSegment::Node(key)],
                ordinal as u64,
            )
            .unwrap();
        }
        let encoded = OriginalNodeKey::of(&key).expect("the documented nesting bound encodes");
        assert_eq!(encoded.key().unwrap(), key);
        let value = OriginalValueType::of(&ValueType::Composite(key.clone())).unwrap();
        assert_eq!(value.value().unwrap(), ValueType::Composite(key.clone()));
        let over = NodeKey::new("original", vec![quire_exact::PathSegment::Node(key)], 0).unwrap();
        assert!(OriginalNodeKey::of(&over).is_err());
        assert!(OriginalValueType::of(&ValueType::Composite(over.clone())).is_err());
        let text = qsl_foundation::node_wire::node_key_to_text(&over).unwrap();
        let parsed: OriginalNodeKey = serde_json::from_str(&text).unwrap();
        assert!(parsed.key().is_err());
    }

    /// FR-531-AC-17: a row of every fact tag, every value-type tag, every
    /// owner kind and a compound unit id writes then reads back equal, and a
    /// second write is byte-identical.
    #[trace("FR-531-AC-17", "TC-1331")]
    #[test]
    fn every_fact_tag_value_type_owner_and_compound_unit_round_trips() {
        use EqualityFactWitness as F;
        let (joins, keys) = joins();
        let unit = UnitId::compound(vec![
            (keys["U"].clone(), Integer::from(2_i64)),
            (keys["V"].clone(), Integer::from(-1_i64)),
        ]);
        let rational = |n: &str| CheckedRational {
            numerator: n.into(),
            denominator: "3".into(),
        };
        let facts = vec![
            F::OperandType {
                position: 1,
                role: EqualityOperandTypeRole::ConversionTarget,
            },
            F::Unit {
                unit_id: unit.clone(),
                quantity_type: id("V"),
                operand_position: 0,
                role: EqualityUnitRole::Source,
            },
            F::DimensionDeclaration {
                unit_id: unit.clone(),
                operand_position: 0,
                role: EqualityDimensionRole::CompoundBase,
                ordinal: 2,
            },
            F::DimensionTerm {
                unit_id: unit.clone(),
                operand_position: 0,
                ordinal: 0,
                base: keys["U"].clone(),
                base_node: id("U"),
                exponent: "-2".into(),
            },
            F::ConversionEdge {
                unit_id: unit,
                operand_position: 1,
                ordinal: 0,
                target: keys["V"].clone(),
                target_node: id("V"),
                scale: rational("-5"),
                offset: rational("0"),
            },
            F::TextPlanType {
                position: 0,
                role: EqualityOperandTypeRole::Source,
            },
            F::EnumMember {
                source_declaration: keys["U"].clone(),
                source_declaration_node: id("U"),
                declaration: keys["V"].clone(),
                declaration_node: id("V"),
                member: keys["V"].clone(),
                member_node: id("V"),
                case: "on".into(),
                ordinal: 1,
                ordered: true,
            },
        ];
        let mut rows = vector_rows(&keys);
        rows.facts = every_value_type(&keys)
            .into_iter()
            .map(Some)
            .chain([None])
            .zip(facts.clone().into_iter().cycle())
            .map(|(ty, fact)| fact_row(&keys, ty, fact))
            .chain(facts.into_iter().map(|fact| fact_row(&keys, None, fact)))
            .collect();
        for (index, owner) in [
            None,
            Some(CheckedNodeOwner::ModelRoot {
                identity: "m".into(),
            }),
            Some(CheckedNodeOwner::Model {
                identity: "m".into(),
                node: "n".into(),
            }),
        ]
        .into_iter()
        .enumerate()
        {
            rows.facts[index].owner = owner;
        }
        let bytes = rows.encode(&joins).unwrap();
        assert_eq!(ReadEqualityRows::decode(&bytes, "p", &joins).unwrap(), rows);
        assert_eq!(rows.encode(&joins).unwrap(), bytes);
    }

    /// FR-531-AC-17: non-canonical rationals and a compound source unit are
    /// `Malformed`.
    #[trace("FR-531-AC-17", "TC-1331")]
    #[test]
    fn non_canonical_rationals_and_a_compound_source_unit_are_malformed() {
        use EqualityFactWitness as F;
        let (joins, keys) = joins();
        let edge = |scale: CheckedRational| {
            let mut rows = vector_rows(&keys);
            rows.facts = vec![fact_row(
                &keys,
                None,
                F::ConversionEdge {
                    unit_id: UnitId::declared(keys["U"].clone()),
                    operand_position: 0,
                    ordinal: 0,
                    target: keys["V"].clone(),
                    target_node: id("V"),
                    scale,
                    offset: CheckedRational {
                        numerator: "0".into(),
                        denominator: "1".into(),
                    },
                },
            )];
            rows.encode(&joins)
        };
        let good = CheckedRational {
            numerator: "1".into(),
            denominator: "2".into(),
        };
        let good_bytes = edge(good).unwrap();
        assert!(ReadEqualityRows::decode(&good_bytes, "p", &joins).is_ok());
        for (numerator, denominator) in [
            ("+1", "2"),
            ("01", "2"),
            ("-0", "2"),
            ("1", "0"),
            ("1", "-2"),
        ] {
            // The writer refuses what the reader refuses (FND-005) ...
            assert!(
                matches!(
                    edge(CheckedRational {
                        numerator: numerator.into(),
                        denominator: denominator.into(),
                    }),
                    Err(EqualityFileRefusal::Malformed { .. })
                ),
                "write {numerator}/{denominator}"
            );
            // ... and the reader refuses the spelling in a file.
            let text = String::from_utf8(good_bytes.clone())
                .unwrap()
                .replacen(
                    "\"numerator\": \"1\"",
                    &format!("\"numerator\": \"{numerator}\""),
                    1,
                )
                .replacen(
                    "\"denominator\": \"2\"",
                    &format!("\"denominator\": \"{denominator}\""),
                    1,
                );
            assert!(
                matches!(
                    ReadEqualityRows::decode(text.as_bytes(), "p", &joins),
                    Err(EqualityFileRefusal::Malformed { .. })
                ),
                "read {numerator}/{denominator}"
            );
        }
        let mut compound = vector_rows(&keys);
        compound.source_units[0].unit_id =
            UnitId::compound(vec![(keys["U"].clone(), Integer::from(1_i64))]);
        assert!(matches!(
            compound.encode(&joins),
            Err(EqualityFileRefusal::Malformed { .. })
        ));
        let mut text: Value = serde_json::from_slice(&vector()).unwrap();
        text["source_units"][0]["unit_id"] = json!({ "kind": "compound", "terms": [] });
        let bytes = serde_json::to_vec(&text).unwrap();
        assert!(matches!(
            ReadEqualityRows::decode(&bytes, "p", &joins),
            Err(EqualityFileRefusal::Malformed { .. })
        ));
    }

    /// FR-517-AC-22: a row that names a node the package does not hold is
    /// refused by the writer's encoding, writing nothing.
    #[trace("FR-517-AC-22", "TC-1312")]
    #[test]
    fn a_row_naming_a_node_the_package_does_not_hold_is_not_encoded() {
        let (joins, keys) = joins();
        let mut rows = vector_rows(&keys);
        let (partial, _) = {
            let mut partial = Joins {
                ids: joins.ids.clone(),
                keys: joins.keys.clone(),
                equality: joins.equality.clone(),
            };
            partial.ids.remove(&keys["U"]);
            (partial, ())
        };
        rows.source_units[0].unit_id = UnitId::declared(keys["U"].clone());
        assert!(matches!(
            rows.encode(&partial),
            Err(EqualityFileRefusal::Malformed { .. })
        ));
    }

    /// FR-531-AC-18: the carried-quantity, bounded-quantity and map value
    /// types round trip, the example's subtree equals the spec's JSON with its
    /// members in order, and a bound in another unit or an unknown carrier tag
    /// is `Malformed`.
    #[trace("FR-531-AC-18", "TC-1331")]
    #[test]
    fn quantity_and_map_value_types_encode_as_the_spec_states() {
        let (joins, keys) = joins();
        let unit = UnitId::declared(keys["U"].clone());
        let quantity = |n: i64, u: &UnitId| {
            Quantity::new(
                Rational::new(Integer::from(n), Integer::from(1_i64)).unwrap(),
                u.clone(),
            )
        };
        let bounded = |min: Option<Quantity>, max: Option<Quantity>| {
            ValueType::BoundedQuantity(
                BoundedQuantityType::new(unit.clone(), min, max)
                    .unwrap()
                    .into(),
            )
        };
        let int =
            |lo: i64, hi: i64| IntegerInterval::new(Integer::from(lo), Integer::from(hi)).unwrap();
        let carried = |carrier| {
            ValueType::CarriedQuantity(CarriedQuantityType::new(unit.clone(), carrier).into())
        };
        let example = ValueType::Map(
            MapType::new(
                bounded(None, Some(quantity(5, &unit))),
                CardinalityBound::new(0, 4).ok(),
            )
            .into(),
        );
        let types = vec![
            carried(QuantityCarrier::Decimal(
                DecimalType::new(
                    Integer::from(0_i64),
                    Integer::from(9_i64),
                    0,
                    2,
                    RoundingMode::TowardZero,
                )
                .unwrap(),
            )),
            carried(QuantityCarrier::Integer {
                domain: int(-3, 3),
                rounding: RoundingMode::NearestAway,
            }),
            carried(QuantityCarrier::Rational(
                RationalDomain::new(int(-1, 1), int(1, 4)).unwrap(),
            )),
            bounded(Some(quantity(-2, &unit)), Some(quantity(5, &unit))),
            bounded(None, None),
            ValueType::Map(MapType::new(ValueType::String, None).into()),
            example.clone(),
        ];
        let mut rows = vector_rows(&keys);
        rows.facts = types
            .into_iter()
            .map(|ty| {
                fact_row(
                    &keys,
                    Some(ty),
                    EqualityFactWitness::OperandType {
                        position: 0,
                        role: EqualityOperandTypeRole::Source,
                    },
                )
            })
            .collect();
        let bytes = rows.encode(&joins).unwrap();
        assert_eq!(ReadEqualityRows::decode(&bytes, "p", &joins).unwrap(), rows);
        assert_eq!(rows.encode(&joins).unwrap(), bytes);
        let expected: Json = serde_json::from_str(
            r#"{ "kind": "map", "values": { "kind": "bounded_quantity", "unit": {
              "kind": "declared", "node": { "package": "p", "path": ["U"], "ordinal": 0 } },
              "min": null, "max": { "magnitude": { "numerator": "5", "denominator": "1" },
              "unit": { "kind": "declared", "node": { "package": "p", "path": ["U"], "ordinal": 0 } } } },
              "bound": { "minimum": 0, "maximum": 4 } }"#,
        )
        .unwrap();
        assert_eq!(value_type(&joins, &example).unwrap(), expected);
        // A bound in another unit, and a carrier tagged other than the three.
        let text = String::from_utf8(bytes).unwrap();
        let wrong: Json = serde_json::from_str(
            r#"{ "kind": "bounded_quantity", "unit": { "kind": "declared",
              "node": { "package": "p", "path": ["U"], "ordinal": 0 } }, "min": null,
              "max": { "magnitude": { "numerator": "5", "denominator": "1" },
              "unit": { "kind": "declared", "node": { "package": "p", "path": ["V"], "ordinal": 0 } } } }"#,
        )
        .unwrap();
        assert!(matches!(
            value_type_from(&joins, &wrong),
            Err(Bad::Malformed(_))
        ));
        let float = text.replace("\"kind\": \"integer\"", "\"kind\": \"float\"");
        assert_ne!(float, text);
        assert!(matches!(
            ReadEqualityRows::decode(float.as_bytes(), "p", &joins),
            Err(EqualityFileRefusal::Malformed { .. })
        ));
    }
}

/// Failure while admitting one equality decision under the entry budget.
pub(crate) enum EqualityDecisionFailure {
    Witness(EqualityWitnessRefusal),
    Exhausted(Box<AdmittedEqualityReadError>),
}
