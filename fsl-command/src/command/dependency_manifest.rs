// SPDX-License-Identifier: AGPL-3.0-or-later
//! The closed dependency artifact manifest (FR-517, FR-850).

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::fmt;

pub(super) const FORMAT: &str = "filament.checked-handoff.dependencies/1";

pub(super) struct Manifest {
    pub(super) dependencies: Vec<Row>,
}

pub(super) struct Row {
    pub(super) identity: String,
    pub(super) directory: String,
}

pub(super) struct Refusal {
    pub(super) cause: &'static str,
    pub(super) pointer: String,
    pub(super) supplied_format: Option<String>,
}

struct ObjectSeed<'a> {
    pointer: &'a mut String,
    row: Option<usize>,
}

impl<'de> DeserializeSeed<'de> for ObjectSeed<'_> {
    type Value = Vec<(String, serde_json::Value)>;
    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        decoder.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for ObjectSeed<'_> {
    type Value = Vec<(String, serde_json::Value)>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a closed manifest object")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let base = self
            .row
            .map_or_else(String::new, |row| format!("/dependencies/{row}"));
        let fields = if self.row.is_some() {
            ["identity", "directory"]
        } else {
            ["format", "dependencies"]
        };
        let mut values = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            *self.pointer = format!("{base}/{}", key.replace('~', "~0").replace('/', "~1"));
            if !fields.contains(&key.as_str()) || values.iter().any(|(held, _)| held == &key) {
                return Err(de::Error::custom("unknown or repeated member"));
            }
            let value = if self.row.is_none() && key == "dependencies" {
                let rows = map.next_value_seed(RowsSeed {
                    pointer: self.pointer,
                })?;
                serde_json::Value::Array(rows)
            } else {
                let value = map.next_value::<String>()?;
                serde_json::Value::String(value)
            };
            values.push((key, value));
        }
        for field in fields {
            if !values.iter().any(|(held, _)| held == field) {
                *self.pointer = format!("{base}/{field}");
                return Err(de::Error::missing_field(field));
            }
        }
        Ok(values)
    }
}

struct RowsSeed<'a> {
    pointer: &'a mut String,
}
impl<'de> DeserializeSeed<'de> for RowsSeed<'_> {
    type Value = Vec<serde_json::Value>;
    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        decoder.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for RowsSeed<'_> {
    type Value = Vec<serde_json::Value>;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("dependency rows")
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
        let mut rows = Vec::new();
        loop {
            *self.pointer = format!("/dependencies/{}", rows.len());
            let Some(row) = seq.next_element_seed(ObjectSeed {
                pointer: self.pointer,
                row: Some(rows.len()),
            })?
            else {
                break;
            };
            rows.push(serde_json::Value::Object(row.into_iter().collect()));
        }
        Ok(rows)
    }
}

impl Manifest {
    pub(super) fn read(bytes: &[u8]) -> Result<Self, Refusal> {
        let mut pointer = String::new();
        let mut decoder = serde_json::Deserializer::from_slice(bytes);
        let read = ObjectSeed {
            pointer: &mut pointer,
            row: None,
        }
        .deserialize(&mut decoder);
        let values = read.map_err(|_| Refusal {
            cause: "malformed-file",
            pointer: pointer.clone(),
            supplied_format: None,
        })?;
        decoder.end().map_err(|_| Refusal {
            cause: "malformed-file",
            pointer: String::new(),
            supplied_format: None,
        })?;
        let value = serde_json::Value::Object(values.into_iter().collect());
        let format = value["format"].as_str().unwrap_or_default();
        if format != FORMAT {
            return Err(Refusal {
                cause: "unknown-format",
                pointer: "/format".to_owned(),
                supplied_format: Some(format.to_owned()),
            });
        }
        let mut dependencies: Vec<Row> = Vec::new();
        for (index, row) in value["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let identity = row["identity"].as_str().unwrap_or_default();
            let directory = row["directory"].as_str().unwrap_or_default();
            let defect = if identity.is_empty()
                || dependencies
                    .last()
                    .is_some_and(|last| last.identity.as_str() >= identity)
            {
                Some("identity")
            } else if directory != format!("dependencies/{index}") {
                Some("directory")
            } else {
                None
            };
            if let Some(member) = defect {
                return Err(Refusal {
                    cause: "invalid-value",
                    pointer: format!("/dependencies/{index}/{member}"),
                    supplied_format: None,
                });
            }
            dependencies.push(Row {
                identity: identity.to_owned(),
                directory: directory.to_owned(),
            });
        }
        Ok(Self { dependencies })
    }
}
