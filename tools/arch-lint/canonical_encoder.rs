// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 §2 (ADR-013:113, QSL-194): one RFC 8785 implementation.
//!
//! "One RFC 8785 JCS implementation produces every RFC 8785 encoding: the
//! `quire-canonical` crate." QSL once spelled "serialize with `serde_json`,
//! then SHA-256" at eight identity sites, each its own canonical encoder,
//! and a helper in one file (`key.rs`'s `jcs_bytes`) was hashed in others
//! (`population.rs`, `intake.rs`). QSL-194 moved them all to
//! `quire-canonical`; this check refuses a new one.
//!
//! **The rule keys on the hash.** A shipped source file that hashes
//! ([`HASHERS`]) fails unless it is on [`EXEMPT`], when it also either
//!
//! 1. names `serde_json` at all -- a serializer call, a grouped or renamed
//!    `use serde_json::{to_vec as tv, Value}`, a `serde_json::Value`
//!    whose `to_string()` it hashes; or
//! 2. calls a JSON-producing free function defined in another shipped file
//!    -- a `json_bytes` helper in one file hashed in another. A free function
//!    produces JSON when its body calls a [`SERIALIZERS`] entry, by path
//!    (`serde_json::to_vec`) or through a `use serde_json::...` import,
//!    renamed or not.
//!
//! So a file that reads or writes JSON hashes nothing, and a file that
//! hashes neither names JSON nor calls into a file that does. Shipped
//! production code hashes through `quire-canonical` (not a [`HASHERS`]
//! entry: it is the one encoder) or, for exact bytes, `ByteDigest::of` in a
//! file that does neither.
//!
//! **How.** The same token scan FR-060's T12-B/C/D use
//! ([`crate::api_surface`]): patterns match the file's `proc_macro2`
//! tokens, so a comment or string literal never matches, and `#[cfg(test)]`
//! items and out-of-line `#[cfg(test)] mod` files are excluded -- a test may
//! take an independent digest over `serde_json` output to check the encoder
//! against.
//!
//! **Stated limitations.** Rule 2 follows free functions (`fn` items) by
//! name, one call deep, within one crate: a method (`value.json_bytes()`), a function passed
//! as a value, a helper producing JSON through `Value`'s `Display`, or a
//! chain through a second helper file is not followed, nor is a helper in
//! another crate. A same-named
//! JSON-producing free function elsewhere can match; it is then named in
//! [`EXEMPT`] or renamed.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use syn::visit::Visit;

use crate::api_surface::{
    cfg_test_lines, cfg_test_module_declarations, flatten_tokens, module_path_of,
    pattern_match_lines, qsl_scan_src_roots, walk_rs_files, CallPattern, LocatedToken, Role, Token,
};
use crate::error::{Error, Result};

/// What names JSON: the `serde_json` crate, however it is imported.
pub(crate) const JSON: &[&str] = &["serde_json"];

/// The `serde_json` functions that produce JSON text or a JSON tree.
pub(crate) const SERIALIZERS: &[&str] = &[
    "to_vec",
    "to_vec_pretty",
    "to_string",
    "to_string_pretty",
    "to_writer",
    "to_writer_pretty",
    "to_value",
];

/// What hashes bytes in QSL: the SHA-256 hasher and its crate, and the two
/// raw-byte digest constructors that call it.
pub(crate) const HASHERS: &[&str] = &["Sha256", "sha2", "of_preimage(", "ByteDigest::of("];

/// Why a file on [`EXEMPT`] may pair a serializer with a hasher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExemptionKind {
    /// Not a canonical encoding at all: the digest is over exact emitted
    /// bytes, never over a canonical form of a value.
    NotAnIdentity,
    /// A canonical encoder of its own that predates QSL-194 and is outside
    /// its scope: reported as debt on every run. It leaves this list in the
    /// change that moves it to `quire-canonical`.
    Debt,
}

/// One file allowed to pair a serializer with a hasher, named by its
/// crate's source root and module path.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ExemptFile {
    pub(crate) crate_src: &'static str,
    pub(crate) module: &'static str,
}

