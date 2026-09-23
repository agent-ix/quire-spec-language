// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-46 (FR-080-AC-3): `cargo xtask route-lint` -- scans the `#185`
//! registry crate (every `.rs` file under `qsl-route/src`, QSL-184) for a
//! `static`, `OnceLock`,
//! `thread_local!` or `lazy_static!` item and fails when it finds one.
//!
//! ADR-012 §5.3's registry evidence requires "a lint gate that finds no
//! `static`, `OnceLock` or `thread_local!` in the registry module". The
//! registry is specified to be an ordinary value, built by the
//! orchestrating driver and passed as an argument to every consumer
//! (FR-075 "The registry is an ordinary value, not ambient state"); any of
//! those constructs -- or `lazy_static!`, which this scan also catches
//! (PR #305 review, finding 11): its `static ref NAME: TYPE = EXPR;`
//! syntax is not valid Rust outside the macro invocation, so it never
//! reaches the plain `static` check and needs its own -- would let a
//! consumer read registrations through hidden global state instead, which
//! FR-075-CON-1 forbids. This scan catches that regression at the one
//! module FR-075 actually implements it in -- not the whole crate, so it
//! stays targeted at the seam FR-080 names, and not skipped, so ambient
//! state introduced in the registry module itself is actually caught
//! (TC-206's own two failure modes).

use std::fs;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::error::{Error, Result};

/// The registry crate's source root this gate scans, relative to the
/// workspace root. FR-080-AC-3's own scope is "the registry module (the
/// module implementing FR-075)"; since QSL-184 that module is the crate
/// `qsl-route`, so every file under its `src/` is scanned, and a module
/// added beside `lib.rs` cannot hold ambient state unseen.
pub const REGISTRY_SRC_ROOT: &str = "qsl-route/src";

/// One finding: a `static`, `OnceLock`-typed `static`, `thread_local!` or
/// `lazy_static!` item, named and located.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// What was found (`"static"`, `"OnceLock"`, `"thread_local!"` or
    /// `"lazy_static!"`).
    pub kind: &'static str,
    /// The item's own name, when it has one (a `static`'s identifier, or
    /// `"<thread_local! block>"`/`"<lazy_static! block>"` for a macro
    /// invocation, which syntactically may bind more than one name).
    pub name: String,
    /// 1-based source line the item starts on.
    pub line: u32,
}

struct AmbientStateVisitor {
    findings: Vec<Finding>,
}

impl<'ast> Visit<'ast> for AmbientStateVisitor {
    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        let type_mentions_once_lock = type_mentions_ident(&node.ty, "OnceLock");
        self.findings.push(Finding {
            kind: if type_mentions_once_lock {
                "OnceLock"
            } else {
                "static"
            },
            name: node.ident.to_string(),
            line: line_of(node),
        });
        syn::visit::visit_item_static(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Some(segment) = node.path.segments.last() {
            let (kind, name): (&'static str, &'static str) = if segment.ident == "thread_local" {
                ("thread_local!", "<thread_local! block>")
            } else if segment.ident == "lazy_static" {
                // `lazy_static!`'s `static ref NAME: TYPE = EXPR;` syntax
                // is not valid Rust `static` syntax on its own -- it
                // parses only inside the macro invocation, so it never
                // reaches `visit_item_static` above and needs its own
                // check here, the same way `thread_local!` does.
                ("lazy_static!", "<lazy_static! block>")
            } else {
                return syn::visit::visit_macro(self, node);
            };
            self.findings.push(Finding {
                kind,
                name: name.to_owned(),
                line: line_of(node),
            });
        }
        syn::visit::visit_macro(self, node);
    }
}

fn line_of<T: Spanned>(node: &T) -> u32 {
    u32::try_from(node.span().start().line).unwrap_or(u32::MAX)
}

/// Whether `ty` mentions `ident` anywhere in its type path (e.g. `OnceLock<T>`,
/// `std::sync::OnceLock<T>`, or a `OnceCell`-style alias someone renamed to
/// contain the substring is *not* detected here -- this is a syntactic
/// check over the literal written identifier).
fn type_mentions_ident(ty: &syn::Type, ident: &str) -> bool {
    struct IdentSeeker<'a> {
        target: &'a str,
        found: bool,
    }
    impl<'ast> Visit<'ast> for IdentSeeker<'_> {
        fn visit_ident(&mut self, node: &'ast proc_macro2::Ident) {
            if node == self.target {
                self.found = true;
            }
        }
    }
    let mut seeker = IdentSeeker {
        target: ident,
        found: false,
    };
    seeker.visit_type(ty);
    seeker.found
}

/// Scan `source` (already-read Rust source text) for `static`, `OnceLock`,
/// `thread_local!` and `lazy_static!` items. Pure function, used both by
/// the real gate (over every file under `qsl-route/src` on disk) and by this module's own
/// tests (over literal injected source strings).
pub fn scan_source(source: &str) -> std::result::Result<Vec<Finding>, syn::Error> {
    let parsed = syn::parse_file(source)?;
    let mut visitor = AmbientStateVisitor {
        findings: Vec::new(),
    };
    visitor.visit_file(&parsed);
    Ok(visitor.findings)
}

