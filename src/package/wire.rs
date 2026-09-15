// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-020: closed representation adapters; admitted models still come from callers.

use crate::serde_object::from_object;
use crate::ByteDigest;
use quire_contract_ir as ir;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// These record orders match the producer contract. Serializing a decoded claim
// allows bounded comparison without another parser, JSON tree, or semantic model.
macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Debug, Serialize)]
        pub(super) struct $name { $(pub $field: $ty),* }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Fields { $($field: $ty),* }
                let Fields { $($field),* } = from_object(decoder)?;
                Ok(Self { $($field),* })
            }
        }
    };
}

record!(Manifest {
    format: String, semantics: Semantics, required_features: Vec<String>, source: Source,
    models: Inventory<Model, 64>, clauses: Inventory<Clause, 256>, canonical_identity: CanonicalIdentity,
});
record!(Definition {
    revision: String,
    digest: Digest
});
record!(Semantics {
    language: String,
    edition: String,
    syntax_profile: String,
    model_profile: String,
    checking_contract: String,
    ir_revision: String,
    base_definition: Definition,
    rules_definition: Definition,
});
record!(Source {
    identity: Text,
    revision: Text,
    digest: Digest,
    formal: Formal
});
record!(CanonicalIdentity {
    domain: String,
    version: String,
    algorithm: String,
    digest: CanonicalDigest
});
record!(Model {
    alias: Text,
    owner: Owner,
    digest: Digest,
    artifact: Text
});
record!(Clause {
    name: Text, owner: Owner, clause: ir::ClauseId, kind: ClauseKind, execution_point: Point,
    span: Span, expression: Expression, context: Location, operation: Nullable<Location>,
    occurrences: Vec<Occurrence>, runtime: Runtime, projections: (NativeProjection, IrProjection),
});
record!(Location {
    identity: Identity,
    source: SourceSpan
});
record!(Identity {
    owner: Owner,
    key: Key
});
record!(Occurrence { expression: Nullable<Expression>, span: Span, target: Target });
record!(Runtime {
    context: Location, context_observations: Vec<ir::StateObservation>, universes: Vec<Universe>,
    operation: Nullable<Operation>, validate_frame: bool,
});
record!(Universe { model: Owner, record: Symbol, universe: Symbol, observations: Vec<ir::StateObservation> });
record!(Operation {
    model: Owner,
    context: Symbol,
    name: Symbol
});
record!(NativeProjection {
    target: String,
    status: String,
    cost_model: String
});
record!(IrProjection {
    target: String,
    status: String,
    code: String,
    span: Span
});

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ClauseKind {
    Invariant,
    Precondition,
    Postcondition,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Key {
    Type {
        name: Symbol,
    },
    Value {
        name: Symbol,
    },
    Scalar {
        name: Symbol,
    },
    Field {
        record: Symbol,
        field: Symbol,
    },
    Variant {
        enumeration: Symbol,
        variant: Symbol,
    },
    Operation {
        context: Symbol,
        name: Symbol,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Target {
    Formal { declaration: Location },
    Local { binding: Span },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Point {
    Initialization { name: ir::AnchorName },
    Handler { name: ir::AnchorName },
    Pre { operation: ir::AnchorName },
    Post { operation: ir::AnchorName },
}

impl Point {
    pub fn native(&self) -> ir::ExecutionPoint {
        match self {
            Self::Initialization { name } => {
                ir::ExecutionPoint::Initialization { name: name.clone() }
            }
            Self::Handler { name } => ir::ExecutionPoint::Handler { name: name.clone() },
            Self::Pre { operation } => ir::ExecutionPoint::Pre {
                operation: operation.clone(),
            },
            Self::Post { operation } => ir::ExecutionPoint::Post {
                operation: operation.clone(),
            },
        }
    }
}

record!(Owner {
    package: ir::PackageId,
    requirement: ir::RequirementId,
    revision: ir::RequirementRevision
});
impl Owner {
    pub fn native(&self) -> ir::RequirementRef {
        ir::RequirementRef::new(
            self.package.clone(),
            self.requirement.clone(),
            self.revision,
        )
    }
}
record!(Formal {
    document: ir::SourceDocumentId,
    revision: ir::SourceRevision
});
impl Formal {
    pub fn native(&self) -> ir::SourceIdentity {
        ir::SourceIdentity::new(self.document.clone(), self.revision)
    }
}

record!(PositionFields {
    source: Formal,
    line: u32,
    column: u32,
    byte_offset: u64
});
#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "PositionFields")]
pub(super) struct Position(ir::SourceLocation);
impl TryFrom<PositionFields> for Position {
    type Error = ir::Diagnostic;
    fn try_from(fields: PositionFields) -> Result<Self, Self::Error> {
        ir::SourceLocation::new(
            fields.source.native(),
            fields.line,
            fields.column,
            fields.byte_offset,
        )
        .map(Self)
    }
}
record!(SourceSpanFields {
    start: Position,
    end: Position
});
#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "SourceSpanFields")]
pub(super) struct SourceSpan(ir::SourceSpan);
impl TryFrom<SourceSpanFields> for SourceSpan {
    type Error = ir::Diagnostic;
    fn try_from(fields: SourceSpanFields) -> Result<Self, Self::Error> {
        ir::SourceSpan::new(fields.start.0, fields.end.0).map(Self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub(super) struct Symbol(ir::SymbolName);
impl TryFrom<String> for Symbol {
    type Error = ir::Diagnostic;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        ir::SymbolName::new(value).map(Self)
    }
}

record!(SpanFields {
    start: usize,
    end: usize
});
#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "SpanFields")]
pub(super) struct Span(SpanFields);
impl TryFrom<SpanFields> for Span {
    type Error = &'static str;
    fn try_from(fields: SpanFields) -> Result<Self, Self::Error> {
        if fields.start > fields.end || fields.end > 1_048_576 {
            return Err("invalid native package span");
        }
        Ok(Self(fields))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "usize")]
pub(super) struct Expression(usize);
impl TryFrom<usize> for Expression {
    type Error = &'static str;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value >= 10_000 {
            return Err("package expression exceeds native checker domain");
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub(super) struct Text(pub String);
impl TryFrom<String> for Text {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err("package label cannot be empty");
        }
        Ok(Self(value))
    }
}

#[derive(Debug)]
pub(super) struct Digest(pub ByteDigest);
impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        String::deserialize(decoder)?
            .parse()
            .map(Self)
            .map_err(de::Error::custom)
    }
}
impl Serialize for Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub(super) struct CanonicalDigest(String);
impl TryFrom<String> for CanonicalDigest {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("invalid native canonical digest spelling");
        }
        Ok(Self(value))
    }
}

// Unlike Option<T>'s missing-field behavior, this wrapper requires the member
// to exist while accepting an explicit JSON null.
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub(super) struct Nullable<T>(Option<T>);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Optional<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Optional<T> {
            type Value = Nullable<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a required value or explicit null")
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Nullable(None))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Nullable(None))
            }
            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                T::deserialize(d).map(|v| Nullable(Some(v)))
            }
        }
        // The newtype wrapper prevents Serde's missing-field deserializer from
        // directly treating this member as a missing Option.
        struct Required<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Required<T> {
            type Value = Nullable<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an explicitly present member")
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                d: D,
            ) -> Result<Self::Value, D::Error> {
                d.deserialize_option(Optional(std::marker::PhantomData))
            }
        }
        decoder.deserialize_newtype_struct("RequiredNullable", Required(std::marker::PhantomData))
    }
}