/// Every exempt file, its kind and its reason. An entry with no remaining
/// match fails the check, so a fixed file cannot later hide a new encoder.
pub(crate) const EXEMPT: &[(ExemptFile, ExemptionKind, &str)] = &[
    (
        ExemptFile {
            crate_src: "src",
            module: "command",
        },
        ExemptionKind::NotAnIdentity,
        "the FR-001 `ByteDigest` of the exact artifact bytes it read from disk, handed \
         to the command's action; not an identity over a canonical form",
    ),
    (
        ExemptFile {
            crate_src: "src",
            module: "native_model",
        },
        ExemptionKind::NotAnIdentity,
        "the FR-001 `ByteDigest` of the native model artifact bytes it has just emitted; \
         not an identity over a canonical form",
    ),
    (
        ExemptFile {
            crate_src: "src",
            module: "package",
        },
        ExemptionKind::NotAnIdentity,
        "the FR-001 `ByteDigest` of the emitted package bytes, and FR-021's \
         `NativePackageIdentity`, which ADR-013 §2 names as a typed-record encoding, not \
         RFC 8785, retiring with `NativePackage` (ADR-011 §7.3 M-6c)",
    ),
    (
        ExemptFile {
            crate_src: "src",
            module: "protocol_artifact::encoding",
        },
        ExemptionKind::NotAnIdentity,
        "the FR-001 `ByteDigest` of a transport candidate's exact bytes; not an identity \
         over a canonical form",
    ),
    (
        ExemptFile {
            crate_src: "src",
            module: "runtime::reading",
        },
        ExemptionKind::NotAnIdentity,
        "the FR-001 `ByteDigest` of supplied artifact bytes, checked against the \
         expected digest before decoding; not an identity over a canonical form",
    ),
    (
        ExemptFile {
            crate_src: "src",
            module: "runtime::construction",
        },
        ExemptionKind::NotAnIdentity,
        "the FR-001 `ByteDigest` of the artifact bytes it has just emitted: an exact-byte \
         digest of those bytes, not an identity over a canonical form",
    ),
    (
        ExemptFile {
            crate_src: "src",
            module: "protocol_artifact::checked_handoff",
        },
        ExemptionKind::Debt,
        "the checked-handoff document identity hashes its `serde_json` struct-order \
         encoding, not RFC 8785; outside QSL-194's eight sites, moved by QSL-220",
    ),
    (
        ExemptFile {
            crate_src: "src",
            module: "protocol_artifact::native_temporal::common",
        },
        ExemptionKind::Debt,
        "the native-temporal request/result identities hash their `serde_json` \
         struct-order encoding, not RFC 8785; outside QSL-194's eight sites, moved by QSL-220",
    ),
];

/// One hashing file that also names JSON or calls into a file that does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EncoderSite {
    pub(crate) file: PathBuf,
    pub(crate) module: String,
    /// Every line naming a [`HASHERS`] entry.
    pub(crate) hasher_lines: Vec<usize>,
    /// Every line naming `serde_json`.
    pub(crate) json_lines: Vec<usize>,
    /// Every call of a JSON-producing free function defined in another
    /// file, as `(line, function, defining file)`.
    pub(crate) json_calls: Vec<(usize, String, PathBuf)>,
}

