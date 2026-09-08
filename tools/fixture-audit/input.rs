// SPDX-License-Identifier: AGPL-3.0-only
//! FR-012: bounded immutable file intake and strict decoded JSON maps.
use crate::error::{ensure, Code, Error, Result};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::{
    cell::Cell,
    fmt,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

/// Maximum accepted bytes in one fixture file.
pub(crate) const FILE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum accepted bytes across one complete audit.
const TOTAL_BYTES: usize = 64 * 1024 * 1024;
/// Maximum files and decoded values in an audit.
const RECORDS: usize = 10_000;
/// Maximum nested values, kept below serde_json's own recursion guard.
const JSON_DEPTH: usize = 64;

/// Per-invocation ceilings; tests can lower them without changing production logic.
pub(crate) struct Input {
    root: PathBuf,
    pub(crate) file_bytes: usize,
    pub(crate) bytes_left: usize,
    pub(crate) files_left: usize,
    records_left: Cell<usize>,
}

impl Input {
    /// Bind an explicit local fixture root. Its tree must stay immutable during use.
    pub(crate) fn new(root: &Path) -> Result<Self> {
        let root = root.canonicalize().map_err(|e| Error::io(root, e))?;
        ensure(
            root.is_dir(),
            Code::Usage,
            "fixture root must be a directory",
        )?;
        Ok(Self {
            root,
            file_bytes: FILE_BYTES,
            bytes_left: TOTAL_BYTES,
            files_left: RECORDS,
            records_left: Cell::new(RECORDS),
        })
    }

    /// Read a relative, contained regular file, charging the shared work budget.
    pub(crate) fn read(&mut self, relative: impl AsRef<Path>) -> Result<Vec<u8>> {
        let relative = relative.as_ref();
        ensure(
            !relative.is_absolute(),
            Code::ForeignPath,
            "absolute manifest path",
        )?;
        let selected = self.root.join(relative);
        let path = selected
            .canonicalize()
            .map_err(|e| Error::io(&selected, e))?;
        ensure(
            path.starts_with(&self.root),
            Code::ForeignPath,
            "fixture path escapes selected root",
        )?;
        self.read_selected(&path)
    }

    /// Read the fixed profile definition adjacent to the historical fixture root.
    pub(crate) fn profile(&mut self) -> Result<Vec<u8>> {
        let parent = self
            .root
            .parent()
            .ok_or_else(|| Error::new(Code::Usage, "fixture directory has no profile parent"))?;
        let selected = parent.join("profile.md");
        let path = selected
            .canonicalize()
            .map_err(|e| Error::io(&selected, e))?;
        ensure(
            path.starts_with(parent),
            Code::ForeignPath,
            "fixed profile escapes selected parent",
        )?;
        self.read_selected(&path)
    }

    fn read_selected(&mut self, path: &Path) -> Result<Vec<u8>> {
        // Check before open: opening a FIFO can block even without a concurrent
        // writer. The immutable-tree precondition covers replacement races.
        let selected = std::fs::metadata(path).map_err(|e| Error::io(path, e))?;
        ensure(
            selected.is_file(),
            Code::InvalidFixture,
            "fixture is not a regular file",
        )?;
        ensure(
            self.files_left > 0,
            Code::ResourceExhausted,
            "file count exhausted",
        )?;
        self.files_left -= 1;
        let file = File::open(path).map_err(|e| Error::io(path, e))?;
        let metadata = file.metadata().map_err(|e| Error::io(path, e))?;
        ensure(
            metadata.is_file(),
            Code::InvalidFixture,
            "fixture is not a regular file",
        )?;
        let limit = self.file_bytes.min(self.bytes_left);
        ensure(
            metadata.len()
                <= u64::try_from(limit)
                    .map_err(|_| Error::new(Code::ResourceExhausted, "byte limit out of range"))?,
            Code::ResourceExhausted,
            "fixture byte budget exhausted",
        )?;
        // A one-byte sentinel detects growth after metadata inspection. Inputs
        // are required to be immutable; even a violated precondition stays bounded.
        let cap = u64::try_from(limit)
            .ok()
            .and_then(|v| v.checked_add(1))
            .ok_or_else(|| Error::new(Code::ResourceExhausted, "byte limit overflow"))?;
        let mut bytes = Vec::new();
        file.take(cap)
            .read_to_end(&mut bytes)
            .map_err(|e| Error::io(path, e))?;
        ensure(
            bytes.len() <= limit,
            Code::ResourceExhausted,
            "fixture grew beyond byte budget",
        )?;
        self.bytes_left -= bytes.len();
        Ok(bytes)
    }

    /// Decode a file with shared byte, record and JSON recursion limits.
    pub(crate) fn json(&mut self, path: impl AsRef<Path>) -> Result<Value> {
        let bytes = self.read(path.as_ref())?;
        decode(&bytes, &self.records_left).map_err(|e| e.at(path.as_ref()))
    }

    /// Decode already-read selected bytes against the same record budget.
    pub(crate) fn decode(&self, bytes: &[u8]) -> Result<Value> {
        decode(bytes, &self.records_left)
    }
}

/// Strict JSON for standalone controls, bounded by the same default limits.
pub(crate) fn json(bytes: &[u8]) -> Result<Value> {
    ensure(
        bytes.len() <= FILE_BYTES,
        Code::ResourceExhausted,
        "JSON byte budget exhausted",
    )?;
    decode(bytes, &Cell::new(RECORDS))
}

fn decode(bytes: &[u8], left: &Cell<usize>) -> Result<Value> {
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    let exhausted = Cell::new(false);
    let depth = Cell::new(0);
    let value = Seed(left, &exhausted, &depth)
        .deserialize(&mut parser)
        .map_err(|e| {
            Error::new(
                if exhausted.get() {
                    Code::ResourceExhausted
                } else {
                    Code::InvalidJson
                },
                e.to_string(),
            )
        })?;
    parser
        .end()
        .map_err(|e| Error::new(Code::InvalidJson, e.to_string()))?;
    Ok(value)
}

struct Seed<'a>(&'a Cell<usize>, &'a Cell<bool>, &'a Cell<usize>);
impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Value, D::Error> {
        let depth = self.2.get();
        if depth >= JSON_DEPTH {
            self.1.set(true);
            return Err(de::Error::custom("JSON depth budget exhausted"));
        }
        let remaining = self.0.get().checked_sub(1).ok_or_else(|| {
            self.1.set(true);
            de::Error::custom("record budget exhausted")
        })?;
        self.0.set(remaining);
        self.2.set(depth + 1);
        let result = deserializer.deserialize_any(JsonVisitor(self.0, self.1, self.2));
        self.2.set(depth);
        result
    }
}