/// Every `.rs` file under `dir`, recursively, sorted. A missing `dir` is
/// an error, not an empty scan.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|source| Error::io(dir, source))? {
        let path = entry.map_err(|source| Error::io(dir, source))?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

/// `cargo xtask route-lint`: scan every file under [`REGISTRY_SRC_ROOT`]
/// and fail, naming every finding by file and line, if any contains a
/// `static`, `OnceLock`, `thread_local!` or `lazy_static!` item.
pub fn run(workspace_root: &Path) -> Result<String> {
    let mut files = Vec::new();
    rust_files(&workspace_root.join(REGISTRY_SRC_ROOT), &mut files)?;
    let mut summary = Vec::new();
    for path in &files {
        let source = fs::read_to_string(path).map_err(|source| Error::io(path.clone(), source))?;
        let findings = scan_source(&source).map_err(|source| Error::RouteLintParse {
            path: path.clone(),
            source,
        })?;
        let relative = path
            .strip_prefix(workspace_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        summary.extend(findings.iter().map(|finding| {
            format!(
                "{relative}:{} {} `{}`",
                finding.line, finding.kind, finding.name
            )
        }));
    }
    if !summary.is_empty() {
        return Err(Error::RouteLintFound {
            summary: summary.join("\n"),
        });
    }
    Ok(format!(
        "route-lint: {REGISTRY_SRC_ROOT} ({} files) has no static, OnceLock, thread_local! or \
         lazy_static! item.\n",
        files.len()
    ))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// TC-206 step 1: the unmodified module passes.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn the_real_registry_module_has_no_ambient_state() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask is one level under the workspace root");
        let outcome = run(workspace_root).expect("the real registry module must pass this gate");
        assert!(outcome.contains("no static, OnceLock, thread_local! or lazy_static!"));
    }

    /// QSL-184: the gate scans the whole registry crate. A `static` in a
    /// module beside `lib.rs` is found and named by its file, and a
    /// missing `qsl-route/src` is an error, not a clean scan.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn a_static_in_another_module_of_the_registry_crate_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join(REGISTRY_SRC_ROOT);
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("lib.rs"), "mod nested;\npub struct Registry;\n").unwrap();
        fs::write(src.join("nested/mod.rs"), "mod cache;\n").unwrap();
        fs::write(
            src.join("nested/cache.rs"),
            "static DEFAULT: std::sync::OnceLock<u8> = std::sync::OnceLock::new();\n",
        )
        .unwrap();
        let error = run(dir.path()).expect_err("the planted static fails the gate");
        assert!(
            error
                .to_string()
                .contains("qsl-route/src/nested/cache.rs:1 OnceLock `DEFAULT`"),
            "{error}"
        );

        let empty = tempfile::tempdir().unwrap();
        assert!(run(empty.path()).is_err(), "a missing qsl-route/src fails");
    }

    /// TC-206 step 2: an injected plain `static` is found and named.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_an_injected_plain_static() {
        let findings = scan_source(
            r#"
            pub struct Registry;
            static REGISTRY: Registry = Registry;
            "#,
        )
        .unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "static");
        assert_eq!(findings[0].name, "REGISTRY");
    }

    /// TC-206 step 3: an injected `OnceLock`-typed `static` is found and
    /// distinguished from a plain `static`.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_an_injected_once_lock_static() {
        let findings = scan_source(
            r#"
            static REGISTRY: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
            "#,
        )
        .unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "OnceLock");
        assert_eq!(findings[0].name, "REGISTRY");
    }

    /// TC-206 step 4: an injected `thread_local!` block is found.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_an_injected_thread_local_block() {
        let findings = scan_source(
            r#"
            thread_local! {
                static REGISTRY: u8 = 0;
            }
            "#,
        )
        .unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "thread_local!");
    }

    /// PR #305 review, finding 11: `lazy_static!`'s `static ref NAME: TYPE
    /// = EXPR;` syntax parses only inside the macro invocation, so it never
    /// reaches `visit_item_static` and needs its own detection, the same
    /// way `thread_local!` does.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_an_injected_lazy_static_block() {
        let findings = scan_source(
            r#"
            lazy_static::lazy_static! {
                static ref REGISTRY: u8 = 0;
            }
            "#,
        )
        .unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "lazy_static!");
    }

    /// A lint rule that only caught `static` would pass an implementation
    /// using any of the others (TC-206's own stated failure mode); confirm
    /// all four are independently detected in one scan.
    #[test]
    #[trace("TC-206", "FR-080-AC-3")]
    fn detects_all_four_kinds_in_one_module() {
        let findings = scan_source(
            r#"
            static PLAIN: u8 = 0;
            static LOCKED: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
            thread_local! {
                static LOCAL: u8 = 0;
            }
            lazy_static::lazy_static! {
                static ref LAZY: u8 = 0;
            }
            "#,
        )
        .unwrap();
        let kinds: Vec<&str> = findings.iter().map(|finding| finding.kind).collect();
        assert_eq!(
            kinds,
            ["static", "OnceLock", "thread_local!", "lazy_static!"]
        );
    }
}