/// The check's result over one tree.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Outcome {
    /// Files not on [`EXEMPT`]: each fails the check.
    pub(crate) violations: Vec<EncoderSite>,
    /// Files on [`EXEMPT`] as [`ExemptionKind::Debt`]: reported, not failing.
    pub(crate) debt: Vec<(EncoderSite, &'static str)>,
    /// Files on [`EXEMPT`] as [`ExemptionKind::NotAnIdentity`].
    pub(crate) exempt: Vec<(EncoderSite, &'static str)>,
    /// [`EXEMPT`] entries with no remaining match: each fails the check.
    pub(crate) stale: Vec<ExemptFile>,
}

impl Outcome {
    pub(crate) fn passed(&self) -> bool {
        self.violations.is_empty() && self.stale.is_empty()
    }
}

/// What one shipped file names, outside its `#[cfg(test)]` items.
struct ScannedFile {
    path: PathBuf,
    crate_src: String,
    module: String,
    hasher_lines: Vec<usize>,
    json_lines: Vec<usize>,
    /// The file's shipped free functions (`fn` items).
    free_functions: BTreeSet<String>,
    /// Those of them that call a [`SERIALIZERS`] entry.
    json_producers: BTreeSet<String>,
    /// `use` imports: each local name and the `(parent module segment,
    /// imported name)` it stands for.
    imports: BTreeMap<String, (String, String)>,
    /// Module segments glob-imported (`use a::b::*` gives `b`).
    glob_imports: BTreeSet<String>,
    tokens: Vec<LocatedToken>,
    excluded: BTreeSet<usize>,
}

impl ScannedFile {
    /// Every shipped line calling free function `name` of the module whose
    /// last path segment is `module`: `module::name(`, or a bare `name(` (or
    /// its local rename) that this file imports from `module` by `use`,
    /// named or by glob. A method call (`.name(`) is not one.
    fn function_calls(&self, module: &str, name: &str) -> Vec<usize> {
        let mut local_names: Vec<&str> = self
            .imports
            .iter()
            .filter(|(_, (parent, original))| parent == module && original == name)
            .map(|(local, _)| local.as_str())
            .collect();
        if self.glob_imports.contains(module) {
            local_names.push(name);
        }
        let ident = |index: usize| match self.tokens.get(index).map(|t| &t.token) {
            Some(Token::Ident(word)) => Some(word.as_str()),
            _ => None,
        };
        let punct = |index: usize, c: char| {
            self.tokens.get(index).map(|t| &t.token) == Some(&Token::Punct(c))
        };
        let mut lines = Vec::new();
        for index in 0..self.tokens.len() {
            let Some(called) = ident(index) else {
                continue;
            };
            if self.tokens.get(index + 1).map(|t| &t.token) != Some(&Token::OpenParen) {
                continue;
            }
            let qualified = called == name
                && index >= 3
                && punct(index - 1, ':')
                && punct(index - 2, ':')
                && ident(index - 3) == Some(module);
            let bare = local_names.contains(&called)
                && !(index >= 1 && (punct(index - 1, '.') || punct(index - 1, ':')))
                && !(index >= 1 && ident(index - 1) == Some("fn"));
            if qualified || bare {
                lines.push(self.tokens[index].line);
            }
        }
        lines.retain(|line| !self.excluded.contains(line));
        lines
    }

    fn lines(&self, patterns: &[&str]) -> Vec<usize> {
        let compiled: Vec<CallPattern> = patterns.iter().map(|p| CallPattern::compile(p)).collect();
        pattern_match_lines(&self.tokens, &compiled)
            .into_iter()
            .filter(|line| !self.excluded.contains(line))
            .collect()
    }
}

/// Every shipped free function (`fn` item, not a method) in `parsed`
/// outside its `#[cfg(test)]` lines, with its first and last line.
fn free_functions(parsed: &syn::File, excluded: &BTreeSet<usize>) -> Vec<(String, usize, usize)> {
    struct Visitor<'a> {
        excluded: &'a BTreeSet<usize>,
        functions: Vec<(String, usize, usize)>,
    }
    impl<'ast> Visit<'ast> for Visitor<'_> {
        fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
            let span = syn::spanned::Spanned::span(node);
            if !self.excluded.contains(&node.sig.ident.span().start().line) {
                self.functions.push((
                    node.sig.ident.to_string(),
                    span.start().line,
                    span.end().line,
                ));
            }
            syn::visit::visit_item_fn(self, node);
        }
    }
    let mut visitor = Visitor {
        excluded,
        functions: Vec::new(),
    };
    visitor.visit_file(parsed);
    visitor.functions
}