struct JsonVisitor<'a>(&'a Cell<usize>, &'a Cell<bool>, &'a Cell<usize>);
impl<'de> Visitor<'de> for JsonVisitor<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JSON with unique decoded keys")
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> std::result::Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> std::result::Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("nonfinite number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Value, E> {
        Ok(Value::String(value.into()))
    }
    fn visit_string<E: de::Error>(self, value: String) -> std::result::Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> std::result::Result<Value, A::Error> {
        let mut array = Vec::new();
        while let Some(value) = access.next_element_seed(Seed(self.0, self.1, self.2))? {
            array.push(value);
        }
        Ok(Value::Array(array))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> std::result::Result<Value, A::Error> {
        let mut map = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if map.contains_key(&key) {
                return Err(de::Error::custom("duplicate decoded JSON key"));
            }
            map.insert(key, access.next_value_seed(Seed(self.0, self.1, self.2))?);
        }
        Ok(Value::Object(map))
    }
}

/// Require an object member; absent/null never compare equal by default.
pub(crate) fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value> {
    value
        .as_object()
        .and_then(|v| v.get(name))
        .filter(|v| !v.is_null())
        .ok_or_else(|| {
            Error::new(
                Code::InvalidFixture,
                format!("missing or invalid field {name}"),
            )
        })
}
/// Require a scalar string at the fixture boundary.
pub(crate) fn text(value: &Value) -> Result<&str> {
    value
        .as_str()
        .ok_or_else(|| Error::new(Code::InvalidFixture, "expected string"))
}
/// Require an array at the fixture boundary.
pub(crate) fn array(value: &Value) -> Result<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::new(Code::InvalidFixture, "expected array"))
}
/// Require an unsigned scalar offset, without narrowing casts.
pub(crate) fn number(value: &Value) -> Result<usize> {
    value
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| Error::new(Code::InvalidFixture, "expected bounded unsigned integer"))
}
/// Compare a required field to a separately selected expected value.
pub(crate) fn equal(value: &Value, name: &str, expected: &Value) -> Result<()> {
    ensure(
        field(value, name)? == expected,
        Code::InvalidFixture,
        format!("field {name} mismatch"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("TC-006", "FR-012-AC-6")]
    #[test]
    fn malformed_json_and_fields() {
        for raw in [
            b"{\"x\":1,\"x\":2}".as_slice(),
            br#"{"x":1,"\u0078":2}"#,
            b"[",
            b"{} true",
            b"\xff",
            br#""\ud800""#,
        ] {
            assert_eq!(json(raw).unwrap_err().code, Code::InvalidJson);
        }
        for depth in 0..32 {
            let good = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
            assert!(json(good.as_bytes()).is_ok());
            let bad = format!(
                "{}{{\"a\":1,\"\\u0061\":2}}{}",
                "[".repeat(depth),
                "]".repeat(depth)
            );
            assert_eq!(json(bad.as_bytes()).unwrap_err().code, Code::InvalidJson);
        }
        for raw in [
            b"{}".as_slice(),
            br#"{"path":null}"#,
            br#"{"path":42}"#,
            br#"{"path":[]}"#,
        ] {
            let value = json(raw).unwrap();
            assert!(field(&value, "path").and_then(text).is_err());
        }
    }

    #[trace("TC-007", "FR-012-AC-7")]
    #[test]
    fn contained_and_escaping_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("inside"), b"selected").unwrap();
        std::fs::write(temp.path().join("outside"), b"foreign").unwrap();
        let mut input = Input::new(&root).unwrap();
        for depth in 0..16 {
            let good = format!("{}inside", "./".repeat(depth));
            assert_eq!(input.read(good).unwrap(), b"selected");
            let bad = format!("{}../outside", "./".repeat(depth));
            assert_eq!(input.read(bad).unwrap_err().code, Code::ForeignPath);
        }
        assert_eq!(
            input.read(root.join("inside")).unwrap_err().code,
            Code::ForeignPath
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(temp.path().join("outside"), root.join("escape")).unwrap();
            assert_eq!(input.read("escape").unwrap_err().code, Code::ForeignPath);
        }
        std::fs::create_dir(root.join("directory")).unwrap();
        assert_eq!(
            input.read("directory").unwrap_err().code,
            Code::InvalidFixture
        );
    }

    #[trace("TC-008", "FR-012-AC-9")]
    #[test]
    fn exact_and_exceeded_budgets() {
        let temp = tempfile::tempdir().unwrap();
        let exact = format!(
            "{}0{}",
            "[".repeat(JSON_DEPTH - 1),
            "]".repeat(JSON_DEPTH - 1)
        );
        assert!(json(exact.as_bytes()).is_ok());
        let over = format!("[{exact}]");
        assert_eq!(
            json(over.as_bytes()).unwrap_err().code,
            Code::ResourceExhausted
        );
        for limit in 0..24 {
            std::fs::write(temp.path().join("exact"), vec![b' '; limit]).unwrap();
            std::fs::write(temp.path().join("over"), vec![b' '; limit + 1]).unwrap();
            let mut input = Input::new(temp.path()).unwrap();
            input.file_bytes = limit;
            assert_eq!(input.read("exact").unwrap().len(), limit);
            assert_eq!(
                input.read("over").unwrap_err().code,
                Code::ResourceExhausted
            );
            let left = Cell::new(limit);
            let raw = format!("[{}]", vec!["0"; limit].join(","));
            assert_eq!(
                decode(raw.as_bytes(), &left).unwrap_err().code,
                Code::ResourceExhausted
            );
            assert!(decode(raw.as_bytes(), &Cell::new(limit + 1)).is_ok());
        }
        std::fs::write(temp.path().join("one"), b"x").unwrap();
        let mut input = Input::new(temp.path()).unwrap();
        input.bytes_left = 1;
        assert_eq!(input.read("one").unwrap(), b"x");
        assert_eq!(input.read("one").unwrap_err().code, Code::ResourceExhausted);
        let mut input = Input::new(temp.path()).unwrap();
        input.files_left = 1;
        input.read("one").unwrap();
        assert_eq!(input.read("one").unwrap_err().code, Code::ResourceExhausted);
        let file = File::create(temp.path().join("huge")).unwrap();
        file.set_len(u64::try_from(FILE_BYTES).unwrap() + 1)
            .unwrap();
        assert_eq!(
            Input::new(temp.path())
                .unwrap()
                .read("huge")
                .unwrap_err()
                .code,
            Code::ResourceExhausted
        );
    }
}