/// Wire inventory cardinality is checked before retaining the next item.
#[derive(Debug, Serialize)]
pub(super) struct Inventory<T, const MAX: usize>(Vec<T>);
impl<T, const MAX: usize> std::ops::Deref for Inventory<T, MAX> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'de, T: Deserialize<'de>, const MAX: usize> Deserialize<'de> for Inventory<T, MAX> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Items<T, const MAX: usize>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>, const MAX: usize> de::Visitor<'de> for Items<T, MAX> {
            type Value = Inventory<T, MAX>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "an inventory of at most {MAX} items")
            }
            fn visit_seq<A: de::SeqAccess<'de>>(
                self,
                mut array: A,
            ) -> Result<Self::Value, A::Error> {
                struct Item<T>(usize, std::marker::PhantomData<T>);
                impl<'de, T: Deserialize<'de>> de::DeserializeSeed<'de> for Item<T> {
                    type Value = T;
                    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<T, D::Error> {
                        if self.0 == 0 {
                            return Err(de::Error::custom(
                                "package inventory cardinality exceeded",
                            ));
                        }
                        T::deserialize(d)
                    }
                }
                let mut items = Vec::new();
                while let Some(item) =
                    array.next_element_seed(Item(MAX - items.len(), std::marker::PhantomData))?
                {
                    items.push(item);
                }
                Ok(Inventory(items))
            }
        }
        decoder.deserialize_seq(Items::<T, MAX>(std::marker::PhantomData))
    }
}