/// A file's `use` imports: local name to `(parent module segment,
/// imported name)`, and the module segments it glob-imports.
fn imports(parsed: &syn::File) -> (BTreeMap<String, (String, String)>, BTreeSet<String>) {
    fn walk(
        tree: &syn::UseTree,
        parent: &str,
        names: &mut BTreeMap<String, (String, String)>,
        globs: &mut BTreeSet<String>,
    ) {
        match tree {
            syn::UseTree::Path(path) => walk(&path.tree, &path.ident.to_string(), names, globs),
            syn::UseTree::Name(name) => {
                let name = name.ident.to_string();
                names.insert(name.clone(), (parent.to_owned(), name));
            }
            syn::UseTree::Rename(rename) => {
                names.insert(
                    rename.rename.to_string(),
                    (parent.to_owned(), rename.ident.to_string()),
                );
            }
            syn::UseTree::Glob(_) => {
                globs.insert(parent.to_owned());
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    walk(item, parent, names, globs);
                }
            }
        }
    }
    struct Visitor(BTreeMap<String, (String, String)>, BTreeSet<String>);
    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
            walk(&node.tree, "", &mut self.0, &mut self.1);
        }
    }
    let mut visitor = Visitor(BTreeMap::new(), BTreeSet::new());
    visitor.visit_file(parsed);
    (visitor.0, visitor.1)
}

/// The local names a file's `use serde_json::...` items give the
/// [`SERIALIZERS`], renamed (`to_vec as tv`) or not.
fn imported_serializers(parsed: &syn::File) -> BTreeSet<String> {
    fn walk(tree: &syn::UseTree, under_serde_json: bool, out: &mut BTreeSet<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                walk(
                    &path.tree,
                    under_serde_json || path.ident == "serde_json",
                    out,
                );
            }
            syn::UseTree::Name(name) if under_serde_json => {
                if SERIALIZERS.contains(&name.ident.to_string().as_str()) {
                    out.insert(name.ident.to_string());
                }
            }
            syn::UseTree::Rename(rename) if under_serde_json => {
                if SERIALIZERS.contains(&rename.ident.to_string().as_str()) {
                    out.insert(rename.rename.to_string());
                }
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    walk(item, under_serde_json, out);
                }
            }
            _ => {}
        }
    }
    struct Visitor(BTreeSet<String>);
    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
            walk(&node.tree, false, &mut self.0);
        }
    }
    let mut visitor = Visitor(BTreeSet::new());
    visitor.visit_file(parsed);
    visitor.0
}

fn scan_file(path: &Path, crate_src: &str, module: &str) -> Result<ScannedFile> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let parsed = syn::parse_file(&text).map_err(|source| Error::source_parse(path, source))?;
    let stream: proc_macro2::TokenStream = text
        .parse()
        .map_err(|source| Error::source_parse(path, source))?;
    let mut tokens = Vec::new();
    flatten_tokens(stream, &mut tokens);
    let excluded = cfg_test_lines(&parsed);
    let functions = free_functions(&parsed, &excluded);
    let aliases = imported_serializers(&parsed);
    let (imports, glob_imports) = imports(&parsed);
    let mut file = ScannedFile {
        path: path.to_path_buf(),
        crate_src: crate_src.to_owned(),
        module: module.to_owned(),
        hasher_lines: Vec::new(),
        json_lines: Vec::new(),
        free_functions: functions.iter().map(|(name, _, _)| name.clone()).collect(),
        json_producers: BTreeSet::new(),
        imports,
        glob_imports,
        tokens,
        excluded,
    };
    file.hasher_lines = file.lines(HASHERS);
    file.json_lines = file.lines(JSON);
    let mut serializer_patterns: Vec<String> = SERIALIZERS
        .iter()
        .map(|name| format!("serde_json::{name}("))
        .collect();
    serializer_patterns.extend(aliases.iter().map(|alias| format!("{alias}(")));
    let patterns: Vec<&str> = serializer_patterns.iter().map(String::as_str).collect();
    let serializer_lines = file.lines(&patterns);
    file.json_producers = functions
        .into_iter()
        .filter(|(_, start, end)| {
            serializer_lines
                .iter()
                .any(|line| (start..=end).contains(&line))
        })
        .map(|(name, _, _)| name)
        .collect();
    Ok(file)
}

/// Every shipped file under every scanned crate root of `qsl_root`.
fn scan_tree(qsl_root: &Path) -> Result<Vec<ScannedFile>> {
    let mut scanned = Vec::new();
    for src_root in qsl_scan_src_roots(Role::Qsl, qsl_root) {
        if !src_root.exists() {
            return Err(Error::new(
                crate::error::Code::Usage,
                format!("source root does not exist: {}", src_root.display()),
            ));
        }
        let crate_src = src_root
            .strip_prefix(qsl_root)
            .map_err(|_| {
                Error::new(
                    crate::error::Code::Usage,
                    format!("{} is not under {}", src_root.display(), qsl_root.display()),
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");
        let mut files = Vec::new();
        walk_rs_files(&src_root, &mut files)?;
        files.sort();
        let mut modules = Vec::with_capacity(files.len());
        for file in files {
            let module = file
                .strip_prefix(&src_root)
                .map(module_path_of)
                .map_err(|_| {
                    Error::new(
                        crate::error::Code::Usage,
                        format!("{} is not under {}", file.display(), src_root.display()),
                    )
                })?;
            modules.push((file, module));
        }
        let test_modules = cfg_test_module_declarations(&modules)?;
        for (file, module) in &modules {
            let under = |ancestor: &String| {
                module == ancestor || module.starts_with(&format!("{ancestor}::"))
            };
            if !test_modules.iter().any(under) {
                scanned.push(scan_file(file, &crate_src, module)?);
            }
        }
    }
    Ok(scanned)
}

/// Run the check over the QSL checkout at `qsl_root`: every shipped QSL
/// crate's `src/`, the roots FR-060's QSL-side rules scan.
pub(crate) fn evaluate(qsl_root: &Path) -> Result<Outcome> {
    let files = scan_tree(qsl_root)?;
    // Rule 2's targets: each JSON-producing free function, and the file
    // defining it.
    let mut json_functions: Vec<(&str, &str, &str, &Path)> = Vec::new();
    for file in &files {
        let Some(segment) = file.module.rsplit("::").next().filter(|s| !s.is_empty()) else {
            continue;
        };
        for name in &file.json_producers {
            json_functions.push((name, segment, &file.crate_src, &file.path));
        }
    }
    let mut outcome = Outcome::default();
    let mut seen: BTreeSet<ExemptFile> = BTreeSet::new();
    for file in files.iter().filter(|file| !file.hasher_lines.is_empty()) {
        let mut json_calls = Vec::new();
        for (name, segment, crate_src, defined_in) in &json_functions {
            if *crate_src != file.crate_src
                || *defined_in == file.path
                || file.free_functions.contains(*name)
            {
                continue;
            }
            for line in file.function_calls(segment, name) {
                json_calls.push((line, (*name).to_owned(), defined_in.to_path_buf()));
            }
        }
        json_calls.sort();
        if file.json_lines.is_empty() && json_calls.is_empty() {
            continue;
        }
        let site = EncoderSite {
            file: file.path.clone(),
            module: file.module.clone(),
            hasher_lines: file.hasher_lines.clone(),
            json_lines: file.json_lines.clone(),
            json_calls,
        };
        let entry = EXEMPT.iter().find(|(exempt, _, _)| {
            exempt.crate_src == file.crate_src && exempt.module == file.module
        });
        match entry {
            Some((exempt, kind, reason)) => {
                seen.insert(*exempt);
                match kind {
                    ExemptionKind::Debt => outcome.debt.push((site, reason)),
                    ExemptionKind::NotAnIdentity => outcome.exempt.push((site, reason)),
                }
            }
            None => outcome.violations.push(site),
        }
    }
    outcome.stale = EXEMPT
        .iter()
        .map(|(exempt, _, _)| *exempt)
        .filter(|exempt| !seen.contains(exempt))
        .collect();
    Ok(outcome)
}

/// The report `main` prints.
pub(crate) fn report(outcome: &Outcome) -> String {
    let lines = |numbers: &[usize]| {
        numbers
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };
    let site = |site: &EncoderSite| {
        let mut text = format!(
            "{} (module {}): hash at line {}",
            site.file.display(),
            site.module,
            lines(&site.hasher_lines)
        );
        if !site.json_lines.is_empty() {
            text.push_str(&format!(", serde_json at line {}", lines(&site.json_lines)));
        }
        for (line, name, defined_in) in &site.json_calls {
            text.push_str(&format!(
                ", calls JSON-producing {name} (defined in {}) at line {line}",
                defined_in.display()
            ));
        }
        text
    };
    let mut summary = String::new();
    summary.push_str(
        "ADR-013 §2 one-RFC-8785-encoder check (ADR-013:113, QSL-194)\n  Note: a shipped file \
         that hashes (Sha256, sha2, of_preimage, ByteDigest::of) fails if it names serde_json \
         in any form or calls a JSON-producing free function defined in another file. \
         Methods, functions passed as values, Display-based helpers, other crates' helpers \
         and chains through a second helper are not followed.\n",
    );
    if outcome.passed() {
        summary.push_str("  PASS: no second canonical encoder beside quire-canonical\n");
    } else {
        summary.push_str("  FAIL\n");
        for violation in &outcome.violations {
            summary.push_str(&format!(
                "    second canonical encoder: {} -- encode through quire-canonical instead\n",
                site(violation)
            ));
        }
        for stale in &outcome.stale {
            summary.push_str(&format!(
                "    stale exemption, no remaining match: {} {}\n",
                stale.crate_src, stale.module
            ));
        }
    }
    for (debt, reason) in &outcome.debt {
        summary.push_str(&format!("    debt: {} -- {reason}\n", site(debt)));
    }
    for (exempt, reason) in &outcome.exempt {
        summary.push_str(&format!("    exempt: {} -- {reason}\n", site(exempt)));
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOTS: [&str; 10] = [
        "src",
        "qsl-foundation/src",
        "qsl-cst/src",
        "qsl-source/src",
        "qsl-forms/src",
        "qsl-semantics/src",
        "qsl-package/src",
        "qsl-eval/src",
        "qsl-route/src",
        "qsl-replay/src",
    ];

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// A QSL tree with every scan root and every [`EXEMPT`] file matching,
    /// so a test sees only the scenario it plants.
    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for relative in ROOTS {
            fs::create_dir_all(dir.path().join(relative)).unwrap();
        }
        for (exempt, _, _) in EXEMPT {
            write(
                dir.path(),
                &format!(
                    "{}/{}.rs",
                    exempt.crate_src,
                    exempt.module.replace("::", "/")
                ),
                "fn e(v: &V) { let b = serde_json::to_vec(v); Sha256::digest(b); }\n",
            );
        }
        dir
    }

    const SECOND_ENCODER: &str = "\
use sha2::{Digest, Sha256};

pub(super) fn jcs_bytes(value: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

pub(super) fn digest_of(value: &serde_json::Value) -> [u8; 32] {
    Sha256::digest(jcs_bytes(value)).into()
}
";

    fn violations(dir: &tempfile::TempDir) -> Vec<EncoderSite> {
        let outcome = evaluate(dir.path()).unwrap();
        assert_eq!(outcome.stale, Vec::new(), "{}", report(&outcome));
        outcome.violations
    }

    /// The deleted `key.rs` pair, serializing in one function and hashing
    /// in another, fails, naming both lines.
    #[test]
    fn a_second_encoder_split_across_functions_fails() {
        let dir = tree();
        write(dir.path(), "qsl-semantics/src/model/key.rs", SECOND_ENCODER);
        let found = violations(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].module, "model::key");
        assert_eq!(found[0].hasher_lines, vec![1, 8]);
        assert_eq!(found[0].json_lines, vec![3, 4, 7]);
    }

    /// PR #389 review, planted case 1: a grouped, renamed serializer import.
    #[test]
    fn a_renamed_serializer_import_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "use serde_json::{to_vec as tv, Value};\nuse sha2::{Digest, Sha256};\n\
             pub fn id(v: &Value) -> [u8; 32] { Sha256::digest(tv(v).unwrap()).into() }\n",
        );
        let found = violations(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].json_lines, vec![1]);
        assert_eq!(found[0].hasher_lines, vec![2, 3]);
    }

    /// PR #389 review, planted case 2: a hash over `Value::to_string()`.
    #[test]
    fn a_hash_over_value_display_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/intake.rs",
            "pub fn id(v: &serde_json::Value) -> ByteDigest { \
             ByteDigest::of(v.to_string().as_bytes()) }\n",
        );
        assert_eq!(violations(&dir).len(), 1);
    }

    /// PR #389 review, planted case 3: a `json_bytes` helper in one file,
    /// hashed in another that never names `serde_json` itself.
    #[test]
    fn a_json_helper_hashed_in_another_file_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/json.rs",
            "pub fn json_bytes<T: serde::Serialize>(v: &T) -> Vec<u8> { \
             serde_json::to_vec(v).unwrap() }\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "use super::json::json_bytes;\n\
             pub fn id<T: serde::Serialize>(v: &T) -> [u8; 32] {\n    \
             *ByteDigest::of(&json_bytes(v)).as_bytes()\n}\n",
        );
        let found = violations(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].module, "model::key");
        assert!(found[0].json_lines.is_empty());
        let calls: Vec<(usize, &str)> = found[0]
            .json_calls
            .iter()
            .map(|(line, name, _)| (*line, name.as_str()))
            .collect();
        assert_eq!(calls, vec![(3, "json_bytes")]);
    }

    /// A tree whose hashing files name no JSON, and whose JSON files hash
    /// nothing, passes; the exempt files are still reported.
    #[test]
    fn a_tree_with_no_second_encoder_passes() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub fn digest(value: &V) -> [u8; 32] { \
             *quire_canonical::sha256(value, LIMITS).unwrap().as_bytes() }\n\
             pub fn raw(bytes: &[u8]) -> ByteDigest { ByteDigest::of(bytes) }\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/intake.rs",
            "pub fn read(b: &[u8]) -> serde_json::Value { serde_json::from_slice(b).unwrap() }\n\
             pub fn digest(b: &[u8]) -> [u8; 32] { super::key::raw(b) }\n",
        );
        let outcome = evaluate(dir.path()).unwrap();
        assert!(outcome.passed(), "{}", report(&outcome));
        let of_kind = |kind| EXEMPT.iter().filter(|(_, k, _)| *k == kind).count();
        assert_eq!(outcome.debt.len(), of_kind(ExemptionKind::Debt));
        assert_eq!(outcome.exempt.len(), of_kind(ExemptionKind::NotAnIdentity));
    }

    /// A serializer and a hasher inside `#[cfg(test)]` code, in an
    /// out-of-line `#[cfg(test)] mod` file, or named only in a comment or a
    /// string, are not shipped code and do not match.
    #[test]
    fn test_code_comments_and_strings_do_not_match() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "// serde_json::to_vec then Sha256\n\
             pub const NOTE: &str = \"serde_json::to_vec Sha256\";\n\
             #[cfg(test)]\nmod tests {\n    fn f(v: &V) { Sha256::digest(serde_json::to_vec(v).unwrap()); }\n}\n",
        );
        write(
            dir.path(),
            "qsl-package/src/checked_v2.rs",
            "#[cfg(test)]\nmod tests;\npub fn read() {}\n",
        );
        write(
            dir.path(),
            "qsl-package/src/checked_v2/tests.rs",
            SECOND_ENCODER,
        );
        let outcome = evaluate(dir.path()).unwrap();
        assert!(outcome.passed(), "{}", report(&outcome));
    }

    /// An exemption whose file no longer pairs the two fails as stale.
    #[test]
    fn a_stale_exemption_fails() {
        let dir = tree();
        write(
            dir.path(),
            "src/protocol_artifact/checked_handoff.rs",
            "pub fn encode(v: &V) -> Vec<u8> { quire_canonical::to_vec(v, L).unwrap() }\n",
        );
        let outcome = evaluate(dir.path()).unwrap();
        assert!(!outcome.passed());
        assert_eq!(
            outcome.stale,
            vec![ExemptFile {
                crate_src: "src",
                module: "protocol_artifact::checked_handoff",
            }]
        );
    }
}
